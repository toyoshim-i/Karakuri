//! ndjson records.
//!
//! One record per line, and concatenation is composition. The same record types
//! serve three kinds of files:
//!
//! - Set file (`.kbset`): A persistent state projection of what is loaded and current
//!   parameter values. Contains no [`Record::Tick`].
//! - Session stream: A timeline consisting of initial Set state followed by ticks
//!   and interleaved operations.
//! - Artifact metadata (`<hash>.meta.ndjson`): Declarations extracted from a procedure's
//!   source and compilation output.
//!
//! The vocabulary uses dedicated types for disambiguated slot concepts:
//! - [`Record::Slot`]: A node in a Set at `(layer, index)`.
//! - [`DeckSlot`]: An index into mixer decks.
//! - [`InputPort`]: A named input port on a node.

pub mod helpers;
pub mod types;
pub mod variants;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use helpers::*;
pub use types::*;
pub use variants::*;
