//! The console, drawn: every region in its place, with its heading, and
//! nothing inside.
//!
//! **One window loop, not two.** This replaces `examples/layout.rs`, which
//! painted every region as a flat rectangle and drew no text at all — a
//! harness written to answer *does a divider drag* before there was a toolkit
//! to answer it with. `egui` supersedes the painter, and keeping both would
//! have meant two window loops disagreeing about who owns the pointer.
//!
//! ```sh
//! cargo run -p karakuri-console --example panel
//! cargo test -p karakuri-console                   # the model and the view
//! cargo test -p karakuri-console --example panel   # this file's own tests
//! ```
//!
//! # What this is for
//!
//! Five things, and only one of them is a bay.
//!
//! 1. That `egui` renders through `wgpu` 30 into this window at all, which is
//!    the bet
//!    [ADR-0155](../../../docs/adr/0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md)
//!    made.
//! 2. **That the engine's rendered texture reaches the panel**, which is the
//!    other half of that bet: a `Deck` of one Set on the same `Device`, drawn
//!    through `Present` into the picture's rectangle **and again into deck A's
//!    preview cell**, sampled by `egui` in the same submission. One `Present`
//!    and two targets of different sizes, because `Present::draw` letterboxes
//!    into whatever it is handed. Both are `karakuri_engine::Sink`s and the
//!    frame is one `frame::compose`; the panel goes into that frame's own
//!    encoder through its `finally`, because the panel is not a sink — it does
//!    not receive the composited frame, it samples what a sink produced. See
//!    [`Engine`] and [`Presented`].
//! 3. That the arrangement's numbers look right at real sizes against
//!    `docs/manual/console.html`.
//! 4. That dragging a boundary still works with a toolkit in the loop.
//! 5. **What the window costs**, printed once nobody has touched it for a
//!    while. It used to be *what a still panel costs*, and with a live picture
//!    in the Program bay and deck A auditioning under it there is no still
//!    panel to measure — so what is printed is the price of the thing that
//!    replaced it. See [`Costs::say`].
//!
//! **It is deliberately not the start of a bay.** Every body is empty except
//! the picture and the preview row, and both are empty of everything this file
//! could have invented — no label, no frame, no placeholder; see
//! [`karakuri_console::view`].
//!
//! **There is one frame loop now, and it is not in this file.**
//! `frame::compose` is `karakuri-engine`'s, and `karakuri-cli`'s window and
//! its PNG writer are the other two callers — which is the point of it: this
//! example used to hand-roll `begin_frame`, a render, a conditional present
//! pass per target, the panel and a submit, beside a loop in another crate
//! that said the same thing differently, and every replay defect this project
//! has found came from two such loops disagreeing.
//!
//! # The engine here is scaffolding and looks it
//!
//! One slot, built from two of the repository's own `examples/*.kir` the way
//! `karakuri-cli` builds them, and **no more than that**: no audio, no MIDI,
//! no store, no argument parsing, no governor, no records. What
//! `karakuri-cli` puts around a deck is a program; this is the shortest path
//! from two files to texels, because the question being answered is whether
//! the texels arrive.
//!
//! **One slot is why three of the four preview cells are off.** There is no
//! second deck here to audition, so deck A gets a live cell and B, C and D
//! read `off` — which is a state an operator chooses rather than a thing not
//! built yet, and is the truth about this example rather than a gap in it.
//!
//! # This file owns none of the model, and none of the view
//!
//! The arrangement, the drag, the operations, the palette, the bay head and
//! the rule about who gets a pointer event are all `karakuri-console`. What is
//! left here is a window, a surface, the `egui` plumbing between them, and the
//! English — a `Dragged` into the line it prints, an `Outcome` into the line a
//! key prints. **The model returns what happened; the words are this file's.**
//!
//! # The readout is driven by the layout, not by the pointer
//!
//! A line is printed when the boundary **moves**, and a stop is announced
//! once. That is `Panel::moved`'s doing rather than this file's: it returns
//! nothing at all for a move that changed nothing. A line per pointer event is
//! one comparison less and wrong twice over — the interesting lines are buried
//! under identical ones, and a hand held against the edge of the window, which
//! is where a drag ends up, produces hundreds a second into a terminal that
//! has to keep up with them while the window waits.
//!
//! # The loop sleeps, and what wakes it is a decision made elsewhere
//!
//! [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)'s
//! first clause: a panel with nothing changing on it does no per-frame work.
//! **Whether a frame is owed is `karakuri_console::repaint`'s to answer**, not
//! this file's — the same seam the model came out of the window loop through,
//! and for the same reason: an event handler cannot be called from a test, and
//! an under-repaint is a stale pixel rather than an error, so there is nothing
//! to assert against in here. Every arm below reaches
//! [`Change::repaint`](karakuri_console::repaint::Change::repaint) and none of
//! them decides for itself.
//!
//! Three things wake the loop and they are the whole list: a window event,
//! the deadline `egui` named for its own next frame, and the deadline the
//! reading below is taken on. `App::about_to_wait` is the one place
//! `ControlFlow` is set.
//!
//! # Every frame that could not be acquired gets a decision
//!
//! This loop waits for events rather than spinning, so nothing asks for
//! another frame on its own. A `get_current_texture` that comes back
//! `Outdated` and is dropped with a bare `return` is therefore a window that
//! stops drawing and never starts again, with nothing said anywhere — see
//! [`missed`], carried over from the example this replaces, and
//! `karakuri-cli`'s window sink, which carries the same table after the same
//! failure.
//!
//! # A panic here aborts the process
//!
//! macOS cannot unwind a Rust panic across the Objective-C frame a `winit`
//! callback is called from, so an assertion reachable from an event handler is
//! a crash and not an error. Nothing below asserts on input.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::time::{Duration, Instant};

use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Dragged, Op, Outcome, Panel, Pressed, Released, Visibility};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::Room;
use karakuri_console::view::{
    self, outputs, picture_rect, preview_rects, Kind, Picture, View, DECKS,
};
use karakuri_engine::{
    compose, Committed, Deck, Gpu, HotSwap, Look, MaskKind, Present, Residency, Set, Sink, Skip,
    TonemapOp,
};
use karakuri_layout::{Axis, Hit, NodeId, Point};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// The window this opens, in logical pixels. Comfortably above the smallest
/// viewport the arrangement is claimed to work at, so nothing starts clamped.
const WINDOW: (f64, f64) = (1440.0, 900.0);

// ---------------------------------------------------------------------------
// What a still panel costs
// ---------------------------------------------------------------------------

/// How long the window has to go untouched before the reading is taken, and
/// the stretch every number in it is measured over.
///
/// **The measurement is the claim now, and it used to be its own opposite.**
/// What was here drove the window for 180 frames and reported what one of them
/// cost; the loop had to spin for the sample to fill, so the number described
/// a program that no longer exists the moment the loop stops spinning.
/// [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)'s
/// first clause claims that nothing is drawn at all while nothing is
/// happening, so that is what is counted: frames drawn in three seconds of an
/// untouched window, and what they allocated.
///
/// **Three seconds** because a 60 Hz spin fills it with 180 frames — the old
/// sample, to the frame, so the two readings are about the same stretch of
/// wall clock — and because a twelve-second run has room for it several times
/// over.
const STILL: Duration = Duration::from_secs(3);

/// How many frames the per-frame sample holds.
///
/// It is a cap and not a target: nothing drives the window to fill it, and a
/// run where nobody touches anything leaves it nearly empty, which is the
/// point. Allocated once, so measuring does not allocate on the path it is
/// measuring.
const SAMPLE: usize = 240;

/// The allocator, counting. **Per thread, not per process** — `wgpu` allocates
/// on threads of its own and a process-wide counter would attribute that to
/// the `egui` pass, which is the one number this exists to get right.
struct Counting;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
    static BYTES: Cell<u64> = const { Cell::new(0) };
}

/// `(allocations, bytes)` this thread has asked for since it started.
/// Reallocations count as one allocation of the new size, which overstates a
/// growing `Vec` and is the conservative direction.
fn counted() -> (u64, u64) {
    (
        ALLOCS.try_with(Cell::get).unwrap_or(0),
        BYTES.try_with(Cell::get).unwrap_or(0),
    )
}

