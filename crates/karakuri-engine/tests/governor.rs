//! The budget governor: what may prime, and what it refuses to touch.
//!
//! **It used to decide a rate as well**, one step every *n* frames, and about a
//! third of this file asserted the arithmetic of it. ADR-0269 retired that: a
//! drawn slot steps every frame, every slot is drawn, and a request now either
//! fits at its whole measured cost or is parked.
//!
//! Two halves, deliberately.
//!
//! The **decision procedure** is a pure function of slot states — a
//! measurement per slot, a *requested* residency, and a closed-form flag — so
//! most of it is asserted without a GPU, which is what lets the arithmetic be
//! pinned exactly rather than approximately. `Governor::decide` allocating a
//! report is the only side effect it has.
//!
//! The **deck integration** needs a GPU, because the thing worth asserting
//! there is that the report is applied: that a demoted slot really stops
//! stepping and a Live slot really is not moved. A report nobody acted on would
//! satisfy every assertion in the first half.
//!
//! **Everything about a park is in the second half**, and deliberately so. A
//! park is a request that outlives a refusal, and a request only outlives
//! anything if something stores it — so a pure `decide` handed the same
//! `SlotState` twice would demonstrate recovery whether or not the deck kept
//! the request at all. The tests that pin it therefore drive a real `Deck`
//! across several passes, and say what a caller does *not* do between them.
//!
//! Measurements are constructed by hand here rather than probed. That is the
//! point of the split `swap.rs` makes: the measurement is data that travels
//! with a Set, so the budget arithmetic is testable at exact numbers instead of
//! against whatever this machine happened to be doing.
//!
//! **The estimates are constructed the same way, and for a second reason.** A
//! slot can now arrive with two numbers — one draw at 1280x720, and a fit
//! through two draws at the output's size — and the governor prefers the
//! second where it answers. `estimate::fit` is pure arithmetic over two
//! `Measurement`s, so the whole of *Two numbers, and which one is budgeted on*
//! is pinned here at exact figures. What the device is for is the path:
//! `Deck::estimate_slots` taking the two draws, storing the answer on the slot,
//! and `govern` reading it back.

use karakuri_engine::deck::{Deck, Residency};
use karakuri_engine::estimate::{fit, rungs, Estimate, Floor, Unfit};
use karakuri_engine::governor::{Basis, Estimated, FloorRead, Governor, Reason, SlotState};
use karakuri_engine::probe::{Measurement, MeasurementMethod};
use karakuri_engine::swap::HotSwap;
use karakuri_engine::{Gpu, Present, Set};
use karakuri_ir::typed::Checked;
use karakuri_ir::Topology;

const WIDTH: u32 = 128;
const HEIGHT: u32 = 128;
const CAPACITY: u32 = 1024;

/// A measurement of `ms`, at nothing in particular. The resolution and capacity
/// are recorded on a real one and the governor does not read either — it sums
/// `ms` — so they are filled with the reference values rather than left
/// meaningful.
fn cost(ms: f32) -> Measurement {
    Measurement {
        ms,
        method: MeasurementMethod::GpuTimestamp,
        capacity: CAPACITY,
        resolution: (1280, 720),
    }
}

fn live(ms: f32) -> SlotState {
    SlotState {
        requested: Residency::Live,
        cost: Some(cost(ms)),
        estimate: None,
        closed_form: false,
    }
}

fn priming(ms: f32) -> SlotState {
    SlotState {
        requested: Residency::Priming,
        cost: Some(cost(ms)),
        estimate: None,
        closed_form: false,
    }
}

/// A Live slot nothing has measured — every `HotSwap::fixed` Set until
/// `Deck::measure_slots` runs. Its cost is not zero, it is unknown.
fn unmeasured_live() -> SlotState {
    SlotState {
        requested: Residency::Live,
        cost: None,
        estimate: None,
        closed_form: false,
    }
}

// ---------------------------------------------------------------------------
// The second number.
//
// `estimate` was built on 2026-09-06 and exported to no caller; ADR-0293 made
// it answer for every piece of material this instrument ships, and these are
// the tests of it reaching `Governor::decide`. The estimates below are fitted
// through **synthetic** rungs built from a known `a` and `b`, exactly as
// `src/estimate.rs`'s own unit tests are and for the same reason: the
// arithmetic is the whole of the rule, and a threshold on a real draw would be
// a test of this machine (`docs/contributing.md` §1).
// ---------------------------------------------------------------------------

fn area((w, h): (u32, u32)) -> f64 {
    f64::from(w) * f64::from(h)
}

/// One rung: `ms` at `resolution`, on the instrument the rest of this file uses.
fn at(ms: f64, resolution: (u32, u32)) -> Measurement {
    Measurement {
        ms: ms as f32,
        method: MeasurementMethod::GpuTimestamp,
        capacity: CAPACITY,
        resolution,
    }
}

/// **An estimate for `target` costing `a + fragment_ms` there**, fitted through
/// the two rungs `estimate::rungs` would itself place under `floor`.
///
/// The floor is what decides *which* pair, so passing a real one is what makes
/// the floored branch reachable from here: a floor at or below a quarter of the
/// target's height gets the cheap pair and an exact fit, and one above half its
/// height gets the accurate pair and a correction. ADR-0293's table is the map.
///
/// `Floor::Analysed` rather than the `Floor::Stated` `fit` produces, because
/// that is what `estimate` writes on the way out and it is the difference the
/// governor reports. The bounds themselves are empty here — this file is about
/// what the governor does with the floor's *provenance*, and
/// `crates/karakuri-ir/tests/rate.rs` is where the bounds are written out.
fn estimate_of(a: f64, fragment_ms: f64, target: (u32, u32), floor: u32) -> Estimate {
    let [low, high] = rungs(target, floor).expect("the target holds two rungs");
    let b = fragment_ms / area(target);
    let mut e = fit(
        at(a + b * area(low), low),
        at(a + b * area(high), high),
        target,
        floor,
        vec![Topology::Points],
    );
    e.floor_from = Floor::Analysed {
        bounds: Vec::new(),
        contradicted: None,
    };
    e
}

/// An estimate that **refuses**, and the refusal is a real one rather than a
/// hand-built variant: the upper rung reads cheaper than the lower, so `b` is
/// negative, which is a failed measurement and not a cheap Set.
fn refused_estimate(target: (u32, u32), floor: u32) -> Estimate {
    let [low, high] = rungs(target, floor).expect("the target holds two rungs");
    let mut e = fit(
        at(9.0, low),
        at(8.0, high),
        target,
        floor,
        vec![Topology::Points],
    );
    assert!(
        matches!(e.fit, Err(Unfit::FragmentTermNegative { .. })),
        "the refusal this file is built on stopped being reachable: {:?}",
        e.fit
    );
    e.floor_from = Floor::Analysed {
        bounds: Vec::new(),
        contradicted: None,
    };
    e
}

fn with_estimate(mut slot: SlotState, e: &Estimate) -> SlotState {
    slot.estimate = Some(Estimated::from(e));
    slot
}

// ---------------------------------------------------------------------------
// The decision procedure.
// ---------------------------------------------------------------------------

