//! Desktop instrument hosting the Karakuri console UI and engine.
//!
//! Runs `winit`/`egui`/`wgpu` pipelines with audio, MIDI, MCP, and session recording (ADR-0164, ADR-0217).

use karakuri::*;
use karakuri_environment::{Opening, SlotPolicies};
use karakuri_mcp as mcp;
use karakuri_store::store::Store;
use winit::event_loop::{ControlFlow, EventLoop};

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
