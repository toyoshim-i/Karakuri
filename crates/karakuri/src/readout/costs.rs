//! Cost tracking, still-panel budget measurement, allocator counting, and drift checking.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::time::{Duration, Instant};

use karakuri_console::budget::PANEL_PASS;
use karakuri_engine::probe::MeasurementMethod;

use super::*;
use crate::{PROFILE, WINDOW};

// ---------------------------------------------------------------------------
// What a still panel costs
// ---------------------------------------------------------------------------

/// How long the window has to go untouched before the reading is taken, and the
/// stretch every number in it is measured over.
///
/// The measurement is the claim now, and it used to be its own opposite. What
/// was here drove the window for 180 frames and reported what one of them cost;
/// the loop had to spin for the sample to fill, so the number described a
/// program that no longer exists the moment the loop stops spinning.
/// [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s
/// still-panel clause claims that nothing is drawn at all while nothing is
/// happening, so that is what is counted: frames drawn in three seconds of an
/// untouched window, and what they allocated.
///
/// Three seconds because a 60 Hz spin fills it with 180 frames — the old
/// sample, to the frame, so the two readings are about the same stretch of wall
/// clock — and because a twelve-second run has room for it several times over.
pub(crate) const STILL: Duration = Duration::from_secs(3);

/// How many frames the per-frame sample holds.
///
/// It is a cap and not a target: nothing drives the window to fill it, and a
/// run where nobody touches anything leaves it nearly empty, which is the
/// point. Allocated once, so measuring does not allocate on the path it is
/// measuring.
pub(crate) const SAMPLE: usize = 240;

/// What the `egui` pass on this panel last read, and the day it was read — the
/// figure the reading below quotes, and the figure it holds the run it has just
/// taken against.
///
/// It is a constant rather than a sentence because a number written into prose
/// reads as current forever, and this one did. The line that quotes it cited
/// [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s
/// 184 allocations and 226.2 kB and said the `egui` pass "is still that", which
/// stopped being true when the mixer bay landed — 456 there, and 525 once deck
/// B was parked
/// ([ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)).
/// Nobody re-checked it for two commits, because nothing was checking it.
///
/// Taken on 2026-08-31, over nine runs of this program on an Apple M4 Pro with
/// nothing touching the window, at the window size [`WINDOW`] opens: every one
/// of the nine read the same per-frame median, and it is the two figures below.
/// What the panel had in it while they were taken is the last paragraph the
/// reading prints. Re-take all three together, several runs at a time — one run
/// is not a number here — and re-date them.
///
/// The spread was nothing at all, which is a reading and not a guarantee. The
/// nine of 2026-08-26 disagreed by 14 allocations and these nine agreed to the
/// allocation, because an untouched panel tessellates the same work every frame
/// and nothing in the run varies it. It is not a promise that a tenth run
/// agrees, and it is not a licence to take one: what makes a number here
/// trustworthy is that several runs were asked, and a single run's median is
/// what produced the last wrong one.
///
/// The reading before this one predicted 1.26x and the panel did 2.89x, which
/// is kept because it is the argument for the counter rather than against it.
/// 2026-08-26 read 525, and the note beside it reasoned from ADR-0191's 69
/// allocations and 137.4 kB a strip that the two mixer strips since would put
/// it near 663 and 969 kB — inside [`DRIFT`], so the run would have called the
/// sentence current. It was not two strips that landed. Three things the last
/// reading's own *what the panel had in it* paragraph does not mention are on
/// the panel now: the Inspector draws two panes off the running Set, the
/// transport row draws an armed `audio-in` pill over an input measured every
/// frame, and it draws the arrangement pill. A figure predicted from the one
/// change somebody remembered is precisely the figure that goes stale in
/// silence, and re-taking it needs a window, three still seconds and several
/// runs, none of which is reachable from `cargo test`.
pub(crate) const WRITTEN_ALLOCS: u64 = 1518;
pub(crate) const WRITTEN_KB: f64 = 1781.6;
pub(crate) const WRITTEN_ON: &str = "2026-08-31";

/// How far a run may sit from [`WRITTEN_ALLOCS`] before the reading says the
/// sentence quoting it has gone stale.
///
/// A factor, and a generous one, because an allocation count is not a constant:
/// a hard equality here would be a guard nobody could keep passing. The nine
/// runs behind 2026-08-26's figure disagreed by 14 allocations; the nine behind
/// the current one agreed to the allocation, and one machine's nine agreeing is
/// not a promise the next machine's will. Two is the smallest factor that still
/// catches what actually happened — 184 to the mixer bay's 456 is 2.5x, so a
/// band of two would have said so on the first run after that bay landed, and a
/// band of ten would not have. It is also what caught 525 going to 1518.
///
/// Two figures are held and the bytes are not, which is a distinction rather
/// than an omission: the bytes move with the allocation count, so a verdict on
/// them would be the same verdict twice. The second is [`PANEL_PASS`] — what
/// one update of a live region costs, declared under
/// [P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
/// and read by `tests/schedulable.rs` rather than by anything at runtime. It is
/// a different claim from an allocation count and it is the one a
/// schedulability condition is asserted against, so it gets its own verdict.
pub(crate) const DRIFT: f64 = 2.0;

/// Has the reading moved away from the sentence that quotes it? `Some` is the
/// factor between them, where that factor is past [`DRIFT`] in either
/// direction.
///
/// Either direction on purpose: a pass that got cheaper makes the sentence
/// exactly as untrue as one that got dearer, and only one of those two is ever
/// noticed by accident.
pub(crate) fn drifted(measured: u64, written: u64) -> Option<f64> {
    let factor = measured.max(written) as f64 / measured.min(written).max(1) as f64;
    (factor > DRIFT).then_some(factor)
}

