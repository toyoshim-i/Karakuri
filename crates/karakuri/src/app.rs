//! Application state and event loop handling.

use std::sync::Arc;
use std::time::{Duration, Instant};

use karakuri_console::focus;
use karakuri_console::input::Claim;
use karakuri_console::panel::Panel;
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::view::{
    self, inspector as inspector_pane, look as look_row, mixer as mixer_bay, tracker_group, Scope,
    Sequenced, Tracker, View,
};
use karakuri_console::{egui, egui_wgpu, egui_winit};
use karakuri_engine::{
    compose, Committed, Deck, DeckSlot as EngineSlot, Gpu, Sink, Skip, WindowSink,
};
use karakuri_environment::clock::Clock;
use karakuri_environment::{audio, history, midi, watch, Asked, Opening};
use karakuri_layout::Point;
use karakuri_mcp as mcp;
use karakuri_operation::{BeatSource, GridScale, Operation, Output, SetTransfer, Undecided};
use karakuri_operation_record::{written, Written};
use karakuri_store::record::Record;
use karakuri_store::store::Store;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::engine_bridge::*;
use crate::gfx::*;
use crate::keymap::{KeyAction, KeyCtx, KEY_BINDINGS};
use crate::launch::*;
use crate::readout::*;
use crate::session::{Keeping, Sessions};
use crate::{CANVAS, MAPPED, SERVED, WINDOW};

/// **Open or close the projector window**, and say what happened.
///
/// The one operation in this file that makes a *window*, which is why it takes
/// an `&ActiveEventLoop` and why it is reached from `window_event` rather than
/// from [`App::performed`]: `winit` will not create a window without one, and
/// `performed` is handed a [`Gfx`] and no event loop.
///
/// # What each answer is
///
/// - **`Projector(0)` on** opens a window, makes a surface on the device the
///   panel is already using, configures it and puts a [`WindowSink`] over it.
///   The next frame's [`render_size`] sees a second output and the frame
///   follows the larger of the two.
/// - **`Projector(0)` off** drops the [`Projector`], which drops the surface
///   and the last reference to the window — so the window closes and the sink
///   leaves the slice on the same statement. Nothing is torn down in an order
///   this file has to remember.
/// - **`Program`** never arrives: the picture's on and off is the fold, and
///   the console performs it where the arrangement is (`Readout::sink`). It is
///   an arm here so that the match is exhaustive and says so.
/// - **`Plugin(n)`** is refused with the sentence that names what is missing,
///   which is [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
///   there is no manifest to read a plugin sink out of, and the console draws
///   both plugin chips `no plugin` for that reason, so this is only reachable
///   from a route that is not the panel.
///
/// # The surface has to be the format the present pass was built for
///
/// [`Present`]'s pipeline names one target format at construction, and that is
/// [`PICTURE_FORMAT`] — the picture's texture, sRGB, so the encode is the
/// hardware's and happens exactly once (P-0064). A surface in another format
/// would need a second pipeline, which is a *second present pipeline* and is
/// exactly what ADR-0247 says nothing needs. So this asks the surface for that
/// format and **refuses to open the window when it is not offered**, naming
/// both — a window that opened and drew nothing would be the silent wrong
/// picture P-0094 refuses, and this is a refusal before the show rather than a
/// fault during one.
pub(crate) fn routed(
    gfx: &mut Gfx,
    event_loop: &ActiveEventLoop,
    output: Output,
    on: bool,
) -> Option<String> {
    let n = match output {
        // The console's, and it is already done by the time this is asked.
        Output::Program => return None,
        Output::Projector(n) => n,
        Output::Plugin(n) => {
            return Some(format!(
                "outputs: plugin {n} is not loaded — there is no plugin manifest to read a                  sink out of, and `docs/plugins.md` is the specification nothing implements"
            ))
        }
    };
    if !on {
        return Some(match gfx.projector.take() {
            Some(_) => format!("outputs: projector {n} is off — the window is closed"),
            None => format!("outputs: projector {n} was already off"),
        });
    }
    if gfx.projector.is_some() {
        return Some(format!("outputs: projector {n} is already on"));
    }
    let attrs = Window::default_attributes()
        .with_title("Karakuri — projector")
        // **The session canvas**, which is the size an output starts at when
        // nothing else says (ADR-0246). The operator resizes it, or makes it
        // fullscreen, and the frame follows.
        .with_inner_size(winit::dpi::LogicalSize::new(CANVAS.0, CANVAS.1));
    let window = match event_loop.create_window(attrs) {
        Ok(window) => Arc::new(window),
        // **Reported rather than panicked**, for `resumed`'s reason: a panic
        // here is reached from a `winit` callback and cannot unwind across the
        // Objective-C frame on macOS, so it aborts with no sentence anywhere.
        Err(e) => return Some(format!("outputs: the projector window did not open: {e}")),
    };
    let surface = match Gpu::instance().create_surface(window.clone()) {
        Ok(surface) => surface,
        Err(e) => return Some(format!("outputs: the projector has no surface: {e}")),
    };
    let caps = surface.get_capabilities(&gfx.gpu.adapter);
    if !caps.formats.contains(&PICTURE_FORMAT) {
        return Some(format!(
            "outputs: the projector window cannot be opened — the present pass draws              {PICTURE_FORMAT:?} and this surface offers {:?}. A second format would want a              second present pipeline, which is what one render scaled into every output              exists to avoid",
            caps.formats
        ));
    }
    let size = window.inner_size();
    let size = (size.width.max(1), size.height.max(1));
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: PICTURE_FORMAT,
        color_space: wgpu::SurfaceColorSpace::Auto,
        width: size.0,
        height: size.1,
        // **`Fifo`, like the panel's**, and the two are not a stall on each
        // other: `compose` asks every sink for a target before it commits, and
        // a projector that has none loses its own frame rather than the
        // window's (ADR-0171).
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&gfx.gpu.device, &config);
    gfx.projector = Some(Projector {
        window,
        sink: WindowSink::new(surface, config),
        size,
    });
    Some(format!(
        "outputs: projector {n} is on — a window at {} x {}, and the frame is composited at the          largest enabled output",
        size.0, size.1
    ))
}

/// **What the window loop does about what a press asked for**, for every answer
/// but the two that reach the store.
///
/// `Asked::Scope` and `Asked::Load` are the caller's, because both end in a
/// directory read or a file write and this function has neither the store nor
/// the folder; everything else is one operation or one sentence.
///
/// **A refusal is said out loud**, which is the whole of what `Asked::Nothing`
/// carries: a key that declines and a key that is not bound are the same
/// experience, so the console's own sentence is printed rather than swallowed
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
pub(crate) fn answered(
    gfx: &mut Gfx,
    started: Instant,
    readout: &mut Readout,
    recorder: Option<&mut karakuri_environment::session::Recorder>,
    asked: &focus::Asked,
) -> Repaint {
    let acted = match asked {
        focus::Asked::Nothing(why) => {
            println!("  key: {why}");
            Acted::Nothing
        }
        // **The address moved and nothing was asked of anything**, which is
        // `Change::Pointed`'s own case: focus is a pointer this console owns,
        // so nothing in the arrangement moved and no `Outcome` says so.
        focus::Asked::Moved => return Change::Pointed(true).repaint(),
        focus::Asked::Emitted(operation) => Acted::Emitted(Some(operation.clone())),
        // **A move of the arrangement**, which is not the vocabulary's: the
        // fold `space` performs on a bay, and the Program head's solo. It
        // leaves by `Readout::op` like every other arrangement press, so the
        // sentence a fold prints and the frame it asks for are the ones `f`
        // printed and asked for until 2026-09-10.
        focus::Asked::Panel(op) => {
            let outcome = readout.op(*op);
            return Change::Operated(&outcome).repaint();
        }
        // **The picture's on and off, which is one press asking for two
        // things** — `Readout::sink` is where the pair is said out loud, and
        // it is the same method the Outputs row's dot goes through.
        focus::Asked::Routed(asked, op) => {
            let outcome = readout.sink(asked.clone(), *op);
            return Change::Operated(&outcome).repaint();
        }
        // **The level, named here because the step is this file's arithmetic**
        // — [`gain_key`] and [`opacity_key`], whose tenth is `karakuri-cli`'s
        // and whose clamp decides what the record says.
        //
        // **Five levels and one shape.** The two on a strip are read off the
        // deck through [`held`]; the master out is read off the same deck one
        // pass along, the exposure off the look and the offset off the audio
        // session — each of them a value this file has in front of it and the
        // console does not (ADR-0156, ADR-0333).
        focus::Asked::Stepped { level, step } => match level {
            focus::Level::Trim(deck) | focus::Level::Fader(deck) => {
                match held(&gfx.engine.deck, *deck) {
                    Some(slot) => match level {
                        focus::Level::Trim(_) => Acted::Emitted(Some(Operation::SetGain {
                            deck: *deck,
                            gain: gain_key(*step, gfx.engine.deck.gain(slot)),
                        })),
                        _ => Acted::Emitted(Some(Operation::SetOpacity {
                            deck: *deck,
                            opacity: opacity_key(*step, gfx.engine.deck.opacity(slot)),
                        })),
                    },
                    None => Acted::Nothing,
                }
            }
            focus::Level::Out => Acted::Emitted(Some(Operation::SetMasterOut {
                out: out_key(*step, gfx.engine.deck.out()),
            })),
            focus::Level::Exposure => Acted::Emitted(Some(Operation::SetExposure {
                exposure: exposure_key(*step, gfx.engine.look.exposure),
            })),
            focus::Level::Offset => match gfx.audio.as_ref() {
                Some(open) => Acted::Emitted(Some(Operation::SetLatencyOffset {
                    ms: offset_key(*step, open.latency_offset_ms()),
                })),
                None => {
                    println!("{NO_ROOM_FOR_AN_OFFSET}");
                    Acted::Nothing
                }
            },
        },
        // The caller answers these two, and it returns before it gets here.
        focus::Asked::Scope | focus::Asked::Load => {
            unreachable!("the scope and the load are answered where the store is")
        }
    };
    App::performed(gfx, started, readout, recorder, &acted, Repaint::Never)
}

/// **A load, performed** — [`loading`] reached from an operation, and `None`
/// for every operation that is not one.
///
/// `Operation::LoadSet` writes no record either, so this is [`pointed`]'s
/// shape one bay along: the surface that names it performs it. What it does
/// **not** do is touch the deck, which is the whole design — see [`loading`]
/// and [`Engine::aimed`].
///
/// Every failure is a sentence and none of them moves anything: a slot the
/// deck has not got, a store that will not open, a Set that is not there, a
/// procedure that no longer checks, or a worker that has gone.
pub(crate) fn played(gfx: &mut Gfx, operation: &Operation) -> Option<String> {
    let Operation::LoadSet { deck, set } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = gfx.engine.aimed.len();
    let Some(aim) = gfx.engine.aimed.get_mut(slot) else {
        return Some(format!(
            "  load: deck {letter} refused — this deck has {count} slot{}, and `{set}` has \
             nowhere to land",
            match count {
                1 => "",
                _ => "s",
            }
        ));
    };
    match loading(&gfx.store, slot, slot_salt(slot), aim, set) {
        Ok(line) => {
            // **What that slot is now playing**, written on the press that
            // changed it. A `Set` has no name of its own, so the strip and the
            // pane head read whatever whoever built it says — and after a load
            // that is the id the operator picked out of the library, which is
            // the same word the row they pressed on carries.
            //
            // **Written on the aim rather than on the swap**, which is a
            // choice and not an oversight: the build may still be refused, or
            // land and stop its slot for cost, and a name that waited for the verdict
            // would leave the strip naming material that is no longer in the
            // file. The staging lane is what says which of the three happened,
            // on the deck it happened to, and it is the surface built for
            // exactly that disagreement — a strip name that hedged would be a
            // second, quieter answer to the question that lane is about.
            if let Some(name) = gfx.material.get_mut(slot) {
                name.clear();
                name.push_str(set);
            }
            Some(line)
        }
        Err(e) => Some(format!(
            "  load: `{set}` did not reach deck {letter}: {e} — nothing moved, and what is on \
             that deck is still running"
        )),
    }
}

/// **A procedure loaded over one layer of what a deck is playing, performed** —
/// a press on a procedure row's `load`, on one of its row menu's four loads, or
/// a drag of it onto a strip or a cell, and `None` for every operation that is
/// not one.
///
/// # It is [`played`]'s shape with one file instead of every file
///
/// A Set load re-points the slot at every file that Set names; this re-points
/// it at **the files it is already on with one of them replaced**, which is the
/// whole of ADR-0338 taken literally. Both are one `Aiming::re_point`, both are
/// compiled off the render thread and judged at a frame boundary on what one
/// frame of the result costs, and the Staging lane says which of the three
/// happened. Nothing is installed and nothing is written where the presets are.
///
/// **`Aim::set` is left where it is**, which is the half of the maintainer's
/// answer that has a mechanism behind it: the versions this slot writes from
/// here on go on being filed under the Set it started from, so the `history`
/// chip keeps listing that deck's versions and the snapshot every compile takes
/// stays alive (ADR-0304, ADR-0308). It follows from `overlaying` restating the
/// aim rather than building one.
///
/// **The strip then reads `<base> + <kir>`**, so what is on air says what it is
/// made of and never claims to be a Set the library holds. `keep` is what gives
/// it a name, and it files a new Set exactly as it does for any other deck —
/// what `Playing` gathers is the aim's own files, which is what this changed.
///
/// **Written on the aim rather than on the swap**, which is [`played`]'s own
/// choice and its argument word for word: the build may be refused or land and
/// stop its slot for cost, and a readout that waited for the verdict would name
/// material that is no longer in the file. The staging lane is the surface built
/// for that disagreement.
pub(crate) fn overlaid(gfx: &mut Gfx, operation: &Operation) -> Option<String> {
    let Operation::LoadProcedure { deck, procedure } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = gfx.engine.aimed.len();
    // **The base before the aim is borrowed**, because the sentence and the
    // readout both want it and `overlaying` takes the aim mutably.
    let base = base_material(
        gfx.engine
            .aimed
            .get(slot)
            .and_then(|aim| aim.at.set.as_deref()),
        &gfx.launch,
    );
    let presets = gfx.presets.clone();
    let store = gfx.store.clone();
    let Some(aim) = gfx.engine.aimed.get_mut(slot) else {
        return Some(format!(
            "  load: deck {letter} refused — this deck has {count} slot{}, and `{procedure}` has \
             nowhere to land",
            match count {
                1 => "",
                _ => "s",
            }
        ));
    };
    match overlaying(&store, presets.as_deref(), slot, aim, procedure) {
        Ok(line) => {
            if let Some(name) = gfx.material.get_mut(slot) {
                name.clear();
                name.push_str(&derived_material(&base, procedure));
            }
            Some(line)
        }
        Err(e) => Some(format!(
            "  load: `{procedure}` did not reach deck {letter}: {e} — nothing moved, and what is \
             on that deck is still running"
        )),
    }
}

/// **A version put back** — a row of the Library bay's `history` scope landed
/// on the node it was a version of, and `None` for every operation that is not
/// one.
///
/// # It is a load, and it goes the way every other load goes
///
/// The snapshot's bytes are written over that node's **working copy** under
/// `<store>/scratch/`, where the slot's watcher is already looking, and
/// nothing else is touched: no deck, no aim, no channel. So the worker reads
/// it, compiles it off the render thread, swaps it at a frame boundary and
/// rolls it back on its own if it cannot hold the budget — which is
/// [`loading`]'s argument met from the other end, and
/// `docs/adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md`
/// is where it is written down. Nothing is installed.
///
/// **The version it replaces is kept by the same act.** The watcher snapshots
/// at its compile-success point
/// (`docs/adr/0089-history-is-gated-on-compiling-not-on-landing.md`), so what
/// was on the node before this write is the next row of the very listing the
/// press came off — a landing you did not mean is itself undoable.
///
/// # Finding the file again, and why no path crosses the seam
///
/// The console says a **name** — [`version_row`]'s spelling, the one it was
/// handed — and this re-asks `history::list` and rebuilds that spelling per
/// candidate to find the row. That is `SetTransfer::Take`'s arrangement
/// exactly: *"the panel's route re-asks the listing and finds the row by the
/// word that was pressed, so no surface spells a path"*.
///
/// **Which node the file is is asked of the aim rather than of the launch
/// copies.** [`Engine::pointing`] is the run's one `mcp::Slots`, written out of
/// what each watcher is *pointed at* — `Aiming::at` — so the answer follows a
/// library load. `Slots::file` is the one walk that turns
/// `(slot, layer, index)` into a file, and it is asked rather than repeated.
///
/// **This used to build a `Slots` of its own here**, because the one the MCP
/// server held was the launch working copies and went stale on the first load
/// (ADR-0308's *Doubted*). The server reads this same handle now, so the second
/// one is gone rather than kept beside it.
///
/// # Five refusals, and each says where the deck is still pointed
///
/// A slot the deck has not got, a deck playing no Set at all, a version that
/// is not one of that Set's, a node the deck does not hold, and a file that is
/// gone or will not be written. `rm -rf history/2026/07` is this store's whole
/// retention policy, so the fourth is an ordinary state of a working store and
/// the sentence names the file rather than calling it damaged
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
pub(crate) fn restored(gfx: &Gfx, operation: &Operation) -> Option<String> {
    let Operation::RestoreProcedure { deck, revision } = operation else {
        return None;
    };
    // **What was asked for, in the words a refusal has to say it in.** Both
    // arms name a version; one says which and one says where, and this is the
    // only place in the sentence they differ.
    let asked = asked_for(revision);
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = gfx.engine.aimed.len();
    let Some(aiming) = gfx.engine.aimed.get(slot) else {
        return Some(format!(
            "  put back: deck {letter} refused — this deck has {count} slot{}, so {asked} \
             has nowhere to land",
            match count {
                1 => "",
                _ => "s",
            }
        ));
    };
    let Some(id) = aiming.at.set.as_deref() else {
        return Some(format!(
            "  put back: deck {letter} is playing the pair this run was launched with rather \
             than a Set, so it has no history to put {asked} back from — load a Set onto \
             that deck first; nothing moved"
        ));
    };
    // **The run's one published layout, asked rather than rebuilt** — this
    // function's own head. [`Engine::pointing`] is written from `Aiming::at` on
    // every re-point, so a slot that has had a Set loaded onto it resolves to
    // that Set's scratch files here and answers a model the same way.
    Some(put_back(
        &gfx.store,
        slot,
        letter,
        id,
        revision,
        &gfx.engine.pointing,
    ))
}

/// **The Set the load pulldown's deck is running**, or `None` for a deck
/// playing the pair the run was launched with.
///
/// **It is read off the aim and off nothing else**, which is ADR-0304: the id
/// rides the `watch::Aim` a load sends, restated by every rewiring, and
/// `Gfx::material` beside it is the mixer strip's *readout* — the pair at
/// launch, and never an id a listing can match.
///
/// **The pulldown and not the selection**, which is ADR-0305 read on a second
/// control: the letter in that foot is what says where a load lands, so it is
/// what says whose history the `history` scope is showing. A target past the
/// slots the deck has answers `None`, which `View::aim_at` already refuses and
/// this does not depend on.
///
/// **Owned, because the caller is about to take `&mut View`.** One `String` per
/// press on a path that is about to read a directory.
pub(crate) fn aimed_set(gfx: &Gfx, view: &View) -> Option<String> {
    gfx.engine
        .aimed
        .get(usize::from(view.target_deck()))
        .and_then(|aiming| aiming.at.set.clone())
}