fn count(size: usize) {
    let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
    let _ = BYTES.try_with(|c| c.set(c.get() + size as u64));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
        count(layout.size());
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: AllocLayout) -> *mut u8 {
        count(layout.size());
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: AllocLayout, new_size: usize) -> *mut u8 {
        count(new_size);
        System.realloc(ptr, layout, new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout) {
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// What one frame cost.
#[derive(Debug, Clone, Copy, Default)]
struct Cost {
    /// **The engine's half**: [`compose`] from its first statement up to the
    /// moment it hands the encoder back for the panel — both sinks' `acquire`,
    /// the committing closure, the tone-map write, `Deck::begin_frame` with
    /// every install in it, the deck's render, and the present passes into
    /// whichever sinks took the frame. **Two present passes and one deck
    /// render** with both sinks on, because an audition is a second present of
    /// the same canvas and not a second simulation of it — and one pass, or
    /// none, when a region is folded away and its sink refuses.
    ///
    /// **Where it stops is the start of [`Cost::paint`] and not a second
    /// clock**: the panel's half begins when `compose` calls `finally`, so the
    /// two are adjacent by construction and no part of the frame falls between
    /// them. CPU time only, like everything else here; what the GPU then does
    /// with the command buffer is not on this clock, and
    /// `docs/contributing.md` §1 says why there is no other one.
    engine: Duration,
    /// `take_egui_input` through `tessellate`: the whole immediate-mode pass,
    /// including this console's own layout walk and every shape it emits.
    ui: Duration,
    /// Uploading the tessellated geometry, recording the panel's render pass,
    /// and the one submission that carries both halves — the whole of what the
    /// example records from inside [`compose`]'s `finally`, plus `compose`'s
    /// own tail: `Frame::finish`, and each sink's `present`, which for these
    /// two sinks is nothing at all. Excludes `Queue::present` on the surface,
    /// which is the display's pace and not a cost.
    paint: Duration,
    /// **Uploading `egui`'s texture deltas, out of [`Cost::paint`]** — the
    /// font atlas and its patches. Zero on a steady frame, because the atlas
    /// is built once.
    ///
    /// This and the two below exist because of what happens when they do not.
    /// The split was taken once with temporary instrumentation, published as a
    /// conclusion and removed, and the next machine asked to answer *how much
    /// of the frame is the cost of moving data to the GPU* could not: the
    /// question is about `buffers` and the readout stopped at `paint`. A
    /// measurement that has to be re-instrumented to be repeated is one number
    /// rather than a series.
    textures: Duration,
    /// **Uploading the tessellated geometry, out of [`Cost::paint`]** — and
    /// the number the caching decision turns on.
    ///
    /// **It is not the price of moving bytes**, which took three machines to
    /// establish and is `docs/adr/0167-the-panel-keeps-re-uploading-what-did-not-change.md`.
    /// The discrete GPU that has to cross a bus pays 0.022 ms; an APU that
    /// crosses nothing pays 0.010; this machine, which also crosses nothing,
    /// pays 0.166 — seven times the one with the bus. It is what a backend's
    /// upload path costs, and the decision it was printed for is taken: **not
    /// re-uploading the unchanged panel is worth at most 4% of a frame, and
    /// the dirty-tracking it needs is not.**
    buffers: Duration,
    /// **Recording the panel's render pass, out of [`Cost::paint`]** — the
    /// view, the pass, and `egui`'s draw calls into it. Recording only; what
    /// the GPU then does with it is on no clock here.
    record: Duration,
    /// **The submission alone, out of [`Cost::paint`]** — `Queue::submit` and
    /// `finish` on the frame's encoder, which is [`compose`]'s last act and is
    /// why this is measured from inside `finally` to after `compose` returns.
    /// The only other thing in that window is each sink's `present`, and both
    /// of this example's are `Ok(())` — it is five sixths of `paint`.
    ///
    /// Printed because the whole of the rest of `paint` is what caching a bay
    /// into a texture would make cheaper, and this is not: it is `wgpu`'s
    /// per-submission cost, it is proportional to what was recorded rather
    /// than to what the GPU then does with it, and it is unmoved by a canvas
    /// 256 times the area. It is host-side work and not a wait — measured
    /// against `CLOCK_THREAD_CPUTIME_ID` it burns 99% of its wall time on the
    /// CPU. **The wait is [`Cost::wait`], which is a different number
    /// entirely.**
    submit: Duration,
    /// **What the frame spent blocked in `get_current_texture`**, waiting for
    /// the display to free a swapchain image. `PresentMode::Fifo`, so at 60 Hz
    /// this is most of the 16.6 ms and the frame is not paying for it: on the
    /// same clock as above it burns under 1% of itself on the CPU.
    ///
    /// It is in none of the three numbers above, which is why it is here — a
    /// reader who sums those three and compares the total to a frame gets an
    /// answer that is 12% of the truth, and the missing 88% is this doing
    /// nothing on purpose. Switching to `PresentMode::Immediate`, which this
    /// adapter does offer, moves it and nothing else.
    wait: Duration,
    /// Allocations and bytes during `ui`, on this thread.
    allocs: u64,
    bytes: u64,
}

impl Cost {
    /// **What the whole frame cost on the CPU**: the three fields the reading
    /// below adds up under *"the whole frame is a median"*, for one frame
    /// rather than for the median of each.
    ///
    /// The three tile the frame exactly and do not overlap — `engine` ends
    /// where `paint` begins, by construction, and `ui` is the `egui` pass
    /// before either — and [`Cost::wait`] is deliberately not in it: blocking
    /// in `get_current_texture` is the display's pace and not a price, which
    /// is the sentence that field carries.
    fn whole(&self) -> Duration {
        self.engine + self.ui + self.paint
    }
}

/// What was drawn while nobody was touching the window. **This is the number**
/// P-0072's first clause is about, and the clause says every field of it is
/// zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Still {
    frames: usize,
    allocs: u64,
    bytes: u64,
}

/// The sample, the stillness, and the summary printed once.
struct Costs {
    /// What the frames that *were* asked for cost.
    frames: Vec<Cost>,
    /// Every frame drawn, including any past the end of `frames`.
    drawn: usize,
    /// When the window was last touched by anything at all.
    quiet_since: Instant,
    /// **A frame is owed to something that happened**, and the next one drawn
    /// is that frame rather than a frame drawn on a still panel.
    ///
    /// Without it the reading blames the panel for the frame it was asked for:
    /// an event resets the stretch and the frame it asked for lands a
    /// millisecond into the new one, so a window that did exactly the right
    /// thing reports having drawn on an untouched panel.
    owed: bool,
    /// What has been drawn since `quiet_since` that nothing asked for.
    still: Still,
    /// **The frame just drawn**, kept so the transport row can print what it
    /// cost.
    ///
    /// Not a second measurement and not a second sample: it is the last
    /// [`Cost`] [`Costs::push`] was handed, which `frames` stops keeping after
    /// [`SAMPLE`] of them because that vector is a sample rather than a
    /// history. A readout wants the last frame and the reading wants the
    /// sample; both are the same numbers.
    last: Option<Cost>,
    said: bool,
    /// **What ran it**, as the adapter reports it: the backend, the adapter's
    /// own name, and whether it calls itself discrete.
    ///
    /// Printed because a readout that does not name its backend is how a
    /// measurement gets written into a table as though it were another one's.
    /// `WGPU_BACKEND` was honoured by nothing for months and the failure was
    /// invisible precisely here — twenty-five runs were recorded as DX12 and
    /// were Vulkan, and nothing on screen could have said otherwise
    /// (`docs/adr/0168-a-backend-override-is-honoured-because-a-no-op-cannot-be-caught.md`).
    /// The adapter is asked rather than the environment, so an override that
    /// does not take reads as what it is.
    taken_on: String,
    /// **Whether anything in the Program bay is making texels** — the
    /// picture, a deck auditioning in a preview cell, or both. [`live`] is the
    /// rule and this is the frame's answer to it.
    ///
    /// It decides which sentence the reading prints about the frames it
    /// counted, and nothing else: the frames are counted the same way either
    /// way, which is the point — the number is not adjusted for knowing the
    /// answer.
    live: bool,
}

impl Costs {
    fn new() -> Costs {
        Costs {
            frames: Vec::with_capacity(SAMPLE),
            drawn: 0,
            quiet_since: Instant::now(),
            owed: false,
            still: Still::default(),
            last: None,
            said: false,
            taken_on: String::from("an adapter nobody asked"),
            live: false,
        }
    }

    /// **Something touched the window**, so the stillness starts again from
    /// here and what was drawn during the last stretch is no longer about a
    /// still panel.
    ///
    /// Every window event that is not a frame this loop asked for itself
    /// counts, including the ones the operator did not cause — a move, a
    /// focus, an occlusion. Resetting too eagerly only ever makes the reading
    /// harder to reach, never easier to pass.
    fn touched(&mut self) {
        self.quiet_since = Instant::now();
        self.still = Still::default();
    }

    /// **A frame was asked for by something that happened**, so the next one
    /// drawn is not on the panel's account.
    ///
    /// Every `request_redraw` in this file goes through here except one, and
    /// the exception is the point of the reading: **the frame `egui` asked for
    /// after a delay it named.** The distinction is between `egui` saying *I
    /// have not finished drawing what you just asked me to* — a `repaint_delay`
    /// of zero, a second pass of a frame already owed, which is what it does
    /// for a pass or two while the font atlas settles — and `egui` saying
    /// *wake me in 250 ms*, which is an animation and is per-frame work on a
    /// panel nobody is touching. The first goes through here and the second
    /// does not, so a delay mishandled into a spin shows in the reading as the
    /// frames it actually drew.
    ///
    /// The other way it can fail is loud rather than quiet: something asking
    /// for frames without pause never lets the window be still for [`STILL`],
    /// and then no reading is printed at all. **A run of this example that
    /// prints no reading is that failure.**
    fn owes(&mut self) {
        self.owed = true;
        self.touched();
    }

    fn push(&mut self, cost: Cost) {
        self.drawn += 1;
        self.last = Some(cost);
        if !self.owed {
            self.still.frames += 1;
            self.still.allocs += cost.allocs;
            self.still.bytes += cost.bytes;
        }
        self.owed = false;
        if self.frames.len() < SAMPLE {
            self.frames.push(cost);
        }
    }

    /// **Frames a second**: what was drawn on the untouched window, over the
    /// stretch it was drawn in.
    ///
    /// The stretch is the caller's because the two callers are at different
    /// points in it and neither may guess the other's. [`Costs::say`] is taken
    /// exactly [`STILL`] after `quiet_since` and says so in the sentence
    /// above the number; the transport row is asked on every frame and its
    /// stretch is however much of one has elapsed. One quotient, two stretches
    /// — and a second expression of *frames over seconds* would be the readout
    /// and the reading disagreeing about the rate of the same window.
    fn rate_over(&self, stretch: f64) -> f64 {
        self.still.frames as f64 / stretch
    }

    /// **The rate as it stands, for the transport row** — or `None` where
    /// there is not yet a stretch with a frame in it to divide.
    ///
    /// `None` is the honest answer twice over. Just after something touched
    /// the window there is no stretch, and a rate over no time is an infinity.
    /// And a window with nothing live on it stops asking for frames entirely,
    /// so the stretch goes on growing while the frames do not — which is a
    /// rate falling towards zero and is exactly what the window is doing.
    fn rate_now(&self) -> Option<f64> {
        let stretch = self.quiet_since.elapsed().as_secs_f64();
        (self.still.frames > 0 && stretch > 0.0).then(|| self.rate_over(stretch))
    }

    /// When the reading is due, and `None` once it has been taken. It is also
    /// what keeps the loop on a deadline until then — see
    /// [`App::about_to_wait`].
    fn due(&self) -> Option<Instant> {
        match self.said {
            true => None,
            false => Some(self.quiet_since + STILL),
        }
    }

    /// The reading. Median and worst rather than a mean for the per-frame
    /// figures: a frame path is judged by its tail.
    fn say(&mut self) {
        if self.said {
            return;
        }
        self.said = true;

        println!();
        println!(
            "{}",
            match self.live {
                // There is no still panel to cost while anything is live, and
                // calling it one would be the reading describing a program
                // that is not running — which is the failure the sample this
                // replaces made, one revision ago. **"Something live" rather
                // than "a live picture"**: the picture can be folded away with
                // deck A still auditioning under it, and the window is no more
                // still then than it was before.
                true => "what an untouched window costs with something live in it:",
                false => "what a still panel costs, measured on this window:",
            }
        );
        println!(
            "  over {:.1} s with nothing touching it: {} frames drawn, {} allocations, \
             {} bytes",
            STILL.as_secs_f64(),
            self.still.frames,
            self.still.allocs,
            self.still.bytes
        );
        let rate = self.rate_over(STILL.as_secs_f64());
        match (self.live, self.still == Still::default()) {
            // The reading this was written for, and it is now only reachable
            // with the picture off.
            (false, true) => println!(
                "  so P-0072's first clause holds here: no per-frame work is done to \
                 redraw what nobody has touched and nothing has moved."
            ),
            (false, false) => println!(
                "  so P-0072's first clause does NOT hold here — something is asking \
                 for frames on an untouched window, and with nothing on the panel \
                 making texels the likeliest something is an `egui` repaint delay \
                 answered immediately instead of waited out."
            ),
            // **The expected reading now**, and the whole of what this run is
            // for. It is stated as a price rather than as a failure, because
            // that is what it is: the clause is about a panel with nothing
            // changing on it, and a live engine frame is something changing on
            // it.
            (true, _) => {
                println!(
                    "  so P-0072's first clause has stopped holding, and the reason is the \
                     engine: there is a live frame in the Program bay — the picture, deck \
                     A auditioning in the preview row under it, or both — so every one of \
                     those frames was asked for by what is live rather than by anybody \
                     touching the window."
                );
                println!(
                    "  that is {rate:.1} frames a second, against 0 with the engine out — \
                     which is the whole of the difference, since what one frame costs is \
                     below and did not change."
                );
                println!(
                    "  it is P-0072's second clause from here — what moves declares its \
                     price — and this is the price, measured. Nothing in this run \
                     schedules, caches the panel to a texture or declares anything; the \
                     number is what the next decision gets made on."
                );
            }
        }

        if !self.frames.is_empty() {
            let mut engine: Vec<f64> = self.frames.iter().map(|c| ms(c.engine)).collect();
            let mut ui: Vec<f64> = self.frames.iter().map(|c| ms(c.ui)).collect();
            let mut paint: Vec<f64> = self.frames.iter().map(|c| ms(c.paint)).collect();
            let mut textures: Vec<f64> = self.frames.iter().map(|c| ms(c.textures)).collect();
            let mut buffers: Vec<f64> = self.frames.iter().map(|c| ms(c.buffers)).collect();
            let mut record: Vec<f64> = self.frames.iter().map(|c| ms(c.record)).collect();
            let mut submit: Vec<f64> = self.frames.iter().map(|c| ms(c.submit)).collect();
            let mut wait: Vec<f64> = self.frames.iter().map(|c| ms(c.wait)).collect();
            // **Median, where the figure this replaces was a mean.** The mean
            // was over 180 frames and the first one was lost in it; the sample
            // here is however many frames somebody asked for, which on a run
            // nobody touches is three — and the first of those builds the font
            // atlas and allocates ten times what a frame does. A mean of three
            // is that one frame with two others attached.
            let mut allocs: Vec<u64> = self.frames.iter().map(|c| c.allocs).collect();
            let mut bytes: Vec<u64> = self.frames.iter().map(|c| c.bytes).collect();
            engine.sort_by(f64::total_cmp);
            ui.sort_by(f64::total_cmp);
            paint.sort_by(f64::total_cmp);
            textures.sort_by(f64::total_cmp);
            buffers.sort_by(f64::total_cmp);
            record.sort_by(f64::total_cmp);
            submit.sort_by(f64::total_cmp);
            wait.sort_by(f64::total_cmp);
            allocs.sort_unstable();
            bytes.sort_unstable();
            let n = self.frames.len();

            println!();
            println!(
                "what a frame costs when something asks for one, over the {} drawn so far \
                 ({} sampled):",
                self.drawn, n
            );
            println!(
                "  engine pass  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                engine[n / 2],
                engine[n * 95 / 100],
                engine[n - 1]
            );
            println!(
                "  egui pass    median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                ui[n / 2],
                ui[n * 95 / 100],
                ui[n - 1]
            );
            println!(
                "  upload+pass  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                paint[n / 2],
                paint[n * 95 / 100],
                paint[n - 1]
            );
            // **The four parts of `upload+pass`, and the reason they are
            // printed rather than derived.** `submit` alone told the caching
            // decision what it was *not* — the submission is not what caching
            // a bay into a texture would make cheaper — without telling it
            // what it was. The upload is the line that decision turns on, and
            // it is worth what a bus costs on the machine reading it, so it
            // has to be a number this program prints on every machine rather
            // than one somebody instruments for once.
            println!(
                "    of which texture uploads  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 the font atlas, built once, so zero on a steady frame",
                textures[n / 2],
                textures[n * 95 / 100],
                textures[n - 1]
            );
            println!(
                "    of which buffer uploads   median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 the tessellated geometry. Not the price of crossing a bus: it is seven times \
                 larger here than on a discrete GPU that crosses one (ADR-0167)",
                buffers[n / 2],
                buffers[n * 95 / 100],
                buffers[n - 1]
            );
            println!(
                "    of which record the pass  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                record[n / 2],
                record[n * 95 / 100],
                record[n - 1]
            );
            println!(
                "    of which submit  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 `wgpu`'s per-submission work, not the GPU's and not a wait",
                submit[n / 2],
                submit[n * 95 / 100],
                submit[n - 1]
            );
            println!(
                "  waiting for vsync  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 blocked in `get_current_texture`, and in none of the three above",
                wait[n / 2],
                wait[n * 95 / 100],
                wait[n - 1]
            );
            println!(
                "  the egui pass allocates a median {} times a frame, {:.1} kB a frame \
                 (worst {} and {:.1} kB, which is the first frame building the font atlas)",
                allocs[n / 2],
                bytes[n / 2] as f64 / 1024.0,
                allocs[n - 1],
                bytes[n - 1] as f64 / 1024.0
            );
            println!(
                "  the whole frame is a median {:.3} ms, so at {:.1} frames a second the \
                 loop is spending {:.1}% of a second drawing.",
                engine[n / 2] + ui[n / 2] + paint[n / 2],
                rate,
                (engine[n / 2] + ui[n / 2] + paint[n / 2]) * rate / 10.0
            );
            println!(
                "  that per-frame price is not what changed, and immediate mode pays it by \
                 construction: ADR-0164 measured 184 allocations and 226.2 kB a frame here \
                 with every bay empty, and the egui pass above is still that. What changed \
                 is how many frames pay it — 0 with a still panel and nothing in the \
                 Program bay, and the rate above with anything live in it."
            );
            println!("  taken on {}", self.taken_on);
            println!(
                "  the panel half is taken on this window at {:.0}x{:.0} logical with every \
                 other bay empty, which is NOT the workspace's reference workload. The \
                 engine half IS: one Set of {} elements at {}x{}, one step a frame, and \
                 that one canvas presented twice — into the picture's rectangle, \
                 and again into deck A's preview cell, both of which are the \
                 canvas's own shape \
                 (docs/contributing.md §1). Host clock, debug profile with dependencies \
                 at opt-level 3.",
                WINDOW.0, WINDOW.1, CAPACITY, CANVAS.0, CANVAS.1
            );
            println!(
                "  and every figure above is taken on a core that spends the vsync wait \
                 asleep. On THIS machine that matters a great deal — the identical run with \
                 the other cores loaded reports about a sixth of these numbers, proportions \
                 unchanged — and it is this machine's power management rather than a rule: \
                 two Windows machines were asked the same way and got 1.3x and 1.6x WORSE \
                 under load, which is ordinary contention. Compare ratios, not magnitudes."
            );
        }
        println!();
        println!(
            "{}",
            match self.live {
                // **Two sinks, so two folds.** This said "fold the picture
                // away (f over it) and it does" while deck A was auditioning
                // in the row underneath, which is a sentence that sends an
                // operator to watch a window that is still drawing at full
                // rate — the same false claim, in the same place, that cost
                // this file 270 frames once already.
                true =>
                    "the loop asks for the next frame from inside the last one for as long \
                     as anything is making texels, so `ControlFlow::Wait` never gets to \
                     block. Two things are: the picture, and deck A auditioning in the \
                     preview row under it. Fold the picture away (f over it) and deck A \
                     keeps the loop awake on its own; fold the preview row away as well \
                     and `ControlFlow::Wait` finally blocks — that is the same window \
                     drawing nothing, and it is what P-0072's remaining clauses are for.",
                false =>
                    "the loop is on `ControlFlow::Wait` from here: it does nothing at all \
                     until the window is touched or `egui` names a deadline of its own.",
            }
        );
        println!();
    }
}

/// Say what failed, say whether an override is the likely reason, and stop.
///
/// Never returns, so it can stand where a `?` cannot: this is inside a `winit`
/// callback, where a panic aborts rather than unwinds and takes the message
/// with it.
fn no_gpu(what: &str) -> ! {
    eprintln!("{what}");
    match std::env::var("WGPU_BACKEND") {
        Ok(want) => eprintln!(
            "WGPU_BACKEND is set to `{want}`, and this codebase honours it — so the most likely \
             reason is that this machine has no {want}. Unset it to take whatever the machine \
             offers."
        ),
        Err(_) => eprintln!(
            "WGPU_BACKEND is not set, so this is the machine's own default backend failing \
             rather than an override."
        ),
    }
    std::process::exit(1)
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

// ---------------------------------------------------------------------------
// The readout: English for what the model returned. No window, no device.
// ---------------------------------------------------------------------------

/// The panel and the view, plus the words for what just happened.
struct Readout {
    panel: Panel,
    view: View,
}

impl Readout {
    fn new(width: f32, height: f32) -> Readout {
        Readout {
            panel: Panel::new(width, height),
            view: View::new(Room::Day),
        }
    }

    // -- the words ------------------------------------------------------

    fn label(&self, id: NodeId) -> String {
        match self.panel.layout().name(id) {
            Some(name) => name.to_owned(),
            None => "(unnamed split)".to_owned(),
        }
    }

    /// The two regions a boundary is between. A split is often unnamed — the
    /// console's body row is, deliberately — so `split #0` alone does not say
    /// which boundary the pointer has hold of, and the pair does.
    fn pair(&self, split: NodeId, index: usize) -> String {
        match self.panel.pair(split, index) {
            Some((a, b)) => format!("{} | {}", self.label(a), self.label(b)),
            None => "no pair".to_owned(),
        }
    }

    // -- input ----------------------------------------------------------

    fn press(&mut self, p: Point) {
        match self.panel.press(p) {
            Pressed::Grabbed {
                split,
                index,
                axis,
                at,
                offset,
            } => println!(
                "press ({:.0}, {:.0}): the boundary {} — divider #{} of {}, {:?} — is at \
                 {:.1}, grabbed {:+.1} from it",
                p.x,
                p.y,
                self.pair(split, index),
                index,
                self.label(split),
                axis,
                at,
                offset
            ),
            Pressed::NoPair { .. } => {
                println!("press ({:.0}, {:.0}): a divider with no pair", p.x, p.y)
            }
            Pressed::Region { id, rect } => println!(
                "press ({:.0}, {:.0}): region {} at {:.0},{:.0} {:.0}x{:.0}",
                p.x,
                p.y,
                self.label(id),
                rect.x,
                rect.y,
                rect.w,
                rect.h
            ),
            Pressed::Nothing => println!("press ({:.0}, {:.0}): nothing", p.x, p.y),
        }
    }

    fn moved(&mut self, p: Point) {
        let Some(dragged) = self.panel.moved(p) else {
            return;
        };
        println!("{}", self.say_drag(dragged));
    }

    /// A drag, in words: what was asked, where it landed, what held it, and
    /// what the pair either side is now.
    fn say_drag(&self, d: Dragged) -> String {
        let sizes = match self.panel.pair(d.split, d.index) {
            Some((a, b)) => format!(
                "{} {:.0} | {} {:.0}",
                self.label(a),
                d.axis.extent(self.panel.layout().rect(a)),
                self.label(b),
                d.axis.extent(self.panel.layout().rect(b))
            ),
            None => "no pair".to_owned(),
        };
        let stop = match d.held {
            Some(by) => format!(" — held {by:+.1} by a stop, and it stays there until it moves"),
            None => String::new(),
        };
        format!(
            "  drag: asked {:.1}, landed {:.1}{stop} [{sizes}]",
            d.asked, d.landed
        )
    }

    fn released(&mut self) {
        match self.panel.released() {
            Some(Released::Rests { split, index, at }) => {
                println!("release: {} rests at {at:.1}", self.pair(split, index))
            }
            Some(Released::Gone { split, index }) => {
                println!("release: {} is gone", self.pair(split, index))
            }
            None => {}
        }
    }

    /// Act, say what happened, and hand the outcome back — **the repaint
    /// decision is taken from what the operation did, not from the key that
    /// asked for it.** `p` over an empty panel and `z` with nothing folded
    /// both reach the model and move nothing.
    fn op(&mut self, op: Op) -> Outcome {
        let outcome = self.panel.op(op);
        self.say_op(op, &outcome);
        outcome
    }

    /// What an operation did, in words. The model returns the facts; which
    /// English they take is the operation that was asked for, which is why
    /// this has both.
    fn say_op(&self, op: Op, outcome: &Outcome) {
        match outcome {
            Outcome::Folded { id, folded, root } => {
                let what = match op {
                    Op::FoldEnclosing(_) => "the split ",
                    _ => "",
                };
                println!(
                    "fold: {what}{} is now {}{}",
                    self.label(*id),
                    folding(*folded),
                    match *root && *folded {
                        true => " — that was the root, so the panel is empty; z brings it back",
                        false => "",
                    }
                );
            }
            Outcome::Unfolded(ids) => match ids.is_empty() {
                true => println!("unfold: nothing is folded"),
                false => {
                    let names: Vec<String> = ids.iter().map(|id| self.label(*id)).collect();
                    println!("unfold: {}", names.join(", "));
                }
            },
            Outcome::Soloed(id) => println!(
                "solo: {} — everything else folded (soloed = {})",
                self.label(*id),
                self.panel.layout().is_soloed()
            ),
            Outcome::Unsoloed { was } => println!(
                "unsolo: {}",
                match was {
                    true => "the arrangement before the solo is back",
                    false => "nothing was soloed",
                }
            ),
            Outcome::Reset => println!("reset: a fresh arrangement, at the same viewport"),
            Outcome::Report(rows) => {
                println!("regions:");
                let lines: Vec<String> = rows
                    .iter()
                    .map(|row| {
                        format!(
                            "  {:width$}{:<18} {:>7.1},{:>7.1}  {:>7.1} x {:>7.1} {}",
                            "",
                            self.label(row.id),
                            row.rect.x,
                            row.rect.y,
                            row.rect.w,
                            row.rect.h,
                            match row.state {
                                Visibility::Folded => "folded",
                                Visibility::InsideAFold => "inside a fold",
                                Visibility::Visible => "",
                            },
                            width = row.depth * 2
                        )
                    })
                    .collect();
                println!("{}", lines.join("\n"));
            }
            // One operation can still find nothing to act on, and it is not
            // about the pointer: the root has no split enclosing it. *Nothing
            // under the pointer* is said by `key`, before an operation is
            // named at all — see `Panel::under`.
            Outcome::Nothing => {
                println!("fold: that is the root, and nothing encloses it")
            }
        }
    }

    fn room(&mut self) {
        self.view.room = self.view.room.other();
        println!("room: {}", self.view.room.word());
    }

    /// **The one place a pointer event is routed**, and the only place this
    /// file decides anything about input.
    ///
    /// Returns who the event belonged to. The caller's whole job with the
    /// answer is to hand the event to `egui` when it is [`Claim::Egui`] and
    /// not when it is not — see `karakuri_console::input` for the rule and
    /// for why it is written there rather than here.
    ///
    /// It is a method rather than four arms in `window_event` so that a
    /// gesture can be driven without a window: `winit` cannot be asked for an
    /// `ActiveEventLoop` outside its own loop, so an event handler is not
    /// something a test can call, and the part worth testing is this.
    fn pointer(&mut self, ctx: &egui::Context, event: Pointer) -> (Claim, Option<Outcome>) {
        let at = match event {
            Pointer::Moved(p) => p,
            _ => self.panel.cursor(),
        };
        // Asked **before** anything acts. `released` takes the drag out of
        // hand, so a claim asked after it would see no drag, route the release
        // to `egui`, and hand `egui` a button-up it never saw the button-down
        // for.
        let claim = claim(&mut self.panel, ctx, at);
        let mut did = None;
        match (event, claim) {
            // The panel learns where the pointer is either way — every
            // keyboard operation is addressed to it — and drags if a boundary
            // is in hand. Whether `egui` is also told is the claim.
            (Pointer::Moved(p), _) => self.moved(p),
            // **A press the panel claimed is either on a control or on the
            // panel itself**, and the control is asked first for the reason
            // `claim` asked it last: rule 2 has already had its refusal, so a
            // press that got here and is on the chip is the chip's. It is the
            // same `outputs` call `claim` made — asked again, not copied.
            (Pointer::Down, Claim::Panel) => {
                self.panel.solve();
                match outputs(ctx, self.panel.layout()).filter(|row| row.hit(at)) {
                    Some(row) => did = Some(self.sink(row.op())),
                    None => self.press(at),
                }
            }
            (Pointer::Up, Claim::Panel) => self.released(),
            (Pointer::Down | Pointer::Up | Pointer::Wheel, _) => {}
        }
        (claim, did)
    }

    /// A press on the Outputs row's one control. **The dot says what it did**
    /// — which of the two operations it asked for, and what the picture is
    /// now — because the whole point of the control is that it is the same
    /// fold `f` over the picture performs, reached from the other end of the
    /// panel.
    fn sink(&mut self, op: Op) -> Outcome {
        // Two operations and no third, which is `Outputs::op`'s whole
        // argument: the toggle is the dot choosing between them, and what
        // arrives here is one of the two by name.
        println!(
            "outputs: program view — {}",
            match op {
                Op::Fold(_) =>
                    "the sink was on, so the picture folds away and the \
                                inspector takes its height",
                Op::Unfold(_) =>
                    "the sink was off, so the picture comes back — with \
                                  whatever was folded over it",
                other => unreachable!("the dot asked for {other:?}"),
            }
        );
        self.op(op)
    }

    /// **What the pointer is over, resolved to a target for a key press.**
    ///
    /// The half of an operation that used to be inside it: `f` means *fold*
    /// and the pointer is how this surface says *which*. A key that lands on
    /// a divider or outside every region names nothing, so nothing is emitted
    /// — and the two sentences that used to be [`Outcome`]s are said here,
    /// where the resolution failed, rather than by a model that was asked to
    /// fold something nobody had named.
    fn target(&mut self, key: &str) -> Option<NodeId> {
        match self.panel.under() {
            Hit::View(id) => Some(id),
            Hit::Divider { split, index } => {
                println!(
                    "{key}: the pointer is on divider {}#{index} — move it into a region",
                    self.label(split)
                );
                None
            }
            Hit::Nothing => {
                println!("{key}: nothing under the pointer");
                None
            }
        }
    }

    /// `g`'s target, which is the one resolution with two answers: **a
    /// divider already names its split**, so over a gap the split to fold is
    /// that one and the operation is a plain [`Op::Fold`] of it, while over a
    /// region it is [`Op::FoldEnclosing`] and the model reads the parent.
    ///
    /// Both arms were inside `Op::FoldEnclosing` when an operation meant
    /// *whatever is under the pointer*; they are the same two arms, out where
    /// the pointer is.
    fn enclosing(&mut self) -> Option<Op> {
        match self.panel.under() {
            Hit::View(id) => Some(Op::FoldEnclosing(id)),
            Hit::Divider { split, .. } => Some(Op::Fold(split)),
            Hit::Nothing => {
                println!("g: nothing under the pointer");
                None
            }
        }
    }

    // -- the legend -----------------------------------------------------

    fn print_legend(&mut self, budget_ms: Option<f32>) {
        self.panel.solve();
        let layout = self.panel.layout();
        let viewport = layout.viewport();
        println!();
        println!(
            "the console, in a {:.0} x {:.0} viewport. every leaf gets its region; seven of \
             them get a bay head, and every body is empty but the Program bay's two: the \
             picture is a live engine frame, and deck A is auditioning in the first of the \
             four preview cells under it.",
            viewport.w, viewport.h
        );
        println!(
            "B, C and D read `off` because this example's engine is a deck of ONE slot — \
             there is no second deck to audition, so three cells saying off are what this \
             program is rather than something left unfinished. each cell that is on costs \
             a present pass of its own."
        );
        println!(
            "the transport row reads the session's own oscillator — the tempo, the beat \
             inside the bar, and the bar counted from one — beside what the last frame \
             cost{}. the four dots are `karakuri_signal`'s BEATS_PER_BAR, which that \
             crate calls a provisional assumption of common time, so the console takes \
             the number a frame rather than assuming four.",
            match budget_ms {
                Some(budget) => format!(
                    ", against this display's refresh interval of {budget:.1} ms — which is \
                     the budget a frame is held to on a Fifo surface, and not the 20 ms a \
                     candidate Set is held to"
                ),
                // Said rather than passed over: a reading nobody can take is
                // worth a sentence, because the alternative is a reader
                // wondering where the mock's `/16.6` went.
                None => String::from(
                    " — with no budget beside it, because winit will not say what this \
                     display's refresh rate is"
                ),
            }
        );
        println!(
            "six things the mock draws in that row are NOT drawn, and each is a control \
             over machinery that is in neither this crate nor this example: audio-in, tap, \
             learn, map, landed and rec. `view::transport` names them one by one with what \
             is missing behind each."
        );
        println!(
            "the mixer draws {} strip{}, because a strip is a deck SLOT and this deck has \
             {} — the mock's four is the most a deck can hold, and the page keeps its four \
             tracks either way, so a track with nothing behind it is empty rather than a \
             strip full of dashes. every strip is a READOUT: the residency, the trim, the \
             fader, the meter, the opacity and the two modes are what the deck says, and a \
             press on any of them reaches nothing yet.",
            self.view.mixer.len(),
            match self.view.mixer.len() {
                1 => "",
                _ => "s",
            },
            self.view.mixer.len()
        );
        println!(
            "the crossfade row under the strips is NOT drawn — an A/B track, wipe, iris, \
             next bar, 8 beats and go are a transition being armed and fired, and nothing \
             here holds what is armed. nor are the two focuses: the deck selection and \
             keyboard focus must not look alike, and this console keeps neither."
        );
        println!();
        for node in self.panel.nodes() {
            let (min, max) = layout.bounds(node.id);
            let bounds = format!(
                "min {min:.0}, max {}",
                match max.is_finite() {
                    true => format!("{max:.0}"),
                    false => "none".to_owned(),
                }
            );
            let what = match layout
                .name(node.id)
                .and_then(karakuri_console::view::region)
            {
                Some(region) => match region.kind {
                    Kind::Bay { grip: true, .. } => "bay, with a grip".to_owned(),
                    Kind::Bay { .. } => "bay".to_owned(),
                    // The other row with something in it, and everything in
                    // it is a readout: the six controls the mock draws here
                    // are six things that do not exist behind this panel, and
                    // `view::transport` names each of them.
                    Kind::Transport => "row, no heading: bpm, beat, bar, frame".to_owned(),
                    // The one row with something in it: the console's first
                    // control, and the only thing on the panel a press acts
                    // on that is not a boundary.
                    Kind::Outputs => "row, one sink: program view".to_owned(),
                    Kind::Pane => "pane, inside a bay".to_owned(),
                    Kind::Picture => "the picture, a sink".to_owned(),
                    // Four cells, and this file knows which of them are on:
                    // one slot in the deck, so deck A and no other.
                    Kind::Previews => "four previews, A live".to_owned(),
                    // A bay like the other six, and then one strip: this
                    // deck has one slot, so there is one thing to mix.
                    Kind::Mixer => format!(
                        "bay, {} strip{}",
                        self.view.mixer.len(),
                        match self.view.mixer.len() {
                            1 => "",
                            _ => "s",
                        }
                    ),
                },
                None => match layout.axis(node.id) {
                    Some(Axis::Row) => "split, left to right".to_owned(),
                    Some(Axis::Column) => "split, top to bottom".to_owned(),
                    None => "not drawn".to_owned(),
                },
            };
            println!(
                "  {:width$}{:<16} {:<22} {}",
                "",
                self.label(node.id),
                what,
                bounds,
                width = node.depth * 2
            );
        }
        println!();
        println!(
            "the outputs row's dot is the one control on the panel: it folds the picture by \n\
             name, so clicking it and pressing f over the picture are the same operation \n\
             reached from two surfaces. it is lit while the picture is on screen."
        );
        println!();
        println!("keys — the pointer's position decides what each one acts on:");
        println!("  click    the outputs dot: turn the program view sink off, and on again");
        println!("  drag     press the left button in a gap and move: the boundary follows");
        println!("  f        fold the region under the pointer");
        println!("  g        fold the split enclosing the region under the pointer");
        println!("  z        unfold everything folded (a folded region has no rectangle, so");
        println!("           the pointer cannot reach it to unfold it)");
        println!("  s        solo the region under the pointer");
        println!("  u        undo the solo");
        println!("  r        reset to a fresh arrangement");
        println!("  p        print every region's rectangle");
        println!("  n        the room: day or night");
        println!("  esc      quit");
        println!();
    }
}

/// A pointer event, stripped to what the rule needs. A button is left or it
/// is not routed at all, and which wheel axis it was does not change who gets
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Pointer {
    Moved(Point),
    Down,
    Up,
    Wheel,
}

fn folding(folded: bool) -> &'static str {
    match folded {
        true => "folded",
        false => "unfolded",
    }
}

// ---------------------------------------------------------------------------
// The engine in the Program bay
// ---------------------------------------------------------------------------

/// The canvas: what the Set renders at, and what the deck is sized to.
///
/// **The workspace's reference workload**, 262144 elements at 1280x720
/// (`docs/contributing.md` §1), and it is that on purpose rather than by
/// default: this is the one place in the console where a number is about the
/// engine rather than about the panel, and a number taken at the reference
/// workload can be put beside every other one in this repository. **Not the
/// size of the picture, and not the size of a preview cell either** — see
/// [`Present::draw`], which letterboxes this into whatever it is drawn into
/// and is handed two rectangles of different sizes a frame, and which is what
/// the manual means by *"it letterboxes into the width it has"*.
///
/// **It is also the shape the picture is now given**, through
/// [`aims`] and `picture_rect`: the console sizes the picture's rectangle to
/// this rather than to whatever the region happens to be, so the two targets
/// `Present::draw` is handed are both the canvas's shape and its fit into each
/// is sub-texel. The number reaches `picture_rect` from `Present::size` rather
/// than from this constant, so the shape the console draws and the canvas the
/// engine renders cannot come from two places — see [`aims`].
const CANVAS: (u32, u32) = (1280, 720);

/// `drift_shell.kir`'s own `capacity [4096, 1048576] = 262144`, written here
/// because `Set::build` takes a number rather than reading the `.kir`'s
/// default. The same number `karakuri-cli` runs by default, for the reason
/// above.
const CAPACITY: u32 = 262144;

/// The seed salt, which decides where the elements start. Any value is a
/// picture; 7 is the one `karakuri-cli`'s own tests use, so this looks like
/// what they look like.
const SEED_SALT: u32 = 7;

/// **The two `.kir` files the one Set is built from**, named once so that what
/// is loaded and what the mixer strip is called cannot drift apart. See
/// [`MATERIAL`].
const L1_KIR: &str = "examples/drift_shell.kir";
const L4_KIR: &str = "examples/soft_points.kir";

/// **What the mixer strip calls what this deck is playing**, and it is this
/// file's word rather than the engine's.
///
/// `view::Strip::name` says why there is no other answer: nothing reachable
/// from a `Deck` carries a name for the material in a slot. A `Set` names its
/// *nodes* and its *published controls* and has no name of its own, which is
/// right — a Set is built from a list of `.kir` files, and only whoever passed
/// that list knows what to call the result. **This is that list**, derived
/// from the two paths above rather than typed again, so a strip cannot go on
/// saying `drift_shell` after somebody loads something else.
fn material() -> String {
    let stem = |path: &str| {
        std::path::Path::new(path)
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
    };
    format!("{} + {}", stem(L1_KIR), stem(L4_KIR))
}

/// **One simulation step per frame drawn, and no clock anywhere.**
///
/// [P-0002](../../../docs/principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md)
/// says simulation time comes from a record and never from a clock. There is
/// no record here — this is a panel harness, not a session — so the honest
/// third option is neither: a fixed count per frame, which makes the picture's
/// motion a function of frames drawn and of nothing else. It is not a
/// performance, and a `karakuri-cli` that measured an interval and wrote a
/// `tick` is what a performance is.
const STEPS_A_FRAME: u8 = 1;

/// **The look every sink is drawn under**, and it is exactly what
/// [`Present::new`] uploads before anybody calls `set_tonemap`.
///
/// [`compose`] writes the tone-map uniform on every frame from the
/// [`Committed`] the closure hands back, so a harness that has no operator on
/// a key still has to say what the look is. Saying *what `Present` already
/// defaults to* is the honest answer here: this file has no session, no `look`
/// record and no key that changes it, so a different value would be this
/// example inventing an aesthetic the rest of the workspace does not run
/// under.
const LOOK: Look = Look {
    op: TonemapOp::Aces,
    exposure: 1.0,
    white_point: 1.0,
};

/// What the picture and every preview are rendered in: an sRGB format, so the
/// hardware does the one encode `Present`'s shader relies on ([P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)).
const PICTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// **The same texels, read as if they were not sRGB, which is how `egui` wants
/// them.**
///
/// `egui`'s fragment shader says so outright — *"We expect 'normal' textures
/// that are NOT sRGB-aware"* — and multiplies the sample by the vertex tint in
/// gamma space. A view in [`PICTURE_FORMAT`] would have the hardware decode to
/// linear on the way in and `egui` would then write those linear values into a
/// gamma-space surface, which is a picture that comes out visibly dark with no
/// error anywhere. So the render target is sRGB, the sampled view is this, and
/// the encode still happens exactly once — in the present pass.
const PICTURE_SAMPLED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// What the view `egui` samples is called on the device. The **texture**
/// carries the name that says which of the two it is — see
/// [`Presented::label`] — and this says which view of that texture it is, so
/// the pair reads as one thing in a validation message rather than as two with
/// the same name.
const SAMPLED_LABEL: &str = "as egui reads it";

/// **A texture the engine presents into and the panel samples**: the texture
/// itself, the view [`Present::draw`] draws into, the registration `egui`
/// reads it by, and its size in physical pixels.
///
/// **Two call sites on the day it is written**, which is this repository's
/// rule about an abstraction and is the same standing
/// [`karakuri_console::view::bay_head`] has: the Program bay's picture, and
/// deck A's preview cell under it. The four fields were `Engine`'s own until
/// the second one needed them, and every argument written on them then is
/// written on them here, because all of it is still true of both.
///
/// # It is a [`Sink`], and the aiming is the half a `Sink` cannot carry
///
/// [`Sink::acquire`] is handed a `&Gpu` and nothing else, and sizing one of
/// these needs the region's rectangle, the scale factor and the
/// [`egui_wgpu::Renderer`] the registration lives in. So the frame does that
/// first, through [`Presented::aim`], and `acquire` answers from what it was
/// aimed at: a rectangle means a target, no rectangle means [`Skip::Transient`]
/// and nothing at all is drawn into it.
///
/// **The split is where it is because of what a test can call.** Deciding
/// *which rectangle, at what size* inside `window_event` is deciding it
/// somewhere `winit` will not let a test reach — and sizing deck A's texture
/// from the picture's rectangle was injected there once and every test still
/// passed. [`aims`] and [`Presented::aim`] are that decision, whole, outside
/// the event handler; `mod gpu` calls them the way the frame does.
///
/// **Neither of these is presented anywhere**, which is why
/// [`Sink::present`] is `Ok(())` for both: the picture and the preview cell
/// are textures the panel samples, and the thing that reaches a display is the
/// window — which is not a sink here. `karakuri-cli`'s window shows the
/// canvas; this window shows the panel, and the panel goes in `finally`.
struct Presented {
    /// The texture. Held because the views below are of it, and because
    /// freeing the `egui` registration does not free this — `egui-wgpu` stores
    /// a bind group for a registered native texture and no texture at all, so
    /// dropping this is the only thing that releases the memory. Read only by
    /// `mod gpu`, which asks it what size it came out, and that is the whole
    /// of why the lint has to be told: the window does not read it and the
    /// window is not what it is for.
    #[allow(dead_code)]
    texture: wgpu::Texture,
    /// What [`Present::draw`] draws into: sRGB, so the encode is the
    /// hardware's.
    target: wgpu::TextureView,
    /// The registration `egui` draws by, of a view in
    /// [`PICTURE_SAMPLED_FORMAT`].
    id: egui::TextureId,
    /// The texture's size in physical pixels — **its region's**, not the
    /// window's.
    size: (u32, u32),
    /// What the texture is called on the device. Kept rather than passed to
    /// [`Presented::fit`], so the texture a resize makes is called what the
    /// one it replaces was called; and its own per texture, so a device
    /// message about the preview does not read as one about the picture.
    label: &'static str,
    /// **Whether this sink has a rectangle on screen this frame**, written by
    /// [`Presented::aim`] and read by nothing but [`Sink::acquire`].
    ///
    /// It is a field rather than an argument because the two questions are
    /// asked at different moments and by different callers: the frame aims
    /// every sink before it composes, and `compose` then asks each one for
    /// itself. A `Presented` that has never been aimed answers no, which is
    /// the right answer — it has a 1x1 placeholder texture and nothing has
    /// said where it goes.
    aimed: bool,
}

impl Presented {
    /// **A sink aimed at the rectangle its region has**, or at nothing where
    /// the region is folded away.
    ///
    /// It goes through [`Presented::aim`] rather than sizing the texture here,
    /// so that *which rectangle, at what size* has exactly one derivation in
    /// this file and the window's first frame cannot disagree with the window
    /// it opened at. The 1x1 below is never drawn into and never on screen: it
    /// is what `aim` replaces on the same statement.
    ///
    /// The `freed` it counts into is discarded, and that is the one place in
    /// this file where it may be. [`Engine::freed`] is a tally of registrations
    /// leaked by a **resize**, and this is construction — a caller that saw a
    /// 1 here would be told a leak had already happened before the window drew.
    fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        label: &'static str,
        at: Option<egui::Rect>,
        scale: f32,
    ) -> Presented {
        let mut presented = Presented::made(gpu, renderer, label, (1, 1));
        let mut construction = 0;
        presented.aim(gpu, renderer, at, scale, &mut construction);
        presented
    }

    /// The texture, the view the engine draws into, and the registration
    /// `egui` reads it by, at a size in physical pixels. One constructor
    /// because the three are made together and are replaced together, and
    /// private to this type because a caller aims at a rectangle — turning one
    /// into a size is [`Presented::aim`]'s and [`Presented::fit`]'s alone.
    fn made(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        label: &'static str,
        size: (u32, u32),
    ) -> Presented {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PICTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            // Declared so the sampled view below may reinterpret it — the two
            // formats differ only in whether the transfer function is applied.
            view_formats: &[PICTURE_SAMPLED_FORMAT],
        });
        let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampled = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(SAMPLED_LABEL),
            format: Some(PICTURE_SAMPLED_FORMAT),
            ..Default::default()
        });
        let id = renderer.register_native_texture(&gpu.device, &sampled, wgpu::FilterMode::Linear);
        Presented {
            texture,
            target,
            id,
            size,
            label,
            aimed: false,
        }
    }

    /// **Where this sink goes this frame, and how big its texture therefore
    /// is** — one statement, and it is the whole of what [`Sink::acquire`]
    /// then answers from.
    ///
    /// Returns what the console should draw, or `None` where there is no
    /// rectangle. **The returned [`Picture`] carries the same rectangle the
    /// texture was just sized from and the id the sizing may have just
    /// replaced**, which is the pairing `karakuri_console::view::Picture`'s own
    /// documentation asks for: whoever sized the texture and whoever placed it
    /// are one statement, so a texture sized from the window and drawn into the
    /// picture's region cannot be written by accident, and a resize cannot
    /// leave a freed id in the view.
    ///
    /// **Called before [`compose`], and it has to be**: `acquire` takes only a
    /// `&Gpu`, and this needs the rectangle, the scale factor and the
    /// renderer. It is also a reallocation on the frames a size changed, so it
    /// belongs at the top of the frame rather than mid-pass — see
    /// [`Presented::fit`].
    fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        at: Option<egui::Rect>,
        scale: f32,
        freed: &mut usize,
    ) -> Option<Picture> {
        self.aimed = at.is_some();
        // **No rectangle, so nothing to fit.** The texture is left exactly as
        // it was rather than shrunk: a folded region is one an operator
        // unfolds, and remaking it small and large again would put two
        // reallocations on a fold that costs none. Nothing is drawn into it
        // meanwhile — `acquire` refuses, which is the whole of what a fold
        // has to mean.
        let rect = at?;
        self.fit(gpu, renderer, physical(rect, scale), freed);
        Some(Picture { id: self.id, rect })
    }

    /// **The region is a different size, so the texture is remade at that size
    /// and the registration it replaces is freed.**
    ///
    /// Returns whether anything was remade, which is `false` on all but a
    /// handful of frames — every frame of a drag on the program's height is
    /// one of them, and that is exactly the case the free is for. A
    /// `register_native_texture` per dragged frame with no `free_texture`
    /// beside it is a bind group and a sampler leaked per frame, for as long
    /// as somebody keeps hold of a divider.
    ///
    /// A reallocation, so it is the frame's first act and not something done
    /// mid-pass ([P-0001](../../../docs/principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md)
    /// is about the render thread, and this is that thread; what it costs is
    /// paid on the frames a size changed and on no others).
    ///
    /// **`freed` is handed in rather than kept here** because the tally is the
    /// whole engine's — one number over both textures, which is what
    /// [`Engine::freed`] is and what `mod gpu` asserts on. Taking it as an
    /// argument is what makes it impossible for a call site to remake a
    /// texture and forget to count what it freed.
    fn fit(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        size: (u32, u32),
        freed: &mut usize,
    ) -> bool {
        if size == self.size {
            return false;
        }
        // Freed **before** the replacement is registered, which is the order
        // the leak is about rather than a tidiness: the atlas holds the old
        // bind group until this call and nothing else ever drops it.
        renderer.free_texture(&self.id);
        *freed += 1;
        // Carried across, because a resize is not an un-aiming: the rectangle
        // this was aimed at is the one it was just resized to.
        let aimed = self.aimed;
        *self = Presented::made(gpu, renderer, self.label, size);
        self.aimed = aimed;
        true
    }
}

