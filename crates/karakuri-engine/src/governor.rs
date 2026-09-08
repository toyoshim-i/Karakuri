//! The budget governor: what may prime, and how fast.
//!
//! The load-bearing question left open before this was built was whether GPU
//! timestamps work on the performing machine, because a governor steering on a
//! host clock reacts to submission overhead as much as to shader cost. The
//! decision this module is built on
//! (`docs/adr/0054-the-governor-budgets-from-the-probe-and-never-touches-a-live-slot.md`):
//!
//! > **The governor budgets against a per-Set cost measured by
//! > [`crate::probe`] when the Set is built** — not against live GPU timestamps
//! > on the frame path, and not against the deck's frame interval.
//!
//! Three reasons, all of them already written down in this repository before
//! this module existed:
//!
//! 1. **Live GPU timestamps are not available here.** `probe.rs` says at length
//!    that on this machine `TIMESTAMP_QUERY` is advertised, enabled, and
//!    unreliable — an enormous workload resolves to zero, to a negative delta,
//!    or occasionally to something plausible. Flaky is worse than absent: a
//!    governor that reads 0.0 ms for a heavy Set promotes everything. `Probe`
//!    already calibrates against a known-heavy workload rather than trusting
//!    the feature flag, and labels every number with
//!    [`MeasurementMethod`](crate::probe::MeasurementMethod). Using it is how
//!    this module gets an answer to that open question instead of assuming one.
//! 2. **A frame interval cannot be divided among the slots that produced it.**
//!    That is not a hypothetical either — it is the defect a review already
//!    found in the swap watchdog and `swap.rs` records verbatim: "every Live
//!    slot in a deck is judged against the whole deck's frame interval, so a
//!    budget that fits one Set rolls back every candidate in a deck of four."
//!    A governor deciding *which* slots may prime needs per-slot numbers by
//!    construction.
//! 3. **There is already somewhere to measure that is not the render thread.**
//!    `swap.rs`'s worker does a submit-and-wait before handing a Set over. The
//!    probe's method is submit-and-wait; it fits there and nowhere else.
//!
//! So the worker probes a Set as part of building it, the measurement travels
//! with the Set on the same channel, and this module sums what it is about to
//! ask for and decides. What that measurement is *of*, and the four things it
//! cannot see — resolution, the deck's own per-frame cost, the future, and
//! host-side overhead on the fallback path — are stated once, in "The worker
//! also measures what it built" in [`crate::swap`], rather than restated here.
//!
//! ## Requested and effective
//!
//! The distinction this module is built around, defined once in
//! [`crate::deck`]'s "Residency: requested and effective" and used here without
//! restating it: a slot's **requested** residency is what the operator asked
//! for and only the operator changes it; its **effective** residency is what
//! the engine is doing right now. This module reads the first and computes the
//! second, every pass, from the request and the budget. It holds no memory of
//! either.
//!
//! That is what makes a refusal a **park** rather than a cancellation. A slot
//! whose request is Priming and whose effective residency is Allocated is
//! waiting for room, is reported as [`Decision::is_parked`], and primes again
//! by itself on the first pass where the arithmetic works — the deck emptying
//! is enough, and no operator action is needed or expected. Writing a demotion
//! back over the request instead would destroy the one fact this module is not
//! entitled to change, and the operator would have to ask twice for something
//! they never withdrew.
//!
//! ## What it decides
//!
//! Deliberately two things, and no more:
//!
//! - **Which slots are effectively [`Priming`](crate::deck::Residency::Priming)
//!   at all**, out of those requesting it.
//!
//! It used to decide a second thing — **how fast each of those steps**, as one
//! step every *n* frames — and that is gone.
//! `docs/adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md`
//! steps every drawn slot on every frame, every slot is drawn, and a rate that
//! applies to no slot is not a rate. What a park now changes is the word on the
//! strip and not the work in the frame: an off-air slot steps either way, so
//! this module decides whether a request was **granted** rather than whether a
//! simulation runs.
//!
//! ## Two numbers, and which one is budgeted on
//!
//! **A slot can arrive here with two costs, and they are not the same
//! quantity.**
//!
//! - [`SlotState::cost`] is [`crate::swap::measure`]'s: **one** draw, at
//!   whatever size the caller named through
//!   [`Deck::set_measure_size`](crate::deck::Deck::set_measure_size), taken
//!   when the Set was built. Comparing two of them is the admission decision
//!   this module was built on, and they are comparable because every slot on a
//!   deck is told the same size.
//! - [`SlotState::estimate`] is [`crate::estimate`]'s: a fit of `a + b·area`
//!   through **two** draws, evaluated at the **output's** size. It is the same
//!   material read at the size the frame is actually paying for.
//!
//! **Since ADR-0303 those two can be about different frames, and this module
//! sums them.** The size the measurement is taken at was a constant 1280x720
//! until then and is now the caller's — `karakuri`'s window names its deck
//! preview cell, which is 252x142 there. A deck where one slot has an estimate
//! and another has fallen back to its measurement adds an output-size figure to
//! an audition-size one and holds the total against one budget, which is
//! exactly what ADR-0015 exists to prevent. **It is not fixed here**, because
//! which way it should go is a decision rather than a defect with one repair:
//! the measurement could be extrapolated (which ADR-0266 measured and refused
//! as a rule), or stop being a budget input where an estimate is possible, or
//! the budget could be stated per size. ADR-0303 records the finding and leaves
//! the ruling to the maintainer. [`Decision::budgeted_ms`] carries the number
//! and [`Decision::basis`] says which of the two it was, so a consumer can
//! already tell them apart — what it cannot yet tell is that a sum contained
//! both.
//!
//! **Where the estimate answers, it is the number.** The reason is the one
//! ADR-0246 gives for making the render size the output's: a deck drawing into
//! a 640x360 window budgeted against 720p figures is refusing priming on
//! fragment work nobody is doing, and a deck on a 4K output is admitting
//! against a number several times too small. The measurement cannot tell those
//! apart, because one draw gives `a + b·area` and no way to divide it — which
//! is the whole of [`crate::estimate`]'s first line.
//!
//! **Where it refuses, nothing changes.** A refusal is not a licence and not a
//! zero: the slot falls back to its measurement and is decided exactly as it
//! was before any of this existed, and a slot with neither number is
//! [`Reason::Unmeasured`] — parked if it asked to prime, and
//! [`Reason::CommittedUnknown`] for the whole deck if it is on air. That is the
//! same rule as "Why an unmeasured Set is not a free one" below, reached from a
//! second direction: **an instrument that declined to answer has not said the
//! answer is small.**
//!
//! **And it says what it did.** [`Decision::basis`] is which of the two the
//! arithmetic used, [`Decision::budgeted_ms`] is the number itself, and
//! [`Decision::estimate`] carries the estimate's own record — the target it
//! was for, the instrument, the floor, **where that floor came from**
//! ([`Estimated::floor_from`]) and **how strictly it was read**
//! ([`Estimated::floored`]). `P-0095` is why both of the last two travel: an
//! estimate taken with every primitive at least a pixel across at both rungs
//! and one taken with some of them rounded up are not the same statement, and
//! a consumer that cannot see which it has cannot check the number. What this
//! module *does* with them is: nothing arithmetic — [`Fit::ms`](crate::estimate::Fit::ms)
//! already carries [`Floored::correction`], so applying it again would inflate
//! a number that is already sound — and everything reportorial: the counts are
//! on [`Report`] and the [`Display`](std::fmt::Display) line says them.
//!
//! **It does not re-derive a floor and it does not refuse one.** A
//! [`FloorRead::Stated`] floor is a caller's assertion nothing checked and a
//! [`FloorRead::Contradicted`] one is a bound a held value falsified; ADR-0285
//! makes the first a record rather than a refusal, and ADR-0293 §6 makes the
//! second *the greatest floor there is* — placed and paid for, not refused. So
//! both are numbers this module spends, and both are numbers it names.
//!
//! ## What it does not touch
//!
//! **Live slots. Ever.** An operator who puts four heavy Sets on air has made a
//! decision, and a governor that took a slot off air mid-set would be the worst
//! behaviour available to it — worse than the dropped frames it was trying to
//! prevent, because the operator can see dropped frames and cannot see a
//! reason. So a deck whose Live slots already exceed the budget gets a
//! [`Report::over_budget`] flag and nothing else happens to it. It **warns**;
//! it does not act. That is the same order this repository already takes with
//! the level meter: show the number first, and decide later whether anything
//! should move by itself.
//!
//! It also does not **promote past the request**. Asking for a slot to be
//! primed is the operator's (later, an agent's) move, made through
//! [`Deck::set_residency`](crate::deck::Deck::set_residency); this module
//! answers yes, yes-but-slower, or not yet. A governor that put slots into
//! Priming nobody asked for would be deciding what material is worth
//! preparing, which is a judgement about the set rather than about the budget.
//! Restoring a slot it parked earlier is not that: the request was there the
//! whole time.
//!
//! And it does not budget **VRAM**. The deck's size is two budgets — VRAM
//! bounding how many Sets can be Allocated at all, compute bounding how many
//! can step
//! (`docs/adr/0026-the-deck-is-l5s-surface-and-its-size-is-two-budgets.md`) —
//! and only the second is buildable today. Bounding VRAM means
//! being able to refuse an allocation and to free one on demand, which needs an
//! allocator's cooperation that does not exist in this engine: a Set's buffers
//! are created by `Set::build` and released by dropping it, with nothing in
//! between to consult. Half of it here would be a number in a status line that
//! nothing enforces.
//!
//! ## Closed-form Sets are not primed, they are skipped
//!
//! `docs/ir-spec.md`'s "Closed form versus accumulating": a procedure that
//! never reads an attribute it emits is a pure function of `seed`, `t`, and its
//! parameters, so any `t` can be evaluated directly. There is no state to warm.
//! Priming one would spend compute budget to arrive at a state it was already
//! going to be in, and — worse — would spend it *instead of* a slot that
//! genuinely needed it. So a closed-form slot asked to prime is parked with
//! [`Reason::NoPrimingNeeded`], which is not a refusal: it can go straight to
//! Live whenever the operator wants it.
//!
//! The check pass decides that property and records it on `Checked`; this
//! module reads it off the Set and never looks at IR. It is conservative in the
//! direction that costs a warm-up rather than the one that shows an unwarmed
//! image — see `is_closed_form` in `karakuri-ir`.
//!
//! Skipping priming is the smaller half of what that property is for. The
//! larger one is that closed-form material can be **scrubbed** — run forwards
//! at any rate, held, or backwards — which is what a tape-style transport locked
//! to the beat grid needs. Nothing here builds that; it is named so that the
//! flag is not mistaken for a governor-internal hint that could be relaxed. A
//! seek needs strictly more than this flag guarantees; `Checked::closed_form`
//! says what.
//!
//! ## Why an unmeasured Set is not a free one
//!
//! A Set nobody measured gets [`Reason::Unmeasured`] and is parked rather than
//! primed. Treating `None` as zero would make the *least* known Set the
//! cheapest thing on the deck and the first thing admitted, which inverts the
//! whole point.
//!
//! **The same rule applies one field over, to the Live slots.** An unmeasured
//! Live slot means the deck's committed cost is *unknown*, and unknown is not
//! headroom: a deck with one unmeasured Live slot and a 16 ms budget would
//! otherwise admit a 16 ms candidate at full rate against an on-air cost nobody
//! has a number for. So priming is refused wholesale while any Live slot is
//! unmeasured — [`Reason::CommittedUnknown`], which is deliberately a different
//! reason from [`Reason::NoHeadroom`] because "there is no room" and "I cannot
//! tell whether there is room" call for different actions. `headroom_ms` is
//! `None` in that state for the same reason: a headroom figure computed from an
//! understated commitment is a number pretending to be a budget.
//!
//! The default this lands on is that **nothing primes until the Live Sets have
//! been measured**, because every [`HotSwap::fixed`](crate::swap::HotSwap::fixed)
//! Set and every Set [`HotSwap::new`](crate::swap::HotSwap::new) is constructed
//! with arrives unmeasured. That is the right rule with the wrong default, and
//! the fix belongs on the other side: a Set should be measured before it goes
//! on air. [`Deck::measure_slots`](crate::deck::Deck::measure_slots) is the one
//! call that does it, at startup, where a stall is free.
//!
//! ## Determinism
//!
//! Slots are visited in index order and every input is engine state — a
//! measurement taken at build time, a residency, a flag off the IR. Nothing
//! here reads a clock or a frame time, so two runs of the same record stream
//! reach the same decisions in the same frames, which is what keeps priming
//! inside the determinism invariant rather than beside it.

