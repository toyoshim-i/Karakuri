//! The port: the only part of this crate that talks to hardware.
//!
//! Deliberately thin, in the same shape as `karakuri-audio`'s device module —
//! though not on the same terms, which the callback section below is careful
//! about. It opens a port, parses each callback's bytes, and pushes whatever it
//! recognises down a channel. Nothing here decides anything: [`crate::Map`] is
//! where a message becomes a request, and it is tested against bytes.
//!
//! ## What the callback may not do
//!
//! It runs on the MIDI system's thread, and the frame loop reads from the other
//! end of a channel. An `mpsc::Sender` is the whole of it: **neither side ever
//! waits on the other**, since `send` on an unbounded channel does not block
//! and the frame drains with `try_recv`.
//!
//! **It is not allocation-free, and `karakuri-audio`'s callback is** — so this
//! is the weaker of the two claims and saying otherwise would be borrowing that
//! module's guarantee. `std::sync::mpsc` allocates a block periodically on
//! send, and an allocation can wait on the allocator. What makes that
//! acceptable here and not there is the rate: an audio callback runs every 11
//! ms with a hard deadline behind it, where a MIDI callback runs when a hand
//! moves and a late one shows up as a knob that lagged. A lock-free queue with
//! a fixed backing store would close it, and would be fifty lines of ordering
//! argument for a control surface's worth of traffic.
//!
//! Unbounded rather than a ring, and the difference is worth stating: a fader
//! sweep is a few hundred messages a second at the very most, and a frame loop
//! that has stopped draining has larger problems than a queue. What a ring
//! would buy is a bound on memory if nothing ever drained again; what it would
//! cost is deciding which message to drop, and the answer for a control surface
//! is "none of them" — the last value of a fader is the fader's position, and a
//! dropped one leaves it somewhere the operator is not holding it.
//!
//! ## A frame loop that sleeps has to be woken
//!
//! [`Port::drain`] never waits, which is right for a caller that renders every
//! frame anyway — `karakuri-cli` is one, and it asks on every pass. **A caller
//! that sleeps until something happens is the other case**, and for it a
//! message arriving is an event exactly as a key press is: nothing else is
//! going to wake it, so a knob turned on a still panel would be applied
//! whenever the operator next moved the mouse.
//!
//! So [`Port::waking`] takes a closure and calls it once per message, on the
//! MIDI thread, immediately after the send. **It says *something arrived* and
//! carries nothing**, which is what keeps this crate free of whoever is
//! listening: the panel hands in an `EventLoopProxy`'s wake and this crate
//! never learns that a window exists.
//!
//! **It is one more thing the callback does**, and the paragraph above is the
//! standard it is held to rather than an exemption from it: a wake is a write
//! to whatever the caller's loop blocks on, which is the same order of cost as
//! the `send` beside it and is bounded by the same rate — a hand moving.
//!
//! ## Latency is not compensated here
//!
//! A knob move is an operator's hand, so it is *already* where they want it by
//! the time it arrives; there is nothing to lead. That is the opposite of the
//! audio path, where the analysis lag and the output lag both have to be led —
//! see `karakuri-audio`'s `lock` module. Nothing here has a clock at all.

use std::sync::mpsc::{self, Receiver, Sender};

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

use crate::Message;

/// An open MIDI input.
///
/// Holds the connection alive: dropping this closes the port, which is why it
/// is returned rather than leaked, even though nothing reads its fields.
pub struct Port {
    _connection: MidiInputConnection<Callback>,
    messages: Receiver<Message>,
    name: String,
}

/// **What the MIDI thread is handed**: where to put a message, and who to tell.
///
/// A struct rather than the bare `Sender` this used to be, because there are
/// now two things to do per message and `midir` carries exactly one value into
/// the callback. See *A frame loop that sleeps has to be woken* above for why
/// the second is not a second channel.
struct Callback {
    messages: Sender<Message>,
    /// **Called once per message, on the MIDI thread**, or `None` for a caller
    /// that is going to ask anyway. Boxed because this crate must not know
    /// what a wake *is*: the panel's is an event-loop proxy and
    /// `karakuri-cli`'s is nothing at all.
    wake: Option<Box<dyn Fn() + Send>>,
}

impl Port {
    /// Open the first input whose name contains `wanted`, case-insensitively,
    /// or the first input at all when `wanted` is empty.
    ///
    /// A substring rather than an exact name because the name a MIDI port
    /// reports is the manufacturer's and often carries a port number and a
    /// bus — "nanoKONTROL2 SLIDER/KNOB" is one real example — and asking an
    /// operator to type that exactly is asking them to run the tool once to
    /// find out what to type.
    ///
    /// The error names every port there was, for the same reason.
    ///
    /// **Nothing is woken.** This is the constructor for a caller that drains
    /// every frame regardless — see [`Port::waking`] for the other one.
    pub fn open(wanted: &str) -> Result<Port, String> {
        Port::opened(wanted, None)
    }

