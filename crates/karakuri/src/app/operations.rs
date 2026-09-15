//! Operation execution, projector window routing, and load/overlay/restore dispatch.

use super::*;
use karakuri_console::focus;
use karakuri_console::repaint::{Change, Repaint};
pub(crate) use karakuri_console::view::TextInputKind;
use karakuri_console::view::View;
use karakuri_engine::WindowSink;
use karakuri_operation::{Operation, Output};
use std::sync::Arc;
use std::time::Instant;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use crate::CANVAS;

/// Open or close the projector window, and say what happened.
///
/// The one operation in this file that makes a *window*, which is why it takes
/// an `&ActiveEventLoop` and why it is reached from `window_event` rather than
/// from [`App::performed`]: `winit` will not create a window without one, and
/// `performed` is handed a [`Gfx`] and no event loop.
///
/// # What each answer is
///
/// - `Projector(0)` on opens a window, makes a surface on the device the
///   panel is already using, configures it and puts a [`WindowSink`] over it.
///   The next frame's [`render_size`] sees a second output and the frame
///   follows the larger of the two.
/// - `Projector(0)` off drops the [`Projector`], which drops the surface
///   and the last reference to the window — so the window closes and the sink
///   leaves the slice on the same statement. Nothing is torn down in an order
///   this file has to remember.
/// - `Program` never arrives: the picture's on and off is the fold, and
///   the console performs it where the arrangement is (`Readout::sink`). It is
///   an arm here so that the match is exhaustive and says so.
/// - `Plugin(n)` is refused with the sentence that names what is missing,
///   which is [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
///   there is no manifest to read a plugin sink out of, and the console draws
///   both plugin chips `no plugin` for that reason, so this is only reachable
///   from a route that is not the panel.
///
/// # The surface has to be the format the present pass was built for
///
/// [`Present`]'s pipeline names one target format at construction, and that is
/// [`Gfx::picture_format`] — the first sRGB format the console's own surface
/// offers, read off it once in [`App::resumed`], so the encode is the
/// hardware's and happens exactly once (P-0064). A surface in another format
/// would need a second pipeline, which is a *second present pipeline* and is
/// exactly what ADR-0247 says nothing needs. So this asks the projector's
/// surface for that same format and refuses to open the window when it is not
/// offered, naming both — a window that opened and drew nothing would be the
/// silent wrong picture P-0094 refuses, and this is a refusal before the show
/// rather than a fault during one.
///
/// Two surfaces of one adapter offer the same formats, so the refusal is
/// reached only where the projector's window is on a display the console's
/// adapter does not drive.
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
                "outputs: plugin {n} is not loaded — there is no plugin manifest to read a \
                 sink out of, and `docs/plugins.md` is the specification nothing implements"
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
    let mut attrs = Window::default_attributes()
        .with_title("Karakuri — projector")
        // **The session canvas**, which is the size an output starts at when
        // nothing else says (ADR-0246). The operator resizes it, or makes it
        // fullscreen, and the frame follows.
        .with_inner_size(winit::dpi::LogicalSize::new(CANVAS.0, CANVAS.1));
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::WindowAttributesExtMacOS;
        attrs = attrs.with_tabbing_identifier("projector");
    }
    let window = match event_loop.create_window(attrs) {
        Ok(window) => Arc::new(window),
        // **Reported rather than panicked**, for `resumed`'s reason: a panic
        // here is reached from a `winit` callback and cannot unwind across the
        // Objective-C frame on macOS, so it aborts with no sentence anywhere.
        Err(e) => return Some(format!("outputs: the projector window did not open: {e}")),
    };
    // **The instance the adapter came from**, and never a fresh one: a
    // surface asked about an adapter of another instance is a resource that
    // instance does not hold, and `wgpu-core` aborts on it inside a `winit`
    // callback with no sentence anywhere.
    let surface = match gfx.gpu.instance.create_surface(window.clone()) {
        Ok(surface) => surface,
        Err(e) => return Some(format!("outputs: the projector has no surface: {e}")),
    };
    let caps = surface.get_capabilities(&gfx.gpu.adapter);
    if !caps.formats.contains(&gfx.picture_format) {
        return Some(format!(
            "outputs: the projector window cannot be opened — the present pass draws {:?} \
             and this surface offers {:?}. A second format would want a second present \
             pipeline, which is what one render scaled into every output exists to avoid",
            gfx.picture_format, caps.formats
        ));
    }
    let size = window.inner_size();
    let size = (size.width.max(1), size.height.max(1));
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: gfx.picture_format,
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

