//! MIDI control surface input handling and translation to declarative operations.
//!
//! # Architecture
//!
//! `karakuri-midi` captures incoming MIDI wire messages, parses them into structured
//! [`Message`] events, and matches them against user `.kmap` tables to emit pure
//! [`karakuri_operation::Operation`] commands.
//!
//! Hardware bindings are kept separate from the engine:
//! - Hardware mapping maps messages into standard system operation vocabulary.
//! - Mutations flow into the record stream, enabling full deterministic session replay
//!   without hardware attached.
//! - Map files describe local physical controller layouts and are not serialized into
//!   portable session archives.
//!
//! # Submodules
//!
//! - [`message`]: Pure, zero-allocation MIDI packet decoding.
//! - [`map`]: Domain mapping table translating control changes/notes into operations.
//! - [`device`]: Hardware port interface wrapping `midir`.

mod device;
mod map;
mod message;

pub use device::{Out, Port};
pub use map::{Control, Echo, Half, Map, Parameter, Shown, Wide};
pub use message::Message;
