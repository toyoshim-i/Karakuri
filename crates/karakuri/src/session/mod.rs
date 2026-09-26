//! Session recording and persistence state machines.
//!
//! Provides [`Sessions`] for recording runs and [`Keeping`] for background file
//! and procedure persistence queued by key presses or MCP requests.

pub mod keeping;
pub mod recording;
pub mod watch;

pub(crate) use keeping::Keeping;
pub(crate) use recording::Sessions;
pub(crate) use watch::{rewired, watched};

/// Deck slot whose Set file is carried in full by the session head (slot 0).
pub(crate) const HEAD_SLOT: usize = 0;
