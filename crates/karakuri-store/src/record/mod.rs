//! ndjson records used across Set files (`.kbset`), session streams, and artifact metadata.
//! Represents declarative state, timeline ticks, parameter assignments, and node topology.

pub mod helpers;
pub mod types;
pub mod variants;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use helpers::*;
pub use types::*;
pub use variants::*;
