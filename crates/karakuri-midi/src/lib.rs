//! MIDI in: a control surface driving the deck through the record stream.
//!
//! `docs/roadmap.md`, M2: "MIDI control surface on a dedicated controller, not
//! the DJ controller." The reason it is a separate surface is that the DJ
//! controller belongs to whoever is playing the music, and the visuals cannot
//! be on a device somebody else's hands are on.
//!
//! ## Why this crate knows nothing about the engine
//!
//! It produces an [`Action`] — "the operator asked for gain 0.7 on slot 2" —
//! and stops. `karakuri-cli` turns that into the same `gain` record a keypress
//! writes, and the engine is driven through that.
//!
//! **That is the invariant, not an arrangement.** `docs/invariants.md`'s record stream
//! is the sole mutation path, and the reason M6's agents will be safe to run is
//! that they can do nothing a human could not do through the same interface. A
//! surface is the first thing to test that claim against, because a surface is
//! the first thing that is not the keyboard. Every consequence follows from it:
//! a session recorded from a controller **replays with no controller
//! attached**, an unmapped knob cannot reach anything, and there is no control
//! a surface can move that a key cannot.
//!
//! It also settles where the map lives. The map is **not** in the session
//! stream: which knob is which is a property of the hardware in the room, and
//! replaying one room's wiring in another is not replaying a performance. It is
//! the same argument `residency` makes about the *effective* level, from the
//! other end.
//!
//! ## The split
//!
//! The same shape as `karakuri-audio`, and for the same reason: everything that
//! decides anything is a pure function, tested against bytes rather than
//! against a device.
//!
//! - [`Message`] is the wire, parsed. Three message kinds, and everything else
//!   ignored rather than misread.
//! - [`Map`] is the operator's table, and turns a message into an [`Action`].
//! - `device` opens a port and pushes bytes across a channel. It is the only
//!   part that cannot be tested without hardware, and it is deliberately the
//!   part with nothing in it.

mod device;
mod map;
mod message;

pub use device::Port;
pub use map::{Action, Map};
pub use message::Message;
