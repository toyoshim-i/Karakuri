//! Desktop instrument hosting the Karakuri console UI and engine.
//!
//! Runs `winit`/`egui`/`wgpu` pipelines with audio, MIDI, MCP, and session recording (ADR-0164, ADR-0217).

use std::time::Duration;

use karakuri_engine::deck::MAX_SLOTS;
use karakuri_environment::{Opening, SlotPolicies};
use karakuri_mcp as mcp;
use karakuri_store::store::Store;
use winit::event_loop::{ControlFlow, EventLoop};

mod session;
pub(crate) use session::{rewired, watched, Keeping};

/// Keyboard bindings ([`keymap::KEY_BINDINGS`]) and key context handling for the window loop.
mod keymap;

/// The window this opens, in logical pixels. Comfortably above the smallest
/// viewport the arrangement is claimed to work at, so nothing starts clamped.
pub(crate) const WINDOW: (f64, f64) = (1440.0, 900.0);

mod readout;
pub(crate) use readout::*;

// ---------------------------------------------------------------------------
// The engine in the Program bay
// ---------------------------------------------------------------------------

/// Default canvas resolution (1280x720) used as the starting session frame canvas and aspect ratio reference (ADR-0246, ADR-0247, ADR-0270).
pub(crate) const CANVAS: (u32, u32) = (1280, 720);

/// Which profile this binary was built with, for the legend's own reading.
#[allow(dead_code)]
pub(crate) const PROFILE: &str = match cfg!(debug_assertions) {
    true => "debug profile with dependencies at opt-level 3",
    false => "release profile",
};

/// Initial RNG seed salt for Deck A.
pub(crate) const SEED_SALT: u32 = 7;

/// Maximum slot capacity for the deck, matching [`MAX_SLOTS`] (ADR-0178).
///
/// The deck opens with slot 0 on air and remaining slots muted.
pub(crate) const SLOTS: usize = MAX_SLOTS;

/// Slot index opened on air at launch.
pub(crate) const ON_AIR: usize = 0;
#[allow(dead_code)]
pub(crate) const ASKED_TO_PRIME: usize = 1;

/// Command-line argument parsing and working-copy setup for launch procedure pairs (ADR-0230).
mod launch;
pub(crate) use launch::*;

/// Periodic poll interval (100 ms) for event loop wakeups when MCP serving is active (ADR-0164).
pub(crate) const SERVED: Duration = Duration::from_millis(100);

/// Pre-allocated buffer capacity for draining MIDI operations per frame without allocations (P-0091).
pub(crate) const MAPPED: usize = 32;

// Note: Simulation step counts are measured dynamically per frame via App::clock (ADR-0297).

mod bridge;
pub(crate) use bridge as engine_bridge;
pub(crate) use bridge::*;

mod gfx;
pub(crate) use gfx::*;

mod app;
pub(crate) use app::*;

/// Application entry point: parses CLI arguments, initializes scratch storage and MCP, then runs the event loop.
fn main() {
    let launch = match sources_from(std::env::args().skip(1)) {
        Ok(launch) => launch,
        // Print usage and exit cleanly on `--help`.
        Err(why) if why.is_empty() => {
            println!("{USAGE}");
            return;
        }
        Err(why) => {
            eprintln!("{why}");
            eprintln!();
            eprintln!("{USAGE}");
            std::process::exit(2)
        }
    };
    // Create per-slot working copies in scratch storage before initializing the window/GPU.
    // This isolates live edits from preset source files (P-0096) and enables early CLI exit on failure.
    let (scratch, running) = match working_copies(&launch.store, &launch.sources, SLOTS) {
        Ok(made) => made,
        Err(why) => {
            eprintln!("{why}");
            eprintln!();
            eprintln!(
                "that is the material this run was told to play, and the deck runs from \
                 copies of it — so nothing was built and nothing was written."
            );
            std::process::exit(1)
        }
    };
    println!("{}", running_from(&scratch, &running));
    // Open the artifact store in the configured root directory.
    let held = match Store::open(&launch.store) {
        Ok(store) => std::sync::Arc::new(store),
        Err(why) => {
            eprintln!("store `{}`: {why}", launch.store.display());
            eprintln!();
            eprintln!(
                "that is where every deck's copies were just written and where a save would \
                 go, so nothing was built."
            );
            std::process::exit(1)
        }
    };
    // Seed initial snapshots for each running deck in the store.
    let snapshots = seeded(&launch.store, &running);
    // Model Control Protocol (MCP) permission gate: all operation classes start closed (ADR-0235).
    let opening = Opening::closed();
    let slot_policies = SlotPolicies::new();
    // Shared slot mapping for MCP pointing to per-deck working copies in scratch storage.
    let pointing = mcp::Slots::of(
        running
            .iter()
            .map(|pair| (pair.l1.clone(), vec![pair.l4.clone()]))
            .collect(),
    );
    // Bind and start MCP server before opening GUI window if `--mcp` was specified.
    let mcp = match launch.mcp {
        Some(port) => {
            match mcp::serve(
                port,
                pointing.clone(),
                launch.store.clone(),
                true,
                opening.clone(),
                slot_policies.clone(),
            ) {
                Ok(reporter) => {
                    println!(
                        "mcp: 127.0.0.1:{} — a model can read and rewrite a deck's procedure, \
                         rewire an input and keep what a deck is playing; every deck is \
                         watched, so a write reaches the screen",
                        reporter.port()
                    );
                    Some(reporter)
                }
                Err(why) => {
                    eprintln!("karakuri: mcp: {why}");
                    std::process::exit(2)
                }
            }
        }
        None => None,
    };
    let event_loop = EventLoop::new().expect("event loop");
    // Proxy waker allowing MIDI callback threads to wake the window event loop.
    let waker = event_loop.create_proxy();
    // Initialize the event loop to Wait mode; frames are driven by inputs, timers, or explicit requests.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop
        .run_app(&mut App::new(
            launch,
            running,
            held,
            snapshots,
            mcp,
            opening,
            pointing,
            waker,
            slot_policies,
        ))
        .expect("run");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
