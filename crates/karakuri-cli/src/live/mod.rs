use super::*;

pub mod audio;
pub mod demo;
pub mod interactive;

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
    /// Where the master chain is compiled and where the chain it replaces is
    /// freed. A press on the Master rows asks this for a build and the frame
    /// loop installs the result at a frame boundary; nothing on this thread
    /// compiles a shader, creates a pipeline or allocates a chain target
    /// (ADR-0354).
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
    /// The audio input, the beat lock, and the operator's latency offset. `None`
    /// without `--audio-in`, and then nothing in the frame path below changes at
    /// all — which is the property the whole slice is about.
    pub(crate) audio: Option<audio::Audio>,
    /// The control surface, when `--midi-in` asked for one. Every operation it
    /// produces ends in the same record a key press ends in — see [`crate::midi`] —
    /// so nothing in the frame path below changes at all when this is `None`.
    pub(crate) midi: Option<midi::Surface>,
    /// The tempo source, when `--tempo-source` asked for one. `None` and nothing in
    /// the frame path below changes at all — the grid comes from `--bpm`, the
    /// tracker and the tap keys, exactly as it did before this existed. That is the
    /// same property `audio` and `midi` have and it is the one worth keeping.
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
    /// Where each slot's files were at launch, one entry per node, in the order
    /// they were spelled.
    ///
    /// The names, and the bytes each node was compiled from. A name belongs to the
    /// use rather than to the procedure, so `--set veil=shell.kir` is a fact about
    /// the command line that no rebuild restates and no hash carries, and
    /// `Live::save_set` zips these onto the hashes by position.
    ///
    /// Not the paths, which is the distinction that matters. This used to answer
    /// "what is this slot running" by re-reading `named.path`, and that was a
    /// second answer to a question [`Running`] already held. What is here now is
    /// [`Placed::source`] — the text the compile read — and it is not a second
    /// answer but the *first*: every hash in `Running` was derived from it, and a
    /// save hands the same buffer to the store so the file it writes resolves.
    /// There is one derivation, and this is where its input lives.
    ///
    /// Empty for a slot filled straight from a Set file without `--watch` or
    /// `--mcp`, which has no files behind it at all — see where `placed` is built.
    pub(crate) startup: Vec<Vec<Placed>>,
    /// The Set file this run was loaded from, for the one refusal that has to name
    /// it: a slot filled from a file with nothing watching it has no sources to
    /// save, and an operator asking why is owed the id and the flag that would
    /// change the answer.
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
    /// The store root a live save writes into. The *root* and not the open store
    /// below: every part of a save that touches a disk happens on the thread that
    /// does it — see [`Save::run`].
    pub(crate) store_root: PathBuf,
    /// The store this run resolves a chain slot's address against.
    ///
    /// Opened when the run starts. The shipped three resolve without a store at
    /// all; every other address resolves out of this one
    /// (`karakuri_environment::mix::resolve_procedure`).
    ///
    /// `Store::open` creates the directories under the root, so a run creates the
    /// store whether or not it ever writes to it.
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
    /// The window changed size. Nothing that is rendered changes.
    ///
    /// This used to resize the HDR target and every deck slot as well, because the
    /// window's size *was* the canvas. Two things came of that, and both are gone
    /// with it: dragging a window reallocated every slot's target once per frame of
    /// the drag — a GPU allocation on the render thread, which is the one thing
    /// this engine's frame path forbids — and what a run rendered depended on how
    /// big its window happened to be, so the same session replayed at a different
    /// size with nothing saying which was the performance. All that is left here is
    /// the swapchain, which has to follow the window because it *is* the window.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.sink.resize(&self.gpu.device, width, height);
    }

    /// Resize the window so the canvas lands in it one texel to one texel.
    ///
    /// The preview is fitted, so an OBS window capture of it would otherwise pick
    /// up the bars and a scale — and a capture that is neither the canvas nor a
    /// clean crop of it is worse than useless downstream. After this the window
    /// contains the canvas exactly, and `letterbox` becomes the identity.
    ///
    /// A request, not a guarantee, and the difference is printed. A 1080-tall
    /// canvas cannot get a 1080-tall content window on a 1080-tall display — there
    /// is a menu bar or a taskbar in the way — so the manager clamps it, and an
    /// operator setting up a capture has to be told that rather than told "1:1".
    ///
    /// `request_inner_size` returns the granted size immediately on the platforms
    /// where the manager decides and `None` where a `Resized` event will follow.
    /// Taking the returned value matters on the first kind: no event arrives, so
    /// nothing else would ever reconfigure the swapchain, and a swapchain that
    /// disagrees with its window does not fail — `set_viewport` is not validated
    /// against the attachment — it just draws the wrong picture, silently.
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

    /// Whatever the control surface did since the last frame, as the same
    /// operations a key press and a console fader name.
    ///
    /// The whole of the MIDI connection, and there is almost nothing in it: a
    /// mapped message *is* an [`Operation`], so this hands each one to
    /// [`Live::operate`] and that is the connection. A surface can do nothing a key
    /// cannot because both end in the same record, and a session recorded from one
    /// replays with neither attached.
    ///
    /// This used to be a match over eight `Action`s claiming that a control added
    /// to one and not the other does not compile. Against a fifty-variant
    /// vocabulary that claim would be false — a router arm nobody wrote is a
    /// wildcard nobody notices. The guarantee is now where it is true:
    /// `karakuri_operation_record::written` is one exhaustive match over all fifty,
    /// so an operation nobody has said what to do with stops the build there.
    ///
    /// [`Operation::TapBeat`] is handled here and it is the only one, for a reason
    /// that is visible rather than incidental: a tap moves the beat tracker rather
    /// than writing a value, `written` answers `Owed::NotSettled` for it, and
    /// `Live::operate` would print that gap instead of tapping. `Live::tap` is what
    /// owns the tracker and what the `b` key reaches, so `note -> tap` goes on
    /// doing exactly what it did. The day the record a tap owes is settled, this
    /// arm is what goes.
    ///
    /// Nothing here prints. The old arms ended in `set_gain` and its neighbours,
    /// each of which reports what it did — which on a fader sweep is an `eprintln!`
    /// per MIDI message inside a frame, several hundred a second, and is the
    /// blocking write per message `crate::midi`'s own "once per control" rule
    /// exists to prevent. What a surface moved is read back from the deck (`s`),
    /// not narrated per message.
    ///
    /// Before the tick, so a fader move lands on the frame it arrived for rather
    /// than the one after — the same placement `run_demo` has, and for the same
    /// reason.
    fn run_surface(&mut self) {
        let Some(surface) = &mut self.midi else {
            return;
        };
        // Into the owned scratch, then out of `self`'s borrow, so the calls
        // below can take `&mut self`. Nothing allocates: both vectors are
        // reused and `take` clears rather than replaces.
        let mut operations = std::mem::take(&mut self.operations);
        // The slot check is the router's — it is where the "say it once"
        // machinery already is, and once per *message* would be a blocking
        // write per message on this thread. See `crate::midi`.
        //
        // **And the deck, for the one target the map cannot finish on its
        // own**: `cc -> param N M` names a *position* in a deck's published
        // interface, which becomes a key only against the Set that is in the
        // deck (ADR-0268, ADR-0336). `midi::Decks` is that reading, and it is
        // the same one the panel makes — a map file means one thing in both
        // programs or it means nothing.
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
        // **And the surface is shown where the deck ended up** — MIDI out, on
        // the frame the change lands and after the frame's operations have
        // been applied, so a motorised fader follows the value the deck holds
        // rather than the one it was asked for.
        //
        // **It does not wait**: `Surface::show` queues into a bounded channel
        // and drops when it is full rather than blocking this thread, which is
        // the same rule every other thing this frame does (P-0094,
        // `crate::midi`). A run with no output port costs one branch.
        //
        // **Every source is shown, not just the surface's own.** A key press,
        // a model over `--mcp` and a transition move the deck too, and the
        // whole point of MIDI out is that two things can move a fader.
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

    /// Every edge asked for since the last frame, written and answered here, on
    /// this frame.
    ///
    /// The decisions are [`rewired`]'s and are written there, because none of them
    /// needs a `Live`. What is here is the two things that do: the deck's own slot
    /// count, which is the only thing that knows how many slots there are, and the
    /// answer going back to whoever asked.
    ///
    /// Answered once, at the frame it was applied on, which is
    /// [`mcp::WireRequest`]'s third point. Not at the swap: what the *build* made
    /// of the edge is `swap_outcome`'s answer, as it is for every other rebuild,
    /// and a tool that waited for thirty judged frames would hold a connection open
    /// across a transition.
    ///
    /// One sentence for both audiences, which is [`refused`]'s rule: what the
    /// terminal is told and what the client is handed are the same words, so the
    /// second cannot be right on the day it is written and wrong at the next
    /// correction.
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

    pub(crate) fn record(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            // Cloned, which allocates for the records that carry a name —
            // `look`, `mask`, `blend`, `residency`, `transport` — and for no
            // other. This was written when the only way in was a key press. A
            // mapped MIDI fader reaches it from `Live::run_surface` inside
            // `Live::frame`, so on `exposure` and `mask-position` it was a
            // small heap touch per control-change message on the render
            // thread, which is the first rule this repository has. It is now
            // **once a frame per control**: `crate::midi`'s router keeps the
            // last value a continuous control sent within a frame and drops
            // the ones before it, so a sweep of several hundred messages
            // reaches here once
            // (`docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md`;
            // the program's `mix.rs` carries the measurement, which is why the
            // coalescer is
            // there).
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
            // **Checked against the Set on screen**, which is the only place
            // the answer is: the decoder knows the deck's size and not what is
            // in it. A record from a session recorded against a Set with three
            // renderers and replayed against one with two lands here.
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
            // **The one change that reaches inside a Set.** Every arm around
            // it moves the deck the Sets are playing on; this writes a number
            // into the Set in one slot, and `Deck::write_param` is the public
            // road to it. It compiles nothing — the value is packed into the
            // uniform by the next `Set::prepare`, which is the next frame.
            //
            // **A rebuild does not carry it**, and that is not settled here:
            // what a `--watch` rebuild restates is `swap::Request::params`, and
            // nothing puts a live write there. See
            // `docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`,
            // which names it as the open question and why it is a bigger one
            // than the record was.
            mix::Change::Ride { slot, writes } => {
                for write in &writes {
                    match self.deck.write_param(EngineSlot(slot as u8), write) {
                        Ok(0) => eprintln!("{}", no_such_param(slot, &write.key)),
                        Ok(_) => {}
                        Err(refused) => eprintln!("{refused}"),
                    }
                }
            }
            // **The other change that reaches inside a Set**, beside the
            // ride above: this one says what a parameter blends *towards*
            // rather than what it blends *from*. `Deck::bind` and
            // `Deck::unbind` are the public roads, and neither compiles
            // anything — a binding is resolved by the next `Set::prepare`.
            //
            // **A rebuild does not carry it**, on the ride's own terms and for
            // the same reason: `swap::Request::bindings` is restated from
            // `Watch::bindings`, which only a re-point writes. See
            // `docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`.
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
            // **Who may move one node**, and it changes what one knob may do
            // from the next press: a bare-name write over nodes that are not
            // all under one authority is refused whole by `Set::write_param`.
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
                // A slot arriving or leaving changes what is committed, and the
                // deck's headroom with it: a slot that could not be admitted a
                // moment ago may fit now, and one that fitted may not. Requests
                // are untouched by the pass, so a park recovers on its own the
                // moment there is room.
                self.govern("residency");
            }
            // **Stored and not applied.** `frame::compose` writes the tone map
            // uniform every frame from the look the committing closure hands
            // it, so writing it here as well would be a second writer of one
            // value — the shape this whole module set out to remove, in
            // miniature. `Live::apply_look` is gone with it.
            mix::Change::Look(look) => self.look = look,
            // **The level at the chain's entry, applied where the mix writes
            // the composited frame** — `Deck::set_out`, which names no slot
            // because it acts on what the fold produced (ADR-0224).
            mix::Change::MasterOut(value) => self.deck.set_out(value),
            // **And the chain itself, applied and not stored, which is the
            // opposite of the look one arm up and for a stated reason.** The
            // `Present` is where a chain lives — it owns the targets its slots
            // read and write — so there is nowhere else to put it, and a copy
            // on this struct beside it would be the second writer the look arm
            // refuses (ADR-0317, and `Present::chain_spec` is how it is read
            // back).
            //
            // **What it costs is now two things and the record decides
            // which**: a list whose shape is the shape already running is a
            // uniform write per slot here, and any other is a build asked of
            // `karakuri-chain` and installed at the frame boundary in
            // [`Live::frame`]. See `mix::apply_chain`.
            //
            // **A refusal is said and the chain that is running stays.** An
            // address the store does not hold is a record this build cannot
            // obey, which is reported at the operation rather than drawn as a
            // wrong picture. A source that compiles and refuses as a chain slot
            // is said where the build lands instead, for the same reason and a
            // frame or two later.
            mix::Change::MasterChain(slots) => {
                if let Err(refusal) = mix::apply_chain(
                    &mut self.chain_swap,
                    &mut self.present,
                    &self.gpu.queue,
                    &slots,
                    // The shipped three first and then this run's store, which
                    // is the order `mix::resolve_procedure` states. The refusal
                    // names the address it could not find.
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
                // The refusal is reported and nothing moves. It cannot happen
                // from a key press — `cycle_sync` only offers modes the Set
                // allows — but a session recorded against one Set and replayed
                // against another is exactly where it can, and a slot silently
                // left free would be a performance replayed wrong.
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
        // **Both halves of the save path, and both above the GPU work below.**
        // Taking the request here is [`Live::run_requests`]'s own reasoning: a
        // client asking to keep what is playing should not be waiting on a
        // swapchain, and nothing a request reaches needs the GPU. The drain
        // used to sit under `frame::compose`, which implemented half of that
        // and quietly withheld the other: a window latched to `Skip::Fault`, or
        // returning `Outdated` every frame, returned before it — so the request
        // was taken, the save thread wrote the file, and the
        // client waited out `SAVE_REPLY` to be told the outcome was neither
        // success nor failure about a save that had already landed. The
        // terminal never said "saved as set X" either, and the `save` record
        // was withheld from the stream until the run quit. A save's outcome has
        // nothing to do with whether there is a surface to draw on, which is
        // exactly what the two calls being here rather than there says.
        self.run_requests();
        self.finished_saves();

        // **The ordering that used to be a comment is still the shape of the
        // call, and what it orders has changed.** A `tick` is a promise that
        // the deck advanced by that many steps, and what makes it honest is
        // that `frame::compose` calls the closure below and renders the deck
        // with nothing between them. It is no longer that a frame with nowhere
        // to draw records nothing: it is that there is no such frame. The
        // defect this all came from is unchanged and is
        // `docs/adr/0078-a-frame-that-is-discarded-must-not-already-have-been-recorded.md`
        // — the loop read the clock, wrote the `tick`, measured the audio, and
        // *then* found the swapchain had nothing, and `Outdated` arrives on
        // every resize, so resizing during a recorded session made the replay
        // diverge from the performance.
        //
        // **The window is one sink and does not gate the frame.** A frame it
        // refuses is composed anyway — the clock is read, the audio is
        // measured, the `tick` is written, the deck advances — and the only
        // thing the refusal costs is the picture, which is what an output being
        // off has to mean before there is a second one. So `Clock::last` now
        // moves on every frame this function runs and no interval is carried
        // between frames: the gap that still has to survive is the one where
        // this function does not run at all, and `MAX_STEPS` is the anti-spiral
        // clamp on it however long it was.
        // **The frame boundary the chain lands on**, before the encoder
        // `frame::compose` opens below: a chain arriving mid-frame would move
        // what the mix writes into after the mix had decided. Nothing here
        // waits — a build still running is not collected, and the chain that is
        // running draws this frame (ADR-0354).
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
        // One sink today, and the slice is the whole of what a second one — a
        // projector, a plugin — costs this function. See `docs/plugins.md`.
        let mut sinks: [&mut dyn frame::Sink; 1] = [sink];
        let outcome = frame::compose(
            gpu,
            deck,
            present,
            &mut sinks,
            // A `Fault` is printed unconditionally, because it is already at
            // most one per condition: the sink latches it — see `frame::Skip`.
            // The latch used to be here, which meant the message was *built*
            // every frame and thrown away, an allocation on the frame path for
            // as long as the window stayed broken.
            //
            // `Transient` is the swapchain being remade and the next frame
            // asking again; there is nothing to say about it sixty times a
            // second. The index says which sink refused, and there is one, so
            // nothing here reads it yet.
            &mut |_at, skip| {
                if let frame::Skip::Fault(why) = skip {
                    eprintln!("surface: {why}");
                }
            },
            |deck| {
                let steps = clock.steps(Instant::now());
                // **The source first, and it takes the grid with it.** Both end in
                // a `tempo` record and `Oscillator::correct` is last-writer-wins,
                // so running the source first and letting the tracker follow would
                // have meant the tracker winning — it returns a trim on every
                // frame once locked, against the source's four a second. The order
                // is not what settles it: `Grid::Followed` is, by telling the
                // tracker to keep tracking and keep quiet.
                let grid = follow_tempo_source(tempo_source, deck, recorder);
                // **Everything this frame decided, and only then the `tick` that
                // closes it.** A tick is a terminator rather than a header:
                // `session::split` files each record into the frame of the *next*
                // tick, so a record written after this frame's tick belongs to the
                // next frame. The audio was on the wrong side of that line, which
                // showed a replay frame N what frame N−1 heard.
                measure_audio(audio, deck, recorder, clock.interval(), steps, grid);
                if let Some(recorder) = recorder {
                    recorder.push(karakuri_store::record::Record::Tick { steps });
                }
                frame::Committed { steps, look: *look }
            },
            // The window's frame is its sinks and nothing else — there is no
            // panel over the top of it here. See `frame::compose`.
            |_| {},
        );

        // **Said and not returned on.** A present that failed costs this frame's
        // picture; it does not cost the frame, which has already committed and
        // advanced the deck. Everything below is about the frame rather than
        // about the window — the builds that landed while it ran, the governor
        // that has to hear about them, and the status line that says how many
        // frames a second are arriving — and a run whose window has stopped
        // taking them is exactly when an operator needs to be told the rest.
        if let Err(e) = outcome {
            eprintln!("surface: {e}");
        }

        // A build landing replaces the Set in a slot, and with it the
        // measurement the deck is budgeting against. **A swap is the whole of
        // that list since ADR-0316**: a verdict against stops the slot with the
        // Set the swap installed and moves nothing. Collected here and governed
        // after the drain, because `Deck::events` borrows the deck for as long
        // as it is being read.
        let mut set_changed = false;
        // Collected rather than recorded inside the loop: `Deck::events`
        // borrows the deck for as long as it is read, and writing a record
        // needs the recorder.
        let mut procedures: Vec<(usize, u64)> = Vec::new();
        for slot in 0..self.deck.slot_count() {
            for event in self.deck.events(EngineSlot(slot as u8)) {
                set_changed |= matches!(event, Event::Swapped { .. });
                // **The same words, to whoever is not at the terminal.** A
                // model that wrote a procedure has no other way to learn that
                // its slot was stopped for cost, and "it compiled" is not the
                // same news as "it is on screen and running".
                //
                // Formatted once and only when there is somebody to tell: a run
                // with no `--mcp` used to pay for a `String` it then dropped.
                match &self.mcp {
                    Some(mcp) => {
                        let said = event.to_string();
                        eprintln!("slot {slot}: {said}");
                        mcp.swap(slot, &said);
                    }
                    None => eprintln!("slot {slot}: {event}"),
                }
                // **What a session says it played, at the moment it changed.**
                // The material used to be written once, before the first frame,
                // so a run in which a procedure was rewritten replayed as
                // though it never had — and with a model at the other end of
                // `--mcp` that is the common case rather than a corner.
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

    /// What the operator needs and nothing that costs a stall to know: which slots
    /// are on air, what they are faded to, where their simulation clocks are, the
    /// output look, and whether frames are still arriving on time.
    ///
    /// Element live counts are deliberately absent — `Set::live_count` blocks until
    /// the queue drains, so a status line carrying it would put a GPU sync on the
    /// render thread twice a second. It is printed once, at exit.
    pub(super) fn print_status(&mut self) {
        let elapsed = self.status_at.elapsed().as_secs_f32();
        let fps = if elapsed > 0.0 {
            self.frames_since_status as f32 / elapsed
        } else {
            0.0
        };
        self.status_at = Instant::now();
        self.frames_since_status = 0;

        self.status.clear();
        for slot in 0..self.deck.slot_count() {
            let addr = EngineSlot(slot as u8);
            let _ = write!(
                self.status,
                "{}{slot} {} g{:.2} t{:.1}s ",
                if slot == self.focus { ">" } else { " " },
                residency_tag(self.deck.residency(addr), self.deck.is_parked(addr)),
                self.deck.gain(addr),
                self.deck.slot(addr).set().time()
            );
            // **The slot has stopped updating, and only while it has.** The
            // version in it costs more than one frame may, so the engine skips
            // its step and its draw and it holds the frame it last drew
            // (ADR-0316). Printed on the same terms as the transport and the
            // fader below — a run in which no slot is stopped prints the line
            // it always printed — and printed *whole*, not as a four-letter
            // column beside the residency: it is not a residency, a stopped
            // slot is still mixed, and `t` standing still beside `LIVE` is
            // exactly the reading an operator would otherwise take for a bug.
            self.status
                .push_str(stopped_tag(self.deck.overloaded(addr)));
            // **What is moving, and where it is going.** An armed fade is
            // invisible otherwise: with the default quantum it is due up to a
            // bar after the key, and the only thing that said so was one line
            // at press time. A control that changes something invisible is
            // indistinguishable from a control that is broken, which is this
            // file's own argument for printing on every key.
            for t in self.deck.transitions_on(addr) {
                let _ = write!(
                    self.status,
                    "{}>{:.2} ",
                    match t.control() {
                        karakuri_engine::transition::Control::Gain => "g",
                        karakuri_engine::transition::Control::Opacity => "o",
                        karakuri_engine::transition::Control::MaskPosition => "w",
                    },
                    t.to()
                );
            }
            // **And what is waiting to be chosen**, on the same terms and for
            // the same reason: a selection armed for the next bar is invisible
            // between the key and the music, and `r>1` is the only thing on
            // screen that says a renderer is about to change.
            for selection in self.deck.selections_on(addr) {
                let _ = write!(self.status, "r>{} ", selection.renderer());
            }
            // Omit opacity and blend mode when at defaults (opacity 1.0, Add).
            if self.deck.opacity(addr) != 1.0 {
                let _ = write!(self.status, "o{:.2} ", self.deck.opacity(addr));
            }
            if self.deck.blend(addr) != Blend::Add {
                let _ = write!(self.status, "{} ", self.deck.blend(addr).name());
            }
            // Display transport status only when active (non-free).
            let transport = self.deck.transport(addr);
            match transport.sync() {
                Sync::Free => {}
                Sync::Tempo => {
                    let _ = write!(self.status, "T{:.0} ", transport.anchor_bpm());
                }
                Sync::Beat => {
                    let _ = write!(
                        self.status,
                        "B{:.0}{} ",
                        transport.anchor_bpm(),
                        if transport.scrub_beats() == 0.0 {
                            String::new()
                        } else {
                            format!("{:+.2}", transport.scrub_beats())
                        }
                    );
                }
            }
            // The level, which is why the meter exists: two Sets are matched
            // on `m` and `p` warns which one will dominate the mix wherever it
            // lands regardless of its fader. `None` for an off-air slot is the
            // meter saying it has nothing current rather than showing the last
            // thing the slot drew — see `Deck::level`.
            match self.deck.level(addr) {
                Some(level) => {
                    let _ = write!(self.status, "m{:.3} p{:.1}", level.mean, level.peak);
                    // Texels the meter left out because they were not a finite
                    // number, and **only when there are any**. Not a warning:
                    // dividing by a value that reaches zero is an ordinary
                    // thing for a shader to do and what it produces is a
                    // blown-out pixel. It is here because it explains the two
                    // numbers beside it — they are a mean and a peak over the
                    // texels this did not count — and because 3 and 300000 are
                    // a stray sprite and a frame that is gone.
                    if level.bad_texels > 0 {
                        let _ = write!(self.status, " x{}", level.bad_texels);
                    }
                    self.status.push_str("  ");
                }
                None => self.status.push_str("m---- p----  "),
            }
            // What every binding on this slot last wrote. Without it a binding
            // that is doing nothing — an unknown signal, or an invented one at
            // a tenth effect — is indistinguishable from a binding that never
            // attached, which is the whole reason confidence is worth seeing
            // rather than merely being applied. Nothing is printed for a slot
            // with no bindings, so the default status line is unchanged.
            for (key, value) in self.deck.slot(addr).set().bound() {
                let _ = write!(self.status, "{key}={value:.3}  ");
            }
        }
        // What audio is doing, when there is any. A performer cannot tune what
        // they cannot see: the confidence says whether the input is alive, and
        // the phase error says which way the offset wants nudging.
        // **Where the grid is coming from, before what it currently is.** The
        // number an operator needs to trust is the tempo; the thing that tells
        // them whether to trust it is its source, and until now there was only
        // ever one. `link 2p` is a shared grid with two other peers on it;
        // `link 0p` is a source running and alone, which is a different
        // situation from no source at all and has to look different.
        if let Some(source) = &self.tempo_source {
            let _ = write!(
                self.status,
                "| {} {}p{}{} ",
                source.name(),
                source.peers(),
                // **Counted, not swallowed.** Anchors thrown away for
                // unusable numbers mean a source that has gone wrong, and
                // nothing else would ever say so.
                match source.rejected() {
                    0 => String::new(),
                    n => format!(" x{n}"),
                },
                match (source.ended().is_some(), source.playing()) {
                    (true, _) => " GONE",
                    // **Three states and not two.** A source that has greeted
                    // and said nothing else is not a stopped source, and the
                    // two used to print the same thing — which is the pair an
                    // operator would act differently on. Shown and not acted
                    // on, like every other measurement here.
                    (false, None) => " ?",
                    (false, Some(false)) => " stop",
                    (false, Some(true)) => "",
                }
            );
            // **The tempo, when nothing else is going to print it.** The
            // grid's bpm has always lived in the audio group, which is fine
            // while a microphone is the only thing that moves it — and wrong
            // the moment something else does: a run with `--tempo-source` and
            // no `--audio-in` followed a tempo that appeared nowhere on
            // screen. Printed here only when the audio group is absent, so it
            // appears exactly once either way.
            if self.audio.is_none() {
                let _ = write!(
                    self.status,
                    "{:.1}bpm ",
                    self.deck.signals().oscillator().bpm()
                );
            }
        }
        if let Some(audio) = &self.audio {
            let a = audio.status();
            let _ = write!(
                self.status,
                "| e{:.2} on{:.2} c{:.2} | {}{:.1}bpm heard{:.1} c{:.2}{} err{:+.3}b off{:.0}ms ",
                a.energy,
                a.onset,
                a.confidence,
                // The grid's own tempo first, because that is what the picture
                // is actually running at; what the tracker hears second, so a
                // disagreement between the two is visible rather than implied.
                if a.locked { "lock " } else { "free " },
                self.deck.signals().oscillator().bpm(),
                a.estimated_bpm,
                a.estimate_confidence,
                // The tracker's note that this grid might be at half the
                // music's tempo, shown only when the estimate behind it is
                // worth anything. It moves nothing by itself — `.` does, and
                // this is what tells a performer to consider pressing it.
                if a.half_tempo_hint
                    && a.estimate_confidence >= karakuri_audio::lock::GATE_CONFIDENCE
                {
                    " x2?"
                } else {
                    ""
                },
                a.error,
                audio.latency_offset_ms(),
            );
        }
        eprintln!(
            "{}| {} exp {:.2} | {fps:.1} fps",
            self.status,
            op_name(self.look.op),
            self.look.exposure
        );
    }
}
