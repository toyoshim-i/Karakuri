use super::common::*;

mod gpu {
    use super::*;

    // ADR-0258: verifies that each slot renders into its dedicated target before composite mix.

    /// Verifies that a slot's dedicated target reflects its rendered texels before channel faders are applied.
    #[test]
    fn a_running_slot_shows_its_own_texels_with_no_fader_on_them() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |gain: f32, opacity: f32, mask: Mask| -> (Vec<u16>, Vec<u16>) {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_gain(karakuri_engine::DeckSlot(1), gain);
            deck.set_opacity(karakuri_engine::DeckSlot(1), opacity);
            deck.set_mask(karakuri_engine::DeckSlot(1), mask);
            for _ in 0..8 {
                frame(&gpu, &mut deck, &present, 1);
            }
            (
                readback(&gpu, deck.slot_target(karakuri_engine::DeckSlot(1))),
                readback(&gpu, present.hdr_texture()),
            )
        };

        let (open, open_mix) = run(1.0, 1.0, Mask::default());
        assert!(
            lit(&open) > 100,
            "the slot drew nothing, so this test is asserting nothing"
        );

        for (gain, opacity, mask, what) in [
            (0.0, 1.0, Mask::default(), "gain at silence"),
            (1.0, 0.0, Mask::default(), "opacity at silence"),
            (0.25, 0.5, Mask::default(), "both faders part way down"),
            (
                1.0,
                1.0,
                Mask::new(MaskKind::Linear, 0.0, 0.5, 0.0),
                "a linear mask half across",
            ),
        ] {
            let (own, mix) = run(gain, opacity, mask);
            assert_eq!(
                own, open,
                "{what} changed what the slot drew into its own target — a fader is an \
             edge property and belongs in the composite, and a cell sampling this \
             texture would be showing the operator the level they already set"
            );
            assert_ne!(
                mix, open_mix,
                "{what} left the mix unchanged, so the comparison above is between two \
             settings neither of which does anything"
            );
        }
    }

    /// Verifies that off-air slots (Allocated and Priming) are rendered and stepped every frame (ADR-0258, ADR-0269).
    #[test]
    fn an_off_air_slot_is_drawn_every_frame_and_steps_every_frame() {
        let gpu = Gpu::headless().expect("no GPU available");

        for residency in [Residency::Allocated, Residency::Priming] {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);

            // Live first, so the slot has element state to draw. A Set that has
            // never stepped draws black, which is the case the next test is
            // about and would make this one unable to tell a draw from no draw.
            for _ in 0..8 {
                frame(&gpu, &mut deck, &present, 1);
            }
            deck.set_residency(karakuri_engine::DeckSlot(1), residency);
            for _ in 0..4 {
                frame(&gpu, &mut deck, &present, 1);
            }
            let off_air_at = steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set());

            // Resize reallocates targets without stale buffer carryover.
            // left over from when the slot was Live.
            deck.resize(&gpu.device, WIDTH / 2, HEIGHT / 2);
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH / 2, HEIGHT / 2);
            assert_eq!(
                lit(&readback(
                    &gpu,
                    deck.slot_target(karakuri_engine::DeckSlot(1))
                )),
                0,
                "the reallocated target came back with something in it, so the assertion \
             below cannot tell a fresh draw from a stale one"
            );

            frame(&gpu, &mut deck, &present, 1);
            assert!(
                lit(&readback(
                    &gpu,
                    deck.slot_target(karakuri_engine::DeckSlot(1))
                )) > 100,
                "a slot at {residency:?} drew nothing into its own target on the frame \
             after a resize, so its console cell is dark at exactly the moment an \
             operator is deciding whether to bring the slot up (ADR-0258)"
            );

            // And it advanced by the frame's steps and by no more: the draw is
            // not a second step on top of the residency's (P-0082).
            assert_eq!(
                steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
                off_air_at + 1,
                "a slot at {residency:?} did not take exactly the one step the frame \
             gave it — either it is standing still, which makes its cell a still \
             (ADR-0269), or the draw is stepping it as well (P-0082)"
            );

            // The mix is still only the Live slot's, which is the other half:
            // drawn is not mixed.
            let mixed = readback(&gpu, present.hdr_texture());
            let solo_present =
                Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH / 2, HEIGHT / 2);
            let mut solo = deck_of_at(&gpu, &[SEED_A], WIDTH / 2, HEIGHT / 2);
            for _ in 0..13 {
                frame(&gpu, &mut solo, &solo_present, 1);
            }
            assert_eq!(
                mixed,
                readback(&gpu, solo_present.hdr_texture()),
                "a slot at {residency:?} reached the mix — being drawn is not being mixed"
            );
        }
    }

    /// Verifies that a rejected build preserves the actively running set in the slot target.
    /// Resizing reallocates the target to verify subsequent frames render the surviving set.
    #[test]
    fn a_rejected_build_shows_what_is_still_running() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        // Slot 1 takes builds and is off air; slot 0 is the Live neighbour that
        // proves a frame was recorded at all.
        let _ = present;
        let (tx, rx) = mpsc::channel();
        let mut deck = Deck::new(
            &gpu.device,
            vec![
                HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                HotSwap::new(
                    &gpu.device,
                    &gpu.queue,
                    build(&gpu, SEED_B, CAPACITY),
                    GENEROUS_MS,
                    Box::new(rx),
                ),
            ],
            WIDTH,
            HEIGHT,
        );

        // --- the rejected build --------------------------------------------
        // Warm the slot first, so there is a picture for a rejection to leave
        // alone. Off air is enough: an off-air slot steps (ADR-0269).
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Allocated);
        deck.resize(&gpu.device, WIDTH / 2, HEIGHT / 2);
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH / 2, HEIGHT / 2);
        for _ in 0..8 {
            frame(&gpu, &mut deck, &present, 1);
        }
        let warmed = steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set());
        assert!(
            warmed > 0,
            "the slot did not warm, so there is nothing to keep"
        );
        frame(&gpu, &mut deck, &present, 1);
        let before = readback(&gpu, deck.slot_target(karakuri_engine::DeckSlot(1)));
        assert!(lit(&before) > 100, "the warmed slot drew nothing");
        let at_rejection_start = steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set());

        // A capacity outside the L1's declared range: the worker builds it and
        // `Set::build` refuses, which is `Event::Rejected` and not a swap.
        const REFUSED: u32 = 1;
        tx.send(Request {
            names: karakuri_engine::swap::RequestNames::default(),
            edges: Vec::new(),
            id: 1,
            l1s: vec![(compile(L1), REFUSED)],
            l2s: Vec::new(),
            l3s: Vec::new(),
            fields: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            published: Vec::new(),
            l4s: vec![compile(L4)],
            seed_salt: SEED_B,
            camera: karakuri_engine::camera::Orbit::default(),
            salts: Vec::new(),
            params: Vec::new(),
            bindings: Vec::new(),
            authorities: Vec::new(),
            label: "slot 1, refused".to_string(),
        })
        .expect("worker alive");

        let started = Instant::now();
        let mut rejected = false;
        // Counted, because the slot goes on running while the worker refuses the
        // build: what a rejection must not do is move the clock by anything
        // other than the frames the room took.
        let mut waited = 0u64;
        while !rejected {
            frame(&gpu, &mut deck, &present, 1);
            waited += 1;
            for event in deck.events(karakuri_engine::DeckSlot(1)) {
                match event {
                    Event::Rejected { .. } => rejected = true,
                    other => panic!("the refused build did not come back as a rejection: {other}"),
                }
            }
            assert!(
                started.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for the rejection and it never arrived"
            );
        }
        assert!(warmed > 0, "the slot never warmed");

        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            at_rejection_start + waited,
            "the rejected build moved the running Set's clock by something other than \
         the frames that went past while it was being refused"
        );
        // The target is thrown away, so what comes back can only be this
        // frame's draw of the Set that survived the rejection.
        deck.resize(&gpu.device, WIDTH, HEIGHT);
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        frame(&gpu, &mut deck, &present, 1);
        assert!(
            lit(&readback(
                &gpu,
                deck.slot_target(karakuri_engine::DeckSlot(1))
            )) > 100,
            "a slot whose build was refused went dark, so the cell says the material is \
         gone when nothing changed at all — the running Set is still running"
        );
    }

    /// Verifies deterministic bit-identical compositing across runs with identical seed sequences and uneven ticks.
    #[test]
    fn the_same_ticks_and_seeds_composite_bit_identically() {
        let gpu = Gpu::headless().expect("no GPU available");
        const TICKS: [u8; 12] = [1, 2, 1, 3, 1, 1, 4, 2, 1, 3, 2, 1];

        let run = || -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B, SEED_A + 1]);
            deck.set_gain(karakuri_engine::DeckSlot(0), 1.5);
            deck.set_gain(karakuri_engine::DeckSlot(1), 0.75);
            deck.set_opacity(karakuri_engine::DeckSlot(2), 0.5);
            for steps in TICKS {
                frame(&gpu, &mut deck, &present, steps);
            }
            readback(&gpu, present.hdr_texture())
        };

        let first = run();
        let second = run();
        assert!(lit(&first) > 100, "the deck drew nothing to compare");
        assert_eq!(
            first, second,
            "two runs of the same ticks and the same seeds composited differently"
        );
    }

    /// Verifies that hot-swapping one slot does not perturb execution or buffers of neighbouring slots.
    #[test]
    fn a_swap_in_one_slot_leaves_the_other_slot_alone() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let (tx, rx) = mpsc::channel();
        let swapping = HotSwap::new(
            &gpu.device,
            &gpu.queue,
            build(&gpu, SEED_A, CAPACITY),
            GENEROUS_MS,
            Box::new(rx),
        );
        let steady = HotSwap::fixed(build(&gpu, SEED_B, CAPACITY));
        let mut deck = Deck::new(&gpu.device, vec![swapping, steady], WIDTH, HEIGHT);

        let mut frames = 0u64;
        for _ in 0..10 {
            frame(&gpu, &mut deck, &present, 1);
            frames += 1;
        }
        let neighbour_live_before = deck
            .slot(karakuri_engine::DeckSlot(1))
            .set()
            .live_count(&gpu.device, &gpu.queue);

        tx.send(Request {
            names: karakuri_engine::swap::RequestNames::default(),
            edges: Vec::new(),
            id: 1,
            l1s: vec![(compile(L1), SWAPPED)],
            l2s: Vec::new(),
            l3s: Vec::new(),
            fields: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            published: Vec::new(),
            l4s: vec![compile(L4)],
            seed_salt: SEED_A,
            camera: karakuri_engine::camera::Orbit::default(),
            salts: Vec::new(),
            params: Vec::new(),
            bindings: Vec::new(),
            authorities: Vec::new(),
            label: "slot 0, second".to_string(),
        })
        .expect("worker alive");

        let started = Instant::now();
        let mut swapped = false;
        while !swapped {
            frame(&gpu, &mut deck, &present, 1);
            frames += 1;
            swapped = deck
                .events(karakuri_engine::DeckSlot(0))
                .any(|e| matches!(e, Event::Swapped { .. }));
            assert!(
                started.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for the swap and it never landed"
            );
        }
        // Frames kept coming while the build was in flight, which is what "the
        // worker does not block the render loop" looks like from outside — and it
        // has to keep being true with N slots, since `Deck::begin_frame` polls
        // every one of them.
        assert!(
            frames > 11,
            "only {frames} frames were produced; the deck waited for the build"
        );

        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(0)).set().capacity(),
            SWAPPED,
            "the swap reported success but slot 0 is still the old Set"
        );
        // Cold, as every V1 swap is: a new procedure means new buffers. Warming
        // one out of sight is Priming, and it is not in this slice.
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(0)).set()),
            1,
            "the swapped-in Set inherited a `t`"
        );

        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(1)).set().capacity(),
            CAPACITY,
            "the swap reached the neighbouring slot"
        );
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            frames,
            "the neighbouring slot's clock did not advance normally across the swap"
        );
        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(1))
                .set()
                .live_count(&gpu.device, &gpu.queue),
            neighbour_live_before,
            "the swap disturbed the neighbouring slot's element buffers"
        );
        assert_eq!(
            deck.live_slots(),
            2,
            "the swap changed which slots are live"
        );
        assert!(
            deck.events(karakuri_engine::DeckSlot(1)).next().is_none(),
            "the untouched slot reported an event"
        );
    }

    /// Verifies that candidate sets on off-air slots are evaluated at install time against their own cost (ADR-0313).
    ///
    /// Off-air slots still receive frame boundaries during `Deck::begin_frame`.
    /// With a zero budget, the candidate must be rejected while the slot is parked.
    #[test]
    fn an_off_air_slots_candidate_is_judged_at_the_install() {
        // Upper bound to verify candidate verdict arrives promptly.
        const A_FULL_WINDOW: usize = 8 + 30 + 12;

        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let (tx, rx) = mpsc::channel();
        let parked = HotSwap::new(
            &gpu.device,
            &gpu.queue,
            build(&gpu, SEED_B, CAPACITY),
            0.0,
            Box::new(rx),
        );
        let mut deck = Deck::new(
            &gpu.device,
            vec![HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)), parked],
            WIDTH,
            HEIGHT,
        );
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Allocated);

        tx.send(Request {
            names: karakuri_engine::swap::RequestNames::default(),
            edges: Vec::new(),
            id: 1,
            l1s: vec![(compile(L1), SWAPPED)],
            l2s: Vec::new(),
            l3s: Vec::new(),
            fields: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            published: Vec::new(),
            l4s: vec![compile(L4)],
            seed_salt: SEED_B,
            camera: karakuri_engine::camera::Orbit::default(),
            salts: Vec::new(),
            params: Vec::new(),
            bindings: Vec::new(),
            authorities: Vec::new(),
            label: "off air".to_string(),
        })
        .expect("worker alive");

        let verdict = |deck: &mut Deck| -> Option<String> {
            deck.events(karakuri_engine::DeckSlot(1))
                .find_map(|e| match e {
                    Event::Accepted { label, cost_ms, .. } => Some(format!(
                        "Accepted `{label}` at {:.3} ms",
                        cost_ms.unwrap_or(f32::NAN)
                    )),
                    Event::Overloaded { label, cost_ms, .. } => {
                        Some(format!("Overloaded `{label}` at {cost_ms:.3} ms"))
                    }
                    _ => None,
                })
        };

        // The build lands on the parked slot and is judged in the same frame, so
        // both are read out of one loop: the events are drained once, and
        // `verdict` would consume a verdict that a second drain then waited for.
        let started = Instant::now();
        let mut seen = None;
        let mut frames = 0usize;
        // The parked slot's step count while it was still running its own Set —
        // read at the top of each frame, so the last reading is the one taken
        // before the build landed on it.
        let mut stepped_off_air = 0u64;
        while seen.is_none() {
            stepped_off_air =
                stepped_off_air.max(steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()));
            frame(&gpu, &mut deck, &present, 1);
            frames += 1;
            seen = verdict(&mut deck);
            assert!(
                started.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for the off-air build and no verdict came"
            );
        }
        let seen = seen.expect("just set");

        assert!(
            seen.starts_with("Overloaded"),
            "a candidate that cannot fit a zero budget was judged in its favour: {seen}"
        );
        // Candidate set occupies slot regardless of on-air status (ADR-0316).
        // here would be the Set the build displaced, put back.
        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(1)).set().capacity(),
            SWAPPED,
            "the verdict took the build out of a parked slot instead of stopping it"
        );
        assert!(
            deck.overloaded(karakuri_engine::DeckSlot(1)),
            "a parked slot over the budget was not marked stopped"
        );
        // The overload verdict arrives on the frame the build lands while still Allocated.
        assert!(
            frames < A_FULL_WINDOW,
            "the verdict took {frames} frames, which is a window: it is waiting again"
        );
        assert_eq!(
            deck.residency(karakuri_engine::DeckSlot(1)),
            Residency::Allocated,
            "the slot went on air by itself, so this says nothing about a parked one"
        );
        assert!(
            stepped_off_air > 0,
            "the off-air slot did not step before the build landed, so this test is no \
         longer about a slot that is paying into the interval it is not being judged by"
        );
        // Freeze suppresses simulation steps on off-air slots.
        // off-air branch of `Frame::render`: a stopped slot takes no step
        // whatever its residency, so the Set that just landed is still at zero
        // several frames later.
        for _ in 0..5 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            0,
            "a stopped slot went on stepping off air"
        );
    }
}