pub(crate) struct App {
    gfx: Option<Gfx>,
    /// **What the command line asked for**, read before the event loop starts
    /// and used once, in `resumed`. It is here rather than in [`Gfx`] because
    /// it is known before there is a device and outlives every remake of one.
    ///
    /// **The pair the operator named, and not what any deck runs from** — see
    /// [`running`](App::running). What this answers is the strips' name, which
    /// is a question about what was asked for: four decks opened on one preset
    /// are playing that preset, whatever their four files are called.
    sources: Sources,
    /// **The working copy each deck runs from**, one pair per slot, in slot
    /// order — [`working_copies`], made in [`main`] before the window.
    ///
    /// Beside `sources` rather than replacing it because the two answer
    /// different questions and always have: this is what a watcher polls and
    /// what an editor opens, and `sources` is what the operator said. They were
    /// one field while every slot watched the typed paths, which is the defect
    /// this pair of fields exists to end.
    running: Vec<Sources>,
    /// **Where the library is** — see [`Launch::store`]. Here for `sources`'
    /// reason, and copied onto [`Gfx::store`] for the readers that are handed
    /// only a device.
    store: std::path::PathBuf,
    /// **The preset library this run resolved**, kept for one purpose: the
    /// legend says which of the places answered, and it says it by printing
    /// what the resolution returned rather than a sentence about what it
    /// probably did. `None` is a machine with no library, which reaches this
    /// far only on a run that was given its pair by hand.
    presets: Option<karakuri_environment::places::Presets>,
    /// **The directory the Library bay is pointed at**, or `None` until a
    /// folder has been dropped on this window — which is where every run
    /// starts, because nothing names one before the run (ADR-0275).
    ///
    /// **Beside [`presets`](App::presets) because it is the same kind of
    /// thing**: a directory outside this store that a scope of the bay lists.
    /// The difference is when it is decided — a presets root is resolved
    /// before the window opens and this arrives during the run — and that is
    /// why one is on [`Launch`] and this is not.
    ///
    /// **The path is here and the *spelling* is in the console**
    /// (`view::View::folder`): this side reads the directory and the panel
    /// draws the line, which is the seam every other library value crosses
    /// (ADR-0156). Two fields for one fact, and they are written in one place
    /// — [`folder_dropped`].
    folder: Option<std::path::PathBuf>,
    /// A validation fault is said once rather than sixty times a second.
    faulted: bool,
    /// **Whether shift is held**, which is the whole of what this loop keeps
    /// of the modifier state and is here for exactly one key.
    ///
    /// `winit`'s `KeyEvent` carries no modifiers, so a key handler that wants
    /// to tell `Tab` from `shift-Tab` has to have been listening to
    /// `WindowEvent::ModifiersChanged` — which is why this is a field rather
    /// than a question asked at the press.
    ///
    /// **It is not a mode and it gives no key a second meaning**, which is the
    /// distinction
    /// [ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
    /// draws when it rejects modifier chords: *"a modifier is a mode with no
    /// readout"*, and rule 04 is *nothing is hidden quietly*. `shift-Tab`
    /// survives that rejection because it reverses a traversal that is drawn
    /// either way — the ring is on a bay whichever direction you arrived from
    /// — so there is no hidden state to read out.
    ///
    /// **`egui`'s own copy is not readable in time.** `egui_winit::State`
    /// keeps modifiers privately and stamps them onto the events it queues;
    /// `Context::input` answers from the last pass, so a `shift` pressed since
    /// the previous frame would be invisible there. One reader, one writer, one
    /// event.
    shift: bool,
    readout: Readout,
    /// **The console's hover layer**, which is `karakuri-console`'s and is
    /// held here for the reason every other piece of console state is: this
    /// file owns the window, the pointer and the clock, and that crate owns
    /// none of the three.
    ///
    /// **It is not part of [`Readout`]** because nothing routes into it: a
    /// tooltip is not an operation, no key reaches it, and MIDI and MCP have
    /// nothing to say to it. What it takes is a pointer move, the claim
    /// `input::claim` already answered for that move, and a time — and what it
    /// gives back is a repaint decision and a box painted after the panel.
    hover: karakuri_console::hover::Hover,
    costs: Costs,
    /// Logical size, so the numbers printed are the arrangement's own units
    /// rather than the display's.
    scale: f64,
    /// **When `egui` asked to be drawn again, kept as the deadline it is.**
    ///
    /// `egui` animates, blinks a text cursor and fades a tooltip in, and it
    /// says so as a `repaint_delay` on the frame's `ViewportOutput`. Turning
    /// that into an immediate `request_redraw` would turn a 250 ms animation
    /// into a spin at whatever rate this loop can manage — which is the cost
    /// ADR-0164's still-panel clause is about, arrived at from the one direction
    /// that
    /// looks like obeying it. So the delay is added to the clock here and
    /// `about_to_wait` sleeps until it.
    ///
    /// `None` where `egui` asked for nothing, which is every frame on a panel
    /// with nothing on it.
    egui_due: Option<Instant>,
    /// **The origin every animation on the panel is measured from**, and the
    /// only clock behind `view::Phase`.
    ///
    /// It is here because this file owns the window and the clock and `src/`
    /// owns neither — every `Instant::now` in this crate is in this file, and
    /// `view::Phase` is a `Duration` for exactly that reason (P-0092). What
    /// crosses the seam is `now - this`, which is a number.
    ///
    /// **Where the origin is does not matter**, which is why it is taken at
    /// construction rather than when something first starts moving: every
    /// presentation is periodic in the phase, so an origin the operator did
    /// not choose is an origin nobody can see. What would matter is having
    /// *two*, and there is one.
    started: Instant,
    /// **When a served run next wakes to take what a model asked for**, and
    /// `None` for a run without `--mcp` — see [`SERVED`], which is the whole of
    /// why this deadline exists.
    ///
    /// A deadline beside `egui_due` rather than a `ControlFlow::Poll`, because
    /// this loop has one rule about when it runs and a second one would be a
    /// second answer to it: [`App::about_to_wait`] takes the soonest of what is
    /// owed, and this is one of the things owed.
    served: Option<Instant>,
    /// **How a thread that is not this one gets this loop to run again**, and
    /// it exists for exactly one of them: the MIDI callback.
    ///
    /// Every other thing this window answers arrives as a `winit` event or on
    /// a deadline [`App::about_to_wait`] already sets. A control surface is
    /// neither — a hand on a knob is an event nothing in `winit` can see — and
    /// this loop sleeps in `Wait` between frames (ADR-0164), so without a wake
    /// a fader would be applied at whatever the operator's next mouse move
    /// happened to be.
    ///
    /// **A poll was the alternative and it is the one this rejects.** A third
    /// deadline beside `egui_due` and `served` would have to run at a hand's
    /// rate to feel like a fader — a hundred and twenty-five wakes a second,
    /// for the whole of a run, whether or not anything is plugged in — which
    /// is `ControlFlow::Poll` with extra steps and is the exact cost the
    /// still-panel clause is about. [`SERVED`]'s tenth of a second is the
    /// other end of that trade and is a fader at 10 Hz.
    ///
    /// **It carries nothing.** The proxy's event type is `()`: the wake says
    /// *ask again*, [`App::user_event`] asks for a frame, and the drain
    /// happens where every other drain happens. Handed to
    /// [`midi::Surface::first`] as a closure, so no crate below this one
    /// learns that a window exists.
    waker: EventLoopProxy<()>,
    /// The sending half of [`watch::Watch::storing_to`]'s channel, handed to
    /// every watcher [`Engine::new`] makes. Kept because a window remade makes
    /// them again.
    built_tx: std::sync::mpsc::Sender<watch::Built>,
    /// **The store the watchers put their builds in**, opened once in [`main`].
    ///
    /// An open store rather than the root beside it, because this one is shared
    /// with four worker threads and each of them writes to it on every build.
    /// [`App::store`] is still the root, and is still what a save, a listing and
    /// an arrangement are handed: those open per call, which is what keeps a
    /// listing from creating a directory it only wanted to read.
    held: std::sync::Arc<Store>,
    /// **Every version that compiles in this run**, seeded in [`main`] from the
    /// files the decks were about to play and handed to every watcher
    /// [`Engine::new`] makes.
    ///
    /// **One for the run**, which is [`karakuri_environment::history`]'s own
    /// requirement rather than a convenience: the seed and the watchers share
    /// the dedup, and seeded separately the first rebuild would write the
    /// untouched procedure a second time. That is also why it is here rather
    /// than on [`Gfx`] — a window remade rebuilds the deck and would rebuild
    /// the history with it.
    ///
    /// **What each version is filed under is not here.** That is the Set the
    /// slot is running, and it lives on the aim, one per slot, moved by a load
    /// — see [`Aiming`] and [`watch::Aim::set`].
    snapshots: history::Shared,
    /// **The run's one published layout**, made in [`main`], handed to
    /// [`karakuri_mcp::serve`] there and to every [`Engine`] this
    /// opens — see [`karakuri_mcp::Slots`] and [`Aiming::pointing`].
    ///
    /// **Here rather than on [`Gfx`]**, for [`App::snapshots`]' reason exactly:
    /// the server is bound before the window and outlives every window this run
    /// remakes, so a handle rebuilt with the swapchain would leave the server
    /// reading one nothing writes.
    pointing: karakuri_mcp::Slots,
    /// **Everything a save and a rewiring need that is not the deck** — see
    /// [`Keeping`].
    keeping: Keeping,
    /// **The session recorder, and the two presses that move it** — see
    /// [`Sessions`].
    ///
    /// It is beside [`App::keeping`] rather than inside it because the two
    /// hold different things for different moments: that one is what a *save*
    /// and a rewiring need, and this is a writer thread the frame path pushes
    /// into. What they share is the arrangement — a press gathers, a thread
    /// works, and the outcome is said at the frame it arrives.
    recording: Sessions,
    /// **How far a frame advances the session**, derived from the interval it
    /// measures — [`karakuri_environment::clock::Clock`], which is the same
    /// derivation `karakuri-cli` makes because there is one of them.
    ///
    /// This is P-0092's live half: *"live, the engine derives the step count
    /// from real time and writes it in"*. The number is read once per composed
    /// frame, handed to [`measure_audio`] as this frame's advance, committed as
    /// [`Committed::steps`], and pushed as the `tick` that closes the frame
    /// where a recording is running — one measurement reaching four places
    /// rather than four answers to how long a frame was.
    ///
    /// **It is on [`App`] rather than on [`Gfx`]**, which is [`Sessions`]'
    /// reason read one field along: a window remade is a display remade, and a
    /// session's clock is not the surface's. A clock rebuilt with the swapchain
    /// would restart the count in the middle of a stream that is still being
    /// written.
    ///
    /// **The run's first frame is capped**, and that is the cap doing its job
    /// rather than an accident to correct: this is started before the window
    /// exists, so everything between here and the first composed frame — the
    /// device, the compile, the first Sets — is one gap, and a gap is counted
    /// whole up to `MAX_STEPS` (ADR-0006).
    clock: Clock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventLoopAction {
    Exit,
    Continue,
}

impl App {
    /// Reaction to a window event regarding event loop termination.
    pub(crate) fn event_loop_action_for(
        is_main_window: bool,
        event: &WindowEvent,
    ) -> EventLoopAction {
        if !is_main_window {
            return EventLoopAction::Continue;
        }
        match event {
            WindowEvent::CloseRequested => EventLoopAction::Exit,
            _ => EventLoopAction::Continue,
        }
    }

    /// Reaction to a window event regarding event loop termination for this app instance.
    pub(crate) fn event_loop_action(&self, id: WindowId, event: &WindowEvent) -> EventLoopAction {
        let is_main_window = self.gfx.as_ref().is_none_or(|g| g.window.id() == id);
        Self::event_loop_action_for(is_main_window, event)
    }

    /// Updates the shift modifier state from a `ModifiersChanged` event.
    pub(crate) fn update_modifiers(&mut self, state: &winit::event::Modifiers) {
        self.shift = state.state().shift_key();
    }
    /// **The opening is handed in rather than made here**, which is the whole of
    /// what pairing the four pills with a server took: [`main`] gives the same
    /// handle to [`karakuri_mcp::serve`] and to this, so a press on
    /// a bay head and the class the server reads are one value. [`Readout::new`]
    /// makes one of its own — it is constructed from a size and nothing else —
    /// and this replaces it before the window opens, which is before anything
    /// can read either.
    // **Eight, where clippy's line is seven, and it went past it when the
    // window took a control surface.** Every one of them is a thing settled
    // *before* there is a device — the command line, the copies each deck runs
    // from, the store, the versions, the server, what is open, where each slot
    // points, and the wake a MIDI callback gets this loop to run again with —
    // and a struct to carry them would be [`App`] itself, constructed with
    // every field that needs a window left out. That is `Watch::new`'s
    // sentence in `karakuri-environment`, which is fourteen for the same
    // reason.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        launch: Launch,
        running: Vec<Sources>,
        held: std::sync::Arc<Store>,
        snapshots: history::Shared,
        mcp: Option<mcp::Reporter>,
        opening: Opening,
        pointing: mcp::Slots,
        // **Made in [`main`] from the loop this is about to be run on**, for
        // [`App::waker`]'s reason: the surface is opened in `resumed`, which
        // is handed an `ActiveEventLoop` and cannot make one of these.
        waker: EventLoopProxy<()>,
    ) -> App {
        let mut readout = Readout::new(WINDOW.0 as f32, WINDOW.1 as f32);
        // **The one handle, and it lives on the readout because that is where
        // the pills reach it.** A copy kept on [`App`] as well would be a second
        // answer to what is open the day one of them was written and the other
        // was not.
        readout.view.opening = opening.read();
        readout.opening = opening;
        let (built_tx, built) = std::sync::mpsc::channel();
        let (save_tx, saves) = std::sync::mpsc::channel();
        let (send_tx, sends) = std::sync::mpsc::channel();
        let (keep_tx, keeps) = std::sync::mpsc::channel();
        App {
            gfx: None,
            sources: launch.sources,
            running,
            store: launch.store,
            presets: launch.presets,
            // **Pointed nowhere**, which is where every run starts: a folder
            // is chosen by dropping one on this window and no flag names one
            // before it opens (ADR-0275).
            folder: None,
            faulted: false,
            // **Nothing held**, which is a window nobody has pressed a key on
            // and is also what `winit` reports the moment focus leaves it.
            shift: false,
            readout,
            // **The manual is read here, before the window opens**, which is
            // where a 340 KB parse belongs: it is one walk of the page
            // compiled into this binary and it is on no frame path (P-0091).
            hover: karakuri_console::hover::Hover::new(),
            costs: Costs::new(),
            scale: 1.0,
            egui_due: None,
            // **Due at once on a served run**, so the first thing a model asks
            // for is taken on the first wake rather than a tenth of a second
            // after it.
            served: mcp.is_some().then(Instant::now),
            started: Instant::now(),
            waker,
            built_tx,
            held,
            snapshots,
            pointing,
            keeping: Keeping {
                mcp,
                // **Empty until there is a deck**, because the launch nodes are
                // what the engine's compile produced and there is no engine
                // before `resumed`. It is seeded there, off [`Engine::placed`],
                // on the same pass that makes the watchers.
                playing: Playing {
                    playing: Vec::new(),
                },
                built,
                pending: Vec::new(),
                saves,
                save_tx,
                sends,
                send_tx,
                keeps,
                keep_tx,
                in_flight: 0,
            },
            recording: Sessions::new(),
            clock: Clock::new(Instant::now()),
        }
    }

    /// Hand an event to `egui`, and nowhere else.
    ///
    /// Every call site has already asked `claim` where a pointer event
    /// belongs; this is the other branch. `EventResponse::repaint` is `egui`'s
    /// own answer for the event it was just given, and it is the reason
    /// `Change::Pointer(Claim::Egui)` asks for nothing: one answer per event,
    /// from whoever got it.
    ///
    /// **`repaint` is taken out of the `EventResponse` on this line and
    /// nothing else survives it** (ADR-0259). `egui-winit` 0.36.1 hard-codes
    /// `EventResponse::consumed` `true` for every `Tab` — *"When pressing the
    /// Tab key, egui focuses the first focusable element, hence Tab always
    /// consumes"* — whether or not anything in this program's `egui::Context`
    /// has focus, so honouring it here would swallow the key that moves focus
    /// between bays on its first press, with no panic and no diagnostic. This
    /// program is told about a window event and never asks `egui` for
    /// permission, so the destructure below is the whole of the fix: past
    /// this line there is no `EventResponse` left in scope for a future
    /// `if … .consumed` to be added to by mistake, only the one `bool` this
    /// function was always allowed to read.
    ///
    /// **This used to be a text scan.** `event_response`'s former `#[test]`,
    /// `the_only_field_read_off_an_event_response_is_repaint`, read this file
    /// for the word `consumed` and for every `response.` field access above
    /// the tests. Both questions are unnecessary now rather than merely
    /// unlikely to trip: `on_window_event` is called nowhere else in this
    /// program, and the only value this call site keeps a name for is a
    /// `bool` with no `EventResponse` behind it to add a second field read to.
    ///
    /// **ADR-0259's second condition is still checked by nothing, as it was
    /// before this line existed.** The claim above holds *because* the
    /// console focuses no `egui` widget — no `Button`, `TextEdit`, `Slider`,
    /// `DragValue`, `.interact(` or `.sense(` anywhere in
    /// `karakuri-console/src`, so `Memory::focused()` is permanently `None`
    /// and `consumed` would be reading a flag that means nothing on this
    /// program's own widgets even where it is read. That was true on
    /// 2026-09-05 and has not been re-checked since; it is written here
    /// rather than left to look covered by a module that no longer exists.
    fn to_egui(gfx: &mut Gfx, costs: &mut Costs, event: &WindowEvent) {
        let egui_winit::EventResponse { repaint, .. } =
            gfx.egui.on_window_event(&gfx.window, event);
        if repaint {
            costs.owes();
            gfx.window.request_redraw();
        }
    }

    /// **Perform what a pointer event asked for, and say what frame it is
    /// owed.** `otherwise` is the answer for an event that acted on nothing —
    /// the claim's, which is the answer this loop had before there were
    /// controls.
    ///
    /// The two kinds of control end in two different places, which is what
    /// [`Acted`] is for:
    ///
    /// - The Outputs dot's operation was already performed by the panel, and
    ///   what is owed is what the [`Outcome`] says happened.
    /// - **A fader's operation is performed here**, because it is the *deck*
    ///   that moves and the panel has no deck (ADR-0156). It becomes a record
    ///   and the record moves the deck — P-0090, which is what makes this
    ///   fader the same control as a key press and a MIDI knob rather than a
    ///   third way of writing a gain. The next frame's strips are read back off
    ///   the deck by [`mixer`], so what the fader shows is what the deck says
    ///   and never what this loop remembered.
    ///
    /// **What the operation becomes is [`written`]'s answer and not this
    /// file's**, out of the operation and what [`reading`] read off the deck.
    /// It used to be a `match` written out here, because there was
    /// nowhere for the conversion to live; ADR-0185 said that function is
    /// deleted the day a home lands, and
    /// [ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)
    /// is that home. Three answers come back and all three are said out loud —
    /// the records go to [`apply`], and the other two go to [`unwritten`],
    /// which is the difference between *this press writes nothing, and that is
    /// settled* and *this press owes a record nobody has decided how to
    /// write*.
    pub(crate) fn performed(
        gfx: &mut Gfx,
        started: Instant,
        readout: &mut Readout,
        // **The session stream, where one is open**, and it is here rather
        // than reached for because this is the one place a record is applied:
        // a session is *the same fader moves at the same instants*, so the
        // records this function hands to [`apply`] are exactly the records a
        // replay has to see. A push that happened anywhere else would be a
        // second list of what a control did.
        //
        // `None` for the whole of a run nobody pressed `rec` on, which is most
        // runs and costs one branch per press.
        recorder: Option<&mut karakuri_environment::session::Recorder>,
        acted: &Acted,
        otherwise: Repaint,
    ) -> Repaint {
        match acted {
            Acted::Nothing => otherwise,
            // **A class pill earns the frame the claimed press already earns,
            // and no more.** Nothing in the arrangement moved and no operation
            // was emitted; what changed is one word in one capsule, and
            // `Change::Pointer(Claim::Panel)` — which is what `otherwise` is on
            // every path that can reach this arm — is already `Repaint::Now`. A
            // `Change` of its own would be a second answer to a question that
            // is already answered.
            Acted::Opened => otherwise,
            // **A carry that moved the library cursor earns the frame the
            // claimed press already earns, and no more**, which is the arm
            // above word for word. `Change::Pointer(Claim::Panel)` — what
            // `otherwise` is on every path that can reach this — is already
            // `Repaint::Now`, and `Change::Pointed(true)` is what the arrow
            // keys raise for the same move and is the same answer. The re-read
            // this press owes is not here because the store is not: see the
            // button-up arm of `App::window_event`, which is where every other
            // press that reaches a disk reaches it.
            Acted::Pointed => otherwise,
            Acted::Operated(outcome) => Change::Operated(outcome).repaint(),
            Acted::Emitted(operation) => {
                if let Some(operation) = operation.as_ref() {
                    // **The two that reach the room's tracker, and they leave
                    // this arm rather than falling through it.** `written`
                    // answers `Owed(NotSettled)` for both — a tap's record is
                    // the *beat lock's* answer, and none of the tapped tempo,
                    // the phase error or the output lag is a value a `Current`
                    // carries — so going on would print *"nothing moved, and
                    // nothing here decides it"* about a press that moved the
                    // grid. They do end in a record, and
                    // `karakuri_environment::audio` is what writes it: see
                    // [`tapped`], where the gap and what would close it are
                    // written down.
                    //
                    // **This is the one place a tap is performed**, which is
                    // what makes the `tap` pill and `b` one control rather than
                    // two spellings of one: both emit `Operation::TapBeat` and
                    // both arrive here.
                    if let Some(line) = tracked(gfx, started, operation) {
                        println!("{line}");
                        return Change::Emitted(Some(operation)).repaint();
                    }
                    // **The two operations that reach a disk**, and they are
                    // taken first because they are not about the deck at all:
                    // an arrangement is the console's own state, `written`
                    // answers `Silent(Surface)` for both, and what they change
                    // is the panel and a file under the store. Everything
                    // below this is the mix. See [`arrangement`], which
                    // answers `None` for every other operation and is why this
                    // is one line rather than a second route into the panel.
                    if let Some(line) = arrangement(
                        &gfx.store,
                        &mut readout.panel,
                        &mut readout.view.arrangement,
                        operation,
                    ) {
                        println!("{line}");
                    }
                    // **The two the console performs itself**, and they are
                    // here for the same reason the arrangement is: both write
                    // no record, so `written` below answers `Silent` for them
                    // and there is nothing for `apply` to do. One moves a
                    // pointer this crate does not hold, the other re-points a
                    // slot's source and lets the worker do the rest — neither
                    // is the mix. Each answers `None` for every other
                    // operation, which is what keeps this two lines rather
                    // than two more routes into the engine.
                    // **The one that reaches a device**, and it is here
                    // beside the arrangement for the same reason: it writes no
                    // record either — `written` answers `Silent(NoRecord)`,
                    // because nothing in the session stream says what the beat
                    // is taken from — and what it changes is this program's
                    // audio session and the pill that reads it. The session
                    // tempo is read first so that the borrow of `gfx.audio`
                    // below does not have to hold the deck as well.
                    let session_bpm = gfx.engine.deck.signals().oscillator().bpm();
                    if let Some(line) = attached(
                        &mut gfx.audio,
                        session_bpm,
                        &mut readout.view.audio,
                        operation,
                    ) {
                        println!("{line}");
                    }
                    // **The second that reaches that device**, and it is here
                    // for the reason the attach is: `written` answers
                    // `Silent(NoRecord)` for it too, so there is nothing for
                    // `apply` to do and the session this program opened is the
                    // only thing that holds the value. See [`nudged`].
                    if let Some(line) = nudged(&mut gfx.audio, operation) {
                        println!("{line}");
                    }
                    // **The third that reaches that device, and the only one
                    // of the three that does not end there.**
                    // `Operation::SetFreeRunTempo` writes a `Record::Tempo`,
                    // so what moves the grid is [`apply`] a few lines down —
                    // the one road into the oscillator, live and on replay
                    // (P-0090). What this hands the session is the state the
                    // record does not carry: the beat lock's run of evidence
                    // and the tracker's window. It does not return early for
                    // that reason, and with no device open it does nothing at
                    // all — which is the state this operation is *for*. See
                    // [`retargeted`].
                    if let Some(line) = retargeted(&mut gfx.audio, operation) {
                        println!("{line}");
                    }
                    if let Some(line) = pointed(&mut readout.view, operation) {
                        println!("{line}");
                    }
                    // **And a pane's own pointer, performed beside the deck
                    // selection**, which is the mark it is deliberately not:
                    // `written` answers `Silent(Surface)` for it too, so the
                    // surface that names it performs it and nothing here
                    // touches the deck. See [`pointed_pane`].
                    if let Some(line) = pointed_pane(&mut readout.view, operation) {
                        println!("{line}");
                        // **And the panes are re-read on the press that moved
                        // one**, which is the transport's own arrangement one
                        // arm down: this operation writes no record, so the
                        // line that re-reads on a `Record::Transport` cannot
                        // catch it, and a pane pointed at a new deck while
                        // still drawing the old one's nodes would be the
                        // readout being wrong and silent. It is a press and
                        // never a frame, which is what `Set::published`
                        // allocating asks of every caller.
                        let targets = readout.view.pane_decks();
                        inspector(
                            &gfx.engine.deck,
                            &gfx.material,
                            &gfx.engine.aimed,
                            targets,
                            &mut readout.view.inspector,
                        );
                    }
                    if let Some(line) = played(gfx, operation) {
                        println!("{line}");
                    }
                    // **The load beside it that replaces one file instead of
                    // every file**, and it is the same act: a re-point of a
                    // slot's watcher with one layer written over what the deck
                    // is playing. `written` answers `Silent(NoRecord)` for it
                    // exactly as it does for `LoadSet`, so the surface that
                    // names it performs it. See [`overlaid`] and ADR-0338.
                    if let Some(line) = overlaid(gfx, operation) {
                        println!("{line}");
                    }
                    // **The deck head's fold, performed where the load beside
                    // it is**, and it is the same act: a re-point of a slot's
                    // watcher, with the layering changed instead of the files.
                    // `written` answers `Silent(NoRecord)` for it as it does
                    // for the load, so the surface that names it performs it,
                    // and nothing here touches the deck. See [`composited`].
                    if let Some(line) = composited(&mut gfx.engine.aimed, operation) {
                        println!("{line}");
                    }
                    // **The two chips beside that fold, performed where it
                    // is**, and each is the same act with a different field of
                    // the aim changed: the element count its geometries run at
                    // and the salt its randomness comes from. `written` answers
                    // `Silent(NoRecord)` for `SetProperty` as it does for the
                    // fold and the load, so the surface that names it performs
                    // it and nothing here touches the deck. See [`resized`] and
                    // [`re_salted`], and `docs/adr/0328-…`.
                    if let Some(line) = resized(&mut gfx.engine.aimed, operation) {
                        println!("{line}");
                    }
                    if let Some(line) = re_salted(&mut gfx.engine.aimed, operation) {
                        println!("{line}");
                    }
                    // **The publish mark, performed with them**, and it is the
                    // fourth field of one aim: the layering, the capacity, the
                    // salt and now the interface. `written` answers
                    // `Silent(NoRecord)` for `Publish` as it does for the other
                    // three, so the surface that names it performs it. See
                    // [`attended`], where the reason it is the aim rather than a
                    // writer into the live Set is argued.
                    if let Some(line) = attended(&mut gfx.engine.aimed, operation) {
                        println!("{line}");
                    }
                    // **A `uses` line's pick, performed beside them**, and it
                    // is the same re-aim with the wiring changed. It needs the
                    // run's edge list as well as the aims, which is why that
                    // list is a field of `Engine`: a press arrives here and the
                    // console's readout holds no engine. See [`wired_input`],
                    // which is [`rewired`] with one request — the function a
                    // model's `wire_input` goes through, so the panel and the
                    // tool rewire by one route.
                    let slots = gfx.engine.deck.slot_count();
                    let Engine {
                        edges: run_edges,
                        aimed,
                        ..
                    } = &mut gfx.engine;
                    if let Some(line) = wired_input(run_edges, aimed, slots, operation) {
                        println!("{line}");
                    }
                    // **A version put back, performed where the load beside it
                    // is.** `written` answers `Silent(OnLanding)` for it — the
                    // `Record::Procedure` is written at the swap, by the same
                    // path a save takes — so what this file owes is the write
                    // into the scratch and nothing else. See [`restored`].
                    if let Some(line) = restored(gfx, operation) {
                        println!("{line}");
                    }
                    // **A candidate kept, performed on the lane it is about.**
                    // `written` answers `Silent(Silent::Surface)` for it, so
                    // there is no record and nothing for `apply` to do: what
                    // changes is one row of `View::staging`, which is this
                    // readout's. It is here beside the put-back rather than
                    // beside the arrangement because the two are the same
                    // row's two presses. See [`kept`].
                    if let Some(line) = kept(&mut readout.view, operation) {
                        println!("{line}");
                    }
                    // **The transition row's three settings, performed on the
                    // console's own pointer**, and it is here for the reason
                    // the selection is one line up: `Operation::SetTransition`
                    // writes no record — `written` answers `Silent(Surface)` —
                    // so the surface that emits it is what performs it. See
                    // [`scheduled`].
                    if let Some(line) = scheduled(&mut readout.view, operation) {
                        println!("{line}");
                    }
                    // **The Sequencer bay's three, performed where the
                    // transition row's are and for their reason**: `written`
                    // answers `Silent(Surface)` for all five of that bay's
                    // operations, so the surface that emits one performs it and
                    // there is nothing here for `apply` to do. What a *lane*
                    // does is not this line — it comes back through this same
                    // function as an `Operation::SetOpacity`, which is the
                    // whole of ADR-0322. See [`sequenced`].
                    if let Some(line) = sequenced(
                        &mut readout.sequencer,
                        &mut readout.playhead,
                        &readout.view,
                        operation,
                    ) {
                        println!("{line}");
                    }
                    // **The reading is taken off the deck and off the console,
                    // and for four of this bay's controls it is *I read
                    // nothing*.** A gain, an
                    // opacity, a blend mode and a residency carry everything
                    // their record carries, so a reading handed in for one of
                    // them would be this file inventing a value — which is
                    // what ADR-0194 refuses a default for. The mask mini's
                    // record is written whole out of two
                    // halves (ADR-0201), so its half is read here rather than
                    // assumed, and a wipe reads three — the transition
                    // settings the row above holds among them, which is why
                    // this is the one call that is handed something the deck
                    // does not know. See [`reading`].
                    //
                    // **Read after [`scheduled`] has run**, so a press on a
                    // pill and the wipe after it are converted against the
                    // settings the operator can see rather than the ones they
                    // just left.
                    let settings = readout.view.transition();
                    let written = written(
                        operation,
                        &reading(
                            operation,
                            &gfx.engine.deck,
                            &gfx.engine.look,
                            &gfx.engine.chain,
                            settings,
                        ),
                    );
                    // **A press that wrote no record says so**, and says
                    // which of the two kinds of nothing it was, before
                    // anything is applied.
                    if let Some(line) = unwritten(operation, &written) {
                        println!("{line}");
                    }
                    // **Every record, in the order it was written.** One
                    // today for each of the four, and a list because
                    // `Crossfade` is four and `Wipe` is up to six — one
                    // control is not one record (P-0090, ADR-0194).
                    if let Written::Records(records) = &written {
                        // **Into the session before the deck moves**, where
                        // one is being recorded: the stream is the timeline
                        // and the deck is what the timeline does, so a record
                        // that reached the deck and not the file would be a
                        // replay that does not reach where this run did. It is
                        // pushed whether or not `apply` finds somewhere to put
                        // it — a record the deck refused is still what the
                        // operator asked for, and a replay refuses it the same
                        // way.
                        //
                        // **Cloned, and it is the one place this program
                        // clones a record.** `push` takes ownership and the
                        // list is borrowed by the loop that applies it; every
                        // record here is scalars and a short string, and a
                        // press is not the frame path — `push_audio` exists
                        // precisely because the *one* record that carries a
                        // buffer must not be copied, and none of these is it.
                        if let Some(recorder) = recorder {
                            for record in records {
                                recorder.push(record.clone());
                            }
                        }
                        for record in records {
                            if let Some(line) = apply(
                                record,
                                &mut gfx.engine.deck,
                                &mut gfx.engine.look,
                                &mut gfx.engine.chain,
                            ) {
                                println!("{line}");
                            }
                        }
                        // **A scrub moves a value an Inspector pane is
                        // drawing, and the panes are read once for the run.**
                        // `Set::published` allocates and says it is not for
                        // the frame path, so [`inspector`] is called at
                        // startup and the anchor's `B128 +0.25` would go on
                        // reading the scrub the deck had when the window
                        // opened — a picture of a value that has moved, which
                        // is exactly what P-0094 is about. It is re-read here,
                        // on the press that moved it: a press is where this
                        // file already reads a directory, and it is not a
                        // frame.
                        //
                        // **Off the record rather than off the operation**,
                        // because what matters is that the deck's transport
                        // changed — the day a second operation writes one, it
                        // is caught by the same line.
                        //
                        // **Three more records move what a pane draws**, and
                        // they are the Inspector's own three. A `ride` moves a
                        // figure and a fader; a `source` turns a row bound or
                        // unbound, which adds or removes a whole sensitivity
                        // row and moves every row under it; an `authority`
                        // moves a chip. Each is off the record for the reason
                        // the transport is: what matters is that the deck
                        // changed, so a second operation writing one is caught
                        // by the same line.
                        if records.iter().any(|record| {
                            matches!(
                                record,
                                Record::Transport { .. }
                                    | Record::Ride { .. }
                                    | Record::Source { .. }
                                    | Record::Authority { .. }
                            )
                        }) {
                            let targets = readout.view.pane_decks();
                            inspector(
                                &gfx.engine.deck,
                                &gfx.material,
                                &gfx.engine.aimed,
                                targets,
                                &mut readout.view.inspector,
                            );
                        }
                    }
                }
                Change::Emitted(operation.as_ref()).repaint()
            }
        }
    }