use crate::deck::Residency;
use crate::estimate::{Estimate, Floor, Floored, Unfit};
use crate::probe::{Measurement, MeasurementMethod};

/// A default compute budget, in milliseconds of measured per-Set cost.
///
/// **Not [`crate::swap::DEFAULT_BUDGET_MS`]**, and the two must not be confused
/// even though both are milliseconds and both default near a 60 Hz frame. That
/// one is a *frame interval* on a host clock and answers "are frames still
/// arriving"; this one is a sum of per-Set measurements at a fixed reference
/// resolution and answers "is there room to step one more simulation". They are
/// not comparable and neither is derivable from the other.
///
/// 16.7 ms is one 60 Hz frame, which is the rate `dt` sets. There is no slack
/// added on top the way `DEFAULT_BUDGET_MS` adds it, because there is no vsync
/// quantisation to clear here — a sum of measurements is not a frame interval.
pub const DEFAULT_COMPUTE_BUDGET_MS: f32 = 16.7;

/// **Where an estimate's floor came from**, as much of
/// [`Floor`](crate::estimate::Floor) as survives being `Copy`.
///
/// The floor is what both rungs were placed against, so `P-0095` makes it part
/// of the number rather than a detail behind it — a consumer that cannot see
/// how it was arrived at cannot check the rungs either. The full record,
/// including one `RateBound` per renderer and the declarations each rests on,
/// stays on the [`Estimate`] the slot is holding
/// ([`HotSwap::estimated_cost`](crate::swap::HotSwap::estimated_cost)); this is
/// the discriminant, so a status line can say which of the three it has without
/// going and fetching it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FloorRead {
    /// The caller stated the floor and nothing checked it — ADR-0285's
    /// `Floor::Stated`. A record, not a refusal: getting it wrong low is the
    /// failure ADR-0245 describes, and this says nothing stood between the
    /// caller and it.
    Stated,
    /// Read off the Set's own `point_rate` expressions, with no held value
    /// contradicting a bound.
    Analysed,
    /// Read off the Set's own expressions, and **a held value falsified one of
    /// them** — somebody wrote a param outside the declaration a bound was
    /// taken over. ADR-0293 §6 makes that an *unknown* floor rather than a
    /// refusal, so [`Estimated::floor`] is `None` and the rungs were placed
    /// against the greatest floor there is.
    Contradicted,
}

