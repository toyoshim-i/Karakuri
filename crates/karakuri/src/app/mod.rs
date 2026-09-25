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
    /// What the command line asked for, read before the event loop starts and used
    /// once, in `resumed`. It is here rather than in [`Gfx`] because it is known
    /// before there is a device and outlives every remake of one.
    ///
    /// The pair the operator named, and not what any deck runs from — see
    /// [`running`](App::running). What this answers is the strips' name, which is a
    /// question about what was asked for: four decks opened on one preset are
    /// playing that preset, whatever their four files are called.
    pub(crate) sources: Sources,
    /// Active scratch source copies polled by watchers and edited during the session.
    pub(crate) running: Vec<Sources>,
    /// Where the library is — see [`Launch::store`]. Here for `sources`' reason,
    /// and copied onto [`Gfx::store`] for the readers that are handed only a
    /// device.
    pub(crate) store: std::path::PathBuf,
    /// The preset library this run resolved, kept for one purpose: the legend says
    /// which of the places answered, and it says it by printing what the resolution
    /// returned rather than a sentence about what it probably did. `None` is a
    /// machine with no library, which reaches this far only on a run that was given
    /// its pair by hand.
    pub(crate) presets: Option<karakuri_environment::places::Presets>,
    /// The plugin directory resolved for this run, if available.
    pub(crate) plugins: Option<karakuri_environment::places::Plugins>,
    /// Discovered out-of-process output plugins.
    pub(crate) discovered_plugins: Vec<karakuri_environment::output_plugin::DiscoveredPlugin>,
    /// The directory the Library bay is pointed at, or `None` until a folder has
    /// been dropped on this window — which is where every run starts, because
    /// nothing names one before the run (ADR-0275).
    ///
    /// Beside [`presets`](App::presets) because it is the same kind of thing: a
    /// directory outside this store that a scope of the bay lists. The difference
    /// is when it is decided — a presets root is resolved before the window opens
    /// and this arrives during the run — and that is why one is on [`Launch`] and
    /// this is not.
    ///
    /// The path is here and the *spelling* is in the console
    /// (`view::View::folder`): this side reads the directory and the panel draws
    /// the line, which is the seam every other library value crosses (ADR-0156).
    /// Two fields for one fact, and they are written in one place —
    /// [`folder_dropped`].
    pub(crate) folder: Option<std::path::PathBuf>,
    /// A validation fault is said once rather than sixty times a second.
    pub(crate) faulted: bool,
    /// Whether shift is held, which is the whole of what this loop keeps of the
    /// modifier state and is here for exactly one key.
    ///
    /// `winit`'s `KeyEvent` carries no modifiers, so a key handler that wants to
    /// tell `Tab` from `shift-Tab` has to have been listening to
    /// `WindowEvent::ModifiersChanged` — which is why this is a field rather than a
    /// question asked at the press.
    ///
    /// It is not a mode and it gives no key a second meaning, which is the
    /// distinction
    /// [ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
    /// draws when it rejects modifier chords: *"a modifier is a mode with no
    /// readout"*, and rule 04 is *nothing is hidden quietly*. `shift-Tab` survives
    /// that rejection because it reverses a traversal that is drawn either way —
    /// the ring is on a bay whichever direction you arrived from — so there is no
    /// hidden state to read out.
    ///
    /// `egui`'s own copy is not readable in time. `egui_winit::State` keeps
    /// modifiers privately and stamps them onto the events it queues;
    /// `Context::input` answers from the last pass, so a `shift` pressed since the
    /// previous frame would be invisible there. One reader, one writer, one event.
    pub(crate) shift: bool,
    pub(crate) keymap: crate::keymap::Keymap,
    pub(crate) readout: Readout,
    /// The console's hover layer, which is `karakuri-console`'s and is held here
    /// for the reason every other piece of console state is: this file owns the
    /// window, the pointer and the clock, and that crate owns none of the three.
    ///
    /// It is not part of [`Readout`] because nothing routes into it: a tooltip is
    /// not an operation, no key reaches it, and MIDI and MCP have nothing to say to
    /// it. What it takes is a pointer move, the claim `input::claim` already
    /// answered for that move, and a time — and what it gives back is a repaint
    /// decision and a box painted after the panel.
    pub(crate) hover: karakuri_console::hover::Hover,
    pub(crate) costs: Costs,
    /// Logical size, so the numbers printed are the arrangement's own units rather
    /// than the display's.
    pub(crate) scale: f64,
    /// When `egui` asked to be drawn again, kept as the deadline it is.
    ///
    /// `egui` animates, blinks a text cursor and fades a tooltip in, and it says so
    /// as a `repaint_delay` on the frame's `ViewportOutput`. Turning that into an
    /// immediate `request_redraw` would turn a 250 ms animation into a spin at
    /// whatever rate this loop can manage — which is the cost ADR-0164's
    /// still-panel clause is about, arrived at from the one direction that looks
    /// like obeying it. So the delay is added to the clock here and `about_to_wait`
    /// sleeps until it.
    ///
    /// `None` where `egui` asked for nothing, which is every frame on a panel with
    /// nothing on it.
    pub(crate) egui_due: Option<Instant>,
    /// The timestamp of the last composed frame, used to prevent duplicate frame
    /// rendering when both the console window and the projector window receive
    /// `RedrawRequested` within the same vsync interval.
    pub(crate) frame_drawn_at: Option<Instant>,
    /// The origin every animation on the panel is measured from, and the only clock
    /// behind `view::Phase`.
    ///
    /// It is here because this file owns the window and the clock and `src/` owns
    /// neither — every `Instant::now` in this crate is in this file, and
    /// `view::Phase` is a `Duration` for exactly that reason (P-0092). What crosses
    /// the seam is `now - this`, which is a number.
    ///
    /// Where the origin is does not matter, which is why it is taken at
    /// construction rather than when something first starts moving: every
    /// presentation is periodic in the phase, so an origin the operator did not
    /// choose is an origin nobody can see. What would matter is having *two*, and
    /// there is one.
    pub(crate) started: Instant,
    /// When a served run next wakes to take what a model asked for, and `None` for
    /// a run without `--mcp` — see [`SERVED`], which is the whole of why this
    /// deadline exists.
    ///
    /// A deadline beside `egui_due` rather than a `ControlFlow::Poll`, because this
    /// loop has one rule about when it runs and a second one would be a second
    /// answer to it: [`App::about_to_wait`] takes the soonest of what is owed, and
    /// this is one of the things owed.
    pub(crate) served: Option<Instant>,
    /// How a thread that is not this one gets this loop to run again, and it exists
    /// for exactly one of them: the MIDI callback.
    ///
    /// Every other thing this window answers arrives as a `winit` event or on a
    /// deadline [`App::about_to_wait`] already sets. A control surface is neither —
    /// a hand on a knob is an event nothing in `winit` can see — and this loop
    /// sleeps in `Wait` between frames (ADR-0164), so without a wake a fader would
    /// be applied at whatever the operator's next mouse move happened to be.
    ///
    /// A poll was the alternative and it is the one this rejects. A third deadline
    /// beside `egui_due` and `served` would have to run at a hand's rate to feel
    /// like a fader — a hundred and twenty-five wakes a second, for the whole of a
    /// run, whether or not anything is plugged in — which is `ControlFlow::Poll`
    /// with extra steps and is the exact cost the still-panel clause is about.
    /// [`SERVED`]'s tenth of a second is the other end of that trade and is a fader
    /// at 10 Hz.
    ///
    /// It carries nothing. The proxy's event type is `()`: the wake says *ask
    /// again*, [`App::user_event`] asks for a frame, and the drain happens where
    /// every other drain happens. Handed to [`midi::Surface::first`] as a closure,
    /// so no crate below this one learns that a window exists.
    pub(crate) waker: EventLoopProxy<()>,
    /// The sending half of [`watch::Watch::storing_to`]'s channel, handed to every
    /// watcher [`Engine::new`] makes. Kept because a window remade makes them
    /// again.
    pub(crate) built_tx: std::sync::mpsc::Sender<watch::Built>,
    /// Shared store instance used by watcher build workers.
    pub(crate) held: std::sync::Arc<Store>,
    /// Every version that compiles in this run, seeded in [`main`] from the files
    /// the decks were about to play and handed to every watcher [`Engine::new`]
    /// makes.
    ///
    /// One for the run, which is [`karakuri_environment::history`]'s own
    /// requirement rather than a convenience: the seed and the watchers share the
    /// dedup, and seeded separately the first rebuild would write the untouched
    /// procedure a second time. That is also why it is here rather than on [`Gfx`]
    /// — a window remade rebuilds the deck and would rebuild the history with it.
    ///
    /// What each version is filed under is not here. That is the Set the slot is
    /// running, and it lives on the aim, one per slot, moved by a load — see
    /// [`Aiming`] and [`watch::Aim::set`].
    pub(crate) snapshots: history::Shared,
    /// The run's one published layout, made in [`main`], handed to
    /// [`karakuri_mcp::serve`] there and to every [`Engine`] this opens — see
    /// [`karakuri_mcp::Slots`] and [`Aiming::pointing`].
    ///
    /// Here rather than on [`Gfx`], for [`App::snapshots`]' reason exactly: the
    /// server is bound before the window and outlives every window this run
    /// remakes, so a handle rebuilt with the swapchain would leave the server
    /// reading one nothing writes.
    pub(crate) pointing: karakuri_mcp::Slots,
    /// Everything a save and a rewiring need that is not the deck — see
    /// [`Keeping`].
    pub(crate) keeping: Keeping,
    /// The session recorder, and the two presses that move it — see [`Sessions`].
    ///
    /// It is beside [`App::keeping`] rather than inside it because the two hold
    /// different things for different moments: that one is what a *save* and a
    /// rewiring need, and this is a writer thread the frame path pushes into. What
    /// they share is the arrangement — a press gathers, a thread works, and the
    /// outcome is said at the frame it arrives.
    pub(crate) recording: Sessions,
    /// How far a frame advances the session, derived from the interval it measures
    /// — [`karakuri_environment::clock::Clock`], which is the same derivation
    /// `karakuri-cli` makes because there is one of them.
    ///
    /// This is P-0092's live half: *"live, the engine derives the step count from
    /// real time and writes it in"*. The number is read once per composed frame,
    /// handed to [`measure_audio`] as this frame's advance, committed as
    /// [`Committed::steps`], and pushed as the `tick` that closes the frame where a
    /// recording is running — one measurement reaching four places rather than four
    /// answers to how long a frame was.
    ///
    /// It is on [`App`] rather than on [`Gfx`], which is [`Sessions`]' reason read
    /// one field along: a window remade is a display remade, and a session's clock
    /// is not the surface's. A clock rebuilt with the swapchain would restart the
    /// count in the middle of a stream that is still being written.
    ///
    /// The run's first frame is capped, and that is the cap doing its job rather
    /// than an accident to correct: this is started before the window exists, so
    /// everything between here and the first composed frame — the device, the
    /// compile, the first Sets — is one gap, and a gap is counted whole up to
    /// `MAX_STEPS` (ADR-0006).
    pub(crate) clock: Clock,
    /// Last seen mixer state revision to detect when mixer bay needs dirtying.
    pub(crate) last_mixer_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventLoopAction {
    Exit,
    Continue,
}

