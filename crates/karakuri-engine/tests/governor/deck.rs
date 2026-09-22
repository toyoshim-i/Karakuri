use super::common::*;

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

use crate::engine_common::compile;

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

/// The master chain is reserved out of the budget ahead of every slot, and it
/// is charged against no one of them.
///
/// A chain is one pass per slot over the whole frame and belongs to no deck, so
/// what it costs leaves the headroom a priming request is judged against and
/// never appears in `committed_ms` — a slot that fitted before the chain was
/// added may not fit after it.
#[test]
fn the_master_chain_is_reserved_ahead_of_the_slots() {
    let mut governor = Governor::new(16.0);
    let free = governor.decide(&[live(10.0), priming(5.0)]);
    assert_eq!(free.chain_ms, 0.0, "an empty chain cost something");
    assert_eq!(free.spendable_ms(), 16.0);
    assert_eq!(free.headroom_ms(), Some(6.0));
    assert_eq!(
        free.decisions[1].reason,
        Reason::Fits,
        "5 ms into 6 ms of headroom did not fit"
    );

    governor.set_chain_ms(2.0);
    let charged = governor.decide(&[live(10.0), priming(5.0)]);
    assert_eq!(charged.chain_ms, 2.0);
    assert_eq!(
        charged.committed_ms, 10.0,
        "the chain was summed into the live slots"
    );
    assert_eq!(charged.spendable_ms(), 14.0);
    assert_eq!(charged.headroom_ms(), Some(4.0));
    assert_eq!(
        charged.decisions[1].reason,
        Reason::NoHeadroom,
        "the chain's 2 ms did not come out of what a request is judged against"
    );
    assert_eq!(
        charged.decisions[0].effective,
        Residency::Live,
        "the governor took a live slot off air to pay for the chain"
    );
    assert!(charged.to_string().contains("chain"), "{charged}");

    // And it can put a deck over budget on its own.
    governor.set_chain_ms(12.0);
    let over = governor.decide(&[live(10.0)]);
    assert!(
        over.over_budget,
        "10 ms of live under a 12 ms chain is not flagged"
    );
    assert_eq!(
        over.decisions[0].effective,
        Residency::Live,
        "the governor took a live slot off air"
    );
}

/// What a chain costs is its ops against the frame's area, linear in both, and
/// the rate is calibrated on one measurement whose resolution is named.
///
/// The engine-side half — that a `Deck` charges what its `Present` is running,
/// and that the governor spends it — is in `src/frame.rs`'s
/// `a_composed_frame_hands_the_chain_its_clock_and_the_deck_its_price`.
#[test]
fn a_chains_price_is_linear_in_its_ops_and_in_the_frames_area() {
    use karakuri_engine::estimate::{
        chain_ms, CHAIN_REFERENCE_MS, CHAIN_REFERENCE_OPS, CHAIN_REFERENCE_SIZE,
    };

    assert_eq!(
        chain_ms(0, CHAIN_REFERENCE_SIZE),
        0.0,
        "an empty chain is free"
    );
    let reference = chain_ms(CHAIN_REFERENCE_OPS, CHAIN_REFERENCE_SIZE);
    assert!(
        (reference - CHAIN_REFERENCE_MS).abs() < 1e-3,
        "the rate does not return the measurement it was calibrated on: {reference}"
    );
    assert!(
        (chain_ms(CHAIN_REFERENCE_OPS / 2, CHAIN_REFERENCE_SIZE) - reference / 2.0).abs() < 1e-3,
        "half the ops is not half the price"
    );
    assert!(
        (chain_ms(CHAIN_REFERENCE_OPS, (640, 360)) - reference / 4.0).abs() < 1e-3,
        "a quarter of the area is not a quarter of the price"
    );
    assert!(
        chain_ms(u32::MAX, (u32::MAX, u32::MAX)).is_finite(),
        "an absurd chain priced at something that is not a number"
    );
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

        // **A resize re-reads it at the new size rather than dropping it**
        // (ADR-0356). ADR-0296 dropped it, because a number taken for 128x128
        // is the wrong number for a frame twice as wide — true of the number
        // and not of the estimate: the fit and the rungs it came from are on
        // the record, so the answer at the new target is arithmetic and no
        // draw. Dropping it put every slot back on its measurement for every
        // frame of a drag.
        deck.resize(&gpu.device, WIDTH * 2, HEIGHT * 2);
        let kept = deck
            .slot(karakuri_engine::DeckSlot(0))
            .estimated_cost()
            .expect("a resize dropped the estimate instead of re-reading it")
            .clone();
        assert_eq!(
            kept.target,
            (WIDTH * 2, HEIGHT * 2),
            "the estimate still answers for the size the deck left"
        );
        assert_eq!(
            kept.rungs, e.rungs,
            "the re-read drew again; the rungs are data already taken"
        );
        assert_eq!(kept.floor, e.floor);
        let resized = deck.govern();
        assert_eq!(
            resized.decisions[0].basis,
            match kept.ms() {
                Some(_) => Basis::Estimated,
                None => Basis::Measured,
            },
            "the re-read estimate is not what the slot was budgeted on"
        );
        assert_eq!(
            resized.estimated_target(),
            kept.ms().map(|_| (WIDTH * 2, HEIGHT * 2)),
            "the report names a size the estimates did not answer for"
        );
    }
}