/// **What an estimate says, as this module reads it** — every field of
/// [`Estimate`] a budget decision or a status line needs, and nothing that
/// would make it allocate.
///
/// [`Estimate`] itself carries a `Vec` of topologies and a `Vec` of rate
/// bounds, and a governor pass is a `Copy` of small things by construction.
/// So the record is summarised here and **not thrown away**: the whole of it is
/// still on the slot, and this names which of it was read.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Estimated {
    /// **The number, or why there is none** — [`Estimate::ms`] and
    /// [`Estimate::fit`]'s refusal, with the fit's other three terms left
    /// behind because a budget spends the answer and not the line.
    ///
    /// An `Err` here is what makes this slot fall back to its measurement.
    pub fit: Result<f32, Unfit>,
    /// The size the estimate answered for — the output's, not a probe's. Kept
    /// so a report can say what the number is *of*: an estimate for a target
    /// the deck is no longer drawing is a right number about the wrong frame,
    /// which is why [`HotSwap::resize`](crate::swap::HotSwap::resize) drops
    /// one.
    pub target: (u32, u32),
    /// Which clock took the two rungs, or [`None`] where nothing was drawn.
    /// [`MeasurementMethod::HostWallClock`] biases the answer high, and
    /// [`Report::host_clock`] is where that reaches a caller.
    pub method: Option<MeasurementMethod>,
    /// The sub-pixel floor the rungs were placed against, in rows, or [`None`]
    /// where it was not knowable at all — which ADR-0293 places rather than
    /// refuses.
    pub floor: Option<u32>,
    /// **Where that floor came from.** See [`FloorRead`].
    pub floor_from: FloorRead,
    /// **How strictly that floor was read.** [`None`] means both rungs cleared
    /// it, nothing was rounded up and the fit is exact. [`Some`] carries the
    /// share ADR-0245's flooring can hide and the factor
    /// [`Fit::ms`](crate::estimate::Fit::ms) was **already** multiplied by to
    /// cover it — so this module spends the number as it stands and never
    /// applies the correction a second time.
    pub floored: Option<Floored>,
}