/// [`drifted`] for a figure in milliseconds, which is what a cost is.
///
/// The same band and the same both-directions rule, said again for `f64`
/// because the two numbers are of different kinds and neither is convertible
/// into the other without saying something untrue about it. A run that read
/// nothing at all cannot divide, and a zero-length sample never reaches here —
/// the caller is inside `if !self.frames.is_empty()`.
pub(crate) fn drifted_ms(measured: f64, written: f64) -> Option<f64> {
    let factor = measured.max(written) / measured.min(written).max(f64::MIN_POSITIVE);
    (factor > DRIFT).then_some(factor)
}

/// The allocator, counting. Per thread, not per process — `wgpu` allocates on
/// threads of its own and a process-wide counter would attribute that to the
/// `egui` pass, which is the one number this exists to get right.
pub(crate) struct Counting;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
    static BYTES: Cell<u64> = const { Cell::new(0) };
}

/// `(allocations, bytes)` this thread has asked for since it started.
/// Reallocations count as one allocation of the new size, which overstates a
/// growing `Vec` and is the conservative direction.
pub(crate) fn counted() -> (u64, u64) {
    (
        ALLOCS.try_with(Cell::get).unwrap_or(0),
        BYTES.try_with(Cell::get).unwrap_or(0),
    )
}

pub(crate) fn count(size: usize) {
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
pub(crate) static ALLOCATOR: Counting = Counting;

/// What one frame cost.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Cost {
    /// The engine's half: [`compose`] from its first statement up to the moment it
    /// hands the encoder back for the panel — the picture's `acquire`, the
    /// committing closure, the tone-map write, `Deck::begin_frame` with every
    /// install in it, the deck's render, and the present pass into the picture if
    /// it took the frame. One deck render and one present pass, or none when the
    /// picture's region is folded away and its sink refuses.
    ///
    /// The preview cells are in this number and are inside `finally`, and the two
    /// are not in disagreement. [`monitor`]'s [`DECKS`] present passes are the
    /// first thing `finally` does and the panel's clock is started after them, so
    /// the cells are recorded here with the rest of the engine's work rather than
    /// against the panel — which is the split this field names. Nothing separates
    /// the cells from the picture; if they ever want a number of their own it is a
    /// field here and not a subtraction.
    ///
    /// Where it stops is the start of [`Cost::paint`] and not a second clock: the
    /// panel's half begins on the statement after [`monitor`] inside `finally`, so
    /// the two are adjacent by construction and no part of the frame falls between
    /// them. CPU time only, like everything else here; what the GPU then does with
    /// the command buffer is not on this clock, and `docs/contributing.md` §1 says
    /// why there is no other one.
    pub(crate) engine: Duration,
    /// `take_egui_input` through `tessellate`: the whole immediate-mode pass,
    /// including this console's own layout walk and every shape it emits.
    pub(crate) ui: Duration,
    /// Uploading the tessellated geometry, recording the panel's render pass, and
    /// the one submission that carries both halves — the whole of what this program
    /// records from inside [`compose`]'s `finally` after [`monitor`], plus
    /// `compose`'s own tail: `Frame::finish`, and the picture's `present`, which
    /// for this sink is nothing at all. Excludes `Queue::present` on the surface,
    /// which is the display's pace and not a cost.
    pub(crate) paint: Duration,
    /// Uploading `egui`'s texture deltas, out of [`Cost::paint`] — the font atlas
    /// and its patches. Zero on a steady frame, because the atlas is built once.
    ///
    /// This and the two below exist because of what happens when they do not. The
    /// split was taken once with temporary instrumentation, published as a
    /// conclusion and removed, and the next machine asked to answer *how much of
    /// the frame is the cost of moving data to the GPU* could not: the question is
    /// about `buffers` and the readout stopped at `paint`. A measurement that has
    /// to be re-instrumented to be repeated is one number rather than a series.
    pub(crate) textures: Duration,
    /// Uploading the tessellated geometry, out of [`Cost::paint`] — and the number
    /// the caching decision turns on.
    ///
    /// It is not the price of moving bytes, which took three machines to establish
    /// and is `docs/adr/0167-the-panel-keeps-re-uploading-what-did-not-change.md`.
    /// The discrete GPU that has to cross a bus pays 0.022 ms; an APU that crosses
    /// nothing pays 0.010; this machine, which also crosses nothing, pays 0.166 —
    /// seven times the one with the bus. It is what a backend's upload path costs,
    /// and the decision it was printed for is taken: not re-uploading the unchanged
    /// panel is worth at most 4% of a frame, and the dirty-tracking it needs is
    /// not.
    pub(crate) buffers: Duration,
    /// Recording the panel's render pass, out of [`Cost::paint`] — the view, the
    /// pass, and `egui`'s draw calls into it. Recording only; what the GPU then
    /// does with it is on no clock here.
    pub(crate) record: Duration,
    /// The submission alone, out of [`Cost::paint`] — `Queue::submit` and `finish`
    /// on the frame's encoder, which is [`compose`]'s last act and is why this is
    /// measured from inside `finally` to after `compose` returns. The only other
    /// thing in that window is each sink's `present`, and both of this program's
    /// are `Ok(())` — it is five sixths of `paint`.
    ///
    /// Printed because the whole of the rest of `paint` is what caching a bay into
    /// a texture would make cheaper, and this is not: it is `wgpu`'s per-submission
    /// cost, it is proportional to what was recorded rather than to what the GPU
    /// then does with it, and it is unmoved by a canvas 256 times the area. It is
    /// host-side work and not a wait — measured against `CLOCK_THREAD_CPUTIME_ID`
    /// it burns 99% of its wall time on the CPU. The wait is [`Cost::wait`], which
    /// is a different number entirely.
    pub(crate) submit: Duration,
    /// What the frame spent blocked in `get_current_texture`, waiting for the
    /// display to free a swapchain image. `PresentMode::Fifo`, so at 60 Hz this is
    /// most of the 16.6 ms and the frame is not paying for it: on the same clock as
    /// above it burns under 1% of itself on the CPU.
    ///
    /// It is in none of the three numbers above, which is why it is here — a reader
    /// who sums those three and compares the total to a frame gets an answer that
    /// is 12% of the truth, and the missing 88% is this doing nothing on purpose.
    /// Switching to `PresentMode::Immediate`, which this adapter does offer, moves
    /// it and nothing else.
    pub(crate) wait: Duration,
    /// What the frame cost the window: the wall clock from the top of one
    /// `RedrawRequested` to the top of the next, and the one number here with the
    /// wait, the present and everything this file times nowhere inside it.
    ///
    /// It is the answer to *what did this frame cost*, and the three fields above
    /// are not. They are CPU time by construction and [`Cost::whole`] leaves
    /// [`Cost::wait`] out of them on purpose, so a frame that spent 240 ms on the
    /// GPU and 5 ms on the CPU reads there as a 5 ms frame — which is not a
    /// rounding error, it is the difference between a loop that is idle and a loop
    /// that is at its limit. This is measured between two identical points of
    /// successive frames, so it tiles the run exactly: nothing falls between two
    /// periods and nothing is in two.
    ///
    /// Host clock, and it cannot be anything else. GPU timestamps bracket work
    /// inside a command buffer; most of this is not in one — the block in
    /// `get_current_texture`, the `egui` pass, `Queue::present` — so the whole
    /// frame is a host-clock figure on any adapter, working timestamps or not.
    /// [`Cost::drained`] is where the GPU's own share is asked for, and
    /// `Costs::clock` is what this program was able to establish about this
    /// adapter's timestamps rather than what it advertises (P-0095).
    ///
    /// A period is a cost only while the loop is asking for the next frame
    /// immediately, which on this program is whenever something is [`live`]. With
    /// nothing live the next frame comes off an `egui` deadline or an operator, and
    /// the interval is then how long the window was left alone rather than what a
    /// frame cost. The reading says which case it is printing.
    ///
    /// `None` on the first frame of a run, which has no predecessor to be an
    /// interval from.
    pub(crate) period: Option<Duration>,
    /// What the GPU still owed when the CPU had finished the frame — `Device::poll`
    /// to a drained queue, timed on the host clock, on the frames [`Costs::audit`]
    /// picks and `None` on every other.
    ///
    /// This is the field the three medians cannot have. They stop at the
    /// submission; a command buffer that takes a quarter of a second to execute
    /// costs the same in every one of them as one that takes a microsecond, and on
    /// a `Fifo` surface the difference surfaces one frame later as [`Cost::wait`] —
    /// where it is indistinguishable from the vsync idle that field is named for.
    ///
    /// What it includes, and it is not one pass. Everything the queue had
    /// outstanding when the poll started: this frame's whole command buffer — four
    /// slots stepped and drawn, the composite, the preview presents, the picture's
    /// present pass and the panel's — plus whatever of the previous frame was still
    /// in flight, plus the poll's own round trip. It is an upper bound on this
    /// frame's GPU work and it is biased high, in the same direction and for the
    /// same reason [`karakuri_engine::probe::MeasurementMethod::HostWallClock`] is.
    ///
    /// It is not free and it is not taken every frame. Blocking here stands the CPU
    /// still until the GPU catches up, which is the one pattern the frame path is
    /// not allowed to make a habit of; [`Costs::AUDIT`] is how rarely it happens
    /// and the reading prints what those frames cost against the rest rather than
    /// asserting it is negligible.
    ///
    /// It moves the frame after it, by exactly what it took. Work waited for here
    /// is work the next frame's [`Cost::wait`] does not have to wait for, so an
    /// audited frame shortens its successor's wait — which is the same milliseconds
    /// counted in a different field rather than any of them going missing, and it
    /// is one frame in the tens the reading samples.
    pub(crate) drained: Option<Duration>,
    /// Allocations and bytes during `ui`, on this thread.
    pub(crate) allocs: u64,
    pub(crate) bytes: u64,
}

