//! Unified command vocabulary and grammar for the Karakuri visual performance system.
//!
//! Every operation Karakuri can perform is named once here, so that panel controls,
//! hotkeys, mapped MIDI messages, and AI MCP pair-programming tools route into the
//! exact same strongly typed [`Operation`] enumeration.
//!
//! # Architecture and Principles
//!
//! - **Single Source of Truth**: [`Operation`] defines all allowable actions.
//! - **Zero Dependencies**: Pure leaf crate; contains no GPU, audio, or GUI dependencies.
//! - **Surface Independence**: Operations express target parameters and explicit values,
//!   independent of how the gesture was entered.
//! - **Deterministic Grammar**: Decks and targets are always explicitly addressed.

pub mod gate;
pub mod op;
pub mod types;

pub use op::*;
pub use types::*;