impl Estimated {
    /// The number, where there is one.
    pub fn ms(&self) -> Option<f32> {
        self.fit.ok()
    }

    /// Whether [`Fit::ms`](crate::estimate::Fit::ms) carries a correction for a
    /// rung that sat under the floor. A corrected number is sound and is
    /// higher than the line it was fitted from — at most `1/(1 - 1/4)` of it,
    /// which is one band of the badge it feeds.
    pub fn corrected(&self) -> bool {
        self.floored.is_some()
    }
}

impl From<&Estimate> for Estimated {
    fn from(e: &Estimate) -> Estimated {
        Estimated {
            fit: e.fit.as_ref().map(|f| f.ms).map_err(|u| *u),
            target: e.target,
            method: e.method(),
            floor: e.floor,
            floor_from: match &e.floor_from {
                Floor::Stated => FloorRead::Stated,
                Floor::Analysed {
                    contradicted: Some(_),
                    ..
                } => FloorRead::Contradicted,
                Floor::Analysed { .. } => FloorRead::Analysed,
            },
            floored: e.floored,
        }
    }
}

/// **Which of a slot's two numbers the arithmetic was done on.**
///
/// Not decoration: the two are measurements of different things — one draw at
/// whatever size the caller named
/// ([`Deck::set_measure_size`](crate::deck::Deck::set_measure_size)) against a
/// fit at the output's size — so a caller comparing a `budgeted_ms` against
/// anything has to know which it is holding, and since ADR-0303 the sizes can
/// differ by more than an order of magnitude. See "Two numbers, and which one is budgeted
/// on" in the module doc.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Basis {
    /// **Neither number.** Nothing measured this Set and nothing estimated it,
    /// or an estimate was taken and refused with no measurement behind it.
    /// [`Decision::budgeted_ms`] is `None` and the slot is unbudgetable — which
    /// is not the same as free.
    Unbudgetable,
    /// The single-draw measurement at the reference resolution, because
    /// nothing estimated this slot or the estimate refused.
    /// [`Decision::estimate`] says which, and why.
    Measured,
    /// The two-draw fit, at the size on [`Estimated::target`].
    Estimated,
}

/// Why a slot ended up where it did.
///
/// The first three describe a slot doing what was asked of it. The last four
/// describe a **park**: the request is Priming, the effective residency is
/// Allocated, and this is what has to change for that to stop being true.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// Live. Not the governor's to touch — see the module doc.
    OnAir,
    /// Priming, within the headroom. **At the room's tempo**, which is the
    /// only rate there is: every slot steps every frame, so a granted request
    /// costs what the slot was already costing.
    Fits,
    /// Allocated, and that is what was asked for. The governor was not asked
    /// about this slot and did nothing to it.
    OffAir,
    /// Parked: the headroom left after the Live slots will not take this
    /// slot's measured cost.
    ///
    /// **The request is untouched**, so this is "not now" rather than "no": the
    /// next pass over a deck with room admits it with nothing else changing.
    ///
    /// **What it does not mean is that the slot stops.** It steps every frame
    /// whatever this says, because it is drawn every frame (ADR-0269). A park
    /// withholds the grant, not the simulation, and an operator sees it on the
    /// strip rather than in the cell.
    NoHeadroom,
    /// Parked: **no number**, so it cannot be budgeted for. Not a claim that
    /// it is expensive — a claim that its cost is unknown, which is not the
    /// same as zero.
    ///
    /// Two ways to arrive: nothing measured this Set and nothing estimated it,
    /// or an estimate was taken and **refused** with no measurement behind it
    /// to fall back to. [`Decision::estimate`] tells them apart and carries the
    /// [`Unfit`] where there is one — an instrument declining to answer has
    /// not said the answer is small.
    Unmeasured,
    /// Parked: a **Live** slot on this deck has no number — neither a
    /// measurement nor an estimate that answered — so what the deck is already
    /// spending is unknown and there is no headroom figure to admit against.
    ///
    /// Deliberately distinct from [`Reason::NoHeadroom`]: that one says there
    /// is no room, this one says the governor cannot tell whether there is.
    /// They call for different actions — one waits for a slot to come off air,
    /// the other for [`Deck::measure_slots`](crate::deck::Deck::measure_slots).
    CommittedUnknown,
    /// Parked: the Set is closed form and has no state to warm. It can go Cold
    /// to Live whenever it is wanted.
    ///
    /// The one park that no amount of budget resolves — but it is still a park
    /// rather than a cancellation, because what it turns on is the Set and a
    /// build landing on this slot can change it.
    NoPrimingNeeded,
}

