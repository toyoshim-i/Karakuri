//! The V1 entry point.
//!
//! One to [`MAX_SLOTS`] pairs of `.kir` files go in, each through parse, type
//! and contract checking, cost estimation, WGSL generation and pipeline
//! creation, and come out as either a window or a PNG. Nothing here is
//! hand-written shader code.
//!
//! Every pair becomes one deck slot with its own `HotSwap`, and the deck
//! composites the Live ones. A window is therefore an instrument rather than a
//! demo: the keyboard puts slots on and off air, moves their gain, and cycles
//! the tone mapper — see [`BINDINGS`], which is both what `--help` prints and
//! what the window prints at startup.
//!
//! This is also the only place a clock is read. The engine advances by `steps`
//! from a `tick` record and never measures anything; deriving that count from
//! elapsed real time is the job of whoever drives the engine live, and on
//! replay it is read back from the stream instead. Keeping the measurement out
//! here is what lets the same engine code be deterministic.

// **The program is not this binary's**, and there is no `mod` line above this
// one any more: all thirteen modules moved to `karakuri-environment` under
// ADR-0214 and ADR-0215, and they are reached here by name so that every call
// site below reads exactly as it did. What is left in this file is the window,
// the arguments, the key handler and `Live` — a surface over them.
/// The step count this run's frames advance by, and it is
/// `karakuri-environment`'s rather than this file's since ADR-0297: a frame's
/// step count is the live half of P-0092 and there is one derivation of it, now
/// that two programs make it. ADR-0215 had already decided the placement —
/// *"`Clock` moves and `Live` does not"* — and this is the move.
use clock::Clock;
use karakuri_environment::{
    audio, clock, compile, history, midi, mix, places, render, scratch, session, setfile,
    tempo_source, watch,
};
// The MCP server now lives in its own crate. Aliased to `mcp` so every
// `mcp::` call site below is unchanged.
use karakuri_mcp as mcp;
// **Brought into scope rather than reached through their modules**, because
// each was written here and every call site below is the one it already was.
// `Names` crossed with `setfile` in the second slice; the rest crossed with the
// watcher, the MCP server, the mixer and the MIDI map — the material types
// those read a slot off a disk through, the layer spelling a Set file and a
// `--param` share, the save path's own values, the operator's name for a tone
// map, and the sentences P-0090 says belong where every surface can reach them.
// Four more are reached only from the tests below and are brought in there.
use karakuri_environment::compile::{sort_slot, Material, Named, Placed};
use karakuri_environment::mix::{op_name, op_wire_names, parse_op};
use karakuri_environment::setfile::{layer_named, Names, SavedNode, Sources};
use karakuri_environment::{
    accepted_save, no_such_param, no_such_renderer, no_such_slot, nothing_to_save, Asked, SAVE_WAIT,
};

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use karakuri_engine::binding::{Curve, CONTROL_PREFIX, CURVES, DEFAULT_BPM, NOISE_SIGNAL};
use karakuri_engine::deck::MAX_SLOTS;
use karakuri_engine::frame;
use karakuri_engine::swap::Event;
use karakuri_engine::transport::Sync;
use karakuri_engine::{
    Binding, Blend, Deck, DeckSlot as EngineSlot, Gpu, HotSwap, Look, MaskKind, ParamWrite,
    Present, Residency, Set, Signals, TonemapOp, DEFAULT_BUDGET_MS,
};
use karakuri_operation::Operation;
use karakuri_operation_record::{Current, Written};
use karakuri_signal::NoiseConfig;
use karakuri_store::record::{BindNoise, DeckSlot, Layer, Record};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

mod args;
pub(crate) use args::*;
mod replay;
pub(crate) use replay::*;
mod save;
pub(crate) use save::*;
mod aiming;
pub(crate) use aiming::*;

mod live;
pub(crate) use live::*;
mod app;
pub(crate) use app::*;

