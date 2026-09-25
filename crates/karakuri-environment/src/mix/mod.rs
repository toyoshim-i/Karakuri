//! Mix state translation, recording, and application.
//! Bridges live performance gestures to engine updates via [`karakuri_operation_record::Record`].
//! Continuous controls coalesce per frame (ADR-0207, Principle 0091).

pub mod chain;
pub mod change;
pub mod current;
pub mod shipped;
pub mod translate;

pub use chain::*;
pub use change::*;
pub use current::*;
pub use translate::*;

#[cfg(test)]
mod tests;
