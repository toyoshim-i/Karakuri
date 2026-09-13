//! ApplicationHandler event loop implementation for App.

use std::sync::Arc;
use std::time::{Duration, Instant};

use karakuri_console::focus;
use karakuri_console::input::Claim;
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::view::{self, Scope, Sequenced};
use karakuri_console::{egui, egui_wgpu, egui_winit};
use karakuri_engine::{compose, Committed, Gpu, Sink, Skip};
use karakuri_environment::{midi, Asked};
use karakuri_layout::Point;
use karakuri_mcp as mcp;
use karakuri_operation::{Operation, Output, SetTransfer, Undecided};
use karakuri_store::record::Record;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::engine_bridge::*;
use crate::gfx::*;
use crate::keymap::{KeyAction, KeyCtx, KEY_BINDINGS};
use crate::readout::*;
use crate::{CANVAS, MAPPED, SERVED, WINDOW};

use super::*;

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

    /// Where a deadline comes due, which is the start of every iteration the loop
    /// makes — including the one a `ControlFlow::WaitUntil` woke it for.
    ///
    /// Both deadlines are checked whatever the [`StartCause`] rather than only on
    /// `ResumeTimeReached`: a wait that is cancelled early by a real event still
    /// has to leave a due deadline serviced, and checking two `Instant`s costs
    /// nothing.
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

    /// The one place the control flow is set, and it is a deadline or nothing.
    ///
    /// `Wait` is a window that costs the machine nothing at all until somebody
    /// touches it, which is ADR-0164's still-panel clause as the operating system
    /// sees it. `WaitUntil` is the soonest of the three things that are owed at a
    /// time rather than on an event: the frame `egui` asked for after a delay, the
    /// reading `Costs` takes once the window has been still long enough, and — on a
    /// run with `--mcp` — the wake that takes what a model asked for ([`SERVED`]).
    /// None is `Poll`, and nothing here asks for a frame in order to have something
    /// to measure.
    ///
    /// The third one is the only one that can be owed forever, and that is what a
    /// served run is: something outside this process is driving the instrument, so
    /// the window is being touched even though nobody is at it. A thread that is
    /// not this one said there is something to drain, and there is exactly one of
    /// them: the MIDI callback — see [`App::waker`].
    ///
    /// It asks for a frame and does nothing else. The drain itself is
    /// [`App::mapped`], at the top of `RedrawRequested` beside the other two, which
    /// is what makes a sweep spanning several wakes one operation on one frame
    /// instead of a partial apply per message. The wake carries no payload for the
    /// same reason: what arrived is the port's to say and this loop's only job is
    /// to run again.
    ///
    /// `request_redraw` and not a repaint decision, because there is nothing yet to
    /// decide about — whether the frame changes anything is what `performed`
    /// answers on the frame this asks for, and `costs.owes` is what says the frame
    /// was owed to an event rather than to a still panel.
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
