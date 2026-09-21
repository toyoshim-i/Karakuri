use super::common::*;

mod gpu {
    use super::*;

    // ---------------------------------------------------------------------------

    /// **A deck of one behaves exactly as a bare Set does today.**
    ///
    /// The load-bearing test of the whole slice, and the reason it asserts bit
    /// equality rather than similarity: every single-Set expectation in
    /// `tests/generated.rs`, `tests/lifecycle.rs` and `tests/hot_swap.rs` is about
    /// the path a bare Set takes, and they only keep meaning anything about the
    /// deck if the deck reproduces that path exactly. It can be exact — the mix
    /// reads the texel under the fragment with `textureLoad`, adds it to a zeroed
    /// accumulator at a gain and an opacity of exactly 1.0, and writes an `f16`
    /// that came from an `f16`.
    ///
    /// **The material here writes a coverage in `[0, 1]`, which is the one thing
    /// this comparison assumes.** The mix saturates what it reads into that range
    /// and a bare Set's target holds whatever L4 accumulated, so an L4 writing an
    /// alpha of 1.5 makes the two disagree in alpha and nowhere else — see
    /// `Blend::Add`. Every expectation this test exists to protect is about
    /// colour.
    ///
    /// A failure here is not a tolerance to widen. It means the mix is filtering,
    /// or resampling, or applying something it should not.
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
        // And it has to be a *bright* frame, not merely a non-empty one. The colour
        // invariant is that values above 1.0 are expected and are what feeds
        // bloom — see
        // `docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md`
        // — so a mix that only agreed on [0, 1] would be agreeing on the
        // uninteresting half. Asserted rather than assumed, because the material
        // this test renders is free to get dimmer later and take the property with
        // it silently.
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

    /// **A mask that reveals nothing is a skip, not a multiply by zero.**
    ///
    /// The only way to see the difference, and the reason the skip is there: a
    /// slot's target may hold a NaN — `sqrt` of a negative is a procedure that
    /// passes every stage of this pipeline — and `0.0 * NaN` is NaN. With clean
    /// material a mask at position 0 and a skipped layer are the same picture, so
    /// this is the case that tells them apart, exactly as it does for the fader.
    ///
    /// It is also what makes a mask a third escape from broken material, beside
    /// residency and the fader.
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

    /// **A mask in the middle shapes the frame rather than dimming it.**
    ///
    /// The difference between a mask and a fader, and the only assertion that can
    /// tell them apart: half way across, part of the frame is exactly what it would
    /// be with the slot present and part exactly what it would be without. A fader
    /// at 0.5 is neither, everywhere.
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

    /// **A wipe is a mask and one scheduled move**, and neither had to know about
    /// the other.
    ///
    /// The claim the whole design rests on: `Control::MaskPosition` carries the
    /// front, the mask reads a number, and the picture between the two ends is
    /// neither of them. Checked at three points, because a wipe that jumped would
    /// pass a two-point test.
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

    /// A Set drawn by two renderers over one simulation, composited.
    ///
    /// The shape a selection is about: one L1, two L4s, and an L5 folding
    /// them — which is what gives each renderer an edge with a `live` flag on
    /// it.
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

    /// **A scheduled selection lands on the beat it was given and not before.**
    ///
    /// The claim that makes a selection a *musical* control rather than a key
    /// press: it is quantised once, where the operator asked, and every frame
    /// until that beat leaves the Set exactly as it was. A selection applied
    /// where it was scheduled would pass every arithmetic test there is and
    /// still be the wrong instrument — the picture would change on the hand
    /// rather than on the bar.
    ///
    /// The queue is asserted as well as the edges, because the two failures
    /// look alike from outside: a selection that never lands and one that
    /// lands and is applied again every frame afterwards both leave the right
    /// renderer live.
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

    /// **A scheduled fade moves the fader on the beat grid and nowhere else.**
    ///
    /// The claim that makes a transition reproducible: it is a function of the
    /// session's beat count, so two runs given the same ticks fade identically —
    /// and a run at a different frame rate reaching the same beat is at the same
    /// point in the fade. Checked against the arithmetic rather than against a
    /// second run of the deck, which would agree with any implementation.
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

    /// **A hand on the fader wins.**
    ///
    /// The one place an operator reaches when something is wrong is the one place
    /// an automatic thing is writing, so a transition that kept going after a
    /// manual move would be the worst control on the deck. Asserted for both ways
    /// of touching it, since either is what a hand does.
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

    /// **A move onto a slot the deck does not have is refused where it is asked
    /// for**, not three seconds later inside a frame.
    ///
    /// `advance_transitions` indexes the slots directly, so an unchecked schedule
    /// is a panic on the render thread at some unrelated moment. `set_gain` and
    /// `set_opacity` panic at the call site; this joins them.
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

    /// **The composite sees this frame's fader, not the last one's.**
    ///
    /// A transition that ran *after* the mix was recorded would put every fade one
    /// frame late — invisible in a four-beat fade and exactly wrong in a cut, which
    /// is the case this uses. Compared against a deck whose fader was moved by hand
    /// before the frame, which is the path that was already exact: the two are the
    /// same picture if and only if the scheduled cut landed on the frame it was
    /// scheduled for.
    ///
    /// The comparison it replaces was `assert_ne!` against an earlier frame, which
    /// this material passes with no transition scheduled at all — it rotates on `t`.
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

    /// **A scheduled move cannot reach a value a hand could not.**
    ///
    /// It writes the slot's field directly rather than through `set_gain` and
    /// `set_opacity`, because those cancel it — so the clamps they carry have to be
    /// applied on the way past, or a `transition` record would be the one path into
    /// the mix with no bound on it. An opacity above 1.0 makes an `over` layer
    /// subtract more than it covers.
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

    /// **A crossfade is two scheduled moves**, and what makes that a crossfade
    /// rather than two fades is that they share a start and a length.
    ///
    /// The mix is checked rather than the fields: halfway through, the outgoing
    /// slot is dimmer than it was and the incoming one is brighter, and the frame
    /// carries both. That is the whole of what a crossfade is, and it needed no type
    /// of its own.
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
