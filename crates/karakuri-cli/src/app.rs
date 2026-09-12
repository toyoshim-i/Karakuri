use super::*;

pub(crate) struct App {
    pub(crate) args: Args,
    pub(crate) procs: Option<Vec<Material>>,
    /// Where each slot's files ended up, slot for slot beside `procs` — what a
    /// session's head is written from. See [`Placed`].
    pub(crate) placed: Vec<Vec<Placed>>,
    pub(crate) live: Option<Live>,
    /// The run's edit history, already holding what it started with. Handed to
    /// every slot's watcher when the deck is built — see [`history`].
    pub(crate) snapshots: Option<history::Shared>,
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
