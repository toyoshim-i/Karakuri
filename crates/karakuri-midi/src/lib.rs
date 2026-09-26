//! MIDI control surface input handling and translation to declarative operations.
//! Captures MIDI wire messages, parses them into structured messages, and
//! translates them to operations via user `.kmap` tables.

mod device;
mod map;
mod message;

pub use device::{Out, Port};
pub use map::{Control, Echo, Half, Map, Parameter, Shown, Wide};
pub use message::Message;