/// What the governor decided about one slot.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Decision {
    pub slot: usize,
    /// What the operator asked for. **Passed through untouched** — the
    /// governor never writes this — so a caller reading a report can always
    /// tell what was wanted from what is happening.
    pub requested: Residency,
    /// What the engine should be doing with this slot until the next pass.
    pub effective: Residency,
    pub reason: Reason,
    /// What this slot was **measured** at — one draw at the size the deck was
    /// told to measure at, which travels on
    /// [`Measurement::resolution`](crate::probe::Measurement::resolution) — if
    /// anything measured it.
    ///
    /// **No longer necessarily the number the arithmetic was done on**, which
    /// is [`Decision::budgeted_ms`]. It is kept unchanged and beside it because
    /// the two are different quantities and a status line showing an estimate
    /// where it means a measurement would be overstating what was taken.
    pub cost_ms: Option<f32>,
    /// **The number this slot was budgeted at**, and the one the headroom
    /// arithmetic above spent. [`None`] where there was neither.
    ///
    /// Equal to [`Decision::cost_ms`] under [`Basis::Measured`] and to
    /// [`Estimated::ms`] under [`Basis::Estimated`].
    pub budgeted_ms: Option<f32>,
    /// **Which of the two it is.** See [`Basis`].
    pub basis: Basis,
    /// **The estimate this slot arrived with**, answering or refusing, or
    /// [`None`] where nothing estimated it.
    ///
    /// Present under [`Basis::Measured`] too, and that is the point: an
    /// estimate that refused is why the measurement is being used, and
    /// [`Estimated::fit`]'s `Err` names what would have to change. Dropping it
    /// there would leave a slot that fell back looking like one nothing had
    /// ever asked.
    pub estimate: Option<Estimated>,
}

impl Decision {
    /// **Asked to prime, and not priming.** The request stands and is
    /// reconsidered every pass; [`Decision::reason`] is what it is waiting on.
    ///
    /// This is the whole observable difference between a park and a
    /// cancellation, so a status line that shows residencies and not this is
    /// showing the operator a slot that looks like one they turned off.
    pub fn is_parked(self) -> bool {
        self.requested == Residency::Priming && self.effective == Residency::Allocated
    }
}

/// One pass of the governor over one deck.
///
/// **Every millisecond in here was taken once, off a *cold* Set, and nothing
/// re-measures.** A Set whose population grows, whose points get bigger, or
/// whose overdraw rises as it spreads costs more than this the longer it runs
/// — see "What it cannot see" in [`crate::swap`] for the whole list. The
/// numbers are comparable *between* slots, which is what an admission decision
/// needs; they are not a prediction of this machine's frame time, and a status
/// line that presents them as one is overstating them.
///
/// **Two of that list's four are answered for an estimated slot and two are
/// not.** Resolution is: [`Basis::Estimated`] means the figure is a fit
/// evaluated at the output's own size rather than a draw at 1280x720, so a deck
/// on a 4K output is no longer being budgeted several times too small. So is
/// the host-side bias, in the sense that it is *named* —
/// [`Estimated::method`], and [`Report::host_clock`] either way. The deck's own
/// per-frame cost and the future are untouched: an estimate is still one
/// reading of one cold Set, and nothing takes a second.
///
/// [`Report::host_clock`] is the caveat the [`Display`](std::fmt::Display) line
/// carries because it is the one that can change from run to run;
/// [`Report::estimated`], [`Report::corrected`] and [`Report::floor_unknown`]
/// are on it for `P-0095`'s reason — which of two quantities a number is, and
/// how strictly the floor under it was read, are facts about the number and not
/// about the deck.
#[derive(Clone, Debug)]
pub struct Report {
    /// Every slot, in index order.
    pub decisions: Vec<Decision>,
    /// The budget these decisions were made against.
    pub budget_ms: f32,
    /// What the Live slots are already budgeted at, summed — each at
    /// [`Decision::budgeted_ms`], so an estimated slot contributes its estimate
    /// at the output's size and a merely measured one contributes its
    /// reference-resolution draw. **Understates by `unmeasured_live` Sets'
    /// worth** — see that field.
    ///
    /// **A sum of two kinds of number where a deck is part-estimated**, and
    /// that is the honest reading rather than a defect: each term is this
    /// crate's best statement about that slot, and [`Report::estimated`] says
    /// how many of them are the better kind. Refusing to mix them would mean
    /// discarding the estimate on a deck where one slot happens to lack one.
    pub committed_ms: f32,
    /// What the slots left Priming add on top, summed at their whole budgeted
    /// cost — there is no rate to amortise over any more (ADR-0269).
    ///
    /// **It is not what the deck spends off air**, and the difference grew
    /// when every slot started stepping: a parked slot and a slot nobody asked
    /// about cost the same as a granted one and appear in neither this nor
    /// [`Report::committed_ms`]. Both fields say what they say — what is on
    /// air, and what priming was granted — and what the deck spends in total
    /// is the per-slot [`Decision::cost_ms`] summed by whoever wants it.
    pub priming_ms: f32,
    /// Live slots with **no number at all** — neither a measurement nor an
    /// estimate that answered. `committed_ms` cannot include them, so
    /// a non-zero value here means the deck's committed cost is **unknown**
    /// rather than merely approximate: [`Report::headroom_ms`] is `None`, and
    /// every slot asking to prime is parked with
    /// [`Reason::CommittedUnknown`].
    pub unmeasured_live: usize,
    /// **The warning, and the only one.** The Live slots alone are measured
    /// above the budget. Nothing was done about it: see "What it does not
    /// touch" in the module doc.
    ///
    /// A lower bound rather than an estimate, and it survives
    /// `unmeasured_live` being non-zero for that reason: an unmeasured slot can
    /// only add to what is committed, so measured-alone-over-budget is still
    /// over budget.
    pub over_budget: bool,
    /// Whether any number this report saw came from a host clock rather than
    /// from GPU timestamps — a slot's measurement, or the two rungs an
    /// estimate was fitted through. Every number in the report reads coarser
    /// and biased high when this is true, and on this crate's development
    /// machine it always is.
    ///
    /// **Both sources, because an estimate inherits the bias whole**: a host
    /// measurement brackets a submit-and-wait the GPU never spent, which does
    /// not move with the target, so it lands in the fit's invariant term and is
    /// added to the answer once. See [`Estimate::biased_high`].
    pub host_clock: bool,
}

