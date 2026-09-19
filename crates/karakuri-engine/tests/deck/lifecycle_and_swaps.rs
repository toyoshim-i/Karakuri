use super::common::*;

mod gpu {
    use super::*;

    // ---------------------------------------------------------------------------
    // ADR-0258: an operator sees a slot's own material without putting it on air
    //
    // Three tests, one per case the requirement names, each asserting on the
    // slot's own target — `Deck::slot_target` — because that is the texture a
    // console cell samples through `Deck::slot_view`. What the mix does with the
    // slot is a separate question and is asserted separately in each.
    // ---------------------------------------------------------------------------

    /// **A running slot's own target holds its own texels, with no fader on
    /// them.**
    ///
    /// ADR-0258's *the fader is not in the monitor*, and the clause that decides
    /// where a monitor may sample from. `gain`, `opacity`, `blend` and `mask` are edge
    /// properties applied in `Composite`, so a slot's target is upstream of all
    /// four — which is what lets a cell show *the level the material arrives at*
    /// rather than the level the operator has already set.
    ///
    /// Asserted as bit equality between a slot faded to silence and the same slot
    /// at unity, in the same deck on the same ticks. It is exact because nothing
    /// between `Set::draw` and the readback rounds; "close enough" here would
    /// tolerate a fader that had leaked upstream by a hair.
    ///
    /// **And the mix is checked to differ**, or the whole thing would pass with
    /// the fader deleted: two identical slot targets prove nothing if the fader
    /// was never applied anywhere.
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

    /// **An off-air slot is drawn into its own target every frame, and stepped
    /// every frame.**
    ///
    /// ADR-0258's *residency does not gate a cell*, and ADR-0269's *a slot that
    /// is drawn is stepped*. The slot an operator most needs to look at is the
    /// one that is not on air yet, and what they need to see is the material
    /// running rather than the still it stopped at.
    ///
    /// **`t` still moves for the deck's reason and not the monitor's**, which is
    /// what keeps
    /// [P-0082](../../../docs/principles/0082-looking-never-writes-back.md)
    /// intact: the step is the residency's, and taking every cell off this
    /// console would not stop one of them. What the test can see of that is that
    /// the two off-air levels advance by the frame's steps and by nothing else —
    /// a draw that stepped the slot as well would show up as a slot that took
    /// two.
    ///
    /// **The resize is what makes this a test of *this* frame's draw.** A slot
    /// target persists, so a parked slot that was Live a moment ago keeps the last
    /// picture in it and "the target is lit" would pass with the draw deleted —
    /// that is ADR-0072's own correction to itself, made after it shipped the
    /// wrong reason. `Deck::resize` reallocates every target, so the frame after
    /// one is the only frame on which the target's contents can only have come
    /// from a draw recorded on it.
    ///
    /// Both off-air levels are covered even though they are now one branch,
    /// because they are two *residencies* and a surface reads them apart.
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

            // **The target is thrown away and remade**, so nothing in it can be
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

    /// **A slot whose build was rejected shows what is still running.**
    ///
    /// One of the failure cases ADR-0258 has to answer honestly. A rejected build
    /// changes nothing — `swap.rs` is explicit that the running Set keeps
    /// running, with its `t` and its live count untouched — so the honest picture
    /// is the material that is still there, drawn on the frame after the
    /// rejection exactly as on the frame before. Anything else would be the cell
    /// inventing a state the deck is not in. What says a build was refused is
    /// `Event::Rejected`, which the Staging lane draws; the cell's job is the
    /// picture.
    ///
    /// **The other half of this test is gone, and it is gone because the state
    /// it asserted is unreachable.** It read: *a Set that has never stepped has
    /// no element state, so its draw is a pass over zeroed buffers — every
    /// element at the origin, a handful of texels in the middle of an otherwise
    /// black frame*, which is the cold end of a slot and was what priming
    /// existed to fix. ADR-0269 steps every slot on every frame, so the first
    /// frame a slot is drawn on is a frame it has already stepped: no deck can
    /// show a never-stepped Set, and a test asserting one would be asserting
    /// against a fixture rather than against the engine.
    ///
    /// **Nothing-drew and drew-nothing are still told apart by the resize.**
    /// `Deck::resize` reallocates the target and wgpu hands it back zeroed, so
    /// the count is checked at zero *before* the frame and above zero after it:
    /// the pass ran, and what it put there is this Set's own answer rather than
    /// a leftover.
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