    /// **Every operation a model has asked for since the last frame, performed
    /// where a press of the same operation is performed.**
    ///
    /// **It is [`Keeping::requests`]'s neighbour and not part of it**, because
    /// what a save and an edge need is the engine and what an operation needs is
    /// everything a press needs: the window, the readout, the session being
    /// recorded. So the drain is here, beside [`App::performed`], and the two
    /// are called one after the other at the two places this loop takes what a
    /// model asked for.
    ///
    /// **Nothing decides anything here.** The operation arrived already audited
    /// — `karakuri_operation::gate` ran on the server's own thread, which is the
    /// one call ADR-0235 puts the whole mechanism on — and it is handed to
    /// [`App::performed`] as an `Acted::Emitted`, which is the value a fader
    /// hands it. A model's `SetGain` and a hand on the strip are the same press
    /// from here on
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// which is the whole of what routing into one vocabulary was for.
    ///
    /// **Collected out of the borrow before any of it is acted on**, exactly as
    /// [`Keeping::requests`] collects, and for the same reason: the loop below
    /// takes `&mut` of things the reporter is reached through. An empty
    /// `collect` allocates nothing, which is every frame of a run nobody is
    /// driving.
    ///
    /// **Answered once, at the frame it was performed on.** What a *rebuild*
    /// the operation started makes of it lands thirty judged frames later and is
    /// `swap_outcome`'s answer, and what a *scheduled* move comes to is the
    /// deck's own reading — so the sentence says where each of those is rather
    /// than holding a connection open across a transition. That is
    /// `mcp::WireRequest`'s third point, one route along.
    ///
    /// # The three whose performer is not in [`App::performed`]
    ///
    /// **Most operations end in `performed` and this function names none of
    /// them.** Three do not, and each is taken here in the order and by the
    /// call the pointer's button-up arm takes it in — a second route into one
    /// of them would be the second answer
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// exists to prevent, so this calls the same functions rather than
    /// repeating what they do
    /// ([ADR-0341](../../../docs/adr/0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md)).
    ///
    /// - **A star, refused** — [`favourite`] with [`Asked::Model`], which is
    ///   ADR-0301's decision reached by the route that record said it was owed.
    ///   `my sets` is the list of Sets the operator chose, so the refusal is
    ///   the answer and it goes back as one: `Err`, which reaches the client as
    ///   a failed call, carrying the id and where the Set actually is. A
    ///   success reported for an act that had no effect is what that record
    ///   refused, and answering `ok` here would be it.
    /// - **A projector, opened** — [`routed`], which needs an `ActiveEventLoop`
    ///   and is why this function takes one. **The drain is already on the
    ///   event loop's thread**: both call sites are `winit` handlers holding
    ///   the loop, so the argument was there to be passed and the row was
    ///   `plan` for want of one parameter.
    /// - **A recording, started or stopped** — [`Sessions::asked`], which needs
    ///   the store as well and is why this takes that too. Both ends are
    ///   gathered here and written off the render thread, exactly as the `rec`
    ///   pill's press does.
    ///
    /// **They fall through to `performed` afterwards, as a press does**, so a
    /// row that also writes a record writes it once and in one place.
    // **Nine, where clippy's line is seven**, and the two past it are the two
    // the three arms above need: the event loop a window is made on, and the
    // store a recording's head is written into. [`App::mapped`] carries the
    // same allowance for the same reason — a struct here would be `App` itself
    // with the fields that need no window left out.
    #[allow(clippy::too_many_arguments)]
    fn operated(
        gfx: &mut Gfx,
        event_loop: &ActiveEventLoop,
        started: Instant,
        readout: &mut Readout,
        recording: &mut Sessions,
        keeping: &mut Keeping,
        store: &std::path::Path,
        egui_due: &mut Option<Instant>,
        costs: &mut Costs,
    ) {
        // **Collected out of the borrow before any of it is acted on**, and it
        // is a `match` rather than a `let else` on `keeping.mcp` because the
        // loop below takes `&mut` of the same struct: the reporter is read,
        // the queue is drained into a `Vec`, and the borrow ends on this line.
        let asked: Vec<mcp::OperateRequest> = match keeping.mcp.as_ref() {
            Some(mcp) => mcp.operations().collect(),
            None => return,
        };
        for mcp::OperateRequest { operation, reply } in asked {
            // **The one act in this drain that ends on a disk**, and it leaves
            // here rather than falling through: a keep is *"on a worker"* on
            // the page it is specified on, so the press goes and the answer
            // arrives later — which is what the reply riding the request is
            // for (`Kept::reply`). The sentence below would say it was
            // performed on this frame, and the file is not written yet.
            //
            // **`Asked::Model`, so it lands in `<store>/sandbox/`** — stamped,
            // overwriting nothing, and readable by nothing that reads the
            // library. A model is not refused here where its star is, because
            // what it keeps is a file and so has a sandbox form to land in
            // (P-0096, ADR-0261, ADR-0301).
            if let Operation::KeepProcedure { deck, node, ref id } = operation {
                keeping.keep_procedure(
                    &gfx.engine,
                    store,
                    Asked::Model,
                    usize::from(deck),
                    node,
                    id.clone(),
                    Some(reply),
                );
                continue;
            }
            let title = operation.title();
            // **A star is answered and never performed**, and [`favourite`]
            // answers `None` for every other operation, so this is the whole
            // of the branch. Nothing falls through: nothing was written, so
            // there is no record to write and no frame to ask for.
            if let Some(refusal) = favourite(store, Asked::Model, &operation) {
                println!("{refusal}");
                reply.settled(Err(refusal));
                continue;
            }
            // **The projector, opened where the chip in the Outputs row opens
            // it**, which is the one act in this file that makes a window and
            // is why this function takes the loop.
            let mut aside = None;
            if let Operation::RouteFrame { output, on } = &operation {
                aside = routed(gfx, event_loop, *output, *on);
            }
            // **And the `rec` pill's two ends**, gathered here and written off
            // the render thread exactly as the press does — see [`Sessions`].
            if let Operation::RecordSession {
                recording: asked_for,
            } = &operation
            {
                recording.asked(keeping, &gfx.engine, store, asked_for);
            }
            if let Some(line) = aside.as_deref() {
                println!("{line}");
            }
            let repaint = App::performed(
                gfx,
                started,
                readout,
                recording.recorder(),
                &Acted::Emitted(Some(operation)),
                Repaint::Never,
            );
            App::wants(gfx, egui_due, costs, repaint);
            // **The performer's own line goes back where there is one**, which
            // is the projector's: whether a window opened, was already open,
            // or could not be made at the format the present pass draws. The
            // sentence below says where a *later* answer lands and would have
            // said nothing about a window that never opened.
            reply.settled(Ok(format!(
                "`{title}` was performed on the frame it arrived on, where the same \
                 operation from the panel, a key or a mapped control is performed. What the \
                 deck made of it is on this run's terminal. Anything it started rather than \
                 finished is reported where it lands: ask `swap_outcome` for a rebuild, and \
                 a scheduled move arrives on the grid.{}",
                match aside {
                    Some(line) => format!("\n{}", line.trim_start()),
                    None => String::new(),
                }
            )));
        }
    }

    /// **Every operation a hand on a control surface asked for since the last
    /// frame, performed where a press of the same operation is performed.**
    ///
    /// **It is [`App::operated`]'s neighbour**, and the two are one shape: a
    /// door outside this window hands in an [`Operation`], and it is given to
    /// [`App::performed`] as an `Acted::Emitted` — the value a fader hands it.
    /// A knob's `SetGain`, a model's and a hand on the strip are the same
    /// press from here on
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// which is what routing every surface into one vocabulary was for, and it
    /// is why **a session recorded from this surface replays with neither the
    /// surface nor the map attached**
    /// ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)):
    /// what reaches the stream is the record, and no record names a knob.
    ///
    /// **Nothing decides anything here**, which is the same sentence
    /// [`App::operated`] carries and is true for a different reason. A model's
    /// request was audited on the server's thread; a hand needs no audit at
    /// all — `karakuri_operation::gate` is a model's boundary and not an
    /// operator's, and
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// says a hand cancels whatever automatic thing was writing the control,
    /// on every route in. A gate over the operator's own surface would be the
    /// instrument refusing its player.
    ///
    /// **The slot check is the router's and is said once per slot per run.**
    /// `Surface::take` drops a message naming a slot this deck does not hold,
    /// because `Deck::gain` and its neighbours index directly — see
    /// `karakuri_environment::midi`, where that whole argument lives.
    ///
    /// **The buffer is [`Gfx::performed_by_hand`] and is cleared by the
    /// drain**, so nothing here allocates on a frame nobody touched the
    /// surface — which is every frame of a run with no surface at all, and
    /// costs one branch.
    ///
    /// **Drained inside the frame rather than on the wake**, which is the
    /// difference between this and a key press: a key arrives as a `winit`
    /// event and is performed on it, and a MIDI message arrives on the MIDI
    /// thread. [`App::user_event`] asks for a frame and this is what that
    /// frame does about it, so a sweep spanning two wakes is one operation on
    /// one frame rather than two half-applied ones.
    // **Eight, where clippy's line is seven, and every one is a thing a press
    // needs**: the window, the clock, the readout, the stream being recorded,
    // the layer that knows what the pointer is on, where a learned map is
    // written, and the two halves of a repaint decision. A struct would be
    // `App` itself with the fields that need no window left out, which is
    // `App::new`'s sentence one function along.
    #[allow(clippy::too_many_arguments)]
    fn mapped(
        gfx: &mut Gfx,
        started: Instant,
        readout: &mut Readout,
        recording: &mut Sessions,
        hover: &karakuri_console::hover::Hover,
        maps: &std::path::Path,
        egui_due: &mut Option<Instant>,
        costs: &mut Costs,
    ) {
        if gfx.midi.is_none() {
            return;
        }
        // **Armed, and then this frame is a learn rather than a play.**
        //
        // Nothing is performed while the pill is lit, and that is the
        // decision rather than a consequence: a knob already mapped to deck
        // B's gain, turned while the pointer is on deck A's, would otherwise
        // move B on the way to being bound to A. **It is not hidden**, which
        // is what makes it safe — rule 04 — the pill is lit for as long as it
        // is true and the operator lit it.
        if readout.view.learn {
            App::learned(gfx, readout, hover, maps, egui_due, costs);
            App::showed(gfx);
            return;
        }
        let Some(surface) = gfx.midi.as_mut() else {
            return;
        };
        // Split rather than borrowed together: the drain writes the surface
        // and the buffer, and both are fields of `gfx`, which `performed`
        // then takes whole. `std::mem::take` would hand the allocation back
        // only if nothing panicked in between, so the buffer is swapped out
        // and swapped back.
        let mut asked = std::mem::take(&mut gfx.performed_by_hand);
        // **And the deck, for the one target the map cannot finish on its
        // own.** `cc -> param N M` names a *position* in a deck's published
        // interface, and a position becomes a key only against the Set that is
        // in the deck right now — which is the whole point of binding to one
        // (ADR-0268). `midi::Decks` is that reading, and `karakuri-cli` makes
        // the same one: a map file means one thing in both programs or it
        // means nothing.
        surface.take(
            gfx.engine.deck.slot_count(),
            &midi::Decks(&gfx.engine.deck),
            &mut asked,
        );
        for operation in asked.drain(..) {
            let repaint = App::performed(
                gfx,
                started,
                readout,
                recording.recorder(),
                &Acted::Emitted(Some(operation)),
                Repaint::Never,
            );
            App::wants(gfx, egui_due, costs, repaint);
        }
        gfx.performed_by_hand = asked;
        App::showed(gfx);
    }

    /// **The surface is shown where the deck is** — MIDI out, the other
    /// direction of [`App::mapped`] and the send beside its drain.
    ///
    /// **After the frame's operations have been applied**, so a motorised
    /// fader follows the value the deck holds rather than the one it was asked
    /// for — and after a learn too, because a knob just bound has never been
    /// shown and the control it took over may have been lit on another knob.
    ///
    /// **Every source is shown and not just this surface's own**, which is the
    /// whole reason MIDI out is worth having: a key, a model over `--mcp`, the
    /// pointer on a strip and a transition all move a fader, and *two things
    /// can move a fader* is the sentence `docs/roadmap.md` gives this row.
    ///
    /// **It does not wait** (P-0094): `Surface::show` queues into a bounded
    /// channel and drops when it is full rather than blocking the render
    /// thread — `karakuri_environment::midi`, where that whole argument lives.
    /// **And it writes no record**: what changes is the wire, so a session
    /// recorded from this surface still replays with neither surface nor map
    /// attached (P-0092).
    fn showed(gfx: &mut Gfx) {
        let Gfx { midi, engine, .. } = gfx;
        let Some(surface) = midi.as_mut() else {
            return;
        };
        surface.show(&midi::Lit {
            deck: &engine.deck,
            exposure: engine.look.exposure,
        });
    }

    /// **A knob turned while `learn` is lit binds the control under the
    /// pointer to it**, and says what happened.
    ///
    /// # The gesture is three things and the panel already knew two
    ///
    /// *Arm, point, turn.* The pill is the arming; [`Hover::resting`] is what
    /// the pointer is on, which the tooltip layer works out anyway; and
    /// [`asked_at`] turns that into the operation a press there would ask for,
    /// which [`target_of`] spells as the right-hand side of a map line. What
    /// is left is the left-hand side, and that is the message that just
    /// arrived.
    ///
    /// # It stays armed until it is pressed again
    ///
    /// Mapping a surface is *turn every knob once*, a dozen bindings in a row,
    /// and re-arming between each would be a click per knob. Nothing is hidden
    /// by that — the pill is lit for exactly as long as this is true.
    ///
    /// # Every refusal is out loud
    ///
    /// A knob turned with the pointer on nothing, on a control no map line can
    /// name, or against a map file that will not open — each says so and
    /// leaves the arming alone. **A learn that quietly did nothing is the one
    /// outcome
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// rules out**, and it is the likely one: the operator is looking at the
    /// controller rather than the screen.
    ///
    /// # And the frame is owed
    ///
    /// The tip under the pointer has just changed — its last line is read off
    /// the live map — so the layer is asked for a frame. That is the whole of
    /// what *the assignment shows at once* takes.
    fn learned(
        gfx: &mut Gfx,
        readout: &mut Readout,
        hover: &karakuri_console::hover::Hover,
        maps: &std::path::Path,
        egui_due: &mut Option<Instant>,
        costs: &mut Costs,
    ) {
        let ctx = gfx.egui.egui_ctx().clone();
        let Gfx { midi, engine, .. } = gfx;
        let Some(surface) = midi.as_mut() else {
            return;
        };
        let Some(message) = surface.learning() else {
            return;
        };
        let Some((p, _)) = hover.resting() else {
            println!(
                "learn: something moved, and the pointer is not on a control. point at the                  control you want on that knob and move it again — the pill stays lit until                  you press it."
            );
            return;
        };
        let target = asked_at(&readout.panel, &ctx, &readout.view, p)
            .ok_or_else(|| {
                String::from(
                    "learn: the pointer is on nothing this panel can name. it has to be on a                      control, not beside one.",
                )
            })
            .and_then(|operation| target_of(&operation, &engine.deck));
        let target = match target {
            Ok(target) => target,
            Err(why) => {
                println!("{why}");
                return;
            }
        };
        match surface.learn(message, &target, maps) {
            Ok(line) => println!(
                "learn: `{line}` — written to {}. the control's tooltip says so now, and the                  map is loaded from there on the next start.",
                maps.display()
            ),
            Err(why) => println!("learn: {why} — nothing was bound and nothing was written"),
        }
        // The tip under the pointer has a different last line now.
        App::wants(gfx, egui_due, costs, Repaint::Now);
    }

    /// **Act on a repaint decision, and the only place a frame is asked for
    /// outside `to_egui` and `missed`.**
    ///
    /// Three answers and three actions: ask for a frame, note a deadline, or
    /// do nothing at all — and the third is the one ADR-0164's still-panel
    /// clause is made of.
    ///
    /// It takes the two fields rather than `&mut self` so that a caller
    /// holding `self.gfx` can still reach `self.egui_due`.
    pub(crate) fn wants(
        gfx: &Gfx,
        egui_due: &mut Option<Instant>,
        costs: &mut Costs,
        repaint: Repaint,
    ) {
        match repaint {
            Repaint::Never => {}
            Repaint::Now => {
                costs.owes();
                gfx.window.request_redraw();
            }
            Repaint::After(delay) => {
                let due = Instant::now() + delay;
                *egui_due = Some(match *egui_due {
                    Some(had) => had.min(due),
                    None => due,
                });
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("The Karakuri console")
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW.0, WINDOW.1))
            // **The panel is not dragged under its own arrangement.**
            // `karakuri_console::MINIMUM_VIEWPORT` is the declared minima
            // summed along each axis — **777 x 658.5** — and below it the
            // solve stops honouring them and scales everything down together
            // (ADR-0250), which takes the Mixer's strips off the panel while
            // the deck previews stay: the pointer can no longer select a deck
            // and `0`..`3` still can.
            //
            // **The width is the body row's three tracks plus its two column
            // dividers**: `left-pane` 160, `centre` 425, `right-pane` 172,
            // `+ 10 + 10`. The centre is the term that moved — it was 340,
            // which was `.body-grid`'s CSS track rather than a reading of
            // what the console draws, and it is now an inspector pane's own
            // minimum twice over one pane divider, `2 x 208 + 9`. At 340 a
            // pane is 165.5 where a parameter row's fixed tracks want 207
            // before the fader has any width, so the faders were not drawn at
            // the centre's declared minimum — and a divider drag reaches that
            // centre at any window width, so no window minimum could close
            // it. **A pane that cannot draw a fader is not a minimum**
            // (ADR-0279), which is the change; the height is unmoved.
            //
            // **This comment said 692 x 658.5 until 2026-09-08**, and the
            // arithmetic behind it went with the total. The one place either
            // figure is stated is `karakuri_console::MINIMUM_VIEWPORT`, whose
            // own documentation carries every term, and
            // `karakuri-console`'s `tests/arrangement.rs` recomputes both from
            // the tree.
            //
            // The units are the same on both sides — the viewport handed to
            // `Panel::set_viewport` below is this window's inner size divided
            // by the scale factor. A screen narrower than this leaves the
            // window larger than the screen, which is an ordinary state and
            // not a failure (ADR-0272).
            .with_min_inner_size(winit::dpi::LogicalSize::new(
                karakuri_console::MINIMUM_VIEWPORT.0,
                karakuri_console::MINIMUM_VIEWPORT.1,
            ));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let size = window.inner_size();
        self.scale = window.scale_factor();

        let instance = Gpu::instance();
        // **Reported rather than panicked, and both of these can fail for one
        // reason.** A panic here is reached from a `winit` callback and cannot
        // unwind across the Objective-C frame on macOS, so it aborts — with
        // `<unknown>` for every frame of the backtrace and no sentence
        // anywhere saying what went wrong.
        //
        // The surface is the one that goes first when a backend was asked for
        // and the machine has none of it: an instance with only that backend
        // enabled has nothing that can make a surface, so the failure arrives
        // as `FailedToCreateSurfaceForAnyBackend` before any adapter is
        // requested. That is the exact path
        // `docs/adr/0168-a-backend-override-is-honoured-because-a-no-op-cannot-be-caught.md`
        // opened, so it names the variable first.
        let surface = match instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(e) => no_gpu(&format!("no surface: {e}")),
        };
        let gpu = match pollster::block_on(Gpu::from_instance(instance, Some(&surface))) {
            Ok(gpu) => gpu,
            Err(e) => no_gpu(&format!("no adapter: {e}")),
        };

        let caps = surface.get_capabilities(&gpu.adapter);
        // **A non-sRGB format, and that is the opposite of what the program
        // this replaces wanted.** `egui`'s own shader encodes: it is told the
        // target is gamma space and writes gamma-encoded texels, so a surface
        // that also encoded on write would encode twice and wash the panel
        // out. P-0064 says sRGB is encoded once at final output, and for this
        // window the toolkit is that output.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&gpu.device, &config);

        let ctx = egui::Context::default();
        let egui = egui_winit::State::new(
            ctx,
            egui::ViewportId::ROOT,
            &window,
            Some(self.scale as f32),
            None,
            Some(gpu.device.limits().max_texture_dimension_2d as usize),
        );
        let renderer =
            egui_wgpu::Renderer::new(&gpu.device, format, egui_wgpu::RendererOptions::default());

        self.readout.panel.set_viewport(
            size.width as f32 / self.scale as f32,
            size.height as f32 / self.scale as f32,
        );