/// **The two textures are sinks, and this is the whole of what that costs.**
///
/// `acquire` answers from [`Presented::aim`] and nothing else; `present` is
/// `Ok(())` because neither of these is presented anywhere — they are sampled
/// by the panel, and the panel is drawn in [`compose`]'s `finally` rather than
/// by a sink. See the type's own documentation.
impl Sink for Presented {
    fn acquire(&mut self, _gpu: &Gpu) -> Result<(), Skip> {
        match self.aimed {
            true => Ok(()),
            // **[`Skip::Transient`] and never a `Fault`.** A folded region is
            // an operator's choice and the next `f` over it undoes it, so
            // there is nothing to say about it — and a `Fault` here would be
            // a line printed the first frame of every fold.
            false => Err(Skip::Transient),
        }
    }

    fn view(&self) -> &wgpu::TextureView {
        &self.target
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    /// **Nothing.** The picture and deck A's cell are textures the panel
    /// samples in the same submission; what reaches a display is the window,
    /// and the window is not a sink here — `karakuri-cli`'s window shows the
    /// canvas, and this one shows the panel.
    fn present(&mut self, _gpu: &Gpu) -> Result<(), String> {
        Ok(())
    }
}

/// **The engine behind the Program bay: a deck of one Set, the present pass,
/// and the two textures it lands in.**
///
/// Scaffolding, and it looks it: one slot, no audio, no MIDI, no store, no
/// arguments. What `karakuri-cli` does around this is a program; what is here
/// is the shortest path from two `.kir` files to texels, which is the whole of
/// what the Program bay needs to be shown to be reachable.
///
/// **One slot is also why three of the four preview cells are off.** A deck
/// holds up to four and this one holds one, so deck A is the only audition
/// there is to show: B, C and D have nothing behind them, and three cells
/// reading `off` are the truth about this example rather than a gap in it.
struct Engine {
    deck: Deck,
    present: Present,
    /// The Program bay's picture.
    picture: Presented,
    /// **Deck A's preview cell**, and the second target the one [`Present`]
    /// serves.
    ///
    /// [`Present::draw`] letterboxes the canvas into whatever target size it
    /// is handed — it takes the size as an argument and fits to it — so a
    /// second target of a different size costs one extra pass and no extra
    /// state: no second `Present`, no second canvas, no second deck render.
    /// That is the manual's *"each preview is an audition and costs a pass"*,
    /// made literally true.
    ///
    /// **Both targets are the canvas's shape now**, the picture's through
    /// `picture_rect` and this one through the mock's own 16:9 cell, so
    /// neither of the two fits has real work to do and neither texture carries
    /// a bar wider than a texel. What the fit still earns is the rounding: see
    /// `karakuri_console::view::picture_rect`, and
    /// `the_picture_is_the_canvass_shape_and_carries_no_bars` under `mod gpu`.
    preview: Presented,
    /// How many registrations have been freed, **over both textures**. The
    /// atlas leak this exists to prevent is invisible from outside: a resize
    /// that registers without freeing leaves a bind group per drag frame and
    /// nothing says so, so the count is kept and `mod gpu` asserts on it. It
    /// is the whole engine's tally rather than either texture's, which is why
    /// it lives here and is handed to [`Presented::fit`].
    freed: usize,
}

impl Engine {
    /// **Sized from the arrangement rather than from the window**, by the same
    /// two calls the frame aims with — see [`aims`]. The window this opens at
    /// gives the picture and deck A's cell their first rectangles, so no frame
    /// has to correct a guess and there is no second derivation here to drift
    /// from the one in [`Engine::aim`].
    fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        layout: &karakuri_layout::Layout,
        scale: f32,
    ) -> Engine {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let l1 = checked(&root.join(L1_KIR));
        let l4 = checked(&root.join(L4_KIR));
        let set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, CAPACITY, SEED_SALT)
            .expect("the example pair builds a Set");
        let mut deck = Deck::new(&gpu.device, vec![HotSwap::fixed(set)], CANVAS.0, CANVAS.1);
        // **The meters are on, and that is a decision rather than a default.**
        // Five of the six things a mixer strip shows are settings the deck was
        // told; the meter is the only one that is a *measurement*, so with it
        // off this bay would draw five readouts that never move beside a well
        // that is always empty — which is the scaffolding-that-looks-finished
        // this panel refuses, read from the other side. It costs a pipeline,
        // two buffers and a ring of staging buffers per slot, allocated here
        // and never on the render thread, which is the same terms `Deck::new`
        // above is on; there is one slot, so it is one of each.
        deck.enable_meters(&gpu.device);
        let present = Present::new(&gpu.device, PICTURE_FORMAT, CANVAS.0, CANVAS.1);
        let [picture, preview] = aims(layout, present.size());
        Engine {
            deck,
            present,
            picture: Presented::new(gpu, renderer, "program view", picture, scale),
            // Named for the deck it is of, because there is one of these per
            // audition and a device message that says `deck preview` four
            // times over says nothing.
            preview: Presented::new(gpu, renderer, "deck A preview", preview, scale),
            freed: 0,
        }
    }

    /// **Aim every sink at its own rectangle, and hand back what the console
    /// should draw in each** — the picture, and one entry per preview cell.
    ///
    /// This is *which rectangle, at what size* for the whole engine, in one
    /// call that takes a solved layout and a scale factor and touches no
    /// window. That is the point of it: the same decision used to live inside
    /// `window_event`, which `winit` will not let a test call, so **sizing
    /// deck A's texture from the picture's rectangle was injected there and
    /// every test still passed**. `mod gpu` calls this the way the frame does.
    ///
    /// **B, C and D stay `None`, and that is this example rather than a gap in
    /// it**: the deck above has one slot, so deck A is the only audition there
    /// is and there is no second one to put in a cell. An empty cell is what
    /// off looks like, and the console draws it saying `off`.
    fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        layout: &karakuri_layout::Layout,
        scale: f32,
    ) -> (Option<Picture>, [Option<Picture>; DECKS]) {
        let [picture_at, preview_at] = aims(layout, self.present.size());
        let picture = self
            .picture
            .aim(gpu, renderer, picture_at, scale, &mut self.freed);
        let mut previews = [None; DECKS];
        previews[0] = self
            .preview
            .aim(gpu, renderer, preview_at, scale, &mut self.freed);
        (picture, previews)
    }
}

