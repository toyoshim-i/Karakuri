//! Redraw rendering pass and frame composition.

use std::time::{Duration, Instant};

use winit::event_loop::ActiveEventLoop;

use karakuri_console::egui_wgpu;
use karakuri_console::view::{self, Sequenced};
use karakuri_engine::{compose, Committed, Sink, Skip};
use karakuri_store::record::Record;

use super::super::*;
use crate::CANVAS;

impl App {
    pub(crate) fn handle_redraw_requested(&mut self, event_loop: &ActiveEventLoop) {
        let Some(gfx) = self.gfx.as_mut() else {
            return;
        };
        let now = Instant::now();
        if let Some(last) = self.frame_drawn_at {
            if now.duration_since(last) < Duration::from_millis(4) {
                return;
            }
        }
        self.frame_drawn_at = Some(now);

        // Capture frame start clock to compute total frame period including swapchain wait ([`Cost::period`]).
        let period = self.costs.tick(now);
        // Process client model requests and save completions before window operations so faulted windows still respond.
        self.keeping.requests(&mut gfx.engine, &self.store);
        // Drain operations requested by external models before swapchain operations ([`App::operated`]).
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
        // Drain incoming control surface / MIDI operations before swapchain operations ([`App::mapped`]).
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
        // Drain both save and kept procedure channels using bitwise OR to avoid short-circuiting.
        if self.keeping.finished_saves() | self.keeping.finished_keeps() {
            let running = aimed_set(gfx, &self.readout.view);
            // Re-read library directory only on the frame a new Set or procedure was saved (P-0091).
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
        // Drain recording start/stop results and update recording pill status for this frame.
        self.recording.finished();
        let waited = Instant::now();
        let acquired = gfx.surface.get_current_texture();
        let waited = waited.elapsed();
        let frame = if let Some(missed) = missed(&acquired) {
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
            if gfx.projector.is_none() {
                return;
            }
            None
        } else {
            self.faulted = false;
            match acquired {
                wgpu::CurrentSurfaceTexture::Success(frame)
                | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
                _ => {
                    if gfx.projector.is_none() {
                        return;
                    }
                    None
                }
            }
        };

        let mut cost = Cost {
            wait: waited,
            period,
            ..Cost::default()
        };

        // Update sink placement and texture sizes before egui rendering and frame composition.
        self.readout.panel.solve();

        // Layout Program bay cells and canvas dimensions against the largest enabled output ([`CANVAS`]).
        // Emits `Change::Rearranged` to signal whether geometry changed.
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
        let plugin = gfx.plugin.as_ref().map(|p| p.size());
        let external_output = crate::bridge::render_size(&[projector, plugin]);
        let (picture, previews) = gfx.engine.aim(
            &gfx.gpu,
            &mut gfx.renderer,
            self.readout.panel.layout(),
            scale,
            external_output,
        );
        // **What the console draws on the projector's chip**, written
        // per frame beside the frame it is about, exactly as the
        // picture's own registration is — see `view::View::projector`.
        self.readout.view.projector = projector.is_some();
        self.readout.view.plugin = gfx.plugin.is_some();
        self.readout.view.plugin_available = crate::app::operations::is_plugin_available(gfx, 0);
        self.readout.view.plugin_name = gfx.plugin_name;
        self.readout.view.picture = picture;
        self.readout.view.previews = previews;
        // Determine whether each deck displays live or held material following watchdog stops (ADR-0269, ADR-0316).
        self.readout.view.overloaded = stopped_slots(&gfx.engine.deck);

        // Query transport row metrics once per frame to coordinate frame-rate reporting and redraw scheduling.
        let live = live(&self.readout.view);
        // Measure frame step count once from real time after all early-exit points, advancing deck clock (ADR-0078, P-0092).
        let steps = self.clock.steps(Instant::now());
        // Sample audio room tempo correction before transport readouts are updated (`measure_audio`).
        measure_audio(
            &mut gfx.audio,
            &mut gfx.engine.deck,
            self.clock.interval(),
            steps,
            self.recording.recorder(),
        );
        // Poll sequencer lanes on render thread against accumulated beats, emitting live-only operations (ADR-0227, ADR-0322, P-0092).
        let beats = gfx.engine.deck.signals().oscillator().beats();
        let step = self
            .readout
            .playhead
            .advance(self.readout.sequencer.pattern(), beats);
        if let Some(step) = step {
            // Emit lane operations sequentially through `App::performed` without allocating on the frame path (ADR-0222).
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
        // Clone sequencer pattern state and current step for bay rendering (ADR-0156).
        self.readout.view.sequencer = Some(Sequenced {
            pattern: self.readout.sequencer.pattern().clone(),
            bank: self.readout.sequencer.armed(),
            step: self.readout.playhead.at(),
        });
        // Rebuild active deck's aim ID for Library history scope selection (ADR-0304, ADR-0308).
        self.readout.view.aimed = aimed_set(gfx, &self.readout.view);
        self.readout.view.transport = transport(
            &gfx.engine.deck,
            &self.costs,
            gfx.budget_ms,
            live,
            self.readout.health,
            // Read active session recorder status alongside current frame to avoid stale channel state.
            self.recording.rec(),
        );
        // Update look controls and audio tempo octave readouts against the committed frame.
        self.readout.view.tracker = Some(tracking(
            gfx.audio.as_ref(),
            gfx.engine.deck.signals().oscillator().bpm(),
        ));
        self.readout.view.look = Some(look(&gfx.engine.look));
        // Update Master bay output level and look parameters in a single pass (ADR-0224).
        self.readout.view.master_out = Some(gfx.engine.deck.out());
        self.readout.view.master_chain = Some(chain_view(
            &gfx.engine.present,
            &self.readout.view.chain_add.clone(),
        ));
        self.readout.view.master_chain_building = gfx.engine.chain_swap.building().is_some();
        // Read open model classes directly from the handle shared with the MCP server.
        self.readout.view.opening = self.readout.opening.read();
        // **And what the mixer strips read**, beside the frame they
        // are about for the same reason. One strip per slot, so two —
        // see `mixer`.
        mixer(
            &gfx.engine.deck,
            &gfx.material,
            &mut self.readout.view.mixer,
        );
        for i in 0..gfx.engine.deck.slot_count() {
            let in_mix = gfx.engine.deck.is_in_mix(EngineSlot(i as u8));
            self.readout.slot_policies.set_in_mix(i, in_mix);
        }
        self.last_mixer_revision = gfx.engine.deck.mixer_revision();

        // Update Staging lane verdicts and re-publish inspector panes when a Set lands.
        if staging(
            &mut gfx.engine.deck,
            &mut self.keeping,
            &gfx.engine.aimed,
            &mut self.readout.view.staging,
            &mut self.readout.health,
        ) {
            // Update risk badges with fresh candidate benchmarks when a Set swaps in (ADR-0356).
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

        // Publish unified animation clock for UI strip motion on this frame.
        self.readout.view.phase = view::Phase::since(self.started.elapsed());
        // Query pending animations from view to set deadline without masking idle costs (`Change::Animating`).
        App::wants(
            gfx,
            &mut self.egui_due,
            &mut self.costs,
            Change::Animating(self.readout.view.animating(self.readout.panel.layout())).repaint(),
        );
        // Track hover layer dwell deadline while pointer is resting.
        App::wants(
            gfx,
            &mut self.egui_due,
            &mut self.costs,
            Change::Tip(self.hover.owed(gfx.egui.egui_ctx(), self.started.elapsed())).repaint(),
        );

        // Process drag-and-dropped folders from egui input outside cost timers (ADR-0275).
        folder_over(&mut self.readout.view, &gfx.egui.egui_input().hovered_files);
        let dropped = std::mem::take(&mut gfx.egui.egui_input_mut().dropped_files);
        if !dropped.is_empty() {
            let paths: Vec<&std::path::Path> = dropped.iter().map(|file| file.path()).collect();
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

        // Update MIDI hover tooltip from active map when pointer rests on a mapped control (ADR-0156, ADR-0335, ADR-0336).
        let hovering = gfx.egui.egui_ctx().clone();
        let assignment = self.hover.resting().and_then(|(p, _)| {
            let surface = gfx.midi.as_ref()?;
            let operation = asked_at(&self.readout.panel, &hovering, &self.readout.view, p)?;
            let target = target_of(&operation, &gfx.engine.deck).ok()?;
            surface.bound(&target)
        });
        self.hover.assign(assignment);
        let custom_hk = self.hover.resting().and_then(|(_, on)| {
            karakuri_console::hover::descriptor_at(on)
                .and_then(|d| d.operation_title)
                .and_then(|t| self.keymap.find_binding_by_title(t))
                .map(|b| b.legend.to_string())
        });
        self.hover.set_custom_hotkey(custom_hk);

        // -- the egui pass -------------------------------------
        let started = Instant::now();
        let (allocs, bytes) = counted();
        let input = gfx.egui.take_egui_input(&gfx.window);
        let panel = &mut self.readout.panel;
        let view = &mut self.readout.view;
        // Paint hover layer last so tooltips overlay all other panels and cards.
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

        // Record repaint delay requested by egui for root viewport.
        let asked = Repaint::asked(
            output
                .viewport_output
                .get(&karakuri_console::egui::ViewportId::ROOT)
                .map_or(Duration::MAX, |v| v.repaint_delay),
        );

        gfx.egui
            .handle_platform_output(&gfx.window, output.platform_output);

        // Compose engine frame and UI panel within a single command encoder submission (ADR-0166).
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gfx.config.width, gfx.config.height],
            pixels_per_point: output.pixels_per_point,
        };
        let view_target = frame.as_ref().map(|f| {
            f.texture
                .create_view(&wgpu::TextureViewDescriptor::default())
        });
        // **Read from inside the closure, because that is where the
        // engine's half ends and the panel's begins.** `Cost::engine`
        // and `Cost::paint` are then adjacent by construction, rather
        // than two `Instant::now()`s a statement could get between.
        let mut panel_started = None;
        let mut submitting = None;
        let engine_started = Instant::now();
        // The run's one open store, taken out of `self` before the
        // borrow of `self.gfx` below. It is what a chain slot naming
        // anything but a shipped procedure resolves against, and it is
        // the store this run's builds were put in.
        let held = &*self.held;
        let composed = {
            let Gfx {
                gpu,
                renderer,
                engine,
                projector,
                plugin,
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
                chain_swap,
                ..
            } = engine;
            // Apply chain changes at frame boundary before encoding the mix pass (P-0094).
            chain_swap.begin_frame(present, &gpu.device, &gpu.queue);
            for event in chain_swap.events() {
                eprintln!("{event}");
            }
            // Apply chain changes only when structure moves, querying presets then store (ADR-0340, ADR-0354, P-0091).
            if present.chain_spec() != *chain {
                if let Err(refusal) = karakuri_environment::mix::apply_chain(
                    chain_swap,
                    present,
                    &gpu.queue,
                    chain,
                    &|address| karakuri_environment::mix::resolve_procedure(Some(held), address),
                ) {
                    eprintln!("{refusal} — the chain keeps what it had");
                }
            }
            let textures_delta = &mut output.textures_delta;
            let cost = &mut cost;
            // Collect picture and active auxiliary sinks into fixed array for `compose` (ADR-0171).
            let mut one: [&mut dyn Sink; 1];
            let mut two: [&mut dyn Sink; 2];
            let mut three: [&mut dyn Sink; 3];
            let sinks: &mut [&mut dyn Sink] = match (projector.as_mut(), plugin.as_mut()) {
                (Some(p), Some(pl)) => {
                    three = [picture, &mut p.sink, pl];
                    &mut three
                }
                (Some(p), None) => {
                    two = [picture, &mut p.sink];
                    &mut two
                }
                (None, Some(pl)) => {
                    two = [picture, pl];
                    &mut two
                }
                (None, None) => {
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
                    if let Some(view_target) = &view_target {
                        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("console"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: view_target,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    // Fallback clear color before the central panel renders.
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
                    // Apply all texture deltas to avoid epaint panic on drop.
                    textures_delta.clear();
                    cost.record = recording.elapsed();
                    // Submit any custom egui paint command buffers prior to the composite pass.
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

        // Poll GPU queue timing between submit and present to measure shader duration without idling (P-0095).
        if let Some(frame) = frame {
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
        }

        // Record closing tick with real-time measured step count for the session stream (ADR-0297, P-0092, P-0095).
        if let Some(recorder) = self.recording.recorder() {
            recorder.push(Record::Tick { steps });
        }

        self.costs.push(cost);
        App::wants(gfx, &mut self.egui_due, &mut self.costs, asked);
        // Request continuous redraw if any active surface or preview is generating texels (ADR-0164).
        self.costs.live = live;
        // Timestamp clock mode determined by engine calibration probe (P-0095).
        self.costs.clock = gfx.engine.deck.clock();
        // **And what the panel asked for on its own account**, which
        // is the other half of why frames are being drawn on an
        // untouched window. Asked of the view here for the same reason
        // `live` is: one answer per frame, kept for the reading.
        self.costs.declared = self.readout.view.animating(self.readout.panel.layout());
        if live {
            gfx.window.request_redraw();
            if let Some(projector) = &gfx.projector {
                projector.window.request_redraw();
            }
        }
    }
}
