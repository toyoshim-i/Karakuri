//! Unified command vocabulary and grammar for the Karakuri visual performance system.
//! Strongly typed [`Operation`] enumeration serving as the single source of truth across
//! panel controls, hotkeys, MIDI bindings, and AI MCP pair-programming tools.

pub mod gate;
pub mod op;
pub mod types;

pub use op::*;
pub use types::*;