/// The fixed simulation step. Not the frame delta.
///
/// The engine's, not a second copy of it: the session oscillator a binding
/// reads advances by this, so a value here that drifted from the engine's would
/// put beats in the wrong place with nothing to show for it.
const DT: f32 = karakuri_engine::set::DT;

pub(crate) const SEED: u32 = 19_274;

/// The same question asked once per source, which is the only form the engine
/// accepts — `Set::build_many` takes `(procedure, capacity)` pairs precisely
/// because each source declares its own range.
///
/// It was asked once and answered for everybody. The build resolved
/// `capacity_for(args, &l1[0])` and handed that one number to every source, so
/// a second geometry ran at the first one's count with nothing printed: a grid
/// written for 512 x 256 samples and declared at 131072 drew 32768 of them
/// because it was loaded beside a cube. Nothing refused it either — the number
/// came from a declaration, so it was inside somebody's range, just not the
/// range of the procedure it was applied to.
///
/// `recorded` is what a Set file said, per geometry, and it wins where it said
/// anything: the file is where that geometry's count was decided, and a Set
/// that came back at a different size is a Set that was not saved. Empty for a
/// slot no file filled, which is every slot but slot 0.
fn capacities_for(
    args: &Args,
    l1s: &[karakuri_ir::typed::Checked],
    recorded: &[Option<u32>],
) -> Vec<u32> {
    l1s.iter()
        .enumerate()
        .map(|(at, l1)| {
            recorded
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| capacity_for(args, l1))
        })
        .collect()
}

/// Whether anything in this run can write a `.kir`. `--watch` and `--mcp` are
/// the two things that edit a procedure; a render or a replay opens every file
/// read-only.
///
/// A function rather than a `let`, because three things turn on it and they
/// have to turn on the same one: what gets copied into the scratch, what gets
/// compiled out of `args.sets` rather than out of the Set file directly, and —
/// since a rebuilt slot's sources are what a live save writes — whether the
/// watchers put what they build into the store. The third arrived on the far
/// side of `main`, in `App::resumed`, which is what made the copy worth
/// removing.
pub(crate) fn editable(args: &Args) -> bool {
    args.watch || args.mcp.is_some()
}