        // The picture's first size is the region's, at the window this opened
        // at — not the window's, and not a guess that the first frame then
        // corrects. **It is the same call the frame makes**: `Engine::new`
        // aims both sinks through `aims`, so this and `RedrawRequested` cannot
        // disagree about which rectangle a texture is sized from.
        let mut renderer = renderer;
        self.readout.panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &self.running,
            self.readout.panel.layout(),
            self.scale as f32,
            Some((std::sync::Arc::clone(&self.held), self.built_tx.clone())),
            Some(self.snapshots.clone()),
            self.pointing.clone(),
        );
        // **What every deck is playing, seeded from the compile that just
        // built them**, before a frame has run — see [`Playing::at_launch`].
        // It is written here rather than in [`App::new`] because the nodes are
        // the engine's compile, and it is written on *every* remake for the
        // same reason: a window remade rebuilds the deck from the launch pair,
        // so what each slot is running goes back to what it was seeded with.
        self.keeping.playing = Playing::at_launch(&engine.placed, engine.deck.slot_count());
        // **Before the first frame and before the first strip is written**, so
        // that the panel's first frame draws the deck as it actually is rather
        // than a settled version of it that the second frame corrects.
        let governed = engine.ask_to_prime(&gpu);
        // **The four risk badges, from the pass that just decided them.** The
        // dot is as fresh as the last governor pass and no fresher: a Set that
        // swaps in arrives unestimated and a resize drops the estimate
        // (ADR-0296), so this is written again wherever a later `Deck::govern`
        // report is kept.
        self.readout.view.costs = costs(&governed);
        let info = gpu.adapter.get_info();
        self.costs.taken_on = format!(
            "{:?} — {} ({:?})",
            info.backend, info.name, info.device_type
        );

        let budget = budget_ms(&window);
        // **And the same interval is what a candidate Set is judged against.**
        // The two used to be different numbers with the same word on them: this
        // row's budget was the display's real interval and the swap watchdog's
        // was `DEFAULT_BUDGET_MS`, 20, transcribed in [`watched`] because a
        // `HotSwap` is built before there is a window to ask. They are the same
        // question now — *how long may one frame take* — because ADR-0313 made
        // the watchdog compare one frame of one Set rather than a median of the
        // deck's intervals, so the honest right-hand side is the deadline the
        // display actually imposes.
        //
        // **Where `winit` will not say, the constant stands**, which is what
        // `set_frame_budget_ms` does with a `None` here: a monitor it cannot
        // name or a mode with no refresh rate is not a licence to invent a
        // plausible 16.6
        // (`docs/principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md`).
        //
        // **Read once, when the window opens**, on [`budget_ms`]'s own terms —
        // a window dragged onto a 120 Hz display keeps the interval it opened
        // on, and the watchdog now inherits that limitation exactly as the row
        // above it has it.
        if let Some(budget) = budget {
            engine.deck.set_frame_budget_ms(budget);
        }
        // **The strips before the legend**, because the legend says how many
        // there are and the answer is the deck's rather than a guess. It is
        // written again on every frame; this is the first one.
        // **Every slot opens on the same pair**, because that is what this
        // program builds them from — one name repeated rather than one name
        // shared, so that a load can move one of them without moving the
        // other's readout. See [`Gfx::material`].
        let material: Vec<String> =
            std::iter::repeat_n(self.sources.material(), engine.deck.slot_count()).collect();
        mixer(&engine.deck, &material, &mut self.readout.view.mixer);
        // **The library before the legend too**, and once for the run: the
        // legend says how many Sets the bay lists, and `library` says why
        // where it is none.
        //
        // **The scopes first, because a listing belongs to one of them.** The
        // console draws the chips it is handed and this program is what can
        // answer them — a store, a told directory, and two that answer nothing
        // yet (`why_nothing`). All four are drawn: a chip is the question, and
        // three of the four questions are ones this program can be asked.
        self.readout.view.scopes = Scope::ALL.to_vec();
        // **And it opens on `all`, which is where `my sets` used to be.**
        // The mark says which question is being asked, so the one to open on
        // is the one whose answer is the library itself: `my sets` is the
        // starred subset now (ADR-0299), so a fresh store opening there would
        // draw an empty bay over a library full of Sets. The console refuses a
        // scope it was not handed, so this is asserted rather than assumed.
        //
        // **The state and not the move.** `View::select_scope` answers whether
        // the mark *moved*, and `all` is the first chip and the console's own
        // default, so on a fresh run it has not moved and the answer is
        // `false` — which is the console agreeing rather than refusing.
        // Asserting the return value aborted the program on every launch
        // between this line landing and 2026-09-08, with every test in the
        // workspace green: nothing in the suite opens a window, so nothing ran
        // this line. What is worth asserting is that the mark is where this
        // says it is, which is true whether or not it had to move.
        self.readout.view.select_scope(Scope::AllSets);
        assert_eq!(
            self.readout.view.scope(),
            Some(Scope::AllSets),
            "the console was handed the four scopes and does not have `all` marked"
        );
        println!(
            "{}",
            listing(
                &mut self.readout.view,
                &self.store,
                self.presets.as_ref(),
                self.folder.as_deref(),
                // **No Set, and it is the answer rather than a value not to
                // hand**: every slot launches on the pair the command line
                // settled, so nothing is running a Set until somebody loads
                // one (ADR-0304), and the mark is on `all` two lines up
                // either way.
                None,
            )
        );
        // **And the arrangement pill's menu, once for the run**, for
        // `library`'s reason and for one more: this is a directory read, and
        // the only thing that can add a name to it is a save this program
        // performs — which re-reads it there. See `arrangements`.
        self.readout.view.arrangement.filed = arrangements(&self.store);
        // **And the Inspector's panes, before the first frame.**
        // `Set::published` says it is not for the frame path but *is* what a
        // console reads when a Set lands, and every slot is watched — so this
        // is the first of those readings rather than the only one, and the
        // frame handler takes the rest. See `inspector`, which is also where
        // the controls it could not place are reported.
        let targets = self.readout.view.pane_decks();
        inspector(
            &engine.deck,
            &material,
            &engine.aimed,
            targets,
            &mut self.readout.view.inspector,
        );
        // **And the Master bay's level, for the strips' reason.** The legend
        // reports what each bay draws by asking the view, so a bay whose level
        // has not been written yet reports itself as having no engine behind
        // it — on a run that has one, and over a fader a hand can take hold
        // of. It is written again on every frame; this is the first.
        self.readout.view.master_out = Some(engine.deck.out());
        // And the three rows under it, off the `Present` that holds them — the
        // same seam one row down, and the reading rather than the state
        // (ADR-0156).
        self.readout.view.master_chain = Some(chain_view(&engine.chain));
        // **And the room, before the legend**, because the legend says which
        // input is open and the answer is the host's rather than a sentence
        // here. The session tempo is the deck's own oscillator: it is what the
        // grid free-runs at and where the tracker's octave window starts, and
        // they are one number because they are one statement.
        let (audio, said) = listening(engine.deck.signals().oscillator().bpm());
        println!("{said}");
        // **The pill is told even where nothing opened**, which is the
        // distinction `View::audio` exists to draw: `Some(AudioIn)` with no
        // device is a program that looked and found nothing and draws
        // `audio-in · none`, where `None` would be a console nobody had told
        // and would draw no pill at all — on a program that did look.
        self.readout.view.audio = Some(told(audio.as_ref()));
        // **And what the other three controls in that group read**, for the
        // Master bay's level's reason one bay over: the legend reports what
        // each bay draws by asking the view, so a group whose values have not
        // been written yet reports itself as not drawn — on a run that draws
        // it. It is written again on every frame; this is the first, and the
        // tempo is the same oscillator `listening` was told about.
        self.readout.view.tracker = Some(tracking(
            audio.as_ref(),
            engine.deck.signals().oscillator().bpm(),
        ));
        // **And the surface, beside the room and for its reason**: it is a
        // door this window opens at startup rather than a flag, and which one
        // it got is a sentence rather than a description of a search. The map
        // is resolved here because both tiers are this program's own
        // directories — the store it was given and the preset library it
        // found — and `karakuri-environment` is handed the answer rather than
        // the question (`places`' own rule: each binary keeps its parser).
        let map = midi::map_for(
            &self.store,
            self.presets.as_ref().map(|presets| presets.dir.as_path()),
        );
        // `controller` rather than `surface`, which in this function is the
        // swapchain's.
        let (controller, plugged) = surfaced(map.as_deref(), self.waker.clone());
        println!("{plugged}");
        // **The `map` pill is told, and only where there is a surface** —
        // `View::map`'s own rule, which is `audio-in`'s one pill along:
        // `Some(MapPill::NONE)` is a program that opened a port and found no
        // map, and draws `map · none`; `None` is a program with no surface at
        // all, which draws neither this pill nor `learn`. A console told
        // nothing would be this program answering a question about a device on
        // the console's authority.
        self.readout.view.map = controller.as_ref().map(|open| view::MapPill {
            name: open.map_name().map(str::to_owned),
        });
        // **The port the server bound, asked of the server.** `--mcp 0` takes
        // an ephemeral port, so the flag's argument and the address a client
        // dials are two different numbers on that run; `Reporter::port` is the
        // one `main` already printed and is the only one worth a legend.
        let mcp_port = self.keeping.mcp.as_ref().map(mcp::Reporter::port);
        self.readout.print_legend(
            budget,
            &governed,
            self.presets.as_ref(),
            &self.store,
            mcp_port,
        );

        // The first frame is owed to the window appearing, not drawn on a
        // still panel.
        self.costs.owes();
        window.request_redraw();
        self.gfx = Some(Gfx {
            audio,
            midi: controller,
            // **A frame's worth of a surface's fastest gesture is single
            // figures**, and this is the buffer the drain fills — sized once
            // so the frame path never `realloc`s, which is
            // `karakuri_environment::midi`'s `INBOX` on this side of the
            // channel and the same rule.
            performed_by_hand: Vec::with_capacity(MAPPED),
            budget_ms: budget,
            launch: self.sources.material(),
            presets: self.presets.as_ref().map(|presets| presets.dir.clone()),
            material,
            store: self.store.clone(),
            window,
            // **A run opens with one output.** The projector is a window an
            // operator asks for from the Outputs row; opening one nobody asked
            // for would put a second window on their desk and raise what every
            // frame costs before the first one is drawn.
            projector: None,
            gpu,
            surface,
            config,
            egui,
            renderer,
            engine,
        });
    }

    /// **Where a deadline comes due**, which is the start of every iteration
    /// the loop makes — including the one a `ControlFlow::WaitUntil` woke it
    /// for.
    ///
    /// Both deadlines are checked whatever the [`StartCause`] rather than only
    /// on `ResumeTimeReached`: a wait that is cancelled early by a real event
    /// still has to leave a due deadline serviced, and checking two `Instant`s
    /// costs nothing.
    // **The loop is used now**, and it stopped being `_event_loop` on
    // 2026-09-10: [`App::operated`] is drained here and a model's
    // `RouteFrame` opens a projector window, which `winit` will not make
    // without one (ADR-0341).
    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
        let now = Instant::now();
        if self.egui_due.is_some_and(|due| due <= now) {
            self.egui_due = None;
            if let Some(gfx) = self.gfx.as_ref() {
                // One frame, now that the delay `egui` asked for has passed.
                gfx.window.request_redraw();
            }
        }
        // **Only once there is something to describe.** The reading names the
        // workload it was taken over, and that is the run's `.kir` pair rather
        // than a constant — so it is taken when the engine exists, and not
        // before. A deadline that comes due first is not lost: `due()` goes on
        // returning it until the reading is printed.
        if self.costs.due().is_some_and(|due| due <= now) {
            if let Some(gfx) = self.gfx.as_ref() {
                // **The reading names the workload it was taken over**, and
                // that is the whole deck's rather than one slot's — so the
                // slots' names are joined in slot order, and a run where a
                // load has moved one of them says so instead of naming the
                // pair the window opened with.
                let (capacity, material) = (gfx.engine.capacity, gfx.material.join(" / "));
                // **The refresh interval, because it is what tells the two
                // waits apart.** A period sitting at the display's interval is
                // a loop with headroom; one well past it is a loop at its
                // limit, and a host clock cannot say which without it.
                // **The size the reading is about, read off the `Present`
                // that took it** — the largest enabled output's, which is what
                // the frame was composited at while these numbers were being
                // measured.
                let at = gfx.engine.present.size();
                self.costs.say(capacity, &material, gfx.budget_ms, at);
            }
        }
        // **What a model asked for, taken on the wake it asked to be taken
        // on** — see [`SERVED`], where the whole of this is argued. It is here
        // beside the two deadlines above because it is a third one, and
        // `about_to_wait` is where all three are turned into a control flow.
        if self.served.is_some_and(|due| due <= now) {
            self.served = Some(now + SERVED);
            if let Some(gfx) = self.gfx.as_mut() {
                self.keeping.requests(&mut gfx.engine, &self.store);
                // **And every operation a model named, on the same wake and
                // beside the same drain** — see [`App::operated`], which is
                // where the reason it is a second call rather than a third arm
                // of `requests` is written.
                App::operated(
                    gfx,
                    event_loop,
                    self.started,
                    &mut self.readout,
                    &mut self.recording,
                    &mut self.keeping,
                    &self.store,
                    &mut self.egui_due,
                    &mut self.costs,
                );
                // **And a kept procedure with them**, drained beside the saves and
                // for their reason: the two acts both end on a disk, and the
                // Library bay lists what both of them wrote. `|` and not `||`,
                // so the second drain runs whether or not the first landed
                // anything — a short-circuit here would leave a keep's outcome
                // in its channel until a save happened to arrive.
                if self.keeping.finished_saves() | self.keeping.finished_keeps() {
                    let running = aimed_set(gfx, &self.readout.view);
                    println!(
                        "{}",
                        listing(
                            &mut self.readout.view,
                            &self.store,
                            self.presets.as_ref(),
                            self.folder.as_deref(),
                            running.as_deref(),
                        )
                    );
                }
                // **A frame, because a build lands at a frame boundary and
                // nowhere else.** A write a model made is on disk, compiled on
                // a worker and waiting for `Deck::begin_frame`; a run that
                // answered the write and never drew would go on showing what it
                // was showing.
                gfx.window.request_redraw();
            }
        }
    }

    /// **The one place the control flow is set, and it is a deadline or
    /// nothing.**
    ///
    /// `Wait` is a window that costs the machine nothing at all until somebody
    /// touches it, which is ADR-0164's still-panel clause as the operating
    /// system sees it. `WaitUntil` is the soonest of the three things that are owed at a
    /// time rather than on an event: the frame `egui` asked for after a delay,
    /// the reading `Costs` takes once the window has been still long enough,
    /// and — on a run with `--mcp` — the wake that takes what a model asked for
    /// ([`SERVED`]). None is `Poll`, and nothing here asks for a frame in order
    /// to have something to measure.
    ///
    /// **The third one is the only one that can be owed forever**, and that is
    /// what a served run is: something outside this process is driving the
    /// instrument, so the window is being touched even though nobody is at it.
    /// **A thread that is not this one said there is something to drain**, and
    /// there is exactly one of them: the MIDI callback — see [`App::waker`].
    ///
    /// **It asks for a frame and does nothing else.** The drain itself is
    /// [`App::mapped`], at the top of `RedrawRequested` beside the other two,
    /// which is what makes a sweep spanning several wakes one operation on one
    /// frame instead of a partial apply per message. The wake carries no
    /// payload for the same reason: what arrived is the port's to say and this
    /// loop's only job is to run again.
    ///
    /// **`request_redraw` and not a repaint decision**, because there is
    /// nothing yet to decide about — whether the frame changes anything is
    /// what `performed` answers on the frame this asks for, and `costs.owes`
    /// is what says the frame was owed to an event rather than to a still
    /// panel.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _wake: ()) {
        if let Some(gfx) = self.gfx.as_ref() {
            self.costs.owes();
            gfx.window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let next = [self.egui_due, self.costs.due(), self.served]
            .into_iter()
            .flatten()
            .min();
        event_loop.set_control_flow(match next {
            Some(at) => ControlFlow::WaitUntil(at),
            None => ControlFlow::Wait,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(gfx) = self.gfx.as_mut() else {
            return;
        };
        // **The projector's own events, and they are three.** This program had
        // one window until 2026-09-09 and the id was `_id`; a second window
        // means every arm below has to be about the panel, so the projector's
        // are taken here and returned from rather than falling through into a
        // handler that would resize the panel's swapchain from another
        // window's size.
        //
        // - **`Resized`** is `docs/adr/0246-…` on this window: the destination
        //   decides, so the swapchain follows and `Projector::size` with it —
        //   and the next frame's `render_size` sees the new size. It is not a
        //   redraw request, because this loop's frames are the panel's: the
        //   panel asks for the frame and every sink in the slice gets it, so
        //   what this owes is a frame *asked of the panel*.
        // - **`CloseRequested`** turns this output off rather than quitting.
        //   The window manager's close on a projector is *stop sending to the
        //   projector*, and quitting the program because an operator shut a
        //   second window would be the worst answer a live instrument could
        //   give (P-0094).
        // - **`RedrawRequested`** is answered by doing nothing. A frame for
        //   this window is composed by the panel's own redraw, into the sink
        //   in the slice; drawing here would be a second submission over the
        //   deck's targets, which is exactly the race ADR-0166 is about.
        if gfx.projector.as_ref().is_some_and(|p| p.window.id() == id) {
            match event {
                WindowEvent::Resized(size) => {
                    let at = (size.width.max(1), size.height.max(1));
                    if let Some(projector) = gfx.projector.as_mut() {
                        projector.sink.resize(&gfx.gpu.device, at.0, at.1);
                        projector.size = at;
                    }
                    App::wants(
                        gfx,
                        &mut self.egui_due,
                        &mut self.costs,
                        Change::Viewport.repaint(),
                    );
                }
                WindowEvent::CloseRequested => {
                    if let Some(line) = routed(gfx, event_loop, Output::Projector(0), false) {
                        println!("{line}");
                    }
                    self.readout.view.projector = false;
                    App::wants(
                        gfx,
                        &mut self.egui_due,
                        &mut self.costs,
                        Change::Viewport.repaint(),
                    );
                }
                _ => {}
            }
            return;
        }
        // **The stillness clock, and it is reset by everything except a frame
        // this loop asked for itself.** A frame drawn while this has not been
        // reset is a frame drawn on an untouched window, which is the number
        // the reading is about; anything arriving from the platform — a
        // pointer, a key, a move, a focus, an occlusion — is the window being
        // touched. Resetting too eagerly only makes the reading harder to
        // reach, never easier to pass.
        if !matches!(event, WindowEvent::RedrawRequested) {
            self.costs.touched();
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.event_loop_action(id, &event) == EventLoopAction::Exit {
                    self.keeping.awaited_saves();
                    // **The one place a stall is welcome**, which is
                    // `karakuri-cli`'s own words for the same call in `exiting`:
                    // every frame has been drawn and the run is over. A recording
                    // still open is stopped and waited for here, so what was
                    // written and what was lost are said rather than left to a
                    // `Drop` that flushes and reports nothing.
                    //
                    // **After the saves**, for their reason read the other way: a
                    // save that landed in the last second is answered before the
                    // window goes, and a session is what an operator will look for
                    // afterwards.
                    self.recording.awaited();
                    event_loop.exit()
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor;
                App::to_egui(gfx, &mut self.costs, &event);
                // **A resize that arrives without a redraw request of its
                // own.** The arrangement is stated in logical pixels, so the
                // same window is a different viewport at a different scale.
                // macOS follows this event with a `Resized` and the viewport
                // is set there — but *usually followed by* is a platform's
                // habit rather than a guarantee, and what it would leave
                // behind is a panel drawn at the wrong scale with nothing
                // anywhere saying so. So the frame is asked for here too.
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Viewport.repaint(),
                );
            }
            WindowEvent::Resized(size) => {
                gfx.config.width = size.width.max(1);
                gfx.config.height = size.height.max(1);
                gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                let (w, h) = (
                    size.width as f32 / self.scale as f32,
                    size.height as f32 / self.scale as f32,
                );
                self.readout.panel.set_viewport(w, h);
                println!("viewport: {w:.0} x {h:.0}");
                App::to_egui(gfx, &mut self.costs, &event);
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Viewport.repaint(),
                );
            }

            // -- the three events the rule is about -----------------------
            // Each one asks `Readout::pointer` who it belongs to and hands it
            // to `egui` only if the answer is `egui`.
            WindowEvent::CursorMoved { position, .. } => {
                let p = Point::new(
                    (position.x / self.scale) as f32,
                    (position.y / self.scale) as f32,
                );
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, Pointer::Moved(p));
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                // **A move can now change the mix**, which no pointer event
                // could before: a fader in hand turns this move into one
                // operation of the vocabulary. What is owed for it is what the
                // drag asked for and not the claim — a fader held against the
                // top of its track asks for 1.0 sixty times a second and
                // changes nothing, and `Change::Pointer(Panel)` would draw a
                // frame for every one of them.
                let repaint = App::performed(
                    gfx,
                    self.started,
                    &mut self.readout,
                    self.recording.recorder(),
                    &acted,
                    Change::Pointer(claim).repaint(),
                );
                // **And the hover layer is told where the pointer went**, with
                // the claim `input::claim` has just answered: `Claim::Egui` is
                // the panel saying the pointer is on none of its controls, so
                // the common move costs one comparison there. What comes back
                // is a frame owed **now** only where a tip is on screen that
                // must not be — the dwell itself is a deadline and is asked
                // for on the frame, beside `View::animating`.
                let tip = self.hover.moved(
                    claim,
                    &self.readout.panel,
                    &ctx,
                    &self.readout.view,
                    p,
                    self.started.elapsed(),
                );
                let repaint = repaint.soonest(Change::Tip(tip).repaint());
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            // **The pointer left the window**, which is not a move to
            // anywhere: a tip that is up goes, and no dwell is running. Only
            // the hover layer cares — `egui` is told either way, because this
            // event is not one `input::claim` has a rule about.
            WindowEvent::CursorLeft { .. } => {
                App::to_egui(gfx, &mut self.costs, &event);
                let tip = self.hover.left();
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Tip(tip).repaint(),
                );
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let which = match state {
                    ElementState::Pressed => Pointer::Down,
                    ElementState::Released => Pointer::Up,
                };
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, which);
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                // **A press that named a scope is a listing to read**, and
                // it is read here because this is where the store is: a scope
                // *is* a listing on this side, and a directory read is not a
                // thing to do on a frame (P-0091). It is read on **every**
                // chip press and not only on one that moved the mark, which is
                // the one place this parts company with `e`: the key steps and
                // so a press that changed nothing asked for nothing, where a
                // pointer *names* — and naming the library you are already
                // reading is asking it again, which is a question this console
                // had no way to put before.
                // **And a press that narrowed one is the same question asked
                // of the other half**, so it is the same branch: a scope and a
                // filter both change what the store is being asked, and the
                // answer to either is a directory read this side owns.
                // **And a press on a star is a file beside the Sets to
                // write**, taken before the re-read below because the listing
                // it re-reads is the one this write changes: `my sets` is the
                // starred subset (ADR-0299), so a star taken off under that
                // chip is a row that leaves. See [`favourite`], which answers
                // `None` for every other operation and writes nothing else.
                // **A chip in the Outputs row that is not the picture**, and
                // it is taken here because this is where the event loop is:
                // `winit` will not make a window without one, and
                // `App::performed` below is handed a `Gfx` and no loop. See
                // [`routed`], which is the whole of what a projector costs
                // this file.
                if let Acted::Emitted(Some(Operation::RouteFrame { output, on })) = acted {
                    if let Some(line) = routed(gfx, event_loop, output, on) {
                        println!("{line}");
                    }
                }
                if let Acted::Emitted(Some(ref operation @ Operation::SetFavourite { .. })) = acted
                {
                    // **`Asked::Operator`, because a hand on this panel is the
                    // operator's own act** — the same word `k` and the `keep`
                    // capsule pass for a save, and what it decides is
                    // `favourite`'s own (P-0096, ADR-0301).
                    if let Some(line) = favourite(&self.store, Asked::Operator, operation) {
                        println!("{line}");
                    }
                }
                // **A star is on this branch as well as the two above**, and
                // it is the same question for the same reason: what the bay
                // draws is a listing and a set of marks, both of them read off
                // a disk, and the write above changed one of them.
                // **And a press on the `history` chip is a walk of the store**,
                // which is the same branch because it is the same act: the
                // fifth chip marks a scope like the four beside it and asks for
                // a different row (ADR-0308), so a press that emitted
                // `WalkHistory` and did not re-read left the bay drawing the
                // listing it had before under a mark that says `history`. It
                // was missing here until 2026-09-10 and is `docs/adr/0342-…`'s
                // own defect.
                if matches!(
                    acted,
                    Acted::Emitted(Some(
                        Operation::SelectScope { .. }
                            | Operation::WalkHistory { .. }
                            | Operation::ListSets { .. }
                            | Operation::SetFavourite { .. }
                    ))
                ) {
                    // **Whose history, read before the listing is rewritten**:
                    // a scope press can be the one that marks `history`, and
                    // what that scope lists is the Set the load pulldown's deck
                    // is running (`aimed_set`).
                    let running = aimed_set(gfx, &self.readout.view);
                    println!(
                        "{}",
                        listing(
                            &mut self.readout.view,
                            &self.store,
                            self.presets.as_ref(),
                            self.folder.as_deref(),
                            running.as_deref(),
                        )
                    );
                }
                // **And a press that asked to read a Set is a Set file and its
                // cards to read**, on the same branch and for the same
                // reason: the store is here, a file read is not a thing to do
                // on a frame (P-0091), and `karakuri-console` reaches no disk
                // at all (ADR-0156). What the console holds is the answer and
                // whether the block is down — `view::View::reading`.
                if matches!(acted, Acted::Emitted(Some(Operation::ReadSet { .. }))) {
                    println!("{}", read_reading(&mut self.readout.view, &self.store));
                }
                // **And a press that asked to keep a deck is a Set to
                // write**, here for the reason the two above are: the engine
                // and the store are the window's, and a disk write is not a
                // thing to do on a frame (P-0091). It is `k`'s own call with
                // the deck the *pill* named rather than the one the selection
                // is on, and `Asked::Operator` because a hand on this panel is
                // the operator's own act.
                if let Acted::Emitted(Some(Operation::SaveSet { deck, ref id })) = acted {
                    self.keeping.save_set(
                        &gfx.engine,
                        &self.store,
                        Asked::Operator,
                        usize::from(deck),
                        id.clone(),
                        None,
                    );
                }
                // **And a press that asked to keep a node's procedure is one
                // file to write**, here for the Set keep's reason one line up:
                // the bytes are the run's, the store is the window's, and a
                // disk write is not a thing to do on a frame (P-0091).
                //
                // **`Asked::Operator`, so it lands in `<store>/procedures/`**
                // — a hand on this panel is the operator's own act, which is
                // what makes that tier exist (P-0096). A model's arrives
                // through the operate drain and carries `Asked::Model`.
                //
                // **The id is the pane head's if one is being typed there.**
                // The capsule emits `None`, because it is the press that types
                // nothing (ADR-0128); the head three items along is this
                // console's second letter-taking flow, and a keep sent while
                // that head is asking files under what was typed. The head is
                // looked up by the deck the operation names rather than by the
                // pane the capsule was drawn in, for `View::named_set`'s own
                // reason: the gesture spans frames and the deck is read at the
                // commit.
                if let Acted::Emitted(Some(Operation::KeepProcedure { deck, node, ref id })) = acted
                {
                    let id = id.clone().or_else(|| self.readout.view.naming_over(deck));
                    self.keeping.keep_procedure(
                        &gfx.engine,
                        &self.store,
                        Asked::Operator,
                        usize::from(deck),
                        node,
                        id,
                        None,
                    );
                }
                // **And a press that asked to send a Set is a dialog to open
                // and a file to write**, here for the reason the three above
                // it are: the store is the window's, a bundle is a store read
                // and a file written, and neither is a thing to do on a frame
                // (P-0091, ADR-0156). What this side adds to the operation is
                // the destination, which the operation deliberately does not
                // carry (ADR-0260): a read's answer goes where the surface
                // that asked puts answers, and this surface asks the platform.
                //
                // **Both buttons reach this line**, because the item is picked
                // by whichever press lands on the card while it is down — see
                // the `Secondary` arm below, which calls the same function.
                if let Acted::Emitted(Some(Operation::TransferSet {
                    transfer: SetTransfer::Send { ref id },
                })) = acted
                {
                    println!("  send: naming a file to write `{id}` to");
                    sending(
                        &gfx.window,
                        &self.store,
                        self.folder.as_deref(),
                        id,
                        self.keeping.send_tx.clone(),
                    );
                }
                // **And a press on the `rec` pill is a recording started or
                // stopped**, here for the reason the three above it are: the
                // engine and the store are the window's, and neither end of
                // this is a thing to do on a frame (P-0091) — a start writes
                // the Set file a replay reconstructs the session from, and a
                // stop blocks on the writer thread. The reading is taken here
                // and every byte of I/O is on a thread of its own; see
                // [`Sessions`].
                if let Acted::Emitted(Some(Operation::RecordSession { ref recording })) = acted {
                    self.recording
                        .asked(&self.keeping, &gfx.engine, &self.store, recording);
                }
                // **And a press that moved the library cursor owes that same
                // read** (ADR-0265): [`reread_if_open`] is the arrow keys'
                // own call one event along, and it is here for the reason the
                // two calls above it are — the store is the window's, a file
                // read is not a thing to do on a frame (P-0091), and
                // `karakuri-console` reaches no disk at all (ADR-0156).
                //
                // **The press still names no operation.** `read_reading`
                // emits none, and `Acted::Pointed` is not an `Acted::Emitted`.
                if let Some(line) = reread_if_open(
                    matches!(acted, Acted::Pointed),
                    &mut self.readout.view,
                    &self.store,
                ) {
                    println!("{line}");
                }
                // **A Set dropped out of `presets` is taken in before it is
                // loaded**, which is the load's two-moment press arriving at the
                // pointer: a preset row names a file and a load names an id,
                // and taking it in is what gives the Set the id the load needs
                // (ADR-0229). Two routes to one row have to reach the same
                // place — `console.html`'s *two ways in, one name* — and a
                // drop that skipped this would name a Set this store does not
                // hold and be refused where the key succeeds.
                //
                // **Here rather than in the console**, for the reason every
                // other store question is here: `karakuri-console` reaches no
                // disk at all (ADR-0156), so the drop names the row it was
                // dragged from and this side turns that into the two rows of
                // the vocabulary one gesture performs. `taking_in` and
                // `preset_press` are the key's own two helpers, so this is a
                // second caller and not a second answer.
                //
                // **A folder row is the same press**, and it stopped being
                // refused on 2026-09-08: it is the same two rows of the
                // vocabulary off a directory somebody dropped rather than one
                // the program was told (ADR-0275, ADR-0267), so the two arms
                // are one and [`Taking`] is the difference between them.
                let mut took = Repaint::Never;
                let acted = match (&acted, self.readout.view.scope()) {
                    (
                        Acted::Emitted(Some(Operation::LoadSet { deck, set })),
                        Some(scope @ (Scope::Presets | Scope::Folder)),
                    ) => {
                        let (deck, row) = (*deck, set.clone());
                        let from = match scope {
                            Scope::Folder => Taking::Folder(self.folder.as_deref()),
                            _ => Taking::Presets(self.presets.as_ref()),
                        };
                        match taking_in(&self.store, from, &row) {
                            Ok(taken) => {
                                println!("  take in: {}", taken.said);
                                let [take, load] = taken_in_press(deck, taken);
                                took = App::performed(
                                    gfx,
                                    self.started,
                                    &mut self.readout,
                                    self.recording.recorder(),
                                    &Acted::Emitted(Some(take)),
                                    Repaint::Never,
                                );
                                Acted::Emitted(Some(load))
                            }
                            // Which of the two acts failed is the whole of what
                            // this adds — the load's own sentence, one surface over.
                            Err(e) => {
                                println!(
                                    "  take in: `{row}` was not taken into the store: {e}\n  \
                                     take in: so nothing was loaded, and what is on deck {} is \
                                     still running",
                                    deck_letter(deck)
                                );
                                Acted::Nothing
                            }
                        }
                    }
                    _ => acted,
                };
                // **A press on a control earns its frame from what it did**,
                // and not from the claim: `Change::Pointer(Claim::Panel)` is
                // already a frame, but the operation the dot asked for is the
                // thing that moved every region in the Program bay, and it is
                // the outcome that says so.
                // **One frame asked for, however many operations the press
                // emitted** — the load's rule at the pointer, and `Repaint::soonest`
                // is what combines them.
                let repaint = App::performed(
                    gfx,
                    self.started,
                    &mut self.readout,
                    self.recording.recorder(),
                    &acted,
                    Change::Pointer(claim).repaint(),
                )
                .soonest(took);
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            // **The secondary button, and only its press.** A release is not
            // routed at all, which is the whole of what this gesture is: a
            // secondary press puts a row's menu down and takes nothing in
            // hand, so there is nothing for a release to let go of and a
            // `Pointer::Secondary` up would be an event with no arm to run
            // (ADR-0311).
            //
            // **`egui` is not told either way**, which is what this arm
            // changes least: before it, every button but the left one fell
            // through this handler's `_ => {}` and reached nothing, and
            // `egui` owns no widget anywhere on this console, so a secondary
            // press routed to it would reach nothing there either. The claim
            // is asked for the same reason it is asked on a left press —
            // rule 1's drag and rule 2's cards are about the gesture and not
            // about the button — and the answer is used the same way.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, Pointer::Secondary);
                // **The same send branch the left press takes**, because the
                // item is picked by whichever press lands on the card: a menu
                // opened with the secondary button and picked with it again is
                // one gesture, and the second press is the one that names the
                // item.
                if let Acted::Emitted(Some(Operation::TransferSet {
                    transfer: SetTransfer::Send { ref id },
                })) = acted
                {
                    println!("  send: naming a file to write `{id}` to");
                    sending(
                        &gfx.window,
                        &self.store,
                        self.folder.as_deref(),
                        id,
                        self.keeping.send_tx.clone(),
                    );
                }
                let repaint = App::performed(
                    gfx,
                    self.started,
                    &mut self.readout,
                    self.recording.recorder(),
                    &acted,
                    Change::Pointer(claim).repaint(),
                );
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // **The two shapes a wheel arrives in, and only the vertical
                // half of either.** `LineDelta` is a count of detents and is
                // what a mouse sends, so it is multiplied by the console's own
                // `WHEEL_STEP` — three parameter rows, which is what
                // `docs/manual/console.html` says a notch is worth.
                // `PixelDelta` is a trackpad and is already a distance: it is
                // in physical pixels like every other position this handler
                // reads, so it is divided by the scale and passed through.
                //
                // **Negated, because the axes point opposite ways.** `winit`'s
                // positive `y` is a wheel pushed away from the hand, which
                // moves a list *up* — and a scroll position is how far down the
                // content the pane has come.
                let by = match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        -y * karakuri_console::room::size::WHEEL_STEP
                    }
                    MouseScrollDelta::PixelDelta(at) => -(at.y / self.scale) as f32,
                };
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, Pointer::Wheel(by));
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                // **The frame is owed for what the wheel moved and not for the
                // claim**, which is the `CursorMoved` arm's own rule one event
                // along: a wheel spun against the top of a pane's list is the
                // panel's and changes nothing, and a frame per notch of that
                // would be a repaint for a gesture with no picture in it.
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Wheeled(claim, acted == Acted::Pointed).repaint(),
                );
            }

            // **The one modifier this loop keeps, and it keeps it for one
            // key.** `shift-Tab` is the tab ring walked backwards (ADR-0259)
            // and `winit`'s `KeyEvent` carries no modifier state, so the
            // answer has to have been listened for — see [`App::shift`].
            //
            // **`egui` is still told**, which is what this arm has to add back:
            // every event this `match` does not name reaches `to_egui` through
            // the wildcard at the bottom, and a modifier taken here and not
            // passed on would leave the toolkit's own copy stale.
            WindowEvent::ModifiersChanged(state) => {
                App::to_egui(gfx, &mut self.costs, &event);
                self.update_modifiers(&state);
            }

            WindowEvent::KeyboardInput { .. } => {
                // **`egui` sees every key, and is never asked for
                // permission.** `App::to_egui` reads `EventResponse::repaint`
                // and nothing else — `consumed` is read nowhere in `crates/` —
                // so the `match` below runs whatever `egui` answers, and
                // `egui`'s modifier state stays current for the frame it does
                // ask for.
                //
                // **Not because nothing has focus.** `egui-winit` 0.36.1
                // hard-codes the flag — *"When pressing the Tab key, egui
                // focuses the first focusable element, hence Tab always
                // consumes"* — so `consumed` is `true` for every `Tab`
                // whatever the focus state is, and honouring it would swallow
                // the first press rather than being harmless. **`Tab` is the
                // key that moves focus between bays here since 2026-09-09**
                // (ADR-0259, ADR-0332), which is where that is the failure
                // that looks like nothing at all; the invariant that record
                // names is `App::to_egui`'s own doc comment, and the shape of
                // that function — nothing past its one destructure holds an
                // `EventResponse` to read `consumed` off — is what enforces
                // it now.
                App::to_egui(gfx, &mut self.costs, &event);
                let WindowEvent::KeyboardInput { event: key, .. } = &event else {
                    unreachable!("the arm this is in")
                };
                if key.state != ElementState::Pressed {
                    return;
                }
                // **The one flow on this panel that asks for letters takes the
                // keyboard whole while it is asking.** Every key here is a
                // character, a rub-out, the commit or the abandonment, and none
                // of them is the operation that key names the rest of the time
                // — `s` is an `s` in a name and not a solo, and the pointer is
                // not what a name is addressed to. `escape` leaves the program
                // the rest of the time and leaves the name here, which is the
                // same word for the same act one level in.
                //
                // **Nothing typed is checked here**, which is
                // [`checked_name`]'s half of the same split: the pill takes
                // the letters, and the wall is where the file is written.
                if self.readout.view.arrangement.naming().is_some() {
                    let (acted, moved) = match key.logical_key.as_ref() {
                        Key::Named(NamedKey::Escape) => {
                            println!("arrangement: nothing was saved");
                            self.readout.view.arrangement.shut();
                            (Acted::Nothing, true)
                        }
                        Key::Named(NamedKey::Enter) => (self.readout.named(), true),
                        Key::Named(NamedKey::Backspace) => {
                            (Acted::Nothing, self.readout.view.arrangement.rubbed_out())
                        }
                        // **A `Key::Character` is text and not a key**, so it
                        // may be more than one character — a dead key
                        // resolving, an IME committing a run — and every one
                        // of them goes in. `Arrangement::typed` is what
                        // refuses a control character, because a newline
                        // arriving as text is the commit rather than a letter.
                        Key::Character(text) => {
                            let mut moved = false;
                            for c in text.chars() {
                                moved |= self.readout.view.arrangement.typed(c);
                            }
                            (Acted::Nothing, moved)
                        }
                        Key::Named(NamedKey::Space) => {
                            (Acted::Nothing, self.readout.view.arrangement.typed(' '))
                        }
                        _ => (Acted::Nothing, false),
                    };
                    let repaint = App::performed(
                        gfx,
                        self.started,
                        &mut self.readout,
                        self.recording.recorder(),
                        &acted,
                        Change::Naming(moved).repaint(),
                    );
                    App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                    return;
                }
                // **The second letter-taking flow takes the keyboard on the
                // same terms as the first** (ADR-0292). The two can never both
                // be open — `input::claim`'s rule 2 claims every press while
                // either is — so this is a second store for one gesture rather
                // than an order between two.
                if self.readout.view.naming_set().is_some() {
                    let (acted, moved) = match key.logical_key.as_ref() {
                        Key::Named(NamedKey::Escape) => {
                            println!("inspector: nothing was kept");
                            self.readout.view.stop_naming_set();
                            (Acted::Nothing, true)
                        }
                        Key::Named(NamedKey::Enter) => {
                            (Acted::Emitted(self.readout.view.named_set()), true)
                        }
                        Key::Named(NamedKey::Backspace) => {
                            (Acted::Nothing, self.readout.view.rub_out_of_name())
                        }
                        Key::Character(text) => {
                            let mut moved = false;
                            for c in text.chars() {
                                moved |= self.readout.view.type_into_name(c);
                            }
                            (Acted::Nothing, moved)
                        }
                        Key::Named(NamedKey::Space) => {
                            (Acted::Nothing, self.readout.view.type_into_name(' '))
                        }
                        _ => (Acted::Nothing, false),
                    };
                    let repaint = App::performed(
                        gfx,
                        self.started,
                        &mut self.readout,
                        self.recording.recorder(),
                        &acted,
                        Change::Naming(moved).repaint(),
                    );
                    App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                    // **And the save is the window's, exactly as the capsule's
                    // is**: a disk write is not a thing to do on a frame.
                    if let Acted::Emitted(Some(Operation::SaveSet { deck, ref id })) = acted {
                        self.keeping.save_set(
                            &gfx.engine,
                            &self.store,
                            Asked::Operator,
                            usize::from(deck),
                            id.clone(),
                            None,
                        );
                    }
                    return;
                }
                let op = match key.logical_key.as_ref() {
                    // **The four keys of the grammar, dispatched to the bay
                    // that has focus** — a digit names the nth thing one level
                    // below the address and `0` the bay's head, the arrows take
                    // the neighbour or the next value, `space` is the addressed
                    // thing's next state and `enter` is the act it is for
                    // (ADR-0259).
                    //
                    // **One arm rather than four**, and that is what makes the
                    // re-read below one statement: a digit and an arrow both
                    // move the Library's cursor, and the rule that a reading
                    // follows it (ADR-0265) is paid once here instead of at
                    // each key that could move it.
                    //
                    // **The console resolves the address and this names the
                    // operation** ([ADR-0333](../../../docs/adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md)):
                    // `karakuri_console::focus::press` knows which control a
                    // press landed on and owns the cycle a state goes round
                    // (P-0090); what it cannot know is the value the deck is
                    // holding, the tenth a level steps by, or what a load costs
                    // — so those stay here, where they were.
                    //
                    // **Every write still goes through the method that already
                    // refused it.** A digit that names a strip is
                    // `View::select` and one that names a row is
                    // `View::point_at`, so a deck the mixer draws no strip for
                    // and a row past the listing are turned down exactly where
                    // they were before. The grammar adds a route and no
                    // exception.
                    //
                    // **Twelve letters went with it**, and none of them was
                    // unbound before the grammar reached the same row:
                    // `0`–`3` are the Mixer's `1`–`4`, `[ ] \\` and `; '` are
                    // the arrows and `space` on the addressed trim and fader,
                    // `m` is `space` on the blend chip, `e` is `space` on the
                    // Library's head and `l` is `enter` on one of its rows.
                    named if grammar(&named).is_some() => {
                        let press = grammar(&named).expect("the arm this is in");
                        // **What the cursor was on before the press**, so the
                        // re-read below is a *move* and not a press — the
                        // console's own answer would say which of six things
                        // happened and this asks the one question the rule is
                        // about.
                        let was = self.readout.view.cursor_row();
                        // **The deck read here and handed in**, which is the
                        // three mix keys' own rule arriving at the grammar:
                        // `view::Strip` is that same reading copied once a
                        // frame, and a scheduled fade landing between the frame
                        // and the press would cycle from a state the deck has
                        // already left behind. The closure is asked only for
                        // the strip the address is on, and only where a press
                        // needs a state to cycle from.
                        let asked = focus::press(
                            &mut self.readout.view,
                            &self.readout.panel,
                            press,
                            |deck| holding(&gfx.engine.deck, deck),
                        );
                        let moved = self.readout.view.cursor_row() != was;
                        // ADR-0265: the reading follows the cursor, on
                        // whichever surface moved it — see [`reread_if_open`],
                        // the pointer release's own call one arm up.
                        if let Some(line) =
                            reread_if_open(moved, &mut self.readout.view, &self.store)
                        {
                            println!("{line}");
                        }
                        match asked {
                            // **The Library head's scope**, and it is this
                            // file's because a scope *is* a listing on this
                            // side and a directory read is not a thing to do on
                            // a frame (P-0091). What was `e` until 2026-09-10.
                            focus::Asked::Scope => {
                                if self.readout.view.step_scope() {
                                    // Whose history, for the press branch's reason:
                                    // the scope key steps onto the `history` chip
                                    // as readily as the pointer names it.
                                    let running = aimed_set(gfx, &self.readout.view);
                                    println!(
                                        "{}",
                                        listing(
                                            &mut self.readout.view,
                                            &self.store,
                                            self.presets.as_ref(),
                                            self.folder.as_deref(),
                                            running.as_deref(),
                                        )
                                    );
                                }
                                // **Emitted whether or not the mark moved**, which is
                                // the deck keys' rule: what a press asked for is what
                                // is emitted, and `unwritten` is what says the press
                                // wrote no record and that it is settled.
                                //
                                // **And nothing performs it in `performed`**, where
                                // `SelectDeck` has `pointed` — because this payload
                                // cannot say which scope was chosen and a performer
                                // reading `Undecided` would have to guess. The step
                                // above *is* the performance, and it is the surface's
                                // own pointer either way.
                                let acted = Acted::Emitted(Some(Operation::SelectScope {
                                    scope: Undecided,
                                }));
                                let repaint = App::performed(
                                    gfx,
                                    self.started,
                                    &mut self.readout,
                                    self.recording.recorder(),
                                    &acted,
                                    Repaint::Never,
                                );
                                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                                return;
                            }
                            // **The load, and the two operands are already on screen.**
                            // The cursor says which Set and the selection says which
                            // deck, which is `console.html`'s *"a cursor and a key
                            // with no pointer anywhere in it"*. What the press does is
                            // re-point the slot's source — [`loading`] — so the worker
                            // builds it and the watchdog judges it exactly as it does
                            // an edit, and nothing here reaches `Deck::install`.
                            // **The scope, and the key steps where the operation
                            // names.** `Operation::SelectScope`'s payload is
                            // `Undecided` — *"what identifies one member of a growable
                            // list is spelled nowhere"* — and its own doc says where
                            // the stepping goes: *"The key steps and this does not …
                            // that is the translator's arithmetic rather than this
                            // operation's payload"* (P-0090). So the surface moves its
                            // own pointer, exactly as the four deck keys do, and the
                            // operation is emitted through the same route so that the
                            // press is recorded as `Silent(Surface)` rather than as
                            // nothing at all.
                            //
                            // **The listing is re-read here**, on the press that
                            // changed the scope: a scope *is* a listing on this side
                            // (`listing`), and a directory read is not a thing to do
                            // on a frame (P-0091).
                            focus::Asked::Load => {
                                let deck = self.readout.view.selection();
                                let at = self.readout.view.cursor_row();
                                // **The Sets, which is empty under `history`**: a row
                                // of that scope is a version and this key loads a Set,
                                // so the arm below names it rather than this line
                                // handing a word no store holds to a load
                                // (`view::View::sets`).
                                let row = self.readout.view.sets().get(at).cloned();
                                // Whose history, for `why_nothing`'s `history` arm —
                                // which this key cannot reach, because the arm below
                                // answers that scope first, and which is passed anyway
                                // because a sentence chosen by a caller is a sentence
                                // that can be chosen wrongly.
                                let running = aimed_set(gfx, &self.readout.view).is_some();
                                // **What the take-in half of this press asked for**,
                                // where a press that took nothing in leaves it
                                // `Repaint::Never` — see the preset arm below for why
                                // one press emits two operations and why they cannot
                                // be one `Acted`.
                                let mut took = Repaint::Never;
                                let acted = match (self.readout.view.scope(), row) {
                                    // **A preset or a folder row is taken in and then
                                    // loaded**, which is one press because taking it in
                                    // is what gives the Set the id the load needs —
                                    // ADR-0229's *one operation, two moments*,
                                    // performed at the second of them. What lands in
                                    // the store is a Set of the operator's, so `all`
                                    // gains a row they did not make: `console.html`
                                    // says that out loud so that nobody meets it as a
                                    // surprise. It gains no row under `my sets`, which
                                    // is ADR-0299 — a Set the operator did not choose
                                    // is in the library and is not one of their
                                    // favourites.
                                    //
                                    // **The two scopes are one arm**, and the folder
                                    // half is what landed on 2026-09-08: a folder row
                                    // was refused here because the scope had no
                                    // directory to list, and ADR-0275 gave it one. See
                                    // [`Taking`], which is the whole of the difference
                                    // between them.
                                    (Some(scope @ (Scope::Presets | Scope::Folder)), Some(row)) => {
                                        let from = match scope {
                                            Scope::Folder => Taking::Folder(self.folder.as_deref()),
                                            _ => Taking::Presets(self.presets.as_ref()),
                                        };
                                        match taking_in(&self.store, from, &row) {
                                            Ok(taken) => {
                                                println!("  take in: {}", taken.said);
                                                // **The take-in is named as well as
                                                // performed, and that is `e`'s rule
                                                // one key along**: the scope step
                                                // emits `SelectScope` *"so that the
                                                // press is recorded as `Silent` rather
                                                // than as nothing at all"*, and this
                                                // press has just performed a whole row
                                                // of the vocabulary —
                                                // `docs/manual/operations.html`'s
                                                // *Send a Set to somebody, and take
                                                // one in*, *"opening a preset is this
                                                // row"*. Emitting only the load would
                                                // be a press that does two of the
                                                // page's rows and names one.
                                                //
                                                // **Two emissions rather than one**,
                                                // because they are two rows: taking in
                                                // is what gives the Set the id, and
                                                // the load names that id. `Acted`
                                                // carries one operation — a fader
                                                // drag, a chip, a key each emit
                                                // exactly one — so the pair is two
                                                // trips through `App::performed`
                                                // rather than a shape invented here
                                                // for the one press that has two.
                                                //
                                                // **In the order they happened.**
                                                // `written` answers
                                                // `Silent(NoRecord)` for the transfer,
                                                // so nothing in `performed` performs
                                                // it and the emission is the naming;
                                                // the load after it is what re-points
                                                // the slot.
                                                let [take, load] = taken_in_press(deck, taken);
                                                took = App::performed(
                                                    gfx,
                                                    self.started,
                                                    &mut self.readout,
                                                    self.recording.recorder(),
                                                    &Acted::Emitted(Some(take)),
                                                    Repaint::Never,
                                                );
                                                Acted::Emitted(Some(load))
                                            }
                                            // **Which of the two acts failed is the
                                            // whole of what this sentence adds.**
                                            // Nothing was taken in, so nothing was
                                            // loaded — where a load that fails says so
                                            // in `played`'s own words, with the deck
                                            // it did not reach. The refusal itself is
                                            // `setfile::unbundle`'s, including the one
                                            // for an id this store already holds.
                                            Err(e) => {
                                                println!(
                                                "  take in: `{row}` was not taken into the store: \
                                                 {e}\n  take in: so nothing was loaded, and what is \
                                                 on deck {} is still running — a Set already here is \
                                                 listed under `all`, which is where it is loaded \
                                                 from",
                                                deck_letter(deck)
                                            );
                                                Acted::Nothing
                                            }
                                        }
                                    }
                                    // **A row of `history` is a version and not a
                                    // Set**, so this key has nothing to load and says
                                    // so rather than falling through to the sentence
                                    // below, which would report a listing as empty
                                    // while it is drawing rows. What lands a version is
                                    // a press on the row itself —
                                    // `Operation::RestoreProcedure`, on the deck the
                                    // load pulldown names rather than on the selection.
                                    (Some(Scope::History), _) => {
                                        println!(
                                        "  load: `history` lists the versions of a Set rather than \
                                         Sets, so there is nothing here for enter to load — press a \
                                         row to put that version back on its node, or mark `all` \
                                         and load a Set"
                                    );
                                        Acted::Nothing
                                    }
                                    // A row of `all` or of `my sets`, which is a Set
                                    // this store already holds and is the route
                                    // ADR-0228 built.
                                    (_, Some(set)) => {
                                        Acted::Emitted(Some(Operation::LoadSet { deck, set }))
                                    }
                                    // Not a refusal of the load: there is no Set under
                                    // the cursor because this scope lists nothing. The
                                    // bay says so by drawing no rows; this says so in
                                    // words, and it says **which** nothing it is —
                                    // `favourites`, a folder nobody has pointed
                                    // anywhere and a folder holding nothing are three
                                    // different reasons, and a key that did nothing and
                                    // a key that is not bound are the same experience.
                                    (scope, None) => {
                                        println!(
                                        "  load: `{}` lists nothing, so there is no Set under the \
                                         cursor — {}",
                                        match scope {
                                            Some(scope) => scope.name(),
                                            None => "the library",
                                        },
                                        match scope {
                                            Some(scope) =>
                                                why_nothing(scope, self.folder.is_some(), running),
                                            None => "this console was handed no scopes at all",
                                        }
                                    );
                                        Acted::Nothing
                                    }
                                };
                                // **One frame asked for, however many operations the
                                // press emitted.** `App::wants` counts a frame against
                                // the run's costs and asks the window for a redraw, so
                                // calling it twice for one press would ask for two
                                // frames where one is drawn. `Repaint::soonest` is
                                // what combines them, and its own rule is why it is
                                // safe: it can only bring a frame forward.
                                let repaint = App::performed(
                                    gfx,
                                    self.started,
                                    &mut self.readout,
                                    self.recording.recorder(),
                                    &acted,
                                    Repaint::Never,
                                )
                                .soonest(took);
                                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                                return;
                            }
                            asked => {
                                let repaint = answered(
                                    gfx,
                                    self.started,
                                    &mut self.readout,
                                    self.recording.recorder(),
                                    &asked,
                                );
                                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                                return;
                            }
                        }
                    }
                    // **Everything else this loop binds is one lookup into
                    // `KEY_BINDINGS`** — the ten literal keys it used to spell
                    // as ten arms, now a table checked against
                    // `docs/manual/operations.html` by `key_column` directly
                    // (see [`KEY_BINDINGS`] for why a table and not arms, and
                    // for the doc comment each arm here used to carry).
                    other => match KEY_BINDINGS
                        .iter()
                        .find(|binding| binding.key.matches(&other))
                    {
                        Some(binding) => {
                            // **Disjoint fields, not `self`.** `gfx` is
                            // already a live `&mut` borrow out of `self.gfx`
                            // (see the top of `window_event`), so a bound
                            // action takes exactly the other fields it
                            // needs rather than all of `self` — the same
                            // reason `App::performed` and `App::wants` never
                            // took `&mut self` either.
                            let mut ctx = KeyCtx {
                                readout: &mut self.readout,
                                egui_due: &mut self.egui_due,
                                costs: &mut self.costs,
                                recording: &mut self.recording,
                                keeping: &mut self.keeping,
                                store: &self.store,
                                started: self.started,
                                shift: self.shift,
                            };
                            match binding.action {
                                KeyAction::Handled(act) => {
                                    act(&mut ctx, gfx);
                                    return;
                                }
                                KeyAction::Focus(act) => {
                                    let moved = act(&mut ctx);
                                    App::wants(
                                        gfx,
                                        &mut self.egui_due,
                                        &mut self.costs,
                                        Change::Pointed(moved).repaint(),
                                    );
                                    return;
                                }
                                KeyAction::Panel(act) => match act(&mut ctx) {
                                    Some(op) => op,
                                    None => return,
                                },
                            }
                        }
                        // Not one of the grammar's four and not in the table
                        // either — an unbound key, answered with nothing.
                        None => return,
                    },
                };
                let outcome = self.readout.op(op);
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Operated(&outcome).repaint(),
                );
            }

            WindowEvent::RedrawRequested => {
                // **The frame's own clock, and the first statement of the
                // frame because that is the whole of what makes it one.** Two
                // consecutive readings of this bracket a whole redraw — the
                // block on the swapchain, every timed stretch, every untimed
                // one and `Queue::present` — so [`Cost::period`] is the frame
                // and not a part of it. Nothing else in this handler can say
                // that: every other clock here starts after the wait.
                let period = self.costs.tick(Instant::now());
                // **What a model asked for, and what a save came back with —
                // both above everything that touches the window.** A client
                // asking to keep what is playing should not be waiting on a
                // swapchain, and nothing either of these reaches needs one; a
                // window that has faulted returns below this line and still owes
                // a waiting client its answer. That is `karakuri-cli`'s
                // `Live::run_requests` and `Live::finished_saves`, at the top of
                // the frame for the reason written there.
                self.keeping.requests(&mut gfx.engine, &self.store);
                // **And every operation a model named**, above everything that
                // touches the window for the reason the line above it is:
                // nothing this reaches needs a swapchain, and a window that has
                // faulted returns below this line still owing a waiting client
                // its answer. See [`App::operated`].
                App::operated(
                    gfx,
                    event_loop,
                    self.started,
                    &mut self.readout,
                    &mut self.recording,
                    &mut self.keeping,
                    &self.store,
                    &mut self.egui_due,
                    &mut self.costs,
                );
                // **And every operation a hand on a control surface named**,
                // beside the drain above and for its reason: this is a door
                // outside the window handing in an operation, and nothing it
                // reaches needs a swapchain. See [`App::mapped`], which is
                // also where the wake that got this frame asked for is.
                App::mapped(
                    gfx,
                    self.started,
                    &mut self.readout,
                    &mut self.recording,
                    &self.hover,
                    &learned_map(&self.store),
                    &mut self.egui_due,
                    &mut self.costs,
                );
                // **And a kept procedure with them**, drained beside the saves and
                // for their reason: the two acts both end on a disk, and the
                // Library bay lists what both of them wrote. `|` and not `||`,
                // so the second drain runs whether or not the first landed
                // anything — a short-circuit here would leave a keep's outcome
                // in its channel until a save happened to arrive.
                if self.keeping.finished_saves() | self.keeping.finished_keeps() {
                    let running = aimed_set(gfx, &self.readout.view);
                    // **The one thing in this program that adds a Set**, so the
                    // bay that lists them is re-read on the frame it landed —
                    // and only on that frame. A directory read is not a thing to
                    // do per frame (P-0091), and a bay still listing what it
                    // listed before a save is a readout that is wrong and
                    // silent.
                    println!(
                        "{}",
                        listing(
                            &mut self.readout.view,
                            &self.store,
                            self.presets.as_ref(),
                            self.folder.as_deref(),
                            running.as_deref(),
                        )
                    );
                }
                // **And what a recording's start or stop came back with**, on
                // the same terms and above the same line: a thread that opened
                // a session or flushed one answers on a channel, and the
                // answer is said at the frame it arrives. It moves the `rec`
                // pill, which is read below beside the rest of this row.
                self.recording.finished();
                let waited = Instant::now();
                let acquired = gfx.surface.get_current_texture();
                let waited = waited.elapsed();
                if let Some(missed) = missed(&acquired) {
                    match missed {
                        Missed::Remake => {
                            gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                            self.costs.owes();
                            gfx.window.request_redraw();
                        }
                        Missed::Again => {
                            self.costs.owes();
                            gfx.window.request_redraw();
                        }
                        Missed::Idle => {}
                        Missed::Fault => {
                            if !self.faulted {
                                self.faulted = true;
                                println!(
                                    "the surface raised a validation error acquiring a frame — \
                                     the window has stopped drawing"
                                );
                            }
                        }
                    }
                    return;
                }
                self.faulted = false;
                let frame = match acquired {
                    wgpu::CurrentSurfaceTexture::Success(frame)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                    // `missed` returned `None`, so there is a texture here.
                    _ => return,
                };

                let mut cost = Cost {
                    wait: waited,
                    period,
                    ..Cost::default()
                };

                // -- where each sink goes, and how big it is -----------
                // **Before the `egui` pass**, because the pass draws these
                // textures and one registered after it would be a frame
                // behind. **And before `compose`**, because `Sink::acquire` is
                // handed a `&Gpu` and nothing else, while deciding this needs
                // the rectangle, the scale factor and the renderer — see
                // `Presented::aim`.
                //
                // One call, and it is the same one `resumed` sized both
                // textures with. *Which rectangle, at what size* is `aims` and
                // `Engine::aim` and nowhere else, which is what lets a test
                // ask the question this handler used to answer where nothing
                // could reach it.
                self.readout.panel.solve();

                // -- the Program bay arranges itself -------------------
                // **Before `aims`, because the four cells are in one of two
                // places and this is what decides which.** The canvas is
                // written in the same breath, off the session canvas, for the
                // reason `aims` reads it there too: the shape the bay arranges
                // itself for and the shape the texture is sized to are one
                // number or they are a picture drawn for the other
                // arrangement.
                //
                // **It read `Present::size()` until 2026-09-09**, which was
                // the same number by another route while the frame was
                // composited at [`CANVAS`]. It is not any more — the frame
                // follows the largest enabled output — and reading it there
                // would make the bay arrange itself for the rectangle it had
                // last frame, which is a loop with no shape of its own. See
                // [`CANVAS`].
                //
                // `Change::Rearranged` is raised on **every** frame and says
                // whether anything moved, which is the arm's own argument:
                // this is the one change that is re-derived rather than
                // reported, and one that answered *draw* regardless would ask
                // for a frame on every frame.
                self.readout.view.canvas = CANVAS;
                let moved = view::rearrange(&mut self.readout.panel, self.readout.view.canvas);
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Rearranged { moved }.repaint(),
                );

                let scale = self.scale as f32;
                // **The projector's size read before the borrow**, which is
                // the whole of why it is a `Copy` field on [`Projector`]
                // rather than a call on the window: `aim` takes `&mut` of the
                // engine and `Gfx` holds both.
                let projector = gfx.projector.as_ref().map(|p| p.size);
                let (picture, previews) = gfx.engine.aim(
                    &gfx.gpu,
                    &mut gfx.renderer,
                    self.readout.panel.layout(),
                    scale,
                    projector,
                );
                // **What the console draws on the projector's chip**, written
                // per frame beside the frame it is about, exactly as the
                // picture's own registration is — see `view::View::projector`.
                self.readout.view.projector = projector.is_some();
                self.readout.view.picture = picture;
                self.readout.view.previews = previews;
                // **Which of those pictures is a still, and why** — the deck's
                // own answer, read beside the pictures because the caption's
                // word is a function of the pair. A slot the watchdog stopped
                // holds the last frame it drew (ADR-0316), and a held frame of
                // good material is indistinguishable from material: the
                // caption is what says it (ADR-0269).
                //
                // Per frame like the pictures, and for the same reason it is
                // not per verdict: this is a state a slot is in rather than an
                // event it had, and it ends on a build the lane will also
                // report.
                self.readout.view.overloaded = stopped_slots(&gfx.engine.deck);

                // **What the transport row reads, written beside the frame it
                // is about**, exactly as the two lines above are: the picture
                // is a texture id that belongs to this frame and this is a
                // tempo and a cost that belong to the last one. See
                // `transport`.
                //
                // **`live` is asked once and used twice.** It decides whether
                // this frame is followed by another — the last statement in
                // this handler — and the row's frame rate is a claim about
                // that. Two calls would be two answers to *is anything making
                // texels*, taken either side of the whole frame.
                let live = live(&self.readout.view);
                // **This frame's step count, measured once and read three
                // ways.** It is P-0092's live half — *"live, the engine derives
                // the step count from real time and writes it in"* — and it is
                // how much session this frame is worth to the room below, what
                // the deck is committed with, and what the `tick` that closes
                // the frame carries.
                //
                // **Read here rather than at the top of the handler**, which is
                // ADR-0078: a frame that is discarded must not already have
                // been recorded. Everything above this line that can abandon a
                // frame has already returned — a surface to remake, one to ask
                // again for, an idle window, a validation fault — and
                // everything below it composes. `Clock::last` moves only in
                // this call, so a frame the handler returned from early leaves
                // its interval for the next one, and a stretch where this
                // window drew nothing at all is counted whole by the frame that
                // ends it, up to the cap.
                let steps = self.clock.steps(Instant::now());
                // **The room, read before the row that reads the grid it
                // moves.** A measurement taken after `transport` would be a
                // tempo drawn one frame behind the correction that made it,
                // which is the one thing this row cannot be: it is what an
                // operator watches to tell a lock from a coincidence. See
                // `measure_audio`.
                measure_audio(
                    &mut gfx.audio,
                    &mut gfx.engine.deck,
                    self.clock.interval(),
                    steps,
                    self.recording.recorder(),
                );
                // **The verdict it carries is the one `staging` left behind
                // below**, which is one frame back: the drain runs after this
                // line and the events it drains were emitted by the previous
                // frame's `compose` anyway, so the capsule reaches the screen
                // on the frame after the lane's row does. The row is drawn at
                // `BEAT_STALENESS` for as long as there is one, so that frame
                // is at most 24.67 ms away and there is always another —
                // P-0094, and `View::transport_declares`.
                // **The sequencer, polled**, and it is the one thing in this
                // handler that emits an operation nobody pressed.
                //
                // **On the render thread, once a frame, against `beats`** —
                // which is what a transition already is one row finer
                // (`Transition::value_at(beats)`), and the engine has no beat
                // callback for it to be anything else (ADR-0322). Nothing here
                // reads a clock: `beats` is the oscillator's accumulator, a
                // pure function of the `tick` records and the tempo
                // corrections, so P-0092 is untouched.
                //
                // **Live only**, which is the whole of what a replay needs from
                // this: what a lane did is already in the stream as the writes
                // it made, verbatim, so re-deriving the steps at replay would
                // need the pattern in the stream (ADR-0227 refuses it) and
                // would make a replay depend on a file that may have been
                // edited since. This window has no replay path at all, and this
                // block is where one would have to be excluded.
                //
                // **Before the reading below**, so the column the bay draws is
                // the step that was just emitted rather than the one before it.
                let beats = gfx.engine.deck.signals().oscillator().beats();
                let step = self
                    .readout
                    .playhead
                    .advance(self.readout.sequencer.pattern(), beats);
                if let Some(step) = step {
                    // **One emission per unmuted lane, through the same
                    // `performed` a press goes through** — so what reaches the
                    // stream is `Record::Opacity` and `Record::Ride`, records
                    // that already exist and already replay (ADR-0222's *"a
                    // hand and a lane meet at `Live::operate` where every other
                    // conflict is already resolved"*).
                    //
                    // **Indexed rather than iterated**, because the borrow of
                    // the pattern has to end before `performed` takes the whole
                    // readout: the operation is built and the borrow dropped,
                    // and nothing is collected, so a boundary allocates
                    // nothing on the frame path.
                    for lane in 0..self.readout.sequencer.pattern().lanes().len() {
                        let pattern = self.readout.sequencer.pattern();
                        let Some(at) = pattern.lanes().get(lane) else {
                            continue;
                        };
                        if at.muted() {
                            continue;
                        }
                        let operation = at.operation_at(step, pattern.mode());
                        let acted = Acted::Emitted(Some(operation));
                        App::performed(
                            gfx,
                            self.started,
                            &mut self.readout,
                            self.recording.recorder(),
                            &acted,
                            // **A frame is already being drawn**, so a lane
                            // asks for none: this is inside the handler that
                            // composes, and a `Repaint::Now` here would be the
                            // frame this one already is.
                            Repaint::Never,
                        );
                    }
                }
                // **What the bay draws, beside the frame it is about.** The
                // pattern is cloned per frame the way every other reading here
                // is rebuilt per frame — the console holds no session and this
                // is the seam (ADR-0156) — and the step is the *poll's* answer
                // rather than a second derivation from `beats`.
                self.readout.view.sequencer = Some(Sequenced {
                    pattern: self.readout.sequencer.pattern().clone(),
                    bank: self.readout.sequencer.armed(),
                    step: self.readout.playhead.at(),
                });
                // **Which Set the Library bay's walk would be of**, read off
                // the load pulldown's deck's aim and rebuilt per frame like
                // every other reading in this block. It is the one value a
                // `history` chip press needs and the console cannot spell: the
                // bay holds a deck letter and the id rides the aim (ADR-0308,
                // ADR-0304). Written here rather than on the press that changes
                // it, because a load, a key, a mapped control and a model all
                // re-point a slot — see `Readout::chose` and
                // `view::Chosen::asked`.
                self.readout.view.aimed = aimed_set(gfx, &self.readout.view);
                self.readout.view.transport = transport(
                    &gfx.engine.deck,
                    &self.costs,
                    gfx.budget_ms,
                    live,
                    self.readout.health,
                    // **Read here rather than remembered**, which is the rule
                    // every other value in this row follows: the recorder is
                    // opened and closed by threads that answer on a channel,
                    // so the only reading that cannot be stale is the one
                    // taken beside the frame that draws it.
                    self.recording.rec(),
                );
                // **And what the two look controls at the end of that row
                // read**, beside the frame they are about. It is the look this
                // frame is committed under, so the capsule names the operator
                // the picture went through rather than one a press asked for
                // and nothing has applied yet.
                // **And what the tracker's other three read**, beside the
                // frame they are about and after `measure_audio` for the
                // transport row's own reason: a correction that landed this
                // frame moves the tempo, and the octave halves are that tempo
                // against the range. Drawing them from the tempo before the
                // correction would inert a half a press could still reach.
                self.readout.view.tracker = Some(tracking(
                    gfx.audio.as_ref(),
                    gfx.engine.deck.signals().oscillator().bpm(),
                ));
                self.readout.view.look = Some(look(&gfx.engine.look));
                // **And what the Master bay's out row reads**, which is the
                // other end of the same chain: this level is applied where the
                // mix wrote the frame and the look's is applied where the
                // present pass read it, so the two are read off two different
                // objects and written here in the same breath (ADR-0224).
                self.readout.view.master_out = Some(gfx.engine.deck.out());
                self.readout.view.master_chain = Some(chain_view(&gfx.engine.chain));
                // **And which classes are open to a model**, read off the
                // handle rather than remembered from the last press on a pill.
                // Nothing but a pill writes it today; the handle exists because
                // an MCP server holds a clone of it and reads it on every call,
                // and a view that trusted its own last write would be the
                // console answering on that server's behalf.
                self.readout.view.opening = self.readout.opening.read();
                // **And what the mixer strips read**, beside the frame they
                // are about for the same reason. One strip per slot, so two —
                // see `mixer`.
                mixer(
                    &gfx.engine.deck,
                    &gfx.material,
                    &mut self.readout.view.mixer,
                );

                // **And what the Staging lane lists**, off the same deck and
                // beside the frame the verdicts belong to. It reads the
                // *previous* frame's, exactly as the transport row above does
                // and for the same reason: a build is installed and a
                // watchdog reports at a frame boundary, which is `compose`
                // below. See `staging`, which is also where the drain is
                // argued.
                // **And the Inspector's panes with them, on the frames a
                // Set actually landed on.** `Set::published` allocates and
                // says it is not for the frame path — *"A console reads this
                // when a Set lands, not per frame"* — and this is the first
                // thing in this program that knows when one did. Before the
                // slots were watched no Set ever landed after the first, so
                // this read was a startup step and nothing else; it is still
                // a startup step and now also a rebuild's.
                if staging(
                    &mut gfx.engine.deck,
                    &mut self.keeping,
                    &gfx.engine.aimed,
                    &mut self.readout.view.staging,
                    &mut self.readout.health,
                ) {
                    // **And the risk badges, because a Set that landed is a
                    // Set nothing has estimated.** ADR-0296 drops the estimate
                    // on an install, so the slot that just swapped is governed
                    // on its measurement until something estimates it again —
                    // and a dot left saying what the Set before it cost would
                    // be the meter this whole sub-milestone exists to stop.
                    // This is the one place in the program that knows a Set
                    // landed, which is why it is here rather than per frame.
                    self.readout.view.costs = costs(&gfx.engine.deck.govern());
                    let targets = self.readout.view.pane_decks();
                    inspector(
                        &gfx.engine.deck,
                        &gfx.material,
                        &gfx.engine.aimed,
                        targets,
                        &mut self.readout.view.inspector,
                    );
                }

                // **The one clock behind everything that moves on the panel,
                // written as the number it becomes.** Beside the strips it
                // animates, and beside them for the same reason the picture
                // and the transport are written here: it belongs to this
                // frame.
                self.readout.view.phase = view::Phase::since(self.started.elapsed());
                // **And what that costs, asked of the view rather than
                // decided here.** `View::animating` is the panel's own
                // declaration — a staleness while something is pending, and
                // `None` while nothing is — and `Change::Animating` turns it
                // into a deadline. It is asked every frame because nothing an
                // operator does can raise it: a slot is parked by the
                // governor, between frames, through no window event at all.
                //
                // **`Costs::owes` is deliberately not called**, exactly as it
                // is not for the picture below: a frame the roll asks for is a
                // frame drawn on a window nobody touched, and the still-panel
                // reading should print the rate rather than hide it.
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Animating(self.readout.view.animating(self.readout.panel.layout()))
                        .repaint(),
                );
                // **And what the hover layer is owed**, asked on the frame for
                // `View::animating`'s reason: a dwell is a deadline the layer
                // keeps and nothing an operator does raises it — the pointer
                // has already stopped moving by then, so there is no event
                // left to carry it. It answers nothing at all once the tip is
                // up, because the box does not move while it is shown.
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Tip(self.hover.owed(gfx.egui.egui_ctx(), self.started.elapsed()))
                        .repaint(),
                );

                // -- a folder let go on this window --------------------
                // **Read off `egui`'s accumulated input and above the pass
                // that draws it**, for two reasons that are not the same one.
                //
                // *Above the timers*, because the drop asks the file system
                // what a path is and then reads a directory: inside them it
                // would land in `cost.ui`, which is *"`take_egui_input`
                // through `tessellate`"* and is the number the still-panel
                // reading is made of. A drop is one act on one frame and its
                // cost is the operator's, not the panel's.
                //
                // *Off the input rather than the events*, because the platform
                // delivers a multi-item drag as N entries **in one pass** and
                // not as N passes: `egui-winit` appends each `DroppedFile` to
                // one `Vec`, and it is that whole `Vec` the *one path* rule is
                // about. Taking them here is what makes this program the one
                // that answers a drop — nothing else in `crates/` reads either
                // field, which is the mechanism ADR-0275 took rather than
                // designed around.
                folder_over(&mut self.readout.view, &gfx.egui.egui_input().hovered_files);
                let dropped = std::mem::take(&mut gfx.egui.egui_input_mut().dropped_files);
                if !dropped.is_empty() {
                    let paths: Vec<&std::path::Path> =
                        dropped.iter().map(|file| file.path()).collect();
                    if let Some(said) = folder_dropped(
                        &mut self.readout.view,
                        &mut self.folder,
                        &self.store,
                        self.presets.as_ref(),
                        &paths,
                    ) {
                        println!("{said}");
                    }
                }

                // **What the tip under the pointer says about MIDI, read off
                // the live map.** The page's own `⊕ MIDI:` line is the
                // *mock's* assignment and no operator's, so it is derived
                // here and handed across — the console cannot read a map,
                // because `karakuri-midi` pulls `midir` and ADR-0156 is that
                // it takes no device (ADR-0335, ADR-0336).
                //
                // **Once a frame and only while a pointer is resting on
                // something**, which is a branch on every other frame: a tip
                // that is up asks for no frames at all, so this runs on the
                // frame one appears and on the frame a learn changes one.
                let hovering = gfx.egui.egui_ctx().clone();
                let assignment = self.hover.resting().and_then(|(p, _)| {
                    let surface = gfx.midi.as_ref()?;
                    let operation =
                        asked_at(&self.readout.panel, &hovering, &self.readout.view, p)?;
                    let target = target_of(&operation, &gfx.engine.deck).ok()?;
                    surface.bound(&target)
                });
                self.hover.assign(assignment);

                // -- the egui pass -------------------------------------
                let started = Instant::now();
                let (allocs, bytes) = counted();
                let input = gfx.egui.take_egui_input(&gfx.window);
                let panel = &mut self.readout.panel;
                let view = &mut self.readout.view;
                // **The hover layer paints last, inside the same pass.** It is
                // one closure and not two, because a tip has to go over every
                // card and every bay — which is the mock's `z-index: 30`, and
                // is the order `View::draw` already paints its own four cards
                // in. It draws nothing at all until a dwell has run.
                let hover = &mut self.hover;
                let now = self.started.elapsed();
                let mut output = gfx.egui.egui_ctx().run_ui(input, |ui| {
                    view.draw(ui, panel);
                    hover.paint(ui, panel, view, now);
                });
                let primitives = gfx
                    .egui
                    .egui_ctx()
                    .tessellate(output.shapes, output.pixels_per_point);
                cost.ui = started.elapsed();
                let (allocs2, bytes2) = counted();
                cost.allocs = allocs2 - allocs;
                cost.bytes = bytes2 - bytes;

                // **What `egui` asked for, with the delay it asked for.** It
                // is `Duration::MAX` on a pass that wants nothing, which is
                // every pass on a panel with nothing on it, and that is
                // `Repaint::Never` — the loop then has no reason of its own to
                // draw again. Read off the root viewport's output, after the
                // clock above so that a map lookup is not in the number.
                let asked = Repaint::asked(
                    output
                        .viewport_output
                        .get(&karakuri_console::egui::ViewportId::ROOT)
                        .map_or(Duration::MAX, |v| v.repaint_delay),
                );

                gfx.egui
                    .handle_platform_output(&gfx.window, output.platform_output);

                // -- the frame: one compose, one encoder, one submission --
                //
                // **The engine and the panel are one command buffer, engine
                // first.** `frame::compose` asks both sinks, advances the deck
                // whatever they answer, draws the canvas into the ones that
                // took the frame, and then hands **the frame's own encoder**
                // to the closure below — which is where the whole panel goes.
                //
                // The panel is not a sink, and the argument is written on
                // `compose`: it does not receive the composited frame, it
                // receives the panel, and it happens to sample what a sink
                // produced. An encoder of this file's own would be a second
                // submission over a texture that is a colour attachment in one
                // and a sampled resource in the other, ordered by whatever the
                // queue happened to do — it would work today and be a race
                // nobody wrote down. That is
                // `docs/adr/0166-the-engines-frame-and-the-panels-are-one-submission.md`.
                //
                // **This used to be a second frame loop.** `begin_frame`, a
                // render, a conditional present pass per target, the panel,
                // the submit — all spelled out here, beside the one in
                // `karakuri-cli` that says the same thing differently. That is
                // the drift `karakuri_engine::frame` exists to end, and it is
                // one call now.
                let screen = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [gfx.config.width, gfx.config.height],
                    pixels_per_point: output.pixels_per_point,
                };
                let view_target = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                // **Read from inside the closure, because that is where the
                // engine's half ends and the panel's begins.** `Cost::engine`
                // and `Cost::paint` are then adjacent by construction, rather
                // than two `Instant::now()`s a statement could get between.
                let mut panel_started = None;
                let mut submitting = None;
                let engine_started = Instant::now();
                let composed = {
                    let Gfx {
                        gpu,
                        renderer,
                        engine,
                        projector,
                        ..
                    } = &mut *gfx;
                    let gpu = &*gpu;
                    let Engine {
                        deck,
                        present,
                        picture,
                        previews,
                        slot_bind_groups,
                        look,
                        chain,
                        ..
                    } = engine;
                    // **On the frames it moved and on no others**, which is
                    // where this parts company with the tone map one pass
                    // along: that is one `queue.write_buffer` into storage
                    // sized at construction, and this is a list whose *shape*
                    // may have changed — new procedures to compile, new
                    // targets to allocate. `mix::apply_chain` takes the cheap
                    // path where the shape is the one already running, which
                    // is what a press on a Master row produces
                    // (`docs/principles/0091-cost-is-known-before-it-is-paid.md`).
                    //
                    // **The shipped three and nothing else**, and a refusal
                    // names the address: the Library's drop onto the chain is
                    // M5.16's second pass, and it is what brings a store in.
                    if present.chain_spec() != *chain {
                        if let Err(refusal) = karakuri_environment::mix::apply_chain(
                            present,
                            &gpu.device,
                            &gpu.queue,
                            chain,
                            &|address| karakuri_environment::mix::resolve_procedure(None, address),
                        ) {
                            eprintln!("{refusal} — the chain keeps what it had");
                        }
                    }
                    let textures_delta = &mut output.textures_delta;
                    let cost = &mut cost;
                    // **The picture first, and the projector beside it
                    // when there is one.** Two arrays rather than one of
                    // `Option`s because `compose` takes a slice of live
                    // references and a hole in it would be a sink that has to
                    // be asked whether it is there — which is the
                    // `Sink::acquired` this seam already refused
                    // (ADR-0171). The order is the Outputs row's, which is
                    // also `render_size`'s, so an index in the refusal below
                    // means the same thing in all three.
                    let mut one: [&mut dyn Sink; 1];
                    let mut two: [&mut dyn Sink; 2];
                    let sinks: &mut [&mut dyn Sink] = match projector {
                        Some(p) => {
                            two = [picture, &mut p.sink];
                            &mut two
                        }
                        None => {
                            one = [picture];
                            &mut one
                        }
                    };
                    compose(
                        gpu,
                        deck,
                        present,
                        sinks,
                        &mut |_at, skip| {
                            if let Skip::Fault(why) = skip {
                                println!("a sink stopped taking frames: {why}");
                            }
                        },
                        |_| Committed { steps, look: *look },
                        // -- the panel, into the frame's encoder ------
                        |encoder| {
                            monitor(present, previews, slot_bind_groups, encoder);
                            panel_started = Some(Instant::now());
                            // One id can carry several deltas in a frame: a
                            // font atlas that grew arrives as the whole image
                            // followed by its patches, and applying only the
                            // first would leave holes.
                            let uploading = Instant::now();
                            for (id, deltas) in &textures_delta.set {
                                for delta in deltas {
                                    renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
                                }
                            }
                            cost.textures = uploading.elapsed();
                            let uploading = Instant::now();
                            let user = renderer.update_buffers(
                                &gpu.device,
                                &gpu.queue,
                                encoder,
                                &primitives,
                                &screen,
                            );
                            cost.buffers = uploading.elapsed();
                            let recording = Instant::now();
                            {
                                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("console"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view_target,
                                        depth_slice: None,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            // The console's own ground
                                            // is painted by the central
                                            // panel; this only matters
                                            // for the frame before the
                                            // first one lands.
                                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                    multiview_mask: None,
                                });
                                renderer.render(&mut pass.forget_lifetime(), &primitives, &screen);
                            }
                            for id in &textures_delta.free {
                                renderer.free_texture(id);
                            }
                            // **`epaint` panics on a `TexturesDelta` dropped
                            // unapplied**, and a panic here is reached from a
                            // `winit` callback, which on macOS is an abort
                            // rather than an error. Every delta above has been
                            // handed to the renderer, so this says so.
                            textures_delta.clear();
                            cost.record = recording.elapsed();
                            // **`update_buffers` hands back a command buffer
                            // per `egui` paint callback that asked for one**,
                            // and this console registers no paint callbacks —
                            // the picture is a registered texture drawn as an
                            // image, not a callback. So this is empty, and an
                            // empty submission is skipped rather than made. It
                            // is not dropped: a callback's prepared work
                            // submitted after the pass that reads it would be a
                            // frame behind, so if one ever appears it goes in
                            // ahead — and the engine and the panel stay in the
                            // one submission `compose` makes the moment this
                            // closure returns.
                            submitting = Some(Instant::now());
                            if !user.is_empty() {
                                gpu.queue.submit(user);
                            }
                        },
                    )
                };
                // The engine's half ran from the top of `compose` to the
                // moment it handed the encoder over; the panel's is the rest
                // of the call, the one submission included.
                let panel_started = panel_started.expect("`finally` runs on every frame");
                cost.engine = panel_started - engine_started;
                cost.paint = panel_started.elapsed();
                cost.submit = submitting.expect("`finally` runs on every frame").elapsed();
                // **Said and not returned on**, and neither of this program's
                // sinks can produce it — `Presented::present` is `Ok(())`. It
                // is here because a third sink could, and because a frame the
                // other sinks took is not one this window may drop.
                if let Err(e) = composed {
                    println!("a sink failed to present: {e}");
                }

                // **What the GPU still owed, on the frames that pay to find
                // out.** Everything above stops at a submission, so on every
                // other frame the answer to *how much of this was the shader*
                // is not in this file at all — it arrives one frame later,
                // folded into `wait`, where it is indistinguishable from the
                // vsync idle that field is named for.
                //
                // **Before `present` and after `submit`**, which is the window
                // that contains this frame's work and not the display's pace:
                // `Queue::present` queues the image for the compositor, and a
                // poll on the far side of it would be waiting for a monitor.
                //
                // **A poll error is `None` and not a zero.** A drain that did
                // not happen has no duration, and 0.0 ms here would read as a
                // GPU with nothing to do — the exact failure P-0095 exists to
                // refuse.
                cost.drained = self
                    .costs
                    .audit()
                    .then(|| {
                        let owed = Instant::now();
                        gfx.gpu
                            .device
                            .poll(wgpu::PollType::wait_indefinitely())
                            .is_ok()
                            .then(|| owed.elapsed())
                    })
                    .flatten();

                gfx.gpu.queue.present(frame);

                // **The `tick` that closes this frame, where one is being
                // recorded**, and it is last for `karakuri-cli`'s reason: *"a
                // tick is a terminator rather than a header — `session::split`
                // files each record into the frame of the next tick, so a
                // record written after this frame's tick belongs to the next
                // frame."* Everything this frame decided is above this line:
                // the audio it heard, the tempo correction it made, and every
                // record a press between the last two frames applied.
                //
                // **The measured count, which is what a `tick` is for.**
                // This block used to read *"`STEPS_A_FRAME` and not a measured
                // interval … this window is not timing a performance against a
                // wall clock, and a tick that claimed it was would be a number
                // nothing here measured"*, and that had P-0092 backwards: the
                // rule's live path **is** the measurement — *"live, the engine
                // derives the step count from real time and writes it in"* —
                // and what a replay must not do is derive it again. A `tick`
                // that says `1` because nothing was measured is the number
                // nothing measured, and P-0095 is the other half of why: this
                // stream's ticks are now taken the one way `Record::Tick` says
                // they are taken, so a reader can tell how the number was
                // arrived at (ADR-0297).
                if let Some(recorder) = self.recording.recorder() {
                    recorder.push(Record::Tick { steps });
                }

                self.costs.push(cost);
                App::wants(gfx, &mut self.egui_due, &mut self.costs, asked);
                // **Something is live, so the next frame is asked for here —
                // and asked for without `Costs::owes`.**
                //
                // Nothing used to ask for a frame at this point, and that was
                // ADR-0164's still-panel clause holding: a panel with nothing changing
                // on it drew nothing. A picture that moves is something
                // changing on it, so the clause stops holding the moment the
                // engine runs — which is expected, is what the rest of ADR-0164
                // exists for, and is **not fixed here**. There is no scheduler
                // in this file, the panel is not cached to a texture, and
                // `karakuri_console::repaint` has not been given a fourth
                // answer.
                //
                // What is done instead is to make the price visible.
                // `Costs::owes` is deliberately not called, so every frame the
                // picture asks for lands in the still-panel reading as what it
                // is: a frame drawn on a window nobody touched. The reading
                // then prints the rate rather than the zero, and the next
                // decision gets made on a number.
                //
                // **Asked for only while something is on screen making
                // texels.** It used to be unconditional, with `live` set once
                // when the engine was built and never cleared — so folding the
                // picture away left the loop drawing at full rate for nothing,
                // and the reading went on calling it live. Another machine
                // found that by following this file's own instructions and
                // getting 270 frames out of a window that was supposed to have
                // gone quiet.
                //
                // Then it became the picture alone, and deck A's audition put
                // that wrong again in the same direction: fold the picture and
                // the preview goes on rendering under it, so the panel keeps
                // changing while the loop stops asking for frames. **The rule
                // is anything that makes texels, and the list is closed** —
                // [`live`] is where it is written and where a test can reach
                // it.
                self.costs.live = live;
                // **The verdict this program earned about this adapter's
                // timestamps**, kept for the reading beside `live` and for the
                // same reason: one answer per frame, off whoever took it. The
                // deck's startup probe calibrates against a load whose answer
                // is already known, so this is what the adapter *did* rather
                // than what it advertises (P-0095) — and asking the deck costs
                // a copy of an `Option` rather than a second calibration that
                // could disagree with the numbers the governor decided on.
                self.costs.clock = gfx.engine.deck.clock();
                // **And what the panel asked for on its own account**, which
                // is the other half of why frames are being drawn on an
                // untouched window. Asked of the view here for the same reason
                // `live` is: one answer per frame, kept for the reading.
                self.costs.declared = self.readout.view.animating(self.readout.panel.layout());
                if live {
                    gfx.window.request_redraw();
                }
            }
            _ => App::to_egui(gfx, &mut self.costs, &event),
        }
    }
}

