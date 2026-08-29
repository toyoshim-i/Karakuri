//! **The instrument**: the console in a window, with the engine behind it.
//!
//! ```sh
//! cargo run -p karakuri                                # the pair the repository ships
//! cargo run -p karakuri -- geometry.kir renderer.kir   # a pair of your own
//! cargo test -p karakuri                               # this file's own tests
//! ```
//!
//! **This is the program, and it used to be an example.** It was
//! `karakuri-console/examples/panel.rs` until
//! [ADR-0214](../../../docs/adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md),
//! and it was an example for one reason: everything a program needs beyond the
//! panel lived in `karakuri-cli`, which has no library target, so there was
//! nothing for a binary to sit on. There is now — `karakuri-environment` — and
//! [ADR-0213](../../../docs/adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
//! is why the difference is not a packaging preference: a `has` badge in the
//! panel column of `docs/manual/operations.html` means **an operator running
//! the instrument reaches the operation**, and nobody plays a set from
//! `cargo run --example`.
//!
//! **It is still a package with no library target of its own, deliberately.**
//! Nothing may depend on this one. A surface is where the buck stops.
//!
//! # What it is for, and what became free
//!
//! The example this was carried five questions in its header. Four of them are
//! answered by this being a program at all — a person launches it and sees the
//! answer — and they are kept here because the *code* that answers them is what
//! moved:
//!
//! 1. That `egui` renders through `wgpu` 30 into this window at all, which is
//!    the bet
//!    [ADR-0155](../../../docs/adr/0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md)
//!    made.
//! 2. **That the engine's rendered texture reaches the panel**, which is the
//!    other half of that bet: a `Deck` on the same `Device`, with one Set of
//!    its two on air, drawn
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
//!
//! **The fifth is the one that is still a harness, and it is still here.**
//! [`Costs::say`] prints what the window costs once nobody has touched it for
//! three seconds, and to do it this binary installs a counting
//! [`#[global_allocator]`](Counting) over the whole process. That is a
//! measuring instrument inside the instrument, and it is not free: every
//! allocation a player's run makes pays a thread-local increment for it. It is
//! kept because nothing else measures these numbers and two of them are load
//! bearing — `karakuri_console::budget::PANEL_PASS` names this file as what
//! holds its 1.26 ms honest, and both schedulability conditions are asserted
//! against that number. **Whether a measurement harness belongs inside the
//! program a player launches is a decision this move did not take.** Half of
//! what it collects is not a harness at all: the per-frame timing is what the
//! transport row draws as `frame_ms` and `fps`.
//!
//! **It is deliberately not the start of a bay.** Every body is empty except
//! the picture and the preview row, and both are empty of everything this file
//! could have invented — no label, no frame, no placeholder; see
//! [`karakuri_console::view`].
//!
//! **There is one frame loop, and it is not in this file.**
//! `frame::compose` is `karakuri-engine`'s, and `karakuri-cli`'s window and
//! its PNG writer are the other two callers — which is the point of it: this
//! file used to hand-roll `begin_frame`, a render, a conditional present
//! pass per target, the panel and a submit, beside a loop in another crate
//! that said the same thing differently, and every replay defect this project
//! has found came from two such loops disagreeing.
//!
//! # The engine here is the shortest path from two files to texels
//!
//! Two slots, built from one `.kir` pair the way `karakuri-cli` builds them,
//! and **no more than that**: no audio, no MIDI, no MCP, no watcher, no replay
//! and no session. The pair is [`Sources`], and it is the whole of what this
//! program takes from the command line. Two things beyond the engine are here.
//! **The store is opened to be read** — once, at
//! startup, so the Library bay has names to list ([`library`]) — and it is
//! neither created nor written to. And **records exist**: a mixer control
//! emits an operation, `karakuri-operation-record` turns it into a `Record`,
//! and [`apply`] is what moves the deck with it, because
//! [P-0028](../../../docs/principles/0028-every-control-ends-in-the-same-record.md)
//! is that every control ends in the same record. Nothing here reaches a disk
//! either way, and no record stream drives time.
//!
//! **What is missing is named rather than left to be noticed.** Audio, MIDI,
//! MCP, the watcher and replay are all `karakuri-environment`'s and all
//! reachable from here; none of them is wired up in this first version,
//! because a slice that added them would be unreviewable. `karakuri-cli` is
//! still what you play a set with while that is true.
//!
//! **There is a governor, and it is the one thing here that is not the
//! shortest path.** It runs once, at startup, and it is [`Engine::ask_to_prime`]:
//! deck A is Live and deck B is asked to prime against a budget that has no
//! room for it, so the governor parks it. That is the only way a slot on this
//! panel can read one residency and have been asked for another —
//! `Deck::set_residency` writes the request *and* grants it, and only a
//! [`Deck::govern`] pass can hold a slot below what was asked for. Without it
//! the two residencies agree on every slot and every frame, and the roll
//! [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
//! drew could not be seen by running this window.
//!
//! **Two slots, and three of the four preview cells are still off.** Deck A
//! gets a live cell; deck B is parked, which is not stepping and not drawn, so
//! there is nothing to audition in its cell; and C and D have nothing behind
//! them at all. `off` is a state an operator chooses rather than a thing not
//! built yet, and all three are the truth about this program rather than a gap
//! in it.
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
//! [`missed`], carried over from the example that preceded this one, and
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

// **The toolkit comes from `karakuri-console` and is not named in this
// package's manifest.** `egui`, `egui-wgpu` and `egui-winit` move together and
// `egui-wgpu` is what pins `wgpu`; that crate's `lib.rs` re-exports all three
// and says why in as many words — *"a window loop written against this crate
// takes all three from here"*. Depending on them directly would let this
// package pick an `egui-wgpu` that wants a different `wgpu`, and then the
// engine and the panel cannot share a `Device`, which is the whole of what
// ADR-0155 paid a major version for.
use karakuri_console::{egui, egui_wgpu, egui_winit};

use karakuri_console::budget::PANEL_PASS;
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{
    Dragged, InHand, Knob, Op, Outcome, Panel, Pressed, Released, Visibility,
};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::Room;
use karakuri_console::view::{
    self, mixer as mixer_bay, outputs, picture_rect, preview_rects, Kind, Picture, View, DECKS,
    DECK_LETTERS,
};
use karakuri_engine::governor::{Report, SLOWEST_PRIME_ONE_IN};
use karakuri_engine::{
    compose, Blend, Committed, Control, Deck, Gpu, HotSwap, Look, Mask, MaskKind, Present,
    Residency, Set, Sink, Skip, TonemapOp,
};
use karakuri_environment::mix;
use karakuri_layout::{Axis, Hit, NodeId, Point};
use karakuri_operation::{BlendMode, Operation};
use karakuri_operation_record::{written, Current, Written};
use karakuri_store::record::Record;
use karakuri_store::store::Store;
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

/// **What the `egui` pass on this panel last read**, and the day it was read
/// — the figure the reading below quotes, and the figure it holds the run it
/// has just taken against.
///
/// It is a constant rather than a sentence because a number written into prose
/// reads as current forever, and this one did. The line that quotes it cited
/// [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s
/// 184 allocations and 226.2 kB and said the `egui` pass "is still that",
/// which stopped being true when the mixer bay landed — 456 there, and 525
/// once deck B was parked
/// ([ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)).
/// Nobody re-checked it for two commits, because nothing was checking it.
///
/// **Taken on 2026-08-26**, over nine runs of this program on an Apple M4 Pro
/// with nothing touching the window: per-frame medians of 524 to 538
/// allocations and 671.4 to 695.3 kB, of which the two below are the middle.
/// What the panel had in it while they were taken is the last paragraph the
/// reading prints. Re-take all three together, several runs at a time — one run
/// is not a number here — and re-date them.
const WRITTEN_ALLOCS: u64 = 525;
const WRITTEN_KB: f64 = 694.3;
const WRITTEN_ON: &str = "2026-08-26";

/// How far a run may sit from [`WRITTEN_ALLOCS`] before the reading says the
/// sentence quoting it has gone stale.
///
/// **A factor, and a generous one, because an allocation count is not a
/// constant**: a hard equality here would be a guard nobody could keep
/// passing, and this file's own nine runs disagree by 14 allocations. Two is
/// the smallest factor that still catches what actually happened — 184 to the
/// mixer bay's 456 is 2.5x, so a band of two would have said so on the first
/// run after that bay landed, and a band of ten would not have.
///
/// **Two figures are held and the bytes are not**, which is a distinction
/// rather than an omission: the bytes move with the allocation count, so a
/// verdict on them would be the same verdict twice. The second is
/// [`PANEL_PASS`] — what one update of a live region costs, declared under
/// [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)
/// and read by `tests/schedulable.rs` rather than by anything at runtime. It
/// is a different claim from an allocation count and it is the one a
/// schedulability condition is asserted against, so it gets its own verdict.
const DRIFT: f64 = 2.0;

/// **Has the reading moved away from the sentence that quotes it?** `Some` is
/// the factor between them, where that factor is past [`DRIFT`] in either
/// direction.
///
/// Either direction on purpose: a pass that got cheaper makes the sentence
/// exactly as untrue as one that got dearer, and only one of those two is ever
/// noticed by accident.
fn drifted(measured: u64, written: u64) -> Option<f64> {
    let factor = measured.max(written) as f64 / measured.min(written).max(1) as f64;
    (factor > DRIFT).then_some(factor)
}

