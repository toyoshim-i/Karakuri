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

/// Opens or closes the projector window, configuring a [`WindowSink`] using the shared adapter format (ADR-0247, P-0064, P-0083, P-0094).
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
        Output::Plugin(n) => return route_plugin(gfx, n, on),
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
        // Session canvas inner size (ADR-0246).
        .with_inner_size(winit::dpi::LogicalSize::new(CANVAS.0, CANVAS.1));
    #[cfg(target_os = "macos")]
    let attrs = {
        use winit::platform::macos::WindowAttributesExtMacOS;
        attrs.with_tabbing_identifier("projector")
    };
    let window = match event_loop.create_window(attrs) {
        Ok(window) => Arc::new(window),
        // Log error and report refusal instead of panicking across the winit boundary (ADR-0168).
        Err(e) => return Some(format!("outputs: the projector window did not open: {e}")),
    };
    // Create surface on the adapter's parent GPU instance (ADR-0324).
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
        // Fifo presentation mode matching console window (ADR-0171).
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

#[cfg(target_os = "windows")]
pub(crate) const REQUIRED_SURFACE: &str = "dxgi";
#[cfg(target_os = "macos")]
pub(crate) const REQUIRED_SURFACE: &str = "iosurface";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(crate) const REQUIRED_SURFACE: &str = "unknown";

pub(crate) fn is_plugin_available(gfx: &Gfx, n: u8) -> bool {
    gfx.discovered_plugins
        .get(n as usize)
        .is_some_and(|p| p.supports_surface(REQUIRED_SURFACE))
}

fn route_plugin(gfx: &mut Gfx, n: u8, on: bool) -> Option<String> {
    if !on {
        return Some(match gfx.plugin.take() {
            Some(_) => format!("outputs: plugin {n} is off"),
            None => format!("outputs: plugin {n} was already off"),
        });
    }
    if gfx.plugin.is_some() {
        return Some(format!("outputs: plugin {n} is already on"));
    }
    let plugin_info = match gfx.discovered_plugins.get(n as usize) {
        Some(p) => p,
        None => {
            return Some(format!(
                "outputs: plugin {n} is not loaded — no compatible plugin discovered in places"
            ));
        }
    };
    if !plugin_info.supports_surface(REQUIRED_SURFACE) {
        return Some(format!(
            "outputs: plugin {n} ({}) does not support required surface `{REQUIRED_SURFACE}`",
            plugin_info.name
        ));
    }
    let command = plugin_info.path.to_string_lossy().into_owned();
    let (w, h) = CANVAS;
    match crate::bridge::PluginSink::open(&gfx.gpu, &command, w, h, gfx.picture_format) {
        Ok(sink) => {
            let server_name = sink.server_name().to_string();
            gfx.plugin = Some(sink);
            Some(format!(
                "outputs: plugin {n} ({}) is on — streaming {w} x {h} via {server_name}",
                plugin_info.name
            ))
        }
        Err(e) => Some(format!("outputs: plugin {n} failed to open: {e}")),
    }
}

/// Dispatches UI focus responses, handling in-process operations and refusals (P-0083).
///
/// Returns whether the action triggered state changes, delegating scope navigation and loads to the caller.
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
        // Pointer navigation updates focus state without modifying arrangement.
        focus::Asked::Moved => return Change::Pointed(true).repaint(),
        focus::Asked::Emitted(operation) => Acted::Emitted(Some(operation.clone())),
        // Perform arrangement actions (such as bay folding or Program solo) and route through `Readout::op`.
        focus::Asked::Panel(op) => {
            let outcome = readout.op(*op);
            return Change::Operated(&outcome).repaint();
        }
        // Toggle sink routing via Readout::sink.
        focus::Asked::Routed(asked, op) => {
            let outcome = readout.sink(asked.clone(), *op);
            return Change::Operated(&outcome).repaint();
        }
        // Clamp and record stepped level adjustments for gain, opacity, master out, exposure, and offset (ADR-0156, ADR-0333).
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
        // Enumerate audio input devices upon opening the audio-in card (ADR-0156, ADR-0350).
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

/// Loads a selected Set into a deck slot via [`loading`], reporting status sentences on failure ([`Engine::aimed`]).
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
            // Update slot material ID on aim before build verification; staging reports discrepancies.
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

