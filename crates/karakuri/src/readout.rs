//! Metric / cost accounting and Readout HUD / pointer event translation logic.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::time::{Duration, Instant};

use karakuri_console::budget::PANEL_PASS;
use karakuri_console::egui;
pub(crate) use karakuri_console::input::{claim, wheeled, Claim, Turned, CONTROLS};
pub(crate) use karakuri_console::panel::{
    Dragged, InHand, Knob, Op, Outcome, Panel, Pressed, Released,
};
pub(crate) use karakuri_console::room::Room;
pub(crate) use karakuri_console::view::{
    self, arrangement as arrangement_pill, audio_in as audio_in_pill, bay_grip, class_at,
    deck_head as deck_head_row, deck_name, inspector as inspector_pane, keep_pill,
    library as library_bay, look as look_row, master as master_row, mcp_pill, mixer as mixer_bay,
    outputs, program_bay, program_head, sequencer as sequencer_bay, staging as staging_bay,
    tracker_group, transition as transition_row, transport as transport_row, Aim, Ask, AudioAsk,
    AudioIn, Chose, Chosen, Go, Kind, McpPill, Picked, Read, Scope, Taken, View, Wiring, DECKS,
    DECK_LETTERS, REGIONS,
};
pub(crate) use karakuri_engine::governor::{Reason, Report};
use karakuri_engine::probe::MeasurementMethod;
use karakuri_environment::Opening;
use karakuri_layout::{Axis, NodeId, Point};
pub(crate) use karakuri_operation::gate::{Class, Open};
use karakuri_operation::{Operation, Output};

use crate::{
    demonstration_banks, ir_layer, node_addr, presets_listing, refusal, showing, ASKED_TO_PRIME,
    PROFILE, WINDOW,
};

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

// ---------------------------------------------------------------------------
// The readout: English for what the model returned. No window, no device.
// ---------------------------------------------------------------------------

/// The panel and the view, plus the words for what just happened.
pub(crate) struct Readout {
    pub(crate) panel: Panel,
    pub(crate) view: View,
    /// What the operator has opened to a model, and the one piece of state in this
    /// struct that is neither the panel's nor a reading of the engine.
    ///
    /// It is a handle rather than a value because the whole point of it is that a
    /// *second* reader has it: `karakuri_mcp::serve` takes a clone and reads it on
    /// every call, so an opening is live rather than a snapshot taken at startup.
    /// `View::opening` is this handle read once a frame; this is the model of
    /// record.
    ///
    /// This process serves MCP when `--mcp` names a port, and the server is handed
    /// this same handle rather than a copy, so the four pills and the audit read
    /// one value. Without the flag the pills still write it and only this program
    /// and its tests read it back.
    pub(crate) opening: Opening,
    /// What the last write did, which the transport row's health capsule draws —
    /// `view::Transport::health`, kept here because that value is rebuilt whole
    /// every frame by `transport` and a verdict arrives on one frame in a thousand.
    ///
    /// The one reading in this struct that is a stream rather than a state.
    /// Everything else the row draws is asked of the deck on the frame it is drawn
    /// on; a `swap::Event` exists once, in the drain `staging` makes, and is gone.
    /// So the last one is remembered here for the same reason `view.staging`'s rows
    /// are remembered in the view: it is the drain that forces it, not a
    /// preference.
    ///
    /// `None` until a build produces a verdict, which is most of most runs.
    pub(crate) health: Option<view::Stage>,
    /// The session's four sequencer banks, and the one piece of state in this
    /// struct that is neither the panel's nor a reading of the engine —
    /// `Readout::opening`'s category, over a pattern instead of over what a model
    /// may reach.
    ///
    /// A pattern is authored state and the engine holds none of it (ADR-0222: a
    /// lane is a fifth *route*, so what a lane does reaches the deck as
    /// `Operation::SetOpacity` like everything else). It is kept here because this
    /// window is what polls it and what the press arms edit; the console draws a
    /// copy handed to `View::sequencer` per frame and applies nothing to it
    /// (ADR-0156).
    ///
    /// Nothing saves or loads one yet. ADR-0227 settles where a pattern is kept and
    /// ADR-0320 leaves the file form to the record that has something to serialise,
    /// which is the row that saves one — so these four banks live for the run and
    /// no longer.
    pub(crate) sequencer: karakuri_pattern::Banks,
    /// Where the poll left the playhead, so a lane emits at a step boundary and
    /// never twice for one step.
    ///
    /// It is not in the pattern, and that is the shape rather than an accident: a
    /// pattern is a thing that gets saved and where the playhead has got to is not
    /// (`karakuri_pattern::Playhead`).
    pub(crate) playhead: karakuri_pattern::Playhead,
}

impl Readout {
    pub(crate) fn new(width: f32, height: f32) -> Readout {
        Readout {
            panel: Panel::new(width, height),
            view: View::new(Room::Day),
            // Four classes shut, which is what a run starts with (ADR-0235).
            opening: Opening::closed(),
            // Nothing has been written yet, so the capsule is not drawn.
            health: None,
            sequencer: demonstration_banks(),
            // **Nothing polled yet**, which draws no playhead column and makes
            // the first poll a boundary — a bay nobody has run is not a bay at
            // step zero.
            playhead: karakuri_pattern::Playhead::default(),
        }
    }

    // -- the words ------------------------------------------------------

    pub(crate) fn label(&self, id: NodeId) -> String {
        match self.panel.layout().name(id) {
            Some(name) => name.to_owned(),
            None => "(unnamed split)".to_owned(),
        }
    }

    /// The two regions a boundary is between. A split is often unnamed — the
    /// console's body row is, deliberately — so `split #0` alone does not say which
    /// boundary the pointer has hold of, and the pair does.
    pub(crate) fn pair(&self, split: NodeId, index: usize) -> String {
        match self.panel.pair(split, index) {
            Some((a, b)) => format!("{} | {}", self.label(a), self.label(b)),
            None => "no pair".to_owned(),
        }
    }

    // -- input ----------------------------------------------------------

