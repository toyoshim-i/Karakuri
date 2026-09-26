//! Application state and event loop handling.

use std::time::Instant;

use karakuri_console::egui_winit;
use karakuri_console::repaint::{Change, Repaint};
use karakuri_environment::clock::Clock;
use karakuri_environment::{history, midi, watch, Asked, Opening, SlotPolicies};
use karakuri_mcp as mcp;
use karakuri_operation::Operation;
use karakuri_operation_record::{written, Written};
use karakuri_store::record::Record;
use karakuri_store::store::Store;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::engine_bridge::*;
use crate::gfx::*;
use crate::launch::*;
use crate::readout::*;
use crate::session::{Keeping, Sessions};
use crate::WINDOW;
use karakuri_engine::DeckSlot as EngineSlot;
mod audio_midi;
mod handler;
mod operations;

pub(crate) use audio_midi::*;
pub(crate) use operations::*;

pub(crate) struct App {
    pub(crate) gfx: Option<Gfx>,
    /// Initial material pair requested on the command line, used to seed decks in `resumed`.
    pub(crate) sources: Sources,
    /// Active scratch source copies polled by watchers and edited during the session.
    pub(crate) running: Vec<Sources>,
    /// Where the library is — see [`Launch::store`]. Here for `sources`' reason,
    /// and copied onto [`Gfx::store`] for the readers that are handed only a
    /// device.
    pub(crate) store: std::path::PathBuf,
    /// Preset library resolved at startup, reported in UI legend readouts.
    pub(crate) presets: Option<karakuri_environment::places::Presets>,
    /// The plugin directory resolved for this run, if available.
    pub(crate) plugins: Option<karakuri_environment::places::Plugins>,
    /// Discovered out-of-process output plugins.
    pub(crate) discovered_plugins: Vec<karakuri_environment::output_plugin::DiscoveredPlugin>,
    /// Directory loaded via drag-and-drop for the Library bay, or `None` initially (ADR-0156, ADR-0275).
    pub(crate) folder: Option<std::path::PathBuf>,
    /// A validation fault is said once rather than sixty times a second.
    pub(crate) faulted: bool,
    /// Tracks Shift modifier state from `WindowEvent::ModifiersChanged` for reverse Tab navigation (ADR-0259).
    pub(crate) shift: bool,
    pub(crate) keymap: crate::keymap::Keymap,
    pub(crate) readout: Readout,
    /// UI hover layer managing tooltip dwell timing and rendering over console panels.
    pub(crate) hover: karakuri_console::hover::Hover,
    pub(crate) costs: Costs,
    /// Logical size, so the numbers printed are the arrangement's own units rather
    /// than the display's.
    pub(crate) scale: f64,
    /// Deadline for the next egui repaint based on viewport `repaint_delay` (ADR-0164).
    pub(crate) egui_due: Option<Instant>,
    /// The timestamp of the last composed frame, used to prevent duplicate frame
    /// rendering when both the console window and the projector window receive
    /// `RedrawRequested` within the same vsync interval.
    pub(crate) frame_drawn_at: Option<Instant>,
    /// Reference instant for UI animations, converted to elapsed duration for `view::Phase` (P-0092).
    pub(crate) started: Instant,
    /// Periodic wake deadline for handling external MCP requests when `--mcp` is active ([`SERVED`]).
    pub(crate) served: Option<Instant>,
    /// Event loop waker proxy invoked by the MIDI callback thread to signal incoming inputs (ADR-0164).
    pub(crate) waker: EventLoopProxy<()>,
    /// The sending half of [`watch::Watch::storing_to`]'s channel, handed to every
    /// watcher [`Engine::new`] makes. Kept because a window remade makes them
    /// again.
    pub(crate) built_tx: std::sync::mpsc::Sender<watch::Built>,
    /// Shared store instance used by watcher build workers.
    pub(crate) held: std::sync::Arc<Store>,
    /// Snapshot history across engine window rebuilds, seeded from launch files ([`Aiming`], [`watch::Aim`]).
    pub(crate) snapshots: history::Shared,
    /// Shared MCP layout slots handle preserved across window recreations ([`karakuri_mcp::Slots`]).
    pub(crate) pointing: karakuri_mcp::Slots,
    /// Everything a save and a rewiring need that is not the deck — see
    /// [`Keeping`].
    pub(crate) keeping: Keeping,
    /// Session recorder and background writer thread handle ([`Sessions`]).
    pub(crate) recording: Sessions,
    /// Clock measuring real-time frame intervals to advance session step count (ADR-0006, P-0092).
    pub(crate) clock: Clock,
    /// Last seen mixer state revision to detect when mixer bay needs dirtying.
    pub(crate) last_mixer_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventLoopAction {
    Exit,
    Continue,
}

/// Result of an [`App::performed`] invocation, detailing requested repaint urgency and optional conversion outcome.
pub(crate) struct Performed {
    /// The frame this performance is owed.
    pub(crate) repaint: Repaint,
    /// What [`written`] made of the operation, where one reached it.
    pub(crate) written: Option<Written>,
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

