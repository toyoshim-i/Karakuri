use super::*;

pub mod audio;
pub mod demo;
pub mod interactive;
pub mod status;

pub(crate) use audio::*;
pub(crate) use demo::*;
pub(crate) use interactive::*;

/// How often the status line is printed. Not every frame: at 120 Hz that is a
/// line of stderr per 8 ms, which is unreadable and is I/O on the render thread
/// that nobody asked for.
pub(crate) const STATUS_INTERVAL: Duration = Duration::from_millis(500);

/// Every node of one slot's build: the layer it was sorted onto, its index
/// within that layer, and the hash of the source it was compiled from.
pub(crate) type Nodes = Vec<(&'static str, u32, karakuri_store::hash::Hash)>;

pub(crate) struct Live {
    pub(crate) window: Arc<Window>,
    pub(crate) gpu: Gpu,
    /// Where a composed frame goes. The window is a sink rather than *the* output —
    /// see `docs/plugins.md`, where the others hang.
    pub(crate) sink: frame::WindowSink,
    pub(crate) present: Present,
    /// Asynchronously compiles master chains and manages chain swaps at frame boundaries (ADR-0354).
    pub(crate) chain_swap: karakuri_engine::ChainSwap,
    /// Every Set, whatever is being built to replace any of them, and the mix.
    /// Without `--watch` every slot is a `HotSwap::fixed` and there is no worker at
    /// all, so the frame loop below is the same code either way.
    pub(crate) deck: Deck,
    pub(crate) look: Look,
    /// The slot the gain keys act on. There is no on-screen UI, so this is printed
    /// on every change and marked in the status line.
    pub(crate) focus: usize,
    pub(crate) clock: Clock,
    /// Audio input processor, beat lock, and operator latency offset.
    pub(crate) audio: Option<audio::Audio>,
    /// Control surface handler when `--midi-in` is enabled.
    pub(crate) midi: Option<midi::Surface>,
    /// External tempo source when `--tempo-source` is specified.
    pub(crate) tempo_source: Option<tempo_source::Source>,
    /// What each slot's watcher built, by build id, until the swap that build
    /// produced lands. Not a log: an entry is taken when its build lands or dropped
    /// when a newer one supersedes it.
    pub(crate) rebuilds: Option<std::sync::mpsc::Receiver<watch::Built>>,
    pub(crate) pending_builds: std::collections::HashMap<u64, watch::Built>,
    /// What each slot is running. Seeded before the first frame from the text the
    /// compile read, so there is exactly one way to answer the question a live save
    /// asks — see [`Running`].
    pub(crate) running: Running,
    /// Startup file mappings and source text for each slot node, used during Set saves.
    pub(crate) startup: Vec<Vec<Placed>>,
    /// Set file identifier if loaded from a store Set file.
    pub(crate) loaded_set: Option<String>,
    /// Declared input slot wiring edges for the run.
    ///
    /// Updated during runtime via rewiring operations and propagated to slot
    /// watchers via [`Aiming`] for Set rebuilding and Set file saving.
    pub(crate) edges: Vec<karakuri_engine::set::Edge>,
    /// Where each slot's watcher can be re-pointed, one entry per slot and `None`
    /// for a slot with no watcher — a run without `--watch`, and every
    /// `HotSwap::fixed`. See [`Aiming`]: this is how an edge written over MCP
    /// reaches the thing that rebuilds with it.
    pub(crate) aims: Vec<Option<Aiming>>,
    /// Store root directory for live Set saves.
    pub(crate) store_root: PathBuf,
    /// Opened store handle used for resolving procedure and chain slot addresses.
    pub(crate) store: karakuri_store::store::Store,
    /// Where a save reports back. One thread per save writes into the sender's
    /// clone; the frame loop drains the receiver, which is the shape `rebuilds`
    /// already has and for the same reason: an outcome arrives when it arrives, and
    /// a frame must not wait for it.
    pub(crate) save_tx: std::sync::mpsc::Sender<Saved>,
    pub(crate) saves: std::sync::mpsc::Receiver<Saved>,
    /// How many saves have been started and not yet reported back. The threads are
    /// detached, so this is the only thing that knows a file is still being written
    /// — see [`Live::awaited_saves`], which is why anything counts them at all.
    pub(crate) saves_in_flight: usize,
    /// The MCP server's half of the channel, when `--mcp` asked for one. Told what
    /// the swap machinery said, and nothing else — see [`crate::mcp`].
    pub(crate) mcp: Option<mcp::Reporter>,
    /// Reusable scratch buffer for [`midi::Surface::take`], avoiding per-frame
    /// allocations on the render thread.
    pub(crate) operations: Vec<Operation>,
    /// The musical grid a scheduled fade starts on — see [`QUANTA`]. State on the
    /// operator rather than in the record: what reaches the stream is the resolved
    /// beat count, so this is a setting for the hand and not for the timeline.
    pub(crate) quantum: f64,
    /// How long a scheduled fade lasts, in beats. Same reasoning.
    pub(crate) fade_beats: f64,
    /// The shape the next wipe uses, and which way it runs. Not a slot's mask: this
    /// is what `c` will *give* a slot, where the slot's own is deck state and
    /// travels in the record stream.
    pub(crate) mask_kind: MaskKind,
    pub(crate) mask_angle: f32,
    /// When the session started, so a tap has an origin to be measured from.
    pub(crate) started: Instant,
    pub(crate) status_at: Instant,
    pub(crate) frames_since_status: u32,
    /// Writes the timeline, when `--record-session` asked for one. The frame path
    /// pushes into it and never blocks or allocates — see [`crate::session`].
    pub(crate) recorder: Option<session::Recorder>,
    /// Which demonstration is running and how far into its script, or `None` when
    /// `--demo` was not given and nothing drives itself.
    pub(crate) demo: Option<(Demo, usize)>,
    /// When the current pass through [`DEMO_SCRIPT`] started. The script loops, so
    /// this is not [`Live::started`]: that one is the session's origin and a tap is
    /// measured from it.
    pub(crate) demo_started: Instant,
    /// Reused by the status line. Printing at all on this thread means locking
    /// stderr, but there is no reason for it to mean a fresh allocation twice a
    /// second as well.
    pub(crate) status: String,
}

impl Live {
    /// Handles window resize events by reconfiguring the swapchain sink.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.sink.resize(&self.gpu.device, width, height);
    }

    /// Resizes the window to match the internal render canvas dimensions exactly.
    pub(super) fn snap_to_canvas(&mut self) {
        let (w, h) = self.present.size();
        match self
            .window
            .request_inner_size(winit::dpi::PhysicalSize::new(w, h))
        {
            Some(granted) => {
                self.resize(granted.width, granted.height);
                if (granted.width, granted.height) == (w, h) {
                    eprintln!("window: {w}x{h}, 1:1 with the canvas");
                } else {
                    eprintln!(
                        "window: asked for {w}x{h} and got {}x{} — a capture of this is \
                         not the canvas",
                        granted.width, granted.height
                    );
                }
            }
            // A `Resized` is coming, and it reports what was actually granted.
            None => eprintln!("window: asked for {w}x{h}, 1:1 with the canvas"),
        }
    }

    /// Drains MIDI events from control surfaces and dispatches corresponding operations.
    fn run_surface(&mut self) {
        let Some(surface) = &mut self.midi else {
            return;
        };
        let mut operations = std::mem::take(&mut self.operations);
        surface.take(
            self.deck.slot_count(),
            &karakuri_environment::midi::Decks(&self.deck),
            &mut operations,
        );
        for operation in &operations {
            match operation {
                Operation::TapBeat => self.tap(),
                other => self.operate(other),
            }
        }
        self.operations = operations;
        // Non-blocking update of motorized faders and LED feedback to mirror current deck state.
        if let Some(surface) = &mut self.midi {
            surface.show(&karakuri_environment::midi::Lit {
                deck: &self.deck,
                exposure: self.look.exposure,
            });
        }
    }

    /// Polls and processes incoming MCP save, wire, and operate requests.
    ///
    /// Drains requests at the top of the frame before composition or early exits,
    /// decoupling client responses from swapchain and GPU state.
    fn run_requests(&mut self) {
        let Some(mcp) = &self.mcp else {
            return;
        };
        let asked: Vec<mcp::SaveRequest> = mcp.saves().collect();
        // Drain queues before acting so long saves do not block wiring or operation updates.
        let wires: Vec<mcp::WireRequest> = mcp.wires().collect();
        let operations: Vec<mcp::OperateRequest> = mcp.operations().collect();
        for request in asked {
            self.save_set(Asked::Model, request.slot, request.id, Some(request.reply));
        }
        self.rewire(wires);
        self.run_operations(operations);
    }

    /// Executes MCP model operations through [`Live::operate`], returning whether
    /// the requested operation succeeded or was refused.
    fn run_operations(&mut self, asked: Vec<mcp::OperateRequest>) {
        for mcp::OperateRequest { operation, reply } in asked {
            let title = operation.title();
            let done = match &operation {
                Operation::TapBeat => {
                    self.tap();
                    Ok(())
                }
                other => self.performed(other),
            };
            match done {
                Ok(()) => reply.settled(Ok(performed_at_the_frame(title))),
                Err(said) => refused(Some(reply), said),
            }
        }
    }

    /// Applies requested edge wiring updates and returns outcomes to callers.
    fn rewire(&mut self, asked: Vec<mcp::WireRequest>) {
        if asked.is_empty() {
            return;
        }
        let mut wires = Vec::with_capacity(asked.len());
        let mut replies = Vec::with_capacity(asked.len());
        for mcp::WireRequest { slot, edge, reply } in asked {
            wires.push((slot, edge));
            replies.push(reply);
        }
        let said = rewired(
            &wires,
            &mut self.edges,
            &mut self.aims,
            self.deck.slot_count(),
        );
        for (reply, said) in replies.into_iter().zip(said) {
            match &said {
                Ok(line) | Err(line) => eprintln!("{line}"),
            }
            reply.settled(said);
        }
    }

    /// Records an operation and applies its effects to the live deck mix.
    pub(crate) fn record(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record.clone());
        }
        match mix::change(&record, self.deck.slot_count()) {
            Ok(Some(change)) => self.apply(change),
            Ok(None) => {}
            Err(message) => eprintln!("mix: {message}"),
        }
    }

    /// What one decoded record does. The only place the mix is written.
    fn apply(&mut self, change: mix::Change) {
        match change {
            mix::Change::Gain { slot, value } => self.deck.set_gain(EngineSlot(slot as u8), value),
            mix::Change::Opacity { slot, value } => {
                self.deck.set_opacity(EngineSlot(slot as u8), value)
            }
            mix::Change::Blend { slot, mode } => self.deck.set_blend(EngineSlot(slot as u8), mode),
            mix::Change::Mask { slot, mask } => self.deck.set_mask(EngineSlot(slot as u8), mask),
            mix::Change::Transition {
                slot,
                control,
                to,
                start,
                beats,
                curve,
            } => {
                let t = schedule_from(&self.deck, slot, control, to, start, beats, curve);
                self.deck.schedule(t);
            }
            mix::Change::Select {
                slot,
                renderer,
                start,
            } => {
                let count = self.deck.slot(EngineSlot(slot as u8)).set().inputs().len();
                match renderer_in_range(slot, renderer, count) {
                    Ok(()) => {
                        self.deck
                            .schedule_selection(karakuri_engine::transition::Selection::new(
                                slot, renderer, start,
                            ))
                    }
                    Err(refusal) => eprintln!("{refusal}"),
                }
            }
            // Writes parameter values directly to the active Set on the current deck.
            mix::Change::Ride { slot, writes } => {
                for write in &writes {
                    match self.deck.write_param(EngineSlot(slot as u8), write) {
                        Ok(0) => eprintln!("{}", no_such_param(slot, &write.key)),
                        Ok(_) => {}
                        Err(refused) => eprintln!("{refused}"),
                    }
                }
            }
            // Binds or unbinds parameter drivers in the live Set.
            mix::Change::Source {
                slot,
                layer,
                index,
                key,
                binding,
            } => match binding {
                Some(binding) => match self.deck.bind(EngineSlot(slot as u8), binding) {
                    karakuri_engine::set::Bound::Yes => {}
                    karakuri_engine::set::Bound::NoSuchParam => {
                        eprintln!("{}", no_such_param(slot, &key))
                    }
                    karakuri_engine::set::Bound::NoSuchControl => eprintln!(
                        "slot {slot}: `{key}` is bound to a control this Set does not publish"
                    ),
                },
                None => {
                    if !self.deck.unbind(EngineSlot(slot as u8), layer, index, &key) {
                        eprintln!("slot {slot}: nothing was driving `{key}`");
                    }
                }
            },
            mix::Change::Authority {
                slot,
                layer,
                index,
                authority,
            } => {
                if !self
                    .deck
                    .set_authority(EngineSlot(slot as u8), layer, index, authority)
                {
                    eprintln!(
                        "slot {slot}: no node {}:{index} to make {}",
                        karakuri_environment::meta::layer_name(
                            karakuri_environment::meta::layer_of(layer),
                        ),
                        authority.name()
                    );
                }
            }
            mix::Change::Residency { slot, level } => {
                self.deck.set_residency(EngineSlot(slot as u8), level);
                self.govern("residency");
            }
            mix::Change::Look(look) => self.look = look,
            mix::Change::MasterOut(value) => self.deck.set_out(value),
            // Applies master post-processing chain changes to Present (ADR-0224, ADR-0317).
            mix::Change::MasterChain(slots) => {
                if let Err(refusal) = mix::apply_chain(
                    &mut self.chain_swap,
                    &mut self.present,
                    &self.gpu.queue,
                    &slots,
                    &|address| mix::resolve_procedure(Some(&self.store), address),
                ) {
                    eprintln!("  {refusal} — the chain keeps what it had");
                }
            }
            mix::Change::Transport {
                slot,
                sync,
                anchor_bpm,
                scrub_beats,
            } => {
                if let Err(refusal) =
                    self.deck
                        .set_transport(EngineSlot(slot as u8), sync, anchor_bpm, scrub_beats)
                {
                    eprintln!("slot {slot}: {} sync refused — {refusal}", sync.name());
                }
            }
        }
    }

    pub(crate) fn frame(&mut self) {
        self.run_demo();
        self.run_surface();
        // Process model requests and complete pending saves before composition.
        self.run_requests();
        self.finished_saves();

        // Chain swap boundary: commit any pending master chain compilations before frame composition (ADR-0354).
        self.chain_swap
            .begin_frame(&mut self.present, &self.gpu.device, &self.gpu.queue);
        for event in self.chain_swap.events() {
            eprintln!("  {event}");
        }
        let Live {
            gpu,
            deck,
            present,
            sink,
            clock,
            audio,
            recorder,
            look,
            tempo_source,
            ..
        } = self;
        let mut sinks: [&mut dyn frame::Sink; 1] = [sink];
        let outcome = frame::compose(
            gpu,
            deck,
            present,
            &mut sinks,
            &mut |_at, skip| {
                if let frame::Skip::Fault(why) = skip {
                    eprintln!("surface: {why}");
                }
            },
            |deck| {
                let steps = clock.steps(Instant::now());
                let grid = follow_tempo_source(tempo_source, deck, recorder);
                measure_audio(audio, deck, recorder, clock.interval(), steps, grid);
                if let Some(recorder) = recorder {
                    recorder.push(karakuri_store::record::Record::Tick { steps });
                }
                frame::Committed { steps, look: *look }
            },
            |_| {},
        );

        if let Err(e) = outcome {
            eprintln!("surface: {e}");
        }

        // Process deck swap events, report status to terminal/MCP, and trigger governor check on changes.
        let mut set_changed = false;
        let mut procedures: Vec<(usize, u64)> = Vec::new();
        for slot in 0..self.deck.slot_count() {
            for event in self.deck.events(EngineSlot(slot as u8)) {
                set_changed |= matches!(event, Event::Swapped { .. });
                match &self.mcp {
                    Some(mcp) => {
                        let said = event.to_string();
                        eprintln!("slot {slot}: {said}");
                        mcp.swap(slot, &said);
                    }
                    None => eprintln!("slot {slot}: {event}"),
                }
                if let Event::Swapped { id, .. } = event {
                    procedures.push((slot, id));
                }
            }
        }
        for (slot, landed) in procedures {
            self.took_up(slot, landed);
        }
        if set_changed {
            self.govern("build landed");
        }

        self.frames_since_status += 1;
        if self.status_at.elapsed() >= STATUS_INTERVAL {
            self.print_status();
        }
    }
}
