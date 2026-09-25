use super::common::*;

// Synthetic rungs verifying linear cost fitting within Governor::decide.

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

/// Helper creating a synthetic estimate for `target` costing `a + fragment_ms` under `floor`.
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

/// Verifies that a priming slot is demoted to allocated when committed cost exceeds headroom.
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

/// Verifies that a candidate exceeding headroom is parked rather than partially stepped (ADR-0269).
#[test]
fn a_candidate_that_does_not_fit_is_parked_rather_than_slowed() {
    let report = Governor::new(20.0).decide(&[live(16.0), priming(10.0)]);

    let decision = report.decisions[1];
    assert_eq!(decision.effective, Residency::Allocated);
    assert_eq!(decision.reason, Reason::NoHeadroom);
    assert!(decision.is_parked(), "the request was not held");
    assert_eq!(report.priming_ms, 0.0);
}

/// Verifies that priming slots are admitted in index order against remaining headroom.
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

/// Verifies that closed-form sets are not primed and report NoPrimingNeeded.
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

/// Verifies that unmeasured priming slots are refused rather than treated as zero cost.
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

/// Verifies that priming is refused while any active live slot remains unmeasured.
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

/// Verifies distinct reasons and display messages for unknown vs exhausted commitment.
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

/// Verifies that headroom_ms returns None when commitment is unknown.
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

/// Verifies that an over-budget condition is reported even if unmeasured live slots are present.
#[test]
fn over_budget_still_fires_when_a_live_slot_is_unmeasured() {
    let report = Governor::new(16.0).decide(&[live(20.0), unmeasured_live()]);

    assert!(report.over_budget);
    assert!(!report.committed_known());
    assert_eq!(report.headroom_ms(), None);
}

/// Verifies that a parked slot preserves its requested residency in the report.
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

/// Verifies that governor prioritizes estimates over measurements when evaluating admission.
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

/// Verifies that a refused estimate falls back to the measurement basis.
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

/// Verifies that a refused estimate without measurement is treated as unbudgetable.
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

/// Verifies that floor details and corrections are preserved and applied once without duplication.
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

/// Verifies that an unknown or contradicted floor is tracked and reported accurately.
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

/// Verifies that host clock bias on estimate rungs is propagated into the report caveat.
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

/// Verifies that budgeted estimates provide valid numerical inputs for UI performance bands.
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