/// What the window loop does about what a press asked for, for every answer but
/// the two that reach the store.
///
/// `Asked::Scope` and `Asked::Load` are the caller's, because both end in a
/// directory read or a file write and this function has neither the store nor
/// the folder; everything else is one operation or one sentence.
///
/// A refusal is said out loud, which is the whole of what `Asked::Nothing`
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
            // The grid's own tempo, read off the oscillator at the press —
            // the same reading the transport row is drawn from. What a press
            // moves it by is [`tempo_key`].
            focus::Level::Tempo => Acted::Emitted(Some(Operation::SetFreeRunTempo {
                bpm: tempo_key(*step, gfx.engine.deck.signals().oscillator().bpm()),
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
        // A press on the audio-in pill or on one of its rows, performed
        // through the method a pointer press on the same rectangle already
        // reaches: opening the card enumerates the machine's inputs, which is a
        // device read and not a thing `karakuri-console` can do at all
        // (ADR-0156, ADR-0350).
        focus::Asked::Listened(ask) => {
            let acted = readout.listened(ask.clone());
            return App::performed(
                gfx,
                started,
                readout,
                recorder,
                &acted,
                Change::Pointed(true).repaint(),
            )
            .repaint;
        }
        // And a press on the arrangement pill or on one of its menu rows: the
        // names filed are a directory and a save writes a file.
        focus::Asked::Arranged(ask) => {
            let acted = readout.arranged(ask.clone());
            return App::performed(
                gfx,
                started,
                readout,
                recorder,
                &acted,
                Change::Pointed(true).repaint(),
            )
            .repaint;
        }
        // The caller answers these two, and it returns before it gets here.
        focus::Asked::Scope | focus::Asked::Load => {
            unreachable!("the scope and the load are answered where the store is")
        }
    };
    App::performed(gfx, started, readout, recorder, &acted, Repaint::Never).repaint
}

/// A load, performed — [`loading`] reached from an operation, and `None` for
/// every operation that is not one.
///
/// `Operation::LoadSet` writes no record either, so this is [`pointed`]'s shape
/// one bay along: the surface that names it performs it. What it does not do is
/// touch the deck, which is the whole design — see [`loading`] and
/// [`Engine::aimed`].
///
/// Every failure is a sentence and none of them moves anything: a slot the deck
/// has not got, a store that will not open, a Set that is not there, a
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

/// A procedure loaded over one layer of what a deck is playing, performed — a
/// press on a procedure row's `load`, on one of its row menu's four loads, or a
/// drag of it onto a strip or a cell, and `None` for every operation that is
/// not one.
///
/// # It is [`played`]'s shape with one file instead of every file
///
/// A Set load re-points the slot at every file that Set names; this re-points
/// it at the files it is already on with one of them replaced, which is the
/// whole of ADR-0338 taken literally. Both are one `Aiming::re_point`, both are
/// compiled off the render thread and judged at a frame boundary on what one
/// frame of the result costs, and the Staging lane says which of the three
/// happened. Nothing is installed and nothing is written where the presets are.
///
/// `Aim::set` is left where it is, which is the half of the maintainer's answer
/// that has a mechanism behind it: the versions this slot writes from here on
/// go on being filed under the Set it started from, so the `history` chip keeps
/// listing that deck's versions and the snapshot every compile takes stays
/// alive (ADR-0304, ADR-0308). It follows from `overlaying` restating the aim
/// rather than building one.
///
/// The strip then reads `<base> + <kir>`, so what is on air says what it is
/// made of and never claims to be a Set the library holds. `keep` is what gives
/// it a name, and it files a new Set exactly as it does for any other deck —
/// what `Playing` gathers is the aim's own files, which is what this changed.
///
/// Written on the aim rather than on the swap, which is [`played`]'s own choice
/// and its argument word for word: the build may be refused or land and stop
/// its slot for cost, and a readout that waited for the verdict would name
/// material that is no longer in the file. The staging lane is the surface
/// built for that disagreement.
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

/// A version put back — a row of the Library bay's `history` scope landed on
/// the node it was a version of, and `None` for every operation that is not
/// one.
///
/// # It is a load, and it goes the way every other load goes
///
/// The snapshot's bytes are written over that node's working copy under
/// `<store>/scratch/`, where the slot's watcher is already looking, and nothing
/// else is touched: no deck, no aim, no channel. So the worker reads it,
/// compiles it off the render thread, swaps it at a frame boundary and rolls it
/// back on its own if it cannot hold the budget — which is [`loading`]'s
/// argument met from the other end, and
/// `docs/adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md`
/// is where it is written down. Nothing is installed.
///
/// The version it replaces is kept by the same act. The watcher snapshots at
/// its compile-success point
/// (`docs/adr/0089-history-is-gated-on-compiling-not-on-landing.md`), so what
/// was on the node before this write is the next row of the very listing the
/// press came off — a landing you did not mean is itself undoable.
///
/// # Finding the file again, and why no path crosses the seam
///
/// The console says a name — [`version_row`]'s spelling, the one it was handed
/// — and this re-asks `history::list` and rebuilds that spelling per candidate
/// to find the row. That is `SetTransfer::Take`'s arrangement exactly: *"the
/// panel's route re-asks the listing and finds the row by the word that was
/// pressed, so no surface spells a path"*.
///
/// Which node the file is is asked of the aim rather than of the launch copies.
/// [`Engine::pointing`] is the run's one `mcp::Slots`, written out of what each
/// watcher is *pointed at* — `Aiming::at` — so the answer follows a library
/// load. `Slots::file` is the one walk that turns `(slot, layer, index)` into a
/// file, and it is asked rather than repeated.
///
/// This used to build a `Slots` of its own here, because the one the MCP server
/// held was the launch working copies and went stale on the first load
/// (ADR-0308's *Doubted*). The server reads this same handle now, so the second
/// one is gone rather than kept beside it.
///
/// # Five refusals, and each says where the deck is still pointed
///
/// A slot the deck has not got, a deck playing no Set at all, a version that is
/// not one of that Set's, a node the deck does not hold, and a file that is
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

/// The Set the load pulldown's deck is running, or `None` for a deck playing
/// the pair the run was launched with.
///
/// It is read off the aim and off nothing else, which is ADR-0304: the id rides
/// the `watch::Aim` a load sends, restated by every rewiring, and
/// `Gfx::material` beside it is the mixer strip's *readout* — the pair at
/// launch, and never an id a listing can match.
///
/// The pulldown and not the selection, which is ADR-0305 read on a second
/// control: the letter in that foot is what says where a load lands, so it is
/// what says whose history the `history` scope is showing. A target past the
/// slots the deck has answers `None`, which `View::aim_at` already refuses and
/// this does not depend on.
///
/// Owned, because the caller is about to take `&mut View`. One `String` per
/// press on a path that is about to read a directory.
pub(crate) fn aimed_set(gfx: &Gfx, view: &View) -> Option<String> {
    gfx.engine
        .aimed
        .get(usize::from(view.target_deck()))
        .and_then(|aiming| aiming.at.set.clone())
}

/// Coordinates post-event side-effects (disk saving, Set reading) that
/// cannot run inside a rendering frame.
pub(crate) fn handle_post_event_side_effects(
    keeping: &mut super::Keeping,
    engine: &crate::Engine,
    store: &std::path::Path,
    view: &mut View,
    acted: &Acted,
) {
    match acted {
        Acted::Emitted(Some(Operation::SaveSet { deck, id })) => {
            keeping.save_set(
                engine,
                store,
                karakuri_environment::Asked::Operator,
                usize::from(*deck),
                id.clone(),
                None,
            );
        }
        Acted::Emitted(Some(Operation::ReadSet { .. })) => {
            println!("{}", crate::bridge::read_reading(view, store));
        }
        _ => {}
    }
}
