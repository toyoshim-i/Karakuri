use super::common::*;

mod gpu {
    use super::*;

    /// A frame slowed by hand, so that **"the deck is over its period" is a fact
    /// of the harness rather than of the machine**.
    ///
    /// `docs/contributing.md` §1 rules out a test that turns on whether this
    /// adapter is fast: a workload heavy enough to blow a budget here is
    /// comfortable somewhere else. The two tests below need a deck whose *frames*
    /// are certainly over a budget while the *candidate* is certainly under it,
    /// and the only way to have both on every machine is to make the frame long
    /// by construction. It stands in for exactly what the old gate could not tell
    /// apart from a heavy candidate: a long frame, whatever produced it.
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

    /// **A light candidate is kept in a deck whose frames are over the budget —
    /// which is the exact case that used to roll it back.**
    ///
    /// Measured on 2026-09-09, headless at 1280x720 on an M4 Pro, host clock and
    /// biased high: the panel's four default slots take 4.3 ms a frame, the
    /// reference Set in one of four takes 11.3 ms, and on the panel each cell's
    /// present adds about 1.5 ms on top — so loading the reference Set into one
    /// slot of four was about 18 ms of work against a 16.7 ms vsync, landed at 33
    /// ms under Fifo, and was rolled back. What that Set's *own* frame costs is
    /// about 9 ms. The gate was not too strict; it was reading the wrong
    /// quantity
    /// ([ADR-0313](../../../docs/adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)).
    ///
    /// The deck here is four slots with a build landing on one of them, its frames
    /// held over the budget by [`SLOW_FRAME`], and the candidate small. Under the
    /// old gate the median interval decided, so this candidate was certain to go;
    /// under this one it is kept, and the deck being over its period is said in
    /// the place that is entitled to say it.
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
        // **A number and not the absence of one.** Which of the slot's two
        // readings answered is `governor::budgeted`'s and not this test's: a
        // candidate arrives with a measurement and an `estimate` at the deck's
        // own size since ADR-0356, and on this machine the fit can refuse. What
        // this test turns on is that the verdict was reached on the candidate's
        // own frame rather than on the deck's interval, which the band below is
        // what says.
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

        // **And the deck says it is over its period, in the one place entitled
        // to.** The window is a rolling median, so it needs filling before there
        // is a number at all — `None` is not `false` (`P-0095`).
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
        // **And it is not the same flag as `over_budget`**, which is the Live
        // slots' summed per-Set cost against the *compute* budget and is a
        // different quantity. M5.14 item 3 is where the deck's total belongs;
        // this measurement is what that item needs and is not an answer to it.
        assert!(
            report.to_string().contains("OVER its period"),
            "the report's line does not say the deck is over its period: {report}"
        );
    }

    /// **A slot the watchdog stopped takes no step, and its target keeps the
    /// last image it made** — two of the three things ADR-0316 says *stops
    /// updating* means.
    ///
    /// The budget is zero, so the candidate cannot pass and the branch is
    /// certainly reached on every machine (`docs/contributing.md` §1: the
    /// alternative is a shader chosen for being slow somewhere).
    ///
    /// **What each half rules out.** The step count would advance if
    /// `Frame::render` were skipping the draw and not the step, and the target's
    /// bits would go *black* if a stopped slot were skipped by clearing rather
    /// than by being left alone — which is the state an operator cannot tell
    /// from an empty slot.
    ///
    /// **What no assertion here can reach is the draw**, and saying so is
    /// better than implying otherwise. A stopped slot's buffers do not change,
    /// and `points.rs` clears its target and redraws from those buffers — so a
    /// slot that was still being drawn would produce the **same bits**, and this
    /// test would pass. *Keep drawing without stepping* is therefore ruled out
    /// by ADR-0316's argument (it removes almost nothing, because the draw is
    /// the fill-rate half) and not by this file; what an outside observer can
    /// see of it is a cost, and this suite asserts no costs.
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

        // **And the freeze is the version's**: a build that fits clears it and
        // the slot runs again. The budget is what moves, because moving it is
        // what makes the same machinery answer differently.
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

    /// **A fader to zero reaches a stopped slot**, which is
    /// [ADR-0040](../../../docs/adr/0040-a-gain-of-zero-means-no-contribution-so-the-slot-is-skipped.md)'s
    /// zero-skip and `P-0094`'s last resort: the way out of material an operator
    /// cannot use is to pull it down, and it has to work on exactly the material
    /// that has gone wrong. A stopped slot is still mixed — that is the point of
    /// stopping it rather than blanking it — so *still mixed* has to be
    /// something the fader can end.
    ///
    /// One slot Live and the other off air, so the picture is this slot's held
    /// image and nothing else, and *out of the mix* is *black* rather than a
    /// difference somebody has to interpret.
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
        // **And the slot is still stopped**, because a fader is not a build. The
        // operator took it out of the mix; nothing decided that ended the freeze
        // for them.
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

    /// Verifies that candidate budget evaluation is independent of neighbor slot loads (ADR-0313).
    ///
    /// The same candidate receives the identical verdict regardless of whether
    /// neighboring slots are idle or carrying heavy workloads that exceed the
    /// deck-level frame period.
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
        // **Both verdicts read a number of the candidate's own**, and which of
        // its two readings that was is not asserted. A candidate arrives with a
        // measurement and an `estimate` since ADR-0356, and the estimate's two
        // rungs are a few hundred microseconds each on a host clock — so on the
        // loaded deck the slope through them can come out negative, which is
        // `Unfit::FragmentTermNegative` and sends the slot to its measurement
        // (ADR-0296 §2). That is the fallback working and it is load-dependent
        // by construction. What is *not* allowed to move with the neighbours is
        // the verdict, which is the equality above.
        for (which, basis) in [("idle", idle.2), ("loaded", loaded.2)] {
            assert_ne!(
                basis,
                karakuri_engine::Basis::Unbudgetable,
                "the {which} deck's verdict was reached on nothing"
            );
        }

        // **Not vacuous.** The two decks really were in different states, and the
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
