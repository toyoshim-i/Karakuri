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
//! - **How fast each of those steps**, as one step every *n* frames.
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
use crate::probe::{Measurement, MeasurementMethod};

/// The slowest a Priming slot is allowed to be driven: one step every this
/// many frames.
///
/// A rate is only worth having while the slot still finishes warming in a
/// usable time. At 1-in-8 a simulation reaches thirty seconds of `t` after four
/// minutes of wall clock, which is already at the edge of "prepared ahead of a
/// phrase" being a true description. Beyond it, parking the slot and telling
/// the operator so is more honest than pretending it is being prepared.
pub const SLOWEST_PRIME_ONE_IN: u32 = 8;

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

/// Why a slot ended up where it did.
///
/// The first three describe a slot doing what was asked of it. The last four
/// describe a **park**: the request is Priming, the effective residency is
/// Allocated, and this is what has to change for that to stop being true.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// Live. Not the governor's to touch — see the module doc.
    OnAir,
    /// Priming, at full rate, within the headroom.
    Fits,
    /// Priming, slowed down to fit the headroom that was left.
    Slowed,
    /// Allocated, and that is what was asked for. The governor was not asked
    /// about this slot and did nothing to it.
    OffAir,
    /// Parked: it does not fit, at any rate this module is willing to call
    /// priming. Either the headroom left after the Live slots will not take it
    /// even amortised, or its own single-frame cost is larger than the whole
    /// budget — which no rate can fix, because the frames a slowed slot does
    /// step still cost the whole measurement.
    ///
    /// **The request is untouched**, so this is "not now" rather than "no": the
    /// next pass over a deck with room admits it with nothing else changing.
    NoHeadroom,
    /// Parked: nothing measured this Set, so it cannot be budgeted for. Not a
    /// claim that it is expensive — a claim that its cost is unknown, which is
    /// not the same as zero.
    Unmeasured,
    /// Parked: a **Live** slot on this deck is unmeasured, so what the deck is
    /// already spending is unknown and there is no headroom figure to admit
    /// against.
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
    /// One step every this many frames. `1` for Live and for full-rate
    /// priming; meaningless for an Allocated slot, which does not step at all,
    /// and reported as `1` there rather than as a number that looks like a
    /// rate.
    pub prime_one_in: u32,
    pub reason: Reason,
    /// What this slot was measured at, if anything measured it. Carried so a
    /// status line can show what the arithmetic was done on.
    pub cost_ms: Option<f32>,
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
/// **Every millisecond in here is a measurement of a *cold* Set at a fixed
/// reference resolution, taken once, when it was built.** Nothing re-measures.
/// A Set whose population grows, whose points get bigger, or whose overdraw
/// rises as it spreads costs more than this the longer it runs, and a deck on a
/// 4K output costs several times it — see "What it cannot see" in
/// [`crate::swap`] for the whole list. The numbers are comparable *between*
/// slots, which is what an admission decision needs; they are not a prediction
/// of this machine's frame time, and a status line that presents them as one is
/// overstating them. [`Report::host_clock`] is a second caveat of the same kind
/// and is the only one the [`Display`](std::fmt::Display) line carries, because
/// it is the only one that can change from run to run.
#[derive(Clone, Debug)]
pub struct Report {
    /// Every slot, in index order.
    pub decisions: Vec<Decision>,
    /// The budget these decisions were made against.
    pub budget_ms: f32,
    /// What the Live slots are already measured to cost, summed. **Understates
    /// by `unmeasured_live` Sets' worth** — see that field.
    pub committed_ms: f32,
    /// What the slots left Priming add on top, at the rates decided: a step
    /// every `n` frames costs `cost / n` per frame, amortised.
    pub priming_ms: f32,
    /// Live slots with no measurement. `committed_ms` cannot include them, so
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
    /// Whether any measurement in the sum came from a host clock rather than
    /// from GPU timestamps. Every number in the report reads coarser and biased
    /// high when this is true, and on this crate's development machine it
    /// always is.
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
    /// What the Set in this slot was measured at, if anything measured it.
    pub cost: Option<Measurement>,
    /// Whether the Set can be jumped to any `t` — see the module doc.
    pub closed_form: bool,
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
            if slot.requested == Residency::Live {
                match slot.cost {
                    Some(cost) => committed_ms += cost.ms,
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
                // The request decides which question is asked; the answer is
                // the effective residency and is never written back over it.
                let (effective, prime_one_in, reason) = match slot.requested {
                    Residency::Live => (Residency::Live, 1, Reason::OnAir),
                    Residency::Allocated => (Residency::Allocated, 1, Reason::OffAir),
                    Residency::Priming => {
                        let (r, n, why) = self.admit(slot, &mut headroom, committed_known);
                        if r == Residency::Priming {
                            priming_ms += cost_ms.unwrap_or(0.0) / n as f32;
                        }
                        (r, n, why)
                    }
                };
                Decision {
                    slot: i,
                    requested: slot.requested,
                    effective,
                    prime_one_in,
                    reason,
                    cost_ms,
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

    /// Whether one slot asking to prime may, and at what rate. Spends from
    /// `headroom` when it says yes.
    ///
    /// Every `Allocated` it returns is a **park**: the caller keeps the
    /// request, and the next pass asks again.
    fn admit(
        &self,
        slot: &SlotState,
        headroom: &mut f32,
        committed_known: bool,
    ) -> (Residency, u32, Reason) {
        // Before the budget, because it is not a budget question: a closed-form
        // Set has nothing to warm, so priming it would buy nothing at any
        // price. Checked first so that the reason a status line shows is the
        // true one rather than "no headroom" on a deck that happened to be
        // full.
        if slot.closed_form {
            return (Residency::Allocated, 1, Reason::NoPrimingNeeded);
        }
        let Some(cost) = slot.cost else {
            return (Residency::Allocated, 1, Reason::Unmeasured);
        };
        // After this slot's own measurement and before the arithmetic, because
        // the two unknowns are different things to fix: a slot with no
        // measurement of its own stays unbudgetable however well the rest of
        // the deck is known, so naming that first gives each slot the reason
        // that is actually blocking *it*. The deck-wide condition is in
        // `Report::unmeasured_live` and on the `Display` line either way.
        if !committed_known {
            return (Residency::Allocated, 1, Reason::CommittedUnknown);
        }
        // **A peak cap, before the amortisation.** `cost / n` is an average and
        // the frames it does step cost the whole `cost`, so without this a Set
        // measured at six times the entire budget is admitted at one frame in
        // eight — 100 ms into a 16.7 ms budget reads as 12.5 ms and "fits",
        // while every eighth frame runs a hundred milliseconds of hidden work
        // that nothing on air asked for. A rate can spread a cost that fits in
        // a frame across several frames; it cannot make one that does not fit
        // into one that does. A Set this expensive is not a priming problem.
        //
        // `is_nan` is checked rather than left to fall out of the comparison: a
        // measurement that is not a number is not a small one, and every
        // `<=` below would answer `false` for it and land it here anyway — but
        // silently, and by accident.
        if cost.ms.is_nan() || cost.ms > self.budget_ms {
            return (Residency::Allocated, 1, Reason::NoHeadroom);
        }
        // A slot stepping one frame in `n` costs `cost / n` per frame,
        // amortised. That is the whole of the rate model, and it is worth being
        // explicit that it is an average rather than a promise about any single
        // frame: the frames it does step cost the full `cost`, and the deck
        // will be lumpy at coarse rates. The alternative — spreading one step
        // across several frames — is not available, because a step is one
        // dispatch chain and cannot be cut in half.
        for n in 1..=SLOWEST_PRIME_ONE_IN {
            let amortised = cost.ms / n as f32;
            if amortised <= *headroom {
                *headroom -= amortised;
                let reason = if n == 1 { Reason::Fits } else { Reason::Slowed };
                return (Residency::Priming, n, reason);
            }
        }
        (Residency::Allocated, 1, Reason::NoHeadroom)
    }
}