fn main() {
    let args = parse_args();

    // **A Set file replaces the paths and the flags together**, because it
    // carries both: the material and everything the flags were standing in for
    // until it existed. Handled before anything is compiled, so a run either
    // takes its material from a file or from the command line and never from
    // half of each.
    let mut args = args;
    let loaded = args.load_set.clone().map(|id| load_set(&mut args, &id));

    // **Before anything reads a `.kir`**, so that the deck, the watcher and the
    // MCP surface all get the same rewritten paths from one place — they each
    // read `args.sets` and none of them has to know this happened.
    //
    // Only when the run can write one. `--watch` and `--mcp` are the two things
    // that edit a procedure; a render or a replay opens every file read-only,
    // so copying would leave a directory behind for a run that is supposed to
    // be a function of its arguments. See `karakuri-environment`'s `scratch.rs`
    // for the rest of the reasoning, including what happened when there was no
    // such place.
    // The run's edit history, seeded below from the scratch. `None` for a run
    // that cannot be edited, which is the same condition the scratch has and
    // for the same reason: nothing writes a `.kir`, so there is no version to
    // preserve and no directory to leave behind.
    let mut snapshots: Option<history::Shared> = None;
    let editable = editable(&args);
    if editable {
        let root = args.store.clone();
        // A loaded Set names its procedures by hash and has no file anywhere,
        // so it is written into the scratch first and then joins `args.sets` as
        // ordinary material. Everything downstream — the compile, the watcher,
        // the MCP surface — then treats it exactly like a `--set` chain, which
        // is what makes a saved Set editable rather than only playable.
        if let Some(loaded) = &loaded {
            // Every node the file named, in node order — which starts with the
            // geometries, so the first path is the L1 the pair below wants and
            // the rest are sorted by their own `kind` like any `--set` list.
            // **Slot 0's own names**, because the pair below is inserted at
            // the head of `args.sets` and a scratch file carries the slot it
            // belongs to — the same `A0-drift.kir` `materialise` writes a few
            // lines down and `crates/karakuri`'s `loading` writes on a library
            // load. One rule for one directory: `scratch::node_name`.
            let written: Result<Vec<PathBuf>, String> = loaded
                .nodes()
                .enumerate()
                .map(|(at, (checked, src))| {
                    scratch::place(&root, &scratch::node_name(0, at, &checked.name), src)
                })
                .collect();
            match written {
                // **Each path keeps the name the file gave that node**, so a
                // loaded Set that is then edited resolves its edges against the
                // spellings it was saved with rather than against the procedure
                // names the scratch happens to file it under.
                Ok(paths) => {
                    let mut named = paths
                        .iter()
                        .cloned()
                        .zip(loaded.node_names())
                        .map(|(path, name)| Named { name, path });
                    let head = named.next().expect("a Set file names at least an L1");
                    args.sets.insert(0, (head, named.collect()))
                }
                Err(e) => {
                    eprintln!("karakuri-cli: {e}");
                    std::process::exit(1);
                }
            }
        }
        // **Slot by slot rather than one flat list**, because the name of a
        // copy carries the slot it belongs to: the same file given to two
        // slots becomes two files, so an edit moves the slot whose file it is
        // and no other. See `scratch.rs`'s header for the rule this replaced.
        match scratch::materialise(
            &root,
            args.sets.iter_mut().map(|(l1, l4s)| {
                std::iter::once(&mut l1.path).chain(l4s.iter_mut().map(|n| &mut n.path))
            }),
        ) {
            Ok(dir) => {
                eprintln!(
                    "scratch: {} — every slot runs from its own copy here, so the files you \
                     named are not written to. Point an editor at these",
                    dir.display()
                );
                // **What each slot is actually watching, named.** One preset
                // given to four slots is four files now, which is the point
                // and is also the thing an operator cannot guess: the line
                // above says where, and this says which is whose.
                for (slot, (l1, l4s)) in args.sets.iter().enumerate() {
                    let names: Vec<String> = std::iter::once(&l1.path)
                        .chain(l4s.iter().map(|n| &n.path))
                        .map(|p| {
                            p.file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| p.display().to_string())
                        })
                        .collect();
                    eprintln!("  slot {slot}: {}", names.join(" + "));
                }
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        }
        // **Seeded from the scratch, before the first compile.** The first edit
        // records what replaced the original; without this, what it replaced was
        // never written down and the first edit is the one that cannot be undone.
        let shared = history::Snapshots::shared(&args.store);
        // **The Set each slot is about to run**, which is `--load-set`'s id or
        // nothing at all: this program's other route into a slot is a list of
        // paths, and a pair somebody typed is not a Set. `--load-set` fills
        // slot 0 and is refused alongside `--set`, so it is the only slot that
        // can have one. See `history::Snapshots::record` on why the answer is
        // an `Option` and not a word derived from the file names.
        let playing = |slot: usize| match slot {
            0 => args.load_set.as_deref(),
            _ => None,
        };
        history::seed(
            &shared,
            args.sets.iter().enumerate().map(|(slot, (l1, l4s))| {
                (
                    slot,
                    playing(slot),
                    std::iter::once(l1.path.as_path())
                        .chain(l4s.iter().map(|n| n.path.as_path()))
                        .collect(),
                )
            }),
        );
        snapshots = Some(shared);
    }

    eprintln!("compiling:");
    let mut procs: Vec<Material> = Vec::new();
    // Where each slot's files ended up, slot for slot beside `procs`. **Kept
    // parallel**, empty entry and all, because everything downstream indexes it
    // by slot: a list that skipped the slot a Set file filled would hand slot
    // 0's saver slot 1's files. See [`Placed`].
    let mut placed: Vec<Vec<Placed>> = Vec::new();
    // Only when it was *not* materialised into the scratch above. An editable
    // run compiles it out of `args.sets` with everything else, which is the
    // point: one path, so a loaded Set can be watched and rewritten.
    if let Some(loaded) = loaded.filter(|_| !editable) {
        eprintln!("  slot 0: set `{}`", loaded.id);
        // **The names the file recorded**, which are what its edges are
        // written against — a load that dropped them would be a load whose
        // slots resolve to nothing.
        let names = loaded.names.clone();
        procs.push(Material {
            l1s: loaded.l1s,
            l2s: loaded.l2s,
            l3s: loaded.l3s,
            fields: loaded.fields,
            l4s: loaded.l4s,
            names,
        });
        // No files behind it — the sources came out of the store by hash. The
        // empty entry is what keeps `placed` indexed by slot; nothing saves
        // this slot, since `--load-set` with `--save-set` is refused and a
        // session's head is the file itself.
        placed.push(Vec::new());
    }
    for (slot, (l1, rest)) in args.sets.iter().enumerate() {
        let (material, nodes) = sort_slot(slot, l1, rest);
        procs.push(material);
        placed.push(nodes);
    }

    // Saving is a one-shot: it writes what the flags say and stops, on the same
    // terms as `--render`. Running afterwards would leave an operator unsure
    // whether what they are watching is what was written.
    if let Some(id) = &args.save_set {
        let geometries = procs.first().map(|m| m.l1s.as_slice()).unwrap_or(&[]);
        save_set(&args, &placed, geometries, id);
        return;
    }

    if let Some(id) = args.replay.clone() {
        replay_session(&args, &id);
        return;
    }

    match args.render_to.clone().or(args.seq_to.clone()) {
        Some(path) => {
            let gpu = Gpu::headless().expect("no GPU");
            let (w, h) = args.canvas;
            check_canvas(&gpu.device, w, h);
            // Fixed, never watching: an offscreen run is a function of its
            // inputs, and a save landing halfway through a sequence would make
            // it a function of the operator's editor as well.
            // **Nothing to re-aim, and nothing that could ask.** This path
            // watches nothing and serves no MCP, so every entry would be
            // `None` and no `wire_input` can reach it.
            let (mut deck, _) = build_deck(&gpu, &procs, &args, false, false, w, h, None, None);
            eprintln!(
                "rendering the mix of {} Set{}, {w}x{h}, {} frames, \
                 {} at exposure {:.2} -> {}",
                deck.slot_count(),
                if deck.slot_count() == 1 { "" } else { "s" },
                args.frames,
                op_name(args.look.op),
                args.look.exposure,
                path.display()
            );
            let result = if args.seq_to.is_some() {
                render::to_sequence(&gpu, &mut deck, args.look, w, h, args.frames, &path)
            } else {
                render::to_png(&gpu, &mut deck, args.look, w, h, args.frames, &path)
            };
            if let Err(e) = result {
                eprintln!("{e}");
                std::process::exit(1);
            }
            report_live_counts(&gpu, &deck);
        }
        None => {
            let event_loop = EventLoop::new().expect("event loop");
            event_loop.set_control_flow(ControlFlow::Poll);
            event_loop
                .run_app(&mut App {
                    args,
                    procs: Some(procs),
                    placed,
                    live: None,
                    snapshots,
                })
                .expect("run");
        }
    }
}

/// The element live count of every slot. A stall per slot — it copies four
/// bytes off the GPU and blocks until the queue drains — which is why this is
/// called once, after the last frame, and never from the status line. See
/// `Set::live_count`, which says so at the definition.
pub(crate) fn report_live_counts(gpu: &Gpu, deck: &Deck) {
    eprintln!("elements at the end of the run (a stall; not printed while running):");
    for slot in 0..deck.slot_count() {
        let set = deck.slot(EngineSlot(slot as u8)).set();
        eprintln!(
            "  slot {slot}: {} live of {} allocated",
            set.live_count(&gpu.device, &gpu.queue),
            set.capacity()
        );
    }
}

#[cfg(test)]
mod tests;