/// **A Priming slot is demoted when the committed cost leaves no room for it,
/// and the Live slot it is competing with is not touched.**
///
/// 12 ms is on air against a 16 ms budget, leaving 4 ms. The candidate costs
/// 40 ms, which does not fit at any rate the governor is willing to call
/// priming — 40/8 is still 5 ms — so it is parked. Nothing about slot 0
/// changes, and that is asserted explicitly rather than left implied: the worst
/// thing this component could do is take a slot off air mid-set.
#[test]
fn a_priming_slot_is_demoted_when_the_committed_cost_leaves_no_room() {
    let report = Governor::new(16.0).decide(&[live(12.0), priming(40.0)]);

    assert_eq!(report.committed_ms, 12.0);
    assert_eq!(report.headroom_ms(), Some(4.0));
    assert!(!report.over_budget);

    assert_eq!(report.decisions[0].effective, Residency::Live);
    assert_eq!(report.decisions[0].reason, Reason::OnAir);

    assert_eq!(
        report.decisions[1].effective,
        Residency::Allocated,
        "a candidate costing 40 ms was admitted into 4 ms of headroom"
    );
    assert_eq!(report.decisions[1].reason, Reason::NoHeadroom);
    assert_eq!(report.priming_ms, 0.0);
}

/// The negative control for the test above: the identical deck with a budget
/// that fits admits the same slot. Without this, "demoted when there is no
/// room" would also pass on a governor that demoted everything.
#[test]
fn the_same_slot_primes_when_there_is_room() {
    let report = Governor::new(60.0).decide(&[live(12.0), priming(40.0)]);

    assert_eq!(report.decisions[1].effective, Residency::Priming);
    assert_eq!(report.decisions[1].reason, Reason::Fits);
    assert_eq!(report.priming_ms, 40.0);
}

/// **There is nothing between the two.** 16 ms committed against a 20 ms budget
/// leaves 4, and a 10 ms candidate is parked — where it used to be admitted at
/// one frame in three and charged 10/3 against the budget.
///
/// Kept as a test of its own, and kept at these numbers, because this is the
/// case ADR-0269 changed the answer to: a rate could spread a cost that fits in
/// a frame across several frames, and there is no such spreading left to do.
#[test]
fn a_candidate_that_does_not_fit_is_parked_rather_than_slowed() {
    let report = Governor::new(20.0).decide(&[live(16.0), priming(10.0)]);

    let decision = report.decisions[1];
    assert_eq!(decision.effective, Residency::Allocated);
    assert_eq!(decision.reason, Reason::NoHeadroom);
    assert!(decision.is_parked(), "the request was not held");
    assert_eq!(report.priming_ms, 0.0);
}

/// **Two priming slots share what is left, in index order**, and the second one
/// is decided against the headroom the first already spent. Order is by index
/// and nothing else, for the same reason the composite's sum order is: an
/// answer that depended on which slot last had a build land on it would not
/// reproduce.
#[test]
fn priming_slots_are_admitted_in_index_order_against_a_shrinking_headroom() {
    let report = Governor::new(20.0).decide(&[live(10.0), priming(8.0), priming(3.0)]);

    assert_eq!(
        report.decisions[1].reason,
        Reason::Fits,
        "the first one fits"
    );
    // 2 ms left after the first, which will not take 3.
    assert_eq!(report.decisions[2].reason, Reason::NoHeadroom);

    // The same 3 ms candidate behind a cheaper neighbour is admitted, which is
    // what makes "in index order, against a shrinking headroom" a claim rather
    // than a description: the answer for a slot depends on what was decided
    // before it, and "before" is by index.
    let cheaper_first = Governor::new(20.0).decide(&[live(10.0), priming(4.0), priming(3.0)]);
    assert_eq!(cheaper_first.decisions[1].reason, Reason::Fits);
    assert_eq!(
        cheaper_first.decisions[2].reason,
        Reason::Fits,
        "6 ms of headroom takes a 3 ms candidate"
    );
    assert_eq!(cheaper_first.priming_ms, 7.0);
}

/// **Live slots over the budget are a warning and nothing else.**
///
/// Four heavy Sets on air is the operator's decision. The governor says so and
/// suspends priming; what it must never do is take one off air.
#[test]
fn live_slots_over_the_budget_are_warned_about_and_left_alone() {
    let report = Governor::new(16.0).decide(&[live(12.0), live(12.0), priming(1.0)]);

    assert!(
        report.over_budget,
        "24 ms of Live against a 16 ms budget is not flagged"
    );
    assert!(report.headroom_ms().is_some_and(|h| h < 0.0));
    for i in 0..2 {
        assert_eq!(
            report.decisions[i].effective,
            Residency::Live,
            "the governor took slot {i} off air; it may warn and may not act"
        );
        assert_eq!(report.decisions[i].reason, Reason::OnAir);
    }
    assert_eq!(
        report.decisions[2].effective,
        Residency::Allocated,
        "priming continued on a deck whose Live slots are already over budget"
    );
    assert_eq!(report.decisions[2].reason, Reason::NoHeadroom);
}

/// **A closed-form Set is not primed, and the reason says why.**
///
/// It has no state to warm — any `t` is reachable directly — so priming it
/// would spend budget to arrive where it was already going to be, and would
/// spend it instead of a slot that needed it. Parked with a reason that is not
/// a refusal: it can go Cold to Live whenever it is wanted.
///
/// The budget here is enormous, so "no headroom" cannot be the explanation.
#[test]
fn a_closed_form_slot_is_not_primed_however_much_budget_there_is() {
    let slot = SlotState {
        requested: Residency::Priming,
        cost: Some(cost(1.0)),
        estimate: None,
        closed_form: true,
    };
    let report = Governor::new(10_000.0).decide(&[slot]);

    assert_eq!(report.decisions[0].effective, Residency::Allocated);
    assert_eq!(
        report.decisions[0].reason,
        Reason::NoPrimingNeeded,
        "a closed-form Set was primed; it has no state to warm"
    );
    assert_eq!(report.priming_ms, 0.0);
}

/// **An unmeasured Set is not a free one.** Treating `None` as zero would make
/// the least-known Set the cheapest thing on the deck and the first thing
/// admitted, which inverts the whole point of budgeting.
///
/// The budget is again enormous, so the refusal cannot be about room.
#[test]
fn an_unmeasured_priming_slot_is_refused_rather_than_treated_as_free() {
    let slot = SlotState {
        requested: Residency::Priming,
        cost: None,
        estimate: None,
        closed_form: false,
    };
    let report = Governor::new(10_000.0).decide(&[slot]);

    assert_eq!(report.decisions[0].effective, Residency::Allocated);
    assert_eq!(report.decisions[0].reason, Reason::Unmeasured);
}

/// An unmeasured *Live* slot is counted and said so. It cannot be added to
/// `committed_ms`, so the headroom is optimistic by an unknown amount and the
/// report has to carry that or it is a budget figure pretending to be a budget.
#[test]
fn an_unmeasured_live_slot_is_reported_because_the_headroom_understates_it() {
    let report = Governor::new(16.0).decide(&[live(4.0), unmeasured_live()]);

    assert_eq!(report.committed_ms, 4.0);
    assert_eq!(
        report.unmeasured_live, 1,
        "a Live slot with no measurement vanished from the report entirely"
    );
    assert!(report.to_string().contains("+1 unmeasured"));
}