/// Loads an individual procedure into a slot's layer stack without modifying other layers (ADR-0338).
///
/// Re-points the slot watcher at the target `.kir` file while maintaining the underlying base set aim.
pub(crate) fn overlaid(gfx: &mut Gfx, operation: &Operation) -> Option<String> {
    let Operation::LoadProcedure { deck, procedure } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = gfx.engine.aimed.len();
    // Extract base material before mutable borrow of aim.
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

/// Restores a snapshot from the Library history scope onto a slot node's working copy without reinstalling (ADR-0089, ADR-0228, ADR-0308, P-0083).
pub(crate) fn restored(gfx: &Gfx, operation: &Operation) -> Option<String> {
    let Operation::RestoreProcedure { deck, revision } = operation else {
        return None;
    };
    // Format requested revision name for refusal reporting.
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
    // Resolve snapshot against current slot pointing layout (ADR-0304).
    Some(put_back(
        &gfx.store,
        slot,
        letter,
        id,
        revision,
        &gfx.engine.pointing,
    ))
}

/// Returns the Set ID currently running on the load pulldown's target deck (ADR-0304, ADR-0305).
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

/// Executes an emitted operation across console, audio, and engine subsystems,
/// persisting written records and returning any resulting `Written` conversion.
pub(crate) fn perform_emitted_operation(
    gfx: &mut Gfx,
    started: Instant,
    readout: &mut Readout,
    recorder: Option<&mut karakuri_environment::session::Recorder>,
    operation: &Operation,
) -> Option<Option<Written>> {
    // 1. Tracker tap/nudge operations
    if let Some(line) = tracked(gfx, started, operation) {
        println!("{line}");
        return Some(None);
    }

    // 2. Arrangement operations reaching store/disk
    if let Some(line) = arrangement(
        &gfx.store,
        &mut readout.panel,
        &mut readout.view.arrangement,
        operation,
    ) {
        println!("{line}");
    }

    // 3. Audio/tempo local actions
    let session_bpm = gfx.engine.deck.signals().oscillator().bpm();
    if let Some(line) = attached(
        &mut gfx.audio,
        session_bpm,
        &mut readout.view.audio,
        operation,
    ) {
        println!("{line}");
    }
    if let Some(line) = nudged(&mut gfx.audio, operation) {
        println!("{line}");
    }
    if let Some(line) = retargeted(&mut gfx.audio, operation) {
        println!("{line}");
    }

    // 4. View and pane pointers
    if let Some(line) = pointed(&mut readout.view, operation) {
        println!("{line}");
    }
    if let Some(line) = pointed_pane(&mut readout.view, operation) {
        println!("{line}");
        let targets = readout.view.pane_decks();
        inspector(
            &gfx.engine.deck,
            &gfx.material,
            &gfx.engine.aimed,
            targets,
            &mut readout.view.inspector,
        );
    }

    // 5. Deck and slot playback/overlay/aim changes
    if let Some(line) = played(gfx, operation) {
        println!("{line}");
    }
    if let Some(line) = overlaid(gfx, operation) {
        println!("{line}");
    }
    if let Some(line) = composited(&mut gfx.engine.aimed, operation) {
        println!("{line}");
    }
    if let Some(line) = resized(&mut gfx.engine.aimed, operation) {
        println!("{line}");
    }
    if let Some(line) = re_salted(&mut gfx.engine.aimed, operation) {
        println!("{line}");
    }
    if let Some(line) = attended(&mut gfx.engine.aimed, operation) {
        println!("{line}");
    }

    let slots = gfx.engine.deck.slot_count();
    let Engine {
        edges: run_edges,
        aimed,
        ..
    } = &mut gfx.engine;
    if let Some(line) = wired_input(run_edges, aimed, slots, operation) {
        println!("{line}");
    }
    if let Some(line) = restored(gfx, operation) {
        println!("{line}");
    }
    if let Some(line) = kept(&mut readout.view, operation) {
        println!("{line}");
    }
    if let Some(line) = scheduled(&mut readout.view, operation) {
        println!("{line}");
    }
    if let Some(line) = sequenced(
        &mut readout.sequencer,
        &mut readout.playhead,
        &readout.view,
        operation,
    ) {
        println!("{line}");
    }
    if let Operation::ClearSolo = operation {
        gfx.engine.deck.clear_solo();
        println!("  solo: ClearSolo -> cleared all solo");
    }

    // 6. Record conversion & application
    let settings = readout.view.transition();
    let written = written(
        operation,
        &reading(
            operation,
            &gfx.engine.deck,
            &gfx.engine.look,
            &gfx.engine.chain,
            settings,
            &readout.sequencer,
        ),
    );

    if let Some(line) = unwritten(operation, &written) {
        println!("{line}");
    }

    if let Written::Records(records) = &written {
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

    Some(Some(written))
}