// ---------------------------------------------------------------------------
// The room this instrument is listening to
// ---------------------------------------------------------------------------

/// **What this program opens on, and it is not a flag.**
///
/// `karakuri-cli` is told which input to take with `--audio-in` and refuses to
/// start without the one it was told; this program opens the host's default
/// and, from then on, is told by a hand on the `audio-in` pill. The two are
/// different on purpose and the difference is the surface:
///
/// - **A flag is a contract made before the run.** Asking for one and getting
///   none is a run that is not the run that was asked for, so
///   `karakuri-cli` exits — and it is right to, because a render or a set
///   played from a script has nobody standing there to notice.
/// - **This program has somebody standing there.** It draws a pill that says
///   which input is open and lists the others, so *which room* is a question
///   the panel can both ask and answer while it is running. A second way to
///   say it on the command line would be a launch-time answer to a question
///   the panel already answers better, and `USAGE` says in as many words that
///   this is not `karakuri-cli`'s command line.
///
/// **And it opens something rather than nothing**, which is the choice that
/// matters for what this instrument is: material that moves with the room is
/// what the panel looks like, and an instrument that listens only after being
/// asked comes up looking like one that cannot. `default` is what a machine
/// answers when nobody has chosen, which is exactly the state a program that
/// has just started is in.
const LISTEN_ON: &str = "default";