/// **An unmeasured Live slot creates no headroom, and priming is refused while
/// there is one.**
///
/// The committed cost is not "4 ms" on this deck, it is "4 ms plus something
/// nobody measured", and the difference is the whole budget: a 16 ms candidate
/// admitted at full rate against it is spending room that may not exist. The
/// rule the module already applies to a priming candidate — an unmeasured Set
/// is unbudgetable rather than free — is the same rule one field over.
#[test]
fn priming_is_refused_while_a_live_slot_is_unmeasured() {
    let report = Governor::new(16.0).decide(&[live(4.0), unmeasured_live(), priming(1.0)]);

    assert_eq!(
        report.decisions[2].effective,
        Residency::Allocated,
        "a candidate was admitted against a committed cost nobody knows"
    );
    assert!(!report.committed_known());
    assert_eq!(report.priming_ms, 0.0);

    // The negative control, and the one that matters most here: the same deck
    // with that slot measured — at a cost small enough that room is not the
    // question — admits the same candidate at full rate. Without it, "refused
    // while unmeasured" would pass on a governor that refused everything.
    let measured = Governor::new(16.0).decide(&[live(4.0), live(1.0), priming(1.0)]);
    assert_eq!(measured.decisions[2].effective, Residency::Priming);
    assert_eq!(measured.decisions[2].reason, Reason::Fits);
}

/// **"I cannot tell" is a different answer from "there is no room",** and the
/// report says which.
///
/// They call for different actions — one waits for a slot to come off air, the
/// other for a measurement — so a status line that could not tell them apart
/// would send the operator after the wrong one. The `Display` line says so in
/// words too, because that is what an operator actually reads.
#[test]
fn an_unknown_commitment_is_reported_differently_from_a_full_one() {
    let unknown = Governor::new(16.0).decide(&[unmeasured_live(), priming(1.0)]);
    let full = Governor::new(16.0).decide(&[live(15.9), priming(1.0)]);

    assert_eq!(unknown.decisions[1].reason, Reason::CommittedUnknown);
    assert_eq!(full.decisions[1].reason, Reason::NoHeadroom);
    assert_ne!(unknown.decisions[1].reason, full.decisions[1].reason);

    assert!(unknown.to_string().contains("UNKNOWN"));
    assert!(
        !full.to_string().contains("UNKNOWN"),
        "a deck whose commitment is fully known reported it as unknown"
    );
}

/// **`headroom_ms` refuses to answer when the commitment is unknown.**
///
/// The defect this closes is a caller writing `if headroom > x`: on a deck with
/// an unmeasured Live slot, `budget - committed` is a confident-looking number
/// computed from an understated commitment, and nothing in it says so. An
/// `Option` makes that caller handle it, at compile time.
#[test]
fn headroom_is_not_a_number_when_the_committed_cost_is_unknown() {
    let unknown = Governor::new(16.0).decide(&[live(4.0), unmeasured_live()]);
    assert_eq!(
        unknown.headroom_ms(),
        None,
        "an unconfident deck produced a confident headroom of \
         {:?} ms",
        unknown.budget_ms - unknown.committed_ms
    );

    // Measured, the same arithmetic is answered — including when it is
    // negative, which is a known-bad number rather than an unknown one.
    let known = Governor::new(16.0).decide(&[live(4.0), live(2.0)]);
    assert_eq!(known.headroom_ms(), Some(10.0));
    let over = Governor::new(16.0).decide(&[live(20.0)]);
    assert_eq!(over.headroom_ms(), Some(-4.0));
    assert!(over.over_budget);
}

/// **An unmeasured Live slot does not hide an over-budget deck.**
///
/// `committed_ms` is a lower bound when something is unmeasured, and an unknown
/// can only add to it — so measured-alone-over-budget is still over budget, and
/// the one warning this module raises has to survive the state that suspends
/// priming. Otherwise the deck that is worst off reports the least.
#[test]
fn over_budget_still_fires_when_a_live_slot_is_unmeasured() {
    let report = Governor::new(16.0).decide(&[live(20.0), unmeasured_live()]);

    assert!(report.over_budget);
    assert!(!report.committed_known());
    assert_eq!(report.headroom_ms(), None);
}

/// **The request survives the demotion, in the report.**
///
/// A park is "not now", and the only thing that distinguishes it from "no" is
/// that the request is still there to be read. `Decision::is_parked` is the
/// question a status line asks; `requested` is the fact under it.
#[test]
fn a_parked_slot_still_carries_the_request_that_was_refused() {
    let report = Governor::new(16.0).decide(&[live(15.0), priming(16.0)]);

    let decision = report.decisions[1];
    assert_eq!(decision.effective, Residency::Allocated);
    assert_eq!(
        decision.requested,
        Residency::Priming,
        "the governor overwrote the operator's request with its own verdict"
    );
    assert!(decision.is_parked());
    assert_eq!(report.parked().count(), 1);

    // A slot the operator actually parked is not this, and reads differently.
    // Without this the assertions above would pass on a report that called
    // every Allocated slot parked.
    let off = Governor::new(16.0).decide(&[SlotState {
        requested: Residency::Allocated,
        cost: Some(cost(1.0)),
        estimate: None,
        closed_form: false,
    }]);
    assert!(!off.decisions[0].is_parked());
    assert_eq!(off.decisions[0].reason, Reason::OffAir);
    assert_eq!(off.parked().count(), 0);
}

/// An Allocated slot is left alone and reported as such: the governor was not
/// asked about it, and it does not promote on its own.
#[test]
fn an_allocated_slot_is_never_promoted_by_the_governor() {
    let parked = SlotState {
        requested: Residency::Allocated,
        cost: Some(cost(0.1)),
        estimate: None,
        closed_form: false,
    };
    let report = Governor::new(10_000.0).decide(&[parked]);

    assert_eq!(report.decisions[0].effective, Residency::Allocated);
    assert_eq!(report.decisions[0].reason, Reason::OffAir);
}

/// **The estimate is what a slot is budgeted at, and the measurement is not.**
///
/// The two numbers are of different things: `cost` is one draw at 1280x720 and
/// the estimate is a fit at the size the deck is actually drawing. Here the
/// deck draws into 640x360, where this material costs 3 ms, and the 12 ms
/// measurement is a figure about a frame nobody is rendering.
///
/// **It changes an admission and not only a report.** Against 12 ms committed
/// there are 4.7 ms of headroom and the 10 ms candidate is parked; against
/// 3 ms there are 13.7 and it primes. That is the whole of what wiring
/// `estimate` to the governor buys, in one assertion.
#[test]
fn the_estimate_is_what_a_slot_is_budgeted_at_and_the_measurement_is_not() {
    let output = (640, 360);
    // A floor at a quarter of the target's height: the cheap pair, nothing
    // rounded up at either rung, and `floored` is `None`.
    let e = estimate_of(1.0, 2.0, output, 90);
    assert_eq!(e.ms(), Some(3.0));

    let report = Governor::new(16.7).decide(&[with_estimate(live(12.0), &e), priming(10.0)]);

    let on_air = report.decisions[0];
    assert_eq!(on_air.basis, Basis::Estimated);
    assert_eq!(on_air.budgeted_ms, Some(3.0));
    assert_eq!(
        on_air.cost_ms,
        Some(12.0),
        "the measurement is kept beside the estimate, not overwritten by it"
    );
    assert_eq!(report.committed_ms, 3.0);
    assert_eq!(report.estimated(), 1);
    assert_eq!(report.estimated_target(), Some(output));

    assert_eq!(
        report.decisions[1].effective,
        Residency::Priming,
        "the candidate was parked against a commitment measured at a size the \
         deck is not drawing"
    );
    assert_eq!(report.decisions[1].reason, Reason::Fits);

    // The negative control, and it is the state this repository was in until
    // today: the same deck with nothing estimated parks the same candidate.
    let unwired = Governor::new(16.7).decide(&[live(12.0), priming(10.0)]);
    assert_eq!(unwired.decisions[1].effective, Residency::Allocated);
    assert_eq!(unwired.decisions[1].reason, Reason::NoHeadroom);
    assert_eq!(unwired.estimated(), 0);
}