impl Report {
    /// Budget left after the Live slots, before anything primes. Negative when
    /// [`Report::over_budget`].
    ///
    /// **`None` when the committed cost is not known** — that is, whenever a
    /// Live slot is unmeasured. An `Option` rather than an optimistic number
    /// because the optimistic number is the defect: `budget - committed` on a
    /// deck whose commitment understates by an unknown amount reads as
    /// confident headroom and gets spent, and a caller asking
    /// `if headroom > x` has no way to notice. Ask
    /// [`Report::committed_known`] first, or handle the `None`.
    pub fn headroom_ms(&self) -> Option<f32> {
        self.committed_known()
            .then_some(self.budget_ms - self.committed_ms)
    }

    /// Whether every Live slot has a measurement, and therefore whether
    /// [`Report::committed_ms`] is the whole committed cost rather than a lower
    /// bound on it.
    pub fn committed_known(&self) -> bool {
        self.unmeasured_live == 0
    }

    /// Slots asked to prime that are not priming — see [`Decision::is_parked`].
    /// Each one still holds its request and is reconsidered next pass.
    pub fn parked(&self) -> impl Iterator<Item = &Decision> {
        self.decisions.iter().filter(|d| d.is_parked())
    }

    /// **How many slots were budgeted on an estimate** rather than on a
    /// reference-resolution measurement. Zero on a deck nothing has estimated,
    /// which is every deck until
    /// [`Deck::estimate_slots`](crate::deck::Deck::estimate_slots) runs.
    pub fn estimated(&self) -> usize {
        self.decisions
            .iter()
            .filter(|d| d.basis == Basis::Estimated)
            .count()
    }

    /// **How many of those numbers carry a correction for a rung that sat
    /// under ADR-0245's sub-pixel floor.** `P-0095`, reported rather than
    /// silently spent: a corrected number is sound and is up to a third above
    /// the line it was fitted from, which is one band of the badge it feeds.
    pub fn corrected(&self) -> usize {
        self.decisions
            .iter()
            .filter(|d| d.basis == Basis::Estimated && d.estimate.is_some_and(|e| e.corrected()))
            .count()
    }

    /// **How many were placed against a floor nothing could establish.**
    /// ADR-0293 §6 reads an unknown floor as the greatest floor there is, so
    /// these are answers and not refusals — but they are answers taken at the
    /// widest placement, and a reader is owed the distinction.
    pub fn floor_unknown(&self) -> usize {
        self.decisions
            .iter()
            .filter(|d| {
                d.basis == Basis::Estimated && d.estimate.is_some_and(|e| e.floor.is_none())
            })
            .count()
    }

    /// **Slots that were estimated and whose estimate refused**, with the
    /// refusal. Each fell back to its measurement, or to
    /// [`Reason::Unmeasured`] where there was none — the estimate declining is
    /// never what admits a slot.
    pub fn refused_estimates(&self) -> impl Iterator<Item = (usize, Unfit)> + '_ {
        self.decisions
            .iter()
            .filter_map(|d| d.estimate.and_then(|e| e.fit.err()).map(|u| (d.slot, u)))
    }

    /// The size every estimate in this report answered for, or [`None`] where
    /// none did. `Some` is the ordinary case — one deck, one output — and two
    /// different targets in one report would mean a slot kept an estimate
    /// across a resize, which [`HotSwap::resize`](crate::swap::HotSwap::resize)
    /// is what prevents.
    pub fn estimated_target(&self) -> Option<(u32, u32)> {
        self.decisions
            .iter()
            .filter(|d| d.basis == Basis::Estimated)
            .find_map(|d| d.estimate.map(|e| e.target))
    }
}

