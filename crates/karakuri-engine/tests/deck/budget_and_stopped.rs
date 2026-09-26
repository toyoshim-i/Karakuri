use super::common::*;

mod gpu {
    use super::*;

    /// Deliberate artificial frame delay to ensure deck period reliably exceeds budget in tests.
    const SLOW_FRAME: Duration = Duration::from_millis(30);

    /// The budget both of those hold against — a third of [`SLOW_FRAME`], so the
    /// deck's period is over it whatever the machine, and wide enough that a
    /// [`SWAPPED`]-element Set at [`WIDTH`]x[`HEIGHT`] is under it by an order of
    /// magnitude on anything that can run this suite at all.
    const OVER_A_SLOW_FRAME_MS: f32 = 10.0;

    /// How many frames the deck's period is a median over, plus slack — read
    /// across from `swap.rs` rather than transcribed, so that moving it there
    /// moves it here.
    const PERIOD_WINDOW: usize = karakuri_engine::swap::PERIOD_FRAMES + 4;

    /// One frame, and then a wait long enough that the deck's period is over
    /// [`OVER_A_SLOW_FRAME_MS`] on any machine.
    fn slow_frame(gpu: &Gpu, deck: &mut Deck, present: &Present) {
        frame(gpu, deck, present, 1);
        std::thread::sleep(SLOW_FRAME);
    }

    /// Verifies that candidate sets are retained when candidate cost is within budget,
    /// even if aggregate deck period exceeds the frame period limit (ADR-0313).
    #[test]
    fn a_light_candidate_is_kept_in_a_deck_whose_frames_are_over_the_budget() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let (tx, rx) = mpsc::channel();
        let watched = HotSwap::new(
            &gpu.device,
            &gpu.queue,
            build(&gpu, SEED_B, CAPACITY),
            OVER_A_SLOW_FRAME_MS,
            Box::new(rx),
        );
        let mut deck = Deck::new(
            &gpu.device,
            vec![
                HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                watched,
                HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                HotSwap::fixed(build(&gpu, SEED_B, CAPACITY)),
            ],
            WIDTH,
            HEIGHT,
        );
        deck.set_frame_budget_ms(OVER_A_SLOW_FRAME_MS);
        deck.set_residency(karakuri_engine::DeckSlot(0), Residency::Live);

        tx.send(candidate(1)).expect("worker alive");

