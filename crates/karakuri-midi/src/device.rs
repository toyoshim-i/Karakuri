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
//! ## Latency is not compensated here
//!
//! A knob move is an operator's hand, so it is *already* where they want it by
//! the time it arrives; there is nothing to lead. That is the opposite of the
//! audio path, where the analysis lag and the output lag both have to be led —
//! see `karakuri-audio`'s `lock` module. Nothing here has a clock at all.

use std::sync::mpsc::{self, Receiver, Sender};

use midir::{MidiInput, MidiInputConnection};

use crate::Message;

/// An open MIDI input.
///
/// Holds the connection alive: dropping this closes the port, which is why it
/// is returned rather than leaked, even though nothing reads its fields.
pub struct Port {
    _connection: MidiInputConnection<Sender<Message>>,
    messages: Receiver<Message>,
    name: String,
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
    pub fn open(wanted: &str) -> Result<Port, String> {
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
                |_stamp, bytes, tx: &mut Sender<Message>| {
                    // Parsed here rather than in the frame, so what crosses the
                    // channel is a message rather than a buffer — no allocation
                    // on the frame side, and nothing to reassemble.
                    //
                    // The send's error is dropped: it means the receiver is
                    // gone, which means the run is ending, and a MIDI callback
                    // is the last place to report that.
                    if let Some(message) = Message::parse(bytes) {
                        let _ = tx.send(message);
                    }
                },
                tx,
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