impl std::fmt::Display for Report {
    /// One line, for a status line to print. States what it measured on,
    /// because a budget figure whose provenance is not visible is the exact
    /// shape of number this repository keeps refusing to produce.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "governor: {:.2} / {:.2} ms live",
            self.committed_ms, self.budget_ms
        )?;
        if self.priming_ms > 0.0 {
            write!(f, " + {:.2} priming", self.priming_ms)?;
        }
        if self.unmeasured_live > 0 {
            write!(f, " (+{} unmeasured)", self.unmeasured_live)?;
        }
        write!(
            f,
            " [{}]",
            if self.host_clock {
                "host clock"
            } else {
                "gpu timestamps"
            }
        )?;
        // **What the numbers are, said where the numbers are.** A sum that is
        // partly estimates at the output's size and partly measurements at
        // 1280x720 is two quantities added together, and a status line that
        // did not say so would be presenting one. The corrected and
        // unknown-floor counts are `P-0095`'s half of it: an estimate taken
        // under a floored rung is not the same statement as one taken clear of
        // it, and this is where the difference reaches a reader who is not
        // going to go and read `Estimate::floored` themselves.
        let mut said = false;
        if let Some((w, h)) = self.estimated_target() {
            write!(f, " — {} estimated at {w}x{h}", self.estimated())?;
            said = true;
            let corrected = self.corrected();
            if corrected > 0 {
                write!(f, ", {corrected} corrected for a floored rung")?;
            }
            let unknown = self.floor_unknown();
            if unknown > 0 {
                write!(f, ", {unknown} on an unknown floor")?;
            }
        }
        // **A refusal is said out loud too**, and on a deck where every
        // estimate refused it is the only thing there is to say: the numbers
        // above are then measurements at the reference resolution, and a
        // reader who was told an estimate had been taken would otherwise have
        // no way to tell.
        let refused = self.refused_estimates().count();
        if refused > 0 {
            write!(
                f,
                "{}{refused} refused, budgeted on the measurement",
                if said { ", " } else { " — " }
            )?;
        }
        if self.over_budget {
            write!(
                f,
                " — OVER: live slots exceed the budget, priming suspended"
            )?;
        } else if !self.committed_known() {
            // Said in words as well as in the "+N unmeasured" above, because
            // the two facts are different: that one is a count, this one is
            // the consequence — and the consequence is the whole of why
            // nothing is priming.
            write!(
                f,
                " — UNKNOWN: the committed cost cannot be known while a live slot \
                 is unmeasured, priming suspended"
            )?;
        }
        Ok(())
    }
}

/// One slot, as the governor sees it. The deck fills this in; nothing here
/// knows what a `Set` is.
#[derive(Clone, Copy, Debug)]
pub struct SlotState {
    /// **What the operator asked for**, not what the slot is currently doing.
    /// A pass is computed from the request every time, so a slot parked last
    /// pass is reconsidered from scratch this one — which is what makes a park
    /// recover by itself. Handing the effective residency in here instead would
    /// feed a demotion back into the next pass's input and make it permanent.
    pub requested: Residency,
    /// What the Set in this slot was measured at, if anything measured it —
    /// one draw at the size the deck was told to measure at, which is on
    /// [`Measurement::resolution`](crate::probe::Measurement::resolution).
    pub cost: Option<Measurement>,
    /// **What two draws say it would cost at the output's size**, if anything
    /// estimated it. Preferred over [`SlotState::cost`] where it answers; a
    /// refusal falls back to it. See "Two numbers, and which one is budgeted
    /// on" in the module doc.
    ///
    /// [`Estimated::from`] builds one from the [`Estimate`] a slot holds.
    pub estimate: Option<Estimated>,
    /// Whether the Set can be jumped to any `t` — see the module doc.
    pub closed_form: bool,
}

impl SlotState {
    /// **The number to budget this slot at, and where it came from.**
    ///
    /// One rule, used by the committed sum and by the admission test alike,
    /// so the two cannot disagree about which number a slot is being judged
    /// on. The estimate wins where it answers; a refusal, or no estimate at
    /// all, falls back to the measurement; neither is
    /// [`Basis::Unbudgetable`] — **not zero**.
    pub fn budgeted(&self) -> (Basis, Option<f32>) {
        if let Some(ms) = self.estimate.and_then(|e| e.ms()) {
            return (Basis::Estimated, Some(ms));
        }
        match self.cost {
            Some(cost) => (Basis::Measured, Some(cost.ms)),
            None => (Basis::Unbudgetable, None),
        }
    }
}

/// The budget, and the decision procedure over it.
///
/// Holds no per-slot state: a pass is a pure function of the slot states handed
/// to it, so the same deck in the same condition decides the same way whether
/// it is asked once a frame or once a set.
#[derive(Clone, Copy, Debug)]
pub struct Governor {
    budget_ms: f32,
}

impl Default for Governor {
    fn default() -> Governor {
        Governor::new(DEFAULT_COMPUTE_BUDGET_MS)
    }
}

impl Governor {
    pub fn new(budget_ms: f32) -> Governor {
        Governor { budget_ms }
    }

    pub fn budget_ms(&self) -> f32 {
        self.budget_ms
    }

    pub fn set_budget_ms(&mut self, budget_ms: f32) {
        self.budget_ms = budget_ms;
    }

