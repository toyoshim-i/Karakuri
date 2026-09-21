//! Desktop instrument application hosting the Karakuri console UI and engine.
//!
//! # Usage
//! ```sh
//! cargo run -p karakuri                                # default shipped preset pair
//! cargo run -p karakuri -- geometry.kir renderer.kir   # custom L1 and L4 pair
//! ```
//!
//! # Architecture
//! - Combines `karakuri-console` UI rendering via `wgpu`/`egui` with `karakuri-engine`'s GPU render pipelines.
//! - Runs event loop via `winit`, repainting when UI changes or new frames are required (ADR-0164).
//! - Integrates audio input, MIDI control surface, MCP bridge, and session recording through `karakuri-environment`.
//! - Tracks memory allocation overhead via global counting allocator for latency budget profiling (ADR-0164, ADR-0217).

use std::time::Duration;

use karakuri_engine::deck::MAX_SLOTS;
use karakuri_environment::{Opening, SlotPolicies};
use karakuri_mcp as mcp;
use karakuri_store::store::Store;
use winit::event_loop::{ControlFlow, EventLoop};

mod session;
pub(crate) use session::{rewired, watched, Keeping};

/// The window loop's own keyboard: [`keymap::KEY_BINDINGS`], the [`KeyCtx`] its
/// actions take, and [`keymap::key_column`], the unit test that holds it
/// against `docs/manual/operations.html`. Split out the same way [`session`]
/// was, one piece of this file's own decomposition along — `window_event`'s
/// dispatch into the table stays here, in `main.rs`.
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
pub(crate) const PROFILE: &str = match cfg!(debug_assertions) {
    true => "debug profile with dependencies at opt-level 3",
    false => "release profile",
};

/// Initial RNG seed salt for Deck A.
pub(crate) const SEED_SALT: u32 = 7;

/// Maximum slot capacity for the deck, matching [`MAX_SLOTS`] (ADR-0178).
///
/// The deck opens with slot 0 Live and remaining slots idling at [`Residency::Allocated`].
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

// -- where `STEPS_A_FRAME` was ------------------------------------------
//
// **The step count is measured now, and the constant that stood here said the
// opposite until 2026-09-08.** Its documentation read: *"P-0092 says simulation
// time comes from a record and never from a clock. There is no record here —
// this program is not a session — so the honest third option is neither: a
// fixed count per frame … It is not a performance, and a `karakuri-cli` that
// measured an interval and wrote a `tick` is what a performance is."* **Both
// halves of that were wrong.**
//
// **P-0092 does not say *never from a clock*.** Its first sentence is *"Time
// comes from a record: live, the engine derives the step count from real time
// and writes it in; replaying, it reads the number back and derives nothing"* —
// the clock read is the **live** path of the rule, not a thing the rule
// forbids. What it forbids is a clock reached from inside the simulation, and a
// replay deriving the count again. A fixed count is not a third option between
// those; it is a refusal to take the measurement the record is shaped to carry.
//
// **And *this program is not a session* stopped being true with ADR-0289**,
// which gave the transport row a `rec` toggle that opens a recorder and writes
// a stream from this window.
//
// **What it cost was not about recordings.** `DT` is 1/60 s, this window is
// `PresentMode::Fifo`, and `karakuri_signal`'s oscillator advances by
// simulation steps and never by wall clock — so one step per frame *drawn* made
// simulation time advance at the display's refresh rate divided by sixty.
// Correct at 60 Hz by coincidence, **double speed on a 120 Hz display**, and
// 0.6755x on a console with every sink folded and the beat declaring at
// `BEAT_STALENESS`. `README.md` promises the oscillator follows the room, and
// the beat grid is that oscillator.
//
// The live count is `App::clock` — `karakuri_environment::clock::Clock`, the
// derivation `karakuri-cli` already made, moved to where ADR-0215 said it goes.
// See `docs/adr/0297-the-panels-tick-is-measured-and-the-fixed-step-a-frame-ran-the-room-at-the-displays-rate.md`.
//
// **This is a comment and not a `#[cfg(test)]` constant**, which is what it
// briefly was: several tests below scan this file for its own code and bound
// the scan at *the first `#[cfg(test)]`*, so a test-only item up here silences
// every one of them — five failed at once and said so. The fixture the tests
// still want is `gpu::STEPS_A_FRAME`, beside the frames that use it.

mod bridge;
pub(crate) use bridge as engine_bridge;
pub(crate) use bridge::*;

mod gfx;
pub(crate) use gfx::*;

mod app;
pub(crate) use app::*;

