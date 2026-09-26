use super::common::*;

mod gpu {
    use super::*;

    // ---------------------------------------------------------------------------

    /// Verifies bit-exact compositing equivalence between a single-slot deck and a standalone Set.
    #[test]
    fn a_deck_of_one_is_a_bare_set_bit_for_bit() {
        let gpu = Gpu::headless().expect("no GPU available");

        let bare_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut bare = build(&gpu, SEED_A, CAPACITY);
        for _ in 0..12 {
            bare_frame(&gpu, &mut bare, &bare_present, 1);
        }
        let expected = readback(&gpu, bare_present.hdr_texture());

        let deck_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A]);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &deck_present, 1);
        }
        let mixed = readback(&gpu, deck_present.hdr_texture());

        assert!(
            lit(&expected) > 100,
            "the bare Set drew nothing, so this test would pass on two black frames"
        );
        // Verifies the frame contains HDR values exceeding 1.0.
        let brightest = decode(&expected).into_iter().fold(0.0f32, f32::max);
        assert!(
            brightest > 1.0,
            "the bare Set peaked at {brightest}, so bit equality was only checked \
     below 1.0 — the range this pipeline is HDR for is untested"
        );
        assert_eq!(
            mixed, expected,
            "a deck of one slot at unity gain is not the bare Set it composites"
        );
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(0)).set()),
            12
        );
    }

    /// Asserts that a slot faded to silence (via opacity or gain under additive/max)
    /// is skipped and cannot contaminate the mix with NaNs (ADR-0040).
    #[test]
    fn a_slot_faded_to_silence_cannot_take_the_mix_with_it() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |silence: Residency, blend: Blend, gain: f32, opacity: f32| -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, L4_NAN, SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_residency(karakuri_engine::DeckSlot(1), silence);
            deck.set_blend(karakuri_engine::DeckSlot(1), blend);
            deck.set_gain(karakuri_engine::DeckSlot(1), gain);
            deck.set_opacity(karakuri_engine::DeckSlot(1), opacity);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            if silence == Residency::Live {
                // The faded slot is only interesting if its target really does
                // hold a NaN. (Parked, it has never stepped, so what it draws is
                // its zeroed element state and there is no NaN in it to check.)
                let own = readback(&gpu, deck.slot_target(karakuri_engine::DeckSlot(1)));
                assert!(
                    decode(&own).iter().any(|v| v.is_nan()),
                    "the NaN slot rendered no NaN, so this test is asserting nothing"
                );
            }
            readback(&gpu, present.hdr_texture())
        };

        // Off air: skipped by residency, and exact.
        let parked = run(Residency::Allocated, Blend::Add, 1.0, 1.0);
        assert!(lit(&parked) > 100, "the surviving slot drew nothing");

        let nans = |mix: &[u16]| decode(mix).iter().filter(|v| v.is_nan()).count();

        for blend in Blend::ALL {
            let faded = run(Residency::Live, blend, 1.0, 0.0);
            assert_eq!(
                faded,
                parked,
                "a slot at opacity 0.0 under `{}` reached the mix ({} NaN channels), while \
         the same slot taken off air did not",
                blend.name(),
                nans(&faded)
            );
        }
        for blend in [Blend::Add, Blend::Max] {
            let faded = run(Residency::Live, blend, 0.0, 1.0);
            assert_eq!(
                faded,
                parked,
                "a slot at gain 0.0 under `{}` reached the mix ({} NaN channels)",
                blend.name(),
                nans(&faded)
            );
        }
    }

    /// Asserts that masks at position 0.0 or 1.0 produce bit-exact matches to
    /// fully silenced or fully revealed slots respectively.
    #[test]
    fn a_mask_at_either_end_is_exactly_nothing_or_exactly_everything() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |mask: Option<Mask>, opacity: f32| -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, MASK_SIZE, MASK_SIZE);
            let mut deck = wash_deck(&gpu);
            deck.set_opacity(karakuri_engine::DeckSlot(1), opacity);
            if let Some(mask) = mask {
                deck.set_mask(karakuri_engine::DeckSlot(1), mask);
            }
            for _ in 0..4 {
                frame(&gpu, &mut deck, &present, 1);
            }
            readback(&gpu, present.hdr_texture())
        };

        let unmasked = run(None, 1.0);
        let silent = run(None, 0.0);
        assert!(lit(&unmasked) > 100, "the deck drew nothing");
        assert_ne!(unmasked, silent, "the two references are the same picture");
        // The wash has to reach every texel, or an end that is wrong at the edge
        // is an end nothing here can see.
        assert_eq!(
            lit(&unmasked),
            (MASK_SIZE * MASK_SIZE) as usize,
            "the wash does not cover the frame, so a mask's edges are untested"
        );

        for kind in [MaskKind::Linear, MaskKind::Radial] {
            // Every angle, because a linear front's normalisation is per-direction
            // and one that overshot would show at one angle and not another.
            for angle in [0.0, 0.7, std::f32::consts::FRAC_PI_2, 2.4, -0.7] {
                assert_eq!(
                    run(Some(Mask::new(kind, angle, 1.0, 0.3)), 1.0),
                    unmasked,
                    "{} at {angle} rad, fully open, is not the unmasked frame",
                    kind.name()
                );
                assert_eq!(
                    run(Some(Mask::new(kind, angle, 0.0, 0.3)), 1.0),
                    silent,
                    "{} at {angle} rad, fully closed, is not a silent slot",
                    kind.name()
                );
            }
        }
    }

    /// Verifies that fully closed masks skip slot evaluation, preventing NaNs from entering the composite.
    #[test]
    fn a_mask_that_reveals_nothing_keeps_a_nan_out_of_the_mix() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |silence: Residency, mask: Mask| -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, L4_NAN, SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_residency(karakuri_engine::DeckSlot(1), silence);
            deck.set_mask(karakuri_engine::DeckSlot(1), mask);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            if silence == Residency::Live {
                assert!(
                    decode(&readback(
                        &gpu,
                        deck.slot_target(karakuri_engine::DeckSlot(1))
                    ))
                    .iter()
                    .any(|v| v.is_nan()),
                    "the NaN slot rendered no NaN, so this test is asserting nothing"
                );
            }
            readback(&gpu, present.hdr_texture())
        };

        let parked = run(Residency::Allocated, Mask::default());
        assert!(lit(&parked) > 100, "the surviving slot drew nothing");

        for kind in [MaskKind::Linear, MaskKind::Radial] {
            let closed = run(Residency::Live, Mask::new(kind, 0.4, 0.0, 0.1));
            let nans = decode(&closed).iter().filter(|v| v.is_nan()).count();
            assert_eq!(
                closed,
                parked,
                "a slot masked to nothing under `{}` reached the mix ({nans} NaN channels), \
         while the same slot taken off air did not",
                kind.name()
            );
        }
    }

    /// Verifies that intermediate masks spatially shape the layer rather than uniformly dimming it.
    #[test]
    fn a_mask_half_way_leaves_one_part_untouched_and_removes_another() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |mask: Option<Mask>, opacity: f32| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, MASK_SIZE, MASK_SIZE);
            let mut deck = wash_deck(&gpu);
            deck.set_opacity(karakuri_engine::DeckSlot(1), opacity);
            if let Some(mask) = mask {
                deck.set_mask(karakuri_engine::DeckSlot(1), mask);
            }
            for _ in 0..4 {
                frame(&gpu, &mut deck, &present, 1);
            }
            decode(&readback(&gpu, present.hdr_texture()))
        };

        let silent = run(None, 0.0);
        // A hard front straight up the middle, left to right, on a slot at half
        // opacity — so "revealed" and "unmasked" are different pictures and the
        // mask cannot be mistaken for the fader that is also on.
        let halfway = run(Some(Mask::new(MaskKind::Linear, 0.0, 0.5, 0.0)), 0.5);
        let faded = run(None, 0.5);

        // **Only where the slot actually contributes.** Where it drew nothing, the
        // masked frame, the faded one and the silent one all agree, and counting
        // those would drown the claim in background.
        let mut hidden = 0;
        let mut revealed = 0;
        let mut between = 0;
        for i in (0..halfway.len()).filter(|i| i % 4 != 3) {
            if faded[i] == silent[i] {
                continue;
            }
            if halfway[i] == silent[i] {
                hidden += 1;
            } else if halfway[i] == faded[i] {
                revealed += 1;
            } else {
                between += 1;
            }
        }

        assert!(
            hidden > 100,
            "the mask removed the slot from {hidden} of the channels it drew, so the front \
     is not on the frame"
        );
        assert!(
            revealed > 100,
            "the mask left the slot in {revealed} of the channels it drew, so it is hiding \
     everything rather than shaping"
        );
        // A hard edge, so every contributing channel is on one side or the other.
        // A *fader* would put all of them in `between`, which is the difference
        // this test exists to see.
        assert!(
            between * 20 < hidden + revealed,
            "{between} channels are neither the revealed picture nor the hidden one, \
     against {} that are — a hard-edged mask is one or the other",
            hidden + revealed
        );
        // And the front is where it was asked for: half the covered frame, either
        // side. A wipe that finished early would still be "one or the other".
        let split = hidden as f32 / (hidden + revealed) as f32;
        assert!(
            (split - 0.5).abs() < 0.1,
            "the front left {split:.2} of the frame hidden rather than half, so it is not \
     where `position` says"
        );
    }

    /// Verifies that wipes progress smoothly as scheduled transitions controlling mask positions.
    #[test]
    fn a_wipe_is_a_transition_carrying_a_masks_front() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_signals(Signals::new(120.0, 1));
        deck.set_blend(karakuri_engine::DeckSlot(1), Blend::Over);
        deck.set_mask(
            karakuri_engine::DeckSlot(1),
            Mask::new(MaskKind::Linear, 0.0, 0.0, 0.02),
        );

        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            1,
            Control::MaskPosition,
            0.0,
            1.0,
            start,
            4.0,
            Curve::Lin,
        ));

        let mut fronts = Vec::new();
        for i in 0..121 {
            frame(&gpu, &mut deck, &present, 1);
            if i % 30 == 0 {
                fronts.push(deck.mask(karakuri_engine::DeckSlot(1)).position());
            }
        }
        // Monotone and strictly moving, which a jump would not be.
        for pair in fronts.windows(2) {
            assert!(
                pair[1] > pair[0],
                "the front went backwards or stood still: {fronts:?}"
            );
        }
        assert_eq!(
            deck.mask(karakuri_engine::DeckSlot(1)).position(),
            1.0,
            "the wipe did not finish"
        );
        // The shape survived: a move carries the position and leaves the kind
        // alone, which is why `set_mask_shape` does not cancel a transition.
        assert_eq!(
            deck.mask(karakuri_engine::DeckSlot(1)).kind(),
            MaskKind::Linear
        );
        assert_eq!(deck.transitions_on(karakuri_engine::DeckSlot(1)).count(), 0);
    }

    /// Asserts that manual adjustments to mask position cancel in-flight wipes (P-0094),
    /// while manual shape adjustments update the mask without interrupting the transition.
    #[test]
    fn a_hand_on_the_front_stops_the_wipe_and_a_hand_on_the_shape_does_not() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_signals(Signals::new(120.0, 1));
        let wipe = |deck: &mut Deck| {
            deck.set_mask_shape(karakuri_engine::DeckSlot(1), MaskKind::Linear, 0.0);
            deck.set_mask_position(karakuri_engine::DeckSlot(1), 0.0);
            let start = deck.signals().oscillator().beats();
            deck.schedule(Transition::new(
                1,
                Control::MaskPosition,
                0.0,
                1.0,
                start,
                4.0,
                Curve::Lin,
            ));
        };

        wipe(&mut deck);
        deck.set_mask_shape(karakuri_engine::DeckSlot(1), MaskKind::Radial, 0.0);
        assert_eq!(
            deck.transitions_on(karakuri_engine::DeckSlot(1)).count(),
            1,
            "choosing a shape mid-wipe cancelled the move — a shape writes no position, \
         so it is not a hand on the control the transition is carrying, and a wipe \
         that stopped because somebody changed what it wipes with is a move nobody \
         withdrew"
        );
        assert_eq!(
            deck.mask(karakuri_engine::DeckSlot(1)).kind(),
            MaskKind::Radial,
            "the shape did not land"
        );

        wipe(&mut deck);
        deck.set_mask_position(karakuri_engine::DeckSlot(1), 0.75);
        assert_eq!(
            deck.transitions_on(karakuri_engine::DeckSlot(1)).count(),
            0,
            "a hand on the front left the move running — the transition writes that same \
         number every frame, so it would take the front straight back and the \
         operator would be holding a control that fights back"
        );
        assert_eq!(
            deck.mask(karakuri_engine::DeckSlot(1)).position(),
            0.75,
            "the front did not land"
        );
    }

    /// Creates a Set with one L1 geometry and two L4 renderers composited via L5.
    fn set_of_two_renderers(gpu: &Gpu) -> Set {
        let (l1, a, b) = (compile(L1), compile(L4), compile(L4_B));
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&l1, CAPACITY)],
            &[],
            &[],
            &[],
            &[&a, &b],
            karakuri_engine::set::Layering::Composite,
            SEED_A,
            &[],
            karakuri_engine::set::Wiring::default(),
        )
        .expect("one geometry and two renderers over it");
        set.resize(&gpu.device, WIDTH, HEIGHT);
        set
    }

    /// Verifies that scheduled selections execute precisely on their target beat boundaries.
    #[test]
    fn a_scheduled_selection_lands_on_the_beat_it_was_given_and_not_before() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = Deck::new(
            &gpu.device,
            vec![HotSwap::fixed(set_of_two_renderers(&gpu))],
            WIDTH,
            HEIGHT,
        );
        // 120 bpm and dt of 1/60 is two beats a second, so a frame is 1/30 of a
        // beat and the bar below is 120 frames away rather than an unknown
        // number of them.
        deck.set_signals(Signals::new(120.0, 1));
        // Off the boundary first, so that quantising has somewhere to go: on
        // beat 0 the next bar *is* now, which is correct and is not this test.
        frame(&gpu, &mut deck, &present, 1);

        let now = deck.signals().oscillator().beats();
        let start = quantise(now, 4.0);
        assert!(start > now, "the fixture is already on the bar");
        deck.schedule_selection(Selection::new(0, 1, start));

        let live = |deck: &Deck| -> Vec<usize> {
            deck.slot(karakuri_engine::DeckSlot(0))
                .set()
                .inputs()
                .iter()
                .enumerate()
                .filter(|(_, input)| input.live)
                .map(|(at, _)| at)
                .collect()
        };
        assert_eq!(live(&deck), vec![0, 1], "a Set comes up folding both");

        while deck.signals().oscillator().beats() < start {
            assert_eq!(
                live(&deck),
                vec![0, 1],
                "the selection landed at {} beats, before the {start} it was given",
                deck.signals().oscillator().beats()
            );
            assert_eq!(
                deck.selections_on(karakuri_engine::DeckSlot(0)).count(),
                1,
                "the selection is armed"
            );
            frame(&gpu, &mut deck, &present, 1);
        }

        // The frame that crossed the instant applied it: exactly one live, and
        // it is the one that was asked for.
        assert_eq!(live(&deck), vec![1]);
        assert_eq!(
            deck.selections_on(karakuri_engine::DeckSlot(0)).count(),
            0,
            "a selection that has landed is still queued, and would be applied \
         over whatever moves the edges next"
        );
    }

    /// Verifies that scheduled fades evaluate deterministically from the session beat count.
    #[test]
    fn a_scheduled_fade_is_a_function_of_the_beat_count() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A]);
        // 120 bpm and dt of 1/60 is exactly two beats a second, so a frame is
        // 1/30 of a beat and the arithmetic below is exact.
        deck.set_signals(Signals::new(120.0, 1));

        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            0,
            Control::Opacity,
            1.0,
            0.0,
            start,
            4.0,
            Curve::Lin,
        ));

        // 120 frames is four beats: the whole fade.
        for i in 0..120 {
            let beats = deck.signals().oscillator().beats();
            let expected = 1.0 - (beats - start) as f32 / 4.0;
            assert!(
                (deck.opacity(karakuri_engine::DeckSlot(0)) - expected).abs() < 1e-6,
                "frame {i} at {beats} beats: the fader is {} rather than {expected}",
                deck.opacity(karakuri_engine::DeckSlot(0))
            );
            frame(&gpu, &mut deck, &present, 1);
        }
        // Exactly at silence, and the transition gone rather than still writing.
        assert_eq!(deck.opacity(karakuri_engine::DeckSlot(0)), 0.0);
        assert_eq!(
            deck.transitions_on(karakuri_engine::DeckSlot(0)).count(),
            0,
            "a finished fade is still scheduled"
        );
    }

    /// Verifies that manual control adjustments cancel active transitions on the target parameter.
    #[test]
    fn moving_a_control_by_hand_cancels_the_transition_moving_it() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A]);
        deck.set_signals(Signals::new(120.0, 1));

        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            0,
            Control::Opacity,
            1.0,
            0.0,
            start,
            8.0,
            Curve::Lin,
        ));
        for _ in 0..30 {
            frame(&gpu, &mut deck, &present, 1);
        }
        let mid = deck.opacity(karakuri_engine::DeckSlot(0));
        assert!(mid > 0.0 && mid < 1.0, "the fade did not start: {mid}");

        deck.set_opacity(karakuri_engine::DeckSlot(0), 0.75);
        for _ in 0..60 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            deck.opacity(karakuri_engine::DeckSlot(0)),
            0.75,
            "the fade kept writing after the fader was moved by hand"
        );
        assert_eq!(deck.transitions_on(karakuri_engine::DeckSlot(0)).count(), 0);

        // And the other control's transition is untouched by the wrong fader:
        // cancelling has to be per control, or a gain move would stop an opacity
        // fade and an operator would never find out why.
        deck.schedule(Transition::new(
            0,
            Control::Gain,
            1.0,
            0.0,
            start,
            8.0,
            Curve::Lin,
        ));
        deck.set_opacity(karakuri_engine::DeckSlot(0), 0.5);
        assert_eq!(
            deck.transitions_on(karakuri_engine::DeckSlot(0)).count(),
            1,
            "the gain fade was cancelled too"
        );

        // **The gain half of the rule, which this test claimed and did not check.**
        // `[`, `]` and `\` all end in `set_gain`, so a gain fade that kept writing
        // after one of them would be a control fighting the hand on it.
        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            0,
            Control::Gain,
            1.0,
            0.0,
            start,
            8.0,
            Curve::Lin,
        ));
        for _ in 0..30 {
            frame(&gpu, &mut deck, &present, 1);
        }
        let mid = deck.gain(karakuri_engine::DeckSlot(0));
        assert!(mid > 0.0 && mid < 1.0, "the gain fade did not start: {mid}");
        deck.set_gain(karakuri_engine::DeckSlot(0), 2.0);
        for _ in 0..60 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            deck.gain(karakuri_engine::DeckSlot(0)),
            2.0,
            "the gain fade kept writing after the level was moved by hand"
        );
        assert_eq!(deck.transitions_on(karakuri_engine::DeckSlot(0)).count(), 0);
    }

    /// Verifies that scheduling transitions on nonexistent slots panics immediately at the call site.
    #[test]
    #[should_panic(expected = "no slot 3")]
    fn scheduling_a_move_onto_a_slot_that_is_not_there_is_refused_at_the_call() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[SEED_A]);
        deck.schedule(Transition::new(
            3,
            Control::Opacity,
            1.0,
            0.0,
            0.0,
            4.0,
            Curve::Lin,
        ));
    }

    /// Verifies that scheduled cuts take effect on the frame they are scheduled for without single-frame lag.
    #[test]
    fn a_scheduled_cut_lands_on_the_frame_it_was_scheduled_for() {
        let gpu = Gpu::headless().expect("no GPU available");
        const LEAD: usize = 12;

        // The reference: the same deck, the same ticks, the fader moved by hand
        // before the frame in question.
        let by_hand = {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_signals(Signals::new(120.0, 1));
            for _ in 0..LEAD {
                frame(&gpu, &mut deck, &present, 1);
            }
            deck.set_opacity(karakuri_engine::DeckSlot(1), 0.0);
            frame(&gpu, &mut deck, &present, 1);
            readback(&gpu, present.hdr_texture())
        };

        // The same run, with the cut scheduled for the beat that frame lands on.
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_signals(Signals::new(120.0, 1));
        for _ in 0..LEAD {
            frame(&gpu, &mut deck, &present, 1);
        }
        // Where the clock stands. The next frame advances it first, so this
        // instant is already past by the time the transition is read — which is
        // the frame it is due on, and the frame the composite has to see it on.
        let cut_at = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            1,
            Control::Opacity,
            1.0,
            0.0,
            cut_at,
            0.0,
            Curve::Lin,
        ));
        frame(&gpu, &mut deck, &present, 1);
        let scheduled = readback(&gpu, present.hdr_texture());

        assert_eq!(
            deck.opacity(karakuri_engine::DeckSlot(1)),
            0.0,
            "the cut did not land on the frame it was scheduled for"
        );
        assert_eq!(
            scheduled, by_hand,
            "the mix on the frame of a scheduled cut is not the mix of the same fader moved \
     by hand — the composite is reading a fader the transition has not written yet"
        );
    }

    /// Verifies that values written by scheduled transitions respect the same clamps as manual adjustments.
    #[test]
    fn a_scheduled_move_is_clamped_the_way_a_manual_one_is() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A]);
        deck.set_signals(Signals::new(120.0, 1));
        let start = deck.signals().oscillator().beats();

        deck.schedule(Transition::new(
            0,
            Control::Opacity,
            1.0,
            4.0,
            start,
            0.0,
            Curve::Lin,
        ));
        deck.schedule(Transition::new(
            0,
            Control::Gain,
            1.0,
            -3.0,
            start,
            0.0,
            Curve::Lin,
        ));
        frame(&gpu, &mut deck, &present, 1);

        assert_eq!(
            deck.opacity(karakuri_engine::DeckSlot(0)),
            1.0,
            "a scheduled fader passed 1.0"
        );
        assert_eq!(
            deck.gain(karakuri_engine::DeckSlot(0)),
            0.0,
            "a scheduled level went negative"
        );
    }

    /// Verifies crossfade transitions configured as coordinated moves across outgoing and incoming slots.
    #[test]
    fn a_crossfade_is_two_moves_sharing_a_start_and_a_length() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_signals(Signals::new(120.0, 1));
        deck.set_opacity(karakuri_engine::DeckSlot(1), 0.0);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }

        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            0,
            Control::Opacity,
            1.0,
            0.0,
            start,
            4.0,
            Curve::Smooth,
        ));
        deck.schedule(Transition::new(
            1,
            Control::Opacity,
            0.0,
            1.0,
            start,
            4.0,
            Curve::Smooth,
        ));

        // Two beats: halfway, where both are somewhere in the middle.
        for _ in 0..60 {
            frame(&gpu, &mut deck, &present, 1);
        }
        let a = deck.opacity(karakuri_engine::DeckSlot(0));
        let b = deck.opacity(karakuri_engine::DeckSlot(1));
        assert!(a > 0.0 && a < 1.0 && b > 0.0 && b < 1.0, "{a} / {b}");
        assert!(
            (a + b - 1.0).abs() < 0.05,
            "a smooth crossfade should be near unity through the middle: {a} + {b}"
        );

        // Two more beats: the other end, exactly.
        for _ in 0..60 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(deck.opacity(karakuri_engine::DeckSlot(0)), 0.0);
        assert_eq!(deck.opacity(karakuri_engine::DeckSlot(1)), 1.0);
        assert_eq!(
            deck.transitions_on(karakuri_engine::DeckSlot(0)).count()
                + deck.transitions_on(karakuri_engine::DeckSlot(1)).count(),
            0
        );
    }
}