/// **One simulation step**, which is what the audio path has to be told a
/// frame advances the session by so a beat correction lands on the right one.
/// `karakuri-cli` names the same constant for the same reason.
const DT: f32 = karakuri_engine::set::DT;

/// **Open the input this program listens on, and say what happened.**
///
/// Returns the session and one line for the legend — never a refusal that
/// stops the run, and that is the decision rather than an omission. The three
/// cases it has to be right about are the three the window can meet, and two
/// principles point in different directions across them:
///
/// 1. **No device at all.**
///    [P-0084](../../../docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)
///    — *a quiet room is not a missing microphone* — and neither is a missing
///    microphone a fault. Nobody asked for one here: this program opens the
///    default because that is what an instrument does, and a machine with no
///    input is a machine where every name goes on answering what it answered
///    before audio existed and the oscillator free-runs. It is said out loud,
///    once, and the run continues. Exiting would mean a laptop with its
///    microphone switched off cannot open the panel at all.
/// 2. **A device that was named and is not there.** A different case, and
///    [P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
///    is why: somebody said *that one*, and going quietly on with a different
///    one — or with none — is the silently wrong picture. It cannot happen
///    *here*, because nothing names an input at launch; it happens at the
///    pill, where the list an operator picked from was read at the press and a
///    device can have gone away since. [`attached`] is that case and it is
///    loud there. **Loud and not fatal**, which is where this program parts
///    from `karakuri-cli`: a window with a set on it must not close because an
///    interface was unplugged, and the operator is standing in front of the
///    refusal.
/// 3. **A device that goes away mid-set.** Nothing here notices, deliberately,
///    and that *is* the answer: `karakuri-audio`'s `staleness` takes the
///    confidence of both the signals and the tempo estimate to zero over half
///    a second, every bound parameter is handed back to the value it had, and
///    the grid free-runs from wherever it was. A watchdog that re-opened the
///    stream would be a second answer to a question that already has one, and
///    it would re-lock the grid to a room in the middle of a set. What an
///    operator does about it is pick again on the pill.
///
/// A free function rather than a step of `resumed`, for the reason
/// [`sources_from`] is one: `resumed` cannot be called from a test, and a
/// refusal nobody can reach is a refusal nobody checked. See
/// [`unopened`], which is the half of it that has no device in it at all.
fn listening(session_bpm: f32) -> (Option<audio::Audio>, String) {
    match audio::Audio::open(LISTEN_ON, audio::DEFAULT_LATENCY_OFFSET_MS, DT, session_bpm) {
        Ok(open) => {
            let line = format!(
                "audio in: {} at {} Hz — energy, onset and band0..7 are measured from this room \
                 now, and the beat corrects the session's oscillator. output offset {:.0} ms. \
                 the transport row's `audio-in` pill says which input this is and lists the \
                 others; `b` taps the beat, `,` and `.` move the grid an octave, and `o` and \
                 `p` nudge that offset five milliseconds a press.",
                open.description(),
                open.sample_rate(),
                open.latency_offset_ms()
            );
            (Some(open), line)
        }
        Err(why) => (None, unopened(LISTEN_ON, &why)),
    }
}

