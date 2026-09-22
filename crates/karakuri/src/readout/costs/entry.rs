use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::time::Duration;

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