    /// Reaction to a window event regarding event loop termination for this app
    /// instance.
    pub(crate) fn event_loop_action(&self, id: WindowId, event: &WindowEvent) -> EventLoopAction {
        let is_main_window = self.gfx.as_ref().is_none_or(|g| g.window.id() == id);
        Self::event_loop_action_for(is_main_window, event)
    }

    /// Updates the shift modifier state from a `ModifiersChanged` event.
    pub(crate) fn update_modifiers(&mut self, state: &winit::event::Modifiers) {
        self.shift = state.state().shift_key();
    }
    /// Initialize application state with pre-configured MCP handles and launch environment.
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
        slot_policies: SlotPolicies,
    ) -> App {
        let mut readout = Readout::new(WINDOW.0 as f32, WINDOW.1 as f32);
        readout.store_root = Some(launch.store.clone());
        if let Ok(store) = karakuri_store::Store::open(&launch.store) {
            if let Ok(saved_policies) = store.policies() {
                for (slot, name) in saved_policies.iter().enumerate().take(4) {
                    if let Ok(policy) = name.parse::<karakuri_operation::SlotPolicy>() {
                        slot_policies.set_policy(slot, policy);
                    }
                }
            }
        }
        // **The one handle, and it lives on the readout because that is where
        // the pills reach it.** A copy kept on [`App`] as well would be a second
        // answer to what is open the day one of them was written and the other
        // was not.
        readout.view.opening = opening.read();
        readout.opening = opening;
        readout.slot_policies = slot_policies.clone();
        readout.view.slot_policies = [
            slot_policies.policy(0),
            slot_policies.policy(1),
            slot_policies.policy(2),
            slot_policies.policy(3),
        ];
        let (built_tx, built) = std::sync::mpsc::channel();
        let (save_tx, saves) = std::sync::mpsc::channel();
        let (send_tx, sends) = std::sync::mpsc::channel();
        let (keep_tx, keeps) = std::sync::mpsc::channel();
        let keymap = crate::keymap::Keymap::load_or_default(&launch.store);
        let discovered_plugins = match &launch.plugins {
            Some(places) => {
                karakuri_environment::output_plugin::discovery::discover_plugins(places)
            }
            None => Vec::new(),
        };
        App {
            gfx: None,
            sources: launch.sources,
            running,
            store: launch.store,
            presets: launch.presets,
            plugins: launch.plugins,
            discovered_plugins,
            // **Pointed nowhere**, which is where every run starts: a folder
            // is chosen by dropping one on this window and no flag names one
            // before it opens (ADR-0275).
            folder: None,
            faulted: false,
            // **Nothing held**, which is a window nobody has pressed a key on
            // and is also what `winit` reports the moment focus leaves it.
            shift: false,
            keymap,
            readout,
            // **The manual is read here, before the window opens**, which is
            // where a 340 KB parse belongs: it is one walk of the page
            // compiled into this binary and it is on no frame path (P-0091).
            hover: karakuri_console::hover::Hover::new(),
            costs: Costs::new(),
            scale: 1.0,
            egui_due: None,
            frame_drawn_at: None,
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
                slot_policies: slot_policies.clone(),
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
            last_mixer_revision: 0,
        }
    }

    /// Forwards window events to egui context without consulting `consumed` (ADR-0259).
    pub(crate) fn to_egui(gfx: &mut Gfx, costs: &mut Costs, event: &WindowEvent) {
        let egui_winit::EventResponse { repaint, .. } =
            gfx.egui.on_window_event(&gfx.window, event);
        if repaint {
            costs.owes();
            gfx.window.request_redraw();
        }
    }

