//! Hardware MIDI input/output port communication and asynchronous event polling.
//! Non-blocking message ingestion, frontend wake closures, and outgoing transmission.

use std::sync::mpsc::{self, Receiver, Sender};

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

use crate::Message;

/// An open MIDI input.
///
/// Holds the connection alive: dropping this closes the port, which is why it
/// is returned rather than leaked, even though nothing reads its fields.
pub struct Port {
    _connection: Option<MidiInputConnection<Callback>>,
    _handle: Option<Box<dyn std::any::Any + Send>>,
    messages: Receiver<Message>,
    name: String,
}

/// Feeder handle for pushing parsed messages or raw bytes into a `Port`.
#[derive(Clone)]
pub struct SenderPort {
    tx: Sender<Message>,
    wake: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
}

impl SenderPort {
    /// Pushes a parsed MIDI message into the port's queue, triggering `wake` if registered.
    pub fn send(&self, message: Message) {
        let _ = self.tx.send(message);
        if let Some(wake) = &self.wake {
            wake();
        }
    }

    /// Parses raw wire bytes and pushes the parsed message into the port's queue.
    pub fn send_bytes(&self, bytes: &[u8]) {
        if let Some(message) = Message::parse(bytes) {
            self.send(message);
        }
    }
}

/// Context passed to the MIDI input callback.
struct Callback {
    messages: Sender<Message>,
    /// Optional notification closure called once per incoming message on the MIDI thread.
    wake: Option<Box<dyn Fn() + Send>>,
}

impl Port {
    /// Creates a custom `Port` paired with a `SenderPort` feeder
    /// (e.g. for Web MIDI API or testing).
    pub fn custom(
        name: impl Into<String>,
        wake: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
    ) -> (Port, SenderPort) {
        Self::custom_with_handle(name, wake, ())
    }