impl Cost {
    /// What three timed stretches of the frame cost on the CPU: the three fields
    /// the reading below adds up under *"the whole frame is a median"*, for one
    /// frame rather than for the median of each.
    ///
    /// The three do not overlap — `engine` ends where `paint` begins, by
    /// construction, and `ui` is the `egui` pass before either — and [`Cost::wait`]
    /// is deliberately not in it: blocking in `get_current_texture` is the
    /// display's pace and not a price, which is the sentence that field carries.
    ///
    /// # This is not what the frame cost, and it used to say it was
    ///
    /// It said the three "tile the frame exactly", and they do not. They are three
    /// stretches of one redraw with untimed CPU between them — the sinks aimed, the
    /// Program bay rearranged, the panel solved, a listing re-read on the frame a
    /// save lands — and untimed CPU after the last of them, `Queue::present`
    /// included. [`Cost::elsewhere`] is what is left over when they and the wait
    /// are taken off [`Cost::period`], and it is measured rather than argued: the
    /// reading prints it, so the claim is now a number that this window either
    /// produces or does not.
    ///
    /// And it is CPU time whatever the GPU is doing, which is the more expensive
    /// half of the same mistake. Every field it sums stops at a submission, so a
    /// frame whose command buffer takes a quarter of a second to execute costs the
    /// same here as one that takes a microsecond. A panel running at four frames a
    /// second was read off these three as a loop *idle 97.6% of the time*, and the
    /// loop was not idle — it was waiting for a shader, one field along in
    /// [`Cost::wait`], where nothing distinguishes that from the vsync idle the
    /// field is named for.
    ///
    /// It is kept because it answers a different question: what this program's own
    /// code costs per frame, which is what a schedule of live regions is built from
    /// and what `budget::PANEL_PASS` is checked against. What a *frame* cost is
    /// [`Cost::period`], with [`Cost::drained`] beside it for the GPU's share.
    pub(crate) fn whole(&self) -> Duration {
        self.engine + self.ui + self.paint
    }

