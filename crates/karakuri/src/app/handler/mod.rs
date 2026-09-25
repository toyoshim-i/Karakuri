//! ApplicationHandler event loop implementation for App.

use std::sync::Arc;
use std::time::Instant;

use karakuri_console::repaint::Change;
use karakuri_console::view::{self, Scope};
use karakuri_console::{egui, egui_wgpu, egui_winit};
use karakuri_engine::Gpu;
use karakuri_environment::midi;
use karakuri_mcp as mcp;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

use crate::{MAPPED, SERVED, WINDOW};

use super::*;

pub(crate) mod key;
pub(crate) mod pointer;
pub(crate) mod projector;
pub(crate) mod redraw;

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::ActiveEventLoopExtMacOS;
            event_loop.set_allows_automatic_window_tabbing(false);
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
        // **The picture format, read off this same surface and nowhere else.**
        // The engine's present pass writes gamma-encoded texels through the
        // hardware, so it needs an sRGB target (P-0064), and *which* sRGB
        // format exists is the display's and the backend's answer rather than
        // this program's: Metal offers `Bgra8UnormSrgb` and no 8-bit RGBA sRGB
        // format at all. `karakuri-cli` picks its present format the same way
        // — `crates/karakuri-cli/src/app.rs`, `.find(|f| f.is_srgb())` feeding
        // `Present::new`.
        let picture_format = match caps.formats.iter().copied().find(|f| f.is_srgb()) {
            Some(format) => format,
            // **Refused with the offered list**, which is
            // [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
            // there is no sRGB target to draw the picture into, and a run that
            // continued would encode twice or not at all with nothing saying
            // so.
            None => no_gpu(&format!(
                "no sRGB surface format: the present pass writes through the hardware's sRGB \
                 encode and this surface offers {:?}, none of which carries the transfer \
                 function",
                caps.formats
            )),
        };
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
            picture_format,
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
        let governed = engine.startup(&gpu);
        // **The four risk badges, from the pass that just decided them.** The
        // dot is as fresh as the last governor pass and no fresher: a Set that
        // swaps in arrives with its own estimate and a resize re-targets it
        // (ADR-0356), so the number moves without a pass and this is written
        // again wherever a later `Deck::govern` report is kept.
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
        for i in 0..engine.deck.slot_count() {
            let in_mix = engine.deck.is_in_mix(EngineSlot(i as u8));
            self.readout.slot_policies.set_in_mix(i, in_mix);
        }
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
        self.readout.view.master_chain = Some(chain_view(
            &engine.present,
            &self.readout.view.chain_add.clone(),
        ));
        // Initialize audio input tracking using current session tempo from deck oscillator.
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
            self.plugins.as_ref(),
            self.discovered_plugins.len(),
            &self.store,
            mcp_port,
        );

        let plugin_name = self.discovered_plugins.first().map(|p| {
            let s: &'static str = Box::leak(p.display_name().into_boxed_str());
            s
        });

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
            plugins: self.plugins.as_ref().map(|plugins| plugins.dir.clone()),
            discovered_plugins: self.discovered_plugins.clone(),
            plugin_name,
            material,
            store: self.store.clone(),
            window,
            // **A run opens with one output.** The projector is a window an
            // operator asks for from the Outputs row; opening one nobody asked
            // for would put a second window on their desk and raise what every
            // frame costs before the first one is drawn.
            projector: None,
            plugin: None,
            gpu,
            surface,
            config,
            picture_format,
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
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.on_window_event(event_loop, id, event);
        }));
        if let Err(payload) = res {
            let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic payload".to_string()
            };
            eprintln!("panicked in window_event: {msg}");
        }
    }
}

impl App {
    fn on_window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.gfx.is_none() {
            return;
        }
        if self.handle_projector_event(event_loop, id, &event) {
            return;
        }
        if !matches!(event, WindowEvent::RedrawRequested) {
            self.costs.touched();
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.event_loop_action(id, &event) == EventLoopAction::Exit {
                    self.keeping.awaited_saves();
                    self.recording.awaited();
                    event_loop.exit()
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor;
                if let Some(gfx) = self.gfx.as_mut() {
                    App::to_egui(gfx, &mut self.costs, &event);
                    App::wants(
                        gfx,
                        &mut self.egui_due,
                        &mut self.costs,
                        Change::Viewport.repaint(),
                    );
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(gfx) = self.gfx.as_mut() {
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
            }
            WindowEvent::CursorMoved { .. }
            | WindowEvent::CursorLeft { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. } => {
                self.handle_pointer_event(event_loop, event);
            }
            WindowEvent::ModifiersChanged(state) => {
                if let Some(gfx) = self.gfx.as_mut() {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                self.update_modifiers(&state);
            }
            WindowEvent::KeyboardInput { .. } => {
                self.handle_keyboard_event(event_loop, event);
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw_requested(event_loop);
            }
            _ => {
                if let Some(gfx) = self.gfx.as_mut() {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
            }
        }
    }
}