/// **Which rectangle each of the engine's sinks is sized from and drawn into,
/// and there is no second answer to it anywhere in this file.**
///
/// In sink order: the Program bay's picture, and deck A's preview cell. `None`
/// is a region folded away, a bay folded, or the picture soloed — the sink
/// then has no target and [`Sink::acquire`] refuses, so no present pass is
/// recorded for it. That is the manual's *"it is on screen exactly when that
/// sink is on — so there is no state where it is hidden and still costing a
/// pass"*, and it is now `compose`'s doing rather than an `if` in this file.
///
/// A function rather than two lines in [`Engine::aim`] for the reason
/// [`live`] is a function: it is the statement that has actually been got
/// wrong, and it is worth being somewhere a test can hold it on its own.
///
/// **`canvas` is asked of the `Present` rather than read off [`CANVAS`]**, and
/// that is the pairing rather than a preference: the picture's rectangle is
/// now the canvas's shape, and the canvas it has to be the shape *of* is the
/// one `Present::draw` is fitting **from**. Reading the constant here would be
/// a second copy of that number, and the frame it is wrong on is one where the
/// picture is the shape of a canvas nothing is rendering at — which is a
/// letterbox nobody asked for and nothing on screen names.
fn aims(layout: &karakuri_layout::Layout, canvas: (u32, u32)) -> [Option<egui::Rect>; 2] {
    [
        picture_rect(layout, canvas),
        // The **cell**, not the row it is in and not the region the row is
        // in. The row holds four of these side by side with ground between
        // them, so a texture sized from anything but the cell is out by a
        // factor of four in one axis before it is out by the scale — and
        // beside the picture the row is not a row at all, which is why this
        // takes the canvas too: the cells' arrangement is decided by which one
        // leaves the *picture* larger, so the cell's own size is a function of
        // the picture's shape.
        preview_rects(layout, canvas).map(|cells| cells[0]),
    ]
}

/// **Is anything making texels this frame?** — which is the whole of what
/// decides whether the loop asks for another frame.
///
/// **The rule, rather than the expression: anything that makes texels this
/// frame keeps the loop awake, and the list is closed.** Every sink the engine
/// draws into is read here — the picture and all [`DECKS`] preview cells — so
/// a fifth sink added later and not added to this is the same bug again, and
/// it is the bug this file has already shipped once: `live` was the picture
/// alone, so folding the picture away left deck A auditioning under it while
/// the loop stopped asking for frames. It fails in whichever direction the
/// mistake is made — a window that goes on drawing what nobody asked for, or a
/// panel that keeps changing while the loop sleeps.
///
/// A function rather than an expression in the frame path for the reason
/// [`Readout::pointer`] is a method: `window_event` cannot be called from a
/// test, so the part worth asserting is lifted out to where a test can reach
/// it — see `anything_that_makes_texels_keeps_the_loop_awake`.
fn live(view: &View) -> bool {
    view.picture.is_some() || view.previews.iter().any(Option::is_some)
}