    /// What the frame spent where nothing here is looking: the period, less the
    /// wait and less the three stretches [`Cost::whole`] sums.
    ///
    /// A residue rather than a measurement, and it is one on purpose — the point of
    /// it is that a reader can see how much of the frame the timed fields did *not*
    /// see, without this file having to be trusted about where its clocks start and
    /// stop. It is the untimed CPU between them and after them, plus whatever
    /// `winit` does between two redraws.
    ///
    /// `None` on the first frame of a run, which has no period. Saturating, because
    /// the four stretches are read from four separate clocks and a residue of a few
    /// microseconds either side of zero is those clocks and not a negative
    /// duration.
    pub(crate) fn elsewhere(&self) -> Option<Duration> {
        Some(
            self.period?
                .saturating_sub(self.wait)
                .saturating_sub(self.whole()),
        )
    }
}

/// What was drawn while nobody was touching the window. This is the number
/// ADR-0164's still-panel clause is about, and the clause says every field of
/// it is zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Still {
    pub(crate) frames: usize,
    pub(crate) allocs: u64,
    pub(crate) bytes: u64,
}

/// The sample, the stillness, and the summary printed once.
pub(crate) struct Costs {
    /// What the frames that *were* asked for cost.
    pub(crate) frames: Vec<Cost>,
    /// Every frame drawn, including any past the end of `frames`.
    pub(crate) drawn: usize,
    /// When the window was last touched by anything at all.
    pub(crate) quiet_since: Instant,
    /// A frame is owed to something that happened, and the next one drawn is that
    /// frame rather than a frame drawn on a still panel.
    ///
    /// Without it the reading blames the panel for the frame it was asked for: an
    /// event resets the stretch and the frame it asked for lands a millisecond into
    /// the new one, so a window that did exactly the right thing reports having
    /// drawn on an untouched panel.
    pub(crate) owed: bool,
    /// What has been drawn since `quiet_since` that nothing asked for.
    pub(crate) still: Still,
    /// The frame just drawn, kept so the transport row can print what it cost.
    ///
    /// Not a second measurement and not a second sample: it is the last [`Cost`]
    /// [`Costs::push`] was handed, which `frames` stops keeping after [`SAMPLE`] of
    /// them because that vector is a sample rather than a history. A readout wants
    /// the last frame and the reading wants the sample; both are the same numbers.
    pub(crate) last: Option<Cost>,
    pub(crate) said: bool,
    /// What ran it, as the adapter reports it: the backend, the adapter's own name,
    /// and whether it calls itself discrete.
    ///
    /// Printed because a readout that does not name its backend is how a
    /// measurement gets written into a table as though it were another one's.
    /// `WGPU_BACKEND` was honoured by nothing for months and the failure was
    /// invisible precisely here — twenty-five runs were recorded as DX12 and were
    /// Vulkan, and nothing on screen could have said otherwise
    /// (`docs/adr/0168-a-backend-override-is-honoured-because-a-no-op-cannot-be-caught.md`).
    /// The adapter is asked rather than the environment, so an override that does
    /// not take reads as what it is.
    pub(crate) taken_on: String,
    /// Whether anything in the Program bay is making texels — the picture, a deck
    /// auditioning in a preview cell, or both. [`live`] is the rule and this is the
    /// frame's answer to it.
    ///
    /// It decides which sentence the reading prints about the frames it counted,
    /// and nothing else: the frames are counted the same way either way, which is
    /// the point — the number is not adjusted for knowing the answer.
    pub(crate) live: bool,
    /// What the panel declared it needed, as `View::animating` answered it on the
    /// last frame: the soonest *move* out of the regions that are declaring, and
    /// `None` only when none of them is.
    ///
    /// A deadline and not a rate, which is ADR-0283: a region declares its
    /// staleness for the motion it has, so a pending region that is at rest answers
    /// the rest it has left rather than a rate it is not using. The beat is the
    /// exception and it is the interesting one — the light travels on every frame
    /// the session advances, so its deadline is its declared staleness on every
    /// frame there is, and `24.671ms` here is the beat and nothing else.
    ///
    /// It is here for one sentence, and the sentence was wrong without it. With
    /// nothing in the Program bay making texels the reading used to blame the only
    /// other thing it knew about — an `egui` repaint delay answered immediately —
    /// and on this program that is never the answer: folding the picture and the
    /// preview row away leaves the window drawing 28.0 to 28.3 frames a second over
    /// two runs on 2026-08-26, which is the roll's declared 30 Hz and not a
    /// mishandled delay. Folding the mixer bay away as well takes it to 0 frames,
    /// because the chip that declares the 30 Hz is then not laid out (ADR-0193). A
    /// reading that names the wrong cause is worse than one that names none.
    ///
    /// Those two readings were taken before the beat declared, and the beat
    /// declares whenever the transport row is drawn and there is an engine behind
    /// it — 24.7 ms, about forty a second, whether or not anything is pending
    /// ([ADR-0212](../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md),
    /// [P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    /// So this window's `None` now needs the transport row folded away as well,
    /// which is a fourth fold and is the arm below saying so.
    ///
    /// And the roll's 30 Hz is no longer 30 Hz, which is what would make the
    /// 28.0-to-28.3 reading above unreproducible if it were taken again: with the
    /// transport row folded and a slot parked, the mixer asks for fourteen frames a
    /// second rather than thirty-one, because the seventeen that fell inside the
    /// roll's rest drew the chip in the position it was already in
    /// ([ADR-0283](../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md),
    /// `karakuri-console`'s `tests/moving.rs`, which counts both).
    pub(crate) declared: Option<Duration>,
    /// The top of the last frame, which is the one anchor a period is measured from
    /// — see [`Cost::period`] and [`Costs::tick`].
    ///
    /// It is the *same point* of every redraw, so two consecutive values bracket a
    /// whole frame with no gap and no overlap. `None` before the first one.
    pub(crate) began: Option<Instant>,
    /// When the last audited frame was taken — see [`Costs::audit`].
    pub(crate) since_audit: Instant,
    /// What this program was able to establish about this adapter's GPU timestamps,
    /// as the deck's own probe earned it: `GpuTimestamp` where calibration survived
    /// and nothing later caught it lying, `HostWallClock` where it did not, and
    /// `None` where nothing has been probed at all.
    ///
    /// It is not `Features::TIMESTAMP_QUERY`, and the difference is the whole of
    /// P-0095: the machine this was written on advertises the feature and does not
    /// deliver it. The reading prints this beside the frame figures so that *why is
    /// this a host clock* has an answer taken against a load rather than read off a
    /// flag — and prints `None` as *nothing asked* rather than as a verdict.
    pub(crate) clock: Option<MeasurementMethod>,
}

