//! Shared application core, state management, and rendering pipelines for Karakuri.

use std::time::Duration;

use karakuri_engine::deck::MAX_SLOTS;
pub use karakuri_mcp as mcp;

pub mod session;
pub(crate) use session::{rewired, watched, Keeping};

pub mod keymap;

/// The window this opens, in logical pixels. Comfortably above the smallest
/// viewport the arrangement is claimed to work at, so nothing starts clamped.
pub const WINDOW: (f64, f64) = (1440.0, 900.0);

pub mod readout;
pub(crate) use readout::*;

// ---------------------------------------------------------------------------
// The engine in the Program bay
// ---------------------------------------------------------------------------

/// Default canvas resolution (1280x720) used as the starting session frame canvas and aspect ratio reference (ADR-0246, ADR-0247, ADR-0270).
pub const CANVAS: (u32, u32) = (1280, 720);

/// Which profile this binary was built with, for the legend's own reading.
#[allow(dead_code)]
pub const PROFILE: &str = match cfg!(debug_assertions) {
    true => "debug profile with dependencies at opt-level 3",
    false => "release profile",
};

/// Initial RNG seed salt for Deck A.
pub const SEED_SALT: u32 = 7;

/// Maximum slot capacity for the deck, matching [`MAX_SLOTS`] (ADR-0178).
///
/// The deck opens with slot 0 on air and remaining slots muted.
pub const SLOTS: usize = MAX_SLOTS;

/// Slot index opened on air at launch.
pub const ON_AIR: usize = 0;
#[allow(dead_code)]
pub const ASKED_TO_PRIME: usize = 1;

/// Command-line argument parsing and working-copy setup for launch procedure pairs (ADR-0230).
pub mod launch;
pub use launch::*;

/// Periodic poll interval (100 ms) for event loop wakeups when MCP serving is active (ADR-0164).
pub const SERVED: Duration = Duration::from_millis(100);

/// Pre-allocated buffer capacity for draining MIDI operations per frame without allocations (P-0091).
pub const MAPPED: usize = 32;

// Note: Simulation step counts are measured dynamically per frame via App::clock (ADR-0297).

pub mod bridge;
pub(crate) use bridge as engine_bridge;
pub(crate) use bridge::*;

pub mod gfx;
pub(crate) use gfx::*;

pub mod app;
pub use app::*;

#[cfg(test)]
mod tests;