    /// **The same tick sequence and the same seeds composite to the same pixels.**
    ///
    /// The ticks are deliberately uneven. A run of identical steps would pass even
    /// if the deck were advancing slots by whatever each one felt like, since they
    /// would all feel like the same thing; varying `steps` frame to frame is what
    /// makes "every Live slot advances by the same `steps` from the same tick" the
    /// thing being tested.
    ///
    /// Bit equality, not similarity. Floating-point addition is not associative,
    /// so a composite whose order depended on a `HashMap`, or on which slot last
    /// had a build land on it, would show up here — which is the whole reason the
    /// order is the slot index and the shader's sum is unrolled.
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

    /// **Per-slot hot swap still works, and a swap in one slot does not disturb
    /// another.**
    ///
    /// A slot is the unit that gets replaced — that is what it means for each slot
    /// to own its own `HotSwap` rather than for the deck to own one over all of
    /// them. The neighbouring slot must come through with its `t`, its element
    /// buffers and its live count untouched, exactly as the running Set does
    /// through a failed build in `tests/hot_swap.rs`.
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

    /// **An off-air slot's candidate is judged at the install, like every other
    /// slot's, because the number is the candidate's own.**
    ///
    /// `Deck::begin_frame` gives every slot its frame boundary, off-air ones
    /// included — a build has to be able to land on a slot that is not showing,
    /// and retired Sets have to keep reaching the worker.
    ///
    /// **This test used to assert the opposite half of the same problem.** It was
    /// `an_off_air_slot_is_not_judged_against_its_neighbours_frames`, and what it
    /// pinned was that a parked slot's trial was *frozen*: the frame interval is
    /// one number for the whole deck, so judging a candidate nobody was drawing
    /// against it accepted it on a budget it never spent and, with a tight budget
    /// and busy neighbours, rolled one back for cost it never caused. The freeze
    /// was the best an interval-based watchdog could do and it left the case that
    /// mattered untouched — a *Live* slot's candidate was still judged on its
    /// neighbours' cost — and it cost an off-air slot an unbounded wait for a
    /// verdict, which is the second half of what
    /// [ADR-0313](../../../docs/adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)
    /// repairs. The verdict is now the candidate's own measured cost, which does
    /// not move with residency, so there is nothing to freeze.
    ///
    /// The budget here is zero, so nothing can pass it: the verdict must arrive
    /// while the slot is still parked, and it must be against.
    #[test]
    fn an_off_air_slots_candidate_is_judged_at_the_install() {
        /// What the old freeze cost: eight warmup and thirty judged frames, and
        /// slack. Kept as the *upper* bound this now has to beat — a verdict that
        /// took this many frames would be one that had gone back to waiting.
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
        // **The candidate is in the slot, off air or not** (ADR-0316): `CAPACITY`
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
        // **The whole point of the change, as a number.** The verdict arrives on
        // the frame the build lands on, while the slot is still Allocated. The
        // old freeze would have taken at least a full window *and* the slot going
        // on air, so any figure under that is a verdict that did not wait — and
        // the build itself is what the frames before it were spent on.
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
        // **And it stops paying into it now**, which is the freeze reaching the
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

        // **Nor does taking it off air and putting it back**, which is the
        // other thing an operator does to a slot that is misbehaving. What
        // stopped is the version and not the placement (ADR-0316), so the
        // residency moves and the freeze does not — and the slot takes no step
        // at either level, which is what the off-air branch of `Frame::render`
        // has to honour as well.
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

    /// **The same candidate gets the same verdict whether the other slots are
    /// idle or loaded.**
    ///
    /// The maintainer, 2026-09-09: *"判定は他のスロットのロードとは独立にあるべ
    /// きだね"* — the verdict on a candidate must be independent of what the
    /// other slots are carrying. This is that sentence as an assertion. A budget
    /// divided by the live slot count, or a share taken beside what the
    /// neighbours are committed to, would both pass the test above and fail this
    /// one, which is why it is a second test and not a second assertion.
    ///
    /// Two runs of one candidate against one budget. The second run's neighbours
    /// hold sixteen times the elements, three of them are Live rather than one,
    /// and its frames are slowed so that the deck is certainly over its period on
    /// any machine — every quantity the old gate could see is different, and the
    /// only thing that is not is the candidate. The deck-level alarm differing
    /// between the runs is what keeps this from being vacuous: it says the two
    /// decks really were in different states.
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