        let started = Instant::now();
        let mut verdict = None;
        while verdict.is_none() {
            slow_frame(&gpu, &mut deck, &present);
            verdict = deck
                .events(karakuri_engine::DeckSlot(1))
                .find_map(said_verdict);
            assert!(
                started.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for a verdict on the slow deck and none came"
            );
        }
        let (kept, cost_ms, basis) = verdict.expect("just set");
        assert!(
            kept,
            "a candidate costing {cost_ms:?} ms stopped its slot in a deck whose frames \
         are {:?} long — the verdict is still reading the deck's interval",
            SLOW_FRAME
        );
        assert!(
            !deck.overloaded(karakuri_engine::DeckSlot(1)),
            "the verdict was in the candidate's favour and the slot is marked stopped"
        );
        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(1)).set().capacity(),
            SWAPPED,
            "the verdict said kept and the slot is not holding the candidate"
        );
        // Verifies the verdict was reached on the candidate's own frame cost rather than the deck interval.
        assert_ne!(
            basis,
            karakuri_engine::Basis::Unbudgetable,
            "the verdict was reached on nothing"
        );
        let cost_ms = cost_ms.expect("the worker measured and estimated this Set");
        assert!(
            cost_ms < OVER_A_SLOW_FRAME_MS,
            "this machine reads {cost_ms} ms for one frame of a {SWAPPED}-element Set at \
         {WIDTH}x{HEIGHT}, which is over the {OVER_A_SLOW_FRAME_MS} ms this test holds \
         it against — the test's own premise is gone, not the gate"
        );

        // Rolling median window must fill before period overrun is reported (P-0095).
        for _ in 0..PERIOD_WINDOW {
            slow_frame(&gpu, &mut deck, &present);
        }
        let period = deck
            .frame_period_ms()
            .expect("thirty frames is a full window");
        assert!(
            period > OVER_A_SLOW_FRAME_MS,
            "the deck's frames are {:?} long and it measured {period} ms",
            SLOW_FRAME
        );
        let report = deck.govern();
        assert_eq!(
            report.deck_over_period(),
            Some(true),
            "the deck is over its period and the report does not say so"
        );
        assert_eq!(report.frame_period_ms, Some(period));
        assert_eq!(report.frame_budget_ms, Some(OVER_A_SLOW_FRAME_MS));
        // Period overrun is independent from compute budget `over_budget` flag.
        assert!(
            report.to_string().contains("OVER its period"),
            "the report's line does not say the deck is over its period: {report}"
        );
    }

    /// Verifies that a stopped slot ceases stepping its state while preserving its last rendered image target (ADR-0316).
    #[test]
    fn a_stopped_slot_takes_no_step_and_keeps_the_image_it_stopped_at() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let (tx, rx) = mpsc::channel();
        let watched = HotSwap::new(
            &gpu.device,
            &gpu.queue,
            build(&gpu, SEED_B, CAPACITY),
            // Nothing is faster than zero milliseconds.
            0.0,
            Box::new(rx),
        );
        let mut deck = Deck::new(
            &gpu.device,
            vec![HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)), watched],
            WIDTH,
            HEIGHT,
        );
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Live);

        // Frames before the request, so the slot's own Set has drawn something
        // real into its target and the image this test says is *kept* is not an
        // empty one.
        for _ in 0..4 {
            frame(&gpu, &mut deck, &present, 1);
        }
        tx.send(candidate(1)).expect("worker alive");

        let started = Instant::now();
        let mut verdict = None;
        while verdict.is_none() {
            frame(&gpu, &mut deck, &present, 1);
            verdict = deck
                .events(karakuri_engine::DeckSlot(1))
                .find_map(said_verdict);
            assert!(started.elapsed() < PATIENCE, "no verdict on the candidate");
        }
        let (kept, _, _) = verdict.expect("just set");
        assert!(!kept, "a candidate held a budget of zero milliseconds");
        assert!(
            deck.overloaded(karakuri_engine::DeckSlot(1)),
            "the verdict was against and the slot is not marked stopped"
        );
        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(1)).set().capacity(),
            SWAPPED,
            "the verdict took the candidate out of the slot"
        );

        let steps = steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set());
        let image = readback(&gpu, deck.slot_target(karakuri_engine::DeckSlot(1)));
        assert!(
            image.iter().any(|&bits| bits != 0),
            "the slot's target is empty before this test starts, so *kept* is not \
         a claim about anything"
        );

        for _ in 0..8 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            steps,
            "a stopped slot is still being stepped"
        );
        assert_eq!(
            readback(&gpu, deck.slot_target(karakuri_engine::DeckSlot(1))),
            image,
            "a stopped slot's target changed, so something is still drawing into it"
        );

        // Slot freeze state clears when a replacement build fits the budget.
        deck.set_frame_budget_ms(GENEROUS_MS);
        tx.send(candidate(2)).expect("worker alive");
        let started = Instant::now();
        let mut kept = None;
        while kept.is_none() {
            frame(&gpu, &mut deck, &present, 1);
            kept = deck
                .events(karakuri_engine::DeckSlot(1))
                .find_map(said_verdict)
                .map(|v| v.0);
            assert!(
                started.elapsed() < PATIENCE,
                "no verdict on the second build"
            );
        }
        assert_eq!(
            kept,
            Some(true),
            "a generous budget stopped the slot anyway"
        );
        assert!(
            !deck.overloaded(karakuri_engine::DeckSlot(1)),
            "a build that held the budget left the slot marked stopped"
        );
        for _ in 0..4 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()) > 0,
            "the slot is unmarked and still not stepping"
        );
    }

    /// Verifies that setting gain to zero excludes a stopped slot's held image from the composite (ADR-0040).
    #[test]
    fn a_fader_to_zero_takes_a_stopped_slot_out_of_the_picture() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let (tx, rx) = mpsc::channel();
        let watched = HotSwap::new(
            &gpu.device,
            &gpu.queue,
            build(&gpu, SEED_B, CAPACITY),
            0.0,
            Box::new(rx),
        );
        let mut deck = Deck::new(
            &gpu.device,
            vec![HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)), watched],
            WIDTH,
            HEIGHT,
        );
        // The other slot stays off air, so what reaches the picture is this one
        // or nothing.
        deck.set_residency(karakuri_engine::DeckSlot(0), Residency::Allocated);
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Live);

        for _ in 0..4 {
            frame(&gpu, &mut deck, &present, 1);
        }
        tx.send(candidate(1)).expect("worker alive");
        let started = Instant::now();
        let mut verdict = None;
        while verdict.is_none() {
            frame(&gpu, &mut deck, &present, 1);
            verdict = deck
                .events(karakuri_engine::DeckSlot(1))
                .find_map(said_verdict);
            assert!(started.elapsed() < PATIENCE, "no verdict on the candidate");
        }
        assert!(
            deck.overloaded(karakuri_engine::DeckSlot(1)),
            "the slot is not stopped, so this test is about an ordinary fader"
        );

        frame(&gpu, &mut deck, &present, 1);
        let showing = readback(&gpu, present.hdr_texture());
        assert!(
            showing.iter().any(|&bits| bits != 0),
            "a stopped slot is contributing nothing to the picture already, so \
         pulling it down proves nothing"
        );

        deck.set_gain(karakuri_engine::DeckSlot(1), 0.0);
        frame(&gpu, &mut deck, &present, 1);
        let faded = readback(&gpu, present.hdr_texture());
        assert!(
            faded.iter().all(|&bits| bits == 0),
            "a fader at zero on a stopped slot left it in the picture"
        );
        // Fader adjustments do not clear slot budget freeze state.
        assert!(
            deck.overloaded(karakuri_engine::DeckSlot(1)),
            "the fader cleared the freeze, which is a state changing by itself"
        );

        // Toggling residency off-air and back preserves the overload stopped state (ADR-0316).
        let steps = steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set());
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Allocated);
        for _ in 0..3 {
            frame(&gpu, &mut deck, &present, 1);
        }
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Live);
        for _ in 0..3 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert!(
            deck.overloaded(karakuri_engine::DeckSlot(1)),
            "a stopped slot taken off air and put back came back running"
        );
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            steps,
            "a stopped slot stepped while it was off air"
        );
    }

    /// Verifies candidate budget evaluation is independent of neighbor slot loads (ADR-0313).
    #[test]
    fn a_candidates_verdict_does_not_move_with_what_the_other_slots_carry() {
        /// Sixteen times [`CAPACITY`], so the loaded run's neighbours are a real
        /// difference and not only a slower harness.
        const LOADED: u32 = CAPACITY * 16;

        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let run = |neighbours: u32, live: usize, slow: bool| {
            let (tx, rx) = mpsc::channel();
            let watched = HotSwap::new(
                &gpu.device,
                &gpu.queue,
                build(&gpu, SEED_B, CAPACITY),
                OVER_A_SLOW_FRAME_MS,
                Box::new(rx),
            );
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, neighbours)),
                    watched,
                    HotSwap::fixed(build(&gpu, SEED_A, neighbours)),
                    HotSwap::fixed(build(&gpu, SEED_B, neighbours)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_frame_budget_ms(OVER_A_SLOW_FRAME_MS);
            for slot in [0, 2, 3].iter().take(live) {
                deck.set_residency(karakuri_engine::DeckSlot(*slot), Residency::Live);
            }
            tx.send(candidate(1)).expect("worker alive");

            let started = Instant::now();
            let mut verdict = None;
            while verdict.is_none() {
                if slow {
                    slow_frame(&gpu, &mut deck, &present);
                } else {
                    frame(&gpu, &mut deck, &present, 1);
                }
                verdict = deck
                    .events(karakuri_engine::DeckSlot(1))
                    .find_map(said_verdict);
                assert!(started.elapsed() < PATIENCE, "no verdict on this deck");
            }
            for _ in 0..PERIOD_WINDOW {
                if slow {
                    slow_frame(&gpu, &mut deck, &present);
                } else {
                    frame(&gpu, &mut deck, &present, 1);
                }
            }
            let over = deck.govern().deck_over_period();
            (verdict.expect("just set"), over)
        };

        let (idle, idle_over) = run(CAPACITY, 1, false);
        let (loaded, loaded_over) = run(LOADED, 3, true);

        assert!(
            idle.0,
            "the candidate stopped its slot on an idle deck, so this test is about \
         something other than the neighbours"
        );
        assert_eq!(
            idle.0, loaded.0,
            "the same candidate was kept on one deck and thrown out on another; the \
         only difference between them is what the other slots are carrying"
        );
        // Verifies that both verdicts evaluated the candidate's own cost rather than unbudgetable fallback.
        for (which, basis) in [("idle", idle.2), ("loaded", loaded.2)] {
            assert_ne!(
                basis,
                karakuri_engine::Basis::Unbudgetable,
                "the {which} deck's verdict was reached on nothing"
            );
        }

        // Verifies distinct alarm reporting between differing deck states.
        // one thing that is allowed to notice is the deck-level alarm.
        assert_eq!(
            idle_over,
            Some(false),
            "the idle deck is already over its period, so the two runs are not \
         distinguishable and the assertion above proves nothing"
        );
        assert_eq!(
            loaded_over,
            Some(true),
            "the loaded deck is not over its period, so the two runs are not \
         distinguishable and the assertion above proves nothing"
        );
    }
}