    /// Decide, over the slots as they stand.
    ///
    /// Reads [`SlotState::requested`] and returns an effective residency per
    /// slot. **From the request every time, with no memory of the last pass** —
    /// which is what makes a park recover by itself: the arithmetic that
    /// refused a slot is the same arithmetic that admits it once the deck has
    /// room, and nothing had to remember that it was ever refused.
    ///
    /// Index order, and nothing else: two slots that are identical apart from
    /// position are resolved by position, so the answer does not depend on
    /// which one last had a build land on it. That is the same rule the
    /// composite's sum order follows and for the same reason.
    ///
    /// Allocates one `Vec` for the report, so this is not a frame-path call —
    /// it is a decision, at beat or edit granularity, not per frame. Calling it
    /// every frame would work and would allocate every frame; the caller is
    /// expected to call it when something changed.
    pub fn decide(&self, slots: &[SlotState]) -> Report {
        let mut committed_ms = 0.0f32;
        let mut unmeasured_live = 0usize;
        let mut host_clock = false;
        for slot in slots {
            if let Some(cost) = slot.cost {
                if cost.method == MeasurementMethod::HostWallClock {
                    host_clock = true;
                }
            }
            // The rungs an estimate was fitted through carry the same bias and
            // are read whether or not this slot ends up budgeted on them: a
            // refused estimate was still taken, and what took it is still the
            // instrument this deck has.
            if slot.estimate.and_then(|e| e.method) == Some(MeasurementMethod::HostWallClock) {
                host_clock = true;
            }
            if slot.requested == Residency::Live {
                // **The budgeted number, not the measurement**, and the same
                // `budgeted` that `admit` uses — so a Live slot and a priming
                // one are never judged on different readings of the same Set.
                match slot.budgeted().1 {
                    Some(ms) => committed_ms += ms,
                    None => unmeasured_live += 1,
                }
            }
        }

        // The Live slots come first and come out whole. Whatever is left is
        // what priming is allowed to spend; it is never negative in the
        // arithmetic below, because a negative headroom admits nothing and
        // saying so once here is clearer than a signed comparison at every use.
        //
        // `committed_ms` is a *lower bound* when a Live slot is unmeasured, so
        // `over_budget` still holds — an unknown can only add — while the
        // headroom below does not, and `admit` refuses on it rather than
        // spending it. See "Why an unmeasured Set is not a free one".
        let over_budget = committed_ms > self.budget_ms;
        let committed_known = unmeasured_live == 0;
        let mut headroom = (self.budget_ms - committed_ms).max(0.0);
        let mut priming_ms = 0.0f32;

        let decisions = slots
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                let cost_ms = slot.cost.map(|c| c.ms);
                let (basis, budgeted_ms) = slot.budgeted();
                // The request decides which question is asked; the answer is
                // the effective residency and is never written back over it.
                let (effective, reason) = match slot.requested {
                    Residency::Live => (Residency::Live, Reason::OnAir),
                    Residency::Allocated => (Residency::Allocated, Reason::OffAir),
                    Residency::Priming => {
                        let (r, why) = self.admit(slot, &mut headroom, committed_known);
                        if r == Residency::Priming {
                            priming_ms += budgeted_ms.unwrap_or(0.0);
                        }
                        (r, why)
                    }
                };
                Decision {
                    slot: i,
                    requested: slot.requested,
                    effective,
                    reason,
                    cost_ms,
                    budgeted_ms,
                    basis,
                    estimate: slot.estimate,
                }
            })
            .collect();

        Report {
            decisions,
            budget_ms: self.budget_ms,
            committed_ms,
            priming_ms,
            unmeasured_live,
            over_budget,
            host_clock,
        }
    }

    /// Whether one slot asking to prime may. Spends from `headroom` when it
    /// says yes.
    ///
    /// Every `Allocated` it returns is a **park**: the caller keeps the
    /// request, and the next pass asks again.
    ///
    /// **There is no rate to decide any more** (ADR-0269). A drawn slot steps
    /// every frame and every slot is drawn, so the question is whether the
    /// budget has room for what this slot already costs, and the answer is yes
    /// or not yet.
    ///
    /// *What* it costs is [`SlotState::budgeted`]: the estimate at the
    /// output's size where there is one that answers, and the
    /// reference-resolution measurement otherwise.
    fn admit(
        &self,
        slot: &SlotState,
        headroom: &mut f32,
        committed_known: bool,
    ) -> (Residency, Reason) {
        // Before the budget, because it is not a budget question: a closed-form
        // Set has nothing to warm, so priming it would buy nothing at any
        // price. Checked first so that the reason a status line shows is the
        // true one rather than "no headroom" on a deck that happened to be
        // full.
        if slot.closed_form {
            return (Residency::Allocated, Reason::NoPrimingNeeded);
        }
        // **The estimate where it answers, the measurement where it does not**,
        // through the one rule on [`SlotState::budgeted`]. An estimate that
        // refused leaves this exactly where it was before estimates existed:
        // the slot is admitted or parked on its measurement, and a slot with
        // neither number is unbudgetable rather than free.
        let (_, budgeted) = slot.budgeted();
        let Some(ms) = budgeted else {
            return (Residency::Allocated, Reason::Unmeasured);
        };
        // After this slot's own measurement and before the arithmetic, because
        // the two unknowns are different things to fix: a slot with no
        // measurement of its own stays unbudgetable however well the rest of
        // the deck is known, so naming that first gives each slot the reason
        // that is actually blocking *it*. The deck-wide condition is in
        // `Report::unmeasured_live` and on the `Display` line either way.
        if !committed_known {
            return (Residency::Allocated, Reason::CommittedUnknown);
        }
        // **The whole cost, against the headroom, once.** A rate used to sit
        // here: `cost / n` amortised over one step every `n` frames, with a
        // peak cap in front of it because the frames a slowed slot did step
        // still cost the whole measurement. Both are gone with the rate
        // (ADR-0269) — a drawn slot steps every frame — and what is left is
        // the comparison the cap was protecting.
        //
        // `is_nan` is checked rather than left to fall out of the comparison: a
        // measurement that is not a number is not a small one, and the `<=`
        // below would answer `false` for it and land it here anyway — but
        // silently, and by accident.
        if ms.is_nan() || ms > *headroom {
            return (Residency::Allocated, Reason::NoHeadroom);
        }
        *headroom -= ms;
        (Residency::Priming, Reason::Fits)
    }
}
