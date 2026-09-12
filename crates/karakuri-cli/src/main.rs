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
/// **The step count this run's frames advance by**, and it is
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
/// **It was asked once and answered for everybody.** The build resolved
/// `capacity_for(args, &l1[0])` and handed that one number to every source, so
/// a second geometry ran at the first one's count with nothing printed: a grid
/// written for 512 x 256 samples and declared at 131072 drew 32768 of them
/// because it was loaded beside a cube. Nothing refused it either — the number
/// came from a declaration, so it was inside somebody's range, just not the
/// range of the procedure it was applied to.
///
/// `recorded` is what a Set file said, per geometry, and it wins where it said
/// anything: **the file is where that geometry's count was decided**, and a
/// Set that came back at a different size is a Set that was not saved. Empty
/// for a slot no file filled, which is every slot but slot 0.
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

/// **Whether anything in this run can write a `.kir`.** `--watch` and `--mcp`
/// are the two things that edit a procedure; a render or a replay opens every
/// file read-only.
///
/// A function rather than a `let`, because three things turn on it and they
/// have to turn on the same one: what gets copied into the scratch, what gets
/// compiled out of `args.sets` rather than out of the Set file directly, and —
/// since a rebuilt slot's sources are what a live save writes — whether the
/// watchers put what they build into the store. The third arrived on the far
/// side of `main`, in `App::resumed`, which is what made the copy worth
/// removing.
fn editable(args: &Args) -> bool {
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

/// The element live count of every slot. **A stall per slot** — it copies four
/// bytes off the GPU and blocks until the queue drains — which is why this is
/// called once, after the last frame, and never from the status line. See
/// `Set::live_count`, which says so at the definition.
fn report_live_counts(gpu: &Gpu, deck: &Deck) {
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

struct App {
    args: Args,
    procs: Option<Vec<Material>>,
    /// Where each slot's files ended up, slot for slot beside `procs` — what a
    /// session's head is written from. See [`Placed`].
    placed: Vec<Vec<Placed>>,
    live: Option<Live>,
    /// The run's edit history, already holding what it started with. Handed to
    /// every slot's watcher when the deck is built — see [`history`].
    snapshots: Option<history::Shared>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(procs) = self.procs.take() else {
            return;
        };

        let attrs = Window::default_attributes()
            .with_title("Karakuri")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.args.size.0,
                self.args.size.1,
            ));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let size = window.inner_size();

        let instance = Gpu::instance();
        let surface = instance.create_surface(window.clone()).expect("surface");
        let gpu = pollster::block_on(Gpu::from_instance(instance, Some(&surface))).expect("gpu");

        let caps = surface.get_capabilities(&gpu.adapter);
        // sRGB encoding happens once, at final output: pick a surface format
        // that carries the transfer function so the hardware does it on write.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            // The format above already carries the transfer function, and
            // `Auto` is the one value that leaves the presentation engine
            // interpreting the swapchain exactly as it always has: sRGB for an
            // `*Srgb` format, and never a wide-gamut or HDR space picked
            // behind the pipeline's back. See P-0064 — sRGB is encoded once,
            // at final output, and that is here.
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            // **Chosen, not taken.** This was `caps.present_modes[0]`, which is
            // whatever order the backend happened to list — so the pacing of a
            // run was a property of the driver, invisible and unsettable, and
            // the same session ran differently on two machines with nothing
            // saying so. `Fifo` is supported on every platform and is
            // `PresentMode`'s own default, so naming it costs nothing and makes
            // the answer the same everywhere. It is also the right answer for
            // this output: tearing across a projected image is worse than a
            // frame of latency.
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&gpu.device, &config);

        // **The canvas, not the window.** These two were the same number until
        // the window was named a preview: what is drawn is the session's and
        // what it is looked at through is not, so a window that opened at an
        // odd size no longer decides what a run renders — and dragging one no
        // longer reallocates every slot's target on the render thread.
        let (canvas_w, canvas_h) = self.args.canvas;
        check_canvas(&gpu.device, canvas_w, canvas_h);
        let present = Present::new(&gpu.device, format, canvas_w, canvas_h);
        // **Opened before the deck, because a watcher needs it.** A rebuilt
        // procedure has to reach the store from the worker thread that built
        // it; by the time the swap lands on a frame, the file may have changed
        // again and the render thread is the wrong place for file I/O.
        //
        // **Whenever the run is editable**, rather than only when a session is
        // being recorded. Two things read these hashes now — the `procedure`
        // records and `Live::save_set` — and the second is wanted in the
        // ordinary `--watch` case, which records nothing. See
        // `watch::Watch::stored`.
        let (rebuilds, rebuild_rx) = match editable(&self.args) {
            true => {
                let store = std::sync::Arc::new(open_store(&self.args));
                let (tx, rx) = std::sync::mpsc::channel();
                (Some((store, tx)), Some(rx))
            }
            false => (None, None),
        };
        let (mut deck, aims) = build_deck(
            &gpu,
            &procs,
            &self.args,
            self.args.watch,
            true,
            canvas_w,
            canvas_h,
            rebuilds,
            self.snapshots.clone(),
        );
        // Here and nowhere else: before the first frame, where the stall it
        // costs is free. Nothing else measures the Sets a run starts with —
        // only the build worker measures, and at startup it has built nothing
        // — and **one unmeasured live slot makes the whole deck's committed
        // cost unknown**, which parks every priming request there is with
        // `Reason::CommittedUnknown`. Without this call the governor below is
        // an elaborate way of saying no.
        deck.measure_slots(&gpu.device, &gpu.queue);

        eprintln!(
            "running: {} slot{} of {} elements at {:.1} bpm on {}",
            deck.slot_count(),
            if deck.slot_count() == 1 { "" } else { "s" },
            self.args.capacity,
            self.args.bpm,
            gpu.adapter.get_info().name
        );
        // What the deck costs and what it is allowed, printed once at startup
        // so the numbers a park is later explained by are not the first the
        // operator sees. Every millisecond in that line is a cold Set measured
        // at a reference resolution — comparable between slots, not a
        // prediction of this machine's frame time. The line says which clock
        // it came off, which is the part that changes between machines.
        eprintln!("  {}", deck.govern());
        if self.args.watch {
            for (slot, (l1, l4s)) in self.args.sets.iter().enumerate() {
                eprintln!(
                    "  watching slot {slot}: {} and {} — a save recompiles that slot in the \
                     background and swaps it when ready, budget {:.1} ms",
                    l1.path.display(),
                    l4s.iter()
                        .map(|p| p.path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(" and "),
                    self.args.budget_ms
                );
            }
        }
        // Opened before the first frame and never on it. A failure here is
        // fatal on purpose: `--audio-in` was asked for, and a run that quietly
        // continued without it would look exactly like a run whose bindings
        // are all at a tenth effect for some other reason.
        let audio = match &self.args.audio_in {
            Some(selector) => {
                match audio::Audio::open(selector, self.args.latency_offset_ms, DT, self.args.bpm) {
                    Ok(audio) => {
                        eprintln!(
                            "audio in: {} at {} Hz — energy, onset and band0..7 are measured now, \
                             and the beat corrects the oscillator. output offset {:.0} ms (o/p)",
                            audio.description(),
                            audio.sample_rate(),
                            audio.latency_offset_ms()
                        );
                        Some(audio)
                    }
                    Err(e) => {
                        eprintln!("karakuri-cli: {e}");
                        std::process::exit(2);
                    }
                }
            }
            None => None,
        };

        // Opened on the same terms as the audio device and for the same
        // reason: `--midi-in` was asked for, and a run that quietly continued
        // without it would look exactly like a run whose surface is plugged in
        // and doing nothing.
        let midi = match &self.args.midi_in {
            Some(selector) => match midi::Surface::open(selector, self.args.midi_map.as_deref()) {
                Ok(surface) => Some(surface),
                Err(e) => {
                    eprintln!("karakuri-cli: {e}");
                    std::process::exit(2);
                }
            },
            None => None,
        };

        // Third of the same kind. Fatal for the same reason: `--tempo-source`
        // was asked for, and a run that quietly went on following its own grid
        // would look exactly like one whose source is attached and agreeing.
        let tempo_source = match &self.args.tempo_source {
            Some(command) => match tempo_source::Source::open(command) {
                Ok(source) => {
                    eprintln!(
                        "tempo source: `{}` — the grid follows its beat, and `bar` is the \
                         room's rather than one counted from when this started",
                        source.name()
                    );
                    Some(source)
                }
                Err(e) => {
                    eprintln!("karakuri-cli: tempo source: {e}");
                    std::process::exit(2);
                }
            },
            None => None,
        };

        // Fourth of the same kind. Fatal for the same reason the others are:
        // `--mcp` was asked for, and a run that went on without it would look
        // exactly like one whose client is connected and idle.
        let mcp = match self.args.mcp {
            Some(port) => {
                // **Built once and never written again, and that is this
                // program rather than a shortcut.** `mcp::Slots` is a live
                // handle because the panel re-points a slot when the operator
                // loads a Set onto a running deck; nothing here does — the only
                // Set this run names is `--load-set`'s, settled before the deck
                // is built, and `Aiming::re_aim` restates the files it is
                // already pointed at. So the launch pairs are what this deck is
                // running for the whole run.
                let slots = mcp::Slots::of(
                    self.args
                        .sets
                        .iter()
                        .map(|(l1, l4s)| {
                            (
                                l1.path.clone(),
                                l4s.iter().map(|n| n.path.clone()).collect(),
                            )
                        })
                        .collect(),
                );
                // The store this run was given, not a second answer to where
                // the library is: `read_set` reads a saved Set and its cards
                // out of the same root `--save-set`, `--load-set` and the `k`
                // key write into.
                // **Closed, all four classes**, which is the state ADR-0235
                // says a run starts in. It is handed in rather than decided
                // inside the server: what a model may reach is the operator's
                // to say (P-0094), and a constant compiled into the server is
                // the one place it must not be said. **Nothing writes it
                // yet** — the bay-head toggles are the console's — so every
                // closed class stays closed for the whole run, and the handle
                // is what the toggles will hold the other end of.
                let opening = karakuri_environment::Opening::closed();
                match mcp::serve(
                    port,
                    slots,
                    self.args.store.clone(),
                    self.args.watch,
                    opening,
                ) {
                    Ok(reporter) => {
                        // The port bound rather than the one asked for: `--mcp 0`
                        // takes an ephemeral one, and printing the 0 would name
                        // a port that is not the port.
                        let port = reporter.port();
                        eprintln!(
                            "mcp: 127.0.0.1:{port} — a client can read and rewrite a slot's \
                             procedure{}",
                            if self.args.watch {
                                ""
                            } else {
                                ", but without --watch nothing will pick a write up"
                            }
                        );
                        Some(reporter)
                    }
                    Err(e) => {
                        eprintln!("karakuri-cli: mcp: {e}");
                        std::process::exit(2);
                    }
                }
            }
            None => None,
        };

        eprint!("\n{BINDINGS}\n");

        // Opened before the first frame and never on one: it creates a file
        // and spawns a thread. A failure is fatal because `--record-session`
        // was asked for, and a run that quietly continued without it would be
        // a performance nobody can replay and nothing saying so.
        let recorder = match &self.args.record_session {
            Some(id) => {
                let store = open_store(&self.args);
                let geometries = self
                    .procs
                    .as_ref()
                    .and_then(|procs| procs.first())
                    .map(|m| m.l1s.as_slice())
                    .unwrap_or(&[]);
                let head = session_head(&self.args, &self.placed, geometries, &store, id);
                seed_store_for_replay(&store, &self.placed);
                match session::Recorder::open(&store, id, &head) {
                    Ok(recorder) => {
                        eprintln!(
                            "recording session `{id}` — {} record{} of material at its head, \
                             so `--replay {id}` needs nothing else",
                            head.len(),
                            if head.len() == 1 { "" } else { "s" }
                        );
                        Some(recorder)
                    }
                    Err(e) => {
                        eprintln!("karakuri-cli: {e}");
                        std::process::exit(1);
                    }
                }
            }
            None => None,
        };

        let slot_count = deck.slot_count();
        // Opened before the first frame like every other channel here, and
        // never on one. Nothing is spawned until a key is pressed.
        let (save_tx, saves) = std::sync::mpsc::channel();
        // **Before the first frame, and for every windowed run.** This is what
        // makes "what is this slot running" a hash from the outset rather than
        // a path some later state contradicts. No I/O and nothing that can
        // fail: the addresses come off the bytes the compile read — see
        // [`Running::at_launch`].
        let running = Running::at_launch(&self.placed, slot_count);
        let live = Live {
            window,
            gpu,
            sink: frame::WindowSink::new(surface, config),
            present,
            deck,
            look: self.args.look,
            focus: 0,
            clock: Clock::new(Instant::now()),
            audio,
            midi,
            tempo_source,
            rebuilds: rebuild_rx,
            pending_builds: std::collections::HashMap::new(),
            running,
            // **Cloned rather than moved**, because `self.placed` is what
            // `session_head` above was handed and `App` outlives this. It is a
            // handful of paths per slot, once, at startup.
            startup: self.placed.clone(),
            loaded_set: self.args.load_set.clone(),
            edges: self.args.edges.clone(),
            aims,
            store_root: self.args.store.clone(),
            save_tx,
            saves,
            saves_in_flight: 0,
            mcp,
            operations: Vec::new(),
            quantum: QUANTA[0].0,
            fade_beats: FADE_BEATS[0],
            mask_kind: MASK_SHAPES[0].0,
            mask_angle: MASK_SHAPES[0].1,
            started: Instant::now(),
            status_at: Instant::now(),
            frames_since_status: 0,
            status: String::with_capacity(256),
            demo: self.args.demo.map(|d| (d, 0)),
            recorder,
            demo_started: Instant::now(),
        };
        // Through a record at startup too, on the same terms as every later
        // change: `--tonemap` and `--exposure` are an operator's choices rather
        // than the engine's defaults, so a session that did not carry them
        // would replay under whatever look the next build happens to default
        // to. This is also the first thing that decodes one, so a `look` this
        // build cannot obey is reported before a frame is drawn.
        let mut live = live;
        // **Before the look, and once.** A replay reads this out of the stream
        // to size everything it allocates, so it has to be there before any
        // record that describes a frame — and there is deliberately no second
        // writer anywhere, which is what "fixed for the run" means in practice.
        live.record(mix::canvas_record(canvas_w, canvas_h));
        live.record(mix::look_record(&self.args.look));
        self.live = Some(live);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => live.resize(size.width, size.height),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && live.key(&event.logical_key) {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                live.frame();
                live.window.request_redraw();
            }
            _ => {}
        }
    }

    /// The one place a stall is welcome: every frame has been rendered and the
    /// run is over, which is exactly the case `Set::live_count` is documented
    /// to be for.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(live) = &mut self.live {
            // Before the counts, because it ends a thread and flushes a file
            // and those are the things worth knowing failed.
            // Before the counts and before the recorder: it is another process
            // and leaving it running would outlive the window that started it.
            if let Some(source) = live.tempo_source.take() {
                source.close();
            }
            // **Before the recorder is finished**, so a save that landed after
            // the last frame is still in the stream it belongs to — including
            // one that was still being written when the window closed, which is
            // what the bounded wait is for. See [`Live::awaited_saves`].
            live.awaited_saves();
            if let Some(recorder) = live.recorder.take() {
                match recorder.finish() {
                    Ok(w) => {
                        eprintln!("session: {} records written", w.records);
                        // **Named rather than counted quietly**, and named
                        // apart: a lost batch is a second of everything and a
                        // lost audio frame is one frame's measurement, and an
                        // operator deciding what to do about a stream needs to
                        // know which it has.
                        if w.dropped_batches > 0 {
                            eprintln!(
                                "  {} batch{} lost because the disk could not keep up — \
                                 the stream has gaps",
                                w.dropped_batches,
                                if w.dropped_batches == 1 { "" } else { "es" }
                            );
                        }
                        if w.dropped_audio > 0 {
                            eprintln!(
                                "  {} frame{} of audio not recorded — those frames replay \
                                 with the bus's invented values rather than what was heard",
                                w.dropped_audio,
                                if w.dropped_audio == 1 { "" } else { "s" }
                            );
                        }
                    }
                    Err(e) => eprintln!("session: {e}"),
                }
            }
            report_live_counts(&live.gpu, &live.deck);
        }
    }
}

#[cfg(test)]
mod tests;
