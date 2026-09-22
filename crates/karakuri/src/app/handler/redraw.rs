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

        // **The frame's own clock, and the first statement of the
        // frame because that is the whole of what makes it one.** Two
        // consecutive readings of this bracket a whole redraw — the
        // block on the swapchain, every timed stretch, every untimed
        // one and `Queue::present` — so [`Cost::period`] is the frame
        // and not a part of it. Nothing else in this handler can say
        // that: every other clock here starts after the wait.
        let period = self.costs.tick(now);
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
        self.readout.view.master_chain = Some(chain_view(
            &gfx.engine.present,
            &self.readout.view.chain_add.clone(),
        ));
        self.readout.view.master_chain_building = gfx.engine.chain_swap.building().is_some();
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
        for i in 0..gfx.engine.deck.slot_count() {
            let in_mix = gfx.engine.deck.is_in_mix(EngineSlot(i as u8));
            self.readout.slot_policies.set_in_mix(i, in_mix);
        }
        self.last_mixer_revision = gfx.engine.deck.mixer_revision();

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
            // Set with its own two numbers.** The build worker
            // measures and estimates what it built and both install
            // with it (ADR-0356), so the slot that just swapped is
            // governed on the candidate's own frame from the install
            // onward — and a dot left saying what the Set before it
            // cost would be the meter this whole sub-milestone exists
            // to stop.
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
            Change::Animating(self.readout.view.animating(self.readout.panel.layout())).repaint(),
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
            Change::Tip(self.hover.owed(gfx.egui.egui_ctx(), self.started.elapsed())).repaint(),
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
            let operation = asked_at(&self.readout.panel, &hovering, &self.readout.view, p)?;
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
            // **The frame boundary the chain lands on**, before the
            // encoder below exists: a chain arriving mid-frame would
            // move what the mix writes into after the mix had decided.
            // Nothing here waits — a build that is not finished is not
            // collected and the chain that is running draws this frame
            // (P-0094).
            chain_swap.begin_frame(present, &gpu.device, &gpu.queue);
            for event in chain_swap.events() {
                eprintln!("{event}");
            }
            // **On the frames it moved and on no others**, which is
            // where this parts company with the tone map one pass
            // along: that is one `queue.write_buffer` into storage
            // sized at construction, and this is a list whose *shape*
            // may have changed — new procedures to compile, new
            // targets to allocate. `mix::apply_chain` takes the cheap
            // path where the shape is the one already running, which
            // is what a press on a Master row produces
            // (`docs/principles/0091-cost-is-known-before-it-is-paid.md`),
            // and asks `karakuri-chain` for anything else. It is safe
            // to ask on every frame: a list already being built is not
            // asked for twice (ADR-0354).
            //
            // The shipped three first, then this run's store, which is
            // the order `mix::resolve_procedure` states: a chain of
            // presets resolves with no store at all, and any other
            // slot's source is one this run's `procedure` records
            // already put there. An address nothing holds is refused
            // with the address in the message (ADR-0340).
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
            // **The picture first, and the projector and plugin sinks beside it
            // when present.** Fixed arrays rather than one of `Option`s because
            // `compose` takes a slice of live references and a hole in it would
            // be a sink that has to be asked whether it is there — which is the
            // `Sink::acquired` this seam already refused (ADR-0171).
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