/// **An estimate that refuses leaves the measurement deciding, and says it
/// refused.**
///
/// A refusal is not a number and not a zero — it is `P-0095`'s instrument
/// declining to answer — so the slot goes back to exactly the arithmetic it
/// would have had before any of this existed. What is new is that the refusal
/// is on the record: a slot that fell back must not read like one nothing ever
/// asked.
#[test]
fn an_estimate_that_refuses_leaves_the_measurement_deciding() {
    let refused = refused_estimate((1280, 720), 180);
    let report = Governor::new(16.7).decide(&[with_estimate(live(12.0), &refused), priming(10.0)]);

    assert_eq!(report.decisions[0].basis, Basis::Measured);
    assert_eq!(report.decisions[0].budgeted_ms, Some(12.0));
    assert_eq!(report.committed_ms, 12.0);
    assert_eq!(report.estimated(), 0);
    assert_eq!(
        report.decisions[1].reason,
        Reason::NoHeadroom,
        "a refused estimate admitted a candidate the measurement refuses"
    );

    let refusals: Vec<_> = report.refused_estimates().collect();
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].0, 0);
    assert!(matches!(refusals[0].1, Unfit::FragmentTermNegative { .. }));
    assert!(
        report.decisions[0].estimate.is_some(),
        "the estimate that refused was dropped, so nothing says why the \
         measurement is being used"
    );

    // And the one line a status line prints says it, which on a deck where
    // every estimate refused is the only place it is said at all.
    let line = report.to_string();
    assert!(
        line.contains("1 refused, budgeted on the measurement"),
        "{line}"
    );
}

/// **An estimate that refuses is not a licence.**
///
/// With no measurement behind it there is no number at all, and the governor
/// does what it has always done with a slot it cannot budget: parks a request
/// rather than granting it, and refuses the whole deck's priming while such a
/// slot is on air. An instrument declining to answer has not said the answer is
/// small — which is the same rule as "Why an unmeasured Set is not a free one",
/// reached from the second direction.
#[test]
fn an_estimate_that_refuses_is_not_a_licence_to_admit() {
    let refused = refused_estimate((1280, 720), 180);
    let unmeasured = SlotState {
        requested: Residency::Priming,
        cost: None,
        estimate: Some(Estimated::from(&refused)),
        closed_form: false,
    };
    let report = Governor::new(10_000.0).decide(&[unmeasured]);
    assert_eq!(report.decisions[0].effective, Residency::Allocated);
    assert_eq!(report.decisions[0].reason, Reason::Unmeasured);
    assert_eq!(report.decisions[0].basis, Basis::Unbudgetable);
    assert_eq!(report.decisions[0].budgeted_ms, None);
    assert_eq!(report.priming_ms, 0.0);

    // On air, the same slot makes the deck's commitment unknown and suspends
    // priming everywhere — exactly as an unmeasured Live slot always has.
    let on_air = SlotState {
        requested: Residency::Live,
        cost: None,
        estimate: Some(Estimated::from(&refused)),
        closed_form: false,
    };
    let deck = Governor::new(10_000.0).decide(&[on_air, priming(1.0)]);
    assert_eq!(deck.unmeasured_live, 1);
    assert_eq!(deck.headroom_ms(), None);
    assert_eq!(deck.decisions[1].reason, Reason::CommittedUnknown);
}

/// **Where the floor came from and how strictly it was read travel with the
/// number, and neither is spent twice.**
///
/// `P-0095` at one remove: an estimate taken with every primitive at least a
/// pixel across at both rungs and one taken with some of them rounded up are
/// not the same statement. `Estimate::floor_from` says the first half and
/// `Estimate::floored` the second, and a governor that acted on `ms` alone
/// would be handing a status line a number nobody can check.
///
/// **What the governor does with them is report them and nothing arithmetic.**
/// `Fit::ms` already carries `Floored::correction` — ADR-0293 §4 applies it to
/// the answer rather than to either term — so a governor applying it again
/// would inflate a number that is already sound. The assertion below is that
/// the budgeted figure is the corrected one **exactly**, neither the raw line
/// nor the line corrected twice.
#[test]
fn the_floor_and_how_strictly_it_was_read_travel_with_the_number() {
    let target = (1280, 720);
    // `soft_points`' floor over its declared range (ADR-0293's table): far
    // above the half-height rung, so the accurate pair is placed and the
    // answer is corrected.
    let e = estimate_of(4.0, 4.0, target, 4141);
    let floored = e
        .floored
        .expect("a floor of 4141 rows floors the lower rung");
    assert!(
        (floored.share - 0.2492).abs() < 1e-3,
        "the share moved: {}",
        floored.share
    );
    assert!(
        (floored.correction - 1.3319).abs() < 1e-3,
        "the correction moved: {}",
        floored.correction
    );

    let report = Governor::new(16.7).decide(&[with_estimate(live(1.0), &e)]);
    let d = report.decisions[0];
    let carried = d.estimate.expect("the estimate reached the decision");

    assert_eq!(carried.floor, Some(4141));
    assert_eq!(carried.floor_from, FloorRead::Analysed);
    assert_eq!(carried.floored, Some(floored));
    assert!(carried.corrected());
    assert_eq!(report.corrected(), 1);
    assert_eq!(report.floor_unknown(), 0);

    // The raw line is 8.0 ms; the answer that line supports is 8.0 × 1.3319.
    let raw = 4.0 + 4.0;
    let corrected_once = raw * floored.correction as f32;
    let budgeted = d.budgeted_ms.expect("a number");
    assert!(
        (budgeted - corrected_once).abs() < 1e-2,
        "the governor budgeted {budgeted}, which is neither the raw line \
         ({raw}) nor the corrected answer ({corrected_once}) — a correction \
         applied twice would read {}",
        corrected_once * floored.correction as f32
    );
    assert_eq!(report.committed_ms, budgeted);

    // And the one line a status line prints says all of it.
    let line = report.to_string();
    assert!(line.contains("1 estimated at 1280x720"), "{line}");
    assert!(line.contains("1 corrected for a floored rung"), "{line}");
}