/// The command line, then the window.
///
/// The arguments are read *before* the event loop exists, so a refusal is a
/// line on stderr and an exit code rather than a window that opens and closes.
/// `skip(1)` drops the program's own name, which is `std::env::args`'s first
/// element and not an argument.
fn main() {
    let launch = match sources_from(std::env::args().skip(1)) {
        Ok(launch) => launch,
        // An empty message is `--help`, which is a request rather than a
        // mistake: the usage goes to stdout and the exit is 0.
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
    // **Before the window and before a device**, which is where a failure can
    // still be a sentence on a terminal: this reads two files and writes four
    // pairs, and everything after it is inside a `winit` callback where a
    // panic aborts without a message (see `resumed`). It is also the honest
    // place for it — the copies depend on the command line and on nothing
    // else.
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
    // **Opened here, beside the copies and for their reason.** Every build a
    // watcher makes puts its sources in this store, which is what gives a deck
    // an address a Set file can name — see [`Playing`]. It creates four
    // directories under a root this run has already written into, so it is not
    // a promise the copies above did not already make; it is fatal for the same
    // reason they are, because a run whose store will not open is a run that
    // cannot keep anything.
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
    // **Every version this run compiles, kept where a person can find it**, and
    // the version every deck starts on filed before the window opens — see
    // [`seeded`], which carries the whole of why.
    let snapshots = seeded(&launch.store, &running);
    // **What every surface in this run reads and no surface decides**, made
    // here so that there is one of it: the four bay-head pills write it and
    // the MCP server reads it on every call, and a second handle would be a
    // pill that opens a class the server never sees. All four classes start
    // shut, which is the state ADR-0235 says a run starts in.
    let opening = Opening::closed();
    let slot_policies = SlotPolicies::new();
    // **Which files each deck is running, and the one handle that answers it**
    // — see [`karakuri_mcp::Slots`], and [`Aiming::pointing`] for
    // what writes it. Made here beside the opening and for the same reason:
    // the MCP server binds before the window and reads through it on every
    // call, every watcher this run makes publishes its own slot into it, and a
    // second one would be a load the server never sees.
    //
    // **One pair per deck, and they are the working copies rather than the two
    // paths the operator typed.** The server addresses a slot and reads and
    // writes the files behind it, and the files behind a deck are its own copy
    // — see [`working_copies`]. Handing it the typed paths would let a model
    // rewrite the preset library.
    //
    // **Seeded here because the socket is bound before there is a watcher**,
    // and this is what every watcher is about to be pointed at: `watched`
    // builds each slot's launch aim out of this same `running`, and
    // [`Aiming::new`] restates it into this handle the moment the window opens.
    let pointing = mcp::Slots::of(
        running
            .iter()
            .map(|pair| (pair.l1.clone(), vec![pair.l4.clone()]))
            .collect(),
    );
    // **Before the window, for the reason the working copies are**: `serve`
    // binds a socket, and a socket that is already taken has to be a sentence
    // on a terminal. Everything after `run_app` is inside a `winit` callback,
    // where a panic aborts without a message.
    //
    // **Fatal, because `--mcp` was asked for.** A run that went on without it
    // would look exactly like one whose client is connected and idle.
    let mcp = match launch.mcp {
        Some(port) => {
            match mcp::serve(
                port,
                // **Shared and not copied**, which is the whole of the fix:
                // the server resolves an address through this on every call,
                // so a library load that re-points a deck moves what a model
                // reads and writes with it.
                pointing.clone(),
                launch.store.clone(),
                // **True, and not a flag read from anywhere.** Every slot in
                // this program is built over a `watch::Watch` ([`watched`]) and
                // there is no run of this binary that opens a file read-only,
                // so a procedure a model writes is always picked up. This is
                // where that fact is stated to the server, which uses it to
                // tell a client whether a write will reach the screen.
                true,
                opening.clone(),
                slot_policies.clone(),
            ) {
                Ok(reporter) => {
                    // The port bound rather than the one asked for: `--mcp 0`
                    // takes an ephemeral one, and printing the 0 would name a
                    // port that is not the port.
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
    // **Before `run_app`, because that is the last moment the loop is a value
    // rather than a callback.** It is what the MIDI callback wakes this window
    // with — see [`App::waker`], which carries the whole argument for why a
    // control surface is a wake and not a poll.
    let waker = event_loop.create_proxy();
    // **The loop sleeps.** A frame is drawn when something changed it or when
    // `egui` asked for one after a delay it named, and on no other occasion —
    // `App::about_to_wait` sets this again after every iteration and is where
    // the rule actually lives. This is the state it starts in so that the
    // window between here and the first `about_to_wait` is not a spin either.
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