    /// Dispatches UI pointer actions to engine or UI state and determines redraw requirements
    /// (ADR-0156, ADR-0194, Principle 0090).
    pub(crate) fn performed(
        gfx: &mut Gfx,
        started: Instant,
        readout: &mut Readout,
        // Record operation to the active session stream when recording is enabled.
        recorder: Option<&mut karakuri_environment::session::Recorder>,
        acted: &Acted,
        otherwise: Repaint,
    ) -> Performed {
        let prev_mixer_revision = gfx.engine.deck.mixer_revision();
        // **What the conversion answered, filled by the one arm that runs
        // it**, and carried out of this function because the drain that has no
        // terminal has to say it over a socket — see [`Performed`], where why
        // it is carried rather than converted a second time is written down.
        let mut converted = None;
        let repaint = match acted {
            Acted::Nothing => otherwise,
            // Class pill interaction requests an immediate repaint matching panel interaction claims.
            Acted::Opened => otherwise,
            // Acted::Pointed moves the library cursor without generating an operation.
            Acted::Pointed => otherwise,
            Acted::Operated(outcome) => Change::Operated(outcome).repaint(),
            Acted::Emitted(operation) => {
                if let Some(operation) = operation.as_ref() {
                    match perform_emitted_operation(gfx, started, readout, recorder, operation) {
                        Some(None) => {
                            return Performed {
                                repaint: Change::Emitted(Some(operation)).repaint(),
                                written: None,
                            };
                        }
                        Some(Some(written)) => {
                            converted = Some(written);
                        }
                        None => {}
                    }
                }
                Change::Emitted(operation.as_ref()).repaint()
            }
        };
        if gfx.engine.deck.mixer_revision() != prev_mixer_revision {
            readout.view.mark_mixer_dirty();
            for i in 0..gfx.engine.deck.slot_count() {
                readout
                    .slot_policies
                    .set_in_mix(i, gfx.engine.deck.is_in_mix(EngineSlot(i as u8)));
            }
        }
        Performed {
            repaint,
            written: converted,
        }
    }

    /// Dispatches external model operations from MCP queue, routing non-performer commands (star, projector, record)
    /// to their handlers and reporting outcomes back to the caller (ADR-0131, ADR-0235, ADR-0301, ADR-0315, ADR-0341, P-0083, P-0090).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn operated(
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
            // Model keeps execute asynchronously and save to `<store>/sandbox/` (P-0096, ADR-0261, ADR-0301).
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
            let Performed { repaint, written } = App::performed(
                gfx,
                started,
                readout,
                recording.recorder(),
                &Acted::Emitted(Some(operation)),
                Repaint::Never,
            );
            App::wants(gfx, egui_due, costs, repaint);
            // Return refusal or error description to client as an Err outcome (ADR-0131, P-0083).
            if let Some(refused) = written
                .as_ref()
                .and_then(|written| unperformed(title, written))
            {
                reply.settled(Err(refused));
                continue;
            }
            // Report projector window creation outcome back to caller.
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

    /// Drains operations requested by control surfaces / MIDI since last frame and dispatches them via [`App::performed`] (P-0090, P-0092, P-0094).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn mapped(
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
        // In learn mode, MIDI inputs are intercepted for parameter binding rather than dispatched to the deck.
        if readout.view.learn {
            App::learned(gfx, readout, hover, maps, egui_due, costs);
            App::showed(gfx);
            return;
        }
        let Some(surface) = gfx.midi.as_mut() else {
            return;
        };
        // Temporarily swap buffer out of `self.gfx` to satisfy borrow rules during execution.
        let mut asked = std::mem::take(&mut gfx.performed_by_hand);
        // Resolve published parameter positions against active deck state for MIDI mapping (ADR-0268).
        surface.take(
            gfx.engine.deck.slot_count(),
            &midi::Decks(&gfx.engine.deck),
            &mut asked,
        );
        for operation in asked.drain(..) {
            // **The conversion's answer is dropped here and read in
            // [`App::operated`]**, and that is the difference between the two
            // drains rather than an omission: a hand is at the surface that
            // printed the outcome and a model is not (ADR-0315).
            let repaint = App::performed(
                gfx,
                started,
                readout,
                recording.recorder(),
                &Acted::Emitted(Some(operation)),
                Repaint::Never,
            )
            .repaint;
            App::wants(gfx, egui_due, costs, repaint);
        }
        gfx.performed_by_hand = asked;
        App::showed(gfx);
    }

    /// Transmit current deck parameters over MIDI output to sync motorized faders and control feedback (P-0092, P-0094).
    pub(crate) fn showed(gfx: &mut Gfx) {
        let Gfx { midi, engine, .. } = gfx;
        let Some(surface) = midi.as_mut() else {
            return;
        };
        surface.show(&midi::Lit {
            deck: &engine.deck,
            exposure: engine.look.exposure,
        });
    }

    /// Binds the UI control under the mouse pointer to the active MIDI learn message (Principle 0094).
    pub(crate) fn learned(
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

    /// Update next repaint deadline or request immediate redraw according to the repaint decision (ADR-0164).
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
