//! The budget governor: what may prime, how fast, and what it refuses to
//! touch.
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

use karakuri_engine::deck::{Deck, Residency};
use karakuri_engine::governor::{Governor, Reason, SlotState};
use karakuri_engine::probe::{Measurement, MeasurementMethod};
use karakuri_engine::swap::HotSwap;
use karakuri_engine::{Gpu, Present, Set};
use karakuri_ir::typed::Checked;

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
        closed_form: false,
    }
}

fn priming(ms: f32) -> SlotState {
    SlotState {
        requested: Residency::Priming,
        cost: Some(cost(ms)),
        closed_form: false,
    }
}

/// A Live slot nothing has measured — every `HotSwap::fixed` Set until
/// `Deck::measure_slots` runs. Its cost is not zero, it is unknown.
fn unmeasured_live() -> SlotState {
    SlotState {
        requested: Residency::Live,
        cost: None,
        closed_form: false,
    }
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
/// that fits admits the same slot at full rate. Without this, "demoted when
/// there is no room" would also pass on a governor that demoted everything.
#[test]
fn the_same_slot_primes_at_full_rate_when_there_is_room() {
    let report = Governor::new(60.0).decide(&[live(12.0), priming(40.0)]);

    assert_eq!(report.decisions[1].effective, Residency::Priming);
    assert_eq!(report.decisions[1].prime_one_in, 1);
    assert_eq!(report.decisions[1].reason, Reason::Fits);
    assert_eq!(report.priming_ms, 40.0);
}

/// **Between the two, it slows down rather than refusing.** 16 ms committed
/// against a 20 ms budget leaves 4; a 10 ms candidate does not fit at full rate
/// but does at one frame in three, and the amortised cost charged against the
/// budget is 10/3.
#[test]
fn a_candidate_that_does_not_fit_at_full_rate_is_slowed_rather_than_parked() {
    let report = Governor::new(20.0).decide(&[live(16.0), priming(10.0)]);

    let decision = report.decisions[1];
    assert_eq!(decision.effective, Residency::Priming);
    assert_eq!(
        decision.prime_one_in, 3,
        "10 ms into 4 ms of headroom is one frame in three, not one in {}",
        decision.prime_one_in
    );
    assert_eq!(decision.reason, Reason::Slowed);
    assert!((report.priming_ms - 10.0 / 3.0).abs() < 1e-4);
}

/// **Two priming slots share what is left, in index order**, and the second one
/// is decided against the headroom the first already spent. Order is by index
/// and nothing else, for the same reason the composite's sum order is: an
/// answer that depended on which slot last had a build land on it would not
/// reproduce.
#[test]
fn priming_slots_are_admitted_in_index_order_against_a_shrinking_headroom() {
    let report = Governor::new(20.0).decide(&[live(10.0), priming(8.0), priming(8.0)]);

    assert_eq!(
        report.decisions[1].prime_one_in, 1,
        "the first one fits whole"
    );
    assert_eq!(report.decisions[1].reason, Reason::Fits);
    // 2 ms left after the first; 8/4 is 2, which fits exactly.
    assert_eq!(report.decisions[2].prime_one_in, 4);
    assert_eq!(report.decisions[2].reason, Reason::Slowed);

    // The same 8 ms candidate behind a cheaper neighbour gets a different rate,
    // which is what makes "in index order, against a shrinking headroom" a
    // claim rather than a description: the answer for a slot depends on what
    // was decided before it, and "before" is by index.
    let cheaper_first = Governor::new(20.0).decide(&[live(10.0), priming(4.0), priming(8.0)]);
    assert_eq!(cheaper_first.decisions[1].prime_one_in, 1);
    assert_eq!(
        cheaper_first.decisions[2].prime_one_in, 2,
        "6 ms of headroom takes an 8 ms candidate at one frame in two"
    );
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
        closed_form: false,
    };
    let report = Governor::new(10_000.0).decide(&[parked]);

    assert_eq!(report.decisions[0].effective, Residency::Allocated);
    assert_eq!(report.decisions[0].reason, Reason::OffAir);
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

/// **A rate can spread a cost; it cannot shrink one.**
///
/// The amortisation is an average — a slot stepping one frame in `n` costs
/// `cost / n` *on average* and the whole `cost` on the frames it steps. Without
/// a cap on the unamortised figure, a Set measured at six times the entire
/// budget divides by eight, reads as fitting, and buys a hundred-millisecond
/// hidden step every eighth frame on a deck that has nothing else on it. The
/// budget answers "is there room to step one more simulation", and for this Set
/// there is no rate at which there is.
#[test]
fn a_slot_costing_more_than_the_whole_budget_is_refused_at_every_rate() {
    let report = Governor::new(16.7).decide(&[priming(100.0)]);

    assert_eq!(
        report.decisions[0].effective,
        Residency::Allocated,
        "a Set costing six times the whole budget was admitted to prime; 100/8 is \
         12.5 and the frames it steps still cost 100"
    );
    assert_eq!(report.decisions[0].reason, Reason::NoHeadroom);
    assert_eq!(report.priming_ms, 0.0);

    // The negative control, one step either side of the cap: the same empty
    // deck admits a Set that does fit in one frame.
    let fits = Governor::new(16.7).decide(&[priming(16.0)]);
    assert_eq!(fits.decisions[0].effective, Residency::Priming);
    assert_eq!(fits.decisions[0].prime_one_in, 1);
}

// The seven of twenty-two that take a device. The rest reason over a budget and
// a verdict, which is arithmetic — so the majority of this file stays in the set
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
        deck.set_residency(1, Residency::Priming);

        let report = deck.govern();
        assert_eq!(report.decisions[1].reason, Reason::NoHeadroom);
        assert_eq!(
            deck.residency(1),
            Residency::Allocated,
            "the governor decided to demote and the deck did not move"
        );
        assert_eq!(
            deck.residency(0),
            Residency::Live,
            "the governor moved a Live slot"
        );

        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(0).set()),
            FRAMES as u64,
            "the Live slot stopped stepping"
        );
        assert_eq!(
            steps_taken(deck.slot(1).set()),
            0,
            "the demoted slot is still stepping, so the demotion was a report and not \
         an action"
        );

        // The negative control on the same deck: raise the budget and the same slot
        // is admitted and does step. Without this, the assertions above would pass
        // on a `govern` that parked everything it was ever shown. Nothing re-asks
        // for priming here — the request outlived the demotion, which is
        // `a_parked_slot_primes_again_by_itself_when_the_deck_empties`'s subject.
        deck.set_compute_budget_ms(100.0);
        assert_eq!(deck.govern().decisions[1].reason, Reason::Fits);
        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(steps_taken(deck.slot(1).set()), FRAMES as u64);
        assert_eq!(steps_taken(deck.slot(0).set()), 2 * FRAMES as u64);
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
            deck.set_residency(0, Residency::Priming);
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
    /// **`govern` is idempotent, so calling it often does not stall priming.**
    ///
    /// A caller that runs it every frame — which is the shape a status line
    /// invites — must not reset the priming counter every frame, because
    /// `prime_phase = 0` on every call means "one frame in four" becomes "every
    /// frame" and the budget the governor just computed is spent four times over.
    /// The reset belongs to a *change*, and this is what says so.
    #[test]
    fn calling_govern_every_frame_does_not_stall_a_slowed_priming_slot() {
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
        // 8 ms on air against 10 leaves 2, which takes the 8 ms candidate at one
        // frame in four.
        deck.set_compute_budget_ms(10.0);
        deck.set_residency(1, Residency::Priming);
        assert_eq!(deck.govern().decisions[1].prime_one_in, 4);

        for _ in 0..FRAMES {
            let report = deck.govern();
            assert_eq!(
                report.decisions[1].prime_one_in, 4,
                "the rate changed under a repeated call with nothing else changing"
            );
            frame(&gpu, &mut deck, &present, 1);
        }

        assert_eq!(
            steps_taken(deck.slot(1).set()),
            (FRAMES / 4) as u64,
            "a slot priming one frame in four stepped more often than that; `govern` \
         is resetting the counter it should only reset on a change"
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
        // 15 ms on air against 16 leaves 1 ms, and 16/8 is 2: no rate fits. The
        // same 16 ms candidate on an empty deck fits whole, which is what makes
        // this a park rather than a Set that is simply too expensive.
        deck.set_compute_budget_ms(16.0);
        deck.set_residency(1, Residency::Priming);

        assert_eq!(deck.govern().decisions[1].reason, Reason::NoHeadroom);
        assert_eq!(deck.residency(1), Residency::Allocated);
        assert!(
            deck.is_parked(1),
            "a refused request reads as a slot nobody asked about"
        );
        assert_eq!(deck.parked_slots(), 1);
        assert_eq!(
            deck.requested_residency(1),
            Residency::Priming,
            "the demotion destroyed the operator's request"
        );

        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(steps_taken(deck.slot(1).set()), 0, "a parked slot stepped");

        // The deck empties. Slot 1 is not mentioned.
        deck.set_residency(0, Residency::Allocated);
        let report = deck.govern();

        assert_eq!(
            report.decisions[1].reason,
            Reason::Fits,
            "the slot never primed again after the deck emptied; the request did not \
         survive the demotion"
        );
        assert_eq!(deck.residency(1), Residency::Priming);
        assert!(!deck.is_parked(1));
        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(1).set()),
            FRAMES as u64,
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
        deck.set_residency(1, Residency::Priming);
        deck.set_residency(2, Residency::Priming);
        deck.set_residency(3, Residency::Allocated);

        let before = deck.govern();
        assert!(!before.over_budget);
        assert_eq!(deck.priming_slots(), 2);

        // The heavy Set goes on air: 34 ms committed against 16.
        deck.set_residency(3, Residency::Live);
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
            assert_eq!(deck.requested_residency(slot), Residency::Priming);
        }

        // And off again. Nothing is re-requested.
        deck.set_residency(3, Residency::Allocated);
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
        deck.set_residency(1, Residency::Priming);

        let refused = deck.govern();
        assert_eq!(
            refused.decisions[1].reason,
            Reason::CommittedUnknown,
            "a candidate was admitted against an on-air cost nobody has measured"
        );
        assert_eq!(refused.unmeasured_live, 1);
        assert_eq!(refused.headroom_ms(), None);
        assert_eq!(deck.priming_slots(), 0);
        assert!(deck.is_parked(1), "the refusal cancelled the request");

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
        assert_eq!(steps_taken(deck.slot(0).set()), FRAMES as u64);

        assert_eq!(
            deck.measure_slots(&gpu.device, &gpu.queue),
            0,
            "a running Set was measured, which rewinds it"
        );
        assert_eq!(
            steps_taken(deck.slot(0).set()),
            FRAMES as u64,
            "measuring a running slot reset it to cold; `Set::rewind` restores what \
         `build` left, not what `measure` found"
        );

        // The negative control: the same deck before it ever ran does get measured,
        // so the skip above is about the Set having stepped and not about
        // `measure_slots` doing nothing at all.
        let mut cold = Deck::new(&gpu.device, vec![swap_of(&gpu, L1, 1, None)], WIDTH, HEIGHT);
        assert_eq!(cold.measure_slots(&gpu.device, &gpu.queue), 1);
        assert_eq!(steps_taken(cold.slot(0).set()), 0);
    }
}