    /// [`Port::open`], and `wake` is called once per message on the MIDI
    /// thread, right after it is queued.
    ///
    /// **For a caller whose loop sleeps.** A message arriving is the only
    /// thing that can tell such a loop there is anything to drain, and a knob
    /// turned on a panel nobody is touching would otherwise be applied at
    /// whatever the next mouse move was — which is a control that silently
    /// does nothing, and is what
    /// `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`
    /// rules out.
    ///
    /// **The wake carries nothing and answers nothing**, which is what keeps
    /// this crate ignorant of who is listening: it says *ask again*, and what
    /// the caller does about it is the caller's. A failure to wake is dropped
    /// for the send's reason — a loop that has gone is a run that is ending.
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
                    // Parsed here rather than in the frame, so what crosses the
                    // channel is a message rather than a buffer — no allocation
                    // on the frame side, and nothing to reassemble.
                    //
                    // The send's error is dropped: it means the receiver is
                    // gone, which means the run is ending, and a MIDI callback
                    // is the last place to report that.
                    if let Some(message) = Message::parse(bytes) {
                        let _ = back.messages.send(message);
                        // **After the send and never before it**, so the loop
                        // this wakes finds the message already queued rather
                        // than draining nothing and going back to sleep.
                        if let Some(wake) = back.wake.as_ref() {
                            wake();
                        }
                    }
                },
                Callback { messages: tx, wake },
            )
            .map_err(|e| format!("could not open `{name}`: {e}"))?;

        Ok(Port {
            _connection: connection,
            messages,
            name,
        })
    }

    /// Every message that has arrived since the last call. Never waits.
    ///
    /// Drains rather than taking one, because a fader sweep delivers several
    /// between two frames and acting on one of them a frame would put the
    /// fader where the operator's hand was rather than where it is.
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

/// **An open MIDI output, written to from a thread of its own.**
///
/// The other direction, and it is a different problem from [`Port`]'s. An
/// input hands this process bytes on somebody else's thread and the frame
/// drains them; an output is written to *by the frame*, and
/// `MidiOutputConnection::send` is a system call into a driver that this
/// process does not own the timing of. **Nothing on the frame path waits**
/// (`docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`),
/// so the frame does not make that call.
///
/// ## The shape is ADR-0067's, one stream along
///
/// A **bounded** channel to a thread that owns the connection, and a frame
/// that finds it full **drops and counts** rather than blocking or growing:
///
/// - **Bounded**, because the queue is a picture of one moment. A surface's
///   LEDs and faders show where the deck is *now*, so a backlog is a fader
///   travelling through positions it was already past — unlike a session
///   stream, where every record is owed.
/// - **Dropped rather than blocked**, because the alternative is the render
///   thread waiting on a driver. What a drop costs is one stale control until
///   the next change moves it, and the caller re-states a control whenever the
///   deck changes it.
/// - **Counted**, because a surface that quietly stopped following the deck is
///   the failure nobody notices — `docs/adr/0067-the-session-writer-never-blocks-never-grows-and-never-silently-drops.md`
///   is the record and [`Out::dropped`] is the count, said once by whoever
///   holds this.
///
/// **The connection is opened on the sender's own thread** rather than moved
/// on to it, and the result is carried back over a one-shot channel: the
/// caller still finds out why a port would not open, and nothing here depends
/// on a `midir` connection being `Send`.
pub struct Out {
    to: mpsc::SyncSender<[u8; 3]>,
    name: String,
    dropped: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

/// **How many messages may be in flight**, and it is a whole surface's worth
/// several times over.
///
/// A frame writes at most one message per mapped control that changed, and a
/// map is single figures to a few dozen lines; a pair is two. So this is full
/// only when the sender thread has stopped making progress at all — a driver
/// that has stopped taking bytes, or a device unplugged mid-set — which is
/// exactly the case a bound exists for.
const QUEUE: usize = 256;

impl Out {
    /// Open the output whose name contains `wanted`, case-insensitively, or
    /// `Err` naming every output there was.
    ///
    /// A substring for [`Port::open`]'s reason exactly, and it matters more
    /// here: a device's input and output ports are named by the manufacturer
    /// and often differ past the first word — "nanoKONTROL2 SLIDER/KNOB" in
    /// and "nanoKONTROL2 CTRL" out — so whoever pairs an output with an input
    /// hands in as much of the name as the two share.
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
                    // **The only place in this process that writes MIDI.**
                    // `recv` blocks, which is the point: this thread waits so
                    // that the frame never does. A send that fails is a port
                    // that has gone, and a MIDI callback is the last place to
                    // report that — the drop count is what says so.
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
            to,
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

    /// **Queue one message. Never waits.**
    ///
    /// `true` if it was queued and `false` if it was dropped — a full queue or
    /// a sender thread that has gone. A dropped message is counted either way,
    /// so a caller that ignores the answer still has [`Out::dropped`] to say
    /// it with.
    pub fn send(&self, message: [u8; 3]) -> bool {
        match self.to.try_send(message) {
            Ok(()) => true,
            Err(_) => {
                self.dropped
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                false
            }
        }
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

    /// **A full queue drops rather than blocking**, which is the property the
    /// bound exists for and the one that cannot be checked with a device
    /// attached: what it is about is a sender thread that has stopped taking
    /// bytes.
    ///
    /// The queue with nothing draining it stands in for that exactly — no
    /// port, no thread, and the same `SyncSender` the frame writes to. A test
    /// that hangs here is the failure it is looking for.
    #[test]
    fn a_full_queue_drops_rather_than_blocking() {
        let (to, held) = mpsc::sync_channel::<[u8; 3]>(QUEUE);
        let out = Out {
            to,
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
}