/// **What the transport row reads this frame**, out of the two things in this
/// file that know: the deck's oscillator, and what the last frame cost.
///
/// `Transport` here is `karakuri_console::view::Transport` — the console's row
/// of readouts — and not `karakuri_engine::transport::Transport`, which is a
/// slot's sync mode and is a different thing with the same word on it. This
/// takes a `&Deck` because it is on the side of the seam that is allowed one;
/// what crosses into the console is six numbers (ADR-0156).
///
/// # Where each number comes from, and that nothing is measured twice
///
/// - **The tempo, the position and the grid.** `Deck::signals` is the
///   session's one oscillator — the same one every binding reads — and
///   `Oscillator::bpm` and `Oscillator::beats` are its tempo and its musical
///   position. `beats` is unbounded and monotone, so which dot is lit and
///   which bar it is are arithmetic on it and the console does that
///   arithmetic. The deck advances it inside `render`, one step a frame
///   (`STEPS_A_FRAME`), so this reads the position as of the end of the last
///   frame.
/// - **The frame's cost.** `Cost::whole` — the same three fields the reading
///   sums under *"the whole frame is a median"*, for the frame just drawn.
///   Nothing is timed twice: `Costs::push` kept the last `Cost` and this
///   divides nothing.
/// - **The rate.** `Costs::rate_now`, which is the reading's own `rate`
///   asked before its deadline rather than at it.
/// - **How many beats a bar has.** `karakuri_signal::oscillator::BEATS_PER_BAR`, which is
///   where the deck's own grid gets it, and which says of itself that it is
///   provisional until the IR format carries a time signature. Asked rather
///   than transcribed, so that the day it stops being 4 the beat grid stops
///   being four dots.
///
/// `None` before the first frame has been drawn, which is one frame of a run:
/// there is no frame cost yet, and a row that made one up would be inventing
/// exactly the reading this whole seam exists to refuse.
///
/// **The rate is `None` unless something is live**, and that is not caution
/// either. With nothing making texels this loop stops asking for frames, so
/// the last rate it measured would sit in the row describing a window that has
/// stopped drawing — the one number here that goes stale by standing still.
/// The frame time beside it does not: the last frame did cost that, whenever
/// it was.
fn transport(
    deck: &Deck,
    costs: &Costs,
    budget_ms: Option<f32>,
    live: bool,
) -> Option<view::Transport> {
    let last = costs.last?;
    let grid = deck.signals().oscillator();
    Some(view::Transport {
        bpm: grid.bpm(),
        beats: grid.beats(),
        beats_per_bar: karakuri_signal::oscillator::BEATS_PER_BAR,
        fps: live
            .then(|| costs.rate_now())
            .flatten()
            .map(|rate| rate as f32),
        frame_ms: ms(last.whole()) as f32,
        budget_ms,
    })
}

/// **What the mixer strips read this frame**: one per slot the deck has, out
/// of the six things a `Deck` will say about a slot.
///
/// Everything here is reachable from a `Deck` and nothing reaches around one:
/// `slot_count`, `residency`, `gain`, `opacity`, `blend`, `mask` and `level`
/// are its own, and this takes a `&Deck` because it is on the side of the seam
/// that is allowed one — what crosses into the console is a name, a word and
/// four numbers (ADR-0156).
///
/// # Where each one comes from, and the two that are not the deck's
///
/// - **The tally** is `Deck::residency`, which is the **effective** residency
///   the frame loop reads and not `requested_residency`. The governor moves a
///   slot down without anybody asking, and a tally showing the request would
///   be describing a slot that is doing something else.
/// - **The trim and the fader** are `gain` and `opacity`, which are two
///   controls and not one — *"opacity at zero silences under every blend mode,
///   gain at zero does not silence `over`"* — and the bay draws them as two.
/// - **The blend** is `Blend::name`, asked rather than transcribed for the
///   reason the beat grid asks for `BEATS_PER_BAR`: the mock's own tooltip
///   lists four blends and `Blend::ALL` is three, so a word written here would
///   be this file's opinion about the engine's list.
/// - **The mask** is `Deck::mask(slot).kind()`, and its angle, position and
///   softness are left behind: the strip's `.mini` says *which shape*, and
///   three numbers about that shape are an inspector row.
/// - **The level** is `Deck::level`, which is already `None` for every case
///   where a held reading would be about a different image. Its
///   `frames_behind` is not passed on — `view::Level` is where that argument
///   is written out, and the short of it is that the number means different
///   things on different loops and this loop is a `Fifo` one that never stops
///   asking for frames while anything is live.
/// - **The name** is [`material`], and it is this file's because a `Set` has
///   none. See `view::Strip::name`.
///
/// # Written into the `Vec` the view already holds
///
/// `out` is grown to the deck's slot count and then every field of every strip
/// is written, so nothing a `push` left behind is ever read. The name is the
/// one field that owns anything, and it is rewritten only when it differs —
/// which keeps this off the frame's allocation budget (ADR-0164) rather than
/// putting a `String` per strip on it every frame.
fn mixer(deck: &Deck, name: &str, out: &mut Vec<view::Strip>) {
    out.truncate(deck.slot_count());
    while out.len() < deck.slot_count() {
        out.push(view::Strip {
            name: name.to_owned(),
            tally: view::Tally::Allocated,
            gain: 0.0,
            opacity: 0.0,
            blend: "",
            mask: view::Mask::None,
            level: None,
        });
    }
    for (slot, strip) in out.iter_mut().enumerate() {
        if strip.name != name {
            strip.name.clear();
            strip.name.push_str(name);
        }
        strip.tally = match deck.residency(slot) {
            Residency::Live => view::Tally::Live,
            Residency::Priming => view::Tally::Priming,
            Residency::Allocated => view::Tally::Allocated,
        };
        strip.gain = deck.gain(slot);
        strip.opacity = deck.opacity(slot);
        strip.blend = deck.blend(slot).name();
        strip.mask = match deck.mask(slot).kind() {
            MaskKind::None => view::Mask::None,
            MaskKind::Linear => view::Mask::Linear,
            MaskKind::Radial => view::Mask::Radial,
        };
        strip.level = deck.level(slot).map(|level| view::Level {
            mean: level.mean,
            peak: level.peak,
        });
    }
}

/// **What a frame has to fit in on this window**: the display's refresh
/// interval, in milliseconds — the `/16.6` in the mock's transport, at the
/// 60 Hz it was drawn against.
///
/// **It is the refresh interval because that is what this window is held to.**
/// The surface is `PresentMode::Fifo`, so a frame that takes longer than one
/// interval to build is a frame that misses a vsync, and every millisecond
/// under it is the headroom the mock's own tooltip is about. `Cost::wait` is
/// the other side of the same number: at 60 Hz most of the frame is spent
/// blocked in `get_current_texture` waiting for it.
///
/// **It is not `karakuri_engine`'s `DEFAULT_BUDGET_MS`**, which is 20 and is a
/// different budget with the same word on it: that one is what a *candidate
/// Set* has to hold to survive a hot swap, measured offscreen at a fixed size
/// and judged on a median. The mock's tooltip runs the two together — *"12.4
/// of 16.6 — there is headroom. A candidate that cannot hold this is rolled
/// back on its own"* — and they are two numbers. This row draws the one the
/// frame is actually against.
///
/// `None` where `winit` will not say, which is a monitor it cannot name or a
/// mode with no refresh rate on it. The row then draws the frame time and no
/// budget, rather than a plausible 16.6 nothing measured.
///
/// **Read once, when the window opens.** A window dragged onto a 120 Hz
/// display keeps the interval it opened on, which is a real limitation and is
/// the price of not asking the platform for a monitor handle sixty times a
/// second.
fn budget_ms(window: &Window) -> Option<f32> {
    let millihertz = window.current_monitor()?.refresh_rate_millihertz()?;
    match millihertz > 0 {
        true => Some(1.0e6 / millihertz as f32),
        false => None,
    }
}

/// A `.kir` off disk, parsed and checked — the two stages `Set::build` wants a
/// `Checked` from, and no more. `karakuri-cli`'s `compile::load` is the same
/// two with a cost estimate and a source it keeps; neither is wanted here.
fn checked(path: &std::path::Path) -> karakuri_ir::typed::Checked {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let parsed = karakuri_ir::parse(&src)
        .unwrap_or_else(|e| panic!("{}:\n{}", path.display(), rendered(&e, &src)));
    karakuri_ir::check::check(&parsed)
        .unwrap_or_else(|e| panic!("{}:\n{}", path.display(), rendered(&e, &src)))
}

fn rendered(errors: &[karakuri_ir::IrError], src: &str) -> String {
    errors
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// A rectangle in logical pixels, in physical ones. **Both of the engine's
/// textures are sized from this and from nothing else** — the picture from its
/// region, deck A's preview from its cell — which is what makes each of them
/// the size of what it is drawn into rather than of the window. Which
/// rectangle each one gets is [`aims`]; this is the *at what size* half, and
/// [`Presented::aim`] is the one place the two meet.
fn physical(rect: egui::Rect, scale: f32) -> (u32, u32) {
    (
        ((rect.width() * scale).round() as u32).max(1),
        ((rect.height() * scale).round() as u32).max(1),
    )
}

// ---------------------------------------------------------------------------
// The window
// ---------------------------------------------------------------------------

/// What to do about a frame that could not be acquired.
///
/// **Every outcome gets a decision, because ignoring one is invisible.** This
/// loop waits for events rather than spinning, so a `return` that neither
/// reconfigures nor asks for another frame is a window that stops drawing and
/// never starts again — and there is nothing on screen or on stdout to say
/// why. `karakuri-cli`'s window sink carries the same table with the same
/// argument, and says that returning silently is what left it with a frozen
/// window once already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Missed {
    /// The swapchain needs remaking: reconfigure, then ask for another frame.
    Remake,
    /// Ordinary jitter. Ask again.
    Again,
    /// Nobody can see the window. Doing nothing is right — it is the OS that
    /// says when it is back, and asking for frames meanwhile is a spin.
    Idle,
    /// Not self-correcting, and not something a retry mends. Say so.
    Fault,
}

/// `None` where a texture was handed over; a decision for every other case.
fn missed(outcome: &wgpu::CurrentSurfaceTexture) -> Option<Missed> {
    match outcome {
        // `Suboptimal` draws correctly and asks to be reconfigured for
        // performance, which the next resize does anyway.
        wgpu::CurrentSurfaceTexture::Success(_) | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
            None
        }
        wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
            Some(Missed::Remake)
        }
        wgpu::CurrentSurfaceTexture::Timeout => Some(Missed::Again),
        wgpu::CurrentSurfaceTexture::Occluded => Some(Missed::Idle),
        wgpu::CurrentSurfaceTexture::Validation => Some(Missed::Fault),
    }
}

struct Gfx {
    window: Arc<Window>,
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    egui: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    /// **The engine, on the same device as the panel.** It lives beside the
    /// renderer rather than beside the model because everything in it takes a
    /// device: that is the seam `karakuri-console` keeps, and this is the side
    /// of it that is allowed one.
    engine: Engine,
    /// **What a frame has to fit in on this window**, read from the display
    /// once when the window opened — see [`budget_ms`].
    budget_ms: Option<f32>,
    /// **What the mixer strip calls what this deck is playing**, worked out
    /// once from the two `.kir` paths — see [`material`]. Kept rather than
    /// recomputed because a name is a string and the frame path is budgeted.
    material: String,
}

struct App {
    gfx: Option<Gfx>,
    /// A validation fault is said once rather than sixty times a second.
    faulted: bool,
    readout: Readout,
    costs: Costs,
    /// Logical size, so the numbers printed are the arrangement's own units
    /// rather than the display's.
    scale: f64,
    /// **When `egui` asked to be drawn again, kept as the deadline it is.**
    ///
    /// `egui` animates, blinks a text cursor and fades a tooltip in, and it
    /// says so as a `repaint_delay` on the frame's `ViewportOutput`. Turning
    /// that into an immediate `request_redraw` would turn a 250 ms animation
    /// into a spin at whatever rate this loop can manage — which is the cost
    /// P-0072's first clause is about, arrived at from the one direction that
    /// looks like obeying it. So the delay is added to the clock here and
    /// `about_to_wait` sleeps until it.
    ///
    /// `None` where `egui` asked for nothing, which is every frame on a panel
    /// with nothing on it.
    egui_due: Option<Instant>,
}

impl App {
    fn new() -> App {
        App {
            gfx: None,
            faulted: false,
            readout: Readout::new(WINDOW.0 as f32, WINDOW.1 as f32),
            costs: Costs::new(),
            scale: 1.0,
            egui_due: None,
        }
    }

    /// Hand an event to `egui`, and nowhere else.
    ///
    /// Every call site has already asked `claim` where a pointer event
    /// belongs; this is the other branch. `EventResponse::repaint` is `egui`'s
    /// own answer for the event it was just given, and it is the reason
    /// `Change::Pointer(Claim::Egui)` asks for nothing: one answer per event,
    /// from whoever got it.
    fn to_egui(gfx: &mut Gfx, costs: &mut Costs, event: &WindowEvent) {
        let response = gfx.egui.on_window_event(&gfx.window, event);
        if response.repaint {
            costs.owes();
            gfx.window.request_redraw();
        }
    }