/// **A floor nothing could establish is an answer, and it is named as one.**
///
/// ADR-0293 §6 reads an unknown floor as the greatest floor there is — a
/// placement rather than a sentinel — so the estimate answers, and a held value
/// that falsified a bound is the same case. Both are numbers the governor
/// spends; both are numbers it says it spent, because an answer taken at the
/// widest placement is not the same statement as one taken against a floor
/// somebody proved.
#[test]
fn an_unknown_floor_is_an_answer_the_report_still_names() {
    let target = (1280, 720);
    let mut e = estimate_of(4.0, 4.0, target, u32::MAX);
    // What `estimate` writes when `floor_rows` could not bound a renderer's
    // rate, or when a held value fell outside the declaration a bound was
    // taken over.
    e.floor = None;
    e.floor_from = Floor::Analysed {
        bounds: Vec::new(),
        contradicted: Some(("width_var".to_string(), 1.5, [0.0, 1.0])),
    };

    let report = Governor::new(16.7).decide(&[with_estimate(live(1.0), &e)]);
    let carried = report.decisions[0]
        .estimate
        .expect("the estimate reached it");

    assert_eq!(report.decisions[0].basis, Basis::Estimated);
    assert_eq!(carried.floor, None);
    assert_eq!(carried.floor_from, FloorRead::Contradicted);
    assert_eq!(report.floor_unknown(), 1);

    let line = report.to_string();
    assert!(line.contains("1 on an unknown floor"), "{line}");
}

/// **A host clock reaches the report through an estimate as well as through a
/// measurement.**
///
/// A host measurement brackets a submit-and-wait the GPU never spent, and the
/// fit inherits it whole: the round trip does not move with the target, so it
/// lands in the invariant term and is added to the answer once. A report whose
/// numbers came from two rungs on a host clock is as biased as one whose came
/// from a single draw on it, and `Report::host_clock` is the only caveat the
/// `Display` line carries.
#[test]
fn a_host_clock_under_an_estimate_reaches_the_reports_one_caveat() {
    let target = (640, 360);
    let [low, high] = rungs(target, 90).expect("two rungs fit");
    let host = |ms: f64, r: (u32, u32)| Measurement {
        ms: ms as f32,
        method: MeasurementMethod::HostWallClock,
        capacity: CAPACITY,
        resolution: r,
    };
    let e = fit(
        host(1.125, low),
        host(1.5, high),
        target,
        90,
        vec![Topology::Points],
    );
    assert!(e.biased_high());

    // The slot's own measurement is on GPU timestamps, so the estimate is the
    // only route the bias has into the report.
    let report = Governor::new(16.7).decide(&[with_estimate(live(12.0), &e)]);
    assert!(report.host_clock);
    assert!(report.to_string().contains("[host clock]"));
}

/// **A governed slot now has a number a band can be predicted from.**
///
/// The risk badge reads one number into five bands and there was no such number
/// anywhere in this workspace: `estimate` refused every per-element Set, so the
/// badge was omitted rather than drawn, and **the only mechanised statement
/// about it is the console's assertion that it is absent** — which fails when
/// somebody draws one and never when the number arrives. ADR-0293 made the
/// estimate answer and ADR-0296 wired it here; this is the assertion from the
/// other side, and **it fails when the number stops arriving**.
///
/// **This predicts a band, it does not draw one.** The five bands and their
/// boundaries belong to the console and are specified in
/// `docs/manual/console.html` under *What a deck preview cell shows, and when*.
/// They are quoted here rather than shared because the engine does not read a
/// number into a badge and this file is not where that table should end up
/// living. What is asserted is the half that is the engine's: given the
/// estimate, the band is determined.
///
/// The three slots carry measurements that would all land in the first band, so
/// a governor that had gone back to budgeting on them would fail this rather
/// than pass it more easily.
#[test]
fn a_governed_slot_now_has_a_number_a_band_can_be_predicted_from() {
    /// The console's scale: 4, 8, 12 and 16 ms, and **a value on a boundary
    /// rounds to the worse band**.
    fn band(ms: f32) -> &'static str {
        match ms {
            _ if ms >= 16.0 => "purple",
            _ if ms >= 12.0 => "red",
            _ if ms >= 8.0 => "yellow",
            _ if ms >= 4.0 => "blue",
            _ => "green",
        }
    }

    let output = (1280, 720);
    let cheap = estimate_of(1.0, 2.0, output, 180);
    let middling = estimate_of(5.0, 5.0, output, 180);
    let ruinous = estimate_of(10.0, 10.0, output, 180);
    for e in [&cheap, &middling, &ruinous] {
        assert_eq!(
            e.floored, None,
            "a floor at a quarter of the target's height clears both rungs"
        );
    }

    let report = Governor::new(16.7).decide(&[
        with_estimate(live(1.0), &cheap),
        with_estimate(live(1.0), &middling),
        with_estimate(live(1.0), &ruinous),
    ]);

    let bands: Vec<&str> = report
        .decisions
        .iter()
        .map(|d| band(d.budgeted_ms.expect("every slot has a number")))
        .collect();
    assert_eq!(bands, ["green", "yellow", "purple"]);
    assert_eq!(
        report.estimated(),
        3,
        "a band was predicted from a measurement"
    );

    // The last band is the one that stops a slot, and the governor's own
    // warning agrees with it: 33 ms of Live material against a 16.7 ms budget.
    assert!(report.over_budget);
}

// ---------------------------------------------------------------------------
// Applied to a deck.
// ---------------------------------------------------------------------------

const L1: &str = r#"
proc creep {
  kind     L1
  topology points
  capacity [256, 262144] = 1024

  emit position, age

  element {
    let dir = sphere_point(hash1(seed), hash1(seed + 1000u));
    position = position + dir * dt * 4.0;
    age      = age + dt;
  }
}
"#;

/// The same shape with no read of what it emits, so `check` classifies it
/// closed form and the deck-level test can assert the governor reads that off
/// the Set rather than being told.
const L1_CLOSED_FORM: &str = r#"
proc pure_shell {
  kind     L1
  topology points
  capacity [256, 262144] = 1024

  emit position

  element {
    let dir = sphere_point(hash1(seed), hash1(seed + 1000u));
    position = dir * (1.0 + sin(t));
  }
}
"#;

const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0), max(0.0, 1.0 - d));
  }
}
"#;

fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{e:?}"));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{e:?}"));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{e:?}"));
    checked
}

fn swap_of(gpu: &Gpu, l1: &str, seed: u32, ms: Option<f32>) -> HotSwap {
    let mut set = Set::build(
        &gpu.device,
        &gpu.queue,
        &compile(l1),
        &compile(L4),
        CAPACITY,
        seed,
    )
    .expect("the pair is compatible");
    set.resize(&gpu.device, WIDTH, HEIGHT);
    let mut swap = HotSwap::fixed(set);
    if let Some(ms) = ms {
        swap.set_measured_cost(cost(ms));
    }
    swap
}

fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present, steps: u8) {
    let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
    f.render(present.hdr_view(), present.size(), steps);
    f.finish();
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
}

fn steps_taken(set: &Set) -> u64 {
    (set.time() * 60.0).round() as u64
}

