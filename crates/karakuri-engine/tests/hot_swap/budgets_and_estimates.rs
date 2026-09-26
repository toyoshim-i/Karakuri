use std::time::Instant;

use super::common::*;

mod gpu {
    use super::*;

    #[allow(dead_code)]
    fn channel_driven() {
        let _ = Gpu::headless();
    }

    #[allow(dead_code)]
    fn channel_driven_at() {
        let _ = Gpu::headless();
    }

    #[allow(dead_code)]
    fn new() {
        let _ = Gpu::headless();
    }

    /// Verifies that an over-budget candidate replaces the running set while marking the slot stopped (ADR-0316).
    #[test]
    fn an_over_budget_candidate_stays_and_the_slot_is_marked_stopped() {
        // Nothing is faster than zero milliseconds, so every candidate fails.
        let (mut h, tx) = Harness::channel_driven(0.0);

        for _ in 0..10 {
            h.frame();
        }
        tx.send(request(L4, SECOND, "second"))
            .expect("worker alive");

        assert!(
            !h.swap.overloaded(),
            "a slot with nothing judged in it reports itself stopped"
        );

        let mut swapped = false;
        let mut seen: Vec<String> = Vec::new();
        let mut verdict = None;
        let started = Instant::now();
        while !swapped {
            h.frame();
            for event in h.swap.events() {
                if is_swapped(&event) {
                    swapped = true;
                }
                if let Event::Overloaded { cost_ms, basis, .. } = &event {
                    verdict = Some((*cost_ms, *basis));
                }
                seen.push(event.to_string());
            }
            assert!(started.elapsed() < PATIENCE, "the swap never happened");
        }

        let (cost_ms, basis) = verdict
            .unwrap_or_else(|| panic!("the swap landed and no verdict came with it: {seen:?}"));
        // **The candidate is what is in the slot.** `FIRST` here would be the
        // old behaviour exactly: a Set the operator did not ask for, put back
        // by the engine, with the file on disk still holding the one they did.
        assert_eq!(
            h.swap.set().capacity(),
            SECOND,
            "the verdict took the operator's material out of the slot"
        );
        assert!(
            h.swap.overloaded(),
            "a candidate over the budget left the slot unmarked, so nothing \
             downstream can know to stop stepping it"
        );
        // **The number is the candidate's own frame and not an interval.** A
        // frame interval on this harness is milliseconds of a real submit and a
        // real poll; what this has to be is the probe's reading of the Set that
        // just arrived, which is a positive finite number the worker took.
        assert!(
            cost_ms > 0.0 && cost_ms.is_finite(),
            "the verdict decided on {cost_ms}, which is not a cost"
        );
        // Verifies the verdict carries the basis reported by `governor::budgeted` (ADR-0298).
        let (spent, spent_ms) = budgeted_by_the_one_rule(&h.swap);
        assert_eq!(
            (basis, Some(cost_ms)),
            (spent, spent_ms),
            "the verdict does not carry the number `governor::budgeted` spends for this slot"
        );
        assert_ne!(
            basis,
            Basis::Unbudgetable,
            "a candidate the worker both measured and estimated was judged on nothing"
        );

        let said = seen
            .iter()
            .find(|s| s.contains("overloaded"))
            .unwrap_or_else(|| panic!("no overloaded message: {seen:?}"));
        assert!(
            said.contains(match basis {
                Basis::Estimated => "two-draw fit",
                _ => "host clock",
            }),
            "the message does not say what kind of number it decided on: {said}"
        );
        assert!(
            said.contains("its own frame"),
            "the message still describes the number as a frame interval: {said}"
        );
        // **The sentence says the state and not an action taken.** A reader of
        // this line has a slot to attend to, so the words that have to be in it
        // are the ones that say the material is still there and stopped.
        assert!(
            said.contains("still in the slot") && said.contains("stopped updating"),
            "the message does not say what happened to the slot: {said}"
        );

        // **A build that fits clears the freeze**, which is the third way out
        // and the only one the engine takes by itself. The budget is what makes
        // the same machinery answer differently, so it is the budget that moves.
        h.swap.set_budget_ms(GENEROUS_MS);
        tx.send(request(L4, FIRST, "third")).expect("worker alive");
        h.frames_until(
            |e| matches!(e, Event::Accepted { .. }),
            "a candidate that fits",
        );
        assert!(
            !h.swap.overloaded(),
            "the slot is still marked stopped after a build that held the budget"
        );
        assert_eq!(
            h.swap.set().capacity(),
            FIRST,
            "the build that cleared the freeze is not the live Set"
        );
    }

