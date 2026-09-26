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
            // Enforce minimum inner size matching `karakuri_console::MINIMUM_VIEWPORT`
            // to ensure layout tracks and faders remain visible (ADR-0250, ADR-0272, ADR-0279).
            .with_min_inner_size(winit::dpi::LogicalSize::new(
                karakuri_console::MINIMUM_VIEWPORT.0,
                karakuri_console::MINIMUM_VIEWPORT.1,
            ));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let size = window.inner_size();
        self.scale = window.scale_factor();

        let instance = Gpu::instance();
        // Report surface and adapter errors instead of panicking across the winit/OS boundary (ADR-0168).
        let surface = match instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(e) => no_gpu(&format!("no surface: {e}")),
        };
        let gpu = match pollster::block_on(Gpu::from_instance(instance, Some(&surface))) {
            Ok(gpu) => gpu,
            Err(e) => no_gpu(&format!("no adapter: {e}")),
        };

        let caps = surface.get_capabilities(&gpu.adapter);
        // Egui expects a non-sRGB surface format because its shaders output sRGB-encoded colors (P-0064).
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        // Picture present pass requires an sRGB target format offered by the surface (P-0064).
        let picture_format = match caps.formats.iter().copied().find(|f| f.is_srgb()) {
            Some(format) => format,
            // Refuse initialization if no compatible sRGB target format is offered (P-0083).
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

        // Size the initial picture from the region rectangle, matching `RedrawRequested`'s texture sizing.
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
        // Seed deck state from initial compile output on window setup or remake ([`Playing::at_launch`]).
        self.keeping.playing = Playing::at_launch(&engine.placed, engine.deck.slot_count());
        // **Before the first frame and before the first strip is written**, so
        // that the panel's first frame draws the deck as it actually is rather
        // than a settled version of it that the second frame corrects.
        let governed = engine.startup(&gpu);
        // Initialize the four risk badges from governor pass results (ADR-0356).
        self.readout.view.costs = costs(&governed);
        let info = gpu.adapter.get_info();
        self.costs.taken_on = format!(
            "{:?} — {} ({:?})",
            info.backend, info.name, info.device_type
        );

        let budget = budget_ms(&window);
        // Align candidate Set watchdog budget with display refresh interval, falling back to constant if unknown (ADR-0313, P-0095).
        if let Some(budget) = budget {
            engine.deck.set_frame_budget_ms(budget);
        }
        // Initialize strip state from deck slots before legend readout ([`Gfx::material`]).
        let material: Vec<String> =
            std::iter::repeat_n(self.sources.material(), engine.deck.slot_count()).collect();
        mixer(&engine.deck, &material, &mut self.readout.view.mixer);
        for i in 0..engine.deck.slot_count() {
            let in_mix = engine.deck.is_in_mix(EngineSlot(i as u8));
            self.readout.slot_policies.set_in_mix(i, in_mix);
        }
        // Seed available library scopes and count before initializing the legend.
        self.readout.view.scopes = Scope::ALL.to_vec();
        // Default to `Scope::All` on startup and verify active scope selection (ADR-0299).
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
                // Slots launch from command-line pair rather than a Set until explicitly loaded (ADR-0304).
                None,
            )
        );
        // **And the arrangement pill's menu, once for the run**, for
        // `library`'s reason and for one more: this is a directory read, and
        // the only thing that can add a name to it is a save this program
        // performs — which re-reads it there. See `arrangements`.
        self.readout.view.arrangement.filed = arrangements(&self.store);
        // Seed inspector panes from initial published set definitions before the first frame.
        let targets = self.readout.view.pane_decks();
        inspector(
            &engine.deck,
            &material,
            &engine.aimed,
            targets,
            &mut self.readout.view.inspector,
        );
        // Seed Master bay level so the legend reports active engine presence.
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
        // Update audio-in pill state even when no device was detected so the UI draws `audio-in · none`.
        self.readout.view.audio = Some(told(audio.as_ref()));
        // Seed audio tempo and control values for legend readout before the first frame.
        self.readout.view.tracker = Some(tracking(
            audio.as_ref(),
            engine.deck.signals().oscillator().bpm(),
        ));
        // Resolve MIDI surface map from store or preset directory and initialize environment.
        let map = midi::map_for(
            &self.store,
            self.presets.as_ref().map(|presets| presets.dir.as_path()),
        );
        // `controller` rather than `surface`, which in this function is the
        // swapchain's.
        let (controller, plugged) = surfaced(map.as_deref(), self.waker.clone());
        println!("{plugged}");
        // Update map pill when a surface is present, drawing `map · none` if unmapped.
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
            // Pre-allocate buffer to avoid reallocation while draining MIDI messages during the frame path.
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

    /// Handle due deadlines at iteration start, checking both deadlines regardless of [`StartCause`].
    /// Drains [`App::operated`] and routes operations requiring the event loop (ADR-0341).
    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
        let now = Instant::now();
        if self.egui_due.is_some_and(|due| due <= now) {
            self.egui_due = None;
            if let Some(gfx) = self.gfx.as_ref() {
                // One frame, now that the delay `egui` asked for has passed.
                gfx.window.request_redraw();
            }
        }
        // Measure costs only after the engine is initialized, retaining due state until recorded.
        if self.costs.due().is_some_and(|due| due <= now) {
            if let Some(gfx) = self.gfx.as_ref() {
                // Cost reading labels active workload by joining current slot material names in order.
                let (capacity, material) = (gfx.engine.capacity, gfx.material.join(" / "));
                // Record refresh interval and composite output size for accurate cost reporting.
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
                // Drain both save and kept procedure channels using bitwise OR to avoid short-circuiting.
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
                // Request a redraw when a build finishes so the swap takes effect at the frame boundary.
                gfx.window.request_redraw();
            }
        }
    }

    /// Update event loop control flow (`Wait` vs `WaitUntil`) based on soonest pending deadline (ADR-0164, [`SERVED`]).
    /// Triggers redraw if wake events or MIDI inputs are queued ([`App::waker`]).
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