/// **What to say about an input that did not open**, and which of the two
/// kinds of nothing it was.
///
/// Split out from [`listening`] because it is the whole of the judgement and
/// none of the device: a machine with no inputs and a machine whose default
/// vanished are two sentences, and the difference between them is the
/// difference between P-0084 and P-0094. Being a function of an error and a
/// string, it is checkable where no input can be opened at all — which is
/// every machine a test runs on, whatever it happens to have plugged in.
///
/// **The empty case is not apologetic and the non-empty one is not calm.** A
/// machine with no inputs is a state; a machine with inputs where the one
/// asked for is not among them is somebody's mistake or somebody's cable, and
/// the list is what they need rather than an invitation to go and look.
pub(crate) fn unopened(selector: &str, why: &audio::AudioError) -> String {
    match why {
        audio::AudioError::NoMatch { available, .. } if available.is_empty() => String::from(
            "audio in: none — this machine has no audio inputs, which is a state and not a \
             fault: every signal name answers what it answered before audio existed, and the \
             grid free-runs at the session tempo. the `audio-in` pill says `none` and its card \
             says so too.",
        ),
        audio::AudioError::NoMatch { available, .. } => format!(
            "audio in: `{selector}` is not one of this machine's {} input{} — {}. nothing is \
             open; pick one on the `audio-in` pill.",
            available.len(),
            match available.len() {
                1 => "",
                _ => "s",
            },
            available.join(", ")
        ),
        // Config, Build, SampleFormat: a device that is there and would not
        // start. Said in the audio crate's own words rather than translated —
        // it is the only thing that knows what a host refused.
        other => format!(
            "audio in: none — `{selector}` is there and would not open: {other}. nothing is \
             open; pick another on the `audio-in` pill."
        ),
    }
}

// ---------------------------------------------------------------------------
// The control surface this instrument is playing from
// ---------------------------------------------------------------------------

/// **Open the surface this program plays from, and say what happened.**
///
/// Returns the surface and one line for the legend — never a refusal that
/// stops the run, which is [`listening`]'s decision one door along and it is
/// the same decision for the same reason. Three cases, and the three are not
/// the microphone's three:
///
/// 1. **Nothing plugged in**, which is most machines and is a state rather
///    than a fault. Nobody named a port: this program takes whatever is there
///    because that is what an instrument does, and a run with no surface is a
///    run played with the pointer and the keyboard, which is every run this
///    program has had until now. Said out loud, once.
/// 2. **A port that is there and will not open** — taken by another program,
///    usually. Said in the port's own words, and the run continues: a window
///    with a set on it must not fail to start because something else has the
///    controller.
/// 3. **A port that goes away mid-set.** Nothing here notices, deliberately.
///    `midir` holds the connection and a device unplugged stops sending; every
///    control on this panel is still under the pointer and under a key, and
///    nothing on the deck moves on its own. What an operator does about it is
///    plug it back in and restart, which is what the legend says — **there is
///    no pill to re-open one**, because the transport row's `map` is one of
///    the two controls the mock draws and this console does not.
///
/// **Which port is the first one there is**, and it is not a flag.
/// `karakuri-cli` is told with `--midi-in` and refuses the run without the one
/// it was told; [`USAGE`] declines that flag by name for
/// [ADR-0220](../../../docs/adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md)'s
/// reason read one column along — this program is the instrument, and a
/// launch-time answer is one an operator standing in front of the panel cannot
/// change. So it opens what is there and says which, exactly as `audio-in`
/// does.
///
/// **Which map is [`midi::map_for`]** — the operator's own under the store,
/// then the one that ships — and the path is printed rather than described,
/// for the reason every line of [`Readout::print_legend`] is derived.
///
/// A free function rather than a step of `resumed`, for [`listening`]'s
/// reason: `resumed` cannot be called from a test, and the sentences are the
/// half of this that has no device in it. See [`surface_line`] and
/// [`unsurfaced`], which are that half.
fn surfaced(
    map: Option<&std::path::Path>,
    waker: EventLoopProxy<()>,
) -> (Option<midi::Surface>, String) {
    // **The wake, and it is the whole of what this closure is.** `send_event`
    // is called on the MIDI thread once per message; `()` says *ask again* and
    // [`App::user_event`] is what asks. The error is dropped because it means
    // the loop has gone, which means the run is ending.
    match midi::Surface::first(map, move || {
        let _ = waker.send_event(());
    }) {
        Ok((surface, notes)) => {
            let line = surface_line(
                surface.port_name(),
                surface.map_name(),
                map,
                surface.mappings(),
                &notes,
            );
            (Some(surface), line)
        }
        Err(why) => (None, unsurfaced(&why)),
    }
}

/// **What the legend says about an open surface**, and it is a function of
/// four facts and nothing else — so it is checkable on a machine with no MIDI
/// on it at all, which is every machine a test runs on here.
///
/// The map's **path** is printed beside its name because the name alone cannot
/// say which of the two tiers answered: `default` under the store and
/// `surface` in the preset library are two different files and an operator
/// who has just learned one wants to know which of them is loaded.
pub(crate) fn surface_line(
    port: &str,
    map_name: Option<&str>,
    map_path: Option<&std::path::Path>,
    mappings: usize,
    notes: &[String],
) -> String {
    let mut line = format!("midi in: `{port}`");
    match (map_name, map_path) {
        (Some(name), Some(path)) => line.push_str(&format!(
            ", map `{name}` from {} — {mappings} mapping{}",
            path.display(),
            if mappings == 1 { "" } else { "s" }
        )),
        // **No map is a state and the sentence is the one the operator needs
        // next**, which is `karakuri-environment`'s own words for it: a
        // surface with no map still reports what it sends.
        _ => line.push_str(
            ", no map — turn a knob and this will print the line that would map it, once per              control",
        ),
    }
    line.push_str(
        ". every mapped message becomes the operation the map names and is performed on the          frame it arrives on, where a press on the mixer's fader is performed — so a knob and          a hand write one record and a session recorded from this surface replays with          neither the surface nor the map attached. nothing on this panel names the map: the          transport row's `map` pill is one of the two controls the mock draws and this          console does not.",
    );
    for note in notes {
        line.push_str(&format!("\n  midi map: {note}"));
    }
    line
}

/// **Where a learned map is written** — `<store>/maps/default.map`, the first
/// of `karakuri_environment::midi::map_for`'s two tiers.
///
/// It is **always** this file, whatever map the run loaded: a run playing the
/// shipped `examples/surface.map` and learning a control writes into the
/// store, which is
/// [P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)
/// held rather than argued. `default` is the name a program with no way to ask
/// uses; naming another is the `map` pill, which is a readout.
fn learned_map(store: &std::path::Path) -> std::path::PathBuf {
    store
        .join(midi::MAPS)
        .join(format!("{}.{}", midi::DEFAULT_MAP, midi::MAP_SUFFIX))
}