    /// **Act on a repaint decision, and the only place a frame is asked for
    /// outside `to_egui` and `missed`.**
    ///
    /// Three answers and three actions: ask for a frame, note a deadline, or
    /// do nothing at all — and the third is the one P-0072's first clause is
    /// made of.
    ///
    /// It takes the two fields rather than `&mut self` so that a caller
    /// holding `self.gfx` can still reach `self.egui_due`.
    fn wants(gfx: &Gfx, egui_due: &mut Option<Instant>, costs: &mut Costs, repaint: Repaint) {
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

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("The Karakuri console")
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW.0, WINDOW.1));
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
        // **A non-sRGB format, and that is the opposite of what the example
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
        let engine = Engine::new(
            &gpu,
            &mut renderer,
            self.readout.panel.layout(),
            self.scale as f32,
        );
        let info = gpu.adapter.get_info();
        self.costs.taken_on = format!(
            "{:?} — {} ({:?})",
            info.backend, info.name, info.device_type
        );

        let budget = budget_ms(&window);
        // **The strips before the legend**, because the legend says how many
        // there are and the answer is the deck's rather than a guess. It is
        // written again on every frame; this is the first one.
        let material = material();
        mixer(&engine.deck, &material, &mut self.readout.view.mixer);
        self.readout.print_legend(budget);

        // The first frame is owed to the window appearing, not drawn on a
        // still panel.
        self.costs.owes();
        window.request_redraw();
        self.gfx = Some(Gfx {
            budget_ms: budget,
            material,
            window,
            gpu,
            surface,
            config,
            egui,
            renderer,
            engine,
        });
    }

    /// **Where a deadline comes due**, which is the start of every iteration
    /// the loop makes — including the one a `ControlFlow::WaitUntil` woke it
    /// for.
    ///
    /// Both deadlines are checked whatever the [`StartCause`] rather than only
    /// on `ResumeTimeReached`: a wait that is cancelled early by a real event
    /// still has to leave a due deadline serviced, and checking two `Instant`s
    /// costs nothing.
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        let now = Instant::now();
        if self.egui_due.is_some_and(|due| due <= now) {
            self.egui_due = None;
            if let Some(gfx) = self.gfx.as_ref() {
                // One frame, now that the delay `egui` asked for has passed.
                gfx.window.request_redraw();
            }
        }
        if self.costs.due().is_some_and(|due| due <= now) {
            self.costs.say();
        }
    }

    /// **The one place the control flow is set, and it is a deadline or
    /// nothing.**
    ///
    /// `Wait` is a window that costs the machine nothing at all until somebody
    /// touches it, which is P-0072's first clause as the operating system sees
    /// it. `WaitUntil` is the soonest of the two things that are owed at a
    /// time rather than on an event: the frame `egui` asked for after a delay,
    /// and the reading `Costs` takes once the window has been still long
    /// enough. Neither is `Poll`, and nothing here asks for a frame in order
    /// to have something to measure.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let next = [self.egui_due, self.costs.due()]
            .into_iter()
            .flatten()
            .min();
        event_loop.set_control_flow(match next {
            Some(at) => ControlFlow::WaitUntil(at),
            None => ControlFlow::Wait,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(gfx) = self.gfx.as_mut() else {
            return;
        };
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
            WindowEvent::CloseRequested => event_loop.exit(),
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
                let (claim, _) = self.readout.pointer(&ctx, Pointer::Moved(p));
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Pointer(claim).repaint(),
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
                let (claim, did) = self.readout.pointer(&ctx, which);
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                // **A press on a control earns its frame from what it did**,
                // and not from the claim: `Change::Pointer(Claim::Panel)` is
                // already a frame, but the operation the dot asked for is the
                // thing that moved every region in the Program bay, and it is
                // the outcome that says so.
                let repaint = match &did {
                    Some(outcome) => Change::Operated(outcome).repaint(),
                    None => Change::Pointer(claim).repaint(),
                };
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            WindowEvent::MouseWheel { .. } => {
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, _) = self.readout.pointer(&ctx, Pointer::Wheel);
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Wheeled(claim).repaint(),
                );
            }

            WindowEvent::KeyboardInput { .. } => {
                // `egui` sees every key: it has no focused widget in this pass
                // and so consumes nothing, and its modifier state has to stay
                // current for the frame it does.
                App::to_egui(gfx, &mut self.costs, &event);
                let WindowEvent::KeyboardInput { event: key, .. } = &event else {
                    unreachable!("the arm this is in")
                };
                if key.state != ElementState::Pressed {
                    return;
                }
                let op = match key.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => {
                        event_loop.exit();
                        return;
                    }
                    // **The pointer is resolved here and not inside the
                    // operation.** `f` means fold and this is the surface
                    // saying which region — see `karakuri_console::panel::Op`.
                    // A key that names nothing emits nothing, and the readout
                    // says why from `target`.
                    Key::Character("f") => match self.readout.target("f") {
                        Some(id) => Op::Fold(id),
                        None => return,
                    },
                    Key::Character("g") => match self.readout.enclosing() {
                        Some(op) => op,
                        None => return,
                    },
                    Key::Character("z") => Op::UnfoldAll,
                    Key::Character("s") => match self.readout.target("s") {
                        Some(id) => Op::Solo(id),
                        None => return,
                    },
                    Key::Character("u") => Op::Unsolo,
                    Key::Character("r") => Op::Reset,
                    Key::Character("p") => Op::Report,
                    Key::Character("n") => {
                        // **The key that changes the screen without touching
                        // the pointer and without touching the model.** The
                        // room is the view's: every colour on the panel
                        // changes and nothing in the arrangement moves, so no
                        // `Outcome` says so and `Change::Room` is the only
                        // thing that does.
                        self.readout.room();
                        App::wants(
                            gfx,
                            &mut self.egui_due,
                            &mut self.costs,
                            Change::Room.repaint(),
                        );
                        return;
                    }
                    _ => return,
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
                // written in the same breath, off the `Present` that is about
                // to render it, for the reason `aims` reads it there too: the
                // shape the bay arranges itself for and the shape the texture
                // is sized to are one number or they are a picture drawn for
                // the other arrangement.
                //
                // `Change::Rearranged` is raised on **every** frame and says
                // whether anything moved, which is the arm's own argument:
                // this is the one change that is re-derived rather than
                // reported, and one that answered *draw* regardless would ask
                // for a frame on every frame.
                self.readout.view.canvas = gfx.engine.present.size();
                let moved = view::rearrange(&mut self.readout.panel, self.readout.view.canvas);
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Rearranged { moved }.repaint(),
                );

                let scale = self.scale as f32;
                let (picture, previews) = gfx.engine.aim(
                    &gfx.gpu,
                    &mut gfx.renderer,
                    self.readout.panel.layout(),
                    scale,
                );
                self.readout.view.picture = picture;
                self.readout.view.previews = previews;

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
                self.readout.view.transport =
                    transport(&gfx.engine.deck, &self.costs, gfx.budget_ms, live);
                // **And what the mixer strips read**, beside the frame they
                // are about for the same reason. One strip, because this
                // deck has one slot — see `mixer`.
                mixer(
                    &gfx.engine.deck,
                    &gfx.material,
                    &mut self.readout.view.mixer,
                );

                // -- the egui pass -------------------------------------
                let started = Instant::now();
                let (allocs, bytes) = counted();
                let input = gfx.egui.take_egui_input(&gfx.window);
                let panel = &mut self.readout.panel;
                let view = &mut self.readout.view;
                let mut output = gfx.egui.egui_ctx().run_ui(input, |ui| view.draw(ui, panel));
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
                        ..
                    } = &mut *gfx;
                    let gpu = &*gpu;
                    let Engine {
                        deck,
                        present,
                        picture,
                        preview,
                        ..
                    } = engine;
                    let textures_delta = &mut output.textures_delta;
                    let cost = &mut cost;
                    // **The console's sinks are the picture and deck A's
                    // preview cell, and nothing else.** Both are textures the
                    // panel samples and neither is presented anywhere. The
                    // window is not one of them: `karakuri-cli`'s window shows
                    // the canvas, and this window shows the panel.
                    let mut sinks: [&mut dyn Sink; 2] = [picture, preview];
                    compose(
                        gpu,
                        deck,
                        present,
                        &mut sinks,
                        // A region folded away refuses with `Transient` every
                        // frame it stays folded, and there is nothing to say
                        // about it sixty times a second. Neither of these
                        // sinks can fault — a `Presented` either has a
                        // rectangle or it has not — so the arm is what a third
                        // sink would need rather than what these two do.
                        &mut |_at, skip| {
                            if let Skip::Fault(why) = skip {
                                println!("a sink stopped taking frames: {why}");
                            }
                        },
                        // No clock and no record anywhere: see
                        // `STEPS_A_FRAME` and `LOOK`.
                        |_| Committed {
                            steps: STEPS_A_FRAME,
                            look: LOOK,
                        },
                        // -- the panel, into the frame's encoder ------
                        |encoder| {
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
                // **Said and not returned on**, and neither of this example's
                // sinks can produce it — `Presented::present` is `Ok(())`. It
                // is here because a third sink could, and because a frame the
                // other sinks took is not one this window may drop.
                if let Err(e) = composed {
                    println!("a sink failed to present: {e}");
                }

                gfx.gpu.queue.present(frame);

                self.costs.push(cost);
                App::wants(gfx, &mut self.egui_due, &mut self.costs, asked);
                // **Something is live, so the next frame is asked for here —
                // and asked for without `Costs::owes`.**
                //
                // Nothing used to ask for a frame at this point, and that was
                // P-0072's first clause holding: a panel with nothing changing
                // on it drew nothing. A picture that moves is something
                // changing on it, so the clause stops holding the moment the
                // engine runs — which is expected, is what the rest of P-0072
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
                if live {
                    gfx.window.request_redraw();
                }
            }
            _ => App::to_egui(gfx, &mut self.costs, &event),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    // **The loop sleeps.** A frame is drawn when something changed it or when
    // `egui` asked for one after a delay it named, and on no other occasion —
    // `App::about_to_wait` sets this again after every iteration and is where
    // the rule actually lives. This is the state it starts in so that the
    // window between here and the first `about_to_wait` is not a spin either.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut App::new()).expect("run");
}

// ---------------------------------------------------------------------------
// The tests that cannot leave this file
// ---------------------------------------------------------------------------

/// Everything about the model and the view is `tests/`, which
/// `cargo test -p karakuri-console` runs. What is left here is what is about a
/// `wgpu` type: the table below, and `egui` on a device under `mod gpu`.
#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing that comes back from `get_current_texture` is dropped without a
    /// decision. The loop waits for events, so an outcome that neither
    /// reconfigures nor asks for another frame is a window that never draws
    /// again and says nothing about it.
    #[test]
    fn every_frame_that_could_not_be_acquired_is_acted_on() {
        use wgpu::CurrentSurfaceTexture as Acquired;
        // The two that mean the swapchain is stale: reconfigure, and ask again.
        assert_eq!(missed(&Acquired::Outdated), Some(Missed::Remake));
        assert_eq!(missed(&Acquired::Lost), Some(Missed::Remake));
        // Jitter: ask again, without reconfiguring.
        assert_eq!(missed(&Acquired::Timeout), Some(Missed::Again));
        // A window nobody can see: asking again is a spin, and the OS says
        // when it is back.
        assert_eq!(missed(&Acquired::Occluded), Some(Missed::Idle));
        // Not self-correcting, so it is said rather than retried.
        assert_eq!(missed(&Acquired::Validation), Some(Missed::Fault));
    }

    /// **A frame nobody asked for is the one the reading is about.**
    ///
    /// The measurement carries the claim now, so what it counts has to be
    /// asserted rather than eyeballed on stdout. The failure it exists for is
    /// the one that made the first run of this read `3 frames` on a window
    /// that had behaved perfectly: an event resets the stretch, and the frame
    /// that event asked for lands a millisecond into the new one and gets
    /// blamed on the panel. The other direction is worse and is asserted too —
    /// a `push` that never counts anything reads `0 frames` whatever the
    /// window is doing, which is a measurement that cannot fail.
    #[test]
    fn a_frame_nobody_asked_for_is_the_one_counted_against_a_still_panel() {
        let frame = Cost {
            allocs: 7,
            bytes: 70,
            ..Default::default()
        };
        let mut costs = Costs::new();

        // A frame something asked for is that something's.
        costs.owes();
        costs.push(frame);
        assert_eq!(costs.still, Still::default(), "an owed frame was counted");

        // A frame nobody asked for — `egui`'s own deadline, on an untouched
        // window — is per-frame work on a still panel, which is the number.
        costs.push(frame);
        assert_eq!(
            costs.still,
            Still {
                frames: 1,
                allocs: 7,
                bytes: 70
            },
            "a frame nobody asked for was not counted"
        );

        // Both frames happened, whoever they belonged to.
        assert_eq!(costs.drawn, 2);

        // And touching the window starts the stretch again, so what was drawn
        // during the last one stops being about a still panel.
        costs.touched();
        assert_eq!(costs.still, Still::default());
    }

    /// **A whole drag, through the window loop's own routing.**
    ///
    /// `karakuri_console::input`'s tests are about the rule; this is about
    /// this file obeying it, which is a different claim and the one that
    /// actually reaches an operator. It drives the gesture a hand makes —
    /// press on the boundary between the left pane and the centre, run the
    /// pointer well past it and across two bays, let go — through
    /// `Readout::pointer`, which is the method `window_event` calls, and
    /// asserts two things: **the boundary moved**, so the drag works with a
    /// toolkit in the loop, and **`egui` was never told about any of it**, so
    /// the two never both think they are dragging.
    ///
    /// It cannot be a real pointer: synthesising one takes an Accessibility
    /// grant this process does not have, and a test that needs a human to
    /// click is not a test.
    #[test]
    fn a_drag_through_the_window_loops_own_routing_never_reaches_egui() {
        let ctx = drawn_once();
        let mut readout = Readout::new(1440.0, 900.0);
        readout.panel.solve();
        let layout = readout.panel.layout();
        let centre = layout.rect(layout.find("centre").expect("centre"));
        let left = layout.rect(layout.find("left-pane").expect("left-pane"));
        let was = left.w;
        // The gap between the left pane and the centre, at half height.
        let start = Point::new((left.x + left.w + centre.x) * 0.5, left.y + left.h * 0.5);

        // Approaching it is egui's until the pointer is on it.
        assert_eq!(
            readout
                .pointer(&ctx, Pointer::Moved(Point::new(start.x - 60.0, start.y)))
                .0,
            Claim::Egui
        );
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(start)).0, Claim::Panel);
        assert_eq!(readout.pointer(&ctx, Pointer::Down).0, Claim::Panel);

        // A hand does not stay on the boundary: it runs on across the panel,
        // and every one of these is inside a bay.
        for x in [start.x + 40.0, start.x + 120.0, start.x + 200.0] {
            assert_eq!(
                readout
                    .pointer(&ctx, Pointer::Moved(Point::new(x, start.y)))
                    .0,
                Claim::Panel,
                "the drag lost its claim at x = {x}"
            );
            // A wheel in the middle of a drag is the panel's too.
            assert_eq!(readout.pointer(&ctx, Pointer::Wheel).0, Claim::Panel);
        }
        readout.panel.solve();
        let wide = pane_width(&readout);
        assert!(
            wide > was + 100.0,
            "the boundary did not move: the left pane went from {was} to {wide}"
        );

        // Now back the other way, past what the left pane will go below, and
        // let go there. **The pointer ends nowhere near the boundary**, which
        // is the ordinary end of a drag and the case that catches a release
        // routed after `released` rather than before it.
        let far = Point::new(start.x - 200.0, start.y);
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(far)).0, Claim::Panel);
        assert_eq!(readout.pointer(&ctx, Pointer::Up).0, Claim::Panel);

        readout.panel.solve();
        assert!(
            (pane_width(&readout) - 160.0).abs() < 0.01,
            "the left pane's stated minimum did not hold the drag: {}",
            pane_width(&readout)
        );

        // And afterwards the pointer, where it is standing, is egui's again.
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(far)).0, Claim::Egui);
    }

    /// The left pane's width, solved. A helper because the test asks three
    /// times and the chain is four calls long.
    fn pane_width(readout: &Readout) -> f32 {
        let layout = readout.panel.layout();
        layout.rect(layout.find("left-pane").expect("left-pane")).w
    }

    /// **A context that has drawn once**, which is what routing a pointer
    /// takes: the claim rule asks where the Outputs row's control is, that is
    /// the width of the type in it, and `egui`'s fonts are not valid until a
    /// pass has run. The window loop has drawn long before a hand arrives;
    /// a test has to say so.
    ///
    /// The texture delta is cleared because `epaint` panics if one is dropped
    /// unapplied — there is no renderer here to apply it to, which is the
    /// whole of what makes this a test and not a window.
    fn drawn_once() -> egui::Context {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
        out.textures_delta.clear();
        ctx
    }

    /// **A press on the outputs dot, through the window loop's own routing.**
    ///
    /// The other half of the test above: that one is a boundary the panel
    /// claims and `egui` never sees, and this is the console's one control,
    /// which the panel claims for a different reason — `egui` owns no widget
    /// anywhere here, so a press routed to it would reach nothing at all.
    ///
    /// What is asserted is the round trip an operator makes: the picture is on
    /// screen, a click on the dot folds it away by name, and a click on the
    /// same dot brings it back. The dot is where it is drawn and the press is
    /// the panel's at every step.
    #[test]
    fn a_press_on_the_outputs_dot_folds_the_picture_and_unfolds_it() {
        let ctx = drawn_once();
        let mut readout = Readout::new(1440.0, 900.0);
        readout.panel.solve();
        let picture = readout
            .panel
            .layout()
            .find("program-view")
            .expect("program-view");
        let dot = |readout: &mut Readout| {
            readout.panel.solve();
            let row = outputs(&ctx, readout.panel.layout()).expect("the row draws its sink");
            (Point::new(row.sink.center().x, row.sink.center().y), row.on)
        };

        let (at, on) = dot(&mut readout);
        assert!(on, "the picture is on screen, so the sink is on");

        // The pointer arrives, and the control is the panel's.
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        let (claim, did) = readout.pointer(&ctx, Pointer::Down);
        assert_eq!(claim, Claim::Panel);
        assert_eq!(
            did,
            Some(Outcome::Folded {
                id: picture,
                folded: true,
                root: false
            }),
            "the press did not reach the sink"
        );
        assert!(
            !readout.panel.dragging(),
            "the press took a boundary in hand"
        );
        readout.pointer(&ctx, Pointer::Up);

        // And the dot is dark, where it still is, and turns the picture back
        // on rather than unfolding whatever else is folded.
        let (at, on) = dot(&mut readout);
        assert!(!on, "the picture is folded and the sink is still lit");
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        assert_eq!(
            readout.pointer(&ctx, Pointer::Down).1,
            Some(Outcome::Folded {
                id: picture,
                folded: false,
                root: false
            }),
            "the dark dot did not turn the picture back on"
        );
        assert!(
            dot(&mut readout).1,
            "the picture is back and the dot is dark"
        );
    }

    /// **Anything that makes texels this frame keeps the loop awake, and the
    /// list is closed.**
    ///
    /// [`live`] decides whether the loop asks for another frame, and it is the
    /// one decision in this file that has already been got wrong twice in the
    /// same direction. The first time it was set once and never cleared, so
    /// folding the picture away left the window drawing at full rate — found
    /// by an operator on another machine following this file's own
    /// instructions, which said the window goes quiet, and getting 270 frames.
    /// The second time it was **the picture alone**, which is the same failure
    /// with a preview under it: fold the picture and deck A goes on
    /// auditioning while the loop stops asking for frames, so the panel keeps
    /// changing and nothing draws it.
    ///
    /// So the assertion is over every sink, not over the one this example
    /// fills: a cell nobody has wired up yet is asserted live all the same,
    /// because the failure is a sink left out of the list rather than a sink
    /// that is off.
    ///
    /// It needs no device: an `egui::TextureId` is a number, and what is being
    /// asserted is a rule about `Option`s.
    #[test]
    fn anything_that_makes_texels_keeps_the_loop_awake() {
        let some = Picture {
            id: egui::TextureId::User(0),
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(16.0, 9.0)),
        };
        let mut view = View::new(Room::Day);

        // Nothing is making texels, so the loop has no reason of its own to
        // draw and `ControlFlow::Wait` gets to block.
        assert!(!live(&view), "an empty panel was called live");

        // The picture, which is what this rule used to be the whole of.
        view.picture = Some(some);
        assert!(live(&view), "a live picture did not keep the loop awake");

        // **The case the picture-alone rule gets wrong**: the picture folded
        // away with deck A still auditioning under it.
        view.picture = None;
        view.previews[0] = Some(some);
        assert!(
            live(&view),
            "the picture is folded away and deck A is still rendering, and the loop was \
             told to sleep — which is the window that kept drawing 270 frames after it \
             was said to have gone quiet"
        );

        // And the list is closed: every cell counts, including the three this
        // example leaves off, because the bug is a sink that is not read here.
        for deck in 0..DECKS {
            let mut view = View::new(Room::Day);
            view.previews[deck] = Some(some);
            assert!(
                live(&view),
                "deck {deck} is rendering and the loop was told to sleep"
            );
        }

        // The other direction, which costs frames rather than pixels: with
        // every sink off the loop stops asking.
        view.previews[0] = None;
        assert!(
            !live(&view),
            "nothing is rendering and the loop stayed awake"
        );
    }
}

#[cfg(test)]
mod gpu {
    //! The console, through `egui`, through `wgpu` 30, onto a real device.

    use super::*;
    /// **The engine's own fitting, asked rather than re-derived.** It is what
    /// `Present::draw` sets its viewport from, so what it leaves over at the
    /// edges of a target is exactly the bar that gets cleared to black — and a
    /// copy of the arithmetic here would be a test agreeing with itself about
    /// the one thing it is checking.
    use karakuri_engine::letterbox;