impl Costs {
    pub(crate) fn new() -> Costs {
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
            began: None,
            // **Now rather than the beginning of time**, so the first frame of
            // a run is not the audited one: it builds the font atlas and pays
            // for every pipeline the panel touches, and blocking on the GPU
            // there would measure a frame nothing else in this reading is
            // about.
            since_audit: Instant::now(),
            clock: None,
        }
    }

    /// How rarely a frame is audited — the frame that blocks on `Device::poll` to
    /// find out what the GPU still owed ([`Cost::drained`]).
    ///
    /// A stretch of wall clock and not a frame count, and the reason is the case
    /// this instrument exists for. Every so many frames sounds steadier and is
    /// exactly backwards: a loop at four frames a second is the loop that most
    /// needs the GPU's own number, and one audit in sixty frames would take fifteen
    /// seconds to produce one — five times the [`STILL`] the reading is taken over,
    /// so the reading would carry no GPU figure precisely where it is the whole
    /// answer. Half a second gives six samples over a three-second reading whether
    /// the window is drawing at four frames a second or at sixty.
    ///
    /// It costs least where it fires most often, which is the same argument from
    /// the other side: a frame that is already waiting on the GPU pays almost
    /// nothing to be told how long, because the wait is one it was about to take in
    /// `get_current_texture` anyway.
    ///
    /// The cost of it is printed rather than assumed. The reading prints what the
    /// audited frames' periods were against the rest, so a machine where this is
    /// not cheap says so on that machine instead of being reassured by a sentence
    /// written on another one.
    pub(crate) const AUDIT: Duration = Duration::from_millis(500);

    /// The top of a frame: store the anchor and hand back the interval since the
    /// previous one, which is [`Cost::period`].
    ///
    /// Called from one place, at the same statement of every redraw, because that
    /// is what makes two consecutive anchors a whole frame. A redraw that returns
    /// early — a surface to reconfigure, a frame to skip — still ticks here, so the
    /// frame after one of those carries a period spanning both; they are rare, they
    /// show up in the tail rather than the median, and the alternative is an anchor
    /// that moves depending on which branch a frame took.
    pub(crate) fn tick(&mut self, at: Instant) -> Option<Duration> {
        let period = self.began.map(|began| at - began);
        self.began = Some(at);
        period
    }

    /// Whether this frame pays for the GPU's answer — see [`Cost::drained`] and
    /// [`Costs::AUDIT`].
    ///
    /// Reads the clock rather than counting frames, so that the rate the answer
    /// arrives at does not fall away exactly as the frames get slower. It is asked
    /// once per frame and it is the ask that resets the stretch, so a caller that
    /// asks and then declines to measure has thrown a sample away rather than
    /// deferred one.
    pub(crate) fn audit(&mut self) -> bool {
        let now = Instant::now();
        match now.duration_since(self.since_audit) >= Costs::AUDIT {
            true => {
                self.since_audit = now;
                true
            }
            false => false,
        }
    }

    /// Something touched the window, so the stillness starts again from here and
    /// what was drawn during the last stretch is no longer about a still panel.
    ///
    /// Every window event that is not a frame this loop asked for itself counts,
    /// including the ones the operator did not cause — a move, a focus, an
    /// occlusion. Resetting too eagerly only ever makes the reading harder to
    /// reach, never easier to pass.
    pub(crate) fn touched(&mut self) {
        self.quiet_since = Instant::now();
        self.still = Still::default();
    }

    /// A frame was asked for by something that happened, so the next one drawn is
    /// not on the panel's account.
    ///
    /// Every `request_redraw` in this file goes through here except one, and the
    /// exception is the point of the reading: the frame `egui` asked for after a
    /// delay it named. The distinction is between `egui` saying *I have not
    /// finished drawing what you just asked me to* — a `repaint_delay` of zero, a
    /// second pass of a frame already owed, which is what it does for a pass or two
    /// while the font atlas settles — and `egui` saying *wake me in 250 ms*, which
    /// is an animation and is per-frame work on a panel nobody is touching. The
    /// first goes through here and the second does not, so a delay mishandled into
    /// a spin shows in the reading as the frames it actually drew.
    ///
    /// The other way it can fail is loud rather than quiet: something asking for
    /// frames without pause never lets the window be still for [`STILL`], and then
    /// no reading is printed at all. A run of this program that prints no reading
    /// is that failure.
    pub(crate) fn owes(&mut self) {
        self.owed = true;
        self.touched();
    }

    pub(crate) fn push(&mut self, cost: Cost) {
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

    /// Frames a second: what was drawn on the untouched window, over the stretch it
    /// was drawn in.
    ///
    /// The stretch is the caller's because the two callers are at different points
    /// in it and neither may guess the other's. [`Costs::say`] is taken exactly
    /// [`STILL`] after `quiet_since` and says so in the sentence above the number;
    /// the transport row is asked on every frame and its stretch is however much of
    /// one has elapsed. One quotient, two stretches — and a second expression of
    /// *frames over seconds* would be the readout and the reading disagreeing about
    /// the rate of the same window.
    pub(crate) fn rate_over(&self, stretch: f64) -> f64 {
        self.still.frames as f64 / stretch
    }

    /// The rate as it stands, for the transport row — or `None` where there is not
    /// yet a stretch with a frame in it to divide.
    ///
    /// `None` is the honest answer twice over. Just after something touched the
    /// window there is no stretch, and a rate over no time is an infinity. And a
    /// window with nothing live on it stops asking for frames entirely, so the
    /// stretch goes on growing while the frames do not — which is a rate falling
    /// towards zero and is exactly what the window is doing.
    pub(crate) fn rate_now(&self) -> Option<f64> {
        let stretch = self.quiet_since.elapsed().as_secs_f64();
        (self.still.frames > 0 && stretch > 0.0).then(|| self.rate_over(stretch))
    }

    /// When the reading is due, and `None` once it has been taken. It is also what
    /// keeps the loop on a deadline until then — see [`App::about_to_wait`].
    pub(crate) fn due(&self) -> Option<Instant> {
        match self.said {
            true => None,
            false => Some(self.quiet_since + STILL),
        }
    }

    /// The reading. Median and worst rather than a mean for the per-frame figures:
    /// a frame path is judged by its tail. `capacity` and `material` are the run's,
    /// handed in rather than read off a constant: this program takes its `.kir`
    /// pair from the command line and a load can move a slot off it, so what the
    /// engine half of this reading was taken over is only known at run time — and
    /// is every slot's name in slot order rather than one. See
    /// [`Engine::capacity`], [`Sources::material`] and [`Gfx::material`]. `at` is
    /// what the frame was actually composited at when the reading was taken —
    /// `Present::size()`, which is the largest enabled output's size and no longer
    /// a constant ([`render_size`]). A measurement names which resolution it is
    /// about
    /// ([ADR-0303](../../../docs/adr/0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md),
    /// [P-0095](../../../docs/principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md)),
    /// and [`CANVAS`] would now be the wrong one on any run whose Program bay is
    /// not exactly that size — which is every run.
    pub(crate) fn say(
        &mut self,
        capacity: u32,
        material: &str,
        refresh_ms: Option<f32>,
        at: (u32, u32),
    ) {
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
            // the four on screen it is not. The fourth is P-0094 arriving:
            // the beat is a light travelling the grid, it declares for as
            // long as it is drawn, and a console claiming to show a live
            // instrument has something moving on it (ADR-0212).
            (false, true) => println!(
                "  so ADR-0164's still-panel clause holds here: no per-frame work is \
                 done to redraw what nobody has touched and nothing has moved."
            ),
            (false, false) => match self.declared {
                // **The panel said it needed them**, and that is ADR-0164's
                // second clause working rather than its still-panel one failing: the
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
                //
                // **What is printed is a deadline and no longer a rate**, and
                // ADR-0283 is why: a region declares its staleness for the
                // motion it has, so the mixer's answer is 33.3 ms through the
                // 400 ms its roll travels and the remainder of the rest
                // through the 600 ms it does not. The reciprocal of one of
                // those is not the rate anything runs at, and printing it as
                // one is how a reading names the wrong cause. The rate the
                // window actually drew at is the line above this one, which is
                // the number that was measured rather than derived.
                Some(deadline) => println!(
                    "  so ADR-0164's still-panel clause does NOT hold here, and the \
                     reason is a declaration rather than a fault: {}, and the soonest \
                     a declaring region will next move is {:.1} ms away. Folding the \
                     region that draws it ends its term: a region that is not laid out \
                     declares nothing (ADR-0193).",
                    match deadline == view::BEAT_STALENESS {
                        // The one that runs whether or not anything is
                        // happening, which is the whole of why it is here —
                        // and the one region whose deadline is its declared
                        // staleness on every frame, because the light never
                        // rests (ADR-0283).
                        true =>
                            "the beat grid is a light travelling the transport row, \
                                 and it moves for as long as the console is live rather \
                                 than while something is pending (P-0094, ADR-0212)",
                        false =>
                            "something on this panel is parked and the mixer's tally \
                                  is rolling toward a residency nobody granted (ADR-0190)",
                    },
                    deadline.as_secs_f64() * 1000.0,
                ),
                // Nothing live, nothing declared, and frames drawn anyway.
                None => println!(
                    "  so ADR-0164's still-panel clause does NOT hold here — something \
                     is asking for frames on an untouched window, nothing on the panel is \
                     making texels and nothing has declared a staleness, so the likeliest \
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
                    "  so ADR-0164's still-panel clause has stopped holding, and the \
                     reason is the engine: there is a live frame in the Program bay — the \
                     picture, deck A auditioning in the preview row under it, or both — so \
                     every one of those frames was asked for by what is live rather than \
                     by anybody touching the window."
                );
                println!(
                    "  that is {rate:.1} frames a second, against 0 with the engine out — \
                     which is the whole of the difference, since what one frame costs is \
                     below and did not change."
                );
                println!(
                    "  it is P-0091 from here — anything that must be live declares its \
                     price — and this is the price, measured. Two regions declare it: the \
                     transport row for as long as the beat grid is drawn (P-0094, \
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
            // twice (ADR-0210); not `wait`, which is doing nothing on purpose;
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
                "  the CPU's three stretches are a median {:.3} ms between them, so at \
                 {:.1} frames a second the loop is spending {:.1}% of a second inside \
                 them. THAT IS NOT WHAT THE FRAME COST — see the block below, which \
                 measures the frame itself.",
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
                 {WRITTEN_KB:.1} kB a frame, which is what every one of the nine read — to \
                 the allocation, and to the tenth of a kilobyte. What the panel had in it \
                 while they were taken is the last paragraph below. What ADR-0164 is still right about is that \
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
            // **What P-0091 calls a cost, measured and held against what
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
                 is what one update of a live region costs under P-0091. The submission is \
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
                 off, the mixer bay with a strip in every one of its four tracks, the \
                 transport row with its `audio-in` and arrangement pills, the outputs row \
                 and deck B's parked \
                 tally rolling once a second — over the Library bay's scope row and however \
                 many rows the scope marked in it lists, over the Master bay's out row, over \
                 the Inspector's two panes read off the running Set, and over Staging and \
                 Sequencer, which are a head and nothing else. \
                 That is NOT the workspace's \
                 reference workload. The \
                 engine half is four slots of `{}` — {} elements each at {}x{}, and each \
                 advances by the frame's own measured step count, which on a 60 Hz \
                 display is one step a frame apiece and on a faster one is one step \
                 every second or third frame (ADR-0297). The other three are allocated, \
                 one of them parked, and \
                 every one of them steps and draws into its own cell on every frame \
                 (ADR-0269), so all four simulations and all four draws are in these \
                 numbers — and five presents at the canvas's own shape: each slot's \
                 canvas into its own preview cell, and the mix into the picture's \
                 rectangle. The workspace's reference workload is \
                 `examples/drift_cloud.kset` at 1280x720 (docs/contributing.md §1, \
                 ADR-0270) — a named Set rather than whatever this program opens on — \
                 and the size above is this window's rather than that one: the mix is \
                 composited at the largest enabled output and the only output here is the \
                 Program bay's picture, so the number moves with the window and with every \
                 divider (ADR-0247). This reading is comparable with the rest of this \
                 repository's figures exactly as far as it is that material at that size, and \
                 never with a headless one: this is a deck of four stepped and drawn \
                 slots with a panel over it, and a headless figure is one Set. Host \
                 clock, {}.",
                WINDOW.0, WINDOW.1, material, capacity, at.0, at.1, PROFILE
            );
            println!(
                "  and every figure above is taken on a core that spends the vsync wait \
                 asleep. On THIS machine that matters a great deal — the identical run with \
                 the other cores loaded reports about a sixth of these numbers, proportions \
                 unchanged — and it is this machine's power management rather than a rule: \
                 two Windows machines were asked the same way and got 1.3x and 1.6x WORSE \
                 under load, which is ordinary contention. Compare ratios, not magnitudes."
            );

            // -- what a whole frame cost -----------------------------
            // **The block the three medians above cannot be.** Everything
            // printed so far is CPU time that stops at a submission, and the
            // one field that is not — `wait` — is reported beside the frame
            // rather than in it. That reading holds exactly while the GPU is
            // not the bottleneck, and says nothing at all when it is: a panel
            // at four frames a second read off those three as a loop idle
            // 97.6% of the time, and the loop was not idle. See
            // `Cost::period`, `Cost::drained` and ADR-0303.
            //
            // **Filtered series, so they get their own lengths.** A frame has
            // no period until it has a predecessor and no drain unless it was
            // audited, so `n` above is not theirs and neither is its median.
            pub(crate) fn middle(xs: &[f64]) -> Option<f64> {
                let mut xs = xs.to_vec();
                xs.sort_by(f64::total_cmp);
                xs.get(xs.len() / 2).copied()
            }
            let mut periods: Vec<f64> = self
                .frames
                .iter()
                .filter_map(|c| c.period)
                .map(ms)
                .collect();
            let elsewhere: Vec<f64> = self
                .frames
                .iter()
                .filter_map(|c| c.elsewhere())
                .map(ms)
                .collect();
            let drained: Vec<f64> = self
                .frames
                .iter()
                .filter_map(|c| c.drained)
                .map(ms)
                .collect();
            // **What an audited frame's period was, against the rest.** This
            // is the instrument reporting its own price: a blocking poll is
            // the one thing here that could become the cost it is measuring,
            // and the two medians beside each other are the only honest way to
            // say it did not.
            let audited: Vec<f64> = self
                .frames
                .iter()
                .filter(|c| c.drained.is_some())
                .filter_map(|c| c.period)
                .map(ms)
                .collect();
            let rest: Vec<f64> = self
                .frames
                .iter()
                .filter(|c| c.drained.is_none())
                .filter_map(|c| c.period)
                .map(ms)
                .collect();
            periods.sort_by(f64::total_cmp);

            println!();
            println!(
                "what a WHOLE frame cost, with the wait inside it rather than beside it \
                 (ADR-0303):"
            );
            match periods.is_empty() {
                // One frame drawn and no second one, so there is no interval.
                // Said rather than divided: a period over no frames is not a
                // small number, it is not a number (P-0095).
                true => println!(
                    "  no frame had a predecessor to be an interval from, so this run \
                     measured no frame period at all."
                ),
                false => {
                    let m = periods.len();
                    println!(
                        "  frame period  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — top of \
                         one redraw to the top of the next, over {} of the {} sampled",
                        periods[m / 2],
                        periods[m * 95 / 100],
                        periods[m - 1],
                        m,
                        n
                    );
                    println!(
                        "    of which the three stretches above are {:.3} ms and the wait is \
                         {:.3} ms, leaving a median {:.3} ms this file times nowhere — the sinks \
                         aimed, the bay rearranged, the panel solved, `Queue::present`, and \
                         whatever `winit` does between two redraws. `Cost::whole` said the three \
                         *tile the frame exactly*; this is the measurement that says otherwise.",
                        engine[n / 2] + ui[n / 2] + paint[n / 2],
                        wait[n / 2],
                        middle(&elsewhere).unwrap_or(0.0)
                    );
                    println!(
                        "    and 1000/period is {:.1} frames a second against the {:.1} counted \
                         over the stretch above — two routes to one rate, taken by two clocks, \
                         which is the only check either of them gets.",
                        1000.0 / periods[m / 2],
                        rate
                    );
                    match refresh_ms {
                        // **What tells a vsync wait from a wait on the GPU**,
                        // and the only thing that can on a host clock: the
                        // wait itself is one field whichever it was.
                        Some(refresh) => println!(
                            "    against this display's {refresh:.1} ms refresh interval that is \
                             {:.2}x. At about 1x the wait is the display's pace and the loop has \
                             headroom; well past it the wait is the GPU and the three medians \
                             above are measuring a loop that is not idle at all.",
                            periods[m / 2] / f64::from(refresh)
                        ),
                        None => println!(
                            "    and with no refresh interval from `winit` there is nothing to \
                             hold it against, so this run cannot say whether the wait was the \
                             display's pace or the GPU."
                        ),
                    }
                }
            }
            match middle(&drained) {
                Some(owed) => println!(
                    "  the GPU still owed a median {owed:.3} ms when the CPU had finished the \
                     frame — `Device::poll` to a drained queue, over {} audited frames at one \
                     every {:.0} ms. Host clock and biased high: the poll's own round trip is in \
                     it, and so is anything of the previous frame still in flight. It is the \
                     whole submission — four slots stepped and drawn, the composite, five \
                     presents and the panel's pass — and not one of them.",
                    drained.len(),
                    ms(Costs::AUDIT)
                ),
                // Not a zero. A drain nobody measured has no duration, and
                // 0.0 ms here would read as a GPU with nothing to do.
                None => println!(
                    "  and what the GPU owed was not measured on this run: no frame was audited, \
                     so this reading has no GPU number in it and does not have one to give."
                ),
            }
            match (middle(&audited), middle(&rest)) {
                (Some(a), Some(b)) => println!(
                    "  an audited frame ran a median {a:.3} ms against {b:.3} ms for the rest, \
                     which is what this instrument costs on this machine — measured, because a \
                     blocking poll is the one thing here that could become the cost it is \
                     measuring."
                ),
                _ => println!(
                    "  and what the audit costs is not in this run: it takes both audited and \
                     unaudited frames in the sample to say."
                ),
            }
            println!(
                "  taken on a host clock, and this is why: {}",
                match self.clock {
                    Some(MeasurementMethod::GpuTimestamp) =>
                        "this adapter's timestamp queries survived the deck probe's calibration, \
                         so a *pass* can be timed on the GPU here — and a *frame* still cannot. \
                         Most of one is outside every command buffer: the block in \
                         `get_current_texture`, the `egui` pass, `Queue::present`. The period is \
                         a host figure on any adapter",
                    Some(MeasurementMethod::HostWallClock) =>
                        "this adapter's timestamp queries did not survive the deck probe's \
                         calibration — the feature is advertised, and a load that cannot take \
                         zero time resolved to zero anyway (P-0095, ADR-0169) — so there is no \
                         GPU clock here to have used. The period would be a host figure \
                         regardless: most of a frame is outside every command buffer",
                    // Not read off `Features`. A flag is what the platform
                    // says rather than what it does, which is the whole of
                    // P-0095.
                    None =>
                        "nothing probed this adapter on this run, so this program has no verdict \
                         on its timestamps and will not read one off `Features`",
                }
            );
            println!(
                "  what it cannot see: which pass inside the submission the drain belongs to; a \
                 wait on the display told apart from a wait on the GPU except by the ratio \
                 above; and any frame that was not audited, whose GPU cost is in no field here \
                 and arrives one frame later folded into the wait."
            );
            // **Which size each number is about**, which this application can
            // answer in exactly two ways and no third one: ADR-0247 composites
            // the mix once at the output's size and resizes that one render
            // into every rectangle, and a slot is auditioned in a deck cell
            // sized from the cell. A measurement taken at neither — the
            // 1280x720 constant ADR-0303 removed — is a number about a frame
            // nobody draws.
            println!(
                "  and the sizes these are about, of which this program has two: the frame is \
                 this whole window at {:.0}x{:.0} logical, with the mix composited once at \
                 {}x{} and that one render resized into every rectangle it is drawn in \
                 (ADR-0247); a slot's own measurement is taken at its preview cell, which is \
                 sized from the cell (ADR-0303). No figure here is taken at a third size. \
                 The mix's size is the largest enabled output's and is not a constant \
                 (ADR-0247): it is the Program bay's picture here, and it follows a divider \
                 drag and a projector window.",
                WINDOW.0, WINDOW.1, at.0, at.1
            );
        }
        println!();
        println!(
            "{}",
            match self.live {
                // **Two sinks, so two folds.** This said "fold the picture
                // away (f over it) and it does" while deck A's cell was
                // drawing in the row underneath, which is a sentence that
                // sends an operator to watch a window that is still drawing at
                // full rate — the same false claim, in the same place, that
                // cost this file 270 frames once already.
                true =>
                    "the loop asks for the next frame from inside the last one for as long \
                     as anything is making texels, so `ControlFlow::Wait` never gets to \
                     block. Two things are: the picture, and the four cells in the preview \
                     row under it — all four of them, because every slot is drawn whatever \
                     its residency. Fold the picture away (f over it) and the cells keep \
                     the loop awake on their own; fold the preview row away as well \
                     and nothing is making texels — and the window still draws, because \
                     the beat grid declares a deadline of its own for as long as it is on \
                     screen (P-0094, ADR-0212) and the mixer bay declares another while \
                     deck B is parked. `ControlFlow::Wait` blocks when all three are gone, \
                     which is a fourth fold, and it is what ADR-0164's remaining clauses \
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
pub(crate) fn no_gpu(what: &str) -> ! {
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

pub(crate) fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}
