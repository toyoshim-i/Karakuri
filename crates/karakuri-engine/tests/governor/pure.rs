use super::common::*;

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