/// **What a press on the control under the pointer would ask for**, or `None`
/// where the pointer is on no control at all.
///
/// **This is learn's half of the pointer question**, and it is deliberately
/// the *press* derivation rather than a new one: a control's identity is what
/// a press on it asks the deck for, so a knob learned against it moves exactly
/// what a click moves. The hover layer answers *which* control
/// ([`karakuri_console::hover::Hover::resting`]) and this answers *what it
/// is*, and the two walk the same `view::` derivations — a second geometry
/// here would be a second answer that could disagree with the tip the operator
/// is reading while they learn.
///
/// **The value is a placeholder and is thrown away.** What learn wants is the
/// *address*, which is an operation with its value elided — `LaneTarget`'s own
/// sentence one route along (ADR-0321). `Knob::operation` is the only way to
/// get one out of the console, so this asks it at zero and [`target_of`] reads
/// past the value.
///
/// **The order is the hover layer's**, which is the press order: the controls
/// inside a container before the container. It matters in the Mixer, where a
/// strip's chips sit inside the strip.
fn asked_at(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> Option<Operation> {
    if let Some(row) = look_row(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        &view.arrangement,
        view.look,
    ) {
        if let Some(operation) = row.exposure(p) {
            return Some(operation);
        }
    }
    if let Some(group) = tracker_group(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
    ) {
        if let Some(operation) = group.tapped(p) {
            return Some(operation);
        }
    }
    if let Some(bay) = mixer_bay(ctx, panel.layout(), &view.mixer) {
        if let Some(grab) = bay.grab(p) {
            return Some(grab.knob().operation(0.0));
        }
        if let Some(operation) = bay
            .blend(p)
            .or_else(|| bay.tally(p))
            .or_else(|| bay.mask(p))
        {
            return Some(operation);
        }
    }
    for (index, pane) in view.inspector.iter().enumerate() {
        if let Some(at) = inspector_pane(panel.layout(), index, pane, view.scroll_in(index)) {
            if let Some(grab) = at.grab(pane, p) {
                return Some(grab.knob().operation(0.0));
            }
        }
    }
    None
}

/// **The right-hand side of the map line that reaches this control**, or the
/// sentence saying why there is none.
///
/// # It is `karakuri_midi`'s own list, read the other way
///
/// The eight arms are exactly `Target::spelled`'s eight, which is what makes
/// this safe: every string it returns is a string that crate's parser accepts,
/// and `a_learned_target_is_one_the_grammar_accepts` is what holds that. A
/// ninth spelling invented here would be a line an operator's file could not
/// hold.
///
/// # The parameter arm is the whole reason this function exists
///
/// Every other operation carries its own address — a slot number, a word from
/// a closed list. `WriteParam` carries a **name**, and a map line holds a
/// **position** (ADR-0268): *knob 3 is knob 3 whatever Set is loaded*, and
/// binding to the name would make the mapping a cost paid again on every swap.
/// So this is where the name goes back to being a position, against the Set
/// that is in the deck — `Set::published()` in order, counting from one, which
/// is the number the Inspector draws beside the row and the same reading
/// `karakuri_environment::midi::Decks` makes in the other direction.
///
/// # A refusal is a sentence and not a silence
///
/// A control a map line cannot name is most of this panel, and the operator
/// pointing at one is owed the reason —
/// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).
/// The reasons are the console page's, one control at a time, and this points
/// at the tooltip rather than repeating forty of them.
fn target_of(operation: &Operation, deck: &Deck) -> Result<String, String> {
    let named = |what: &str| {
        Err(format!(
            "learn: `{}` is not something a map line can name — {what}. the control's own \
             tooltip says why, on the `⊕ MIDI:` line",
            operation.title()
        ))
    };
    match operation {
        Operation::SetGain { deck, .. } => Ok(format!("gain {deck}")),
        Operation::SetOpacity { deck, .. } => Ok(format!("opacity {deck}")),
        Operation::SetExposure { .. } => Ok("exposure".to_owned()),
        Operation::SetMaskPosition { deck, .. } => Ok(format!("mask-position {deck}")),
        Operation::SetResidency { deck, residency } => {
            Ok(format!("residency {deck} {}", residency.name()))
        }
        Operation::SetBlendMode { deck, blend } => Ok(format!("blend {deck} {}", blend.name())),
        Operation::TapBeat => Ok("tap".to_owned()),
        Operation::WriteParam {
            deck: slot, param, ..
        } => {
            let index = usize::from(*slot);
            if index >= deck.slot_count() {
                return Err(karakuri_environment::no_such_slot(index, deck.slot_count()));
            }
            let set = deck.slot(EngineSlot(*slot)).set();
            let at = set.published().iter().position(|control| {
                control.key == param.key
                    && control
                        .at
                        .map(|(kind, index)| karakuri_operation::NodeAddress {
                            layer: asked_layer(kind),
                            index,
                        })
                        == param.node
            });
            match at {
                Some(at) => Ok(format!("param {slot} {}", at + 1)),
                // **A control off the published interface**, which the pane
                // draws with its number and its fader gone (`view::Param::ord`
                // is `None` for one). A position is exactly what a knob
                // counts, so there is nothing to bind — and the answer is to
                // publish it, which is a press in the same pane.
                None => named(
                    "that deck's Set does not publish it, so it has no position to count \
                     to — put it on the interface and learn it again",
                ),
            }
        }
        // **A mask's shape is the sharp one and it is not an oversight.** A
        // map line can say a slot, a word from a closed list or a range, and
        // this operation carries an angle as well as a kind — so a pad naming
        // the kind would have to invent the angle beside it (ADR-0202,
        // ADR-0209).
        Operation::SetMaskShape { .. } => {
            named("a map line has no way to write an angle, so the shape has no spelling")
        }
        _ => named("no map target names it"),
    }
}

/// **What to say about a surface that did not open**, and which of the two
/// kinds of nothing it was — [`unopened`]'s shape one door along, and split
/// out for its reason.
///
/// **The empty case is not apologetic.** A machine with nothing plugged in is
/// the ordinary state of this program and always has been; saying it as a
/// failure would read as one. A machine that *has* inputs and would not open
/// the first is somebody's other program holding the port, and the message
/// `karakuri-midi` came back with is the only thing that knows which.
pub(crate) fn unsurfaced(why: &str) -> String {
    if why.contains("there are no MIDI inputs") {
        return String::from(
            "midi in: none — nothing is plugged in, which is a state and not a fault: every              control on this panel is reached by the pointer and by the keyboard, and that is              every run this program has had. plug a surface in and start again.",
        );
    }
    format!(
        "midi in: none — {why}. the panel runs; every control is still under the pointer and          under a key. this port is opened once, at startup, so a surface freed later is          reached by starting again."
    )
}

/// **What the `audio-in` pill reads**, out of the session this program opened.
///
/// One line, and it is a function rather than an assignment for the reason
/// [`transport`] is one: it is the seam, and there is exactly one place the
/// answer is derived. The card's list is **not** here — it is read on the
/// press that opens the card and nowhere else (P-0091), so a reading taken
/// every frame would be a directory read on the frame path with a microphone
/// in place of the directory.
fn told(open: Option<&audio::Audio>) -> AudioIn {
    let mut told = AudioIn::NONE;
    told.device = open.map(|open| open.description().to_owned());
    told
}

/// **A press on one of the `audio-in` card's rows, performed**, and what this
/// file says about it. `None` for every operation that is not it, exactly as
/// [`arrangement`] and [`pointed`] answer `None` for everything that is not
/// theirs.
///
/// This is the second of [`listening`]'s three cases and the only one that can
/// arrive during a set: the card lists what the host had **at the press that
/// opened it**, and an interface unplugged between that press and this one is
/// a name the operator picked that is not there any more. P-0094 — the refusal
/// is printed with the list as it is *now*, and **the input that was already
/// open stays open**: dropping it would answer a mistyped pick by taking away
/// the room, which is the one thing nobody asked for.
///
/// **`AttachBeatSource` writes no record** (`written` answers
/// `Silent(NoRecord)`: no session-stream variant carries what the beat is
/// taken from), so nothing downstream of this moves the deck. What moves is
/// this program's own audio session and the pill that reads it.
///
/// A `BeatSource::Process` reaches here and is declined in one sentence: the
/// panel has no control that names one and `--tempo-source` is
/// `karakuri-cli`'s. It is answered rather than ignored, because an operation
/// that arrives and does nothing at all is the failure P-0094 is about.
pub(crate) fn attached(
    open: &mut Option<audio::Audio>,
    session_bpm: f32,
    told_pill: &mut Option<AudioIn>,
    operation: &Operation,
) -> Option<String> {
    let Operation::AttachBeatSource { source } = operation else {
        return None;
    };
    let selector = match source {
        BeatSource::AudioInput(selector) => selector,
        BeatSource::Process(command) => {
            return Some(format!(
                "  attach: `{command}` is a process, and nothing on this panel starts one — \
                 `karakuri-cli --tempo-source` is where that half of the row lives"
            ))
        }
    };
    // The offset the operator has already dialled in survives the change of
    // device: it is a property of this room's outputs and not of its input,
    // which is the whole of what `LATENCY_OFFSET_RANGE`'s documentation is
    // about. A new session at the default would silently undo it.
    let offset = open
        .as_ref()
        .map(|open| open.latency_offset_ms())
        .unwrap_or(audio::DEFAULT_LATENCY_OFFSET_MS);
    match audio::Audio::open(selector, offset, DT, session_bpm) {
        Ok(opened) => {
            let line = format!(
                "  attach: {} at {} Hz -> AttachBeatSource -> no record, and that is settled: \
                 nothing in the session stream says what the beat was taken from. the grid \
                 follows this room now, at offset {:.0} ms",
                opened.description(),
                opened.sample_rate(),
                opened.latency_offset_ms()
            );
            *open = Some(opened);
            *told_pill = Some(told(open.as_ref()));
            Some(line)
        }
        // **The one that was open stays open**, and the pill goes on naming
        // it: what failed is the pick, not the room.
        Err(why) => Some(format!(
            "  attach: {} — {}",
            selector,
            match open.as_ref() {
                Some(open) => format!("{why}. `{}` is still open", open.description()),
                None => format!("{why}. nothing is open"),
            }
        )),
    }
}

/// **What the offset keys say on a panel with no input attached.**
///
/// `docs/manual/console.html` is the specification and it is plain about it:
/// *"It only means anything with an audio input attached, and the audio-in
/// pill is what says whether there is one."* So the press changes nothing,
/// says why, and names the control that would fix it — [`tapped`]'s and
/// [`scaled`]'s sentence for the same state, one row along.
pub(crate) const NO_ROOM_FOR_AN_OFFSET: &str =
    "offset: no audio input — the offset is the delay between \
                                     what a room hears and what it sees, and there is no room. \
                                     open one on the transport row's `audio-in` pill";

/// **What to say about an offset that moved**, out of what was asked for and
/// what the session came back with.
///
/// A function of two numbers and nothing else, so that both halves of
/// `console.html`'s contract are checkable without a device:
///
/// - **The sign, in words.** *"Negative and the picture waits for the music,
///   positive and it leads"* — the page says it in words rather than leaving
///   `−15 ms` to be interpreted, and so does this.
/// - **The bound, when it bit.** The value is *"held inside 200 ms either
///   way"*, which `karakuri_environment::audio` enforces and this reports: a
///   press that asked for 205 and got 200 is a control at the end of its
///   travel, and a control that answers the same number twice with nothing
///   said is indistinguishable from a broken one (P-0094).
pub(crate) fn offset_said(asked: f32, now: f32) -> String {
    let sense = match now < 0.0 {
        true => "the picture waits for the music",
        false => "the picture leads the music",
    };
    let held = match (asked - now).abs() > f32::EPSILON {
        true => format!(
            " — held at {:+.0} ms, which is as far either way as it goes",
            now
        ),
        false => String::new(),
    };
    format!("  offset: {now:+.0} ms — {sense}{held}")
}

/// **The latency offset, performed against the session this program opened**,
/// and `None` for every operation that is not it — [`attached`]'s shape, one
/// control along, and beside it in [`App::performed`] for the same reason.
///
/// **`SetLatencyOffset` writes no record** (`written` answers
/// `Silent(NoRecord)`: nothing in the session stream carries a delay between
/// two outputs, which is a property of a room and not of a performance), so
/// nothing downstream of this moves the deck. What moves is the lead every
/// beat correction is applied with — `Audio::output_lag` — and the frame the
/// picture is drawn on relative to it.
///
/// **The operation is absolute and this is where it lands.** It is applied
/// through `Audio::nudge_latency_offset`, which is the only way in and is the
/// one that clamps: the offset is held inside `LATENCY_OFFSET_RANGE` there, so
/// this file states no bound of its own and cannot state a different one. A
/// *setting* becomes the step that reaches it, which is what lets a fader
/// emit this operation the day one exists without a second application path.
///
/// **With nothing open there is nothing to offset**, and the key arm says so
/// before an operation is built — see [`NO_ROOM_FOR_AN_OFFSET`]. This arm
/// answers the case an operation arrives from anywhere else in that state,
/// because an operation that arrives and does nothing at all is the failure
/// P-0094 is about.
pub(crate) fn nudged(open: &mut Option<audio::Audio>, operation: &Operation) -> Option<String> {
    let Operation::SetLatencyOffset { ms } = *operation else {
        return None;
    };
    let Some(open) = open.as_mut() else {
        return Some(format!("  {NO_ROOM_FOR_AN_OFFSET}"));
    };
    let now = open.nudge_latency_offset(ms - open.latency_offset_ms());
    Some(offset_said(ms, now))
}

/// **The grid's tempo, named by hand and handed to the room's tracker** — and
/// `None` for every operation that is not [`Operation::SetFreeRunTempo`].
///
/// # It does not move the grid, and that is what separates it from [`nudged`]
///
/// The offset above is `Silent(NoRecord)`: nothing in a session stream carries
/// a delay between two outputs, so the session this program opened is the only
/// thing that holds it. A free-run tempo is the opposite — `written` answers a
/// `Record::Tempo` for it, and [`apply`] is what applies it, through the same
/// `audio::apply_tempo` a replay goes through. Moving the oscillator here as
/// well would be a second route into the engine, taken only when a device
/// happens to be open (P-0090).
///
/// So what this hands over is the state the record does not carry: the beat
/// lock's run of evidence, and the window the tracker searches. Both are
/// `Audio::set_tempo`, and the argument for each is there and in
/// `karakuri_audio`'s `BeatLock::retarget`.
///
/// # With nothing open it says nothing, and that is the state it is for
///
/// [`nudged`] refuses out loud with no session — an offset belongs to one, and
/// an operation that arrives and does nothing at all is what P-0094 is about.
/// This is the other way round: *what the grid runs at with nothing driving
/// it* is exactly the case with no device, [`apply`] moves the oscillator and
/// prints the line, and a refusal here would be this file talking about a
/// tracker that is not part of the operation. What it says when there **is**
/// one is that the set was accepted and the room is still being tracked, which
/// is the one thing an operator cannot see from the tempo alone.
fn retargeted(open: &mut Option<audio::Audio>, operation: &Operation) -> Option<String> {
    let Operation::SetFreeRunTempo { bpm } = *operation else {
        return None;
    };
    let open = open.as_mut()?;
    open.set_tempo(bpm);
    Some(format!(
        "  tempo: the room is still being tracked — the window moved to {bpm:.1} with the grid, \
         and the next estimate is made around it rather than about where the grid was"
    ))
}

/// **One frame's worth of audio**: read the room, and hand the session what it
/// said.
///
/// The same three lines `karakuri-cli`'s `measure_audio` is, minus the two
/// halves this program does not have — there is no session recorder to hand
/// the record to, and no tempo source to yield the grid to, so the grid is
/// always this tracker's ([`audio::Grid::Owned`]).
///
/// **The signals are copied out of the deck and back in**, which is what
/// `Deck::signals` and `set_signals` are for: the bus is a `Copy` value and
/// the deck is the model of record for it, so an `AudioFrame` reaching a
/// binding goes through the deck rather than round it.
///
/// `interval` is how fast frames are actually arriving, which is half the
/// output lag a beat correction leads by, and `steps` is how much session this
/// frame is worth. **Both are [`App::clock`]'s one measurement**, which is
/// `karakuri-cli`'s arrangement of the same call: the interval the step count
/// was derived from is the interval the lag is built from, so the two cannot
/// disagree about how long this frame was.
///
/// **It used to be [`Costs::rate_now`] inverted**, on the argument that the
/// row's `fps` and the lag should be one number. They are not one question.
/// `rate_now` is *frames drawn on an untouched window over the stretch since
/// something touched it* — the still-panel reading — so it is `None` on every
/// frame near a pointer, a key or a resize, and its stretch counts frames that
/// were asked for as zero while the seconds go on running. The lag wants the
/// interval between this frame and the last one, on the frames an operator is
/// working, which is exactly the number the clock takes for the `tick`.
/// `Audio::frame` still ignores an interval outside `(0, 1)`: the smoothed
/// value holds, which is the right answer for the first frame of a run and for
/// one that followed a stall.
fn measure_audio(
    open: &mut Option<audio::Audio>,
    deck: &mut Deck,
    interval: f32,
    steps: u8,
    recorder: Option<&mut karakuri_environment::session::Recorder>,
) {
    let Some(open) = open.as_mut() else {
        return;
    };
    let mut signals = *deck.signals();
    let (_audio, tempo) = open.frame(
        &mut signals,
        interval,
        f32::from(steps) * DT,
        audio::Grid::Owned,
    );
    deck.set_signals(signals);

    // **Into the session, where one is being recorded**, and this is the half
    // this program did not have when the paragraph above was written.
    //
    // **Swapped, not cloned**, which is `karakuri-cli`'s own line: the record
    // carries a `Vec` of bands and this is the frame path, so `push_audio`
    // takes this one and leaves an empty shell behind. Nothing allocates. A
    // frame with no shell free is counted rather than dropped silently — see
    // `session::Recorder::push_audio`.
    //
    // **The measurement and the correction are both pushed, in that order**,
    // because that is the order they happened in: a replay reading the stream
    // applies the tempo the frame decided after the audio the frame heard.
    //
    // **After the sentence below rather than before it**, which costs nothing
    // and keeps that reading the way it was written: the report matches on the
    // record and this consumes it.
    let recorder = match recorder {
        Some(recorder) => {
            recorder.push_audio(open.record_mut());
            Some(recorder)
        }
        None => None,
    };

    // **A correction worth saying out loud is one that is a decision rather
    // than a trim** — acquiring, re-acquiring, a tap, an octave — which is
    // `karakuri-cli`'s rule and is here for P-0094's reason: an operator who
    // cannot see the grid decide cannot tell a lock from a coincidence. A trim
    // happens on every frame once locked and says nothing.
    let reason = open.reason();
    if let (Some(Record::Tempo { bpm, .. }), Some(reason)) = (&tempo, reason) {
        if !matches!(reason, karakuri_environment::audio::Reason::Trim) {
            println!("beat: {reason:?} at {bpm:.1} bpm");
        }
    }
    if let (Some(recorder), Some(record)) = (recorder, tempo) {
        recorder.push(record);
    }
}

/// **What the tracker's three controls read this frame**: the offset the open
/// session is holding, and which way the grid can still be moved an octave.
///
/// [`transport`]'s shape one group along the same row — a function of what this
/// program can see and of nothing the console could work out for itself.
///
/// **The offset is `None` where nothing is open**, and that is the state rather
/// than a default: an offset belongs to a session, and
/// `karakuri_environment::audio`'s `DEFAULT_LATENCY_OFFSET_MS` is where the
/// *next* session starts rather than a value anything is holding now. The
/// console draws no track at all for it, which is `View::audio`'s own rule one
/// control to the left.
///
/// **The two octave halves are the range against the session tempo**, which is
/// exactly what `Audio::octave` refuses on — `BeatLock::octave` is
/// `BPM_RANGE.contains(&(bpm * factor))` and nothing else — so the chip the
/// panel draws inert is the press the lock would turn down. The range is asked
/// for by name through `karakuri_environment::audio`, which is the door this
/// program takes its audio through; a `60.0..=200.0` written here would be a
/// second copy of the tracker's own bound.
///
/// **The tempo is the session's oscillator**, which is the number the transport
/// row draws and the number the lock multiplies: one reading, so the chip that
/// is drawn and the press that is refused cannot come apart.
///
/// **Not `Audio::octave` asked twice**, which is the obvious alternative and is
/// wrong twice over: it *performs* the move, and it needs a session, where both
/// halves are drawn on a console with no input open — the refusal that is
/// *drawn* is the range's, and the one for a room that is not being listened to
/// is said out loud by [`scaled`].
pub(crate) fn tracking(open: Option<&audio::Audio>, bpm: f32) -> Tracker {
    Tracker {
        offset_ms: open.map(audio::Audio::latency_offset_ms),
        halve: audio::BPM_RANGE.contains(&(bpm * 0.5)),
        double: audio::BPM_RANGE.contains(&(bpm * 2.0)),
    }
}

/// **The two operations that move the room's tracker**, performed against the
/// session this program opened — and `None` for every operation that is not one
/// of them.
///
/// [`attached`]'s and [`nudged`]'s shape, and it is deliberately **not** beside
/// them in [`App::performed`]: those two are `Silent(NoRecord)`, so the line
/// `unwritten` prints after them is true and they fall through to it. These two
/// are `Owed(NotSettled)`, so the arm that calls this leaves as soon as it
/// answers — the reason is written at the call, and what the alternatives were
/// is
/// [ADR-0278](../../../docs/adr/0278-an-operation-no-record-can-be-written-for-leaves-the-window-before-it-is-written.md).
///
/// **`Instant::now()` is read here rather than passed in**, because the instant
/// a tap means is the instant it arrived and this is the last place that is
/// still true. `started` is the run's own origin and comes from the caller: it
/// is `App::started`, the one this program measures every tap against, and a
/// second origin taken here would put two taps on two clocks.
fn tracked(gfx: &mut Gfx, started: Instant, operation: &Operation) -> Option<String> {
    match operation {
        Operation::TapBeat => Some(tapped(
            &mut gfx.audio,
            &mut gfx.engine.deck,
            Instant::now(),
            started,
        )),
        Operation::ScaleGrid { by } => Some(scaled(&mut gfx.audio, &mut gfx.engine.deck, *by)),
        _ => None,
    }
}

/// **A tap on the beat**, performed against the room this program is listening
/// to, and what to say about it.
///
/// # Why it does not go through `written`
///
/// Every other control on this panel emits an `Operation`, `written` turns it
/// into a `Record` and [`apply`] moves the deck with it — P-0090. A tap
/// **does** end in a record: `karakuri_environment::audio` writes a
/// `Record::Tempo` for it and applies it to the session's oscillator, which is
/// the same record a replay would hand the engine. What it cannot do is come
/// out of `written`: that function is a pure function of the operation and a
/// reading, and a tap's record is the *beat lock's* answer — the tapped tempo,
/// the phase error against the oscillator, the output lag — none of which a
/// `Current` carries. So `written(TapBeat)` answers `Owed(NotSettled)`, and
/// routing this key through [`App::performed`] would print *"nothing moved,
/// and nothing here decides it"* about a press that moved the grid.
///
/// **That is a gap in `karakuri-operation-record` and it is named here rather
/// than papered over**: the day a `Current` can carry a correction, this key
/// emits like every other control and this function goes. Until then it is
/// `karakuri-cli`'s own wiring, which is what the panel was asked to use.
fn tapped(
    open: &mut Option<audio::Audio>,
    deck: &mut Deck,
    at: Instant,
    started: Instant,
) -> String {
    let Some(open) = open.as_mut() else {
        return String::from(
            "tap: no audio input — a tap sets the grid this room is being tracked against, and \
             there is no room. open one on the transport row's `audio-in` pill",
        );
    };
    let mut signals = *deck.signals();
    let record = open.tap(&mut signals, at, started);
    deck.set_signals(signals);
    match record {
        Record::Tempo { bpm, shift, .. } => format!(
            "tap: -> Record::Tempo {{ bpm: {bpm:.1}, shift: {shift:+.3} }} — three taps or more \
             set the tempo and any tap sets the phase"
        ),
        other => format!("tap: -> {other:?}"),
    }
}

/// **The grid, an octave up or down**, performed against the same session, and
/// what to say about it.
///
/// [`tapped`]'s note about `written` word for word: `ScaleGrid` is the other
/// half of that `Owed(NotSettled)` arm, and for the same reason.
///
/// **Refused where the result would leave the trackable range**, which is the
/// lock's call and not this file's — 60 to 200 BPM is under two octaves wide,
/// so at most one of the two directions is ever live and a control that undid
/// itself two seconds later would be worse than one that says no.
fn scaled(open: &mut Option<audio::Audio>, deck: &mut Deck, by: GridScale) -> String {
    let (factor, word) = match by {
        GridScale::Halve => (0.5, "half"),
        GridScale::Double => (2.0, "double"),
    };
    let Some(open) = open.as_mut() else {
        return format!(
            "grid: no audio input — {word} moves the tracker's octave window, which only exists \
             while a room is being tracked. open one on the transport row's `audio-in` pill"
        );
    };
    let mut signals = *deck.signals();
    let moved = open.octave(&mut signals, factor);
    deck.set_signals(signals);
    match moved {
        Some(Record::Tempo { bpm, .. }) => format!(
            "grid: {word} -> Record::Tempo {{ bpm: {bpm:.1} }} — the tracker's window went with \
             it, and the phase did not move"
        ),
        Some(other) => format!("grid: {word} -> {other:?}"),
        None => format!(
            "grid: {word} refused — the result would leave the trackable range, and the next \
             estimate that disagreed would drag the grid straight back"
        ),
    }
}