    pub(crate) fn press(&mut self, p: Point) {
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

    /// A move with something in hand. A boundary drag says what it did here; a
    /// fader drag hands its operation back, because acting on one takes the deck
    /// and the deck is not the readout's.
    pub(crate) fn moved(&mut self, p: Point) -> Option<Operation> {
        match self.panel.moved(p)? {
            boundary @ Dragged::Boundary { .. } => {
                println!("{}", self.say_drag(boundary));
                None
            }
            // **A pane closed by pulling its boundary out through its own
            // edge, or brought back by pulling that edge in**
            // (`docs/adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md`).
            // `Panel` performed it, because the arrangement is its own, so
            // this says what happened and asks for nothing: a fold writes no
            // session record.
            Dragged::Pane(op) => {
                let (what, id) = match op {
                    Op::Fold(id) => ("folded — drag its edge back in", id),
                    Op::Unfold(id) => ("back, at its own minimum", id),
                    other => unreachable!("a pane drag asks for a fold, not {other:?}"),
                };
                println!("  drag: {} {what}", self.label(id));
                None
            }
            Dragged::Fader(operation) => Some(operation),
        }
    }

    /// A drag, in words: what was asked, where it landed, what held it, and what
    /// the pair either side is now.
    pub(crate) fn say_drag(&self, d: Dragged) -> String {
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

    /// The pointer went up, and what the gesture it ended asked for.
    ///
    /// `onto` is which deck's strip the pointer is over — the caller's answer,
    /// because a strip's geometry is `karakuri-console`'s view and not its model
    /// (`Panel::released`). Two of the three drags do not read it.
    ///
    /// It answers an `Acted` where it used to answer nothing, and the drop is why:
    /// a boundary coming to rest and a fader being let go both ask for nothing —
    /// everything either of them wanted was asked for while it was moving — and a
    /// carry asks for its whole operation here or nowhere.
    pub(crate) fn released(&mut self, onto: Option<u8>) -> Acted {
        match self.panel.released(onto) {
            Some(Released::Rests { split, index, at }) => {
                println!("release: {} rests at {at:.1}", self.pair(split, index));
                Acted::Nothing
            }
            Some(Released::Gone { split, index }) => {
                println!("release: {} is gone", self.pair(split, index));
                Acted::Nothing
            }
            // **No value in the line, because there is none to print.** Where
            // a fader came to rest is the deck's, and the last thing the drag
            // asked for was printed when it was asked for.
            Some(Released::Let { knob }) => {
                println!(
                    "release: {} lets go of the {}",
                    knob_where(&knob),
                    knob_word(&knob)
                );
                Acted::Nothing
            }
            // **A Set was let go over a strip**, and the load leaves by the
            // door every other control's operation leaves by — `played` is
            // what performs it and says what the deck did about it, exactly as
            // it does for a load from the keyboard. The line here is the
            // *gesture* ending: two ways
            // in, one name, and the same sentences after the naming.
            Some(Released::Dropped(operation)) => {
                if let Operation::LoadSet { deck, set } = &operation {
                    println!(
                        "release: `{set}` was let go over deck {}",
                        deck_letter(*deck)
                    );
                }
                Acted::Emitted(Some(operation))
            }
            // **A carry let go over nothing asks for nothing**, and it says so
            // rather than saying nothing: a row picked up, carried and then
            // silently forgotten reads as a panel that missed the press. The
            // cursor is left on the row that was taken, which is where
            // `enter` in the library would load from next.
            Some(Released::Nowhere { set }) => {
                println!(
                    "release: `{set}` was let go over nothing, so nothing was loaded — a drop \
                     names its deck by landing on that deck's strip, or on its preview cell in \
                     the Program bay"
                );
                Acted::Nothing
            }
            None => Acted::Nothing,
        }
    }

    /// Act, say what happened, and hand the outcome back — the repaint decision is
    /// taken from what the operation did, not from the key that asked for it. `p`
    /// over an empty panel and `z` with nothing folded both reach the model and
    /// move nothing.
    pub(crate) fn op(&mut self, op: Op) -> Outcome {
        let outcome = self.panel.op(op);
        // **A reset puts the default arrangement on screen and the default has
        // no name**, so the pill stops naming the file it was showing. Here
        // rather than at either control, because `r` and the menu's *start a
        // new one* are one operation and this is the one place both arrive —
        // and it keys off the outcome rather than off the `Op`, so an op that
        // asked for a reset and did not get one leaves the name alone.
        if matches!(outcome, Outcome::Reset) {
            self.view.arrangement.name = None;
        }
        self.say_op(op, &outcome);
        outcome
    }

    /// What an operation did, in words. The model returns the facts; which English
    /// they take is the operation that was asked for, which is why this has both.
    pub(crate) fn say_op(&self, op: Op, outcome: &Outcome) {
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
            // **Nothing that goes through here can produce this.** A restore
            // is not an `Op` — it carries a whole arrangement, which only
            // whoever read the store can hand over — so `arrangement` says its
            // own sentence, where the name is, and this arm exists because the
            // match is exhaustive rather than because a line is owed. See
            // `Panel::restore`.
            Outcome::Restored => {}
            // **And nothing that goes through here can produce this one
            // either, since 2026-08-31.** `Op::Report` was `p`, `p` is the
            // latency offset the operations page specifies, and a panel
            // diagnostic with no useful shortcut to point at loses the letter
            // rather than keeping one of a specified pair. Nothing else in
            // this program names the operation, so no key, no control and no
            // pointer route can reach it and there is no sentence to say.
            //
            // **The words went with the route rather than being kept for
            // one.** A formatter for an outcome nothing produces is this file
            // claiming a route it has not got
            // ([`docs/contributing.md` §4](../../../docs/contributing.md)),
            // and the table it printed is not the one the startup legend
            // prints: that one is each region's *min and max*, once, before
            // anything has been dragged, and this was each region's solved
            // rectangle and whether it is folded, at any moment. The
            // operation still answers that, to `karakuri-console`'s own
            // tests; what is gone is this program asking.
            Outcome::Report(_) => {}
            // One operation can still find nothing to act on, and it is not
            // about the pointer: the root has no split enclosing it. *Nothing
            // under the pointer* is said by `key`, before an operation is
            // named at all — see `Panel::under`.
            Outcome::Nothing => {
                println!("fold: that is the root, and nothing encloses it")
            }
        }
    }

    pub(crate) fn room(&mut self) {
        self.view.room = self.view.room.other();
        println!("room: {}", self.view.room.word());
    }

    /// The one place a pointer event is routed, and the only place this file
    /// decides anything about input.
    ///
    /// Returns who the event belonged to. The caller's whole job with the answer is
    /// to hand the event to `egui` when it is [`Claim::Egui`] and not when it is
    /// not — see `karakuri_console::input` for the rule and for why it is written
    /// there rather than here.
    ///
    /// It is a method rather than four arms in `window_event` so that a gesture can
    /// be driven without a window: `winit` cannot be asked for an `ActiveEventLoop`
    /// outside its own loop, so an event handler is not something a test can call,
    /// and the part worth testing is this.
    pub(crate) fn pointer(&mut self, ctx: &egui::Context, event: Pointer) -> (Claim, Acted) {
        let at = match event {
            Pointer::Moved(p) => p,
            _ => self.panel.cursor(),
        };
        // Asked **before** anything acts. `released` takes the drag out of
        // hand, so a claim asked after it would see no drag, route the release
        // to `egui`, and hand `egui` a button-up it never saw the button-down
        // for.
        // **`mut` for the wheel arm alone.** Every other event's answer is
        // this call's; a wheel asks a second question of `input::wheeled`,
        // whose `Some` widens the claim to the panel — see that arm.
        let mut claim = claim(&mut self.panel, ctx, &self.view, at);
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
            // **A press the panel claimed is on one of the console's
            // controls or on the panel itself**, and they are asked first for
            // the reason `claim` asked them last: rule 2 has already had its
            // refusal, so a press that got here and is on a control is that
            // control's. Every one of them is the same call `claim` made —
            // asked again, not copied. How many there are is
            // `karakuri_console::input::CONTROLS`, which is why this sentence
            // no longer says a number: it went stale four times.
            //
            // **The bay is derived once and asked five times**, exactly as
            // `claim` does it: a knob, a blend chip, a tally chip, a mask mini
            // and the strip they sit in are five questions about one laid-out
            // strip, and five derivations would be five answers.
            (Pointer::Down, Claim::Panel) => {
                self.panel.solve();
                // **A row's menu is asked before every other control**, and
                // that ordering is the rule rather than a convenience: it is
                // the only card whose *pill* is not a capsule of its own, so
                // there is no press that both opens it and belongs to
                // something else, and while it is down `input::claim`'s rule 2
                // has already given every press on the console to the panel.
                // Asked after the two pills below, a press on one of *their*
                // capsules would open that card instead of dismissing this one
                // — a second card down while the first still was, which is the
                // one thing rule 2 is written to make impossible (ADR-0311).
                //
                // **Only while it is down.** A primary press never opens this
                // menu — that is the secondary button's, in the arm at the
                // bottom of this match — so with no card down this block does
                // not run at all and every control below goes on meaning what
                // it means, the Library bay's own rows included.
                if self.view.menu_open() {
                    let picked = library_bay(
                        self.panel.layout(),
                        &self.view.scopes,
                        &self.view.library,
                        self.view.opened(),
                        self.view.pointed(),
                        self.view.library_scroll(),
                    )
                    .and_then(|bay| {
                        bay.menu_ask(
                            ctx,
                            view::to_egui(self.panel.layout().viewport()),
                            self.view.menued(),
                            // **The rows, for the load button's reason two
                            // controls along**: a `history` row is a version
                            // and this menu's items name none, and which of
                            // the two loads an item asks for is what the row
                            // *is* — a Set or a procedure (ADR-0338).
                            self.view.rows(),
                            at,
                        )
                    });
                    did = self.menued(picked.unwrap_or(Picked::Shut));
                    return (claim, did);
                }
                // **The audio-in pill first of the rest**, and it and the arrangement pill
                // are the only two whose order matters: each
                // draws a card *over* the bays, so while one is down a press
                // inside it belongs to the card and not to whatever it is
                // covering. They are asked in the order they are drawn, which
                // is also the order they are laid out in — the arrangement
                // pill's place is measured from this one's right edge.
                //
                // **Only one card can be down**, so the two blocks cannot both
                // claim a press: `input::claim`'s rule 2 gives the press to
                // the panel while either is open, and a press outside the open
                // card is that card's dismissal.
                let listing = audio_in_pill(
                    ctx,
                    self.panel.layout(),
                    self.view.transport,
                    self.view.audio.as_ref(),
                );
                let heard = listing
                    .as_ref()
                    .zip(self.view.audio.as_ref())
                    .and_then(|(pill, audio)| pill.ask(audio, at));
                if self.view.audio.as_ref().is_some_and(AudioIn::open) {
                    did = self.listened(heard.unwrap_or(AudioAsk::Shut));
                    return (claim, did);
                }
                if let Some(ask) = heard {
                    did = self.listened(ask);
                    return (claim, did);
                }
                // **The arrangement pill next, for the same reason.** Its menu is drawn *over* the
                // bays, so while it is down a press inside the card belongs to
                // the card and not to whatever it happens to be covering — and
                // a press anywhere else is the dismissal, which is why the
                // `None` below is `Ask::Shut` rather than a press that fell
                // through. Shut, this is one capsule among many that never
                // overlap and the order is arbitrary.
                let pill = arrangement_pill(
                    ctx,
                    self.panel.layout(),
                    self.view.transport,
                    self.view.audio.as_ref(),
                    self.view.tracker,
                    self.view.map.as_ref(),
                    &self.view.arrangement,
                );
                let asked = pill
                    .as_ref()
                    .and_then(|pill| pill.ask(&self.view.arrangement, at));
                if self.view.arrangement.open() {
                    did = self.arranged(asked.unwrap_or(Ask::Shut));
                    return (claim, did);
                }
                if let Some(ask) = asked {
                    did = self.arranged(ask);
                    return (claim, did);
                }
                // **A `uses` line's card, which is a fifth**, asked here for
                // the four above it and its own: it hangs out of a line inside
                // an Inspector pane and down over the groups under it, so while
                // it is down a press anywhere on the console belongs to it
                // (`input::claim`'s rule 2, `docs/adr/0329-…`), and a press
                // outside it is the dismissal.
                //
                // **The pick is asked before the capsule.** A press inside the
                // card belongs to the card, and the capsule the card came out
                // of is under it: asking the capsule first would make a press
                // on the row that happens to overlap it re-open the list it was
                // picking from.
                //
                // **Derived inline, like the load control below it**, because
                // the press handler is where a control's derivation and its ask
                // are held together — `press_handler::ASKED` reads this body and
                // a call in a helper is a control this window is claiming and
                // then declining, which is the seam that module exists for.
                let room = view::to_egui(self.panel.layout().viewport());
                let picked = self.view.wiring_open().and_then(|(pane_at, node, input)| {
                    let pane = self.view.inspector.get(pane_at)?;
                    let laid = inspector_pane(
                        self.panel.layout(),
                        pane_at,
                        pane,
                        self.view.scroll_in(pane_at),
                    )?;
                    laid.wired(ctx, pane, room, (node, input), at)
                        .map(Wiring::Pick)
                });
                let wiring = picked.or_else(|| {
                    self.view
                        .inspector
                        .iter()
                        .enumerate()
                        .find_map(|(index, pane)| {
                            let laid = inspector_pane(
                                self.panel.layout(),
                                index,
                                pane,
                                self.view.scroll_in(index),
                            )?;
                            let (node, input) = laid.uses_chip(ctx, pane, at)?;
                            Some(Wiring::Chip {
                                pane: index,
                                node,
                                input,
                            })
                        })
                });
                if self.view.wiring_open().is_some() {
                    did = self.wired(wiring.unwrap_or(Wiring::Shut));
                    return (claim, did);
                }
                if let Some(ask) = wiring {
                    did = self.wired(ask);
                    return (claim, did);
                }
                // **A pane head's deck list is a card too**, and it is asked
                // here for the `uses` card's reason one block up: it hangs out
                // of a head at the top of a pane and down over that pane's own
                // groups, so while it is down a press anywhere on the console
                // belongs to it (`input::claim`'s rule 2) and a press outside
                // it is the dismissal.
                //
                // **The pick is asked before the mark**, which is that block's
                // ordering and its reason: a press inside the card belongs to
                // the card, and the mark the card came out of is above it.
                let picked = self.view.pane_target_open().and_then(|pane_at| {
                    let pane = self.view.inspector.get(pane_at)?;
                    let laid = inspector_pane(
                        self.panel.layout(),
                        pane_at,
                        pane,
                        self.view.scroll_in(pane_at),
                    )?;
                    self.view
                        .pane_pulldown(ctx, &laid, pane, pane_at)?
                        .picked(room, at)
                        .map(view::Pointing::Pick)
                });
                let pointing = picked.or_else(|| {
                    self.view
                        .inspector
                        .iter()
                        .enumerate()
                        .find_map(|(index, pane)| {
                            let laid = inspector_pane(
                                self.panel.layout(),
                                index,
                                pane,
                                self.view.scroll_in(index),
                            )?;
                            let target = self.view.pane_pulldown(ctx, &laid, pane, index)?;
                            target.hit(at).then_some(view::Pointing::Mark(index))
                        })
                });
                if self.view.pane_target_open().is_some() {
                    did = self.pointing(pointing.unwrap_or(view::Pointing::Shut));
                    return (claim, did);
                }
                if let Some(ask) = pointing {
                    did = self.pointing(ask);
                    return (claim, did);
                }
                // **The Library bay's load control, and its list is a third
                // card**, so it is asked here rather than beside the bay's own
                // rows below: the card hangs up out of that bay's foot and
                // over its list, and while it is down a press anywhere on the
                // console belongs to it (`input::claim`'s rule 2, ADR-0305).
                // A press outside it is the dismissal, which is why the `None`
                // below is `Aim::Shut` — the same shape the two pills above
                // are in, and for their reason.
                //
                // **The four cards can never be down together**: the press
                // that would open a second one lands while the first is open,
                // so whichever is open claims it and that press shuts it.
                //
                // **Both operands go in with the point.** The deck is the
                // pulldown's (`View::target`) and never the deck selection —
                // that is the whole of the record — and the Set is the row
                // under the cursor, which this side read out of the store
                // (ADR-0156).
                let aimed = library_bay(
                    self.panel.layout(),
                    &self.view.scopes,
                    &self.view.library,
                    self.view.opened(),
                    self.view.pointed(),
                    self.view.library_scroll(),
                )
                .and_then(|bay| {
                    bay.aim(
                        ctx,
                        view::to_egui(self.panel.layout().viewport()),
                        self.view.target(),
                        // **The rows and the cursor, which is empty under
                        // `history`**: this button loads what the cursor is
                        // on and that scope's rows are versions, so it
                        // answers `Aim::NoSet` there rather than naming a
                        // load of a word no store holds
                        // (`view::View::rows`). Which of the two loads it
                        // names is the row's own kind (ADR-0338).
                        self.view.rows(),
                        self.view.cursor_row(),
                        at,
                    )
                });
                if self.view.target_open() {
                    did = self.aimed(aimed.unwrap_or(Aim::Shut));
                    return (claim, did);
                }
                if let Some(ask) = aimed {
                    did = self.aimed(ask);
                    return (claim, did);
                }
                // **The tracker group's three, derived once for all of
                // them** — the offset's figure is as wide as the number in it
                // and the octave is laid out from where the tap ends, so they
                // are three questions about one laid-out group, exactly as the
                // look's two are about theirs. Nothing here can overlap either
                // card: both are asked above, and each takes every press on
                // the console while it is down.
                //
                // **The three are asked in the order they sit in the row**,
                // and no two of them can answer for one point:
                // `TrackerGroup::owns` is the union of exactly these three.
                let group = tracker_group(
                    ctx,
                    self.panel.layout(),
                    self.view.transport,
                    self.view.audio.as_ref(),
                    self.view.tracker,
                );
                let tracked = group.as_ref().and_then(|group| {
                    group
                        .tapped(at)
                        .or_else(|| group.octave(at))
                        .or_else(|| group.nudge(at))
                });
                if let Some(operation) = tracked {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The two look controls, derived once for both** — the
                // exposure track's place is measured from the tone map
                // capsule's, so they are two questions about one laid-out
                // group, exactly as the mixer's four are about one strip.
                // Neither can overlap the pill: this group starts one
                // `.transport` gap after it.
                let look = look_row(
                    ctx,
                    self.panel.layout(),
                    self.view.transport,
                    self.view.audio.as_ref(),
                    self.view.tracker,
                    self.view.map.as_ref(),
                    &self.view.arrangement,
                    self.view.look,
                );
                let tone = look.as_ref().and_then(|row| row.tonemap(at));
                let exposure = look.as_ref().and_then(|row| row.exposure(at));
                if let Some(operation) = tone.or(exposure) {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The `learn` pill, and a press on it is not an
                // operation.**
                //
                // It is the same kind of control as the four `mcp` pills and
                // for a stronger version of their reason —
                // [ADR-0236](../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md),
                // and
                // [ADR-0336](../../../docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md),
                // which carries the argument. **A learn edits the map**: the
                // layer every surface reaches the vocabulary through, rather
                // than a member of the vocabulary the map addresses. Three
                // things make it not an operation, and the last one is
                // mechanical:
                //
                // - **Its operand is the pointer.** Which control a knob binds
                //   to is *what the pointer is on*, and a model has no window
                //   (ADR-0315) while a map line has no pointer — so an
                //   operation for it would be `gap` in three of the page's
                //   four columns, which is a gesture rather than an operation.
                // - **A permission an actor can grant itself is not a
                //   permission**, which is the `mcp` pills' own sentence: a
                //   learn reachable over MCP would let a model rewire the
                //   operator's hands.
                // - **It must not reach the session stream.** A session
                //   recorded from a controller replays with neither controller
                //   nor map attached
                //   (`docs/principles/0092-the-same-inputs-produce-the-same-frame.md`),
                //   because which knob is which is a property of the room's
                //   hardware. An operation writes a record; a record of a
                //   learn would put the room's wiring in the timeline and a
                //   replay would re-learn against whatever map was there.
                //
                // So: no `Operation`, no `Record`, and `Acted::Opened` — the
                // type that will not let this be quietly fixed into the
                // vocabulary.
                if let Some(pill) = view::learn_pill(
                    ctx,
                    self.panel.layout(),
                    self.view.transport,
                    self.view.audio.as_ref(),
                    self.view.tracker,
                    self.view.map.as_ref(),
                    self.view.learn,
                ) {
                    if pill.hit(at) {
                        self.view.learn = pill.next();
                        println!(
                            "{}",
                            match self.view.learn {
                                true =>
                                    "learn: armed. point at a control and move a knob or hit a                                      pad, and the two are bound — the line goes in your own map                                      file. nothing is played from the surface while this is                                      lit, and it stays lit until you press it again.",
                                false => "learn: off. the surface plays again.",
                            }
                        );
                        return (claim, Acted::Opened);
                    }
                }
                // **The `map` pill is a readout, and the press is swallowed
                // here rather than left to fall through.** `input::claim`
                // already keeps it off `egui`, so this changes nothing an
                // operator can see — what it buys is that *this control asks
                // for nothing* is a line of code rather than an absence, and
                // that `ASKED` can name a derivation for it instead of
                // carrying an entry that passes vacuously.
                //
                // It asks for nothing because reaching a different map while
                // running is not built, and a capsule that opened a menu with
                // nothing in it would be the scaffolding `view::transport`
                // refuses.
                if view::map_pill(
                    ctx,
                    self.panel.layout(),
                    self.view.transport,
                    self.view.audio.as_ref(),
                    self.view.tracker,
                    self.view.map.as_ref(),
                )
                .is_some_and(|row| row.pill.contains(egui::Pos2::new(at.x, at.y)))
                {
                    return (claim, Acted::Nothing);
                }
                //
                // **The `rec` pill at the end of that row**, and the row is
                // the whole derivation: the pill takes the row's right padding
                // and the health capsule and the frame readout are laid out
                // backwards from it, so where it is is `transport`'s answer
                // rather than a second one. Nothing else in the row overlaps
                // it — the look group ends one `.transport` gap before the
                // frame readout, which ends one before the capsule before
                // this.
                //
                // **What the press asks for is the pill's own state**, which
                // is why nothing here decides which end of the toggle it is:
                // `TransportRow::record` reads the value the pill was drawn
                // from, so the capsule an operator is looking at and the
                // operation the press names cannot come apart.
                let row = transport_row(ctx, self.panel.layout(), self.view.transport);
                let recording = row.as_ref().and_then(|row| row.record(at));
                if let Some(operation) = recording {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The tempo figure at the head of the same row**, asked
                // through the one derivation for the reason the pill is: the
                // figure is the row's first item and the control is the
                // reading itself. A press names a tempo outright — where along
                // the number it landed is the value — and a press on the guard
                // either side of the band asks for nothing and falls through
                // (ADR-0291).
                let tempo = row.as_ref().and_then(|row| row.tempo(at));
                if let Some(operation) = tempo {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The deck head's six, one pane at a time.** Each pane is
                // derived once and asked for all six, exactly as the mixer
                // bay is asked for its four: the anchor's place is measured
                // from the mode chip's and the arrows' from the anchor's, and
                // the three at the right are measured leftwards from the fold,
                // so they are six questions about one laid-out pane. Nothing
                // else on the panel overlaps a pane — the Inspector's card is
                // its own bay — so the order against the mixer below is
                // arbitrary.
                //
                // **The six are asked in the order they sit in the row**,
                // and no two of them can answer for one point:
                // `DeckHead::owns` is the union of exactly these six, and
                // `tests/deck_head.rs` asserts a press is one of them or none.
                //
                // **The last three are not mix controls**: each one asks for a
                // different field of what this slot's watcher is pointed at —
                // the layering, the capacity its geometries run at, and the
                // salt its randomness comes from — and `composited`, `resized`
                // and `re_salted` below are what turn those into a re-aim.
                // They are asked here with the other three because it is the
                // same laid-out row and the same derivation, not because they
                // go to the same place (ADR-0314, ADR-0328).
                let deck_head = self
                    .view
                    .inspector
                    .iter()
                    .enumerate()
                    .find_map(|(index, pane)| {
                        let at_pane = inspector_pane(
                            self.panel.layout(),
                            index,
                            pane,
                            self.view.scroll_in(index),
                        )?;
                        let head = deck_head_row(ctx, &at_pane, pane)?;
                        head.sync(at)
                            .or_else(|| head.reanchor(at))
                            .or_else(|| head.scrub(at))
                            .or_else(|| head.resized(at))
                            .or_else(|| head.re_salted(at))
                            .or_else(|| head.compositing(at))
                    });
                if let Some(operation) = deck_head {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The Inspector pane heads' name, one pane at a time**, and
                // it is the capsule's arrangement at the other end of the same
                // row: a press puts that head into a naming state and the
                // letters go into it until return or escape (ADR-0292).
                // **Nothing is emitted here** — the operation is the commit's,
                // and the commit is a key.
                let naming = self
                    .view
                    .inspector
                    .iter()
                    .enumerate()
                    .find_map(|(index, pane)| {
                        let at_pane = inspector_pane(
                            self.panel.layout(),
                            index,
                            pane,
                            self.view.scroll_in(index),
                        )?;
                        let named = deck_name(ctx, &at_pane, pane, self.view.naming_set_in(index))?;
                        named.hit(at).then_some(index)
                    });
                if let Some(index) = naming {
                    println!(
                        "inspector: type a name and press return — letters, digits, `-` and \
                         `_`, and escape keeps nothing"
                    );
                    self.view.name_set(index);
                    return (claim, Acted::Nothing);
                }
                // **The Inspector pane heads' `keep`, one pane at a time.**
                // It keeps the deck the pane is *showing* rather than the deck
                // the selection is on, which is what `k` keeps: a bare key
                // press cannot say which deck and a capsule drawn inside a
                // pane can (ADR-0287). The capsule is derived from the
                // laid-out pane, exactly as the deck head above it is, and the
                // write itself is at the call site because a disk write is not
                // a thing to do on a frame.
                let keep = self
                    .view
                    .inspector
                    .iter()
                    .enumerate()
                    .find_map(|(index, pane)| {
                        let at_pane = inspector_pane(
                            self.panel.layout(),
                            index,
                            pane,
                            self.view.scroll_in(index),
                        )?;
                        let pill = keep_pill(ctx, &at_pane, pane)?;
                        pill.keep(at)
                    });
                if let Some(operation) = keep {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The renderer chips, one pane at a time**, and the pane is
                // the whole derivation: which group and which chip are inside
                // `InspectorPane::select_renderer`, which is the `params` chip's
                // arrangement in the Library bay. A press on an overdrawn
                // deck's chips, or on the one chip of a Set with one renderer,
                // answers `None` — drawn and not claimed.
                let chosen = self
                    .view
                    .inspector
                    .iter()
                    .enumerate()
                    .find_map(|(index, pane)| {
                        let at_pane = inspector_pane(
                            self.panel.layout(),
                            index,
                            pane,
                            self.view.scroll_in(index),
                        )?;
                        at_pane.select_renderer(ctx, pane, at)
                    });
                if let Some(operation) = chosen {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The `man / sug / auto` chips on a node head**, and the
                // pane is the whole derivation for the renderer chips' reason:
                // which group and which chip are inside
                // `InspectorPane::set_authority`. A head standing over more
                // than one node draws no chip and answers `None` — drawn and
                // not claimed, which is the renderer row's arrangement one row
                // down.
                let spoken = self
                    .view
                    .inspector
                    .iter()
                    .enumerate()
                    .find_map(|(index, pane)| {
                        let at_pane = inspector_pane(
                            self.panel.layout(),
                            index,
                            pane,
                            self.view.scroll_in(index),
                        )?;
                        at_pane.set_authority(ctx, pane, at)
                    });
                if let Some(operation) = spoken {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The `keep` capsule at the right of the same head**, and
                // the pane is the whole derivation for the authority chips'
                // reason: which group and where the capsule is are inside
                // `InspectorPane::keep_procedure`. **Two heads carry none and
                // answer `None`** — a head standing over several nodes, and
                // the built-in camera, which is a node with no procedure
                // behind it — so both are drawn without a capsule rather than
                // drawn with one that refuses (ADR-0338, decision 4).
                let kept = self
                    .view
                    .inspector
                    .iter()
                    .enumerate()
                    .find_map(|(index, pane)| {
                        let at_pane = inspector_pane(
                            self.panel.layout(),
                            index,
                            pane,
                            self.view.scroll_in(index),
                        )?;
                        at_pane.keep_procedure(ctx, pane, at)
                    });
                if let Some(operation) = kept {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The sensitivity row's chips under a bound parameter**, and
                // two of its four are controls: the curve chip re-attaches the
                // same signal through the next shape, and `take back` removes
                // the attachment. The source and the range answer `None` —
                // drawn and claimed by nothing, for the reason
                // `view::SensChip` carries.
                let sensed = self
                    .view
                    .inspector
                    .iter()
                    .enumerate()
                    .find_map(|(index, pane)| {
                        let at_pane = inspector_pane(
                            self.panel.layout(),
                            index,
                            pane,
                            self.view.scroll_in(index),
                        )?;
                        at_pane.sensitivity(ctx, pane, at)
                    });
                if let Some(operation) = sensed {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **A parameter row's publish mark, one pane at a time**, and
                // it is the leftmost cell of the row the fader is on: the
                // number where the control is on the deck's interface and a dot
                // where it is not. It is asked *before* the fader below because
                // the two are on one row and never overlap — the mark is a
                // fixed track of the mock's grid and the fader begins two
                // tracks along — so the order is arbitrary in fact and this one
                // reads down the row.
                //
                // **What it asks for is the whole interface**, not this entry:
                // a knob is learned against a position, so an operation that
                // said *drop this one* would leave two surfaces disagreeing
                // about what the positions are (`view::InspectorPane::publishing`,
                // `docs/adr/0329-…`).
                let published = self
                    .view
                    .inspector
                    .iter()
                    .enumerate()
                    .find_map(|(index, pane)| {
                        let at_pane = inspector_pane(
                            self.panel.layout(),
                            index,
                            pane,
                            self.view.scroll_in(index),
                        )?;
                        at_pane.publishing(pane, at)
                    });
                if let Some(operation) = published {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The Program bay head's `solo`**, and it is the one
                // control on this panel that acts on the console's own shape
                // from inside a bay rather than from the Outputs row. What it
                // asks for is `ProgramHead::op` — the same two operations
                // `s` and `u` perform, chosen from the layout rather than
                // toggled — and this file performs it exactly as it performs
                // the dot's.
                if let Some(head) = program_head(ctx, self.panel.layout(), self.view.opening)
                    .filter(|head| head.hit(at))
                {
                    return (claim, Acted::Operated(self.soloed(head.op())));
                }
                // **The grip in a bay head**, and it is the `solo` capsule's
                // neighbour in the same head: one derivation per bay, asked
                // whether the point is on the mark, and `FoldGrip::op` for what
                // a press folds. A fold is the console's own shape and writes
                // no record, so this leaves by the door the Outputs dot's fold
                // leaves by rather than through `written`.
                //
                // **Asked before the Library bay's rows**, so a capsule in a
                // head is asked before the list under it — and a pane needs no
                // arm at all, because it folds by its own boundary and rule 3
                // claims that before any control is asked (ADR-0300).
                if let Some(grip) = REGIONS.iter().find_map(|region| {
                    bay_grip(self.panel.layout(), region.name).filter(|grip| grip.hit(at))
                }) {
                    return (claim, Acted::Operated(self.folded(grip.op())));
                }
                // **The four class pills**, and this is the one press in this
                // file that leaves by neither of the other two doors. See
                // `Readout::opened`, which is where the reason is.
                if let Some(pill) = Class::ALL.iter().find_map(|class| {
                    mcp_pill(ctx, self.panel.layout(), *class, self.view.opening)
                        .filter(|pill| pill.hit(at))
                }) {
                    return (claim, self.opened(&pill));
                }
                // **The Library bay's scope chips**, which are the first
                // controls on this panel whose number is a value rather than a
                // constant: one per scope the bay was handed. The bay is
                // derived once and walked once, exactly as `claim` walks it —
                // a chip is as wide as the word in it, so where the fourth one
                // is depends on the first three and a second walk would put
                // the capsule a press lands on somewhere the wash is not.
                //
                // **A press names the chip; it does not step.** `space` on
                // the addressed chip steps, and
                // wraps because a bare press cannot say *which*, and this one
                // can — P-0090's division met by two surfaces rather than an
                // inconsistency between them. What comes back is `Chosen`: the
                // chip, and `Operation::SelectScope` beside it, because that
                // operation's payload is `Undecided` and cannot carry a chip.
                if let Some(chosen) = library_bay(
                    self.panel.layout(),
                    &self.view.scopes,
                    &self.view.library,
                    self.view.opened(),
                    self.view.pointed(),
                    self.view.library_scroll(),
                )
                .and_then(|bay| bay.chip(ctx, &self.view.scopes, at))
                {
                    return (claim, self.chose(chosen));
                }
                // **The Library bay's two filter fields**, one row under the
                // chips and derived from the same call for the same reason:
                // where the second field is depends on how wide the row is, and
                // a second derivation would put the box a press lands on
                // somewhere the border is not.
                //
                // **A press steps the field; it does not name a value.** A chip
                // is one of a row and a pointer lands on exactly one, so it
                // names; a field is one box standing for a list, so a press on
                // it moves along that list and the operation names where it
                // arrived — `LibraryBay::filter`, and `TransitionRow::shape`'s
                // affordance three bays along. What comes back is a whole
                // `Operation::ListSets`: unlike `SelectScope` this payload can
                // carry everything the press decided, so there is no `Chosen`
                // here and nothing beside the operation.
                if let Some(operation) = library_bay(
                    self.panel.layout(),
                    &self.view.scopes,
                    &self.view.library,
                    self.view.opened(),
                    self.view.pointed(),
                    self.view.library_scroll(),
                )
                .and_then(|bay| bay.filter(&self.view.holds, self.view.filters(), at))
                {
                    return (claim, self.narrowed(operation));
                }
                // **The six kind chips, one row under the field**, derived from
                // the same call for the row above's reason and asked after it
                // because the row they are in is drawn only where that one is.
                //
                // **A press names all six.** A chip flips its own field and
                // what leaves carries the whole row — `Operation::FilterLibrary
                // { kinds }` — because six statements each saying *this one
                // changed* are six things a second surface can arrive in the
                // middle of, and one saying *these are the kinds showing* is a
                // destination (ADR-0338). So it lands in `Readout::narrowed`
                // beside the field's own press: a scope, a filter and a kind
                // are one question asked of different halves.
                if let Some(operation) = library_bay(
                    self.panel.layout(),
                    &self.view.scopes,
                    &self.view.library,
                    self.view.opened(),
                    self.view.pointed(),
                    self.view.library_scroll(),
                )
                .and_then(|bay| bay.kind(ctx, self.view.filters(), at))
                {
                    return (claim, self.narrowed(operation));
                }
                // **The `params` chip in the Library bay's foot**, and the bay
                // is derived a third time for the reason `claim` derives it a
                // third time: each of these is a question about one laid-out
                // bay and a value held across all three would outlive the
                // question it answers.
                //
                // **The Set under the cursor goes in with the point**, because
                // the operand of a reading is the cursor — the same operand
                // the `load` button beside it reads, which is what
                // `console.html`'s note means by *"the route costs one chip in
                // the foot and nothing else"*.
                if let Some(ask) = library_bay(
                    self.panel.layout(),
                    &self.view.scopes,
                    &self.view.library,
                    self.view.opened(),
                    self.view.pointed(),
                    self.view.library_scroll(),
                )
                .and_then(|bay| {
                    bay.read(
                        ctx,
                        self.view.target(),
                        // **The Sets, for the load button's reason one
                        // control along**: `Operation::ReadSet` names a Set
                        // this store holds, and a `history` row is a version.
                        self.view
                            .sets()
                            .get(self.view.cursor_row())
                            .map(String::as_str),
                        at,
                    )
                }) {
                    return (claim, self.asked_to_read(ask));
                }
                // **The star at the left of a row**, asked before the row it
                // is in: a star is inside a row, so the order here is what
                // makes a press on the mark reach the mark — `input::claim`'s
                // rule 4, *a control claims what it acts on and no more*.
                //
                // **The bay is derived a fourth time**, for the reason it is
                // derived a third: each of these is a question about one
                // laid-out bay, and a value held across all of them would
                // outlive the question it answers.
                //
                // **The listing and the marks go in with the point**, exactly
                // as the listing does for the `params` chip: a star names a Set
                // this program read out of the store, and which rows are
                // already starred is this side's answer too (ADR-0156). The
                // write itself is not here — it is a disk write, which is the
                // window's, on the branch every other press that reaches a
                // disk takes.
                if let Some(operation) = library_bay(
                    self.panel.layout(),
                    &self.view.scopes,
                    &self.view.library,
                    self.view.opened(),
                    self.view.pointed(),
                    self.view.library_scroll(),
                )
                // **The Sets, for the `params` chip's reason one control up**: a star is
                // a control over a Set this store holds, and a `history` row is
                // not one.
                .and_then(|bay| bay.starred(self.view.rows(), &self.view.starred, at))
                {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **A row of the Library bay's list, and this is the one press
                // on this panel that asks for nothing at all.** What it does
                // is take a Set in hand: `console.html`'s *How a Set reaches a
                // deck* has the panel's route to that row as a drag —
                // *"Dragging a row onto a strip … names both operands in the
                // one gesture"* — and a press names one of the two. The
                // operation is built where the second one is, which is the
                // release, over whatever strip the pointer is then on.
                //
                // **The bay is derived a fifth time**, for the reason it is
                // derived a third: each of these is a question about one
                // laid-out bay, and a value held across all of them would
                // outlive the question it answers.
                //
                // **The listing goes in with the point**, exactly as it does
                // for the `params` chip above: what a row means is a name this
                // program read out of the store and handed over, and the
                // console reads no store (ADR-0156).
                if let Some(taken) = library_bay(
                    self.panel.layout(),
                    &self.view.scopes,
                    &self.view.library,
                    self.view.opened(),
                    self.view.pointed(),
                    self.view.library_scroll(),
                )
                .and_then(|bay| bay.take(self.view.rows(), at))
                {
                    return (claim, self.took(at, taken));
                }
                // **The same rows, meaning the other thing.** Under
                // `history` a row is a version rather than a Set, so a press
                // on it is a landing — `Operation::RestoreProcedure` on the
                // deck the load pulldown names — where the carry above takes
                // nothing in hand because `View::sets` handed it nothing.
                // The two are told apart by which listing goes in with the
                // point and not by an arm that asks the scope, so exactly one
                // of them can answer and `input::claim`'s one row for this
                // rectangle stays one row.
                if let Some(operation) = library_bay(
                    self.panel.layout(),
                    &self.view.scopes,
                    &self.view.library,
                    self.view.opened(),
                    self.view.pointed(),
                    self.view.library_scroll(),
                )
                .and_then(|bay| bay.land(self.view.versions(), self.view.target(), at))
                {
                    return (claim, self.landed(operation));
                }
                // **The Staging lane's `back` capsule, asked before the row
                // it sits in.** The capsule is inside the row, so this order
                // is what makes a press on the capsule reach the capsule —
                // `input::claim`'s rule 4, and the Library bay's star and its
                // row two arms up, in the same order and for the same reason.
                //
                // **The candidates go in with the point**, exactly as the
                // Library's listing does: what a row is — which node, which
                // deck, which verdict — is a value this file derived off the
                // deck and handed the console, and the console holds no store
                // and no engine (ADR-0156).
                //
                // **It emits and performs nothing here**: what a landing does
                // is write a file the store owns, and [`restored`] is where
                // that is done, on the branch every emitted operation already
                // takes.
                if let Some(operation) = staging_bay(self.panel.layout(), &self.view.staging)
                    .and_then(|bay| bay.back(ctx, &self.view.staging, at))
                {
                    return (claim, self.landed(operation));
                }
                // **And the row itself, which is the keep.** A press anywhere
                // on a candidate row that the capsule did not take settles
                // that node: the row leaves the lane, the picture does not
                // move and no record is written. The bay is derived a second
                // time rather than held across the two questions, which is the
                // Library bay's rule one bay up.
                if let Some(operation) = staging_bay(self.panel.layout(), &self.view.staging)
                    .and_then(|bay| bay.keep(ctx, &self.view.staging, at))
                {
                    return (claim, self.kept(operation));
                }
                // **The transition row's four capsules, derived once for all
                // of them**, exactly as `claim` does it: the three settings
                // are laid end to end from the block's left padding and `go`
                // is measured back from the right one, so where each of them
                // is depends on the words beside it and a second walk would
                // put the capsule a press lands on somewhere the word is not.
                //
                // **It cannot overlap the bay below**: `.xfade` is under
                // `.mixer-strips` and outside every strip's rectangle, so the
                // order against the mixer is arbitrary. This is asked first
                // because it is the one control on this panel whose press can
                // be *refused*, and a refusal is a line rather than a value
                // the arms below could carry.
                let row = transition_row(ctx, self.panel.layout(), self.view.transition());
                if let Some(row) = row.as_ref() {
                    if let Some(operation) = row
                        .shape(at)
                        .or_else(|| row.quantum(at))
                        .or_else(|| row.length(at))
                    {
                        return (claim, Acted::Emitted(Some(operation)));
                    }
                    // **The `go` capsule**, which is the only control here
                    // that answers something other than an operation or
                    // nothing: a one-strip mixer and a shape reading `no
                    // shape` are turned away by the control itself, because
                    // the shape is the console's own setting and no conversion
                    // can see it is unset. `karakuri-cli`'s `c` refuses the
                    // same two before it asks, and this is that pair as a
                    // value with the sentence on this side of the seam.
                    match row.go(at, self.view.selection(), self.view.mixer.len()) {
                        Some(Go::Wipe(operation)) => {
                            return (claim, Acted::Emitted(Some(operation)))
                        }
                        Some(refused) => {
                            println!("{}", refusal(&refused, self.view.mixer.len()));
                            return (claim, Acted::Nothing);
                        }
                        None => {}
                    }
                }
                // **The Sequencer bay's three, asked before the knobs
                // below**, and the order is arbitrary rather than a
                // precedence: this bay is in the right pane under the master
                // and no rectangle of it overlaps a strip, a knob or a chip.
                // It is asked as one derivation for all three — a cell, a
                // label and the mode pill are three questions about one
                // laid-out bay, which is `input::claim`'s own row for them.
                //
                // **The press names the bank it landed on**, which is inside
                // the operation: `Sequencer::press` carries `Sequencer::bank`
                // so that a press cannot mean *whichever pattern is armed by
                // the time this is performed*.
                let choices = self.view.lane_choices();
                let seq = sequencer_bay(
                    ctx,
                    self.panel.layout(),
                    self.view.sequencer.as_ref(),
                    &choices,
                );
                // **With the chooser's card down, every press is the card's
                // and this arm is asked first** — `input::claim`'s rule 2, and
                // it has to be *here* rather than after the four controls
                // below: the card hangs over this bay's own rows, so a press
                // on a cell under it is part of the gesture the hand is in the
                // middle of and not a step being set. That is the deck
                // pulldown's `if self.view.target_open()` one bay along, and
                // for its reason.
                //
                // **A pick emits and the two card moves do not**, which is that
                // control's other half: putting a card down is the console's
                // own state and no operation names it (P-0090).
                if self.view.lane_open() {
                    did = self.chosen(
                        seq.as_ref()
                            .and_then(|bay| bay.chose(at, &choices))
                            .unwrap_or(Chose::Shut),
                    );
                    return (claim, did);
                }
                if let Some(operation) = seq.as_ref().and_then(|bay| bay.press(at)) {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The foot's `+ lane` with its card up**, asked after the
                // four above it because the pill is outside every one of their
                // rectangles, so the order is arbitrary rather than a
                // precedence — written down so this file and `input::claim`
                // ask in one order.
                if let Some(chose) = seq.as_ref().and_then(|bay| bay.chose(at, &choices)) {
                    did = self.chosen(chose);
                    return (claim, did);
                }
                // **The whole row now, not the first chip.** `chip_at` asks
                // every chip the row draws and answers with the output it
                // names — which is the same question `input::claim` asks, off
                // the same derivation, so a chip that lights under the pointer
                // is a chip a press reaches.
                let sink = outputs(ctx, self.panel.layout(), self.view.opening)
                    .map(|row| row.told(self.view.projector))
                    .and_then(|row| row.chip_at(at).map(|output| (row, output)));
                let bay = mixer_bay(ctx, self.panel.layout(), &self.view.mixer);
                // **The Master bay's out is a `Grab` like a strip's**, so it
                // joins the knob rather than taking an arm of its own: what
                // this file does with either is take it in hand, and which
                // fader it was is inside the `Knob`. The two bays cannot
                // overlap, so the order is arbitrary — the mixer is asked
                // first because it has five questions to this one's one.
                // **Laid out once and asked twice**, where the mixer is: this
                // bay has a knob question and a chip question now, and two
                // derivations of it would be a chip painted where a hand
                // cannot press it.
                let master = master_row(
                    ctx,
                    self.panel.layout(),
                    self.view.master_out,
                    self.view.master_chain,
                );
                let knob = bay
                    .as_ref()
                    .and_then(|bay| bay.grab(at))
                    .or_else(|| master.as_ref().and_then(|row| row.grab(at)))
                    // **A parameter fader is a `Grab` like a strip's**, so it
                    // joins the knob rather than taking an arm of its own —
                    // the Master bay's arrangement one bay along, and which
                    // fader it was is inside the `Knob`. No two of the three
                    // bays overlap, so the order between them is arbitrary.
                    .or_else(|| {
                        self.view
                            .inspector
                            .iter()
                            .enumerate()
                            .find_map(|(index, pane)| {
                                let at_pane = inspector_pane(
                                    self.panel.layout(),
                                    index,
                                    pane,
                                    self.view.scroll_in(index),
                                )?;
                                at_pane.grab(pane, at)
                            })
                    });
                // **The blend chip and the feedback row's cut chip are one
                // question here**, because what this file does with either is
                // the same three steps and which it was is in the operation.
                let chip = bay
                    .as_ref()
                    .and_then(|bay| bay.blend(at))
                    .or_else(|| master.as_ref().and_then(|row| row.chip(at)));
                let tally = bay.as_ref().and_then(|bay| bay.tally(at));
                let mask = bay.as_ref().and_then(|bay| bay.mask(at));
                match (sink, knob, chip, tally, mask) {
                    // **One press, two performers, and which is which is
                    // the output.** The picture's on and off is a layout node,
                    // so the console performs that one and reports an
                    // `Outcome`; a projector is a window this file owns, so
                    // that one leaves as the operation it is and is performed
                    // where the event loop is (`routed`). Both *ask* for
                    // `Operation::RouteFrame` and the row is what constructs
                    // it — see `view::Outputs::route`.
                    (Some((row, Output::Program)), ..) => {
                        did = Acted::Operated(self.sink(row.route(), row.op()))
                    }
                    (Some((row, output)), ..) => {
                        did = Acted::Emitted(
                            row.more
                                .iter()
                                .find(|chip| chip.output == output)
                                .and_then(view::SinkChip::route),
                        )
                    }
                    // **The value does not move on the press.** The grab keeps
                    // the offset it took hold at, so the first move continues
                    // from where the knob already was — and a press that was
                    // on the *track* never gets here, because `Mixer::grab`
                    // answers `None` for it rather than jumping the mix.
                    (None, Some(grab), ..) => {
                        println!(
                            "press ({:.0}, {:.0}): {} — the {} is in hand",
                            at.x,
                            at.y,
                            knob_where(&grab.knob()),
                            knob_word(&grab.knob())
                        );
                        self.panel.grab(at, grab);
                    }
                    // **A chip acts on the press itself**, where a fader acts
                    // on the moves after it: there is no gesture here, only
                    // one operation naming where the cycle arrived. Both go
                    // down the same path a fader's does — `Acted::Emitted`,
                    // then a record, then the deck — because P-0090 is that
                    // every control ends in the same record, and a chip that
                    // reached the deck another way would be a second route for
                    // the same change.
                    //
                    // **One arm for the four chips**, because what this file
                    // does with any of them is the same three steps; which chip
                    // it was is in the operation, and the line `apply` prints
                    // says so. The fourth is the Master bay's cut chip, which
                    // is a cycle over a closed list of two exactly as the
                    // blend's is over three.
                    (None, None, Some(operation), ..)
                    | (None, None, None, Some(operation), _)
                    | (None, None, None, None, Some(operation)) => {
                        did = Acted::Emitted(Some(operation))
                    }
                    // **The strip itself, asked last.** A strip's rectangle
                    // contains all four of the questions above, so this is
                    // what is left over — a press on the name, on the number,
                    // on the ground between the rows — and it means *address
                    // the keys to this deck*. It is the only control in this
                    // bay that is not drawn as one, which is
                    // `console.html`'s *"a press anywhere on a strip that no
                    // knob under the pointer claimed"*: the whole column is
                    // the affordance, and a sixth capsule would be a control
                    // over a pointer.
                    (None, None, None, None, None) => {
                        match bay.as_ref().and_then(|bay| bay.select(at)) {
                            Some(operation) => did = Acted::Emitted(Some(operation)),
                            None => self.press(at),
                        }
                    }
                }
            }
            // **Where a drop names its deck**, and there are two sets of
            // rectangles it can name it by. A carry is the one gesture here
            // whose destination is not known until the button comes up, so the
            // bays are laid out *now* and asked which of their rectangles the
            // pointer is over — `Mixer::dropped` and `ProgramBay::dropped`,
            // the same derivations the frame drew and the same ones `claim`
            // hit-tests the knobs, chips and cells of. They are asked only
            // with a carry in hand: a boundary and a fader each come to rest
            // without a destination, and laying the mixer out on every release
            // would be two galley lookups per strip to answer a question
            // nobody asked.
            //
            // **The order is arbitrary and cannot matter**: the strips are in
            // the right pane and the cells in the centre column, so a point
            // inside one set is outside the other, and `or_else` is two
            // questions about one point rather than a precedence.
            //
            // **The cell half is asked with the deck's slot count**, which is
            // the strips this console was handed: the row is always `DECKS`
            // cells and a deck holds one to four slots, so a release on the
            // fourth cell of a three-slot deck has nothing to load into and is
            // refused exactly as `3` is refused from the keyboard
            // (`pointed`, and `View::select` under it) —
            // [ADR-0273](../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md).
            (Pointer::Up, Claim::Panel) => {
                let onto = match self.panel.in_hand() {
                    Some(InHand::Carrying) => mixer_bay(ctx, self.panel.layout(), &self.view.mixer)
                        .as_ref()
                        .and_then(|bay| bay.dropped(at))
                        .or_else(|| {
                            program_bay(self.panel.layout(), self.view.canvas)
                                .as_ref()
                                .and_then(|cells| cells.dropped(at, self.view.mixer.len()))
                        }),
                    _ => None,
                };
                did = self.released(onto);
            }
            // **A wheel is routed by region rather than by control**, which
            // is `input::wheeled` and not `input::claim` — see that function
            // for why the two are separate questions. The claim it hands back
            // is a *second* answer to who the event belongs to and it can only
            // widen the first: `claim` gives a wheel to the panel while a drag
            // is in hand, `wheeled` gives it to the panel while the pointer is
            // over a pane or over the Library bay, and neither takes one away.
            //
            // **Two regions scroll and `wheeled` is the one place that says
            // which** — an Inspector pane by index, and the Library bay, which
            // is the only one of itself (ADR-0312). Each arm calls that
            // region's own `scroll`; nothing here decides which region a point
            // is in, for `input`'s own reason: the derivation that draws it is
            // what answers.
            //
            // **`Acted::Pointed` and not `Acted::Nothing`**, for the reason
            // that variant exists: a console pointer moved and no operation
            // was named. It is what tells `window_event` a frame is owed —
            // a wheel spun against the top of a list is `Nothing` and earns
            // none, whichever of the two it was over.
            (Pointer::Wheel(by), _) => {
                if let Some(turned) = wheeled(&mut self.panel, &self.view, at) {
                    let moved = match turned {
                        Turned::Pane(pane) => self.view.scroll_by(pane, by),
                        Turned::Library => self.view.scroll_library_by(by),
                    };
                    if moved {
                        did = Acted::Pointed;
                    }
                    claim = Claim::Panel;
                }
            }
            // **The secondary button, and the whole of what it reaches on
            // this panel is a row of the Library bay's list.** A press on one
            // puts that row's menu down; a press anywhere else asks for
            // nothing at all and is not an error — `menu_ask` answers `None`
            // and this arm leaves `did` as `Acted::Nothing`, which is what a
            // press on a bay's ground already does.
            //
            // **One call for both halves of the gesture**, which is
            // `LibraryBay::menu_ask`'s own shape: with no card down it asks
            // *which row did this name*, and with one down it asks *which item
            // did this pick* — so a secondary press while the menu is open
            // picks or dismisses exactly as a primary one does, and the
            // gesture does not care which button ends it.
            //
            // **The claim is asked the same way and is not this button's
            // question**: `input::claim` decides whose an event is from where
            // the pointer is, so a secondary press over a row is the panel's
            // for the reason a primary one is, and one over a boundary or over
            // nothing is not.
            (Pointer::Secondary, Claim::Panel) => {
                self.panel.solve();
                let picked = library_bay(
                    self.panel.layout(),
                    &self.view.scopes,
                    &self.view.library,
                    self.view.opened(),
                    self.view.pointed(),
                    self.view.library_scroll(),
                )
                .and_then(|bay| {
                    bay.menu_ask(
                        ctx,
                        view::to_egui(self.panel.layout().viewport()),
                        self.view.menued(),
                        self.view.rows(),
                        at,
                    )
                });
                if let Some(ask) = picked {
                    did = self.menued(ask);
                }
            }
            (Pointer::Down | Pointer::Up | Pointer::Secondary, _) => {}
        }
        (claim, did)
    }

    /// A press on the audio-in pill or on its card, and what this program does
    /// about it.
    ///
    /// [`Readout::arranged`]'s shape one pill to the left, and the split is the
    /// same: the two answers that are the *control's* own state are performed here,
    /// and the one that is an operation leaves as one.
    ///
    /// Opening the card is where the host is read, and it is the only place: a
    /// listing of a machine's inputs is a device enumeration, which is not a thing
    /// to do on a frame path (P-0091) — the same rule under which the Library bay's
    /// names and the arrangement pill's are read on a press. So the list a hand is
    /// about to read is the list as of the press that opened it, an interface
    /// plugged in a minute ago included.
    pub(crate) fn listened(&mut self, ask: AudioAsk) -> Acted {
        let Some(audio) = self.view.audio.as_mut() else {
            // A press on a pill that is not drawn, which `audio_in` answers
            // `None` to and this cannot reach. Said rather than unreachable.
            return Acted::Nothing;
        };
        match ask {
            AudioAsk::Open => {
                audio.inputs = karakuri_environment::audio::inputs();
                let held = audio.inputs.len();
                println!(
                    "audio-in: `{}` — {}",
                    audio.word(),
                    match held {
                        0 => String::from(
                            "this machine has no audio inputs, and the card says so rather than                              opening empty"
                        ),
                        1 => String::from("one input to pick from"),
                        many => format!("{many} inputs to pick from"),
                    }
                );
                audio.opened();
                Acted::Nothing
            }
            AudioAsk::Shut => {
                audio.shut();
                Acted::Nothing
            }
            // **Out of this crate and into the one that can open a device.**
            // The pill names the input and `attached` opens it, which is the
            // seam ADR-0156 draws: a control asks, and whoever holds the
            // device decides.
            AudioAsk::Operation(operation) => {
                audio.shut();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// A press on the arrangement pill or on its menu, and what this program does
    /// about it.
    ///
    /// Five answers and this file decides none of them: which one a press asks for
    /// is `ArrangementPill::ask`'s, off the same laid-out pill `claim` hit-tested,
    /// and what arrives here is one of them by name. Two are moves of the control's
    /// own state and are this program telling the console about a press it cannot
    /// see; two are operations and go where every operation goes; the fifth is the
    /// reset, which is an `Op` and not a record, exactly as ADR-0208 has it —
    /// *"`ResetArrangement` reaches code; `RestoreArrangement` reaches a file"*.
    ///
    /// The menu shuts on anything that acts. An operator who has picked an item has
    /// finished with the list, and a card left standing over the console after the
    /// thing it was for has happened is the panel arguing with itself. It stays
    /// open for nothing, because nothing here can be picked twice.
    pub(crate) fn arranged(&mut self, ask: Ask) -> Acted {
        match ask {
            Ask::Open => {
                println!(
                    "arrangement: `{}` — save it, start a new one, or put one of {} back",
                    self.view.arrangement.word(),
                    self.view.arrangement.filed.len()
                );
                self.view.arrangement.opened();
                Acted::Nothing
            }
            Ask::Shut => {
                self.view.arrangement.shut();
                Acted::Nothing
            }
            // **The one flow on this panel that asks for letters.** Reached
            // only with no arrangement in use: with one in use, saving again
            // means that name and the pill asks for the operation instead.
            Ask::Name => {
                println!(
                    "arrangement: type a name and press return — letters, digits, `-` and \
                     `_`, and escape leaves it unsaved"
                );
                self.view.arrangement.asks_a_name();
                Acted::Nothing
            }
            // **The same operation `r` performs**, reached from the other end
            // of the panel exactly as the Outputs row's dot reaches `f`'s
            // fold. `Readout::op` is what says the arrangement in use is the
            // default again, whichever surface asked.
            Ask::Panel(op) => {
                self.view.arrangement.shut();
                Acted::Operated(self.op(op))
            }
            // **Down the path every other emitted operation takes**, which is
            // the whole reason `arrangement` sits on it: a record is written
            // by whoever holds the store, and the pill holds nothing.
            Ask::Operation(operation) => {
                self.view.arrangement.shut();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// What a press on a `uses` line did — the capsule, or a row of the card it
    /// puts down (`docs/adr/0329-…`).
    ///
    /// [`App::aimed`]'s shape one bay along and the same division: every arm is
    /// either this console's own state moving or one operation emitted down the
    /// path every other operation takes. Nothing is performed here — a `WireInput`
    /// is [`wired_input`]'s, which is the same re-aim a model's `wire_input`
    /// already goes through.
    ///
    /// A press on the capsule of a card that is down shuts it, because the capsule
    /// is *outside* the card and every press outside a card that is down is the
    /// dismissal. Pressing it twice therefore opens and closes, and no arm has to
    /// special-case it.
    pub(crate) fn wired(&mut self, ask: Wiring) -> Acted {
        match ask {
            Wiring::Chip { pane, node, input } => {
                let named = self
                    .view
                    .inspector
                    .get(pane)
                    .and_then(|pane| pane.nodes.get(node))
                    .and_then(|node| node.uses.get(input));
                match named {
                    Some(uses) if uses.candidates.is_empty() => {
                        println!(
                            "wire: `{}` takes one node of its kind and this deck holds only the                              one it is already wired to — there is nothing to pick",
                            uses.slot
                        );
                    }
                    Some(uses) => println!(
                        "wire: `{}` is wired to `{}` — pick a node to wire it to instead",
                        uses.slot, uses.to
                    ),
                    None => {}
                }
                self.view.open_wiring(pane, node, input);
                Acted::Nothing
            }
            Wiring::Shut => {
                self.view.shut_wiring();
                Acted::Nothing
            }
            // **The pick puts the card away, rewires, and emits**, which is one
            // gesture: the card left down over the pane the rebuild is about
            // would be a list to dismiss before the picture could be seen.
            //
            // **Emitted and performed where every other operation is**, which
            // is [`App::performed`]: the run's edge list moved to [`Engine`] so
            // that a press could reach it, because this type is the console's
            // readout and holds no engine at all. See [`wired_input`].
            Wiring::Pick(operation) => {
                self.view.shut_wiring();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// What a press on a pane head's `▾` did — the mark, or a row of the card it
    /// puts down (ADR-0338, decision 5).
    ///
    /// [`Readout::wired`]'s shape one row up and the same division: every arm is
    /// either this console's own state moving or one operation emitted down the
    /// path every other operation takes. Nothing is performed here — a `PointPane`
    /// is [`pointed_pane`]'s, which is where the pane is resolved and the deck
    /// refused.
    ///
    /// A press on the mark of a card that is down shuts it, for
    /// [`Readout::wired`]'s reason: the mark is outside the card and every press
    /// outside a card that is down is the dismissal.
    pub(crate) fn pointing(&mut self, ask: view::Pointing) -> Acted {
        match ask {
            view::Pointing::Mark(pane) => {
                println!(
                    "inspector: pane `{}` is showing deck {} — pick a deck to point it at",
                    view::PANE_NAMES.get(pane).copied().unwrap_or("?"),
                    deck_letter(self.view.pane_deck(pane)),
                );
                self.view.open_pane_target(pane);
                Acted::Nothing
            }
            view::Pointing::Shut => {
                self.view.shut_pane_target();
                Acted::Nothing
            }
            // **The pick puts the card away and emits**, and it is one
            // gesture: `View::point_pane` is what takes the card down, and it
            // is reached through [`pointed_pane`] so that a press and a
            // model's `operate` move the pointer by one route.
            view::Pointing::Pick(operation) => Acted::Emitted(Some(operation)),
        }
    }

    /// What a press on the Library bay's load control did — the button, the
    /// pulldown, or a row of the list it puts down (ADR-0305).
    ///
    /// [`Readout::arranged`]'s shape one bay along, and the two are the same
    /// division: every arm is either this console's own state moving or one
    /// operation emitted down the path every other operation takes. Nothing is
    /// performed here, and in particular nothing writes a file: a `LoadSet` is
    /// `played`'s, exactly as it is for the key and for the drop.
    ///
    /// A pick moves no deck selection, which is what the record is about:
    /// `View::aim_at` writes the bay's own mark and never `View::select`, so the
    /// ring on the strip and the letter in the foot are free to name two different
    /// decks. It also puts the list away, because a pick is one gesture and nothing
    /// here is emitted for a caller to end it in.
    pub(crate) fn aimed(&mut self, ask: Aim) -> Acted {
        match ask {
            Aim::Open => {
                println!(
                    "load: aimed at deck {} — pick a deck, or press `load` to send \
                     the cursor's Set there",
                    deck_letter(self.view.target_deck())
                );
                self.view.open_target();
                Acted::Nothing
            }
            Aim::Shut => {
                self.view.shut_target();
                Acted::Nothing
            }
            // **The whole of a pick**, and it asks for nothing: the mark is
            // the console's, exactly as the library cursor is, and no
            // operation in the vocabulary names it. `Operation::SelectDeck` is
            // emphatically not what this is — that one moves the keys.
            Aim::Deck(deck) => {
                self.view.aim_at(deck);
                Acted::Nothing
            }
            // **Down the path the key and the drop already take.** `played`
            // performs `LoadSet` by re-pointing the slot's source, so all
            // three routes arrive at the same place (ADR-0228).
            Aim::Load(operation) => Acted::Emitted(Some(operation)),
            // **A load with one operand missing is not a load**, and the
            // press says so rather than going quiet: P-0083, and the same
            // sentence `Released::Nowhere` is answered with one bay along.
            Aim::NoSet => {
                println!("load: nothing under the cursor — this library is listing no Sets");
                Acted::Nothing
            }
        }
    }

    /// A press on the Sequencer bay's `+ lane`, and what this program does about
    /// it.
    ///
    /// [`Readout::aimed`]'s shape one bay along, and the same division: the two
    /// answers that are the *console's* own state are performed here, and the one
    /// that is an operation leaves as one. Nothing appends a lane here —
    /// `sequenced` is where `Operation::PointLane` lands, by the same road the
    /// bay's other four take.
    ///
    /// The card is put away before the operation is emitted, which is
    /// [`Readout::menued`]'s rule and its reason: a card left standing over a lane
    /// that has already been asked for would claim the next press on the console
    /// for a gesture the hand has finished.
    ///
    /// Opening it says what it is for, on `aimed`'s precedent: a card that went up
    /// in silence is a control an operator has to guess the shape of.
    pub(crate) fn chosen(&mut self, ask: Chose) -> Acted {
        match ask {
            Chose::Open => {
                let choices = self.view.lane_choices();
                println!(
                    "lane: {} target{} — {} fader{} and {} published control{} on deck {}",
                    choices.items.len(),
                    match choices.items.len() == 1 {
                        true => "",
                        false => "s",
                    },
                    choices.faders,
                    match choices.faders == 1 {
                        true => "",
                        false => "s",
                    },
                    choices.items.len() - choices.faders,
                    match choices.items.len() - choices.faders == 1 {
                        true => "",
                        false => "s",
                    },
                    deck_letter(self.view.target_deck())
                );
                self.view.open_lane();
                Acted::Nothing
            }
            Chose::Shut => {
                self.view.shut_lane();
                Acted::Nothing
            }
            Chose::Point(operation) => {
                self.view.shut_lane();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// A press on a row's menu, and what this program does about it.
    ///
    /// [`Readout::aimed`]'s shape one control along, and the same division: the two
    /// answers that are the *console's* own state are performed here, and the two
    /// that are operations leave as ones. Nothing is performed here, and in
    /// particular nothing writes a file: a `LoadSet` is `played`'s, exactly as it
    /// is for the button, the key and the drop, and a send is the window's, because
    /// a bundle is a disk read and a file written.
    ///
    /// The menu is put away before either operation is emitted, and it is put away
    /// on both arms rather than on one: a card left standing over a load that has
    /// already been asked for would claim the next press on the console for a
    /// gesture the hand has finished. That is `Menu::Naming`'s own rule at the
    /// arrangement pill, one bay along.
    ///
    /// The load moves no mark. `View::aim_at` is not called and neither is
    /// `View::select` or the cursor: the item named the deck and the row named the
    /// Set, so there is nothing left for this press to have moved — which is the
    /// whole of why the menu is a route worth having.
    pub(crate) fn menued(&mut self, ask: Picked) -> Acted {
        match ask {
            Picked::Open(row) => {
                println!(
                    "menu: `{}` — load it onto a deck, or save it as a kbset",
                    self.view
                        .sets()
                        .get(row)
                        .map(String::as_str)
                        .unwrap_or_default()
                );
                self.view.open_menu(row);
                Acted::Nothing
            }
            Picked::Shut => {
                self.view.shut_menu();
                Acted::Nothing
            }
            // **Down the path the button, the key and the drop already take.**
            // `played` performs `LoadSet` by re-pointing the slot's source, so
            // all four routes arrive at the same place (ADR-0228).
            Picked::Load(operation) => {
                self.view.shut_menu();
                Acted::Emitted(Some(operation))
            }
            // **The send leaves as an operation and the file is written where
            // every other disk write on this panel is** — the window, on the
            // branch a star and a keep already take, because a bundle is a
            // store read and a `.kbset` is a file (P-0091, ADR-0156).
            Picked::Send(operation) => {
                self.view.shut_menu();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// The name is finished, and what that asks for.
    ///
    /// One operation of the vocabulary, named — the same
    /// `Operation::SaveArrangement` the menu's *save* asks for with an arrangement
    /// already in use, so the two ways to reach a save are two ways to name one
    /// thing rather than two paths to a disk. The menu is shut before the operation
    /// is emitted, whether or not the name is any good: a name that is refused is
    /// refused out loud by `checked_name`, and a card left standing over the
    /// refusal would be the panel asking the question again without saying the
    /// answer.
    ///
    /// An empty name arrives here as an empty name and is refused there, which is
    /// the rule this file keeps everywhere: the surface owns the affordance and
    /// never the authority (P-0090).
    pub(crate) fn named(&mut self) -> Acted {
        let Some(typed) = self.view.arrangement.naming() else {
            return Acted::Nothing;
        };
        let name = typed.to_owned();
        self.view.arrangement.shut();
        Acted::Emitted(Some(Operation::SaveArrangement { name }))
    }

    /// A press on the Program bay head's `solo`. The pill says what it did — which
    /// of the two operations it asked for — because the whole point of the control
    /// is that it is the same solo `s` and `u` perform, reached from a capsule
    /// instead of from the pointer.
    ///
    /// The region is the picture's and never the pointer's, which is the one way
    /// this differs from `s`: a key solos whatever the pointer is over, and this
    /// pill names `program-view` because `docs/manual/console.html` says what it is
    /// for — *"Solo the program view: the panel folds away and only the picture is
    /// left, which is also how you capture this window."* A press on the grip in a
    /// bay head. It says what it did, because the point of the control is that it
    /// is the fold `f` performs, reached from the console's own shape instead of
    /// from the keyboard. One operation and no toggle: a folded bay has no
    /// rectangle, so the grip is not drawn afterwards and the way back is `z`.
    ///
    /// One control and not two. ADR-0295 gave a pane a band on its outer edge and
    /// this doc described both; ADR-0300 replaced that half — a pane folds by its
    /// own boundary being pulled past the narrowest it goes, and comes back by that
    /// boundary being dragged in, so it needs no press arm and its way back is not
    /// `z` alone.
    pub(crate) fn folded(&mut self, op: Op) -> Outcome {
        let Op::Fold(id) = op else {
            unreachable!("a fold control asked for {op:?}")
        };
        println!("fold: {} folds away — `z` brings it back", self.label(id));
        self.op(op)
    }

    pub(crate) fn soloed(&mut self, op: Op) -> Outcome {
        println!(
            "program: {}",
            match op {
                Op::Solo(_) =>
                    "solo the picture — everything else folds away, and the window                      is that region",
                Op::Unsolo =>
                    "the solo comes off — what was folded before it comes back,                      including whatever was already folded",
                other => unreachable!("the solo pill asked for {other:?}"),
            }
        );
        self.op(op)
    }

    /// A press on one of the four class pills, and the one press in this program
    /// that is neither an operation on the arrangement nor one on the mix.
    ///
    /// # Why it takes a different path from every other press in this file
    ///
    /// Everything else here ends in one of two places. A control over the console's
    /// own shape asks for a [`Op`], `Panel` performs it, and what comes back is an
    /// [`Outcome`]. A control over the mix emits an [`Operation`], [`written`]
    /// turns it into a `Record` and [`apply`] moves the deck with it — P-0090, and
    /// every control ends at the same record. A reader who has just met those two
    /// will reach for the second here, because it is the one every new control has
    /// taken for a year.
    ///
    /// It must not be routed as an `Operation`, and
    /// [ADR-0236](../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
    /// is explicit about it. The opening is configuration of the *map* — the layer
    /// every surface reaches the vocabulary through — and not a member of the
    /// vocabulary the map addresses. The rule is narrower than *map configuration
    /// is never an operation*, because `Operation::PointLane` already is one: a
    /// setting that decides whether a surface may reach a class of operations
    /// cannot itself be one of those operations. Rule 01 would make such an
    /// operation reachable from all four surfaces, MCP included, and a permission
    /// an actor can grant itself is not a permission. There is no 65th row on the
    /// operations page for the same reason, and ADR-0235's *"the opening setting
    /// has no operation"* is annotated as settled by exactly this.
    ///
    /// So: no `Operation`, no `Record`, no [`Acted::Emitted`]. What a press hands
    /// over is a value — `McpPill::next`, the opening with one class set the other
    /// way and the other three written back as they were — and the run's `Opening`
    /// is where it goes. Somebody will one day try to fix this into the vocabulary;
    /// this paragraph is what it costs them to do it, and `Acted::Opened` is the
    /// type that will not let it happen quietly.
    ///
    /// The view's copy is written in the same breath as the handle, not left for
    /// the next frame's read. `input::claim` and the probe above both hit-test
    /// against `View::opening`, and the pill is not the same width in its two
    /// states — so a press that moved the handle and not the view would leave the
    /// very next press aimed at the capsule that was there before it.
    pub(crate) fn opened(&mut self, pill: &McpPill) -> Acted {
        // **Annotated**: it says what a press composes — an opening and not a `bool`.
        let next: Open = pill.next(self.view.opening);
        self.opening.set(next);
        self.view.opening = next;
        let open = next.holds(pill.class);
        // **What the pill says it did, in the words a refusal says it in.**
        // `Class::title` and `Class::opened_at` are the gate's own strings, so
        // the sentence a model is refused with and the sentence an operator
        // reads at the pill name one thing the same way (P-0090).
        println!(
            "{}: `{}` — {} is {} to a model. {}. the operator opens it at {}.",
            pill.class.bay(),
            view::mcp_word(open),
            pill.class.title(),
            match open {
                true => "open",
                false => "shut",
            },
            match open {
                true => "calls in this class are performed",
                false => "calls in this class are refused, and the refusal says so",
            },
            pill.class.opened_at()
        );
        Acted::Opened
    }

    /// A press on a scope chip, and it is the surface performing its own pointer —
    /// [`pointed`]'s shape one bay along, done here rather than in `performed` for
    /// the reason the scope key's is done at the key.
    ///
    /// `Operation::SelectScope`'s payload is `Undecided`, so a performer reading
    /// the operation could not tell which library was chosen and would have to
    /// guess. The press *knows*, because a pointer lands on one capsule and no
    /// other, and [`Chosen`] is what carries the two halves together. So the mark
    /// is moved here and the operation is emitted for the record it is owed, which
    /// is `Silent(Surface)` — the same shape as the key, which steps first and
    /// emits afterwards.
    ///
    /// A chip that is already marked is not refused, and the line says which of the
    /// two it was. `View::select_scope` answers `false` for it, and that is a mark
    /// that did not move rather than a press that failed: where the key *steps* and
    /// would go somewhere else, a press names, and naming the library you are
    /// already reading is asking it again. What the caller does with that is
    /// re-read the listing, which is where a directory read belongs (P-0091) and is
    /// not on this side of the seam.
    ///
    /// It cannot refuse for the other reason either: the chip came out of
    /// `View::scopes`, so it is on the row by construction.
    pub(crate) fn chose(&mut self, chosen: Chosen) -> Acted {
        let moved = self.view.select_scope(chosen.scope);
        println!(
            "scope: `{}` — {}",
            chosen.scope.name(),
            match moved {
                true => "the library this bay reads, and the cursor is back at the top of it",
                false => "already the library this bay reads, so this asks that one again",
            }
        );
        // **The Set the walk is of is read on the way out**, and it is this
        // side's answer rather than the chip's: `Operation::WalkHistory` names
        // a Set, the console holds a deck letter, and the id rides the aim
        // (ADR-0308). `View::aimed` is where this file writes it, per frame
        // beside every other reading, and `Chosen::asked` is the one place it
        // is read — so the operation this press emits and the listing the press
        // below re-reads are narrowed by one value. The four library chips
        // ignore it and emit `SelectScope`, which carries nothing.
        Acted::Emitted(Some(chosen.asked(self.view.aimed.as_deref())))
    }

    /// A press on one of the Library bay's two filter fields, and it is
    /// [`Readout::chose`]'s shape one row down: the surface performs its own
    /// pointer and emits the operation for the record it is owed, which is
    /// `Silent(Question)` — *it asks rather than changes*.
    ///
    /// The operation carries everything, where `SelectScope` carries nothing.
    /// `Operation::ListSets { holds, layer }` is exactly the state the two fields
    /// are in, so this applies it rather than guessing at it and there is no value
    /// travelling beside it. `View::narrow` is the one door into that state and is
    /// where a `holds` this console cannot draw is refused — which nothing here can
    /// hand it, because the value came out of `LibraryBay::filter` stepping the
    /// same candidates.
    ///
    /// The listing is not read here. It is a directory read
    /// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md))
    /// and the store is the window's rather than the readout's, so the caller
    /// re-reads on `Operation::ListSets` exactly as it does on
    /// `Operation::SelectScope` — one branch, two operations, because a scope and a
    /// filter are the same question asked of different halves.
    ///
    /// A filter set while the bay is reading something else is said out loud, and
    /// it is the one thing about this row that would otherwise be silent: the
    /// operation is *List what the store holds*, which is `all` and the `my sets`
    /// starred out of it, and `presets` and `folder` are not the store. The press
    /// is still a real question — it is answered the moment one of those two is
    /// marked again — and a press that appears to do nothing is what this line
    /// exists to prevent.
    pub(crate) fn narrowed(&mut self, operation: Operation) -> Acted {
        let holds = match &operation {
            Operation::ListSets { holds, .. } => holds.as_deref(),
            // **A kind press keeps the field where it is**, which is the whole
            // of what two controls on one row means: `FilterLibrary` carries
            // the six chips and says nothing about `holds`, so the value that
            // goes back into `View::narrow` is the one the field is already on.
            Operation::FilterLibrary { .. } => self.view.filters().holds,
            _ => {
                unreachable!("the filter row emits `ListSets` and `FilterLibrary` and nothing else")
            }
        };
        let holds = holds.map(str::to_owned);
        let kinds = match &operation {
            Operation::FilterLibrary { kinds } => *kinds,
            // **And a `holds` press keeps the chips where they are**, for the
            // reason above read the other way: `ListSets` carries no kinds.
            _ => self.view.filters().kinds,
        };
        let moved = self.view.narrow(holds.as_deref(), kinds);
        let at = self.view.filters();
        println!(
            "filter: `{}` / {} — {}",
            at.holds_word(),
            showing(at.kinds),
            match (self.view.scope(), moved) {
                (Some(Scope::AllSets | Scope::MySets), true) =>
                    "the listing under it is what the store holds, narrowed, and the cursor is \
                     back at the top of it",
                // **A step that arrived where it already was**, which is the
                // `holds` field on a store whose Sets name no node: the press
                // asks for the listing again, and that is `Readout::chose`'s
                // answer for the chip that is already marked.
                (Some(Scope::AllSets | Scope::MySets), false) =>
                    "already what this bay is narrowed to, so this asks the store that same \
                     question again",
                _ =>
                    "this narrows the store's own listing, which is `all` and the `my sets` \
                      starred out of it — neither is the library this bay is reading, so mark \
                      one of them and the rows follow",
            }
        );
        Acted::Emitted(Some(operation))
    }

    /// A press on the Library bay's `params` chip, and it is
    /// [`Readout::narrowed`]'s shape one row down with one difference: only one of
    /// the two things a press on this chip can mean is an operation.
    ///
    /// Opening asks for a reading and this file does not answer it, which is
    /// [`Readout::narrowed`]'s division exactly: what a Set declares is on a disk,
    /// the store is the window's rather than the readout's, and a press is where
    /// this program already reads one. So the operation leaves here and the caller
    /// answers it with [`read_reading`], on the same branch it re-reads a listing
    /// on.
    ///
    /// Closing is performed here and emits nothing. It changes which rows this bay
    /// is drawing, which is the console's own state — no more an operation than a
    /// fold is — and a `ReadSet` emitted to put a reading away would say a question
    /// was asked at the moment one stopped being. See `view::Read`, where the
    /// argument is.
    pub(crate) fn asked_to_read(&mut self, ask: Read) -> Acted {
        match ask {
            Read::Open(operation) => Acted::Emitted(Some(operation)),
            Read::Shut => {
                println!(
                    "read: closed — {}",
                    match self.view.shut_reading() {
                        true => "the list is a list again, and the chip asks for it back",
                        // The chip answers `Shut` off the block the bay is
                        // drawing, so this is a reading that went away between
                        // the layout and the press. Said rather than
                        // unreachable.
                        false => "there was nothing open",
                    }
                );
                Acted::Nothing
            }
        }
    }

    /// A press on a row of the Library bay's list, which takes that Set in hand and
    /// asks for nothing.
    ///
    /// # The mark on the row is the whole of what a carry can draw
    ///
    /// `docs/manual/console.html` draws no drag affordance and no drop target — no
    /// ghost under the pointer, no lit strip — and this program draws what that
    /// page draws. What it *does* draw is `.lib-row.cursor`, and the row a hand is
    /// on is exactly what that mark is for, so the press moves it: the row taken is
    /// the row marked, for the length of the carry and afterwards.
    ///
    /// Afterwards is deliberate. The cursor is the operand a load reads
    /// (`view::View::cursor_row`), so a drop that landed and a carry that was let
    /// go over nothing both leave the keyboard aimed at the Set the hand last
    /// touched — *two ways in, one name*, met at the pointer this bay keeps rather
    /// than only at the operation.
    ///
    /// It emits nothing, and there is nothing for it to emit. Moving this cursor
    /// has no row on `docs/manual/operations.html` and is not owed one, and
    /// `docs/manual/console.html` is where that is said: *"The cursor moves on the
    /// arrow keys and gets no row on the operations page, which is a decision and
    /// not an omission"* — a pointer that names a row instead of stepping to it is
    /// the same pointer, which is what `view::View::point_at` is.
    ///
    /// And a pointer that is the same pointer owes what the keys owe.
    /// `view::View::opened` draws the reading only where the row under the cursor
    /// is still the Set it was read of, and the rule that keeps that honest is the
    /// cursor's rather than the keyboard's: *"the reading follows the cursor: a
    /// move with one open is a read of the row it arrived at"*
    /// (`karakuri-console/src/view.rs`, `view::View::reading_open`). So this
    /// answers [`Acted::Pointed`] where the mark actually moved, exactly as the
    /// arrow keys answer `Change::Pointed(moved)`, and the window loop re-reads on
    /// it the way it re-reads on theirs. A carry that discarded the `bool` made the
    /// block under an open reading vanish for the length of the run, because
    /// nothing else on this route ever moves the cursor back.
    ///
    /// That is still no operation. `read_reading` reads the store and writes the
    /// answer into the view; it emits nothing, so the press
    /// `karakuri_console::input`'s own doc calls *"the one offer on this console
    /// whose press names no operation"* goes on naming none (ADR-0265).
    ///
    /// That sentence is now qualified rather than untrue: it is about the four
    /// scopes whose rows are Sets. Under `history` the same rectangle is
    /// [`Readout::landed`], which names `Operation::RestoreProcedure` outright —
    /// and this method is not reached there, because `View::sets` hands the carry
    /// nothing (ADR-0308).
    pub(crate) fn took(&mut self, p: Point, taken: Taken) -> Acted {
        let Taken {
            row,
            set,
            procedure,
        } = taken;
        // **The mark first, and the hand after it.** Both are this console's
        // own pointers and neither is an operation, so the order is only about
        // the borrow — but the mark is what says the press was seen.
        let moved = self.view.point_at(row);
        println!(
            "press ({:.0}, {:.0}): `{set}` is in hand — let it go over a strip to load it there, \
             or anywhere else to load nothing",
            p.x, p.y
        );
        self.panel.carry(p, set, procedure);
        // **The `bool` is answered rather than dropped**, which is the whole
        // of the re-read above: a row the hand arrived at is a row the reading
        // moves to, and a press that landed on the row the cursor was already
        // on moved nothing and asks for nothing.
        match moved {
            true => Acted::Pointed,
            false => Acted::Nothing,
        }
    }

    /// A press on a row of the Library bay's `history` scope, which lands that
    /// version on the node it was a version of.
    ///
    /// [`Readout::took`]'s neighbour on the same rectangle, and the two are the
    /// same press meaning two things: a row of a library is a Set to take in hand
    /// and a row of a history is a version to put back. Which of them answers is
    /// decided by which listing went in with the point (`view::View::sets`,
    /// `view::View::versions`) rather than by an arm here asking the scope.
    ///
    /// It emits and performs nothing, which is [`Readout::asked_to_read`]'s
    /// division: what a landing does is write a file the store owns, and the store
    /// is the window's rather than the readout's. [`restored`] is where it is done,
    /// on the branch every emitted operation already takes.
    ///
    /// The cursor is not moved. A carry moves it because the mark on a row is what
    /// a drag has to draw and because a load reads it afterwards; a landing reads
    /// neither — the row is the operand and the deck is the pulldown's — so moving
    /// the mark would be this press quietly re-aiming the key beside it.
    pub(crate) fn landed(&mut self, operation: Operation) -> Acted {
        println!(
            "press: {} — the version is written over that node's working copy and its \
             watcher builds it, judged against the budget like any edit",
            match &operation {
                Operation::RestoreProcedure {
                    deck,
                    revision: karakuri_operation::Revision::Picked(version),
                } => format!("`{version}` put back on deck {}", deck_letter(*deck)),
                // **The staging lane's arm, which names a node rather than a
                // version.** Which version that is, is `restored`'s to work
                // out — the one before the one running, out of the store's
                // history — so this says the node and lets the performance
                // say the file (ADR-0326).
                Operation::RestoreProcedure {
                    deck,
                    revision: karakuri_operation::Revision::Previous(node),
                } => format!(
                    "{} on deck {} stepped back one version",
                    node_addr(ir_layer(node.layer), node.index),
                    deck_letter(*deck)
                ),
                other => format!("{other:?}"),
            }
        );
        Acted::Emitted(Some(operation))
    }

    /// A press on a candidate row, which keeps that candidate.
    ///
    /// [`Readout::landed`]'s neighbour on the same row, and the two are what the
    /// lane offers: the capsule steps a node back a version and the row around it
    /// says *I have looked at this*. The free act is on the large target and the
    /// act that writes a file is on the small one, which is the whole of why they
    /// are arranged this way round (ADR-0326).
    ///
    /// It emits and performs nothing here, which is [`Readout::asked_to_read`]'s
    /// division: what a keep changes is the lane, and the lane is `View::staging`,
    /// which this readout owns but the window writes — so [`kept`] is where the row
    /// is taken off, on the branch every emitted operation already takes. `written`
    /// answers `Silent(Silent::Surface)` for it, which is that division said in the
    /// record vocabulary.
    pub(crate) fn kept(&mut self, operation: Operation) -> Acted {
        println!(
            "press: {} — it settles the node and leaves the lane; the picture does not move \
             and no record is written",
            match &operation {
                Operation::KeepCandidate { deck, node } => format!(
                    "{} kept on deck {}",
                    node_addr(ir_layer(node.layer), node.index),
                    deck_letter(*deck)
                ),
                other => format!("{other:?}"),
            }
        );
        Acted::Emitted(Some(operation))
    }

    /// A press on the Outputs row's one control. The dot says what it did — which
    /// of the two operations it asked for, and what the picture is now — because
    /// the whole point of the control is that it is the same fold `f` over the
    /// picture performs, reached from the other end of the panel.
    pub(crate) fn sink(&mut self, asked: Operation, op: Op) -> Outcome {
        // Two operations and no third, which is `Outputs::op`'s whole
        // argument: the toggle is the dot choosing between them, and what
        // arrives here is one of the two by name.
        // **What was asked and what performs it, in one line.** The
        // operation names the output — `RouteFrame { output: Program, on }` —
        // and the fold is how this surface carries it out, which is one fact
        // said once rather than a second stored `on` beside the layout node.
        println!(
            "outputs: {} — {}",
            match asked {
                Operation::RouteFrame { output, on } => format!(
                    "{} {}",
                    output.name(),
                    match on {
                        true => "on",
                        false => "off",
                    }
                ),
                ref other => format!("{other:?}"),
            },
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

    // -- the legend -----------------------------------------------------

    /// What this program is, said once at startup — and every line of it derived.
    ///
    /// `presets` and `store` are the two directories this run resolved, and they
    /// are passed in rather than read here for the reason every other number in
    /// this function is asked of the thing it is about: a legend that described the
    /// search instead of printing its answer is exactly the defect this function
    /// was repaired of, one paragraph along. See the paragraph below for what that
    /// repair cost to find.
    ///
    /// `mcp_port` is the port `karakuri_mcp::serve` actually bound —
    /// `Reporter::port` and not `--mcp`'s argument, so `--mcp 0` prints the
    /// ephemeral port it got — and `None` is a run that was not asked to serve. It
    /// is passed in for the same reason `presets` and `store` are: the sentence
    /// about the `mcp` pills was written unconditionally, said *nothing in this
    /// process serves MCP yet*, and went on saying it to every run that had just
    /// printed the address its server was listening on.
    pub(crate) fn print_legend(
        &mut self,
        budget_ms: Option<f32>,
        governed: &Report,
        presets: Option<&karakuri_environment::places::Presets>,
        store: &std::path::Path,
        mcp_port: Option<u16>,
    ) {
        self.panel.solve();
        let layout = self.panel.layout();
        let viewport = layout.viewport();
        println!();
        println!(
            "the console, in a {:.0} x {:.0} viewport. every leaf gets its region, and what \
             each one draws is the list further down rather than a sentence here: that list \
             is the arrangement's own nodes read against the view the frame is drawn from, \
             so a body that fills in says so without anybody rewriting a line of this. **the \
             sentence this replaces said every body was empty but the Program bay's two**, \
             and it went on saying it while bay after bay drew one — which is what a \
             description kept beside the thing it describes is worth.",
            viewport.w, viewport.h
        );
        // **Where this run's data is, and both lines are what the resolution
        // returned.** Not a sentence about how a presets root is looked for:
        // the directory is printed, and the phrase beside it is
        // `places::Found`'s own — so a candidate added, reordered or removed
        // changes this line without anybody editing it. A legend that said
        // *"the ones that ship with the program"* would be right until the day
        // it was not, which is the whole of what the paragraph above is about.
        match presets {
            Some(presets) => {
                // **Counted once and read off the listing the bay is drawn
                // from**, rather than described: a sentence about what a
                // preset directory probably holds is the kind of line this
                // legend was found lying five ways with.
                let held = presets_listing(Some(presets)).len();
                println!(
                    "presets: {} — {}. that directory is the app-preset tier: what ships \
                 with the program, written by nobody, and what a run with no paths on \
                 the command line opens on. the Library bay's `presets` scope lists the \
                 {} `.kset` file{} in it — the parts beside them are what those files \
                 name rather than rows of their own — and `enter` on one takes it into the \
                 store and then loads it, which is why opening a preset leaves a row \
                 under `all` — and under `my sets` only if you star it.",
                    presets.dir.display(),
                    presets.found.how(),
                    held,
                    match held {
                        1 => "",
                        _ => "s",
                    }
                )
            }
            // Said once, out loud, and it is this program's only occasion to
            // say it: a run that needed the library for a default pair was
            // refused before a window opened, so reaching here means the pair
            // was given by hand and nothing is broken — the preset tier is
            // simply empty.
            None => println!("{}", karakuri_environment::places::no_preset_library()),
        }
        println!(
            "store: {} — where the Library bay below reads Sets from, where this panel's \
             arrangements are filed, where each deck's working copy was written before this \
             window opened, and what `--store` moves. `karakuri-cli --store` names the same \
             directory and the default is the same constant, which it now is rather than \
             looks like: both ask `karakuri_environment::places::STORE`.",
            store.display()
        );
        // **The cells, and this sentence has been wrong four times.** It said
        // C and D had nothing behind them and named the number — *a deck of TWO
        // slots* — which was true of the deck that shipped before this one.
        // Then it said the one sink was an audition, and it was not: nothing
        // called `Deck::set_preview`, so the sink was a second copy of the
        // picture and the word was a claim about a control that did not exist.
        // Then it said the cells were a control an operator pressed to move the
        // audition, and ADR-0240 retired that control. Then it said three cells
        // read `off` because an off-air slot "has no new frame to show", which
        // was true only because the engine refused to draw one — the gap
        // ADR-0241 named, and this pass closed it. **Every sentence here is read
        // off the deck**, which is the only way this legend stops being
        // rewritten each time: the numbers are counted, not written.
        let live: Vec<usize> = (0..DECKS)
            .filter(|slot| {
                self.view
                    .mixer
                    .get(*slot)
                    .is_some_and(|strip| strip.tally == view::Tally::Live)
            })
            .collect();
        // A cell has a slot behind it or it has nothing; this deck is full, so
        // it is every cell. `Deck::slot_view` is `None` past `slot_count` and a
        // strip is a slot, so the mixer's length is the same count from the
        // other end.
        let behind = self.view.mixer.len().min(DECKS);
        println!(
            "{behind} of {DECKS} cells have a deck slot behind them and every one of them \
             is ON, whatever that slot's residency — a cell draws its own slot's material \
             through the same transfer curve the picture goes through, with no fader on it, \
             because a fader is applied in the mix and a cell is upstream of the mix. that \
             is what an operator watches to decide whether material is worth putting on \
             air, so it cannot wait until it is on air (ADR-0258). each cell is presented \
             from `Deck::slot_view` for the slot it is lettered for, so no cell can be \
             showing another deck under the wrong letter, and the picture above them is \
             the mix, always — there is no control that swaps it for one deck, and the \
             cells are why there does not need to be (ADR-0240). {} — that is about the \
             MIX and not about the cells: a slot that is not LIVE is drawn into its own \
             target and skipped by the composite, so it is watchable and inaudible. \
             every slot steps every frame at the room's tempo, whatever its residency, so \
             a cell shows material running rather than a still — a preview that is not on \
             the beat is not a preview of what putting that slot on air would look like \
             (ADR-0269). every cell costs a present pass of its own, and the step and the \
             draw of the three that are not LIVE are outside the compute budget — the \
             bill ADR-0258 says is paid rather than argued, with ADR-0269's three steps \
             added to it.",
            match live.as_slice() {
                [] => "nothing on this deck is LIVE, so the mix is empty".to_owned(),
                [one] => format!(
                    "deck {} is the only slot that is LIVE and reaches the mix",
                    deck_letter(*one as u8)
                ),
                many => format!(
                    "{} slots are LIVE and reach the mix — {}",
                    many.len(),
                    many.iter()
                        .map(|slot| format!("deck {}", deck_letter(*slot as u8)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            },
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
        // **Two, and the list is not kept here.** `view::transport`'s *What is
        // in the mock's row and is deliberately not here* is the list, item by
        // item with what is missing behind each; this names the count and the
        // two survivors and points at it, because the crate that draws the row
        // is the thing that holds whether an item is drawn. Written out here it
        // was a second copy and it drifted exactly as one does: it said four
        // and named `landed` and `rec`, both of which left that list on
        // 2026-09-08 and both of which this panel draws.
        println!(
            "every control the mock draws in that row is drawn, and that list is empty for \
             the first time. `view::transport` is where it is kept. it has only ever got \
             shorter: `audio-in` left it when this program opened an input, `tap`, the \
             octave and the offset track left it when the tracker group landed, `rec` and \
             `landed` left it when this program grew a session recorder and a watcher on \
             every slot, and `learn` and `map` left it when this program opened a MIDI port \
             — all of them are drawn, they are pressed, and each emits the operation its \
             key already emitted or reports something this program really has. `map` is the \
             one that is a READOUT: it names the file that is loaded, and reaching a \
             different map while running is not built, so it has no chevron and no menu."
        );
        // **What the pill is actually reading, off the view rather than off a
        // sentence.** The legend was found lying five ways on 2026-08-30 by
        // saying what this program probably does; this says what it did.
        println!(
            "{}",
            match self
                .view
                .audio
                .as_ref()
                .and_then(|audio| audio.device.as_deref())
            {
                Some(device) => format!(
                    "the `audio-in` pill reads `{device}` and is drawn armed. energy, onset and \
                     band0..7 on the session's bus are measured from that input every frame, and \
                     the beat lock corrects the oscillator the transport row above draws. `b` \
                     taps, `,` and `.` move the grid an octave, `o` and `p` step the offset, and \
                     the three controls beside the pill are those same operations under a \
                     pointer — the tap capsule, the `1/2 x2` pair, and an 80px track that sets \
                     the offset outright. a press on the pill lists what else this machine has."
                ),
                None => String::from(
                    "the `audio-in` pill reads `none`, which is a state and not a fault: no \
                     input is open, every signal name answers what it answered before audio \
                     existed, and the grid free-runs at the session tempo. a press on the pill \
                     lists what this machine has, and `b`, `,` and `.` — and the tap capsule \
                     and the octave beside the pill — say so rather than doing nothing. there \
                     is no offset track: an offset belongs to a session and there is none, so \
                     the row closes up rather than drawing one at a number nobody chose."
                ),
            }
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
             must not jump to 1.0 on stage. the chips beside the faders are played too, and \
             this file has stopped counting them: what a pointer reaches on this panel is \
             said once, below, and asked of the crate that hit-tests it.",
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
        // **And why, read off the report rather than asserted here.** This
        // said *the budget has no room* in a fixed string, which was the only
        // park a bare run could reach while the default pair was 262144
        // elements. ADR-0271 moved it to `examples/star_vortex.kset`'s pair,
        // which is closed form — and `Governor::admit` asks *is there anything
        // to warm* before it asks the budget, so the same park comes back as
        // `NoPrimingNeeded` and a sentence naming the budget is this file
        // describing a decision it did not read. The line under this one has
        // printed the governor's own word all along; the two disagreeing is
        // worse than either being wrong alone.
        let why = match governed
            .decisions
            .iter()
            .find(|decision| decision.slot == ASKED_TO_PRIME)
            .map(|decision| decision.reason)
        {
            Some(Reason::NoHeadroom) => "the budget has no room",
            Some(Reason::NoPrimingNeeded) => {
                "the Set is closed form and has nothing to warm, which is the one park no \
                 amount of budget resolves"
            }
            Some(Reason::Unmeasured) => "nothing measured what that slot costs",
            Some(Reason::CommittedUnknown) => {
                "what this deck is already spending is unknown, so there is no headroom \
                 figure to admit against"
            }
            _ => "the governor did not grant it",
        };
        println!(
            "deck {parked}'s strip is the one that MOVES: its chip reads ALLOC and rolls \
             part of the way toward PRIM once a second and falls back, never landing, \
             because what the slot was asked for and what it is doing disagree. this \
             program asks for {parked} to be primed at startup and {why}, \
             so `Deck::govern` holds it at allocated with the request intact — that is a \
             PARK, which is `not now` and not `no`: nothing has to be asked twice, and the \
             next pass over a deck with room admits it. nothing in this file writes an \
             effective residency or draws a park; the strip carries both of the deck's own \
             words for that slot and the view derives the rest.",
            parked = deck_letter(ASKED_TO_PRIME as u8)
        );
        // **The slots nobody asked anything of, counted off the report rather
        // than named here.** `Reason::OffAir` is the governor's own word for
        // *allocated, and that is what was asked for*: it was not asked about
        // these slots and did nothing to them. A list written out in this file
        // would be this paragraph going on saying `C and D` the day the deck
        // opens differently.
        let resting: Vec<&str> = governed
            .decisions
            .iter()
            .filter(|decision| decision.reason == Reason::OffAir)
            .map(|decision| deck_letter(decision.slot as u8))
            .collect();
        if !resting.is_empty() {
            println!(
                "the other {} — {} — {} allocated and {} asked for nothing: the governor \
                 reports `OffAir`, which is a slot at REST rather than a slot refused. each \
                 holds this program's one pair at its own seed salt, because a slot cannot \
                 hold nothing and this program has no second pair to give one. it \
                 reaches the mix not at all — but it IS stepped and drawn, into its own \
                 target, every frame, which is what puts running material in its cell and \
                 what ADR-0258 and ADR-0269 ask for; the frame numbers below include both. \
                 REST is about the mix and about what was asked for, and not about whether \
                 the simulation runs: an operator brings one up by cycling its tally or \
                 loading a Set onto it, and what they see in the cell before they do is \
                 what they will get.",
                match resting.len() {
                    1 => "slot",
                    _ => "slots",
                },
                resting.join(" and "),
                match resting.len() {
                    1 => "is",
                    _ => "are",
                },
                match resting.len() {
                    1 => "was",
                    _ => "were",
                },
            );
        }
        println!("what the governor decided, in its own words:");
        println!("  {governed}");
        for decision in governed.parked() {
            println!(
                "  slot {} (deck {}) parked, request held: {:?} — {}, against {}. the \
                 request stands and the slot goes on stepping either way: a park withholds \
                 the grant and not the simulation (ADR-0269)",
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
            "the transition row under the strips IS drawn — a shape, a grid, a length and \
             the `go` that runs a wipe with them — and this window holds what is armed: \
             the three settings are the console's own, and a wipe is converted against \
             them. the crossfader that used to sit over it is not undrawn but \
             gone: the mixer has no crossfader. of the two focuses only one is drawn: the \
             deck selection is a solid ring round a strip, and keyboard focus is not \
             drawn at all because this console takes none — the two must not look alike, \
             which is why the mock's dashed outline is on nothing."
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
                    // **The pills are named because they are controls.** A
                    // bay head's pills are exactly its controls — the table in
                    // `view::REGIONS` says so, and the Program bay's `solo` is
                    // the only one any bay has — so a head with one is a place
                    // a press reaches and a legend that said `bay` would be
                    // hiding it. Read off the table rather than written here.
                    Kind::Bay { pills, grip, .. } => {
                        let mut what = String::from("bay");
                        if grip {
                            what.push_str(", with a grip");
                        }
                        for pill in pills {
                            what.push_str(&format!(", `{pill}` in its head"));
                        }
                        what
                    }
                    // Four readouts, and then the controls that landed in
                    // this row after them: the audio-in pill, the arrangement
                    // pill and, at the far end, the tone map and the exposure.
                    // What the mock draws here and this panel does not is
                    // `view::transport`'s list, and it is not counted here —
                    // this comment said five while that list was down to two,
                    // which is what a total kept beside a list it does not own
                    // is worth. What a press in this row reaches is not
                    // counted here either — see the pointer's paragraph below.
                    Kind::Transport => "row, no heading: bpm, beat, bar, frame".to_owned(),
                    // The console's first control, and for a while its only
                    // one. How many there are now is
                    // `karakuri_console::input::CONTROLS` and is printed
                    // below; a number kept here would be that claim in a
                    // second place, which is the defect this legend is being
                    // repaired of.
                    Kind::Outputs => "row, one sink: program view".to_owned(),
                    Kind::Pane => "pane, inside a bay".to_owned(),
                    Kind::Picture => "the picture, a sink".to_owned(),
                    // Four cells, and how many are on is counted rather than
                    // written: a cell is on because there is a deck slot behind
                    // it, and a strip is a deck slot. Residency does not come
                    // into it — every slot is drawn — which is the correction
                    // this line carries. A constant here is exactly the legend
                    // naming a control's state from before the control existed,
                    // which this row has been twice already.
                    Kind::Previews => {
                        let on = self.view.mixer.len().min(DECKS);
                        format!("{DECKS} previews, {on} of them monitoring a deck slot")
                    }
                    // A bay like the other five: a row of chips saying which
                    // library is being read, and then a row per Set that one
                    // holds — as many as the bay has room for, and the foot
                    // says so. `n of m`, exactly as the bay draws it, and the
                    // scope is the view's own mark rather than a word written
                    // here.
                    Kind::Library => {
                        match karakuri_console::view::library(
                            layout,
                            &self.view.scopes,
                            &self.view.library,
                            self.view.opened(),
                            self.view.pointed(),
                            self.view.library_scroll(),
                        ) {
                            Some(bay) => format!(
                                "bay, {}, {} listed",
                                match self.view.scope() {
                                    Some(scope) => scope.name(),
                                    None => "no scopes",
                                },
                                bay.count()
                            ),
                            None => "bay, nothing said about any library".to_owned(),
                        }
                    }
                    // A bay like the other six, and then a strip per slot:
                    // a strip is a deck slot, and this deck is built full, so
                    // this reads four.
                    Kind::Mixer => format!(
                        "bay, {} strip{}",
                        self.view.mixer.len(),
                        match self.view.mixer.len() {
                            1 => "",
                            _ => "s",
                        }
                    ),
                    // A bay with four rows in it: the level the composited
                    // frame leaves the mix at, and the three fixed passes it
                    // then goes through. **How many of the three are recorded
                    // is what is worth saying**, because an amount of zero is
                    // no pass at all rather than a pass at nothing — see
                    // `karakuri_engine::master`.
                    Kind::Master => match (self.view.master_out, self.view.master_chain) {
                        (Some(out), Some(chain)) => format!(
                            "bay, out {out:.2}, {} of three passes running",
                            usize::from(chain.feedback > 0.0)
                                + usize::from(chain.bloom > 0.0)
                                + usize::from(chain.rgb_shift > 0.0)
                        ),
                        (Some(out), None) => format!("bay, out {out:.2}, no chain behind it"),
                        (None, _) => "bay, no engine behind it".to_owned(),
                    },
                    // A bay like the other five, and then a row per node a
                    // build changed whose verdict is outstanding. **None at
                    // startup, which is where this prints**: nothing has been
                    // rebuilt yet, and empty is this lane's ordinary state —
                    // see `view::staging`.
                    Kind::Staging => match self.view.staging.len() {
                        0 => "bay, nothing waiting".to_owned(),
                        waiting => format!("bay, {waiting} waiting"),
                    },
                    // **The bay whose body is a pattern**, counted off the
                    // pattern rather than written here: how many lanes there
                    // are and how many of them are driving something is what
                    // this bay *is*, and a constant would be the legend naming
                    // a control's state from before the control existed —
                    // which this line has been repaired of twice.
                    Kind::Sequencer => {
                        let pattern = self.sequencer.pattern();
                        let lanes = pattern.lanes().len();
                        let driving = pattern.lanes().iter().filter(|lane| !lane.muted()).count();
                        format!(
                            "bay, bank {} of {}, {} step{}, {lanes} lane{} and {driving} of them \
                             driving",
                            self.sequencer.armed() + 1,
                            karakuri_pattern::BANKS,
                            pattern.mode().count(),
                            match pattern.mode().count() == 1 {
                                true => "",
                                false => "s",
                            },
                            match lanes == 1 {
                                true => "",
                                false => "s",
                            }
                        )
                    }
                },
                None => match layout.axis(node.id) {
                    Some(Axis::Row) => "split, left to right".to_owned(),
                    Some(Axis::Column) => "split, top to bottom".to_owned(),
                    None => "not drawn".to_owned(),
                },
            };
            // **And the class pill where the region draws one**, appended
            // rather than written into each arm: four regions carry one, they
            // are four different `Kind`s, and an arm apiece would be four
            // copies of one sentence — which is the defect this legend was
            // repaired of the last time. What it says is the gate's own words,
            // so the sentence an operator reads here and the sentence a model
            // is refused with cannot drift apart.
            let what = match layout.name(node.id).and_then(class_at) {
                Some(class) => format!(
                    "{what}, `{}` at {} — {}",
                    view::mcp_word(self.view.opening.holds(class)),
                    class.opened_at(),
                    class.title()
                ),
                None => what,
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
             while the picture is on screen. it is one of two kinds of control and a fader \n\
             is the other: the dot names an operation on the ARRANGEMENT, which this crate \n\
             performs, and a fader names one on the MIX, which it cannot — so the operation \n\
             comes out and this file applies it."
        );
        println!();
        // **Whether anything reads these four is read off the server**, not
        // written here. The unconditional sentence this replaces said *nothing
        // in this process serves MCP yet* and printed it on every run,
        // including one whose server `main` had already bound and whose
        // address it had already printed — the same defect as a device named
        // in prose instead of asked of the host, which is why this reads like
        // the `audio-in` line above.
        println!(
            "four of the regions above carry an `mcp` pill, and each says beside its own line \n\
             which state its class is in. each opens one class of operations to a model; every \n\
             operation stays connected either way, and what shut changes is that the call is \n\
             answered with a refusal instead of being performed — one that names the class and \n\
             says which pill opens it. it is the one control on this panel that is neither an \n\
             operation on the ARRANGEMENT nor one on the MIX: it is a setting of the map every \n\
             surface reaches the vocabulary through, and a setting deciding whether a surface \n\
             may reach a class of operations cannot be a member of that class (ADR-0236). \n\
             {}",
            match mcp_port {
                Some(port) => format!(
                    "this process IS serving MCP, on 127.0.0.1:{port}, and \
                     `karakuri_mcp::serve` holds the very handle these four pills \
                     write — not a copy of it — so a pill opened here is read by the server on \
                     the next call it answers, and one shut here refuses the next call in the \
                     class's own words. what they open is live for the length of this run."
                ),
                None => String::from(
                    "no `--mcp` port was given, so nothing in this process serves MCP on this \
                     run and what these four write is read here, by this file's tests, and by \
                     nothing else. the pills still write it, which is the state a run is in \
                     rather than a control that does nothing: `--mcp PORT` is what gives it a \
                     second reader."
                ),
            }
        );
        println!();
        println!(
            "the pointer, and this file no longer keeps a list of what it reaches. a press \n\
             in a gap takes the boundary and it follows the pointer. a press on a LIBRARY \n\
             row takes that Set in hand and nothing happens until you let it go: over a \n\
             mixer strip it loads that deck, anywhere else it loads nothing. that is the \n\
             third way in to the same load `enter` performs in the library, and the only \n\
             one that names both the Set and the deck in one gesture. the second is the \n\
             `load` button in that bay's foot, which lands on the deck the pulldown \n\
             beside it names rather \n\
             than on the deck the keys are addressed to, so a load can be aimed without \n\
             moving the selection (ADR-0305). everywhere else \n\
             `karakuri_console::input::claim` decides, and the {CONTROLS} controls its rule \n\
             4 hit-tests are painted shapes with no widget behind them — nothing but that \n\
             rule knows a press landed on one. the number is that crate's own constant, \n\
             summed over the derivations the rule actually asks, so a control added there \n\
             and not counted is a compile error rather than a sentence that has gone quiet. \n\
             what each of them is, and what a press on it asks for, is written at the \n\
             derivation that draws it: a copy here is precisely how this legend came to \n\
             name three controls while every one of them answered a press."
        );
        println!();
        println!(
            "keys — `tab` and `esc` move the address, drawn as a dashed ring on the bay \n\
             that has it, and the four below them act on whatever that address is on. all \n\
             nine bays have the grammar, and `space` on a bay is the fold wherever you are. \n\
             `g` is addressed to the focused bay too; the letters after it are global:"
        );
        for (key, what) in KEYS {
            println!("  {key:<10}{what}");
        }
        println!();
    }
}

/// Every key this window binds, and the sentence the legend prints for it.
///
/// The list an operator reads and the list the tests check are one list.
/// `key_column::the_keys_this_file_lists_are_the_keys_the_window_loop_binds`
/// reads the `match` in `window_event` out of this file's own text and asserts
/// it is exactly these keys, so a key bound and not printed — or a key printed
/// and not bound — fails there rather than being found by an operator pressing
/// it and getting nothing.
///
/// That test was already here and the legend was a second copy of its list,
/// which is the copy that drifted: the printed list stayed at nine keys while
/// ten more were bound, and the operator who read it was told this program
/// folds, solos, resets and quits.
///
/// The rows of `docs/manual/operations.html` each key reaches are
/// `key_column::ROWS`, which is keyed off this table and stays in the test
/// module: a page heading is what a check reads, and it is not something this
/// program says to anybody.
///
/// The order is the order they print in, and it is the grammar's shape since
/// 2026-09-10: the two keys that move the address, then the four that act
/// inside a bay and the rub-out beside them, then the letters that survive
/// globally.
///
/// `digit` is one entry and not ten, because a digit is one key of the grammar:
/// `1` is the mixer's first strip and the library's first row, so ten entries
/// would print ten sentences saying the same thing and the page would still
/// have to name the bay. It is also the one key
/// [`key_column::bound`](key_column) cannot read out of this file's text — the
/// arm is a guard rather than ten literals — so it is contributed by
/// `karakuri_console::focus::BUILT`, which is the dispatch table the grammar is
/// written in (ADR-0333).
///
/// `p` is the latency offset here and was the report until 2026-08-31. The page
/// specifies the offset as `o` and `p`; a badge naming two keys with one of
/// them bound would be a badge that lies, and a panel diagnostic with no useful
/// shortcut to point at loses the letter rather than keeping it. The operation
/// it named is still `karakuri_console::panel::Op::Report` and nothing in this
/// program asks for it.
///
/// Twelve letters left on 2026-09-10 and five more with the other seven bays,
/// and none of them was retired before the grammar reached its row. `0`–`3` are
/// the Mixer's `1`–`4`, `[ ] \` and `; '` are the arrows and `space` on the
/// addressed trim and fader, `m` is `space` on the blend chip, `e` is `space`
/// on the Library's head and `l` is `enter` on one of its rows (ADR-0259,
/// ADR-0333). Then `f` and `s` went with `Readout::target` — `space` on a bay
/// is the fold and `space` on the Program head's `solo` is the solo — `u` with
/// `s`, because one control is both states, and `o` and `p` became the arrows
/// on the Transport's offset (ADR-0343).
///
/// What is left is the seven globals the operand rule keeps, plus `g`, which is
/// addressed to the focused bay because the grammar reaches no pane, and `k`,
/// which the Library bay draws no control for.
pub(crate) const KEYS: &[(&str, &str)] = &[
    // **The two that move the address**, and they are the same in every bay
    // because they are not addressed to one.
    (
        "tab",
        "focus the next bay, and shift-tab the one before — the ring is the arrangement's own \
         order, down a column and then across",
    ),
    (
        "esc",
        "up one level of the focused bay's address — or, while a name is being typed, abandon \
         the name. it does not quit: close the window",
    ),
    // **The four that act inside a bay**, and they are the same four in every
    // bay because they are rules about kinds of thing rather than about bays
    // (ADR-0259). Which bays have them is
    // `karakuri_console::focus::BUILT` — the mixer and the library today.
    (
        "digit",
        "the nth thing one level below the address, counting what the bay drew from one — and 0 \
         the bay's own head",
    ),
    (
        "up",
        "the neighbour above, or a level's next value — a tenth on the trim and on the fader. \
         the trim is not held at 1.0, because the mix is HDR; the fader is held inside 0 and 1",
    ),
    ("down", "and the neighbour below, or a tenth down"),
    (
        "left",
        "the neighbour to the left, where a bay draws its items in a row — the mixer's strips \
         are one, and naming a strip is the deck selection",
    ),
    ("right", "and the neighbour to the right"),
    (
        "space",
        "the addressed thing's next state — a residency, a blend mode, a mask shape, a library \
         scope — or, on a level, the value it was declared at. on a bay it is the fold, in every \
         one of the nine. a space in a name while the arrangement pill is asking for one",
    ),
    (
        "enter",
        "the act the addressed thing is for: on a library row, load that Set onto the selected \
         deck. it takes the name the arrangement pill is asking for, while it asks",
    ),
    ("backspace", "rub out a letter of that name"),
    // **The one letter addressed to the focus**, which is the third category
    // ADR-0259 creates and ADR-0343 names: its operand is the focused bay, so
    // it is not global under the operand rule and it is not one of the six
    // keys either. It is here because the grammar reaches no pane — `Tab`
    // stops at a bay — and *Fold a pane away* has to be reachable from the
    // keyboard alone.
    (
        "g",
        "fold the split enclosing the focused bay — the pane it sits in. space folds the bay \
         itself",
    ),
    // **The seven that survive as global letters**, plus the one the grammar
    // has not reached yet. A key is global where the operation it names has no
    // operand for focus to supply, or where its only operand is the choice the
    // key itself spells.
    (
        "z",
        "unfold everything folded — the pointer cannot reach one to unfold it",
    ),
    ("r", "reset to a fresh arrangement"),
    ("n", "the room: day or night"),
    (
        "b",
        "tap the beat — three taps set the tempo, any tap sets the phase",
    ),
    (
        ",",
        "halve the grid, and the tracker's octave window with it",
    ),
    (
        ".",
        "double it — refused where the result leaves 60..200 BPM",
    ),
    (
        "k",
        "keep what the selected deck is playing — a Set filed under the time you saved it. the \
         library bay draws no keep control, so the grammar has nothing to reach here yet",
    ),
];

/// A pointer event, stripped to what the rule needs.
///
/// A button is left, or it is the secondary button, or it is not routed at all.
/// This said *a button is left or it is not routed* until 2026-09-09, and it
/// was exact: `window_event` matched `MouseButton::Left` and every other button
/// fell through its `_ => {}`, so a right press reached nothing in this program
/// and `egui` was never told about one either.
///
/// What made it a second variant rather than a field on [`Pointer::Down`] is
/// where the answer is wanted. `karakuri_console::input::claim` decides *whose*
/// an event is from where the pointer is and has never known which button a
/// press was — rules 1 to 4 are all about position — and nothing about that
/// changes: a secondary press on a control the console draws is the console's
/// for the same reason a primary one is. What differs is only what the press
/// then *asks* for, which is this file's half of the seam. A `Down { secondary:
/// bool }` would have put the flag through every arm of the press handler to be
/// read by one of them
/// ([ADR-0311](../../../docs/adr/0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md)).
///
/// There is no `Secondary` release, and that is the whole of what this button
/// does here: a secondary press opens a menu and the gesture ends at the *next*
/// press, which is [`Pointer::Down`]'s or this one's again. Nothing is taken in
/// hand on a secondary press, so there is nothing for a release to let go of.
///
/// The wheel carries a distance now, and only one axis of it. It used to carry
/// nothing, because nothing on this panel did anything with one — *which wheel
/// axis it was does not change who gets it* is what this said, and it was true
/// while the answer was always `egui`'s. An Inspector pane scrolls down its
/// list
/// ([ADR-0307](../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)),
/// so the vertical distance is now the whole of what the event says; a
/// horizontal one reaches nothing here and is dropped where the two are pulled
/// apart, in `window_event`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Pointer {
    Moved(Point),
    Down,
    Up,
    /// A press of the secondary button, which on this panel opens the menu on a row
    /// of the Library bay's list and does nothing anywhere else.
    Secondary,
    /// How far to scroll, in logical pixels, positive down the list — a notch of a
    /// mouse wheel converted to `karakuri_console::room::size::WHEEL_STEP` and a
    /// trackpad's own pixels passed straight through.
    Wheel(f32),
}

/// What routing a pointer event did, beyond deciding whose it was.
///
/// A list rather than an `Option<Outcome>`, because the controls on this panel
/// end in more than one place: the Outputs dot asks for an operation on the
/// *arrangement*, which this crate performs and reports as an [`Outcome`], a
/// fader asks for an operation on the *mix*, which nothing in
/// `karakuri-console` can perform at all, and two of them ask for no operation
/// and are still not nothing. The repaint decision is taken from which of them
/// it is — see `Change::Operated` and `Change::Emitted`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Acted {
    /// Nothing acted: a press on a boundary, a move, a wheel, a release.
    Nothing,
    /// The Outputs dot, and what the operation it named did.
    Operated(Outcome),
    /// A fader translated a drag into the vocabulary, or the drag moved the pointer
    /// over a value that did not change and asked for nothing.
    Emitted(Option<Operation>),
    /// A class pill was pressed, and what it wrote went to the run's `Opening`
    /// rather than to the arrangement or to the deck.
    ///
    /// A fourth answer rather than a reuse of `Nothing`, and the difference is the
    /// whole of ADR-0236: this press is not an operation and must never be made
    /// into one, so it cannot be an `Emitted`; and it is not nothing either,
    /// because a word on the panel changed. See `Readout::opened`.
    ///
    /// It carries no payload because there is none to carry: what changed is held
    /// in the `Opening`, which is a handle another surface reads, and a copy of it
    /// in this enum would be the second answer to *what is open*.
    Opened,
    /// A press moved the library cursor, and asked for nothing ([`Readout::took`]).
    ///
    /// It is the console's one pointer a press moves *without* naming an operation:
    /// the deck selection moves on a press too, and reaches [`Acted::Emitted`]
    /// through `Operation::SelectDeck` and [`pointed`], which is
    /// `Change::Pointed`'s note on the same pair.
    ///
    /// A fifth answer rather than a reuse of [`Acted::Nothing`], which is
    /// [`Acted::Opened`]'s argument one control along: this press names no
    /// operation and must not be made into one (ADR-0265), so it cannot be an
    /// [`Acted::Emitted`]; and it is not nothing either, because the reading
    /// follows the cursor — a move with one open is a read of the row it arrived at
    /// (`karakuri_console::view::View::reading_open`), and a caller that could not
    /// tell this press from a boundary's would leave the block drawn nowhere.
    ///
    /// The caller is what owes that read, and not [`Readout::took`]: a reading is a
    /// file, and the store is the window's rather than the readout's — the division
    /// [`Readout::asked_to_read`] is written to.
    ///
    /// It carries no payload because there is none to carry: it is answered only
    /// where the cursor moved, which is `View::point_at`'s `bool`, and a copy of
    /// the row here would be a second answer to `View::cursor_row`.
    Pointed,
}

pub(crate) fn folding(folded: bool) -> &'static str {
    match folded {
        true => "folded",
        false => "unfolded",
    }
}

/// Which deck, in the letter the preview cells are drawn with — asked of the
/// view rather than written out again here.
///
/// The vocabulary counts decks from zero (`Operation::SetGain { deck: u8 }`)
/// and the console draws them `A` through `D`, so this is the one place the two
/// meet. A deck outside the four cannot be built by anything in this file — a
/// `Deck` holds `MAX_SLOTS` slots and `DECKS` is that number — so an index past
/// the end is a bug and reads as one rather than wrapping quietly.
pub(crate) fn deck_letter(deck: u8) -> &'static str {
    DECK_LETTERS
        .get(usize::from(deck))
        .copied()
        .unwrap_or("(no such deck)")
}

/// Which fader, in the bay's own word for it.
pub(crate) fn knob_word(knob: &Knob) -> &'static str {
    match knob {
        Knob::Trim { .. } => "trim",
        Knob::Fader { .. } => "fader",
        Knob::Out => "out",
        Knob::Feedback { .. } => "feedback",
        Knob::Bloom => "bloom",
        Knob::RgbShift => "rgb shift",
        Knob::Param { .. } => "parameter",
    }
}

/// Whose fader it is, for a line a reader has to place: a deck by its letter,
/// and the master out by the bay it is in.
///
/// The master out names no deck — it is one level on the whole fold
/// ([ADR-0224](../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md))
/// — so a line that said *deck A* over it would be naming a slot nothing in the
/// gesture ever touched. `Knob::deck` is what answers, and this is the only
/// caller: everything that *acts* takes the deck out of the operation.
pub(crate) fn knob_where(knob: &Knob) -> String {
    match knob.deck() {
        Some(deck) => format!("deck {}", deck_letter(deck)),
        None => "master".to_owned(),
    }
}