    /// Verifies candidate budgeting evaluates against relative measured costs rather than arbitrary constants.
    #[test]
    fn an_over_budget_candidate_is_stopped_on_a_budget_derived_from_its_own_measurement() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);
        for _ in 0..5 {
            h.frame();
        }
        tx.send(request(L4, SECOND, "measured"))
            .expect("worker alive");
        h.frames_until(is_swapped, "the swap");
        let measured = h
            .swap
            .measured_cost()
            .expect("the worker measures what it builds")
            .ms;
        let (_, spent) = budgeted_by_the_one_rule(&h.swap);
        let spent = spent.expect("a candidate the worker read is budgetable");
        assert!(
            measured > 0.0 && measured.is_finite(),
            "the candidate arrived with {measured} ms, which is not a cost"
        );
        assert!(
            spent > 0.0 && spent.is_finite(),
            "the slot is budgeted at {spent} ms, which is not a cost"
        );
        assert_eq!(
            h.swap.set().capacity(),
            SECOND,
            "a generous budget threw the candidate out anyway"
        );

        // An eighth of measured cost ensures the budget is reliably exceeded across headless devices.
        let (mut tight, tx) = Harness::channel_driven(spent / 8.0);
        for _ in 0..5 {
            tight.frame();
        }
        tx.send(request(L4, SECOND, "over")).expect("worker alive");

        let mut verdict = None;
        let mut seen: Vec<String> = Vec::new();
        let started = Instant::now();
        while verdict.is_none() {
            tight.frame();
            for event in tight.swap.events() {
                if let Event::Overloaded { cost_ms, .. } = &event {
                    verdict = Some(*cost_ms);
                }
                seen.push(event.to_string());
            }
            assert!(
                started.elapsed() < PATIENCE,
                "no verdict on a candidate that cannot fit half its own cost: {seen:?}"
            );
        }
        let stopped_at = verdict.expect("just set");
        // **And the verdict spent the slot's own number**, whichever of the two
        // the one rule picked. Exact rather than banded: the candidate was
        // kept, so the readings that travelled with it are the ones the slot is
        // holding now.
        assert_eq!(
            budgeted_by_the_one_rule(&tight.swap).1,
            Some(stopped_at),
            "the verdict decided on a number that is not what this slot is budgeted at"
        );
        // **Kept, and stopped.** The candidate is the live Set — a budget below
        // its own cost takes nothing away — and the flag is what says the slot
        // is not to be stepped.
        assert_eq!(
            tight.swap.set().capacity(),
            SECOND,
            "a budget below the candidate's own measured cost took it out of the slot"
        );
        assert!(
            tight.swap.overloaded(),
            "a candidate that cannot fit half its own measured cost left the slot unmarked"
        );
        // Verifies the measurement is repeatable within host clock noise tolerances.
        let again = tight
            .swap
            .measured_cost()
            .expect("the worker measures what it builds")
            .ms;
        assert!(
            again > measured / 4.0 && again < measured * 4.0,
            "the same Set measured {again} ms where it measured {measured} ms a \
             moment earlier — that is not this Set's own cost"
        );
    }

    /// A candidate that fits is kept **and runs** — the other branch of the same
    /// verdict, and the one that has to work for a hot swap to be useful rather
    /// than merely safe. Since ADR-0316 both branches keep the candidate, so
    /// what separates them is [`HotSwap::overloaded`] and nothing else.
    #[test]
    fn a_candidate_that_holds_the_budget_is_kept() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

        for _ in 0..5 {
            h.frame();
        }
        tx.send(request(L4, SECOND, "second"))
            .expect("worker alive");

        // The swap and its verdict arrive in one drain (ADR-0313), so they are
        // read out of one loop — asking `frames_until` for the second after it
        // has already consumed the first would wait for ever.
        let mut verdict = None;
        let mut seen: Vec<String> = Vec::new();
        let started = Instant::now();
        while verdict.is_none() {
            h.frame();
            for event in h.swap.events() {
                if let Event::Accepted { cost_ms, basis, .. } = &event {
                    verdict = Some((*cost_ms, *basis));
                }
                seen.push(event.to_string());
            }
            assert!(started.elapsed() < PATIENCE, "no verdict: {seen:?}");
        }
        let (cost_ms, basis) = verdict.expect("just set");

        assert_eq!(h.swap.set().capacity(), SECOND, "the candidate was kept");
        assert!(
            seen.iter().any(|s| s.contains("held the budget")),
            "no acceptance message: {seen:?}"
        );
        // Verifies the verdict cost matches the Set's own budgeted cost (ADR-0356).
        let (spent, spent_ms) = budgeted_by_the_one_rule(&h.swap);
        assert_eq!(
            (cost_ms, basis),
            (spent_ms, spent),
            "the verdict was reached on a number that is not this Set's own cost"
        );
        assert_ne!(
            basis,
            Basis::Unbudgetable,
            "a candidate the worker both measured and estimated was judged on nothing"
        );
    }

    /// Verifies that swapped-in Sets arrive with estimates evaluated at output size (ADR-0356).
    #[test]
    fn a_swapped_in_set_arrives_estimated_at_the_output_size_and_is_judged_on_it() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);
        for _ in 0..5 {
            h.frame();
        }
        assert!(
            h.swap.estimated_cost().is_none(),
            "the Set the harness was constructed with was never built by a worker, \
             so nothing can have estimated it"
        );
        // **The size the worker is to answer for is the slot's viewport**, and
        // not the size it measures at. The harness's `HotSwap` was seeded from
        // a Set `Set::build` left at 1x1 and then resized, which is what a deck
        // does to every slot it holds.
        assert_eq!(h.swap.estimate_size(), (WIDTH, HEIGHT));
        assert_ne!(
            h.swap.estimate_size(),
            h.swap.measure_size(),
            "this harness no longer separates the two sizes, so it cannot show \
             that the estimate answered for the right one"
        );

        tx.send(request(L4, SECOND, "estimated"))
            .expect("worker alive");
        let mut verdict = None;
        let started = Instant::now();
        let mut seen: Vec<String> = Vec::new();
        while verdict.is_none() {
            h.frame();
            for event in h.swap.events() {
                if let Event::Accepted { cost_ms, basis, .. } = &event {
                    verdict = Some((*cost_ms, *basis));
                }
                seen.push(event.to_string());
            }
            assert!(started.elapsed() < PATIENCE, "no verdict: {seen:?}");
        }
        let (cost_ms, basis) = verdict.expect("just set");

        let e = h
            .swap
            .estimated_cost()
            .expect("the worker estimates what it builds")
            .clone();
        assert_eq!(
            e.target,
            (WIDTH, HEIGHT),
            "the estimate answered for a size this slot is not drawing"
        );
        // `soft_points` above writes `point_rate = 0.015625`, a literal, so the
        // bound is exact and the floor is 64 rows — a quarter of this harness's
        // 256, which is where the cheap pair sits. Nothing is rounded up at
        // either rung, so there is no correction to carry.
        assert_eq!(e.floor, Some(64));
        assert!(
            matches!(
                e.floor_from,
                karakuri_engine::estimate::Floor::Analysed { .. }
            ),
            "the floor did not come from the rate analysis: {:?}",
            e.floor_from
        );
        assert_eq!(e.floored, None);
        assert_eq!(
            e.rungs.expect("two rungs were drawn").map(|m| m.resolution),
            [(64, 64), (128, 128)],
            "the rungs were not placed against the output's size"
        );

        // **And the verdict spent it, or said why it did not.** One rule, and
        // it is the same one a cold slot is governed by.
        let (spent, spent_ms) = budgeted_by_the_one_rule(&h.swap);
        assert_eq!((cost_ms, basis), (spent_ms, spent));
        match e.ms() {
            Some(ms) => {
                assert_eq!(basis, Basis::Estimated);
                assert_eq!(cost_ms, Some(ms));
                assert!(seen.iter().any(|s| s.contains("two-draw fit")), "{seen:?}");
            }
            None => {
                assert_eq!(basis, Basis::Measured);
                assert_eq!(cost_ms, h.swap.measured_cost().map(|c| c.ms));
                assert!(seen.iter().any(|s| s.contains("host clock")), "{seen:?}");
            }
        }
        // **The measurement is kept beside it rather than overwritten.** A slot
        // that fell back must not read like one nothing ever asked (ADR-0296
        // §2), and a status line saying *measured* has to have a measurement.
        assert!(h.swap.measured_cost().is_some());
        // And estimating left no trace, exactly as measuring does not: the
        // rungs each end in `Set::rewind`, so the Set still arrives cold.
        assert_eq!(
            steps_taken(h.swap.set()),
            1,
            "the swapped-in Set arrived having already been stepped; a rung the \
             estimate drew was not rewound"
        );
    }

    /// Verifies that candidate budgeting falls back to measured cost when estimation is refused (ADR-0296).
    #[test]
    fn a_candidate_the_estimator_refuses_is_judged_on_its_measurement() {
        let (mut h, tx) = Harness::channel_driven_at(GENEROUS_MS, FIRST, TINY);
        for _ in 0..5 {
            h.frame();
        }
        assert_eq!(h.swap.estimate_size(), TINY);
        tx.send(request(L4, SECOND, "refused"))
            .expect("worker alive");

        let mut verdict = None;
        let mut seen: Vec<String> = Vec::new();
        let started = Instant::now();
        while verdict.is_none() {
            h.frame();
            for event in h.swap.events() {
                if let Event::Accepted { cost_ms, basis, .. } = &event {
                    verdict = Some((*cost_ms, *basis));
                }
                seen.push(event.to_string());
            }
            assert!(started.elapsed() < PATIENCE, "no verdict: {seen:?}");
        }
        let (cost_ms, basis) = verdict.expect("just set");

        let e = h
            .swap
            .estimated_cost()
            .expect("a refusal is a value and travels with the build");
        assert_eq!(
            e.fit,
            Err(Unfit::NoRoomBelowTheTarget {
                floor: 64,
                target: TINY
            }),
            "a 2x2 output found room for two rungs above a 64-row floor"
        );
        assert_eq!(e.target, TINY);
        assert_eq!(e.floor, Some(64));
        assert_eq!(
            e.rungs, None,
            "a target that holds no rungs refuses before it draws, and this one drew"
        );
        assert_eq!(e.ms(), None, "a refusal handed out a number");

        // **So the measurement decides**, and says so.
        assert_eq!(basis, Basis::Measured);
        assert_eq!(
            cost_ms,
            h.swap.measured_cost().map(|c| c.ms),
            "the refusal was spent as though it were an answer"
        );
        assert_eq!((cost_ms, basis), {
            let (b, ms) = budgeted_by_the_one_rule(&h.swap);
            (ms, b)
        });
        assert!(
            seen.iter().any(|s| s.contains("host clock")),
            "the message does not say the number was a measurement: {seen:?}"
        );
    }

    /// Verifies that resizing re-evaluates retained estimates at the new target dimensions (ADR-0356).
    #[test]
    fn a_resize_after_a_swap_re_targets_the_estimate_rather_than_dropping_it() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);
        for _ in 0..5 {
            h.frame();
        }
        tx.send(request(L4, SECOND, "resized"))
            .expect("worker alive");
        h.frames_until(is_swapped, "the swap");

        let before = h
            .swap
            .estimated_cost()
            .expect("the worker estimates what it builds")
            .clone();
        assert_eq!(before.target, (WIDTH, HEIGHT));

        let (wide, tall) = (WIDTH * 2, HEIGHT * 2);
        let device = h.gpu.device.clone();
        h.swap.resize(&device, wide, tall);

        let after = h
            .swap
            .estimated_cost()
            .expect("a resize dropped the estimate instead of re-reading it")
            .clone();
        assert_eq!(
            after.target,
            (wide, tall),
            "the estimate still answers for the size the window left"
        );
        // The worker is told too, so the *next* build's rungs are placed
        // against the frame this slot now draws.
        assert_eq!(h.swap.estimate_size(), (wide, tall));
        // Nothing was drawn to get there: the same two rungs, unchanged.
        assert_eq!(after.rungs, before.rungs);
        assert_eq!(after.floor, before.floor);
        assert_eq!(after.floor_from, before.floor_from);

        // **And it is the fit that moved, not the number.** Four times the
        // area, so the fragment term is four times what it was and the
        // invariant term is untouched — which is `a + b·area` re-read rather
        // than a number carried across.
        match (before.fit, after.fit) {
            (Ok(was), Ok(now)) => {
                assert!(
                    (now.invariant_ms - was.invariant_ms).abs() < 1e-4,
                    "the invariant term moved with the target: {} then {}",
                    was.invariant_ms,
                    now.invariant_ms
                );
                assert_eq!(now.ms_per_pixel, was.ms_per_pixel);
                let area = f64::from(wide) * f64::from(tall);
                let predicted = f64::from(now.invariant_ms) + now.ms_per_pixel * area;
                assert!(
                    (f64::from(now.ms) - predicted).abs() < 1e-3,
                    "the kept estimate is {} ms where its own fit at this size is \
                     {predicted} ms, so it was relabelled rather than re-read",
                    now.ms
                );
            }
            (Err(was), Err(now)) => assert_eq!(
                was, now,
                "a refusal changed kind across a resize that drew nothing"
            ),
            // A fit that answered for one target may refuse for another — the
            // rungs can hide too much of a larger frame — and the reverse. Both
            // are the re-read working; `Basis::Measured` is what the slot falls
            // back to, which is ADR-0296 §2 doing its job.
            (was, now) => assert_ne!(
                format!("{was:?}"),
                format!("{now:?}"),
                "the fit is unchanged across a resize, so nothing was re-read"
            ),
        }
    }
}