/// [`drifted`] for a figure in milliseconds, which is what a **cost** is.
///
/// The same band and the same both-directions rule, said again for `f64`
/// because the two numbers are of different kinds and neither is convertible
/// into the other without saying something untrue about it. A run that read
/// nothing at all cannot divide, and a zero-length sample never reaches here —
/// the caller is inside `if !self.frames.is_empty()`.
fn drifted_ms(measured: f64, written: f64) -> Option<f64> {
    let factor = measured.max(written) / measured.min(written).max(f64::MIN_POSITIVE);
    (factor > DRIFT).then_some(factor)
}

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
    /// program records from inside [`compose`]'s `finally`, plus `compose`'s
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
    /// of this program's are `Ok(())` — it is five sixths of `paint`.
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
    /// **What the panel declared it needed**, as `View::animating` answered it
    /// on the last frame: the soonest staleness out of the regions that are
    /// declaring, and `None` only when none of them is.
    ///
    /// It is here for one sentence, and the sentence was wrong without it.
    /// With nothing in the Program bay making texels the reading used to blame
    /// the only other thing it knew about — an `egui` repaint delay answered
    /// immediately — and on this program that is never the answer: folding the
    /// picture and the preview row away leaves the window drawing 28.0 to 28.3
    /// frames a second over two runs on 2026-08-26, which is the roll's
    /// declared 30 Hz and not a mishandled delay. Folding the mixer bay away
    /// as well takes it to 0 frames, because the chip that declares the 30 Hz
    /// is then not laid out (ADR-0193).
    /// A reading that names the wrong cause is worse than one that names none.
    ///
    /// **Those two readings were taken before the beat declared**, and the
    /// beat declares whenever the transport row is drawn and there is an
    /// engine behind it — 24.7 ms, about forty a second, whether or not
    /// anything is pending
    /// ([ADR-0212](../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md),
    /// [P-0077](../../../docs/principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md)).
    /// So this window's `None` now needs the transport row folded away as
    /// well, which is a **fourth** fold and is the arm below saying so.
    declared: Option<Duration>,
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
            declared: None,
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
    /// and then no reading is printed at all. **A run of this program that
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
    /// `capacity` and `material` are the run's, handed in rather than read off
    /// a constant: this program takes its `.kir` pair from the command line, so
    /// what the engine half of this reading was taken over is only known at run
    /// time. See [`Engine::capacity`] and [`Sources::material`].
    fn say(&mut self, capacity: u32, material: &str) {
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
            // The reading this was written for. It is reachable with the
            // picture, the preview row, the mixer bay **and the transport
            // row** folded away — the first two stop the texels and the last
            // two stop the two declarations (ADR-0193) — and with any one of
            // the four on screen it is not. The fourth is P-0077 arriving:
            // the beat is a light travelling the grid, it declares for as
            // long as it is drawn, and a console claiming to show a live
            // instrument has something moving on it (ADR-0212).
            (false, true) => println!(
                "  so P-0072's first clause holds here: no per-frame work is done to \
                 redraw what nobody has touched and nothing has moved."
            ),
            (false, false) => match self.declared {
                // **The panel said it needed them**, and that is P-0072's
                // second clause working rather than its first failing: the
                // parked deck's tally declares a staleness and the window
                // serves it. Measured on 2026-08-26 with the picture and the
                // preview row folded away and the mixer bay on screen: 28.0
                // and 28.3 frames a second over two runs, at 427 and 425
                // allocations a frame.
                //
                // **Folding the mixer bay away takes this arm out of reach**,
                // and that is ADR-0193: the slot stays parked, the chip is not
                // drawn, and a region that is not laid out declares nothing —
                // the same two runs read 0 frames with the bay folded, which
                // is the arm above. It read 28.7 to 29.0 a second at 260
                // allocations before that change, which is the defect that
                // record closes.
                Some(staleness) => println!(
                    "  so P-0072's first clause does NOT hold here, and the reason is a \
                     declaration rather than a fault: {}, and the soonest staleness \
                     declared is {:.1} ms — about {:.0} frames a second. Folding the \
                     region that draws it ends its term: a region that is not laid out \
                     declares nothing (ADR-0193).",
                    match staleness == view::BEAT_STALENESS {
                        // The one that runs whether or not anything is
                        // happening, which is the whole of why it is here.
                        true =>
                            "the beat grid is a light travelling the transport row, \
                                 and it moves for as long as the console is live rather \
                                 than while something is pending (P-0077, ADR-0212)",
                        false =>
                            "something on this panel is parked and the mixer's tally \
                                  is rolling toward a residency nobody granted (ADR-0190)",
                    },
                    staleness.as_secs_f64() * 1000.0,
                    1.0 / staleness.as_secs_f64()
                ),
                // Nothing live, nothing declared, and frames drawn anyway.
                None => println!(
                    "  so P-0072's first clause does NOT hold here — something is asking \
                     for frames on an untouched window, nothing on the panel is making \
                     texels and nothing has declared a staleness, so the likeliest \
                     something is an `egui` repaint delay answered immediately instead of \
                     waited out."
                ),
            },
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
                     price — and this is the price, measured. Two regions declare it: the \
                     transport row for as long as the beat grid is drawn (P-0077, \
                     ADR-0212) and the mixer bay while something in it is pending. \
                     Nothing in this run schedules, caches the panel to a texture or \
                     arbitrates between the two; the number is what the next decision gets \
                     made on."
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
            // **What drawing the panel costs**, which is the figure
            // `karakuri_console::budget::PANEL_PASS` declares and the reason
            // this vector exists: the immediate-mode pass, plus the panel's
            // own texture and geometry uploads and the recording of its render
            // pass. Not `engine`, which is the governor's and would be counted
            // twice (P-0072); not `wait`, which is doing nothing on purpose;
            // and **not `submit`**, which is larger than all of this together
            // and carries the engine's half of the frame as well, so charging
            // it to a region would charge a region for a frame it did not ask
            // for.
            let mut draw: Vec<f64> = self
                .frames
                .iter()
                .map(|c| ms(c.ui + c.textures + c.buffers + c.record))
                .collect();
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
            draw.sort_by(f64::total_cmp);
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
                "  that per-frame price is what immediate mode pays by construction, and it \
                 is no longer ADR-0164's: that record measured 184 allocations and 226.2 kB \
                 a frame here with every bay empty, which this panel has not been since the \
                 mixer bay landed — 456 allocations there, and 525 once deck B was parked \
                 (ADR-0191). Taken again on {WRITTEN_ON} over nine runs of this program \
                 with nothing touching the window: {WRITTEN_ALLOCS} allocations and \
                 {WRITTEN_KB:.1} kB a frame in the middle of the nine, which spread 524 to \
                 538 and 671.4 to 695.3 kB. What the panel had in it while they were taken \
                 is the last paragraph below. What ADR-0164 is still right about is that \
                 the price is paid on every frame drawn; what changed is how many frames pay \
                 it — 0 with a still panel and nothing in the Program bay, and the rate above \
                 with anything live in it."
            );
            // **The sentence above is checked against the run that has just
            // been taken**, which is the only place either can be: the number
            // needs a window, three seconds of nobody touching it and a
            // device, and none of those is reachable from `cargo test`. So the
            // claim and its check are printed together, and the figure in the
            // prose is the figure being checked rather than a second copy of
            // it.
            match drifted(allocs[n / 2], WRITTEN_ALLOCS) {
                Some(factor) => println!(
                    "  and THIS run read {}, which is {factor:.1}x that — past the {DRIFT:.0}x \
                     this file will quote a figure across. **The sentence above is stale.** \
                     Re-take it over several runs of this program, write what the panel had in \
                     it, and re-date `WRITTEN_ALLOCS`, `WRITTEN_KB` and `WRITTEN_ON` in \
                     `crates/karakuri/src/main.rs` — which is what nobody did for the two commits before \
                     this line existed.",
                    allocs[n / 2]
                ),
                None => println!(
                    "  and THIS run read {}, within {DRIFT:.0}x of that, so the sentence above \
                     is still one this window produces.",
                    allocs[n / 2]
                ),
            }
            // **What P-0072 calls a cost, measured and held against what
            // declares it.** `budget::PANEL_PASS` is a constant somebody wrote
            // down — ADR-0164 refuses a schedule made of measurements, because
            // one reorders itself with the machine's noise — and a constant
            // that nothing checks is the failure `WRITTEN_ALLOCS` above exists
            // for, one number along. So the declaration and the reading are
            // printed together, and this is the only place either can be: the
            // figure needs a window, three seconds of nobody touching it and a
            // device, and none of those is reachable from `cargo test`.
            //
            // **It reports and does not fail**, which is deliberate. This
            // machine reads about a sixth of these numbers with its other
            // cores loaded, and a gate on a millisecond here would be one
            // nobody could keep passing — flaky is worse than broken
            // (`docs/contributing.md` §1). What *is* asserted, without a
            // clock, is that every region declares this one constant:
            // `tests/schedulable.rs`.
            println!(
                "  drawing the panel is a median {:.3} ms of that — the egui pass, its \
                 texture and geometry uploads and the recording of its render pass, which \
                 is what one update of a live region costs under P-0072. The submission is \
                 not in it: it carries the engine's half of the frame as well. \
                 `karakuri_console::budget::PANEL_PASS` declares {:.3} ms,",
                draw[n / 2],
                ms(PANEL_PASS),
            );
            match drifted_ms(draw[n / 2], ms(PANEL_PASS)) {
                Some(factor) => println!(
                    "  and THIS run read {:.3}, which is {factor:.1}x that — past the \
                     {DRIFT:.0}x this file will quote a figure across. **The declared cost \
                     is stale.** Re-take it over several runs of this program and rewrite \
                     `PANEL_PASS` in `crates/karakuri-console/src/budget.rs`, with the \
                     machine and the date beside it, because both schedulability \
                     conditions are asserted against that number and nothing else measures \
                     it.",
                    draw[n / 2]
                ),
                None => println!(
                    "  and THIS run read {:.3}, within {DRIFT:.0}x of that, so the declared \
                     cost is still one this window produces.",
                    draw[n / 2]
                ),
            }
            println!("  taken on {}", self.taken_on);
            println!(
                "  the panel half is taken on this window at {:.0}x{:.0} logical, drawing a \
                 live picture, four preview cells with deck A auditioning in one and three \
                 off, the mixer bay, the transport, the outputs row and deck B's parked \
                 tally rolling once a second — over Library, Staging, Inspector, Master and \
                 Sequencer, which are a head and nothing else. That is NOT the workspace's \
                 reference workload. The \
                 engine half is one Set of `{}` — {} elements at {}x{}, one step a frame — the \
                 deck's second slot is parked, and a parked slot neither steps nor draws, \
                 so it is in none of these numbers — and \
                 that one canvas presented twice — into the picture's rectangle, \
                 and again into deck A's preview cell, both of which are the \
                 canvas's own shape. It is the workspace's reference workload \
                 (docs/contributing.md §1) only while nobody passed a pair on the command \
                 line: 262144 elements at 1280x720 is what `drift_shell.kir` declares and \
                 what a bare `cargo run -p karakuri` therefore runs, and the figures above \
                 are comparable with the rest of this repository's exactly that far. Host \
                 clock, debug profile with dependencies at opt-level 3.",
                WINDOW.0, WINDOW.1, material, capacity, CANVAS.0, CANVAS.1
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
                     and nothing is making texels — and the window still draws, because \
                     the beat grid declares a deadline of its own for as long as it is on \
                     screen (P-0077, ADR-0212) and the mixer bay declares another while \
                     deck B is parked. `ControlFlow::Wait` blocks when all three are gone, \
                     which is a fourth fold, and it is what P-0072's remaining clauses are \
                     for.",
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

    /// A move with something in hand. A boundary drag says what it did here;
    /// **a fader drag hands its operation back**, because acting on one takes
    /// the deck and the deck is not the readout's.
    fn moved(&mut self, p: Point) -> Option<Operation> {
        match self.panel.moved(p)? {
            boundary @ Dragged::Boundary { .. } => {
                println!("{}", self.say_drag(boundary));
                None
            }
            Dragged::Fader(operation) => Some(operation),
        }
    }

    /// A drag, in words: what was asked, where it landed, what held it, and
    /// what the pair either side is now.
    fn say_drag(&self, d: Dragged) -> String {
        let Dragged::Boundary {
            split,
            index,
            axis,
            asked,
            landed,
            held,
        } = d
        else {
            // A fader's words are the window loop's, because they are about
            // what happened to the *deck* after the operation left here.
            unreachable!("a fader drag says its own line")
        };
        let sizes = match self.panel.pair(split, index) {
            Some((a, b)) => format!(
                "{} {:.0} | {} {:.0}",
                self.label(a),
                axis.extent(self.panel.layout().rect(a)),
                self.label(b),
                axis.extent(self.panel.layout().rect(b))
            ),
            None => "no pair".to_owned(),
        };
        let stop = match held {
            Some(by) => format!(" — held {by:+.1} by a stop, and it stays there until it moves"),
            None => String::new(),
        };
        format!("  drag: asked {asked:.1}, landed {landed:.1}{stop} [{sizes}]")
    }

    fn released(&mut self) {
        match self.panel.released() {
            Some(Released::Rests { split, index, at }) => {
                println!("release: {} rests at {at:.1}", self.pair(split, index))
            }
            Some(Released::Gone { split, index }) => {
                println!("release: {} is gone", self.pair(split, index))
            }
            // **No value in the line, because there is none to print.** Where
            // a fader came to rest is the deck's, and the last thing the drag
            // asked for was printed when it was asked for.
            Some(Released::Let { deck, knob }) => println!(
                "release: deck {} lets go of the {}",
                deck_letter(deck),
                knob_word(knob)
            ),
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
    fn pointer(&mut self, ctx: &egui::Context, event: Pointer) -> (Claim, Acted) {
        let at = match event {
            Pointer::Moved(p) => p,
            _ => self.panel.cursor(),
        };
        // Asked **before** anything acts. `released` takes the drag out of
        // hand, so a claim asked after it would see no drag, route the release
        // to `egui`, and hand `egui` a button-up it never saw the button-down
        // for.
        let claim = claim(&mut self.panel, ctx, &self.view.mixer, at);
        let mut did = Acted::Nothing;
        match (event, claim) {
            // The panel learns where the pointer is either way — every
            // keyboard operation is addressed to it — and drags if something
            // is in hand. Whether `egui` is also told is the claim.
            //
            // **A move is what a fader emits on**, so this is the one arm that
            // can act without a button, and it acts whoever the claim went to:
            // rule 1 has already given the panel any drag in hand.
            //
            // **Asked before the move, and only with a fader in hand.** An
            // `Emitted(None)` is *a fader that did not change*, which is owed
            // no frame; a plain pointer move with nothing in hand is
            // `Change::Pointer`'s business and is owed one whenever the panel
            // claimed it, because that is the resize cursor going on and off.
            // Answering `Emitted(None)` for both would take the cursor with it.
            (Pointer::Moved(p), _) => {
                let fading = matches!(self.panel.in_hand(), Some(InHand::Fader));
                let operation = self.moved(p);
                if fading {
                    did = Acted::Emitted(operation);
                }
            }
            // **A press the panel claimed is on one of the five controls or
            // on the panel itself**, and the controls are asked first for the
            // reason `claim` asked them last: rule 2 has already had its
            // refusal, so a press that got here and is on a control is that
            // control's. All five are the same calls `claim` made — asked
            // again, not copied.
            //
            // **The bay is derived once and asked four times**, exactly as
            // `claim` does it: a knob, a blend chip, a tally chip and a mask
            // mini are four questions about one laid-out strip, and four
            // derivations would be four answers.
            (Pointer::Down, Claim::Panel) => {
                self.panel.solve();
                let sink = outputs(ctx, self.panel.layout()).filter(|row| row.hit(at));
                let bay = mixer_bay(ctx, self.panel.layout(), &self.view.mixer);
                let knob = bay.as_ref().and_then(|bay| bay.grab(at));
                let chip = bay.as_ref().and_then(|bay| bay.blend(at));
                let tally = bay.as_ref().and_then(|bay| bay.tally(at));
                let mask = bay.as_ref().and_then(|bay| bay.mask(at));
                match (sink, knob, chip, tally, mask) {
                    (Some(row), ..) => did = Acted::Operated(self.sink(row.op())),
                    // **The value does not move on the press.** The grab keeps
                    // the offset it took hold at, so the first move continues
                    // from where the knob already was — and a press that was
                    // on the *track* never gets here, because `Mixer::grab`
                    // answers `None` for it rather than jumping the mix.
                    (None, Some(grab), ..) => {
                        println!(
                            "press ({:.0}, {:.0}): deck {} — the {} is in hand",
                            at.x,
                            at.y,
                            deck_letter(grab.deck()),
                            knob_word(grab.knob())
                        );
                        self.panel.grab(at, grab);
                    }
                    // **A chip acts on the press itself**, where a fader acts
                    // on the moves after it: there is no gesture here, only
                    // one operation naming where the cycle arrived. Both go
                    // down the same path a fader's does — `Acted::Emitted`,
                    // then a record, then the deck — because P-0028 is that
                    // every control ends in the same record, and a chip that
                    // reached the deck another way would be a second route for
                    // the same change.
                    //
                    // **One arm for the three chips**, because what this file
                    // does with any of them is the same three steps; which chip
                    // it was is in the operation, and the line `apply` prints
                    // says so.
                    (None, None, Some(operation), ..)
                    | (None, None, None, Some(operation), _)
                    | (None, None, None, None, Some(operation)) => {
                        did = Acted::Emitted(Some(operation))
                    }
                    (None, None, None, None, None) => self.press(at),
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

    fn print_legend(&mut self, budget_ms: Option<f32>, governed: &Report) {
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
            "B, C and D read `off`, and for two different reasons. C and D have nothing \
             behind them: this program's engine is a deck of TWO slots. B has a deck behind \
             it and no audition — it is PARKED, which is not stepping and not drawn, so \
             there is nothing for a cell to show. three cells saying off are what this \
             program is rather than something left unfinished, and each cell that is on \
             costs a present pass of its own."
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
             over machinery that is in neither this crate nor this program: audio-in, tap, \
             learn, map, landed and rec. `view::transport` names them one by one with what \
             is missing behind each."
        );
        println!(
            "the mixer draws {} strip{}, because a strip is a deck SLOT and this deck has \
             {} — the mock's four is the most a deck can hold, and the page keeps its four \
             tracks either way, so a track with nothing behind it is empty rather than a \
             strip full of dashes. the two FADERS are played: drag the knob on the trim or \
             on the tall fader and the panel emits SetGain or SetOpacity, which \
             karakuri-operation-record turns into a Record — the one place an \
             operation becomes one, for every surface — and this file applies to the \
             deck. the strip then follows because the DECK changed, not because \
             anything here remembered. a press on the track \
             off the knob does nothing, deliberately: a fader at 0.3 whose top is clicked \
             must not jump to 1.0 on stage. everything else in the strip is a READOUT.",
            self.view.mixer.len(),
            match self.view.mixer.len() {
                1 => "",
                _ => "s",
            },
            self.view.mixer.len()
        );
        // **The park, said in this file's voice and then in the engine's.**
        // The sentence is this program's, because the words on this window are;
        // the numbers under it are `Report`'s own `Display` and the same three
        // fields `karakuri-cli`'s `report_governing` prints, so an operator
        // reading a park here and a park there is reading one thing.
        println!(
            "deck B's strip is the one that MOVES: its chip reads ALLOC and rolls part of \
             the way toward PRIM once a second and falls back, never landing, because what \
             the slot was asked for and what it is doing disagree. this program asks for B \
             to be primed at startup and the budget has no room, so `Deck::govern` holds it \
             at allocated with the request intact — that is a PARK, which is `not now` and \
             not `no`: nothing has to be asked twice, and the next pass over a deck with \
             room admits it. nothing in this file writes a residency or draws a park; the \
             strip carries both of the deck's own words for that slot and the view derives \
             the rest. what the governor decided, in its own words:"
        );
        println!("  {governed}");
        for decision in governed.parked() {
            println!(
                "  slot {} (deck {}) parked, request held: {:?} — {}, against {}, and one \
                 step in {SLOWEST_PRIME_ONE_IN} is the slowest rate the governor will call \
                 priming",
                decision.slot,
                deck_letter(decision.slot as u8),
                decision.reason,
                match decision.cost_ms {
                    Some(ms) => format!("warming it was measured at {ms:.3} ms"),
                    None => String::from("nothing has measured it"),
                },
                match governed.headroom_ms() {
                    Some(ms) => format!("{ms:.3} ms of headroom"),
                    None => String::from("a committed cost nobody can know"),
                },
            );
        }
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
                    // control. The mixer's two faders are the other two, and
                    // between them they are everything a press acts on that is
                    // not a boundary.
                    Kind::Outputs => "row, one sink: program view".to_owned(),
                    Kind::Pane => "pane, inside a bay".to_owned(),
                    Kind::Picture => "the picture, a sink".to_owned(),
                    // Four cells, and this file knows which of them are on:
                    // one slot of the deck's two is making texels, so deck A
                    // and no other.
                    Kind::Previews => "four previews, A live".to_owned(),
                    // A bay like the other six, and then a strip per slot:
                    // a strip is a deck slot, and this deck has two.
                    // A bay like the other five, and then a row per Set
                    // the store holds — as many as the bay has room for, and
                    // the foot says so. `n of m`, exactly as the bay draws it.
                    Kind::Library => {
                        match karakuri_console::view::library(layout, &self.view.library) {
                            Some(bay) => format!("bay, {} listed", bay.count()),
                            None => "bay, no store behind it".to_owned(),
                        }
                    }
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
            "the outputs row's dot folds the picture by name, so clicking it and pressing f \n\
             over the picture are the same operation reached from two surfaces. it is lit \n\
             while the picture is on screen. the mixer's two faders are the panel's other \n\
             controls, and they are a different kind: the dot names an operation on the \n\
             ARRANGEMENT, which this crate performs, and a fader names one on the MIX, \n\
             which it cannot — so the operation comes out and this file applies it."
        );
        println!();
        println!("keys — the pointer's position decides what each one acts on:");
        println!("  click    the outputs dot: turn the program view sink off, and on again");
        println!("  drag     press the left button in a gap and move: the boundary follows");
        println!("  fader    press a mixer knob and move: the deck's gain or opacity follows");
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

/// **What routing a pointer event did**, beyond deciding whose it was.
///
/// Three answers rather than an `Option<Outcome>`, because there are now two
/// kinds of control on the panel and they end in two different places: the
/// Outputs dot asks for an operation on the *arrangement*, which this crate
/// performs and reports as an [`Outcome`], and a fader asks for an operation
/// on the *mix*, which nothing in `karakuri-console` can perform at all. The
/// repaint decision is taken from which of the three it is — see
/// `Change::Operated` and `Change::Emitted`.
#[derive(Debug, Clone, PartialEq)]
enum Acted {
    /// Nothing acted: a press on a boundary, a move, a wheel, a release.
    Nothing,
    /// The Outputs dot, and what the operation it named did.
    Operated(Outcome),
    /// **A fader translated a drag into the vocabulary**, or the drag moved
    /// the pointer over a value that did not change and asked for nothing.
    Emitted(Option<Operation>),
}

fn folding(folded: bool) -> &'static str {
    match folded {
        true => "folded",
        false => "unfolded",
    }
}

/// **Which deck, in the letter the preview cells are drawn with** — asked of
/// the view rather than written out again here.
///
/// The vocabulary counts decks from zero (`Operation::SetGain { deck: u8 }`)
/// and the console draws them `A` through `D`, so this is the one place the
/// two meet. A deck outside the four cannot be built by anything in this file
/// — a `Deck` holds `MAX_SLOTS` slots and `DECKS` is that number — so an index
/// past the end is a bug and reads as one rather than wrapping quietly.
fn deck_letter(deck: u8) -> &'static str {
    DECK_LETTERS
        .get(usize::from(deck))
        .copied()
        .unwrap_or("(no such deck)")
}

/// Which of a strip's two faders, in the mixer bay's own words.
fn knob_word(knob: Knob) -> &'static str {
    match knob {
        Knob::Trim => "trim",
        Knob::Fader => "fader",
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

/// The seed salt, which decides where the elements start. Any value is a
/// picture; 7 is the one `karakuri-cli`'s own tests use, so this looks like
/// what they look like.
const SEED_SALT: u32 = 7;

/// **Deck B's salt**, so that the slot the budget parks is a different
/// simulation of the same procedure rather than a second copy of the same one.
/// Nothing can see the difference while it is parked — a parked slot neither
/// steps nor draws — and the frame it matters on is the one where somebody
/// gives the deck room and both are on screen.
const WARM_SEED_SALT: u32 = 8;

/// **The two slots this deck has, and what each is for.**
///
/// Deck A is Live and is the whole of what the Program bay draws. Deck B is
/// asked to prime and is parked by the budget — see
/// [`Engine::ask_to_prime`] — which is the one state on this panel where a
/// slot's two residencies disagree, and until this deck had a second slot it
/// could not be reached by running the window at all.
const ON_AIR: usize = 0;
const ASKED_TO_PRIME: usize = 1;

/// **The `.kir` pair this plays, and the whole of what the command line
/// takes.**
///
/// A Set is built from an L1 and an L4 — a geometry and a renderer — and
/// `Set::build` takes exactly those two. **They are one Set, not two slots**:
/// the deck's two slots are this same pair built twice, at two seed salts
/// ([`SEED_SALT`] and [`WARM_SEED_SALT`]), so that the slot the budget parks is
/// a different simulation of the same procedure rather than a second copy of
/// the same one.
///
/// **Deliberately not `karakuri-cli`'s parser.** That program has thirty-odd
/// flags, `--set a.kir,b.kir` among them, and they live in its own `main.rs`
/// where nothing else can reach them. A second `--flag` vocabulary here would
/// be a second answer to *how does an operator name material*, which is the
/// failure this whole move exists to stop paying for
/// ([P-0031](../../../docs/principles/0031-a-name-means-one-thing-across-the-system.md)).
/// So this is two positional paths and nothing else: enough to pick what plays,
/// and no vocabulary to disagree with. The day the two programs share one, it
/// comes from a package both can reach and this goes.
///
/// `Debug` unconditionally rather than `#[cfg_attr(test, derive(Debug))]`: that
/// idiom does not survive a crate boundary — `cfg(test)` is set when the
/// *defining* crate's tests compile and not when a consumer's do — and
/// ADR-0214 names it as the one class of surprise a move of this kind produces.
/// Nothing consumes this type today, and writing the version that would break
/// is not cheaper than writing the one that would not.
#[derive(Debug)]
struct Sources {
    l1: std::path::PathBuf,
    l4: std::path::PathBuf,
}

impl Default for Sources {
    /// **The repository's own pair, resolved against the workspace root** —
    /// which is what this program drew before it took an argument, so a bare
    /// `cargo run -p karakuri` behaves as it always did.
    ///
    /// Against the workspace root rather than the working directory, and that
    /// asymmetry with a typed path is on purpose: a default nobody named has to
    /// find the file wherever the run was started from, and a path an operator
    /// *typed* is theirs and is read from where they typed it.
    fn default() -> Sources {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        Sources {
            l1: root.join("examples/drift_shell.kir"),
            l4: root.join("examples/soft_points.kir"),
        }
    }
}

impl Sources {
    /// **What the mixer strip calls what this deck is playing**, and it is this
    /// file's word rather than the engine's.
    ///
    /// `view::Strip::name` says why there is no other answer: nothing reachable
    /// from a `Deck` carries a name for the material in a slot. A `Set` names
    /// its *nodes* and its *published controls* and has no name of its own,
    /// which is right — a Set is built from a list of `.kir` files, and only
    /// whoever passed that list knows what to call the result. **This is that
    /// list**, derived from the two paths rather than typed again, so a strip
    /// cannot go on saying `drift_shell` after somebody loads something else.
    fn material(&self) -> String {
        let stem = |path: &std::path::Path| {
            path.file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
        };
        format!("{} + {}", stem(&self.l1), stem(&self.l4))
    }
}

/// **How this program is called**, printed for `--help` and for anything it
/// cannot read as a pair.
const USAGE: &str = "\
usage: karakuri [GEOMETRY.kir RENDERER.kir]

  The console, with a deck behind it. Both paths or neither: a Set is an L1 and
  an L4, and with neither the repository's own pair is played.

  This is not `karakuri-cli`'s command line and does not try to be — that one
  has the flags, the store, the audio, the MIDI and the MCP server, and its
  parser is its own. See `cargo run -p karakuri-cli -- --help`.";

/// **The command line, read.** Two paths or none; `--help` or `-h` prints
/// [`USAGE`]; anything else is a refusal that prints it.
///
/// A free function over an iterator rather than a read of `std::env::args`
/// inside [`main`], for the reason [`karakuri_environment`]'s refusals are free
/// functions: `main` cannot be called from a test and a refusal nobody can
/// reach is a refusal nobody checked. See
/// `a_set_is_two_paths_or_none_and_anything_else_is_refused`.
fn sources_from<I: IntoIterator<Item = String>>(args: I) -> Result<Sources, String> {
    let args: Vec<String> = args.into_iter().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        return Err(String::new());
    }
    match args.as_slice() {
        [] => Ok(Sources::default()),
        [l1, l4] => Ok(Sources {
            l1: std::path::PathBuf::from(l1),
            l4: std::path::PathBuf::from(l4),
        }),
        [one] => Err(format!(
            "one path given (`{one}`) and a Set needs two: a geometry and a renderer"
        )),
        many => Err(format!(
            "{} paths given and a Set is built from two: a geometry and a renderer",
            many.len()
        )),
    }
}

/// **One simulation step per frame drawn, and no clock anywhere.**
///
/// [P-0002](../../../docs/principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md)
/// says simulation time comes from a record and never from a clock. There is
/// no record here — this program is not a session — so the honest
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
/// program inventing an aesthetic the rest of the workspace does not run
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

/// **The engine behind the Program bay: a deck of two Sets, the present pass,
/// and the two textures it lands in.**
///
/// Scaffolding, and it looks it: two slots, no audio, no MIDI, no store, no
/// arguments. What `karakuri-cli` does around this is a program; what is here
/// is the shortest path from two `.kir` files to texels, which is the whole of
/// what the Program bay needs to be shown to be reachable.
///
/// **The second slot is not a picture, it is a residency.** [`ON_AIR`] is Live
/// and is everything on screen; [`ASKED_TO_PRIME`] is asked to warm up and is
/// parked by the budget in [`Engine::ask_to_prime`], which is what puts a
/// pending request on this panel for the mixer's tally to draw. A parked slot
/// neither steps nor draws, so it costs this window nothing per frame — the
/// reading [`Costs::say`] prints is still one Set stepping.
///
/// **Three of the four preview cells are still off, and now for two reasons.**
/// Deck A is the only audition there is to show: deck B is parked and so is
/// making no texels, and C and D have nothing behind them at all. Three cells
/// reading `off` are the truth about this program rather than a gap in it.
struct Engine {
    deck: Deck,
    /// **Elements per geometry, read off the L1's own `capacity` declaration**
    /// rather than named here — see [`Engine::new`]. Kept because the reading
    /// [`Costs::say`] prints names it, and a workload figure that is not the
    /// one the run used is worse than none.
    capacity: u32,
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
        sources: &Sources,
        layout: &karakuri_layout::Layout,
        scale: f32,
    ) -> Engine {
        let l1 = checked(&sources.l1);
        let l4 = checked(&sources.l4);
        let capacity = capacity_of(&l1);
        // **The same pair twice, and the salt is the only difference.** The
        // second slot exists to be asked for and refused, so what is in it
        // matters less than that it is a Set the governor can measure and
        // budget for like any other — building it from a second `.kir` pair
        // would be this program loading material to make a point about a
        // residency.
        let built = |salt| {
            Set::build(&gpu.device, &gpu.queue, &l1, &l4, capacity, salt)
                .expect("the pair builds a Set")
        };
        let mut deck = Deck::new(
            &gpu.device,
            vec![
                HotSwap::fixed(built(SEED_SALT)),
                HotSwap::fixed(built(WARM_SEED_SALT)),
            ],
            CANVAS.0,
            CANVAS.1,
        );
        // **The meters are on, and that is a decision rather than a default.**
        // Five of the six things a mixer strip shows are settings the deck was
        // told; the meter is the only one that is a *measurement*, so with it
        // off this bay would draw five readouts that never move beside a well
        // that is always empty — which is the scaffolding-that-looks-finished
        // this panel refuses, read from the other side. It costs a pipeline,
        // two buffers and a ring of staging buffers per slot, allocated here
        // and never on the render thread, which is the same terms `Deck::new`
        // above is on; there are two slots, so it is two of each. The parked
        // one reports no level — `Deck::level` is `None` for a slot that is
        // not being drawn — which is the meter saying what it measured rather
        // than a strip with a gap in it.
        deck.enable_meters(&gpu.device);
        let present = Present::new(&gpu.device, PICTURE_FORMAT, CANVAS.0, CANVAS.1);
        let [picture, preview] = aims(layout, present.size());
        Engine {
            deck,
            capacity,
            present,
            picture: Presented::new(gpu, renderer, "program view", picture, scale),
            // Named for the deck it is of, because there is one of these per
            // audition and a device message that says `deck preview` four
            // times over says nothing.
            preview: Presented::new(gpu, renderer, "deck A preview", preview, scale),
            freed: 0,
        }
    }

    /// **Ask deck B to warm up, and let the budget answer.** The one governor
    /// pass this program makes, taken at startup where the stall it costs is
    /// free, and the whole of why a strip on this panel can read one residency
    /// and have been asked for another.
    ///
    /// **It is `karakuri-cli`'s order rather than a second one**: measure every
    /// slot before anything is decided about any of them, ask through
    /// [`Deck::set_residency`], and call [`Deck::govern`], which is the only
    /// thing in the engine that writes an *effective* residency. `set_residency`
    /// writes the request **and grants it**, so a harness that never governs
    /// has a deck whose two residencies agree on every slot and every frame —
    /// which is what this file was, and is why the roll ADR-0190 drew was
    /// tested and unreachable. The report comes back whole for the same reason
    /// `karakuri-cli`'s `report_governing` prints one: nothing is printed in
    /// the engine, so what an operator reads and what a test asserts are the
    /// same values.
    ///
    /// # The budget is what moves, and it is moved from what was measured
    ///
    /// A deck starts on `governor::DEFAULT_COMPUTE_BUDGET_MS` — one 60 Hz
    /// frame of measured per-Set cost — and what one of these Sets measures at
    /// is this machine's business rather than anything this file can know. So
    /// the budget is set **from the measurement that was just taken**: what
    /// deck A is already committed to, plus half of the least the slowest
    /// priming rate could ask for. [`SLOWEST_PRIME_ONE_IN`] is the slowest the
    /// governor is willing to call priming and a slowed slot costs
    /// `cost / n` amortised, so a headroom under `cost / SLOWEST_PRIME_ONE_IN`
    /// cannot take this Set at **any** rate — the refusal is arithmetic on
    /// every machine rather than on the ones where the numbers happen to come
    /// out.
    ///
    /// **A number computed from the measurement rather than a constant**,
    /// because a constant is the fixture the product cannot produce
    /// ([P-0047](../../../docs/principles/0047-a-fixture-the-product-can-rewrite-is-not-a-fixture.md)
    /// read from the other side): a budget typed in here parks the request on
    /// this machine and admits it on a faster one, and a *measurement* typed in
    /// — `HotSwap::set_measured_cost` is public and would take one — is this
    /// file writing down the number the probe exists to take.
    ///
    /// **Nothing here writes a residency, a strip or a park.** The deck is
    /// asked and the governor answers; [`mixer`] reads both residencies back
    /// off the deck the way it reads the gain, and `view::Strip::pending`
    /// derives the disagreement. `Deck::is_parked` is not called in this file
    /// at all outside `mod gpu`.
    ///
    /// Where a measurement is missing the budget is left where it was, and the
    /// governor parks the request anyway for a different and more serious
    /// reason — an unmeasured Live slot means the committed cost is unknown,
    /// which suspends priming wholesale. The caller prints the reason it got
    /// rather than the one this comment expects.
    fn ask_to_prime(&mut self, gpu: &Gpu) -> Report {
        // **Before the first frame, and this is the only place it can be.**
        // Measuring means stepping and ends in a rewind, so `measure_slots`
        // skips any slot whose Set has already run — a deck measured late
        // stays unbudgetable rather than losing what it has simulated.
        self.deck.measure_slots(&gpu.device, &gpu.queue);
        self.deck.set_residency(ASKED_TO_PRIME, Residency::Priming);
        if let (Some(committed), Some(warming)) = (
            self.deck.slot(ON_AIR).measured_cost(),
            self.deck.slot(ASKED_TO_PRIME).measured_cost(),
        ) {
            self.deck.set_compute_budget_ms(
                committed.ms + warming.ms / (2.0 * SLOWEST_PRIME_ONE_IN as f32),
            );
        }
        self.deck.govern()
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
    /// **B, C and D stay `None`, and that is this program rather than a gap in
    /// it**: deck A is the only slot making texels, so it is the only audition
    /// there is to put in a cell. Deck B is parked and therefore neither
    /// stepping nor drawn, and C and D have nothing behind them. An empty cell
    /// is what off looks like, and the console draws it saying `off`.
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

/// **Where this program looks for a library**, and it is the directory
/// `karakuri-cli` looks in when nobody passes `--store`.
///
/// Relative to the working directory, deliberately unlike [`Sources`]'s
/// defaults: a store is a place an operator keeps things and belongs beside
/// the session, which is `karakuri-cli`'s own reason for the same choice. The
/// repository keeps one at its root, so a `cargo run -p karakuri` from there
/// has a Set to list rather than an empty bay.
///
/// # Still transcribed, and this is the one thing ADR-0214 said would go and
/// has not
///
/// [ADR-0214](../../../docs/adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md)
/// lists this constant as one of two transcriptions the move *deletes rather
/// than carries*: *"`STORE` asks for `DEFAULT_STORE`, and the residency decode
/// calls `mix::parse_residency`."* The residency half is done — `apply` calls
/// `karakuri_environment::mix::parse_residency` now, and that function is
/// `pub` for this caller.
///
/// **This half cannot be done yet, and the reason is which slice moved.**
/// `DEFAULT_STORE` is at `karakuri-cli/src/main.rs:356` and is private.
/// ADR-0214's boundary section says `main.rs` is *"7,688 non-test lines
/// holding both and will not divide along a line anyone can name today"*, and
/// it is the one module of the fourteen that has not moved: `karakuri-cli/src/`
/// is that file and nothing else. Until it divides there is nothing to ask, so
/// the two are the same directory on purpose and the day one moves this
/// program lists an empty library rather than the wrong one — which is the
/// sentence this comment has carried since it was written, still true and now
/// with the blocker named rather than guessed at.
const STORE: &str = ".karakuri";

/// **What the Library bay lists**: the name of every Set the store holds.
///
/// # Read once, and by the side of the seam that may read a disk
///
/// Called from `resumed`, before the first frame. A listing is a directory
/// read and a frame path does not do those
/// ([P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)),
/// and nothing in `karakuri-console`'s `src/` could do it anyway: opening a
/// store is `karakuri-store`'s and the crate depends on neither it nor the
/// engine (ADR-0156). What crosses into the console is a list of names.
///
/// The cost of reading it once is that a Set saved while this window is up
/// does not appear in the bay until the next run. That is this program's
/// limitation and not the console's — the field is rewritable per frame like
/// every other one — and closing it wants a reason to re-read rather than a
/// timer, which is a decision and not this pass's.
///
/// # A missing store is listed as nothing, and is not created
///
/// [`karakuri_store::store::Store::open`] *"establishes the store layout under
/// `root`, creating any directories that do not exist yet"*, which is the
/// right thing for a program that is about to write one and the wrong thing
/// for one that only wants to read. A program that listed a library by first
/// making one would change the directory it was run in, so the root is
/// required to be there already.
///
/// Either way the answer is a list, and an empty one is a bay with nothing in
/// it — which is what `view::library` draws for it, and is honest: a store
/// this run could not read holds nothing it can name.
fn library(root: &std::path::Path) -> Vec<String> {
    if !root.is_dir() {
        println!(
            "library: no store at {}, so the bay lists nothing",
            root.display()
        );
        return Vec::new();
    }
    let sets = Store::open(root).and_then(|store| store.list_sets());
    match sets {
        Ok(sets) => sets.into_iter().map(|entry| entry.id).collect(),
        Err(e) => {
            // Said rather than swallowed, for the reason every other failure
            // in this file is said: a bay that is empty because the store
            // could not be read looks exactly like a bay that is empty
            // because the store is.
            println!("library: {} could not be listed: {e}", root.display());
            Vec::new()
        }
    }
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
/// - **And the request beside it** — `Deck::requested_residency`, which is the
///   other half of the same pair. Both are handed over and **neither side
///   computes `Deck::is_parked`**: the engine has the predicate and the
///   console derives its own from the two values (`view::Strip::pending`), so
///   what crosses the seam stays a residency and a residency rather than
///   becoming a bit whose meaning is written down in only one of the two
///   crates. It is the same reading as the four numbers below — the deck says
///   what it is doing, and the surface decides what that looks like.
/// - **The trim and the fader** are `gain` and `opacity`, which are two
///   controls and not one — *"opacity at zero silences under every blend mode,
///   gain at zero does not silence `over`"* — and the bay draws them as two.
/// - **The blend** is [`blend_mode`]: the engine's `Blend` turned into the
///   vocabulary's `BlendMode`, because the chip is a control now and a control
///   has to know which of the three it is on to say what the next one is
///   (ADR-0187). **This is where a fourth engine mode with no operation
///   variant stops the build**, which is the failure worth having — the
///   alternative is a word drawn on a chip no map can ask for.
/// - **The mask** is `Deck::mask(slot).kind()` for the mark, **and its angle
///   beside it** — `view::Strip::mask_angle`, which is read to build the
///   press's operation and drawn nowhere
///   ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
///   The position and the softness are left behind: the strip's `.mini` says
///   *which shape*, three numbers about that shape are an inspector row, and
///   the two the record needs are read where the record is written rather
///   than carried across this seam.
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
/// **What a scheduled move on this control is taking it to**, or `None` for a
/// control nothing is moving.
///
/// The engine's `Transition` carries the instant it starts, its length in
/// beats and its curve as well, and none of the three crosses this seam: the
/// console has no beat count, so what it could draw out of them is nothing.
/// See `view::Strip::gain_to`.
fn destination(deck: &Deck, slot: usize, control: Control) -> Option<f32> {
    deck.transitions_on(slot)
        .find(|t| t.control() == control)
        .map(|t| t.to())
}

fn mixer(deck: &Deck, name: &str, out: &mut Vec<view::Strip>) {
    out.truncate(deck.slot_count());
    while out.len() < deck.slot_count() {
        out.push(view::Strip {
            name: name.to_owned(),
            tally: view::Tally::Allocated,
            requested: view::Tally::Allocated,
            gain: 0.0,
            gain_to: None,
            opacity: 0.0,
            opacity_to: None,
            blend: BlendMode::Add,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
        });
    }
    for (slot, strip) in out.iter_mut().enumerate() {
        if strip.name != name {
            strip.name.clear();
            strip.name.push_str(name);
        }
        strip.tally = tally(deck.residency(slot));
        strip.requested = tally(deck.requested_residency(slot));
        strip.gain = deck.gain(slot);
        strip.opacity = deck.opacity(slot);
        // **Where a scheduled move is taking each fader**, which is
        // `Deck::transitions_on` — *"what is moving on this slot, for a status
        // line"* — read for a surface instead. `karakuri-cli`'s line prints
        // `o>0.80` off the same call and for the same reason: with the default
        // quantum a fade is armed up to a bar before it is due, and a control
        // that changes something invisible is indistinguishable from one that
        // is broken.
        //
        // **At most one per control**, because `Deck::schedule` cancels
        // whatever was moving that pair before it pushes — so `find` is the
        // whole answer rather than the first of several.
        strip.gain_to = destination(deck, slot, Control::Gain);
        strip.opacity_to = destination(deck, slot, Control::Opacity);
        strip.blend = blend_mode(deck.blend(slot));
        strip.mask = match deck.mask(slot).kind() {
            MaskKind::None => view::Mask::None,
            MaskKind::Linear => view::Mask::Linear,
            MaskKind::Radial => view::Mask::Radial,
        };
        // **Read for the press and painted nowhere**, which is what
        // `view::Strip::mask_angle` is for: the chip asks for a shape and the
        // operation carries an angle, so the angle it carries is the one the
        // slot already has (ADR-0203). A harness that left this at zero would
        // make every press straighten a diagonal front, and nothing on the
        // panel would show it having happened.
        strip.mask_angle = deck.mask(slot).angle();
        strip.level = deck.level(slot).map(|level| view::Level {
            mean: level.mean,
            peak: level.peak,
        });
    }
}

/// **The engine's residency, as the console's word for it** — and, like
/// [`blend_mode`], a `match` so that a fourth `Residency` stops the build here
/// rather than drawing a chip nothing can read.
///
/// One function for both halves of the pair. It was written inline for the
/// effective residency alone; the request needs exactly the same three arms,
/// and a second copy of them is a translation that can start disagreeing with
/// itself about what `Priming` is called.
fn tally(residency: Residency) -> view::Tally {
    match residency {
        Residency::Live => view::Tally::Live,
        Residency::Priming => view::Tally::Priming,
        Residency::Allocated => view::Tally::Allocated,
    }
}

/// **The engine's blend mode, as the vocabulary's** — and the one place the
/// two lists are made to agree.
///
/// `karakuri-operation` owns its own copy of every list a destination is drawn
/// from, which is the cost P-0074 says the vocabulary pays: *"The two rules —
/// be engine-neutral, and have no toggles — are not jointly satisfiable unless
/// the vocabulary owns the lists."* A copy needs somewhere the two meet, and
/// this is that place for this list, on the harness side of the seam — the
/// same side [`mixer`] reads a `Deck` from (ADR-0156).
///
/// **A match, so the day a fourth mode lands in `karakuri_engine::deck::Blend`
/// this stops compiling.** That is the whole reason `view::Strip::blend` is a
/// `BlendMode` and not the engine's word: a `&str` handed through would draw
/// the new mode's name on a chip, and the chip would cycle three ways past a
/// state no operation can name and no MIDI map can reach, with nothing saying
/// so. Failing here is the loud failure P-0027 asks for.
///
/// **It stays this program's, and that is now settled rather than pending.**
/// The console cannot depend on the engine (ADR-0156), and
/// `karakuri-operation-record` cannot either — it is the vocabulary and the
/// records and nothing else, by charter. So the two lists meet on the harness
/// side of the seam, wherever a harness holds both, and ADR-0180's *"one
/// `From` impl per list in `karakuri-cli`"* cannot be written at all: neither
/// [`Blend`] nor [`BlendMode`] is that package's, and the orphan rule refuses
/// it ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
fn blend_mode(blend: Blend) -> BlendMode {
    match blend {
        Blend::Add => BlendMode::Add,
        Blend::Over => BlendMode::Over,
        Blend::Max => BlendMode::Max,
    }
}

/// **What this window says when a control's operation wrote no record**, and
/// the two ways that happens are not the same thing — so they are not the
/// same sentence.
///
/// [`written`] has three answers and only one of them is a record.
/// A harness that printed a line for that one and nothing at all for the
/// other two would tell an operator that a press did nothing, which is true
/// of neither:
///
/// - [`Written::Silent`] is **settled**. Selecting a deck or folding a bay is
///   a surface's own state and there is nothing to write; the sentence says
///   which of the four kinds of nothing it is, and that is the end of it.
/// - [`Written::Owed`] is **a gap nobody has closed yet**. A fade owes a
///   record and no build can make it, so a press that reads as *nothing
///   happened* is exactly the wrong reading — the sentence names the question
///   instead, which is `Owed::why`'s whole job and the reason `Owed` is not
///   an error.
///
/// `None` for [`Written::Records`], because that line is [`apply`]'s: it says
/// the record *and* what the deck holds afterwards, and printing both would
/// say one press twice.
///
/// **Nothing this program draws can reach either arm today**, and that is why
/// it is written rather than a reason to leave it out. Its five mixer
/// controls emit `SetGain`, `SetOpacity`, `SetBlendMode`, `SetResidency` and
/// `SetMaskShape`, and all five write a record — four from the operation
/// alone and the mask's from that plus the reading [`reading`] takes; the
/// Outputs dot never arrives here at all, because it asks the panel for an
/// arrangement [`Op`] and the panel performs it ([`Acted::Operated`]). The day
/// a control on this panel emits an operation nobody can write a record for,
/// this window says what became of the press rather than swallowing it.
///
/// **The mask is also the one that can reach [`Written::Owed`] by accident**,
/// and that is worth having rather than designing away: a reading that did not
/// arrive answers `Owed(NotRead(Reading::Mask))`, so a harness that stopped
/// handing one in would say so out loud instead of moving nothing.
fn unwritten(operation: &Operation, written: &Written) -> Option<String> {
    match written {
        Written::Records(_) => None,
        Written::Silent(silent) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is settled: {}",
            silent.why()
        )),
        Written::Owed(owed) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is a gap rather than a \
             decision: {}. nothing moved, and nothing here decides it",
            owed.why()
        )),
    }
}

/// **The record, applied to the deck**, and what to say about it.
///
/// **This is not the half ADR-0185 promised to delete, and it did not go with
/// it.** Turning an `Operation` into a `Record` was the shortcut — that
/// function is gone and [`written`] answers instead
/// ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
/// Turning a record into a *deck movement* is a different job and is the
/// harness's by design: `karakuri-operation-record` has no engine and never
/// will, so somebody who owns a deck has to decode.
///
/// `karakuri-cli`'s `mix::change` is the real decoder and it does two things
/// this does not: it refuses a slot the deck has not got, with the same
/// sentence every other surface refuses one with, and it turns a record into a
/// `Change` that a caller applies. This is the shortest path from the records
/// [`written`] answers with to the setters they name.
///
/// The line it returns is the loop closing, printed so that it can be read
/// rather than inferred: the operation, the record, and **what the deck says
/// afterwards** — which is where the next frame's strip comes from.
fn apply(record: &Record, deck: &mut Deck) -> Option<String> {
    // **A slot the deck has not got is refused rather than indexed.**
    // `Deck::set_gain` indexes its slots, a panic reachable from an event
    // handler aborts this process rather than unwinding (see the module
    // documentation), and `mix::change`'s whole reason for taking a
    // `slot_count` is that a stream may name a slot that is not there. Nothing
    // in this file can produce one — the strips are the deck's own count — so
    // this is the guard rather than the message, and the real sentence is
    // `karakuri-cli`'s `no_such_slot`.
    let held = |slot: u8| (usize::from(slot) < deck.slot_count()).then_some(usize::from(slot));
    match *record {
        Record::Gain { slot, value } => {
            let slot = held(slot)?;
            deck.set_gain(slot, value);
            Some(format!(
                "  fader: deck {} trim -> SetGain {{ deck: {slot}, gain: {value:.3} }} \
                 -> Record::Gain -> deck.gain({slot}) = {:.3}",
                deck_letter(slot as u8),
                deck.gain(slot)
            ))
        }
        Record::Opacity { slot, value } => {
            let slot = held(slot)?;
            deck.set_opacity(slot, value);
            Some(format!(
                "  fader: deck {} fader -> SetOpacity {{ deck: {slot}, opacity: {value:.3} }} \
                 -> Record::Opacity -> deck.opacity({slot}) = {:.3}",
                deck_letter(slot as u8),
                deck.opacity(slot)
            ))
        }
        // **The mode comes back off the wire name, and an unknown one is
        // refused rather than defaulted.** `Record::Blend` carries a `String`
        // because what a mode is allowed to be is the engine's to say, so this
        // is the engine saying it — `mix::change` refuses the same way, with
        // the sentence `karakuri-cli`'s `no_such_blend` writes. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `BlendMode::ALL`, so this is the guard rather
        // than the message.
        Record::Blend { slot, ref mode } => {
            let slot = held(slot)?;
            let blend = Blend::from_name(mode)?;
            deck.set_blend(slot, blend);
            Some(format!(
                "  blend: deck {} -> SetBlendMode {{ deck: {slot}, blend: {mode} }} \
                 -> Record::Blend -> deck.blend({slot}) = {}",
                deck_letter(slot as u8),
                deck.blend(slot).name()
            ))
        }
        // **The one record here that is followed by a governor pass**, and it
        // is not a flourish: `Deck::set_residency` writes the request *and
        // grants it*, so a harness that stopped there would put a slot the
        // budget has no room for into `Priming` and draw a primed deck the
        // governor never admitted. That is
        // [ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)
        // exactly — the panel's parked deck is the governor's verdict or it is
        // a drawing of one — and it is `karakuri-cli`'s own order, where
        // `mix::Change::Residency` sets the level and calls `govern` beside
        // it. The report is dropped here rather than printed: the line below
        // says what the deck ended up at, which is the half this window shows.
        Record::Residency { slot, ref level } => {
            let slot = held(slot)?;
            let residency = mix::parse_residency(level)?;
            deck.set_residency(slot, residency);
            deck.govern();
            Some(format!(
                "  tally: deck {} -> SetResidency {{ deck: {slot}, residency: {level} }} \
                 -> Record::Residency -> deck.requested_residency({slot}) = {:?}, \
                 deck.residency({slot}) = {:?}",
                deck_letter(slot as u8),
                deck.requested_residency(slot),
                deck.residency(slot)
            ))
        }
        // **The whole mask, because the record is a state and not an ask.**
        // `Record::Mask` carries a shape, an angle, a position and a softness,
        // and `Deck::set_mask` is what it decodes to — the engine says so at
        // that setter. Reaching for `Deck::set_mask_shape` instead, to keep a
        // running wipe alive, would be this file decoding a record by picking
        // two fields out of it and dropping the softness on the floor: a
        // second route to the deck, where P-0028 is that every control ends in
        // the same record. **So a shape press stops a wipe on that deck**, and
        // that is not a fault here — it is the honest limit
        // `docs/principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md`
        // states about the record stream, met by the first surface to make the
        // press
        // ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
        //
        // **The shape comes back off the wire name**, refused rather than
        // defaulted, exactly as the blend's and the residency's do. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `WipeKind`'s three and the record is written
        // from `WipeKind::name`.
        Record::Mask {
            slot,
            ref kind,
            angle,
            position,
            softness,
        } => {
            let slot = held(slot)?;
            let shape = MaskKind::from_name(kind)?;
            deck.set_mask(slot, Mask::new(shape, angle, position, softness));
            Some(format!(
                "  mask: deck {} -> SetMaskShape {{ deck: {slot}, kind: {kind},                  angle: {angle:.3} }} -> Record::Mask -> deck.mask({slot}) = {} at {:.3} rad,                  front at {:.3}",
                deck_letter(slot as u8),
                deck.mask(slot).kind().name(),
                deck.mask(slot).angle(),
                deck.mask(slot).position()
            ))
        }
        _ => None,
    }
}

/// **The reading an operation's record needs, taken off the deck it names.**
///
/// [`written`] builds `Record::Mask` **whole** — a shape, an angle, a position
/// and a softness — out of an operation that names two of the four, and the
/// other two come from a reading of the mask that is running (ADR-0201). This
/// is that reading, and it is the harness's because the deck is
/// (ADR-0156, ADR-0194).
///
/// **`Current::default()` is *I read nothing*, and it is still the answer for
/// four of this panel's five controls**: a gain, an opacity, a blend mode and
/// a residency each carry everything their record carries, so handing a
/// reading in would be this file inventing a value. The mask mini is the one
/// that needs one, and it needs it for the deck the operation *names* rather
/// than for the deck the pointer is over — which is `Reading::Mask`'s own
/// wording and the reason this takes the operation and not a slot.
///
/// **The softness is read back**, where `karakuri-cli`'s `mix::current_mask`
/// substitutes its own `MASK_SOFTNESS`: that program writes wipes and has a
/// softness of its own to write, and this window has never written one. What
/// is read back here is therefore what is actually on the slot, and reading it
/// back is what stops a press rewriting it — the same argument the angle's is,
/// one field along.
///
/// A slot the deck has not got answers `None`, and [`written`] then says the
/// reading was owed rather than indexing something that is not there — the
/// guard [`apply`] has, at the other end of the same press.
fn reading(operation: &Operation, deck: &Deck) -> Current {
    let mask = match *operation {
        Operation::SetMaskShape { deck: slot, .. } => {
            let slot = usize::from(slot);
            (slot < deck.slot_count()).then(|| {
                let mask = deck.mask(slot);
                karakuri_operation_record::Mask {
                    kind: wipe_kind(mask.kind()),
                    angle: mask.angle(),
                    position: mask.position(),
                    softness: mask.softness(),
                }
            })
        }
        _ => None,
    };
    Current {
        mask,
        ..Current::default()
    }
}

/// **The engine's mask shape, as the vocabulary's** — [`blend_mode`]'s
/// function one control along, and the one place these two lists are made to
/// agree.
///
/// A match, so the day a fourth `MaskKind` lands in the engine this stops
/// compiling rather than reading a shape the vocabulary cannot name into a
/// record that has to name one. The mirror image of it is `view::Mixer::mask`,
/// which turns the console's own word into the same vocabulary — three names
/// for three shapes, which is the cost `karakuri-operation` pays for depending
/// on nothing (P-0074).
fn wipe_kind(kind: MaskKind) -> karakuri_operation::WipeKind {
    match kind {
        MaskKind::None => karakuri_operation::WipeKind::None,
        MaskKind::Linear => karakuri_operation::WipeKind::Linear,
        MaskKind::Radial => karakuri_operation::WipeKind::Radial,
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
/// **How many elements the geometry runs at**, which is the L1's own
/// declaration and not a number written here.
///
/// `capacity [min, max] = default` is in the file and `Set::build` takes a
/// number, so somebody has to read one across. This used to be a
/// `const CAPACITY: u32 = 262144` — `drift_shell.kir`'s declared default,
/// transcribed, which was fine while that was the only file this could load and
/// silently wrong the moment it took a path: a procedure written for 131072
/// would have run at 262144 and nothing would have said so.
///
/// **The fallback is not the answer, it is the arm that cannot happen.**
/// `karakuri_ir::DEFAULT_CAPACITY` is what is left when *nothing* declared one,
/// and `check_header` requires a `capacity` on every L1 — so a `Checked` that
/// passed always carries one and this `map_or` is the shape of the seam type
/// rather than a decision. Pointing the whole thing at `DEFAULT_CAPACITY` would
/// run every procedure at 262144 whatever it declared, which is the defect
/// `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none` is run
/// against.
///
/// This is `karakuri-cli`'s `capacity_for` with no `--capacity` to override it,
/// and `karakuri_environment::watch`'s rebuild is the same line again. There is
/// no `--capacity` here on purpose: see [`USAGE`].
fn capacity_of(l1: &karakuri_ir::typed::Checked) -> u32 {
    l1.capacity
        .map_or(karakuri_ir::DEFAULT_CAPACITY, |declared| declared.default)
}

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
    /// **What the command line asked for**, read before the event loop starts
    /// and used once, in `resumed`. It is here rather than in [`Gfx`] because
    /// it is known before there is a device and outlives every remake of one.
    sources: Sources,
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
    /// **The origin every animation on the panel is measured from**, and the
    /// only clock behind `view::Phase`.
    ///
    /// It is here because this file owns the window and the clock and `src/`
    /// owns neither — every `Instant::now` in this crate is in this file, and
    /// `view::Phase` is a `Duration` for exactly that reason (P-0002). What
    /// crosses the seam is `now - this`, which is a number.
    ///
    /// **Where the origin is does not matter**, which is why it is taken at
    /// construction rather than when something first starts moving: every
    /// presentation is periodic in the phase, so an origin the operator did
    /// not choose is an origin nobody can see. What would matter is having
    /// *two*, and there is one.
    started: Instant,
}

impl App {
    fn new(sources: Sources) -> App {
        App {
            gfx: None,
            sources,
            faulted: false,
            readout: Readout::new(WINDOW.0 as f32, WINDOW.1 as f32),
            costs: Costs::new(),
            scale: 1.0,
            egui_due: None,
            started: Instant::now(),
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

    /// **Perform what a pointer event asked for, and say what frame it is
    /// owed.** `otherwise` is the answer for an event that acted on nothing —
    /// the claim's, which is the answer this loop had before there were
    /// controls.
    ///
    /// The two kinds of control end in two different places, which is what
    /// [`Acted`] is for:
    ///
    /// - The Outputs dot's operation was already performed by the panel, and
    ///   what is owed is what the [`Outcome`] says happened.
    /// - **A fader's operation is performed here**, because it is the *deck*
    ///   that moves and the panel has no deck (ADR-0156). It becomes a record
    ///   and the record moves the deck — P-0028, which is what makes this
    ///   fader the same control as a key press and a MIDI knob rather than a
    ///   third way of writing a gain. The next frame's strips are read back off
    ///   the deck by [`mixer`], so what the fader shows is what the deck says
    ///   and never what this loop remembered.
    ///
    /// **What the operation becomes is [`written`]'s answer and not this
    /// file's**, out of the operation and what [`reading`] read off the deck.
    /// It used to be a `match` written out here, because there was
    /// nowhere for the conversion to live; ADR-0185 said that function is
    /// deleted the day a home lands, and
    /// [ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)
    /// is that home. Three answers come back and all three are said out loud —
    /// the records go to [`apply`], and the other two go to [`unwritten`],
    /// which is the difference between *this press writes nothing, and that is
    /// settled* and *this press owes a record nobody has decided how to
    /// write*.
    fn performed(gfx: &mut Gfx, acted: &Acted, otherwise: Repaint) -> Repaint {
        match acted {
            Acted::Nothing => otherwise,
            Acted::Operated(outcome) => Change::Operated(outcome).repaint(),
            Acted::Emitted(operation) => {
                if let Some(operation) = operation.as_ref() {
                    // **The reading is taken off the deck, and for four of the
                    // five controls it is *I read nothing*.** A gain, an
                    // opacity, a blend mode and a residency carry everything
                    // their record carries, so a reading handed in for one of
                    // them would be this file inventing a value — which is
                    // what ADR-0194 refuses a default for. The mask mini is
                    // the fifth and its record is written whole out of two
                    // halves (ADR-0201), so its half is read here rather than
                    // assumed. See [`reading`].
                    let written = written(operation, &reading(operation, &gfx.engine.deck));
                    // **A press that wrote no record says so**, and says
                    // which of the two kinds of nothing it was, before
                    // anything is applied.
                    if let Some(line) = unwritten(operation, &written) {
                        println!("{line}");
                    }
                    // **Every record, in the order it was written.** One
                    // today for each of the four, and a list because
                    // `Crossfade` is four and `Wipe` is five — one control is
                    // not one record (P-0028, ADR-0194).
                    if let Written::Records(records) = &written {
                        for record in records {
                            if let Some(line) = apply(record, &mut gfx.engine.deck) {
                                println!("{line}");
                            }
                        }
                    }
                }
                Change::Emitted(operation.as_ref()).repaint()
            }
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
            &self.sources,
            self.readout.panel.layout(),
            self.scale as f32,
        );
        // **Before the first frame and before the first strip is written**, so
        // that the panel's first frame draws the deck as it actually is rather
        // than a settled version of it that the second frame corrects.
        let governed = engine.ask_to_prime(&gpu);
        let info = gpu.adapter.get_info();
        self.costs.taken_on = format!(
            "{:?} — {} ({:?})",
            info.backend, info.name, info.device_type
        );

        let budget = budget_ms(&window);
        // **The strips before the legend**, because the legend says how many
        // there are and the answer is the deck's rather than a guess. It is
        // written again on every frame; this is the first one.
        let material = self.sources.material();
        mixer(&engine.deck, &material, &mut self.readout.view.mixer);
        // **The library before the legend too**, and once for the run: the
        // legend says how many Sets the bay lists, and `library` says why
        // where it is none.
        self.readout.view.library = library(std::path::Path::new(STORE));
        self.readout.print_legend(budget, &governed);

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
        // **Only once there is something to describe.** The reading names the
        // workload it was taken over, and that is the run's `.kir` pair rather
        // than a constant — so it is taken when the engine exists, and not
        // before. A deadline that comes due first is not lost: `due()` goes on
        // returning it until the reading is printed.
        if self.costs.due().is_some_and(|due| due <= now) {
            if let Some(gfx) = self.gfx.as_ref() {
                let (capacity, material) = (gfx.engine.capacity, gfx.material.clone());
                self.costs.say(capacity, &material);
            }
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
                let repaint = App::performed(gfx, &acted, Change::Pointer(claim).repaint());
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
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
                // **A press on a control earns its frame from what it did**,
                // and not from the claim: `Change::Pointer(Claim::Panel)` is
                // already a frame, but the operation the dot asked for is the
                // thing that moved every region in the Program bay, and it is
                // the outcome that says so.
                let repaint = App::performed(gfx, &acted, Change::Pointer(claim).repaint());
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            WindowEvent::MouseWheel { .. } => {
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, _acted) = self.readout.pointer(&ctx, Pointer::Wheel);
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
                // are about for the same reason. One strip per slot, so two —
                // see `mixer`.
                mixer(
                    &gfx.engine.deck,
                    &gfx.material,
                    &mut self.readout.view.mixer,
                );

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
                // **Said and not returned on**, and neither of this program's
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

/// **The command line, then the window.**
///
/// The arguments are read *before* the event loop exists, so a refusal is a
/// line on stderr and an exit code rather than a window that opens and closes.
/// `skip(1)` drops the program's own name, which is `std::env::args`'s first
/// element and not an argument.
fn main() {
    let sources = match sources_from(std::env::args().skip(1)) {
        Ok(sources) => sources,
        // An empty message is `--help`, which is a request rather than a
        // mistake: the usage goes to stdout and the exit is 0.
        Err(why) if why.is_empty() => {
            println!("{USAGE}");
            return;
        }
        Err(why) => {
            eprintln!("{why}");
            eprintln!();
            eprintln!("{USAGE}");
            std::process::exit(2)
        }
    };
    let event_loop = EventLoop::new().expect("event loop");
    // **The loop sleeps.** A frame is drawn when something changed it or when
    // `egui` asked for one after a delay it named, and on no other occasion —
    // `App::about_to_wait` sets this again after every iteration and is where
    // the rule actually lives. This is the state it starts in so that the
    // window between here and the first `about_to_wait` is not a spin either.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut App::new(sources)).expect("run");
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
    /// The two answers that are not a record. They are named only here,
    /// because nothing in the running window can reach either arm today —
    /// [`unwritten`] takes the `Written` it is given, and this is where the
    /// two are handed to it.
    use karakuri_operation_record::{Owed, Silent};

    /// **The library is what the store holds, and a store that is not there is
    /// listed as nothing rather than created.**
    ///
    /// Two claims, and the second is the one worth a test: `Store::open`
    /// establishes the layout it is pointed at, so a listing that opened first
    /// would leave a `.karakuri` behind in whatever directory this program was
    /// run from. [`library`] asks whether the root is there before it opens
    /// anything, and this is what says so.
    ///
    /// A CPU test: nothing here takes a device, and the store is a directory.
    #[test]
    fn a_library_is_the_store_and_a_missing_store_is_not_made() {
        let root = std::env::temp_dir().join(format!(
            "karakuri-console-library-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        // Whatever a previous run left, so the first claim is about a root
        // that is genuinely not there.
        let _ = std::fs::remove_dir_all(&root);

        assert!(
            library(&root).is_empty(),
            "a store that is not there listed something"
        );
        assert!(
            !root.exists(),
            "listing a library that is not there created one at {}",
            root.display()
        );

        // And with two Sets in it, both names come back — in the order
        // `list_sets` sorts them, which is the order the bay draws.
        let store = Store::open(&root).expect("a store to list");
        for id in ["night01", "morph01"] {
            store
                .write_set(id, &[])
                .unwrap_or_else(|e| panic!("writing {id}: {e}"));
        }
        let listed = library(&root);
        assert_eq!(
            listed,
            vec!["morph01".to_owned(), "night01".to_owned()],
            "the bay lists {listed:?}"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

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

    /// **A figure quoted in prose is held against the run that was just
    /// taken**, so a panel that grows says so instead of leaving a sentence
    /// that was true of a smaller one.
    ///
    /// This is the failure the guard exists for, and it is not hypothetical:
    /// the line above the reading cited ADR-0164's 184 allocations and said
    /// the `egui` pass "is still that" through the mixer bay landing at 456
    /// and the parked deck at 525 — two commits of a present-tense claim
    /// nobody re-checked, because nothing re-checked it.
    ///
    /// **What can be asserted here is the verdict, not the reading.** A
    /// reading needs a device, a window and three seconds of nobody touching
    /// it, so it cannot be taken from `cargo test`; what this file can do is
    /// make the figure in the sentence and the figure under the verdict one
    /// constant, and hold [`drifted`] to catching what actually went wrong.
    #[test]
    fn a_reading_that_has_moved_says_the_sentence_quoting_it_is_stale() {
        // The band a run has to stay inside to say nothing. Nine runs on
        // 2026-08-26 read between 524 and 538, and a guard that fired on that
        // spread is one nobody could keep passing.
        assert_eq!(drifted(WRITTEN_ALLOCS, WRITTEN_ALLOCS), None);
        assert_eq!(
            drifted(538, WRITTEN_ALLOCS),
            None,
            "the run-to-run spread of the reading this quotes must not read as staleness"
        );

        // And what it is for: ADR-0164's 184 against the mixer bay's 456 is
        // 2.5x, so the first run after that bay landed would have said the
        // sentence had stopped being true. It is the same answer whichever of
        // the two is the one written down, because a pass that got cheaper
        // makes the sentence just as untrue.
        assert!(
            drifted(456, 184).is_some(),
            "the mixer bay's landing is the drift this exists to have caught"
        );
        assert!(
            drifted(184, 456).is_some(),
            "drift is not caught one way round only"
        );

        // A band of two is still a band: an order of magnitude is well out of
        // it, from either end.
        assert!(drifted(WRITTEN_ALLOCS * 10, WRITTEN_ALLOCS).is_some());
        assert!(drifted(WRITTEN_ALLOCS / 10, WRITTEN_ALLOCS).is_some());
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
    ///
    /// `mod gpu` uses it too: a strip is laid out with the type in it, and a
    /// device does not make fonts valid.
    pub(super) fn drawn_once() -> egui::Context {
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
            Acted::Operated(Outcome::Folded {
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
            Acted::Operated(Outcome::Folded {
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

    /// **A control's operation becomes the record every other surface's
    /// control ends in**, and this is the half of that which needs no device.
    ///
    /// **This test is older than the conversion it now checks, and that is the
    /// point of it.** It was written against the hand-written `record` this
    /// file used to carry, asserting term for term what `karakuri-cli`'s
    /// `mix::gain_record` and `mix::opacity_record` already wrote. That
    /// function is deleted and [`written`] answers instead (ADR-0185's promise,
    /// kept where ADR-0194 put the home) — **every expectation below is
    /// unchanged**, so if the crate's conversion disagreed with the one that
    /// was deleted, this is what says so.
    ///
    /// **`mix::gain_record` is deleted too**, by the same record and for the
    /// stronger reason: the conversion *is* the derivation now, and two of
    /// them is the drift `mix.rs` exists to end. The comments below name it
    /// where it stood, because what this test compares against is the record
    /// that function wrote rather than the function.
    ///
    /// And the other direction: an operation this program has no control for
    /// writes no record here either, and the answer says *which* kind of
    /// nothing rather than a bare `None` — which is the whole of what the
    /// three answers buy.
    #[test]
    fn a_controls_operation_becomes_the_record_the_cli_would_have_written() {
        assert_eq!(
            only_record(&Operation::SetGain {
                deck: 2,
                gain: 0.75
            }),
            // What `mix::gain_record(2, 0.75)` wrote, before ADR-0194 deleted
            // it in favour of this conversion.
            Record::Gain {
                slot: 2,
                value: 0.75
            }
        );
        assert_eq!(
            only_record(&Operation::SetOpacity {
                deck: 0,
                opacity: 0.25
            }),
            // `mix::opacity_record(0, 0.25)`.
            Record::Opacity {
                slot: 0,
                value: 0.25
            }
        );
        // **Every mode of the cycle, because a chip that emits three
        // operations has three records to write** — and the mode is a wire
        // name, so a mode that reached `Record::Blend` misspelled would be
        // refused by the engine on the way back rather than here.
        for (deck, blend) in BlendMode::ALL.into_iter().enumerate() {
            let deck = deck as u8;
            assert_eq!(
                only_record(&Operation::SetBlendMode { deck, blend }),
                // `mix::blend_record(deck, blend)`.
                Record::Blend {
                    slot: deck,
                    mode: blend.name().to_owned(),
                },
                "`{}` did not become the record `mix::blend_record` writes",
                blend.name()
            );
            // And the engine reads its own name back, which is what says the
            // two lists are the same three words rather than two spellings of
            // them.
            assert_eq!(
                Blend::from_name(blend.name()),
                Some(blend_mode_back(blend)),
                "the engine does not know the vocabulary's `{}`",
                blend.name()
            );
        }

        // **Every residency of the cycle**, for the same reason as the blend:
        // one chip emitting three operations has three records to write. The
        // spelling is the wire's — `mix::residency_wire_name`'s three words,
        // which are deliberately not the status line's `LIVE`/`prim`/`park`
        // and not the chip's `live`/`prim`/`alloc` either, so a record written
        // in the chip's vocabulary would decode as nothing at all.
        for (deck, (residency, level)) in [
            (karakuri_operation::Residency::Live, "live"),
            (karakuri_operation::Residency::Priming, "priming"),
            (karakuri_operation::Residency::Allocated, "allocated"),
        ]
        .into_iter()
        .enumerate()
        {
            let deck = deck as u8;
            assert_eq!(
                only_record(&Operation::SetResidency { deck, residency }),
                // `mix::residency_record(deck, residency)`.
                Record::Residency {
                    slot: deck,
                    level: level.to_owned(),
                },
                "{residency:?} did not become the record `mix::residency_record` writes"
            );
            // And it reads back as the level it named, which is what says the
            // two spellings are one list rather than two.
            assert_eq!(
                mix::parse_residency(level),
                Some(residency_back(residency)),
                "the wire spelling `{level}` does not come back as {residency:?}"
            );
        }

        // The vocabulary is 50 operations and this program has five controls
        // writing five records. A record invented for the other 45 would be
        // somebody deciding what they mean — and the answer is now *which*
        // nothing rather than `None`, because a surface's own state and a
        // record nobody can write yet are not the same silence.
        assert_eq!(
            written(&Operation::Solo { region: None }, &Current::default()),
            Written::Silent(Silent::Surface)
        );
        assert_eq!(
            written(&Operation::SelectDeck { deck: 1 }, &Current::default()),
            Written::Silent(Silent::Surface)
        );
    }

    /// **The one record an operation writes**, for the tests that know there
    /// is exactly one.
    ///
    /// **For the four whose record needs no reading at all**, which is where
    /// `Current::default()` — *I read nothing* — is the honest answer. A
    /// conversion that answered anything but a single record for one of those
    /// four is this file's assumption breaking rather than a test needing a
    /// helper, which is why the panic says so.
    ///
    /// **The mask's operation is not one of them** and must not be passed
    /// here: its record is written out of the operation *and* a reading of the
    /// running mask (ADR-0201), so it would come back `Owed(NotRead)` and this
    /// would panic — correctly, and saying which operation. What the mask's
    /// tests hand in is a reading, through [`reading`] where there is a deck
    /// and by hand where there is not.
    pub(super) fn only_record(operation: &Operation) -> Record {
        match written(operation, &Current::default()) {
            Written::Records(records) if records.len() == 1 => records.into_iter().next().unwrap(),
            other => panic!(
                "a control's operation did not write exactly one record: \
                 {operation:?} -> {other:?}"
            ),
        }
    }

    /// **An operation whose record nobody can write yet does not silently do
    /// nothing**, and it is not the same event as one that writes no record on
    /// purpose.
    ///
    /// This is what the third answer is *for*, and the cheap harness is the
    /// one that treats *not `Records`* as a no-op. A press that emitted
    /// `FadeDeck` would then look exactly like a press that emitted
    /// `SelectDeck` — nothing printed and nothing moved — and an operator
    /// would read the first as *the fade did not take* when what happened is
    /// *nobody has decided what a fade writes* (`Owed` is a question, not an
    /// error: ADR-0194).
    ///
    /// **Neither sentence is asserted word for word.** What has to hold is
    /// that the window says something, that it names the operation and the
    /// reason, and that the two answers are two different sentences.
    #[test]
    fn an_operation_whose_record_is_owed_is_said_rather_than_swallowed() {
        // Owed, and `NotSettled` is the reason: a fade needs the grid
        // quantised onto a musical instant and the transition settings that
        // no record carries.
        let fade = Operation::FadeDeck { deck: 1, to: 0.0 };
        let owed = written(&fade, &Current::default());
        assert_eq!(
            owed,
            Written::Owed(Owed::NotSettled),
            "a fade is not owed any more — this test names the operation it does, and \
             the one it names has to still be one nobody can write"
        );
        let said = unwritten(&fade, &owed).expect(
            "a fade owes a record and this window said nothing at all — a press whose \
             record nobody has decided how to write reads, in silence, exactly like a \
             press that did not work",
        );
        assert!(
            said.contains("FadeDeck") && said.contains(Owed::NotSettled.why()),
            "the window said `{said}`, which does not name both the operation and the \
             question it is waiting on"
        );

        // Silent, and settled: which deck the keys are addressed to is a
        // surface's own state and there is nothing to write.
        let select = Operation::SelectDeck { deck: 1 };
        let silent = written(&select, &Current::default());
        assert_eq!(silent, Written::Silent(Silent::Surface));
        let settled = unwritten(&select, &silent).expect(
            "selecting a deck writes no record and the window said nothing about it \
             either, so a press on such a control would leave no trace at all",
        );
        assert!(
            settled.contains(Silent::Surface.why()),
            "the window said `{settled}`, which does not say why there is no record"
        );

        // **And the two are different sentences.** Collapsing them is the
        // failure this whole test is about at one remove: a harness that
        // printed one line for both would tell an operator that an undecided
        // fade is as settled as a deck selection.
        assert_ne!(
            said, settled,
            "a record nobody can write yet and a record nobody needs to write came out \
             of this window as the same sentence"
        );

        // A record's line is `apply`'s — it says the record *and* what the
        // deck holds afterwards — so this says nothing about that case.
        // Otherwise one press prints twice.
        let gain = Operation::SetGain { deck: 0, gain: 0.5 };
        assert_eq!(
            unwritten(&gain, &written(&gain, &Current::default())),
            None,
            "an operation that wrote a record was also announced as writing none"
        );
    }

    /// [`tally`] the other way round, for the assertion above alone — the
    /// vocabulary's residency as the engine's, so that the round trip through
    /// the wire name can be compared against something.
    fn residency_back(residency: karakuri_operation::Residency) -> Residency {
        match residency {
            karakuri_operation::Residency::Live => Residency::Live,
            karakuri_operation::Residency::Priming => Residency::Priming,
            karakuri_operation::Residency::Allocated => Residency::Allocated,
        }
    }

    /// [`blend_mode`] the other way round, for the assertion above alone —
    /// which is why it is here and not beside it: nothing the program *runs*
    /// needs to go this direction, and a conversion in `src` with one test as
    /// its only caller would be an abstraction with no second call site.
    fn blend_mode_back(blend: BlendMode) -> Blend {
        match blend {
            BlendMode::Add => Blend::Add,
            BlendMode::Over => Blend::Over,
            BlendMode::Max => Blend::Max,
        }
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
    /// So the assertion is over every sink, not over the one this program
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
        // program leaves off, because the bug is a sink that is not read here.
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

    /// **Two paths or none, and anything else is a refusal rather than a
    /// guess.**
    ///
    /// [`sources_from`] is the whole of this program's command line and this is
    /// what stops it growing a second one. The mistake it will actually be
    /// given is *one* path — a Set is two files and reads like one thing — and
    /// that is refused by name rather than paired with a default renderer,
    /// because a program that silently supplied half the material would draw
    /// something nobody asked for and say nothing about it.
    ///
    /// A CPU test: nothing here opens a file, and a path that does not exist is
    /// still a path. What is behind one is [`checked`]'s to complain about.
    #[test]
    fn a_set_is_two_paths_or_none_and_anything_else_is_refused() {
        let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));

        let bare = of(&[]).expect("no arguments is the pair the repository ships");
        assert_eq!(bare.l1, Sources::default().l1);
        assert_eq!(bare.l4, Sources::default().l4);
        assert!(
            bare.l1.is_file() && bare.l4.is_file(),
            "the default pair is not on the disk at {} and {}, so a bare run cannot draw",
            bare.l1.display(),
            bare.l4.display()
        );

        let named = of(&["a/geo.kir", "b/ren.kir"]).expect("two paths are a Set");
        assert_eq!(named.l1, std::path::PathBuf::from("a/geo.kir"));
        assert_eq!(named.l4, std::path::PathBuf::from("b/ren.kir"));
        assert_eq!(
            named.material(),
            "geo + ren",
            "the strip is not named after what was actually loaded"
        );

        let one = of(&["a/geo.kir"]).expect_err(
            "one path was read as a Set, so this program would have invented the other half",
        );
        assert!(
            one.contains("a/geo.kir"),
            "the refusal `{one}` does not name the path it refused"
        );
        assert!(
            of(&["a.kir", "b.kir", "c.kir"]).is_err(),
            "three paths were read as a Set"
        );

        // **An empty message is `--help`**, which is the one arm that is a
        // request rather than a mistake — [`main`] prints [`USAGE`] to stdout
        // and exits 0 on it, and prints it to stderr and exits 2 on every
        // other. A refusal that came back empty would be a silent exit.
        assert_eq!(of(&["--help"]).err(), Some(String::new()));
        assert_eq!(of(&["-h"]).err(), Some(String::new()));
        assert!(
            !one.is_empty(),
            "a refusal came back with no sentence in it, which `main` reads as `--help`"
        );
    }

    /// **The capacity is the L1's own declaration, read off the `Checked`.**
    ///
    /// It was `const CAPACITY: u32 = 262144` here — `drift_shell.kir`'s
    /// declared default, transcribed — for as long as this file could only ever
    /// load that one file. It takes a path now, so a transcription would be
    /// right about one `.kir` and silently wrong about every other: a procedure
    /// written for 131072 elements would run at 262144 and nothing would say
    /// so.
    ///
    /// **It is not `karakuri_ir::DEFAULT_CAPACITY` either**, which is the
    /// language default for a file that declared nothing and is what
    /// `check_header` makes unreachable for an L1 that passed checking. The
    /// number below is asserted rather than derived on purpose: it is what a
    /// bare `cargo run -p karakuri` runs, which is what makes the reading
    /// [`Costs::say`] prints comparable with the rest of this repository's
    /// figures (`docs/contributing.md` §1's reference workload).
    #[test]
    fn the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none() {
        let sources = Sources::default();
        let l1 = checked(&sources.l1);
        let declared = l1
            .capacity
            .expect("an L1 that passed contract checking always carries a capacity");
        assert_eq!(
            declared.default, 262_144,
            "the pair this program plays by default is no longer the workspace's \
             reference workload, and the reading printed after three quiet seconds \
             says it is"
        );
        assert!(
            declared.contains(declared.default),
            "the file's own default is outside the range the same file declares"
        );

        assert_eq!(capacity_of(&l1), declared.default);

        assert!(
            checked(&sources.l4).capacity.is_none(),
            "the renderer declares a capacity — `Set::build` is handed the L1's, and \
             two declarations would be two answers to how many elements there are"
        );

        // **A second L1, and it is the one that tells the two mistakes apart.**
        // `drift_shell.kir` declares 262144, which is also
        // `karakuri_ir::DEFAULT_CAPACITY` — so every assertion above passes
        // just as well against a [`capacity_of`] that ignored the file and
        // returned the language default. `strand_shell.kir` declares 131072
        // and says why in the file (512 strands x 256 samples), and it is what
        // that defect fails on.
        let other = checked(&sources.l1.with_file_name("strand_shell.kir"));
        assert_eq!(
            capacity_of(&other),
            131_072,
            "a second procedure did not run at what it declares — the capacity is being \
             read from somewhere other than the file"
        );
        assert_ne!(
            capacity_of(&other),
            karakuri_ir::DEFAULT_CAPACITY,
            "the second procedure declares the language default, so this test can no \
             longer tell a per-file read from a constant — pick another `.kir`"
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
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &Sources::default(),
            panel.layout(),
            1.0,
        );
        // **Built at what the file declares**, which is the other half of
        // `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none`:
        // that one says what the `.kir` says, and this one says the deck was
        // built with it rather than with a number written here.
        assert_eq!(
            engine.capacity,
            checked(&Sources::default().l1)
                .capacity
                .expect("the L1 declares a capacity")
                .default,
            "the deck was not built at the capacity its L1 declares"
        );

        // **Aimed by the call the window makes, and the view is what that
        // answered** rather than three lines this test writes by hand: an id
        // or a rectangle assembled here is a test agreeing with itself about
        // the one thing `Engine::aim` exists to decide. Deck A auditions and
        // B, C and D are off, which is the whole of what one *live* slot can
        // show — the second slot is parked and makes no texels to audition.
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
        let engine = Engine::new(
            &gpu,
            &mut renderer,
            &Sources::default(),
            panel.layout(),
            1.0,
        );

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
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &Sources::default(),
            panel.layout(),
            1.0,
        );
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
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &Sources::default(),
            panel.layout(),
            1.0,
        );

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

    /// **The whole loop, closed on a real deck: a hand moves a knob and the
    /// strip follows because the *deck* changed.**
    ///
    /// `tests/fader.rs` asserts everything up to the operation and one thing
    /// past it — that the console keeps no value of its own — and it does all
    /// of that with no deck anywhere, which is the point of that file. This is
    /// the other end, and it needs a device because a `Deck` does: the
    /// operation becomes a `Record`, the record moves the deck, and the strips
    /// are read back off the deck by [`mixer`] exactly as the frame reads
    /// them.
    ///
    /// **The middle step is the one worth the device.** Between the drag and
    /// the record the strip must *not* have moved — if it had, the console
    /// would be showing a number it kept rather than one the deck holds, and
    /// every assertion after it would pass over a second copy of the deck's
    /// state.
    #[test]
    fn a_drag_moves_the_deck_and_the_strip_follows_the_deck() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &Sources::default(),
            panel.layout(),
            1.0,
        );

        // The strips, written the way the frame writes them.
        let material = Sources::default().material();
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(strips.len(), engine.deck.slot_count());
        let was = engine.deck.gain(0);

        // A knob, taken hold of and dragged to the bottom of its track. The
        // context has to have drawn once, because a strip is laid out with the
        // type in it.
        let ctx = super::tests::drawn_once();
        let bay = mixer_bay(&ctx, panel.layout(), &strips).expect("the bay draws its strip");
        let at = bay.strip(0);
        let knob = at.trim_at(strips[0].gain).knob.center();
        let grab = bay
            .grab(Point::new(knob.x, knob.y))
            .expect("the trim's knob");
        panel.grab(Point::new(knob.x, knob.y), grab);
        let floor = Point::new(at.trim.min.x, knob.y);
        let Some(Dragged::Fader(operation)) = panel.moved(floor) else {
            panic!("a drag to the floor of the trim emitted nothing")
        };
        assert_eq!(operation, Operation::SetGain { deck: 0, gain: 0.0 });

        // **Nothing has been told anything yet**, so the deck is where it was
        // and so is the strip the frame would draw.
        assert_eq!(engine.deck.gain(0), was);
        let mut after = Vec::new();
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after, strips,
            "the strip moved before the deck did, so the console is keeping a value"
        );

        // The record, and the deck.
        let record = super::tests::only_record(&operation);
        assert!(apply(&record, &mut engine.deck).is_some());
        assert_eq!(
            engine.deck.gain(0),
            0.0,
            "the record was built and the deck did not move, so the control ends nowhere"
        );

        // And now the strip follows, because it is read off the deck.
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(after[0].gain, 0.0);
        assert_ne!(
            after, strips,
            "the deck moved and the strip did not follow it"
        );

        // The other direction, so that *follows the deck* is not *always
        // zero*: something else writes the deck and the strip says so without
        // a pointer anywhere near it.
        engine.deck.set_gain(0, 0.5);
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(after[0].gain, 0.5);
    }

    /// **A parked deck is reachable by running this window, and both of its
    /// residencies reach the strips.**
    ///
    /// [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
    /// drew a chip that rolls while a request is outstanding and
    /// `tests/parked.rs` asserts every pixel of it — from strips written by
    /// hand. This is the other question, and it was the one answered `no`:
    /// **whether the engine can put this panel in that state at all.** A
    /// `Deck` grants every residency it is asked for until something governs,
    /// so before [`Engine::ask_to_prime`] the two halves of the pair could not
    /// disagree here however long anybody ran the program, and the animation
    /// that is fully tested was unreachable in the one place a person would
    /// look at it.
    ///
    /// Every step is the product's: two Sets are built, `Deck::measure_slots`
    /// measures them with a real probe, the budget is set from what it
    /// measured, [`Deck::govern`] refuses, and [`mixer`] reads the two
    /// residencies back off the deck the way the frame does. **The reason is
    /// asserted and not only the park**, because three of the four reasons
    /// that satisfy `Deck::is_parked` mean this program forgot to do something
    /// — `Unmeasured` and `CommittedUnknown` are a probe that never ran, and
    /// `NoPrimingNeeded` is a closed-form Set that never needed warming. Only
    /// `NoHeadroom` is the budget refusing.
    #[test]
    fn the_budget_parks_a_deck_and_the_strip_carries_both_residencies() {
        use karakuri_engine::governor::Reason;

        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &Sources::default(),
            panel.layout(),
            1.0,
        );
        let material = Sources::default().material();

        // **Before the pass**, which is the state this file was in for its
        // whole life: a deck out of `Deck::new` is Live on every slot, the two
        // residencies agree, and nothing is pending.
        let mut before = Vec::new();
        mixer(&engine.deck, &material, &mut before);
        assert_eq!(before.len(), 2, "the deck is not two slots");
        assert!(
            before.iter().all(|strip| strip.pending().is_none()),
            "a strip was pending before anything had asked for anything"
        );

        let governed = engine.ask_to_prime(&gpu);

        // The engine's predicate first, since the console's is derived from
        // the same two values.
        assert!(
            engine.deck.is_parked(ASKED_TO_PRIME),
            "the request was granted rather than parked — {governed}"
        );
        let parked: Vec<_> = governed.parked().collect();
        assert_eq!(parked.len(), 1, "{governed}");
        assert_eq!(parked[0].slot, ASKED_TO_PRIME);
        assert_eq!(
            parked[0].reason,
            Reason::NoHeadroom,
            "deck B is parked for a reason that is not the budget — {governed}"
        );
        assert_eq!(
            engine.deck.residency(ON_AIR),
            Residency::Live,
            "the governor took the picture off air"
        );

        // And both residencies cross the seam, which is what the roll is drawn
        // from: the strip carries the pair and the view derives the rest.
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(strips[ON_AIR].tally, view::Tally::Live);
        assert_eq!(
            strips[ON_AIR].pending(),
            None,
            "the live strip is pending something"
        );
        assert_eq!(strips[ASKED_TO_PRIME].tally, view::Tally::Allocated);
        assert_eq!(strips[ASKED_TO_PRIME].requested, view::Tally::Priming);
        assert_eq!(
            strips[ASKED_TO_PRIME].pending(),
            Some(view::Tally::Priming),
            "the strip's two residencies agree, so the chip has nothing to roll toward"
        );

        // And the panel is live for as long as they disagree **and the bay
        // the chip is in is laid out**, which is the declaration
        // `tests/parked.rs` asserts against strips and folds of its own. The
        // panel here is this program's own, unfolded, which is the arrangement
        // this window opens on.
        let mut readout = Readout::new(1440.0, 900.0);
        readout.view.mixer = strips;
        assert_eq!(
            readout.view.animating(readout.panel.layout()),
            Some(view::ROLL_STALENESS),
            "a parked deck declared no staleness, so the roll never gets a frame"
        );
    }

    /// **The whole loop, closed on a parked deck: a press on the tally chip
    /// withdraws the prime request the governor could not grant, and the strip
    /// stops rolling because the *deck* changed.**
    ///
    /// `tests/tally.rs` asserts everything up to the operation with no deck
    /// anywhere, which is the point of that file. This is the other end, and
    /// it needs a device because a `Deck` does — and because the state under
    /// test is one only a governor pass can produce
    /// ([ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)):
    /// the request is asked for through the product's own path, refused by a
    /// budget computed from what the probe measured, and read back off the deck
    /// by [`mixer`] exactly as the frame reads it.
    ///
    /// **What separates this from a plausible wrong answer is which residency
    /// the press names.** The parked strip *shows* `alloc` and was *asked for*
    /// `prim`. A chip cycling from what it shows would ask for `live` and put
    /// deck B on air; cycling from the request asks for `allocated`, which is
    /// the withdrawal — and both are asserted here, on the operation and again
    /// on the deck, because the two are the same mistake at two removes
    /// (ADR-0195).
    ///
    /// **The middle step is the one worth the device**, as in the fader's
    /// test: between the press and the record the deck must not have moved,
    /// or the console would be applying what it is only supposed to ask for.
    #[test]
    fn a_press_on_a_parked_tally_withdraws_the_request_and_the_strip_follows_the_deck() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &Sources::default(),
            panel.layout(),
            1.0,
        );
        let material = Sources::default().material();

        // The state, produced by the governor and not written here.
        let governed = engine.ask_to_prime(&gpu);
        assert!(
            engine.deck.is_parked(ASKED_TO_PRIME),
            "the request was granted rather than parked — {governed}"
        );
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(
            strips[ASKED_TO_PRIME].pending(),
            Some(view::Tally::Priming),
            "the strip is not pending, so this test cannot tell the request from the readout"
        );

        // The press, at the centre of that strip's chip.
        let ctx = super::tests::drawn_once();
        let bay = mixer_bay(&ctx, panel.layout(), &strips).expect("the bay draws its strips");
        let chip = bay.strip(ASKED_TO_PRIME).tally.center();
        let operation = bay
            .tally(Point::new(chip.x, chip.y))
            .expect("the tally chip of the parked strip");
        assert_eq!(
            operation,
            Operation::SetResidency {
                deck: ASKED_TO_PRIME as u8,
                residency: karakuri_operation::Residency::Allocated,
            },
            "a press on the parked chip did not ask for the prime request to be withdrawn"
        );
        assert_ne!(
            operation,
            Operation::SetResidency {
                deck: ASKED_TO_PRIME as u8,
                residency: karakuri_operation::Residency::Live,
            },
            "the chip cycled from the residency it is showing, so a press meant to withdraw a \
             request would have put deck B on air"
        );

        // **Nothing has been told anything yet**, so the deck is where the
        // governor left it and so is the strip the frame would draw.
        assert!(engine.deck.is_parked(ASKED_TO_PRIME));
        let mut after = Vec::new();
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after, strips,
            "the strip moved before the deck did, so the console is keeping a value"
        );

        // The record, and the deck. `apply` governs after it, which is what
        // stops this from being a residency the budget never granted.
        let record = super::tests::only_record(&operation);
        assert_eq!(
            record,
            Record::Residency {
                slot: ASKED_TO_PRIME as u8,
                level: "allocated".to_owned(),
            }
        );
        assert!(apply(&record, &mut engine.deck).is_some());
        assert_eq!(
            engine.deck.requested_residency(ASKED_TO_PRIME),
            Residency::Allocated,
            "the record was built and the request did not move, so the control ends nowhere"
        );
        assert_eq!(
            engine.deck.residency(ASKED_TO_PRIME),
            Residency::Allocated,
            "the withdrawal left the slot somewhere other than where it was being held"
        );
        assert!(
            !engine.deck.is_parked(ASKED_TO_PRIME),
            "the slot is still parked, so the request was not withdrawn"
        );
        assert_eq!(
            engine.deck.residency(ON_AIR),
            Residency::Live,
            "withdrawing deck B's request took deck A off air"
        );

        // And now the strip follows, because it is read off the deck: nothing
        // is pending, so nothing rolls and the panel is still again.
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after[ASKED_TO_PRIME].pending(),
            None,
            "the deck stopped being parked and the strip went on rolling"
        );
        assert_ne!(
            after, strips,
            "the deck moved and the strip did not follow it"
        );
        let mut readout = Readout::new(W as f32, H as f32);
        readout.view.mixer = after.clone();
        assert_eq!(
            readout.view.animating(readout.panel.layout()),
            None,
            "the request is withdrawn and the panel is still asking for frames to roll a chip"
        );

        // And the cycle goes on from what the deck now holds rather than from
        // anything the console remembered: the next press asks for `live`.
        let bay = mixer_bay(&ctx, panel.layout(), &after).expect("the bay draws its strips");
        assert_eq!(
            bay.tally(Point::new(chip.x, chip.y)),
            Some(Operation::SetResidency {
                deck: ASKED_TO_PRIME as u8,
                residency: karakuri_operation::Residency::Live,
            }),
            "the second press did not carry on round the cycle from the deck's own request"
        );

        // **And the governor pass in `apply` is what makes a request a
        // request.** Asked to prime again — the record the chip writes when
        // the cycle comes round to it — the budget is still the budget, so the
        // slot is parked again rather than granted. Without the pass
        // `Deck::set_residency` would grant it on the spot and this panel
        // would draw a primed deck the governor never admitted, which is
        // ADR-0191 read forwards.
        let again = Record::Residency {
            slot: ASKED_TO_PRIME as u8,
            level: "priming".to_owned(),
        };
        assert!(apply(&again, &mut engine.deck).is_some());
        assert_eq!(
            engine.deck.requested_residency(ASKED_TO_PRIME),
            Residency::Priming,
            "the request was not written"
        );
        assert!(
            engine.deck.is_parked(ASKED_TO_PRIME),
            "the prime request was granted rather than parked, so nothing governed the record \
             and the panel is drawing a residency the budget never allowed"
        );
    }

    /// **The whole loop, closed on a masked deck: a press on the mask mini
    /// chooses the next shape and leaves the front, the soft edge and — the
    /// one this test exists for — the *angle* exactly where they were.**
    ///
    /// `tests/mask.rs` asserts everything up to the operation with no deck
    /// anywhere, which is the point of that file. This is the other end, and
    /// it needs a device because a `Deck` does.
    ///
    /// **What separates this from the plausible wrong answer is the angle.**
    /// The chip names a *shape*; `Operation::SetMaskShape` carries a shape and
    /// an angle (ADR-0201), so a press must carry an angle it does not
    /// control. Carrying the one the slot already wears is the whole of
    /// [ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md);
    /// carrying `0.0` would look like a chip minding its own business and
    /// would straighten a diagonal wipe on every press, with nothing on the
    /// panel saying so — the mark is the same mark at any angle.
    ///
    /// **The middle step is the one worth the device**, as in the fader's test
    /// and the tally's: between the press and the record the deck must not
    /// have moved, or the console would be applying what it is only supposed
    /// to ask for.
    #[test]
    fn a_press_on_the_mask_mini_chooses_a_shape_and_keeps_the_angle() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// A diagonal front, in radians — **not zero, and not the default**,
        /// which is the only reason this test can tell the two designs apart.
        const ANGLE: f32 = 0.9;
        /// Half way across, and a soft edge, so that a record written from the
        /// operation alone would show up in these two as well.
        const FRONT: f32 = 0.4;
        const SOFTNESS: f32 = 0.05;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &Sources::default(),
            panel.layout(),
            1.0,
        );
        let material = Sources::default().material();

        // A wipe in progress on the deck that is on air: a straight front,
        // running at an angle, part of the way across.
        engine
            .deck
            .set_mask(ON_AIR, Mask::new(MaskKind::Linear, ANGLE, FRONT, SOFTNESS));
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(
            strips[ON_AIR].mask,
            view::Mask::Linear,
            "the strip is not showing the mask the deck is wearing"
        );
        assert_eq!(
            strips[ON_AIR].mask_angle, ANGLE,
            "the strip did not carry the angle off the deck, so a press has nothing to \
             hand back and this test cannot tell the two designs apart"
        );

        // The press, at the centre of that strip's mini.
        let ctx = super::tests::drawn_once();
        let bay = mixer_bay(&ctx, panel.layout(), &strips).expect("the bay draws its strips");
        let chip = bay.strip(ON_AIR).mask.center();
        let operation = bay
            .mask(Point::new(chip.x, chip.y))
            .expect("the mask mini of the masked strip");
        assert_eq!(
            operation,
            Operation::SetMaskShape {
                deck: ON_AIR as u8,
                kind: karakuri_operation::WipeKind::Radial,
                angle: ANGLE,
            },
            "the press did not ask for the next shape at the angle the deck is wearing"
        );
        assert_ne!(
            operation,
            Operation::SetMaskShape {
                deck: ON_AIR as u8,
                kind: karakuri_operation::WipeKind::Radial,
                angle: 0.0,
            },
            "the press sent a default angle, so choosing a shape straightens a diagonal \
             wipe and nothing on the panel says so"
        );

        // **Nothing has been told anything yet**, so the deck is where it was
        // and so is the strip the frame would draw.
        assert_eq!(engine.deck.mask(ON_AIR).kind(), MaskKind::Linear);
        let mut after = Vec::new();
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after, strips,
            "the strip moved before the deck did, so the console is keeping a value"
        );

        // The record, out of the operation and the reading the harness takes
        // off the deck — written **whole**, which is the half the operation
        // does not name (ADR-0201).
        let written = written(&operation, &reading(&operation, &engine.deck));
        let Written::Records(records) = &written else {
            panic!("a press on the mask mini wrote no record: {written:?}")
        };
        assert_eq!(
            records.as_slice(),
            [Record::Mask {
                slot: ON_AIR as u8,
                kind: "radial".to_owned(),
                angle: ANGLE,
                position: FRONT,
                softness: SOFTNESS,
            }],
            "the record is not the whole mask with only the shape changed"
        );

        // And the deck.
        assert!(apply(&records[0], &mut engine.deck).is_some());
        let mask = engine.deck.mask(ON_AIR);
        assert_eq!(
            mask.kind(),
            MaskKind::Radial,
            "the record was built and the shape did not move, so the control ends nowhere"
        );
        assert_eq!(
            mask.angle(),
            ANGLE,
            "choosing a shape straightened the front — the angle the chip does not control \
             was rewritten by a press meant to choose a shape"
        );
        assert_eq!(
            mask.position(),
            FRONT,
            "choosing a shape moved the front, which is the half of the mask this operation \
             does not name"
        );
        assert_eq!(mask.softness(), SOFTNESS, "the soft edge was rewritten");

        // And the strip follows, because it is read off the deck rather than
        // remembered — and the next press carries on round the cycle from what
        // the deck now holds, still at the same angle.
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(after[ON_AIR].mask, view::Mask::Radial);
        assert_eq!(after[ON_AIR].mask_angle, ANGLE);
        let bay = mixer_bay(&ctx, panel.layout(), &after).expect("the bay draws its strips");
        assert_eq!(
            bay.mask(Point::new(chip.x, chip.y)),
            Some(Operation::SetMaskShape {
                deck: ON_AIR as u8,
                kind: karakuri_operation::WipeKind::None,
                angle: ANGLE,
            }),
            "the second press did not wrap round to `none` at the angle the deck still holds"
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
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &Sources::default(),
            panel.layout(),
            1.0,
        );

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
        // The console has thirteen regions and seven headings, so a frame that
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