/// **A Set larger than the whole budget is refused.**
///
/// It used to need saying twice. A rate could spread a cost across frames, so a
/// Set measured at six times the budget divided by eight, read as fitting, and
/// bought a hundred-millisecond hidden step every eighth frame — which is why a
/// peak cap sat in front of the amortisation. With the rate gone (ADR-0269) the
/// cap and the comparison are the same test, and this is it: the budget answers
/// "is there room to step one more simulation", and for this Set there is not.
#[test]
fn a_slot_costing_more_than_the_whole_budget_is_refused() {
    let report = Governor::new(16.7).decide(&[priming(100.0)]);

    assert_eq!(
        report.decisions[0].effective,
        Residency::Allocated,
        "a Set costing six times the whole budget was admitted to prime"
    );
    assert_eq!(report.decisions[0].reason, Reason::NoHeadroom);
    assert_eq!(report.priming_ms, 0.0);

    // The negative control, one step either side of the cap: the same empty
    // deck admits a Set that does fit in one frame.
    let fits = Governor::new(16.7).decide(&[priming(16.0)]);
    assert_eq!(fits.decisions[0].effective, Residency::Priming);
    assert_eq!(fits.decisions[0].reason, Reason::Fits);
}

// The eight of thirty that take a device. The rest reason over a budget and a
// verdict, which is arithmetic — so the majority of this file stays in the set
// `cargo test -- --skip gpu::` runs. See `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use super::*;

    /// **The verdict is applied, not merely reported.**
    ///
    /// A demoted slot has to actually stop stepping and an untouched Live slot has
    /// to actually keep going — a `govern` that returned a correct report and wrote
    /// nothing back would satisfy every assertion in the first half of this file.
    /// `t` is what says so: it advances only through `Set::prepare`, so a slot the
    /// governor parked is one whose clock stands still.
    #[test]
    fn govern_applies_its_verdict_to_the_deck_and_leaves_the_live_slot_running() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        const FRAMES: usize = 12;

        let mut deck = Deck::new(
            &gpu.device,
            vec![
                swap_of(&gpu, L1, 1, Some(12.0)),
                swap_of(&gpu, L1, 2, Some(40.0)),
            ],
            WIDTH,
            HEIGHT,
        );
        deck.set_compute_budget_ms(16.0);
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Priming);

        let report = deck.govern();
        assert_eq!(report.decisions[1].reason, Reason::NoHeadroom);
        assert_eq!(
            deck.residency(karakuri_engine::DeckSlot(1)),
            Residency::Allocated,
            "the governor decided to demote and the deck did not move"
        );
        assert_eq!(
            deck.residency(karakuri_engine::DeckSlot(0)),
            Residency::Live,
            "the governor moved a Live slot"
        );

        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(0)).set()),
            FRAMES as u64,
            "the Live slot stopped stepping"
        );
        // **The demotion is not visible in the slot's `t`, and that is the
        // point rather than a hole in the test.** A parked slot steps every
        // frame like every other off-air slot (ADR-0269), so what `govern`
        // applied is the *residency* — asserted above, off the deck — and what
        // it deliberately did not touch is the simulation. This used to read
        // `0` here, and the sentence it carried, *the demotion was a report and
        // not an action*, is now the wrong test of the right claim.
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            FRAMES as u64,
            "a parked slot stopped stepping, so the cell an operator is judging this \
         candidate by went to a still the moment the budget refused it"
        );

        // The negative control on the same deck: raise the budget and the same slot
        // is admitted. Without this, the assertions above would pass on a `govern`
        // that parked everything it was ever shown. Nothing re-asks for priming
        // here — the request outlived the demotion, which is
        // `a_parked_slot_primes_again_by_itself_when_the_deck_empties`'s subject.
        deck.set_compute_budget_ms(100.0);
        assert_eq!(deck.govern().decisions[1].reason, Reason::Fits);
        assert_eq!(
            deck.residency(karakuri_engine::DeckSlot(1)),
            Residency::Priming
        );
        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            2 * FRAMES as u64
        );
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(0)).set()),
            2 * FRAMES as u64
        );
    }
    /// The closed-form flag reaches the governor **off the Set**, through the check
    /// pass and `Set::build`, rather than being handed to it by a test. Two decks
    /// differing only in which L1 they hold, at a budget that fits either.
    #[test]
    fn a_closed_form_set_is_recognised_through_the_deck() {
        let gpu = Gpu::headless().expect("no GPU available");

        let verdict = |l1: &str| {
            let mut deck = Deck::new(
                &gpu.device,
                vec![swap_of(&gpu, l1, 1, Some(1.0))],
                WIDTH,
                HEIGHT,
            );
            deck.set_compute_budget_ms(1000.0);
            deck.set_residency(karakuri_engine::DeckSlot(0), Residency::Priming);
            deck.govern().decisions[0].reason
        };

        assert_eq!(
            verdict(L1_CLOSED_FORM),
            Reason::NoPrimingNeeded,
            "a closed-form Set was primed; `Checked::closed_form` is not reaching the \
         governor"
        );
        assert_eq!(
            verdict(L1),
            Reason::Fits,
            "an accumulating Set was refused priming as though it were closed form"
        );
    }
    /// **`govern` is idempotent, and a parked slot goes on running.**
    ///
    /// A caller that runs it every frame — which is the shape a status line
    /// invites — has to get the same answer every time, since the pass has no
    /// memory and its inputs did not move. It used to have a second half: the
    /// call must not reset the priming counter, because `prime_phase = 0` every
    /// frame turned "one frame in four" into "every frame". There is no counter
    /// now, and what replaces that half is the other side of the same coin —
    /// the parked slot steps every frame, and repeated governing does not stop
    /// it (ADR-0269).
    #[test]
    fn calling_govern_every_frame_changes_nothing_and_the_parked_slot_runs() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        const FRAMES: usize = 12;

        let mut deck = Deck::new(
            &gpu.device,
            vec![
                swap_of(&gpu, L1, 1, Some(8.0)),
                swap_of(&gpu, L1, 2, Some(8.0)),
            ],
            WIDTH,
            HEIGHT,
        );
        // 8 ms on air against 10 leaves 2, which will not take an 8 ms
        // candidate.
        deck.set_compute_budget_ms(10.0);
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Priming);
        assert_eq!(deck.govern().decisions[1].reason, Reason::NoHeadroom);

        for _ in 0..FRAMES {
            let report = deck.govern();
            assert_eq!(
                report.decisions[1].reason,
                Reason::NoHeadroom,
                "the verdict changed under a repeated call with nothing else changing"
            );
            assert!(
                deck.is_parked(karakuri_engine::DeckSlot(1)),
                "the request was not held across a pass"
            );
            frame(&gpu, &mut deck, &present, 1);
        }

        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            FRAMES as u64,
            "a parked slot did not step; a park withholds the grant and not the \
         simulation, and the cell an operator is judging this candidate by is a \
         still without it"
        );
    }
    /// **A slot parked for lack of headroom primes again by itself.**
    ///
    /// The deck empties and nobody says anything about slot 1: no `set_residency`,
    /// no re-request, nothing. The next pass primes it, because the request was
    /// never the governor's to consume — a demotion writes the *effective*
    /// residency and leaves the operator's ask where it was.
    ///
    /// The version of this that does not work writes `Allocated` back through
    /// `Deck::set_residency`. The slot then reads `Allocated` on every later pass,
    /// is reported as a slot nobody asked about, and never primes again however
    /// empty the deck gets.
    #[test]
    fn a_parked_slot_primes_again_by_itself_when_the_deck_empties() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        const FRAMES: usize = 8;

        let mut deck = Deck::new(
            &gpu.device,
            vec![
                swap_of(&gpu, L1, 1, Some(15.0)),
                swap_of(&gpu, L1, 2, Some(16.0)),
            ],
            WIDTH,
            HEIGHT,
        );
        // 15 ms on air against 16 leaves 1 ms, which will not take a 16 ms
        // candidate. The same candidate on an empty deck fits whole, which is
        // what makes this a park rather than a Set that is simply too
        // expensive.
        deck.set_compute_budget_ms(16.0);
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Priming);

        assert_eq!(deck.govern().decisions[1].reason, Reason::NoHeadroom);
        assert_eq!(
            deck.residency(karakuri_engine::DeckSlot(1)),
            Residency::Allocated
        );
        assert!(
            deck.is_parked(karakuri_engine::DeckSlot(1)),
            "a refused request reads as a slot nobody asked about"
        );
        assert_eq!(deck.parked_slots(), 1);
        assert_eq!(
            deck.requested_residency(karakuri_engine::DeckSlot(1)),
            Residency::Priming,
            "the demotion destroyed the operator's request"
        );

        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        // It runs while it is parked — a park withholds the grant, not the
        // simulation (ADR-0269) — so what changes when the deck empties below is
        // the residency and the report, and not the `t`.
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            FRAMES as u64,
            "a parked slot stopped stepping"
        );

        // The deck empties. Slot 1 is not mentioned.
        deck.set_residency(karakuri_engine::DeckSlot(0), Residency::Allocated);
        let report = deck.govern();

        assert_eq!(
            report.decisions[1].reason,
            Reason::Fits,
            "the slot never primed again after the deck emptied; the request did not \
         survive the demotion"
        );
        assert_eq!(
            deck.residency(karakuri_engine::DeckSlot(1)),
            Residency::Priming
        );
        assert!(!deck.is_parked(karakuri_engine::DeckSlot(1)));
        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            2 * FRAMES as u64,
            "the report said Priming and the slot did not step"
        );
    }
    /// **One over-budget pass defers every priming request and cancels none.**
    ///
    /// `over_budget` clamps the headroom to zero, so every Priming slot on the deck
    /// is refused in the same pass — which is correct, and is exactly why the
    /// refusal must not be written over the requests. A single heavy Set put on air
    /// for one pass would otherwise cancel the whole deck's worth of priming, and
    /// the operator would find out by noticing that nothing ever warmed up again.
    ///
    /// The transient is an operator action with a natural end: a fourth Set goes on
    /// air, and comes off again.
    #[test]
    fn a_transient_over_budget_pass_cancels_nothing() {
        let gpu = Gpu::headless().expect("no GPU available");

        let mut deck = Deck::new(
            &gpu.device,
            vec![
                swap_of(&gpu, L1, 1, Some(4.0)),
                swap_of(&gpu, L1, 2, Some(1.0)),
                swap_of(&gpu, L1, 3, Some(1.0)),
                swap_of(&gpu, L1, 4, Some(30.0)),
            ],
            WIDTH,
            HEIGHT,
        );
        deck.set_compute_budget_ms(16.0);
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Priming);
        deck.set_residency(karakuri_engine::DeckSlot(2), Residency::Priming);
        deck.set_residency(karakuri_engine::DeckSlot(3), Residency::Allocated);

        let before = deck.govern();
        assert!(!before.over_budget);
        assert_eq!(deck.priming_slots(), 2);

        // The heavy Set goes on air: 34 ms committed against 16.
        deck.set_residency(karakuri_engine::DeckSlot(3), Residency::Live);
        let during = deck.govern();
        assert!(
            during.over_budget,
            "34 ms of Live against 16 ms was not flagged"
        );
        assert_eq!(
            deck.priming_slots(),
            0,
            "priming continued on a deck whose Live slots are already over budget"
        );
        assert_eq!(
            deck.parked_slots(),
            2,
            "the requests were cancelled, not parked"
        );
        for slot in [1, 2] {
            assert_eq!(
                deck.requested_residency(karakuri_engine::DeckSlot(slot)),
                Residency::Priming
            );
        }

        // And off again. Nothing is re-requested.
        deck.set_residency(karakuri_engine::DeckSlot(3), Residency::Allocated);
        let after = deck.govern();

        assert!(!after.over_budget);
        assert_eq!(
            deck.priming_slots(),
            2,
            "one over-budget pass permanently cancelled every priming request on the \
         deck"
        );
        assert_eq!(deck.parked_slots(), 0);
        for slot in [1, 2] {
            assert_eq!(after.decisions[slot].reason, Reason::Fits);
        }
    }
    /// **A deck of Sets nobody measured refuses to prime, and measuring them is
    /// what unblocks it.**
    ///
    /// Every `HotSwap::fixed` Set and every Set `HotSwap::new` is constructed with
    /// arrives unmeasured, so this is the state a deck comes up in — and while a
    /// *Live* slot is in it, the committed cost is unknown and there is no headroom
    /// to admit against. `Deck::measure_slots` is the startup call that fixes it,
    /// and this is the whole round trip: refused, measured, admitted, with the
    /// budget never moving.
    ///
    /// The measurement here is a real probe run rather than a number handed in,
    /// because what is being asserted is that the fix is *reachable* — a rule that
    /// turns priming off by default with no pleasant way to turn it back on is not
    /// a fix.
    #[test]
    fn measuring_the_live_slots_is_what_lets_an_unmeasured_deck_prime() {
        let gpu = Gpu::headless().expect("no GPU available");

        // Only the Live slot is unmeasured. The candidate carries a measurement, so
        // nothing about *it* is in question and the refusal below can only be about
        // the deck it is asking to prime on.
        let mut deck = Deck::new(
            &gpu.device,
            vec![swap_of(&gpu, L1, 1, None), swap_of(&gpu, L1, 2, Some(1.0))],
            WIDTH,
            HEIGHT,
        );
        // Generous, so that "no room" can never be the explanation for anything
        // below: whatever this machine measures the Live Set at, it fits.
        deck.set_compute_budget_ms(10_000.0);
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Priming);

        let refused = deck.govern();
        assert_eq!(
            refused.decisions[1].reason,
            Reason::CommittedUnknown,
            "a candidate was admitted against an on-air cost nobody has measured"
        );
        assert_eq!(refused.unmeasured_live, 1);
        assert_eq!(refused.headroom_ms(), None);
        assert_eq!(deck.priming_slots(), 0);
        assert!(
            deck.is_parked(karakuri_engine::DeckSlot(1)),
            "the refusal cancelled the request"
        );

        // The one call a caller owes at startup.
        assert_eq!(deck.measure_slots(&gpu.device, &gpu.queue), 1);
        assert_eq!(
            deck.measure_slots(&gpu.device, &gpu.queue),
            0,
            "a second call re-measured Sets that already had a measurement"
        );

        let admitted = deck.govern();
        assert!(admitted.committed_known());
        assert!(
            admitted.committed_ms > 0.0,
            "a measured Set was recorded as free"
        );
        assert!(admitted.headroom_ms().is_some());
        assert_eq!(
            admitted.decisions[1].reason,
            Reason::Fits,
            "the deck was measured and priming stayed refused"
        );
        assert_eq!(deck.priming_slots(), 1);
    }
    /// **Measuring is stepping, so a Set that has already stepped is left alone.**
    ///
    /// `swap::measure` ends in `Set::rewind`, which restores what `Set::build`
    /// left — cold, `t` at zero — rather than what it found. That is correct for
    /// the cold Set it is meant for and is a state reset for any other, so a
    /// `measure_slots` called late must skip a running slot rather than take an
    /// hour of simulation off it to fill in a budget figure. The governor already
    /// has a way to say it cannot budget a slot; it has no way to undo this.
    #[test]
    fn measuring_never_resets_a_slot_that_has_already_stepped() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        const FRAMES: usize = 6;

        let mut deck = Deck::new(&gpu.device, vec![swap_of(&gpu, L1, 1, None)], WIDTH, HEIGHT);
        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(0)).set()),
            FRAMES as u64
        );

        assert_eq!(
            deck.measure_slots(&gpu.device, &gpu.queue),
            0,
            "a running Set was measured, which rewinds it"
        );
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(0)).set()),
            FRAMES as u64,
            "measuring a running slot reset it to cold; `Set::rewind` restores what \
         `build` left, not what `measure` found"
        );

        // The negative control: the same deck before it ever ran does get measured,
        // so the skip above is about the Set having stepped and not about
        // `measure_slots` doing nothing at all.
        let mut cold = Deck::new(&gpu.device, vec![swap_of(&gpu, L1, 1, None)], WIDTH, HEIGHT);
        assert_eq!(cold.measure_slots(&gpu.device, &gpu.queue), 1);
        assert_eq!(
            steps_taken(cold.slot(karakuri_engine::DeckSlot(0)).set()),
            0
        );
    }

    /// **`estimate` reaches `Deck::govern`, end to end on a device.**
    ///
    /// Everything above pins the arithmetic on hand-built numbers, which is
    /// what the split in this file's header is for. What needs a device is the
    /// path: that `Deck::estimate_slots` draws each cold slot twice at its own
    /// output's size, that the answer is stored on the slot, and that the next
    /// `govern` is decided on it. A report nobody wired would satisfy every
    /// assertion in the first half of this file.
    ///
    /// **No timing is asserted and no fit is demanded**, for
    /// `tests/estimate.rs`' reason: this machine's timestamps demote to a host
    /// clock, so two draws at 32x32 and 64x64 are two noise figures and the
    /// slope through them comes out negative here — a real
    /// `Unfit::FragmentTermNegative`. So both outcomes are asserted, and that
    /// is not a weakened test: ADR-0296's fallback is exercised for real on
    /// this machine rather than simulated. The arithmetic of both branches is
    /// pinned above, on numbers no adapter chose.
    #[test]
    fn estimate_slots_gives_govern_a_number_at_the_decks_own_size() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = Deck::new(
            &gpu.device,
            vec![swap_of(&gpu, L1, 1, Some(12.0))],
            WIDTH,
            HEIGHT,
        );

        // Before: the slot has a measurement and nothing else, and the governor
        // budgets on it exactly as it always did.
        let before = deck.govern();
        assert_eq!(before.decisions[0].basis, Basis::Measured);
        assert_eq!(before.committed_ms, 12.0);
        assert_eq!(before.estimated(), 0);

        assert_eq!(deck.estimate_slots(&gpu.device, &gpu.queue), 1);
        assert_eq!(
            deck.estimate_slots(&gpu.device, &gpu.queue),
            0,
            "a second pass re-estimated a slot that already had one"
        );
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(0)).set()),
            0,
            "estimating left the Set stepped; `Set::rewind` restores what \
         `build` left, and a slot must arrive on air cold"
        );

        // Cloned rather than borrowed: `govern` below needs the deck mutably,
        // and the point of this test is the two readings side by side.
        let e = deck
            .slot(karakuri_engine::DeckSlot(0))
            .estimated_cost()
            .expect("estimate_slots stored one")
            .clone();
        assert_eq!(
            e.target,
            (WIDTH, HEIGHT),
            "the estimate answered for a size this deck is not drawing"
        );
        // `soft_points` above writes `point_rate = 0.03125`, a literal, so the
        // bound is exact and the floor is 32 rows — a quarter of this deck's
        // 128, which is where the cheap pair sits. Nothing is rounded up at
        // either rung and the fit is exact in ADR-0245's terms.
        assert_eq!(e.floor, Some(32));
        assert!(matches!(e.floor_from, Floor::Analysed { .. }));
        assert_eq!(e.floored, None);
        // **Whether it fitted is this machine's business and not this test's.**
        // `docs/contributing.md` §1: the probe demotes to a host clock here, so
        // two draws at 32x32 and 64x64 are two noise figures — and on this
        // crate's development machine the upper rung reads *cheaper* than the
        // lower one, which is `Unfit::FragmentTermNegative`, a failed
        // measurement rather than a cheap Set. Both outcomes are asserted,
        // because both are the wiring working: an answer is budgeted on, and a
        // refusal falls back to the measurement and says why. A threshold on
        // the number would be a test of the adapter.
        let after = deck.govern();
        let d = after.decisions[0];
        match e.ms() {
            Some(ms) => {
                assert_eq!(d.basis, Basis::Estimated);
                assert_eq!(d.budgeted_ms, Some(ms));
                assert_eq!(after.committed_ms, ms);
                assert_eq!(after.estimated(), 1);
                assert_eq!(after.estimated_target(), Some((WIDTH, HEIGHT)));
                assert_eq!(after.refused_estimates().count(), 0);
            }
            None => {
                assert_eq!(d.basis, Basis::Measured);
                assert_eq!(
                    d.budgeted_ms,
                    Some(12.0),
                    "a refused estimate changed what the slot was budgeted at"
                );
                assert_eq!(after.committed_ms, 12.0);
                assert_eq!(after.estimated(), 0);
                let refusals: Vec<_> = after.refused_estimates().collect();
                assert_eq!(
                    refusals.len(),
                    1,
                    "the estimate refused and the report does not say so"
                );
                assert_eq!(refusals[0].0, 0);
            }
        }
        assert_eq!(
            d.cost_ms,
            Some(12.0),
            "the measurement was overwritten rather than kept beside it"
        );
        assert_eq!(
            d.estimate.expect("carried either way").floor_from,
            FloorRead::Analysed,
            "the floor's provenance did not survive the summary"
        );

        // **A resize drops it**, because the size the answer was for has moved.
        // Without this the deck would go on budgeting a 128x128 number against
        // a frame twice as wide.
        deck.resize(&gpu.device, WIDTH * 2, HEIGHT * 2);
        assert!(deck
            .slot(karakuri_engine::DeckSlot(0))
            .estimated_cost()
            .is_none());
        let resized = deck.govern();
        assert_eq!(
            resized.decisions[0].basis,
            Basis::Measured,
            "an estimate for the old size survived the resize"
        );
    }
}