    /// Creates a custom `Port` holding an arbitrary driver handle to keep underlying
    /// connections (e.g. Web MIDI closures or native devices) alive until dropped.
    pub fn custom_with_handle<H: 'static + Send>(
        name: impl Into<String>,
        wake: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
        handle: H,
    ) -> (Port, SenderPort) {
        let (tx, rx) = mpsc::channel();
        let port = Port {
            _connection: None,
            _handle: Some(Box::new(handle)),
            messages: rx,
            name: name.into(),
        };
        let sender = SenderPort { tx, wake };
        (port, sender)
    }

    /// Opens the first input whose name contains `wanted` case-insensitively, or the first
    /// available input if `wanted` is empty.
    pub fn open(wanted: &str) -> Result<Port, String> {
        Port::opened(wanted, None)
    }

    /// Opens the matching input and registers a `wake` closure invoked per received message.
    ///
    /// Wakes sleeping event loops when new MIDI messages arrive.
    pub fn waking(wanted: &str, wake: impl Fn() + Send + 'static) -> Result<Port, String> {
        Port::opened(wanted, Some(Box::new(wake)))
    }

    fn opened(wanted: &str, wake: Option<Box<dyn Fn() + Send>>) -> Result<Port, String> {
        let input = MidiInput::new("karakuri").map_err(|e| format!("no MIDI at all: {e}"))?;
        let ports = input.ports();
        let named: Vec<(usize, String)> = ports
            .iter()
            .enumerate()
            .map(|(i, p)| (i, input.port_name(p).unwrap_or_else(|_| "?".to_string())))
            .collect();
        let found = named
            .iter()
            .find(|(_, name)| {
                wanted.is_empty() || name.to_lowercase().contains(&wanted.to_lowercase())
            })
            .ok_or_else(|| {
                let list = if named.is_empty() {
                    "there are no MIDI inputs".to_string()
                } else {
                    format!(
                        "the inputs are: {}",
                        named
                            .iter()
                            .map(|(_, n)| n.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                format!("no MIDI input matching `{wanted}` — {list}")
            })?;
        let (index, name) = (found.0, found.1.clone());

        let (tx, messages) = mpsc::channel();
        let connection = input
            .connect(
                &ports[index],
                "karakuri-in",
                |_stamp, bytes, back: &mut Callback| {
                    // Parsed on the MIDI thread to avoid per-frame allocations.
                    // Dropped send errors indicate receiver shutdown.
                    if let Some(message) = Message::parse(bytes) {
                        let _ = back.messages.send(message);
                        // Wake after enqueueing so the consumer thread finds the message ready.
                        if let Some(wake) = back.wake.as_ref() {
                            wake();
                        }
                    }
                },
                Callback { messages: tx, wake },
            )
            .map_err(|e| format!("could not open `{name}`: {e}"))?;

        Ok(Port {
            _connection: Some(connection),
            _handle: None,
            messages,
            name,
        })
    }

    /// Drains all messages received since the last call into `into` without blocking.
    ///
    /// Clears `into` before populating it with queued messages.
    pub fn drain(&self, into: &mut Vec<Message>) {
        into.clear();
        while let Ok(message) = self.messages.try_recv() {
            into.push(message);
        }
    }

    /// The port that was opened, as the device named it.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Open MIDI output device connection managed on a dedicated worker thread.
///
/// Emits MIDI bytes asynchronously across a bounded channel to prevent blocking the render thread.
/// Destination sink for outgoing MIDI wire bytes.
enum OutSink {
    SyncChannel(mpsc::SyncSender<[u8; 3]>),
    Custom(Box<dyn Fn([u8; 3]) -> bool + Send>),
}

/// Open MIDI output device connection managed on a dedicated worker thread or custom sender.
///
/// Emits MIDI bytes asynchronously across a bounded channel to prevent blocking the render thread.
/// If the channel fills, subsequent messages are dropped and counted in `dropped`.
pub struct Out {
    sink: OutSink,
    name: String,
    dropped: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

/// Maximum in-flight messages permitted in the output queue.
const QUEUE: usize = 256;

impl Out {
    /// Creates a custom `Out` instance routing messages through an arbitrary closure
    /// (e.g. for Web MIDI API or testing).
    pub fn custom(
        name: impl Into<String>,
        send_fn: impl Fn([u8; 3]) -> bool + Send + 'static,
    ) -> Out {
        Out {
            sink: OutSink::Custom(Box::new(send_fn)),
            name: name.into(),
            dropped: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Opens the first output matching `wanted` case-insensitively.
    ///
    /// Accepts partial substrings because device input/output port names often differ.
    pub fn open(wanted: &str) -> Result<Out, String> {
        let (ready, opened) = mpsc::channel::<Result<String, String>>();
        let (to, from) = mpsc::sync_channel::<[u8; 3]>(QUEUE);
        let wanted = wanted.to_owned();
        std::thread::Builder::new()
            .name("karakuri-midi-out".to_string())
            .spawn(move || match Out::connect(&wanted) {
                Err(why) => {
                    let _ = ready.send(Err(why));
                }
                Ok((mut connection, name)) => {
                    if ready.send(Ok(name)).is_err() {
                        return;
                    }
                    // Dedicated output writer thread. Blocks on `recv` to keep callers non-blocking.
                    while let Ok(bytes) = from.recv() {
                        let _ = connection.send(&bytes);
                    }
                }
            })
            .map_err(|e| format!("no thread for MIDI out: {e}"))?;
        let name = opened
            .recv()
            .map_err(|_| "the MIDI output thread stopped before it opened a port".to_string())??;
        Ok(Out {
            sink: OutSink::SyncChannel(to),
            name,
            dropped: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    /// Find the port and connect, on the sender thread. Split out so that
    /// every early return above is one `match`.
    fn connect(wanted: &str) -> Result<(MidiOutputConnection, String), String> {
        let output = MidiOutput::new("karakuri").map_err(|e| format!("no MIDI at all: {e}"))?;
        let ports = output.ports();
        let named: Vec<(usize, String)> = ports
            .iter()
            .enumerate()
            .map(|(i, p)| (i, output.port_name(p).unwrap_or_else(|_| "?".to_string())))
            .collect();
        let found = named
            .iter()
            .find(|(_, name)| {
                wanted.is_empty() || name.to_lowercase().contains(&wanted.to_lowercase())
            })
            .ok_or_else(|| {
                let list = if named.is_empty() {
                    "there are no MIDI outputs".to_string()
                } else {
                    format!(
                        "the outputs are: {}",
                        named
                            .iter()
                            .map(|(_, n)| n.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                format!("no MIDI output matching `{wanted}` — {list}")
            })?;
        let (index, name) = (found.0, found.1.clone());
        let connection = output
            .connect(&ports[index], "karakuri-out")
            .map_err(|e| format!("could not open `{name}`: {e}"))?;
        Ok((connection, name))
    }

    /// Queues one message without blocking. Returns true if queued, or false if dropped.
    pub fn send(&self, message: [u8; 3]) -> bool {
        let ok = match &self.sink {
            OutSink::SyncChannel(to) => to.try_send(message).is_ok(),
            OutSink::Custom(f) => f(message),
        };
        if !ok {
            self.dropped
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        ok
    }

    /// How many messages have been dropped over the whole run.
    pub fn dropped(&self) -> usize {
        self.dropped.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// The port that was opened, as the device named it.
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Verifies that a full queue drops excess messages without blocking.
    #[test]
    fn a_full_queue_drops_rather_than_blocking() {
        let (to, held) = mpsc::sync_channel::<[u8; 3]>(QUEUE);
        let out = Out {
            sink: OutSink::SyncChannel(to),
            name: "nothing is draining this".to_string(),
            dropped: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };
        for i in 0..QUEUE {
            assert!(out.send([0xb0, 1, 0]), "message {i} was dropped with room");
        }
        assert_eq!(out.dropped(), 0);
        // The queue is full and this returns rather than waiting.
        for i in 0..8 {
            assert!(
                !out.send([0xb0, 1, 0]),
                "message {i} past the bound was queued"
            );
        }
        assert_eq!(out.dropped(), 8, "a drop went uncounted");
        // And a receiver that has gone is a drop too, rather than a panic on
        // the frame path: a run that is ending is not a run that should fault.
        drop(held);
        assert!(!out.send([0xb0, 1, 0]));
        assert_eq!(out.dropped(), 9);
    }

    #[test]
    fn custom_port_sender_and_out_pipeline() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let woken = Arc::new(AtomicBool::new(false));
        let woken_clone = Arc::clone(&woken);

        let (port, sender) = Port::custom(
            "Virtual Port",
            Some(Arc::new(move || {
                woken_clone.store(true, Ordering::SeqCst);
            })),
        );
        assert_eq!(port.name(), "Virtual Port");

        // Send a CC message via bytes
        sender.send_bytes(&[0xb0, 10, 64]);
        assert!(woken.load(Ordering::SeqCst));

        let mut drained = Vec::new();
        port.drain(&mut drained);
        assert_eq!(drained.len(), 1);
        assert_eq!(
            drained[0],
            Message::ControlChange {
                channel: 0,
                controller: 10,
                value: 64,
            }
        );

        // Test Out::custom
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let recv_clone = Arc::clone(&received);
        let out = Out::custom("Virtual Out", move |bytes| {
            recv_clone.lock().unwrap().push(bytes);
            true
        });
        assert_eq!(out.name(), "Virtual Out");
        assert!(out.send([0x90, 60, 127]));
        assert_eq!(*received.lock().unwrap(), vec![[0x90, 60, 127]]);
    }
}