/// What one call of [`App::performed`] came to: the frame it is owed, and —
/// where an emitted operation reached the conversion — what the conversion
/// answered.
///
/// Every caller but one reads `repaint` and drops the rest, because a press
/// and a mapped control are already at the surface that printed the outcome.
/// The one that reads `written` is [`App::operated`], which owes a model a
/// sentence and has no terminal to say it on (ADR-0315).
///
/// It is carried back rather than asked for again. `written` is a pure
/// function and a second call would be free, but it would not be the same
/// answer: the readings are taken *after* this frame's [`scheduled`] and
/// [`sequenced`] have run, so a conversion done before the drain would be
/// converted against lanes the same drain has since muted, and a move an
/// operator was told is now free would be refused — or the reverse (ADR-0323).
/// One conversion, at the one place the readings are true.
///
/// `None` for every [`Acted`] that is not an emission, for the empty emission,
/// and for the two operations that leave the emission arm at [`tracked`]
/// before the conversion runs: *no outcome to report* rather than *an outcome
/// that was nothing*, which is the distinction [`unperformed`] answers `None`
/// on.
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
    /// The opening is handed in rather than made here, which is the whole of what
    /// pairing the four pills with a server took: [`main`] gives the same handle to
    /// [`karakuri_mcp::serve`] and to this, so a press on a bay head and the class
    /// the server reads are one value. [`Readout::new`] makes one of its own — it
    /// is constructed from a size and nothing else — and this replaces it before
    /// the window opens, which is before anything can read either.
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

    /// Hand an event to `egui`, and nowhere else.
    ///
    /// Every call site has already asked `claim` where a pointer event belongs;
    /// this is the other branch. `EventResponse::repaint` is `egui`'s own answer
    /// for the event it was just given, and it is the reason
    /// `Change::Pointer(Claim::Egui)` asks for nothing: one answer per event, from
    /// whoever got it.
    ///
    /// `repaint` is taken out of the `EventResponse` on this line and nothing else
    /// survives it (ADR-0259). `egui-winit` 0.36.1 hard-codes
    /// `EventResponse::consumed` `true` for every `Tab` — *"When pressing the Tab
    /// key, egui focuses the first focusable element, hence Tab always consumes"* —
    /// whether or not anything in this program's `egui::Context` has focus, so
    /// honouring it here would swallow the key that moves focus between bays on its
    /// first press, with no panic and no diagnostic. This program is told about a
    /// window event and never asks `egui` for permission, so the destructure below
    /// is the whole of the fix: past this line there is no `EventResponse` left in
    /// scope for a future `if … .consumed` to be added to by mistake, only the one
    /// `bool` this function was always allowed to read.
    ///
    /// This used to be a text scan. `event_response`'s former `#[test]`,
    /// `the_only_field_read_off_an_event_response_is_repaint`, read this file for
    /// the word `consumed` and for every `response.` field access above the tests.
    /// Both questions are unnecessary now rather than merely unlikely to trip:
    /// `on_window_event` is called nowhere else in this program, and the only value
    /// this call site keeps a name for is a `bool` with no `EventResponse` behind
    /// it to add a second field read to.
    ///
    /// ADR-0259's second condition is still checked by nothing, as it was before
    /// this line existed. The claim above holds *because* the console focuses no
    /// `egui` widget — no `Button`, `TextEdit`, `Slider`, `DragValue`, `.interact(`
    /// or `.sense(` anywhere in `karakuri-console/src`, so `Memory::focused()` is
    /// permanently `None` and `consumed` would be reading a flag that means nothing
    /// on this program's own widgets even where it is read. That was true on
    /// 2026-09-05 and has not been re-checked since; it is written here rather than
    /// left to look covered by a module that no longer exists.
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
    ) -> Performed {
        let prev_mixer_revision = gfx.engine.deck.mixer_revision();
        // **What the conversion answered, filled by the one arm that runs
        // it**, and carried out of this function because the drain that has no
        // terminal has to say it over a socket — see [`Performed`], where why
        // it is carried rather than converted a second time is written down.
        let mut converted = None;
        let repaint = match acted {
            Acted::Nothing => otherwise,
            // **A class pill earns the frame the claimed press already earns,
            // and no more.** Nothing in the arrangement moved and no operation
            // was emitted; what changed is one word in one capsule, and
            // `Change::Pointer(Claim::Panel)` — which is what `otherwise` is on
            // every path that can reach this arm — is already `Repaint::Now`. A
            // `Change` of its own would be a second answer to a question that
            // is already answered.
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

    /// Every operation a model has asked for since the last frame, performed
    /// where a press of the same operation is performed.
    ///
    /// It is [`Keeping::requests`]'s neighbour and not part of it, because
    /// what a save and an edge need is the engine and what an operation needs is
    /// everything a press needs: the window, the readout, the session being
    /// recorded. So the drain is here, beside [`App::performed`], and the two
    /// are called one after the other at the two places this loop takes what a
    /// model asked for.
    ///
    /// Nothing decides anything here. The operation arrived already audited
    /// — `karakuri_operation::gate` ran on the server's own thread, which is the
    /// one call ADR-0235 puts the whole mechanism on — and it is handed to
    /// [`App::performed`] as an `Acted::Emitted`, which is the value a fader
    /// hands it. A model's `SetGain` and a hand on the strip are the same press
    /// from here on
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// which is the whole of what routing into one vocabulary was for.
    ///
    /// Collected out of the borrow before any of it is acted on, exactly as
    /// [`Keeping::requests`] collects, and for the same reason: the loop below
    /// takes `&mut` of things the reporter is reached through. An empty
    /// `collect` allocates nothing, which is every frame of a run nobody is
    /// driving.
    ///
    /// Answered once, at the frame it was performed on. What a *rebuild*
    /// the operation started makes of it lands thirty judged frames later and is
    /// `swap_outcome`'s answer, and what a *scheduled* move comes to is the
    /// deck's own reading — so the sentence says where each of those is rather
    /// than holding a connection open across a transition. That is
    /// `mcp::WireRequest`'s third point, one route along.
    ///
    /// # Answered, and not answered *performed* regardless
    ///
    /// The answer says what happened and not that something did. An operation
    /// the conversion refused or owed goes back as an `Err` in
    /// [`unperformed`]'s sentence — the crate that took the decision words it,
    /// and `karakuri-cli` answers the same string over its own `--mcp`, so one
    /// mistake gets one explanation whichever program a model came through
    /// (ADR-0131, P-0083). A fade onto a fader an unmuted lane of the armed
    /// pattern holds is the case that was reported as performed: nothing was
    /// scheduled, nothing moved, the refusal named the lane to mute on this
    /// run's terminal, and the socket carried *was performed* (ADR-0323).
    ///
    /// A model has neither this window nor this terminal
    /// ([ADR-0315](../../../docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)),
    /// so the reply is the whole of what it is told. That is why the outcome is
    /// carried back out of [`App::performed`] rather than converted again here
    /// — see [`Performed`], where the readings that make a second conversion a
    /// different answer are written down.
    ///
    /// A `Written::Silent` is still answered *performed*, which is
    /// [`unperformed`]'s own paragraph and the one place this program's answer
    /// differs from `karakuri-cli`'s: a `load_set`, a `select_deck` and a
    /// `route_frame` write no record and are all performed here.
    ///
    /// # The three whose performer is not in [`App::performed`]
    ///
    /// Most operations end in `performed` and this function names none of
    /// them. Three do not, and each is taken here in the order and by the
    /// call the pointer's button-up arm takes it in — a second route into one
    /// of them would be the second answer
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// exists to prevent, so this calls the same functions rather than
    /// repeating what they do
    /// ([ADR-0341](../../../docs/adr/0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md)).
    ///
    /// - A star, refused — [`favourite`] with [`Asked::Model`], which is
    ///   ADR-0301's decision reached by the route that record said it was owed.
    ///   `my sets` is the list of Sets the operator chose, so the refusal is
    ///   the answer and it goes back as one: `Err`, which reaches the client as
    ///   a failed call, carrying the id and where the Set actually is. A
    ///   success reported for an act that had no effect is what that record
    ///   refused, and answering `ok` here would be it.
    /// - A projector, opened — [`routed`], which needs an `ActiveEventLoop`
    ///   and is why this function takes one. The drain is already on the
    ///   event loop's thread: both call sites are `winit` handlers holding
    ///   the loop, so the argument was there to be passed and the row was
    ///   `plan` for want of one parameter.
    /// - A recording, started or stopped — [`Sessions::asked`], which needs
    ///   the store as well and is why this takes that too. Both ends are
    ///   gathered here and written off the render thread, exactly as the `rec`
    ///   pill's press does.
    ///
    /// They fall through to `performed` afterwards, as a press does, so a
    /// row that also writes a record writes it once and in one place.
    // **Nine, where clippy's line is seven**, and the two past it are the two
    // the three arms above need: the event loop a window is made on, and the
    // store a recording's head is written into. [`App::mapped`] carries the
    // same allowance for the same reason — a struct here would be `App` itself
    // with the fields that need no window left out.
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
            // **What the conversion refused or owed is the answer, where there
            // is one**, and it leaves as an `Err` for the reason the star above
            // does: a refusal reported as a success is a model told the fade it
            // asked for is running. It is the sentence `karakuri-cli` answers
            // over its own `--mcp`, out of the crate that took the decision
            // (ADR-0131, P-0083) — the terminal has had it from [`unwritten`]
            // for as long as there has been a window, and the socket had
            // nothing.
            if let Some(refused) = written
                .as_ref()
                .and_then(|written| unperformed(title, written))
            {
                reply.settled(Err(refused));
                continue;
            }
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

    /// Every operation a hand on a control surface asked for since the last frame,
    /// performed where a press of the same operation is performed.
    ///
    /// It is [`App::operated`]'s neighbour, and the two are one shape: a door
    /// outside this window hands in an [`Operation`], and it is given to
    /// [`App::performed`] as an `Acted::Emitted` — the value a fader hands it. A
    /// knob's `SetGain`, a model's and a hand on the strip are the same press from
    /// here on
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// which is what routing every surface into one vocabulary was for, and it is
    /// why a session recorded from this surface replays with neither the surface
    /// nor the map attached
    /// ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)):
    /// what reaches the stream is the record, and no record names a knob.
    ///
    /// Nothing decides anything here, which is the same sentence [`App::operated`]
    /// carries and is true for a different reason. A model's request was audited on
    /// the server's thread; a hand needs no audit at all —
    /// `karakuri_operation::gate` is a model's boundary and not an operator's, and
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// says a hand cancels whatever automatic thing was writing the control, on
    /// every route in. A gate over the operator's own surface would be the
    /// instrument refusing its player.
    ///
    /// The slot check is the router's and is said once per slot per run.
    /// `Surface::take` drops a message naming a slot this deck does not hold,
    /// because `Deck::gain` and its neighbours index directly — see
    /// `karakuri_environment::midi`, where that whole argument lives.
    ///
    /// The buffer is [`Gfx::performed_by_hand`] and is cleared by the drain, so
    /// nothing here allocates on a frame nobody touched the surface — which is
    /// every frame of a run with no surface at all, and costs one branch.
    ///
    /// Drained inside the frame rather than on the wake, which is the difference
    /// between this and a key press: a key arrives as a `winit` event and is
    /// performed on it, and a MIDI message arrives on the MIDI thread.
    /// [`App::user_event`] asks for a frame and this is what that frame does about
    /// it, so a sweep spanning two wakes is one operation on one frame rather than
    /// two half-applied ones.
    // **Eight, where clippy's line is seven, and every one is a thing a press
    // needs**: the window, the clock, the readout, the stream being recorded,
    // the layer that knows what the pointer is on, where a learned map is
    // written, and the two halves of a repaint decision. A struct would be
    // `App` itself with the fields that need no window left out, which is
    // `App::new`'s sentence one function along.
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

    /// The surface is shown where the deck is — MIDI out, the other direction of
    /// [`App::mapped`] and the send beside its drain.
    ///
    /// After the frame's operations have been applied, so a motorised fader follows
    /// the value the deck holds rather than the one it was asked for — and after a
    /// learn too, because a knob just bound has never been shown and the control it
    /// took over may have been lit on another knob.
    ///
    /// Every source is shown and not just this surface's own, which is the whole
    /// reason MIDI out is worth having: a key, a model over `--mcp`, the pointer on
    /// a strip and a transition all move a fader, and *two things can move a fader*
    /// is the sentence `docs/roadmap.md` gives this row.
    ///
    /// It does not wait (P-0094): `Surface::show` queues into a bounded channel and
    /// drops when it is full rather than blocking the render thread —
    /// `karakuri_environment::midi`, where that whole argument lives. And it writes
    /// no record: what changes is the wire, so a session recorded from this surface
    /// still replays with neither surface nor map attached (P-0092).
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

    /// Act on a repaint decision, and the only place a frame is asked for outside
    /// `to_egui` and `missed`.
    ///
    /// Three answers and three actions: ask for a frame, note a deadline, or do
    /// nothing at all — and the third is the one ADR-0164's still-panel clause is
    /// made of.
    ///
    /// It takes the two fields rather than `&mut self` so that a caller holding
    /// `self.gfx` can still reach `self.egui_due`.
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
