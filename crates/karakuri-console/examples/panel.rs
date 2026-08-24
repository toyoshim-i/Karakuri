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
//!    through `Present` into the picture's rectangle, sampled by `egui` in the
//!    same submission. See [`Engine`].
//! 3. That the arrangement's numbers look right at real sizes against
//!    `docs/manual/console.html`.
//! 4. That dragging a boundary still works with a toolkit in the loop.
//! 5. **What the window costs**, printed once nobody has touched it for a
//!    while. It used to be *what a still panel costs*, and with a live picture
//!    in the Program bay there is no still panel to measure — so what is
//!    printed is the price of the thing that replaced it. See [`Costs::say`].
//!
//! **It is deliberately not the start of a bay.** Every body is empty except
//! the picture, and the picture is empty of everything this file could have
//! invented — no label, no frame, no placeholder; see
//! [`karakuri_console::view`].
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
use karakuri_console::view::{picture_rect, Kind, Picture, View};
use karakuri_engine::{Deck, Gpu, HotSwap, Present, Set};
use karakuri_layout::{Axis, NodeId, Point};
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
    /// **The engine's half**: `Deck::begin_frame` through the present pass
    /// into the picture — every install, the deck's render, and the recording
    /// of both. CPU time only, like everything else here; what the GPU then
    /// does with the command buffer is not on this clock, and
    /// `docs/contributing.md` §1 says why there is no other one.
    engine: Duration,
    /// `take_egui_input` through `tessellate`: the whole immediate-mode pass,
    /// including this console's own layout walk and every shape it emits.
    ui: Duration,
    /// Uploading the tessellated geometry, recording the panel's render pass,
    /// and the one submission that carries both halves. Excludes `present`,
    /// which is the display's pace and not a cost.
    paint: Duration,
    /// **The submission alone, out of [`Cost::paint`]** — `finish` on the
    /// frame's encoder and `Queue::submit` — and it is five sixths of it.
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
    said: bool,
    /// **Whether there is a live picture in the Program bay.** It decides
    /// which sentence the reading prints about the frames it counted, and
    /// nothing else: the frames are counted the same way either way, which is
    /// the point — the number is not adjusted for knowing the answer.
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
            said: false,
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
                // There is no still panel to cost while the picture is live,
                // and calling it one would be the reading describing a program
                // that is not running — which is the failure the sample this
                // replaces made, one revision ago.
                true => "what an untouched window costs with a live picture in it:",
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
        let rate = self.still.frames as f64 / STILL.as_secs_f64();
        match (self.live, self.still == Still::default()) {
            // The reading this was written for, and it is now only reachable
            // with the picture off.
            (false, true) => println!(
                "  so P-0072's first clause holds here: no per-frame work is done to \
                 redraw what nobody has touched and nothing has moved."
            ),
            (false, false) => println!(
                "  so P-0072's first clause does NOT hold here — something is asking \
                 for frames on an untouched window, and with no picture running the \
                 likeliest something is an `egui` repaint delay answered immediately \
                 instead of waited out."
            ),
            // **The expected reading now**, and the whole of what this run is
            // for. It is stated as a price rather than as a failure, because
            // that is what it is: the clause is about a panel with nothing
            // changing on it, and a live picture is something changing on it.
            (true, _) => {
                println!(
                    "  so P-0072's first clause has stopped holding, and the reason is the \
                     picture: there is a live engine frame in the Program bay, so every \
                     one of those frames was asked for by the picture rather than by \
                     anybody touching the window."
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
                 Program bay, and the rate above with a picture in it."
            );
            println!(
                "  the panel half is taken on this window at {:.0}x{:.0} logical with every \
                 other bay empty, which is NOT the workspace's reference workload. The \
                 engine half IS: one Set of {} elements at {}x{}, one step a frame, \
                 letterboxed into the picture's region \
                 (docs/contributing.md §1). Host clock, debug profile with dependencies \
                 at opt-level 3.",
                WINDOW.0, WINDOW.1, CAPACITY, CANVAS.0, CANVAS.1
            );
            println!(
                "  and every figure above is taken on a core that spends the vsync wait \
                 asleep, so it is measured at the clock a mostly-idle machine runs at: \
                 the identical run with this machine's other cores loaded reports about a \
                 sixth of these numbers, with the proportions between them unchanged."
            );
        }
        println!();
        println!(
            "{}",
            match self.live {
                true =>
                    "the loop asks for the next frame from inside the last one for as long \
                     as the picture is live, so `ControlFlow::Wait` never gets to block. \
                     Fold the picture away (f over it) and it does — that is the same \
                     window drawing nothing, and it is what P-0072's remaining clauses are \
                     for.",
                false =>
                    "the loop is on `ControlFlow::Wait` from here: it does nothing at all \
                     until the window is touched or `egui` names a deadline of its own.",
            }
        );
        println!();
    }
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
                    Op::FoldEnclosing => "the split ",
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
            Outcome::OnDivider { split, index } => println!(
                "fold: the pointer is on divider {}#{index} — move it into a region",
                self.label(*split)
            ),
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
            // The three operations that act on what is under the pointer are
            // the only ones that can find nothing there.
            Outcome::Nothing => println!(
                "{}",
                match op {
                    Op::Fold => "fold: nothing under the pointer".to_owned(),
                    Op::FoldEnclosing => "fold: nothing encloses the pointer".to_owned(),
                    Op::Solo => "solo: no region under the pointer".to_owned(),
                    other => format!("{other:?}: nothing under the pointer"),
                }
            ),
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
    fn pointer(&mut self, event: Pointer) -> Claim {
        let at = match event {
            Pointer::Moved(p) => p,
            _ => self.panel.cursor(),
        };
        // Asked **before** anything acts. `released` takes the drag out of
        // hand, so a claim asked after it would see no drag, route the release
        // to `egui`, and hand `egui` a button-up it never saw the button-down
        // for.
        let claim = claim(&mut self.panel, at);
        match (event, claim) {
            // The panel learns where the pointer is either way — every
            // keyboard operation is addressed to it — and drags if a boundary
            // is in hand. Whether `egui` is also told is the claim.
            (Pointer::Moved(p), _) => self.moved(p),
            (Pointer::Down, Claim::Panel) => self.press(at),
            (Pointer::Up, Claim::Panel) => self.released(),
            (Pointer::Down | Pointer::Up | Pointer::Wheel, _) => {}
        }
        claim
    }

    // -- the legend -----------------------------------------------------

    fn print_legend(&mut self) {
        self.panel.solve();
        let layout = self.panel.layout();
        let viewport = layout.viewport();
        println!();
        println!(
            "the console, in a {:.0} x {:.0} viewport. every leaf gets its region; seven of \
             them get a bay head, and every body is empty.",
            viewport.w, viewport.h
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
                    Kind::Row => "row, no heading".to_owned(),
                    Kind::Pane => "pane, inside a bay".to_owned(),
                    Kind::Picture => "the picture, a sink".to_owned(),
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
        println!("keys — the pointer's position decides what each one acts on:");
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
/// size of the picture** — see [`Present::draw`], which letterboxes this into
/// whatever it is drawn into, and which is what the manual means by *"it
/// letterboxes into the width it has"*.
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

/// What the picture is rendered in: an sRGB format, so the hardware does the
/// one encode `Present`'s shader relies on ([P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)).
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

/// **The engine behind the picture: a deck of one Set, the present pass, and
/// the texture the two of them land in.**
///
/// Scaffolding, and it looks it: one slot, no audio, no MIDI, no store, no
/// arguments. What `karakuri-cli` does around this is a program; what is here
/// is the shortest path from two `.kir` files to texels, which is the whole of
/// what the Program bay needs to be shown to be reachable.
struct Engine {
    deck: Deck,
    present: Present,
    /// The picture. Held because the views below are of it, and because
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
    /// The texture's size in physical pixels — **the picture region's**, not
    /// the window's.
    size: (u32, u32),
    /// How many registrations have been freed. The atlas leak this exists to
    /// prevent is invisible from outside: a resize that registers without
    /// freeing leaves a bind group per drag frame and nothing says so, so the
    /// count is kept and `mod gpu` asserts on it.
    freed: usize,
}

impl Engine {
    fn new(gpu: &Gpu, renderer: &mut egui_wgpu::Renderer, size: (u32, u32)) -> Engine {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let l1 = checked(&root.join("examples/drift_shell.kir"));
        let l4 = checked(&root.join("examples/soft_points.kir"));
        let set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, CAPACITY, SEED_SALT)
            .expect("the example pair builds a Set");
        let deck = Deck::new(&gpu.device, vec![HotSwap::fixed(set)], CANVAS.0, CANVAS.1);
        let present = Present::new(&gpu.device, PICTURE_FORMAT, CANVAS.0, CANVAS.1);
        let (texture, target, id) = picture(gpu, renderer, size);
        Engine {
            deck,
            present,
            texture,
            target,
            id,
            size,
            freed: 0,
        }
    }

    /// **The picture's region is a different size, so the texture is remade at
    /// that size and the registration it replaces is freed.**
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
    fn fit(&mut self, gpu: &Gpu, renderer: &mut egui_wgpu::Renderer, size: (u32, u32)) -> bool {
        if size == self.size {
            return false;
        }
        renderer.free_texture(&self.id);
        self.freed += 1;
        let (texture, target, id) = picture(gpu, renderer, size);
        self.texture = texture;
        self.target = target;
        self.id = id;
        self.size = size;
        true
    }
}

/// The picture's texture, the view the engine draws into, and the
/// registration `egui` reads it by. One function because the three are made
/// together and are replaced together.
fn picture(
    gpu: &Gpu,
    renderer: &mut egui_wgpu::Renderer,
    size: (u32, u32),
) -> (wgpu::Texture, wgpu::TextureView, egui::TextureId) {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("program view"),
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
        label: Some("program view, as egui reads it"),
        format: Some(PICTURE_SAMPLED_FORMAT),
        ..Default::default()
    });
    let id = renderer.register_native_texture(&gpu.device, &sampled, wgpu::FilterMode::Linear);
    (texture, target, id)
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

