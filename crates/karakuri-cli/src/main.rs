//! The V1 CLI entry point for executing, watching, rendering, or replaying sets.

use clock::Clock;
use karakuri_environment::compile::{sort_slot, Material, Named, Placed};
use karakuri_environment::mix::{op_name, op_wire_names, parse_op};
use karakuri_environment::setfile::{layer_named, Names, SavedNode, Sources};
use karakuri_environment::{
    accepted_save, no_such_param, no_such_renderer, no_such_slot, nothing_to_save, Asked, SAVE_WAIT,
};
use karakuri_environment::{
    audio, clock, compile, history, midi, mix, places, render, scratch, session, setfile,
    tempo_source, watch,
};
use karakuri_mcp as mcp;

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
use karakuri_operation_record::{not_performed, Current, Written};
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

/// Fixed simulation step duration.
const DT: f32 = karakuri_engine::set::DT;

pub(crate) const SEED: u32 = 19_274;

/// Resolves capacities per geometry source, honoring recorded or default values.
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

/// Returns whether the run supports live editing via `--watch` or `--mcp`.
pub(crate) fn editable(args: &Args) -> bool {
    args.watch || args.mcp.is_some()
}

fn main() {
    let args = parse_args();

    // Load pre-packaged or saved Set if specified, replacing CLI arguments.
    let mut args = args;
    let loaded = args.load_set.clone().map(|id| load_set(&mut args, &id));

    // Initialize scratch space and snapshot tracking for editable sessions.
    let mut snapshots: Option<history::Shared> = None;
    let editable = editable(&args);
    if editable {
        let root = args.store.clone();
        if let Some(loaded) = &loaded {
            let written: Result<Vec<PathBuf>, String> = loaded
                .nodes()
                .enumerate()
                .map(|(at, (checked, src))| {
                    scratch::place(&root, &scratch::node_name(0, at, &checked.name), src)
                })
                .collect();
            match written {
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
        let shared = history::Snapshots::shared(&args.store);
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
    // Retain slot-indexed structure for downstream mapping.
    let mut placed: Vec<Vec<Placed>> = Vec::new();
    if let Some(loaded) = loaded.filter(|_| !editable) {
        eprintln!("  slot 0: set `{}`", loaded.id);
        let names = loaded.names.clone();
        procs.push(Material {
            l1s: loaded.l1s,
            l2s: loaded.l2s,
            l3s: loaded.l3s,
            fields: loaded.fields,
            l4s: loaded.l4s,
            names,
        });
        placed.push(Vec::new());
    }
    for (slot, (l1, rest)) in args.sets.iter().enumerate() {
        let (material, nodes) = sort_slot(slot, l1, rest);
        procs.push(material);
        placed.push(nodes);
    }

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
            // Headless render uses fixed deck without hot-reload watchers.
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

/// Queries and logs the final element live count for each slot.
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