    /// **ADR-0155's other half, as an assertion: the engine's texels reach the
    /// panel.**
    ///
    /// The first half — `egui` and `karakuri-engine` resolving one `wgpu` and
    /// sharing one `Device` — is what
    /// `egui_paints_the_console_onto_a_device` below settles. This is the
    /// question that was left: a Set is built, a deck frame is rendered, the
    /// present pass letterboxes the canvas into the picture's rectangle, and
    /// the panel's own pass samples that texture — **all into one command
    /// encoder and one submission, engine first** — and what lands in the
    /// window is read back.
    ///
    /// # Why the assertion is "lit" and not "not the bay's colour"
    ///
    /// A picture that never had anything drawn into it is black, and black is
    /// already not `--c-panel`. So "the picture is not the card" passes for a
    /// texture that was registered, sampled and never rendered — which is
    /// exactly what the ordering defect produces: record the panel's pass
    /// before the engine's and the frame samples an empty texture, with no
    /// complaint from anywhere. **The particles are the evidence**, so the
    /// count of lit texels inside the picture is what is asserted, and the two
    /// controls beside it — the bay's own body, and the deck preview row —
    /// say the picture stayed in its region.
    #[test]
    fn the_engines_frame_reaches_the_picture_in_the_program_bay() {
        // Gamma space, and 1408 rather than 1440 because
        // `copy_texture_to_buffer` wants `bytes_per_row` a multiple of 256.
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1408;
        const H: u32 = 900;
        const ROOM: Room = Room::Night;

        let gpu = Gpu::headless().expect("no GPU");
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("program probe"),
            size: wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let rect = picture_rect(panel.layout(), CANVAS).expect("the picture is on screen");
        let cells = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen");
        let mut engine = Engine::new(&gpu, &mut renderer, panel.layout(), 1.0);

        // **Aimed by the call the window makes, and the view is what that
        // answered** rather than three lines this test writes by hand: an id
        // or a rectangle assembled here is a test agreeing with itself about
        // the one thing `Engine::aim` exists to decide. Deck A auditions and
        // B, C and D are off, which is the whole of what one slot can show.
        let mut view = View::new(ROOM);
        (view.picture, view.previews) = engine.aim(&gpu, &mut renderer, panel.layout(), 1.0);
        assert_eq!(
            view.picture.expect("the picture was not aimed").rect,
            rect,
            "the picture is drawn somewhere other than the region it was sized from"
        );
        assert_eq!(
            view.previews[0].expect("deck A was not aimed").rect,
            cells[0]
        );

        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(W as f32, H as f32),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| view.draw(ui, &mut panel));
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [W, H],
            pixels_per_point: 1.0,
        };
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }

        let row = W * 4;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("program probe"),
            size: (row * H) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // **The frame, through the call the window loop makes** — not a
        // hand-rolled copy of it beside it, which is what this used to be and
        // is the drift `karakuri_engine::frame` exists to end. `compose` asks
        // both sinks, advances the deck, presents the canvas into each of
        // them, and hands the frame's own encoder to the closure: the panel,
        // over the top of all of it, in one submission.
        let mut user_empty = false;
        let mut refusals: Vec<(usize, Skip)> = Vec::new();
        let outcome = {
            let textures_delta = &mut output.textures_delta;
            let Engine {
                deck,
                present,
                picture,
                preview,
                ..
            } = &mut engine;
            let mut sinks: [&mut dyn Sink; 2] = [picture, preview];
            compose(
                &gpu,
                deck,
                present,
                &mut sinks,
                &mut |at, skip| refusals.push((at, skip)),
                |_| Committed {
                    steps: STEPS_A_FRAME,
                    look: LOOK,
                },
                |encoder| {
                    let user = renderer.update_buffers(
                        &gpu.device,
                        &gpu.queue,
                        encoder,
                        &primitives,
                        &screen,
                    );
                    {
                        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("program probe"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &target_view,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
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
                    textures_delta.clear();
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &target,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &readback,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(row),
                                rows_per_image: Some(H),
                            },
                        },
                        wgpu::Extent3d {
                            width: W,
                            height: H,
                            depth_or_array_layers: 1,
                        },
                    );
                    user_empty = user.is_empty();
                },
            )
            .expect("neither of the console's sinks presents anything")
        };
        assert!(
            user_empty,
            "a paint callback appeared: it has to be submitted ahead of the pass"
        );
        assert!(refusals.is_empty(), "a sink refused: {refusals:?}");
        // **Both sinks took the frame, and nothing else was in the slice.**
        // The panel is not one of them — it is drawn in `finally`, and a
        // `reached` of 3 here would be the console counting a consumer as an
        // output. See `frame::compose`.
        assert_eq!(
            outcome,
            karakuri_engine::Outcome {
                reached: 2,
                missed: 0
            }
        );

        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device stopped");
        let texels = readback.slice(..).get_mapped_range().expect("readback");
        let at = |x: u32, y: u32| {
            let i = (y * row + x * 4) as usize;
            [texels[i], texels[i + 1], texels[i + 2]]
        };
        let brightest = |rgb: [u8; 3]| rgb.into_iter().max().unwrap_or(0);

        // **The picture, and it is lit.** Every texel of the region the
        // rectangle names, counted rather than sampled: the material is
        // additive points on a black clear, so a handful of rows through the
        // middle could miss and a count cannot.
        // Two counts over every texel of the region, rather than a handful of
        // samples through the middle: the material is additive points on a
        // black clear, so a row that missed would say nothing and a count
        // cannot.
        //
        // **Dark** is the present pass's own clear — the letterbox bars, and
        // the empty sky between the particles — and it is what the bay's card
        // is not: `--c-panel` at night is `#17142a`, whose brightest channel
        // is 42. It is the half the ordering defect fails, and it fails it
        // completely rather than by a margin: an unwritten texture is
        // `rgba(0, 0, 0, 0)` and `egui` blends premultiplied, so a picture
        // sampled before it was drawn is not black — it is *transparent*, and
        // the card shows through every texel of it. Recording the panel's pass
        // before the engine's, and leaving the present pass out altogether,
        // both read here as **zero** dark texels.
        //
        // **Bright** is the particles, well past anything the panel draws. It
        // is the control on the fixture, in the sense `karakuri-cli`'s frame
        // tests use: a picture that is opaque and empty — a deck compositing
        // nothing — is all dark and no bright, and would satisfy the first
        // count while showing an operator a black rectangle.
        // Two counts over every texel of a rectangle: dark, and lit. A helper
        // because the picture and deck A's cell are the same question asked of
        // two rectangles, and a second copy of the loop is a second threshold
        // to keep in step.
        let counted = |r: egui::Rect| {
            let mut dark = 0usize;
            let mut bright = 0usize;
            let mut inside = 0usize;
            for y in r.min.y as u32..r.max.y as u32 {
                for x in r.min.x as u32..r.max.x as u32 {
                    inside += 1;
                    match brightest(at(x, y)) {
                        b if b <= 8 => dark += 1,
                        b if b >= 192 => bright += 1,
                        _ => {}
                    }
                }
            }
            (dark, bright, inside)
        };

        let (dark, bright, inside) = counted(rect);
        assert!(
            dark * 2 > inside,
            "only {dark} of {inside} texels in the picture are darker than anything the \
             panel draws — the picture is the bay's card, so the engine's texture never \
             reached it"
        );
        assert!(
            bright * 100 > inside,
            "{bright} of {inside} texels in the picture are lit — the picture reached the \
             panel and there is nothing in it, so the deck composited nothing and this \
             would pass over a black rectangle"
        );

        // **Deck A's cell, and the same two counts.** It is the second present
        // pass arriving, and it fails the same two ways: a cell the pass never
        // wrote is an unrendered texture, which is transparent rather than
        // black, so the well shows through every texel of it and *nothing* is
        // dark. The lit count is the control on that — a cell that is opaque
        // and empty would satisfy the first and show an operator a black
        // thumbnail.
        //
        // The cell is a fifth the picture's width, so this is also the claim
        // that one `Present` fits its canvas into two targets of different
        // sizes rather than drawing the picture's rectangle twice.
        //
        // **The same two thresholds as the picture**, on the same helper, and
        // they are not tuned to this rectangle: measured here the cell comes
        // out 76% dark and 16% lit, against the 50% and 1% asked for. A cell
        // that reads anything like a lit picture passes; one the pass missed
        // reads zero dark, which is a factor away rather than a margin.
        let (dark, bright, inside) = counted(cells[0]);
        assert!(
            dark * 2 > inside,
            "only {dark} of {inside} texels in deck A's cell are darker than anything the \
             panel draws — the cell is still the mock's well, so the second present pass \
             never reached it"
        );
        assert!(
            bright * 100 > inside,
            "{bright} of {inside} texels in deck A's cell are lit — the audition reached \
             the panel and there is nothing in it"
        );

        // **And each stayed where it was put.** Three controls, because a
        // picture drawn over the whole window would satisfy every count above:
        // deck D's cell is off, so it is the mock's well and nothing else;
        // and a bay the Program is nowhere near is still the bay's card.
        let pal = ROOM.palette();
        let panel_rgb = [pal.panel.r(), pal.panel.g(), pal.panel.b()];
        let well_rgb = [pal.well.r(), pal.well.g(), pal.well.b()];
        let d = cells[DECKS - 1].center();
        assert_eq!(
            at(d.x as u32, d.y as u32),
            well_rgb,
            "deck D's cell is off and is not the mock's well — either the picture painted \
             over the preview row, or an audition was drawn outside its own cell"
        );
        let library = panel
            .layout()
            .rect(panel.layout().find("library").expect("library"));
        assert_eq!(
            at(
                (library.x + library.w * 0.5) as u32,
                (library.y + library.h * 0.5) as u32
            ),
            panel_rgb,
            "the picture reached the Library bay"
        );
    }

    /// **The picture's texture is the size of the picture, the picture is the
    /// canvas's shape, and the texture therefore carries no bars.**
    ///
    /// Three claims and every one of them fails without a mark on the screen.
    /// A texture sized from the window looks perfectly correct — the picture
    /// fills whatever rectangle it is given — and is wrong by however much the
    /// panel is not the picture, which here is most of it. A texture sized
    /// from the whole **region** looks perfectly correct too, and that is the
    /// one this change is about: it is the shape of the region rather than of
    /// the canvas, so `Present::draw` fills the middle of it and clears the
    /// rest, and the bars are allocated, cleared and sampled sixty times a
    /// second for nobody. At this window that is **225 texels down each side**
    /// of a 916-wide texture.
    ///
    /// **The bars are asked of the engine's own `letterbox`** rather than
    /// re-derived here, because that is the function that draws them: it
    /// answers where the canvas sits inside the texture, so a bar is what it
    /// leaves over. What is asserted is that the bar is **under one texel** —
    /// not zero, and the difference is the whole of why `Present::draw` stays.
    /// `picture_rect` rounds to whole pixels, so the picture is the mock's
    /// 466 x 262 rather than exactly 16:9, and the fit still has a quarter of
    /// a pixel to absorb. Sub-texel is what this change makes it; redundant is
    /// what it does not.
    #[test]
    fn the_picture_is_the_canvass_shape_and_carries_no_bars() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let rect = picture_rect(panel.layout(), CANVAS).expect("on screen");
        let want = physical(rect, 1.0);
        let engine = Engine::new(&gpu, &mut renderer, panel.layout(), 1.0);

        // The picture's, in both axes, and **neither of them is the window's**
        // — the picture is narrower than the window by both panes and taller
        // by nothing like the window's height.
        assert_eq!(
            (
                engine.picture.texture.width(),
                engine.picture.texture.height()
            ),
            want
        );
        assert_ne!(engine.picture.size, (W, H));
        assert!(engine.picture.size.0 < W && engine.picture.size.1 < H / 2);
        assert!(renderer.texture(&engine.picture.id).is_some());

        // **And it is not the region's either**, which is the texture this
        // change removes: the region is the same height and hundreds of pixels
        // wider, all of it bars.
        let region = panel
            .layout()
            .rect(panel.layout().find("program-view").expect("program-view"));
        assert!(
            (region.w - rect.width()) > 400.0,
            "the picture is the width of its region, so it is the region that was sized \
             from and the bars are still inside the texture: {} against {}",
            rect.width(),
            region.w
        );

        // **No bars, asked of the pass that would draw them.** `letterbox` is
        // what `Present::draw` sets its viewport from, so what it leaves over
        // at the edges is exactly what gets cleared to black.
        let (x, y, w, h) = letterbox(CANVAS, engine.picture.size);
        let (tw, th) = (engine.picture.size.0 as f32, engine.picture.size.1 as f32);
        assert!(
            x < 1.0 && y < 1.0,
            "the canvas sits {x} x {y} into its own texture, which is {} and {} texels of \
             bar down each side — the picture is not the canvas's shape",
            x.round(),
            y.round()
        );
        assert!(
            w > tw - 2.0 && h > th - 2.0,
            "the canvas covers {w} x {h} of a {tw} x {th} texture, so the rest is cleared \
             to black every frame"
        );

        // The control on all of it: a region-sized texture is what the
        // assertions above would pass over, and it does not — this is the
        // number in the doc, computed rather than quoted.
        let (bar, _, _, _) = letterbox(CANVAS, (region.w.round() as u32, want.1));
        assert!(
            bar > 200.0,
            "a texture sized from the region would carry {bar} texels of bar, and the \
             thresholds above are not measuring anything"
        );
    }

    /// **A wider window remakes no texture at all, and a drag on the program's
    /// height remakes one and frees the registration it replaces.**
    ///
    /// The first half is new and is the saving this change is for. The
    /// picture's rectangle used to follow the window's width, so every frame
    /// of a horizontal drag was a texture destroyed and rebuilt and a
    /// registration freed and re-registered — on the render thread. It is the
    /// canvas's shape now and the arrangement pins its height, so a widening
    /// moves nothing and there is nothing to remake. **That is asserted rather
    /// than described**, because a rule that quietly went back to remaking
    /// costs exactly what it used to and says nothing.
    ///
    /// The second half is the claim the first one must not be allowed to
    /// weaken: a `register_native_texture` with no `free_texture` beside it
    /// leaks a bind group and a sampler per remade frame, and the height is
    /// still something an operator drags. So the free is asserted on the
    /// resize that still happens.
    #[test]
    fn a_wider_window_remakes_nothing_and_a_taller_picture_frees_the_old_texture() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let want = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        let mut engine = Engine::new(&gpu, &mut renderer, panel.layout(), 1.0);
        let first = engine.picture.id;
        assert_eq!(engine.picture.size, want);

        // **A window 100 wider, and nothing moves.** The region widens and the
        // picture does not, so `aim` — the call the frame makes — finds the
        // size it already had and remakes nothing.
        //
        // **It was 400 wider until the bay started arranging itself**, and 400
        // is no longer a wider window of the same panel: 1840 is past the
        // 1588 where the four cells go down the sides, and there the picture
        // is 592 x 333 rather than the mock's 466 x 262. That is a texture
        // remade **once**, which is the subject of
        // `deck_a_preview_texture_is_its_cells_size_and_a_resize_frees_the_old_one`
        // below and not of this one; this test is about a width that changes
        // nothing, so it stays inside one arrangement and says so.
        panel.set_viewport(W as f32 + 100.0, H as f32);
        view::rearrange(&mut panel, CANVAS);
        assert!(
            !panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "1540 is past the crossover, so this is two arrangements and not one width"
        );
        let wider = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        assert_eq!(
            wider, want,
            "a wider window changed the picture's texture, so the picture is still the \
             width of its region and every frame of a horizontal drag reallocates"
        );
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0);
        assert_eq!(
            engine.freed, 0,
            "a wider window freed a registration, so it remade the texture"
        );
        assert_eq!(engine.picture.id, first);
        assert_eq!(engine.picture.size, want);

        // **A drag on the program's bottom edge is what does change it** —
        // through the panel's own pointer, which is the gesture the leak is
        // about rather than a size written by hand.
        let program = panel
            .layout()
            .rect(panel.layout().find("program").expect("program"));
        let edge = Point {
            x: program.x + program.w * 0.5,
            y: program.y + program.h + 2.0,
        };
        assert!(
            matches!(panel.press(edge), Pressed::Grabbed { .. }),
            "the boundary under the program is not where the drag starts"
        );
        panel.moved(Point {
            x: edge.x,
            y: edge.y + 300.0,
        });
        panel.released();
        view::rearrange(&mut panel, CANVAS);
        let taller = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        assert!(
            taller.1 > want.1 && taller.0 > want.0,
            "dragging the program taller did not grow the picture: {taller:?} against \
             {want:?}"
        );

        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0);
        assert_eq!(engine.picture.size, taller);
        assert_eq!(
            (
                engine.picture.texture.width(),
                engine.picture.texture.height()
            ),
            taller
        );
        assert_ne!(engine.picture.id, first);
        assert!(renderer.texture(&engine.picture.id).is_some());
        assert!(
            renderer.texture(&first).is_none(),
            "the registration the resize replaced is still in the atlas, so the atlas \
             grows once per dragged frame"
        );
        assert_eq!(engine.freed, 1);

        // And a second aim with nothing moved remakes nothing, which is what
        // keeps all of the above on the resize path instead of on every frame.
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0);
        assert_eq!(engine.freed, 1);
        assert!(renderer.texture(&engine.picture.id).is_some());
    }

    /// **Deck A's texture is the size of its cell, a resize frees the
    /// registration it replaces, and the tally counts both textures.**
    ///
    /// The picture's own test above, one cell down, and it fails the same
    /// silent ways. A preview sized from anything but its cell — the row, the
    /// region, the picture, the window — looks perfectly correct on screen,
    /// because the cell is drawn at whatever size it is and the texture fills
    /// it; it is simply four to twenty times more texels than the audition
    /// needs, per frame, for as long as the deck runs. And a
    /// `register_native_texture` with no `free_texture` beside it leaks a bind
    /// group and a sampler per remade frame.
    ///
    /// # What this test is for now, and the sentence it used to carry
    ///
    /// It used to say: *"the size a cell is remade at is the scale rather than
    /// the window ... `deck-previews` is pinned at 72 tall, so a cell is 16:9
    /// inside a fixed height and stays exactly as big at any wider window"* —
    /// and it widened the window by 400 to prove it. **That is false since the
    /// bay started arranging itself**, and it was false at exactly the two
    /// widths this test already used: 1440 is below the crossover and 1840 is
    /// past it, so the 400 the test widens by is the one resize that makes a
    /// cell **eleven times** the texels it was.
    ///
    /// So the widths stay and the claim is the other one, which is the claim
    /// worth having: **a cell's texture is the size of a cell in whichever
    /// arrangement the bay is in, and a resize that changes that frees the
    /// registration it replaces.** Three sizes, and each is a different way of
    /// getting it wrong:
    ///
    /// - **112 x 63 in the row**, which is the cell and not the row, the
    ///   region, the picture or the window.
    /// - **290 x 163 beside the picture**, which is the same rule read off a
    ///   column instead of a track — 47,270 texels against 7,056, which is
    ///   **6.7x**, **remade once** at the crossover and not once per frame of
    ///   the drag that crossed it.
    /// - **and the scale**, which is the display it is dragged onto rather
    ///   than the window it is in: `ScaleFactorChanged`, and
    ///   `physical(cell, scale)`.
    ///
    /// Between them they hold the cell's size against every one of the four
    /// things that can change it, and the freed tally counts every remake.
    #[test]
    fn deck_a_preview_texture_is_its_cells_size_and_a_resize_frees_the_old_one() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let row = panel.layout().find("deck-previews").expect("the row");
        assert!(
            !panel.layout().is_set_aside(row),
            "1440 is past the crossover, so the cells start beside the picture and the \
             row's own arithmetic is not what is asserted below"
        );
        let picture = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        let cells = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen");
        let want = physical(cells[0], 1.0);
        assert_eq!(
            want,
            (112, 63),
            "the mock's own cell, at the mock's own width"
        );
        let mut engine = Engine::new(&gpu, &mut renderer, panel.layout(), 1.0);

        // **The cell's, in both axes** — not the row's, not the picture's and
        // not the window's. The row holds four of these side by side with
        // ground between them, so a texture sized from the row is out by a
        // factor of four in one axis alone.
        assert_eq!(
            (
                engine.preview.texture.width(),
                engine.preview.texture.height()
            ),
            want
        );
        assert_eq!(engine.preview.size, want);
        assert_ne!(engine.preview.size, (W, H));
        assert_ne!(
            engine.preview.size, engine.picture.size,
            "deck A's texture is the picture's size, so it was sized from the wrong \
             rectangle and nothing on screen would say so"
        );
        assert!(
            engine.preview.size.0 * 4 < engine.picture.size.0
                && engine.preview.size.1 * 2 < engine.picture.size.1,
            "a preview cell is not much smaller than the picture: {:?} against {:?}",
            engine.preview.size,
            engine.picture.size
        );
        assert!(renderer.texture(&engine.preview.id).is_some());

        // **A window 400 wider is the crossover**, and it is the one resize a
        // cell's texture is remade by. 1840 puts the body past 1064, the four
        // cells go down the sides two to a column, and a cell stops being a
        // track of the row and becomes half of a column: 290 x 163, which is
        // 6.7 times the texels — **once**, on the frame the arrangement
        // changed, with the registration it replaces freed.
        panel.set_viewport(W as f32 + 400.0, H as f32);
        view::rearrange(&mut panel, CANVAS);
        assert!(
            panel.layout().is_set_aside(row),
            "1840 is not past the crossover, so this resize is not the one being asserted"
        );
        let beside = physical(
            preview_rects(panel.layout(), CANVAS).expect("on screen")[0],
            1.0,
        );
        assert_eq!(
            beside,
            (290, 163),
            "a cell beside the picture is not half a column"
        );
        let was = engine.preview.id;
        assert!(
            engine
                .preview
                .fit(&gpu, &mut renderer, beside, &mut engine.freed),
            "the cells moved beside the picture and deck A's texture was not remade, so \
             the audition is 112 x 63 texels stretched over a 290 x 163 cell"
        );
        assert_eq!(engine.preview.size, beside);
        assert_eq!(engine.freed, 1);
        assert!(
            renderer.texture(&was).is_none(),
            "the registration the crossover replaced is still in the atlas"
        );
        assert!(renderer.texture(&engine.preview.id).is_some());

        // **And a wider window inside *that* arrangement remakes nothing
        // either**, which is the sentence this test used to make about the row
        // and is true of a column for a better reason: a column is
        // `(H - 6) / 2` at 16:9, a function of the bay's **height** alone, so
        // every pixel of width past the crossover goes to the picture. A frame
        // where nothing moved remakes nothing, which is what keeps the free on
        // the resize path instead of on every frame.
        panel.set_viewport(W as f32 + 800.0, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let wider = physical(
            preview_rects(panel.layout(), CANVAS).expect("on screen")[0],
            1.0,
        );
        assert_eq!(wider, beside, "a wider window changed the size of a cell");
        assert!(!engine
            .preview
            .fit(&gpu, &mut renderer, wider, &mut engine.freed));
        assert_eq!(engine.freed, 1);

        // A display of a different scale is what changes it next.
        let was = engine.preview.id;
        let retina = physical(
            preview_rects(panel.layout(), CANVAS).expect("on screen")[0],
            2.0,
        );
        assert_eq!(retina, (beside.0 * 2, beside.1 * 2));
        assert!(
            engine
                .preview
                .fit(&gpu, &mut renderer, retina, &mut engine.freed),
            "a cell that changed size did not remake the texture"
        );
        assert_eq!(engine.preview.size, retina);
        assert_eq!(
            (
                engine.preview.texture.width(),
                engine.preview.texture.height()
            ),
            retina
        );
        assert_ne!(engine.preview.id, was);
        assert!(renderer.texture(&engine.preview.id).is_some());
        assert!(
            renderer.texture(&was).is_none(),
            "the registration the resize replaced is still in the atlas, so the atlas \
             grows once per remade frame"
        );
        assert_eq!(engine.freed, 2);

        // **The tally is the whole engine's, over both textures.** Fitting the
        // picture as well takes it to three: a count kept per texture would
        // read two here, and `mod gpu` would be asserting on half the leak.
        assert!(engine.picture.fit(
            &gpu,
            &mut renderer,
            (picture.0 + 40, picture.1),
            &mut engine.freed
        ));
        assert_eq!(
            engine.freed, 3,
            "the freed tally did not count both textures"
        );
    }

    /// **Which rectangle each sink's texture is sized from, and where the
    /// console then draws it — asked of the call the frame actually makes.**
    ///
    /// This is the hole `docs/roadmap.md` recorded, closed. The decision used
    /// to be two `match`es inside `App::window_event`, and `winit` will not
    /// hand a test an `ActiveEventLoop`, so nothing could call it: `mod gpu`
    /// asserted what `Engine::new` did and not what the frame chose.
    /// **Sizing deck A's texture from the picture's rectangle was injected
    /// there and every test still passed.** It is [`aims`] and [`Engine::aim`]
    /// now, which take a solved layout and a scale factor and touch no window,
    /// and this asks them at a viewport and a scale neither of which
    /// `Engine::new` was given — so what is asserted is what `aim` decided
    /// rather than what construction left behind.
    ///
    /// Every half of it fails silently. A texture sized from the wrong
    /// rectangle looks perfectly correct — the cell is drawn at whatever size
    /// it is and the texture fills it — and is four to twenty times the texels
    /// the audition needs, per frame, for as long as the deck runs. A
    /// `Picture` carrying an id from before a resize is a freed registration,
    /// which `egui` draws as nothing at all. And a folded region whose sink
    /// still acquires is the manual's *"no state where it is hidden and still
    /// costing a pass"* quietly stopping being true.
    #[test]
    fn the_frame_aims_each_sink_at_its_own_rectangle() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// A display of a different scale, because the size is the rectangle
        /// **and** the scale and a test at 1.0 cannot tell them apart.
        const SCALE: f32 = 2.0;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(&gpu, &mut renderer, panel.layout(), 1.0);

        // A different window on a different display, so nothing asserted below
        // can be what construction happened to leave in place — and at 1760
        // wide it is a window past the crossover, so what is asserted below is
        // the arrangement with the four cells **beside** the picture. The
        // rearrangement is the frame's own first act and this test makes it in
        // the same order.
        panel.set_viewport(W as f32 + 320.0, H as f32 - 120.0);
        view::rearrange(&mut panel, CANVAS);
        assert!(
            panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "1760 is not past the crossover, so the cells are still in the row"
        );
        let rect = picture_rect(panel.layout(), CANVAS).expect("the picture is on screen");
        let cell = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen")[0];
        let (picture, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), SCALE);

        // **Each texture is the size of its own rectangle, at this scale.**
        assert_eq!(
            engine.picture.size,
            physical(rect, SCALE),
            "the picture's texture is not the size of the picture's region"
        );
        assert_eq!(
            engine.preview.size,
            physical(cell, SCALE),
            "deck A's texture is not the size of deck A's cell — it was sized from some \
             other rectangle, and nothing on screen would say so"
        );
        assert_ne!(
            engine.preview.size, engine.picture.size,
            "deck A's texture is the picture's size"
        );
        // **Half the picture in each direction, and it is half rather than the
        // quarter this used to ask for.** A quarter of the width was the row's
        // arithmetic — four tracks across the bay — and beside the picture a
        // cell is half a column, so it is about half the picture each way and
        // a quarter of its texels. The claim being made is the one that
        // catches the defect either way: a cell sized from the picture's
        // rectangle, or from the window, is *larger* than this and not
        // smaller.
        assert!(
            engine.preview.size.0 * 2 <= engine.picture.size.0
                && engine.preview.size.1 * 2 <= engine.picture.size.1,
            "a preview cell is not much smaller than the picture: {:?} against {:?}",
            engine.preview.size,
            engine.picture.size
        );

        // **And where the console draws it is the same statement**: the
        // rectangle the texture was just sized from, and the id the sizing may
        // have just replaced.
        let drawn = picture.expect("the picture is on screen and the frame aimed nothing at it");
        assert_eq!(
            drawn.rect, rect,
            "the picture is drawn somewhere other than the region \
             its texture was sized from"
        );
        assert_eq!(
            drawn.id, engine.picture.id,
            "the view carries the id from before the resize, which is a freed registration"
        );
        assert!(renderer.texture(&drawn.id).is_some());
        let audition =
            previews[0].expect("deck A is auditioning and the frame aimed nothing at it");
        assert_eq!(audition.rect, cell);
        assert_eq!(audition.id, engine.preview.id);
        assert!(renderer.texture(&audition.id).is_some());
        assert!(
            previews[1..].iter().all(Option::is_none),
            "a deck with no slot behind it was aimed at a cell"
        );

        // **Aimed is what `Sink::acquire` answers from**, and that is the
        // whole of what `compose` asks either of them.
        assert_eq!(engine.picture.acquire(&gpu), Ok(()));
        assert_eq!(engine.preview.acquire(&gpu), Ok(()));

        // **Fold the picture away and its sink has no target** — so `compose`
        // records no present pass into it, the deck still advances, and deck A
        // goes on auditioning underneath. Both halves matter: a fold that took
        // the preview with it is the console going dark from one keystroke.
        let picture_node = panel.layout().find("program-view").expect("program-view");
        assert!(
            matches!(
                panel.op(Op::Fold(picture_node)),
                Outcome::Folded { folded: true, .. }
            ),
            "the picture did not fold"
        );
        // **The bay rearranges around the fold, and this is the guard rule
        // reached through the frame's own call.** The cells were beside the
        // picture; with the picture gone the row comes back under it, because
        // a bay whose only laid-out child is set aside can use nothing at all
        // and would claim no height. Reading a rectangle without this is
        // reading one from before the fold — the same contract `Layout::rect`
        // has about a stale solve.
        view::rearrange(&mut panel, CANVAS);
        assert!(picture_rect(panel.layout(), CANVAS).is_none());
        assert!(
            !panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "the picture is folded and the row is still set aside, so the Program bay \
             claims nothing and has gone from the panel"
        );
        let (picture, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), SCALE);
        assert!(
            picture.is_none(),
            "the picture is folded away and the frame still gave the console one to draw"
        );
        assert_eq!(
            engine.picture.acquire(&gpu),
            Err(Skip::Transient),
            "the picture is folded away and its sink still took the frame, so a present \
             pass is recorded into a texture nothing shows"
        );
        assert!(
            previews[0].is_some() && engine.preview.acquire(&gpu) == Ok(()),
            "folding the picture away stopped deck A auditioning under it"
        );
    }

    /// **ADR-0155's bet, as an assertion.**
    ///
    /// The record chose `egui` and paid a `wgpu` major version for it on the
    /// grounds that the panel and the engine share one `Device`. This builds
    /// the console's frame with an `egui` context, tessellates it, renders it
    /// through `egui-wgpu` into a texture on a device `karakuri-engine`
    /// created, and reads the texels back. If the two ever resolve different
    /// `wgpu`s it does not compile; if the render path breaks, `poll` gives a
    /// device-side complaint somewhere to surface; and if what lands is not
    /// the console, the two pixels below say so.
    ///
    /// **The pixels are the point.** A frame that renders without complaining
    /// and is the wrong colour is the failure that is easy to ship: `egui`'s
    /// shader writes gamma-encoded texels because it is told the target is
    /// gamma space, so an sRGB target encodes a second time and the whole
    /// panel washes out — with no error anywhere. So a bay's body is asserted
    /// to be exactly `--c-panel` and a divider is asserted not to be.
    #[test]
    fn egui_paints_the_console_onto_a_device() {
        // Gamma space, not sRGB: see above, and the window's own choice of
        // surface format, which is made for this reason.
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        // 1408 rather than 1440 for one reason: `copy_texture_to_buffer` wants
        // `bytes_per_row` a multiple of 256, and 1408 * 4 is 5632.
        const W: u32 = 1408;
        const H: u32 = 900;
        const ROOM: Room = Room::Night;

        let gpu = Gpu::headless().expect("no GPU");
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("console probe"),
            size: wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let ctx = egui::Context::default();
        let mut panel = Panel::new(W as f32, H as f32);
        let mut view = View::new(ROOM);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(W as f32, H as f32),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| view.draw(ui, &mut panel));
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        // The console has eleven regions and seven headings, so a frame that
        // tessellated to nothing is a frame that drew nothing.
        assert!(
            !primitives.is_empty(),
            "the console tessellated to no primitives at all"
        );

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [W, H],
            pixels_per_point: 1.0,
        };
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("console probe"),
            });
        let user =
            renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &primitives, &screen);
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("console probe"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
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
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
        output.textures_delta.clear();

        let row = W * 4;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("console probe"),
            size: (row * H) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(H),
                },
            },
            wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit(user.into_iter().chain([encoder.finish()]));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device stopped");
        let texels = readback.slice(..).get_mapped_range().expect("readback");
        let at = |p: karakuri_layout::Point| {
            let i = (p.y as u32 * row + p.x as u32 * 4) as usize;
            [texels[i], texels[i + 1], texels[i + 2]]
        };

        let pal = ROOM.palette();
        let panel_rgb = [pal.panel.r(), pal.panel.g(), pal.panel.b()];

        // A bay's body, well clear of its head and its edges.
        panel.solve();
        let library = panel
            .layout()
            .rect(panel.layout().find("library").expect("library"));
        let inside =
            karakuri_layout::Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);
        assert_eq!(
            at(inside),
            panel_rgb,
            "an empty bay's body is not --c-panel; the colour space or the palette is wrong"
        );

        // A divider: the ground shows through. Not `--c-ground` exactly, and
        // that is right rather than a tolerance — the bays either side cast
        // their shadow into the gap, as they do in the mock. So the assertion
        // is which of the two colours it is nearer, which is the question
        // "does the ground show through" and is not a threshold anybody has to
        // tune.
        let ground_rgb = [pal.ground.r(), pal.ground.g(), pal.ground.b()];
        let (split, index) = panel.layout().boundaries().next().expect("no boundary");
        let gap = panel.layout().boundary(split, index).expect("no pair");
        let in_gap = karakuri_layout::Point::new(gap.x + gap.w * 0.5, gap.y + gap.h * 0.5);
        let found = at(in_gap);
        let away = |from: [u8; 3]| -> i32 {
            (0..3)
                .map(|i| (found[i] as i32 - from[i] as i32).abs())
                .sum()
        };
        assert!(
            away(ground_rgb) < away(panel_rgb),
            "a divider at {found:?} is nearer the bay {panel_rgb:?} than the ground \
             {ground_rgb:?}, so the ground is not showing through"
        );
    }
}