/// A rectangle in logical pixels, in physical ones. **The picture's texture is
/// sized from this and from nothing else**, which is what makes it the
/// region's size rather than the window's.
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
        let surface = instance.create_surface(window.clone()).expect("surface");
        let gpu = pollster::block_on(Gpu::from_instance(instance, Some(&surface))).expect("gpu");

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
        // corrects. `fit` takes it from here on.
        let mut renderer = renderer;
        self.readout.panel.solve();
        let first = picture_rect(self.readout.panel.layout())
            .map(|rect| physical(rect, self.scale as f32))
            .unwrap_or((1, 1));
        let engine = Engine::new(&gpu, &mut renderer, first);
        self.costs.live = true;

        self.readout.print_legend();

        // The first frame is owed to the window appearing, not drawn on a
        // still panel.
        self.costs.owes();
        window.request_redraw();
        self.gfx = Some(Gfx {
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
                let claim = self.readout.pointer(Pointer::Moved(p));
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
                let claim = self.readout.pointer(which);
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
            WindowEvent::MouseWheel { .. } => {
                let claim = self.readout.pointer(Pointer::Wheel);
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
                    Key::Character("f") => Op::Fold,
                    Key::Character("g") => Op::FoldEnclosing,
                    Key::Character("z") => Op::UnfoldAll,
                    Key::Character("s") => Op::Solo,
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

                // -- where the picture goes, and how big it is ---------
                // **Before the `egui` pass**, because the pass draws the
                // texture and a texture registered after it would be a frame
                // behind. The rectangle and the texture's size are the same
                // statement: one call to `picture_rect`, one conversion to
                // physical pixels, and both the region's rather than the
                // window's.
                self.readout.panel.solve();
                self.readout.view.picture = match picture_rect(self.readout.panel.layout()) {
                    Some(rect) => {
                        gfx.engine.fit(
                            &gfx.gpu,
                            &mut gfx.renderer,
                            physical(rect, self.scale as f32),
                        );
                        Some(Picture {
                            id: gfx.engine.id,
                            rect,
                        })
                    }
                    // Folded away: no rectangle, so no picture is drawn
                    // and — see below — no pass is recorded for one
                    // either.
                    None => None,
                };

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

                // -- the engine's pass, and it is first -----------------
                //
                // **One encoder and one submission, engine before panel.**
                // The encoder is the deck frame's, which is the only encoder
                // `karakuri-engine` will record a Set into — `Frame::encoder`
                // exists precisely so that a caller's own further work belongs
                // to the frame that produced it, and the present pass and a
                // readback in `karakuri-cli` are already recorded through it.
                // The panel's pass goes in after, so the picture is written
                // and then sampled inside one command buffer and `wgpu`
                // places the barrier between them.
                //
                // The alternative — an encoder of this file's own, submitted
                // beside the deck's — is two submissions whose order is the
                // queue's business rather than this file's, over a texture
                // that is a colour attachment in one and a sampled resource in
                // the other. It would work today and it would be a race
                // nobody wrote down.
                let started = Instant::now();
                let mut drawing = gfx.engine.deck.begin_frame(&gfx.gpu.device, &gfx.gpu.queue);
                // Rendered even where the picture is folded away, and that is
                // not the manual's *"no state where it is hidden and still
                // costing a pass"* being broken: the deck's own render is what
                // steps the simulation, and only the present pass into the
                // picture is skipped. Turning the deck off with the sink is
                // the sink's decision to make and there is no sink here yet.
                drawing.render(
                    gfx.engine.present.hdr_view(),
                    gfx.engine.present.size(),
                    STEPS_A_FRAME,
                );
                if self.readout.view.picture.is_some() {
                    // The canvas, letterboxed into the picture's rectangle —
                    // `Present::draw` does the fitting, which is the manual's
                    // *"it letterboxes into the width it has"* and is why
                    // nothing in this file computes an aspect ratio.
                    gfx.engine
                        .present
                        .draw(drawing.encoder(), &gfx.engine.target, gfx.engine.size);
                }
                cost.engine = started.elapsed();

                // -- the wgpu pass -------------------------------------
                let started = Instant::now();
                let screen = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [gfx.config.width, gfx.config.height],
                    pixels_per_point: output.pixels_per_point,
                };
                // One id can carry several deltas in a frame: a font atlas
                // that grew arrives as the whole image followed by its
                // patches, and applying only the first would leave holes.
                for (id, deltas) in &output.textures_delta.set {
                    for delta in deltas {
                        gfx.renderer
                            .update_texture(&gfx.gpu.device, &gfx.gpu.queue, *id, delta);
                    }
                }
                let user = gfx.renderer.update_buffers(
                    &gfx.gpu.device,
                    &gfx.gpu.queue,
                    drawing.encoder(),
                    &primitives,
                    &screen,
                );
                let view_target = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                {
                    let pass = drawing
                        .encoder()
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("console"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view_target,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    // The console's own ground is painted by
                                    // the central panel; this only matters for
                                    // the frame before the first one lands.
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                    gfx.renderer
                        .render(&mut pass.forget_lifetime(), &primitives, &screen);
                }
                for id in &output.textures_delta.free {
                    gfx.renderer.free_texture(id);
                }
                // **`epaint` panics on a `TexturesDelta` dropped unapplied**,
                // and a panic here is reached from a `winit` callback, which
                // on macOS is an abort rather than an error. Every delta above
                // has been handed to the renderer, so this says so.
                output.textures_delta.clear();
                // **`update_buffers` hands back a command buffer per `egui`
                // paint callback that asked for one**, and this console
                // registers no paint callbacks — the picture is a registered
                // texture drawn as an image, not a callback. So this is empty,
                // and an empty submission is skipped rather than made. It is
                // not dropped: a callback's prepared work submitted after the
                // pass that reads it would be a frame behind, so if one ever
                // appears it goes in ahead, and the engine and the panel stay
                // in the one submission below.
                let submitting = Instant::now();
                if !user.is_empty() {
                    gfx.gpu.queue.submit(user);
                }
                // The one submission: the deck's passes, the present pass into
                // the picture, and the panel's pass over the window, in the
                // order they were recorded.
                drawing.finish();
                cost.submit = submitting.elapsed();
                cost.paint = started.elapsed();

                gfx.gpu.queue.present(frame);

                self.costs.push(cost);
                App::wants(gfx, &mut self.egui_due, &mut self.costs, asked);
                // **The picture is live, so the next frame is asked for here
                // — and asked for without `Costs::owes`.**
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
                gfx.window.request_redraw();
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
            readout.pointer(Pointer::Moved(Point::new(start.x - 60.0, start.y))),
            Claim::Egui
        );
        assert_eq!(readout.pointer(Pointer::Moved(start)), Claim::Panel);
        assert_eq!(readout.pointer(Pointer::Down), Claim::Panel);

        // A hand does not stay on the boundary: it runs on across the panel,
        // and every one of these is inside a bay.
        for x in [start.x + 40.0, start.x + 120.0, start.x + 200.0] {
            assert_eq!(
                readout.pointer(Pointer::Moved(Point::new(x, start.y))),
                Claim::Panel,
                "the drag lost its claim at x = {x}"
            );
            // A wheel in the middle of a drag is the panel's too.
            assert_eq!(readout.pointer(Pointer::Wheel), Claim::Panel);
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
        assert_eq!(readout.pointer(Pointer::Moved(far)), Claim::Panel);
        assert_eq!(readout.pointer(Pointer::Up), Claim::Panel);

        readout.panel.solve();
        assert!(
            (pane_width(&readout) - 160.0).abs() < 0.01,
            "the left pane's stated minimum did not hold the drag: {}",
            pane_width(&readout)
        );

        // And afterwards the pointer, where it is standing, is egui's again.
        assert_eq!(readout.pointer(Pointer::Moved(far)), Claim::Egui);
    }

    /// The left pane's width, solved. A helper because the test asks three
    /// times and the chain is four calls long.
    fn pane_width(readout: &Readout) -> f32 {
        let layout = readout.panel.layout();
        layout.rect(layout.find("left-pane").expect("left-pane")).w
    }
}

#[cfg(test)]
mod gpu {
    //! The console, through `egui`, through `wgpu` 30, onto a real device.

    use super::*;

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
        let rect = picture_rect(panel.layout()).expect("the picture is on screen");
        let mut engine = Engine::new(&gpu, &mut renderer, physical(rect, 1.0));

        let mut view = View::new(ROOM);
        view.picture = Some(Picture {
            id: engine.id,
            rect,
        });

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

        // **The frame, in the order the window loop records it**: the deck's
        // encoder, the deck's render, the present pass into the picture, then
        // the panel over the top of all of it — one encoder, one submission.
        let mut drawing = engine.deck.begin_frame(&gpu.device, &gpu.queue);
        drawing.render(
            engine.present.hdr_view(),
            engine.present.size(),
            STEPS_A_FRAME,
        );
        engine
            .present
            .draw(drawing.encoder(), &engine.target, engine.size);
        let user = renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            drawing.encoder(),
            &primitives,
            &screen,
        );
        {
            let pass = drawing
                .encoder()
                .begin_render_pass(&wgpu::RenderPassDescriptor {
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
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
        output.textures_delta.clear();

        let row = W * 4;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("program probe"),
            size: (row * H) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        drawing.encoder().copy_texture_to_buffer(
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
        assert!(
            user.is_empty(),
            "a paint callback appeared: it has to be submitted ahead of the pass"
        );
        drawing.finish();

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
        let mut dark = 0usize;
        let mut bright = 0usize;
        let mut inside = 0usize;
        for y in rect.min.y as u32..rect.max.y as u32 {
            for x in rect.min.x as u32..rect.max.x as u32 {
                inside += 1;
                match brightest(at(x, y)) {
                    b if b <= 8 => dark += 1,
                    b if b >= 192 => bright += 1,
                    _ => {}
                }
            }
        }
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

        // **And it stayed in its region.** Two controls, because a picture
        // drawn over the whole window would satisfy the count above: the deck
        // preview row under it is still the bay's card, and so is a bay the
        // Program is nowhere near.
        let pal = ROOM.palette();
        let panel_rgb = [pal.panel.r(), pal.panel.g(), pal.panel.b()];
        let previews = panel
            .layout()
            .rect(panel.layout().find("deck-previews").expect("previews"));
        assert_eq!(
            at(
                (previews.x + previews.w * 0.5) as u32,
                (previews.y + previews.h * 0.5) as u32
            ),
            panel_rgb,
            "the picture painted over the deck previews, which are not its region"
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

    /// **The picture's texture is the size of its region, and a resize frees
    /// the registration it replaces.**
    ///
    /// Two claims that fail the same silent way. A texture sized from the
    /// window looks perfectly correct — the picture fills whatever rectangle
    /// it is given — and is wrong by however much the panel is not the
    /// picture, which here is most of it. And a `register_native_texture` with
    /// no `free_texture` beside it leaks a bind group and a sampler per remade
    /// frame: every frame of a drag on the program's height is one, and
    /// nothing on screen or in a log says a word about it.
    #[test]
    fn the_pictures_texture_is_its_regions_size_and_a_resize_frees_the_old_one() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let rect = picture_rect(panel.layout()).expect("on screen");
        let want = physical(rect, 1.0);
        let mut engine = Engine::new(&gpu, &mut renderer, want);

        // The region's, in both axes, and **neither of them is the window's**
        // — the picture is narrower than the window by both panes and taller
        // by nothing like the window's height.
        assert_eq!((engine.texture.width(), engine.texture.height()), want);
        assert_ne!(engine.size, (W, H));
        assert!(engine.size.0 < W && engine.size.1 < H / 2);
        assert!(renderer.texture(&engine.id).is_some());

        // A wider window is a wider picture and the same height, which is the
        // arrangement's "sized by height" arriving at the texture.
        let was = engine.id;
        panel.set_viewport(W as f32 + 400.0, H as f32);
        panel.solve();
        let grown = physical(picture_rect(panel.layout()).expect("on screen"), 1.0);
        assert_eq!(grown.1, want.1, "the height followed the window");
        assert_ne!(grown.0, want.0);

        assert!(
            engine.fit(&gpu, &mut renderer, grown),
            "a region that changed size did not remake the texture"
        );
        assert_eq!(engine.size, grown);
        assert_eq!((engine.texture.width(), engine.texture.height()), grown);
        assert_ne!(engine.id, was);
        assert!(renderer.texture(&engine.id).is_some());
        assert!(
            renderer.texture(&was).is_none(),
            "the registration the resize replaced is still in the atlas, so the atlas \
             grows once per dragged frame"
        );
        assert_eq!(engine.freed, 1);

        // And a frame where nothing moved remakes nothing, which is what keeps
        // all of the above on the resize path instead of on every frame.
        assert!(!engine.fit(&gpu, &mut renderer, grown));
        assert_eq!(engine.freed, 1);
        assert!(renderer.texture(&engine.id).is_some());
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
