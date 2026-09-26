use super::common::*;

mod gpu {
    use super::*;

    /// Verifies that `over` blend hides occluded layers while `add` sums them.
    #[test]
    fn over_hides_what_is_under_it_and_add_does_not() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |blend: Blend| -> (Vec<f32>, Vec<f32>) {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, L4_CARD, SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_blend(karakuri_engine::DeckSlot(1), blend);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            (
                decode(&readback(&gpu, present.hdr_texture())),
                decode(&readback(
                    &gpu,
                    deck.slot_target(karakuri_engine::DeckSlot(1)),
                )),
            )
        };

        let (added, card) = run(Blend::Add);
        let (overed, _) = run(Blend::Over);

        // The card has to actually cover something, or every assertion below is
        // `A == A` and this test says nothing. Alpha is the fourth channel.
        let covered = card.iter().skip(3).step_by(4).filter(|c| **c > 0.5).count();
        assert!(
            covered > 100,
            "the card covered only {covered} texels, so there is nothing for `over` to hide"
        );

        const TOLERANCE: f32 = 1.0 / 1024.0;
        let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

        let mut misses = 0;
        let mut worst = 0.0f32;
        let mut hidden = 0;
        // Colour only: the fourth channel is coverage, and what the mix does with
        // coverage is the same under every mode by construction.
        for i in (0..added.len()).filter(|i| i % 4 != 3) {
            let coverage = card[(i / 4) * 4 + 3];
            let expected = added[i] * (1.0 - coverage);
            if !close(overed[i], expected) {
                misses += 1;
                worst = worst.max((overed[i] - expected).abs());
            }
            if !close(overed[i], added[i]) {
                hidden += 1;
            }
        }

        assert_eq!(
            misses,
            0,
            "`over` is not `A*(1 - coverage)`: {misses} of {} colour channels disagree, \
     worst by {worst}",
            added.len()
        );
        // The other half, and the half that fails if `over` silently stayed `add`:
        // the two modes have to differ somewhere, or the first assertion passed
        // only because the coverage was zero everywhere it looked.
        assert!(
            hidden > 100,
            "only {hidden} channels distinguish `over` from `add`, so this run cannot tell \
     the two modes apart"
        );
    }

    /// Verifies that `max` blend selects the channel-wise maximum rather than summing.
    #[test]
    fn max_takes_the_larger_of_two_layers_rather_than_their_sum() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |blend: Blend, off_air: Option<usize>| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_blend(karakuri_engine::DeckSlot(1), blend);
            if let Some(slot) = off_air {
                deck.set_residency(karakuri_engine::DeckSlot(slot as u8), Residency::Allocated);
            }
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            decode(&readback(&gpu, present.hdr_texture()))
        };

        let a = run(Blend::Add, Some(1));
        let b = run(Blend::Add, Some(0));
        let summed = run(Blend::Add, None);
        let maxed = run(Blend::Max, None);

        const TOLERANCE: f32 = 1.0 / 1024.0;
        let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

        let mut misses = 0;
        let mut worst = 0.0f32;
        let mut distinct = 0;
        for i in (0..maxed.len()).filter(|i| i % 4 != 3) {
            let expected = a[i].max(b[i]);
            if !close(maxed[i], expected) {
                misses += 1;
                worst = worst.max((maxed[i] - expected).abs());
            }
            // Where both layers are lit, `max` is strictly less than `add`. If
            // nowhere is, the two slots never overlap and the comparison above is
            // `A + 0` against `max(A, 0)`, which agree.
            if !close(maxed[i], summed[i]) {
                distinct += 1;
            }
        }

        assert_eq!(
            misses,
            0,
            "`max` is not the per-channel maximum: {misses} of {} colour channels disagree, \
     worst by {worst}",
            maxed.len()
        );
        assert!(
            distinct > 100,
            "only {distinct} channels distinguish `max` from `add`, so the two slots barely \
     overlap and this run cannot tell them apart"
        );
    }

    /// Verifies that opacity values are clamped to `[0.0, 1.0]`, with NaN defaulting to 0.0.
    #[test]
    fn an_opacity_a_record_could_carry_is_clamped_to_a_fader() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[SEED_A]);

        for (asked, expected) in [
            (3.0, 1.0),
            (1.0, 1.0),
            (0.5, 0.5),
            (0.0, 0.0),
            (-2.0, 0.0),
            (f32::INFINITY, 1.0),
            (f32::NEG_INFINITY, 0.0),
            (f32::NAN, 0.0),
        ] {
            deck.set_opacity(karakuri_engine::DeckSlot(0), asked);
            assert_eq!(
                deck.opacity(karakuri_engine::DeckSlot(0)),
                expected,
                "an opacity of {asked} reached the mix as {}",
                deck.opacity(karakuri_engine::DeckSlot(0))
            );
        }
    }

    /// Verifies that gain values are floored at zero without an upper bound, with NaN defaulting to 0.0.
    #[test]
    fn a_gain_a_record_could_carry_is_floored_but_not_ceilinged() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[SEED_A]);

        for (asked, expected) in [
            (4.0, 4.0),
            (1.0, 1.0),
            (0.0, 0.0),
            (-2.0, 0.0),
            (f32::INFINITY, f32::INFINITY),
            (f32::NEG_INFINITY, 0.0),
            (f32::NAN, 0.0),
        ] {
            deck.set_gain(karakuri_engine::DeckSlot(0), asked);
            assert_eq!(
                deck.gain(karakuri_engine::DeckSlot(0)),
                expected,
                "a gain of {asked} reached the mix as {}",
                deck.gain(karakuri_engine::DeckSlot(0))
            );
        }
    }

    /// Asserts that out-of-range alpha values (greater than 1, negative, infinity, or NaN)
    /// are saturated on input and cannot invert the mix or produce NaNs in composite output.
    #[test]
    fn an_alpha_that_is_not_a_coverage_cannot_invert_the_mix_or_nan_it() {
        let gpu = Gpu::headless().expect("no GPU available");

        for (name, alpha) in OVERDRAWN_ALPHA {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build_with(&gpu, L4_WIDE, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, &overdrawn_card(alpha), SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_blend(karakuri_engine::DeckSlot(1), Blend::Over);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }

            // The material has to actually be out of range, or this is a test of
            // ordinary coverage under a frightening name.
            let own = decode(&readback(
                &gpu,
                deck.slot_target(karakuri_engine::DeckSlot(1)),
            ));
            let bad = own
                .iter()
                .skip(3)
                .step_by(4)
                .filter(|a| !(0.0..=1.0).contains(*a))
                .count();
            assert!(
                bad > 100,
                "the {name} card left only {bad} texels outside a coverage of [0, 1], so \
         nothing here is being saturated"
            );

            let mixed = decode(&readback(&gpu, present.hdr_texture()));
            let nan = (0..mixed.len())
                .filter(|i| i % 4 != 3)
                .filter(|&i| mixed[i].is_nan())
                .count();
            assert_eq!(
                nan, 0,
                "{nan} colour channels of the mix are NaN under the {name} card, so an alpha \
         nothing draws with reached every channel of the frame"
            );
            let negative = (0..mixed.len())
                .filter(|i| i % 4 != 3)
                .filter(|&i| mixed[i] < 0.0)
                .count();
            assert_eq!(
                negative, 0,
                "{negative} colour channels of the mix are negative under the {name} card, so \
         the coverage turned `over` from hiding into subtracting"
            );
            let unbounded = mixed
                .iter()
                .skip(3)
                .step_by(4)
                .filter(|a| !(0.0..=1.0).contains(*a))
                .count();
            assert_eq!(
                unbounded, 0,
                "{unbounded} texels of the mix carry a coverage outside [0, 1] under the \
         {name} card, which is what the next slot in the stack would be composited \
         against"
            );
        }
    }

    /// Verifies that opacity scales the composite between 0.0 and 1.0 across all blend modes.
    #[test]
    fn opacity_moves_the_mix_at_settings_between_zero_and_one() {
        let gpu = Gpu::headless().expect("no GPU available");
        const HALF: f32 = 0.5;
        const GAIN: f32 = 1.4;

        // In colour, opacity is the same multiply as gain under `add`.
        let add_run = |gain: f32, opacity: f32| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_gain(karakuri_engine::DeckSlot(1), gain);
            deck.set_opacity(karakuri_engine::DeckSlot(1), opacity);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            decode(&readback(&gpu, present.hdr_texture()))
        };
        let colour = |mix: &[f32]| -> Vec<f32> {
            (0..mix.len())
                .filter(|i| i % 4 != 3)
                .map(|i| mix[i])
                .collect()
        };

        let half_fader = colour(&add_run(GAIN, HALF));
        let half_gain = colour(&add_run(GAIN * HALF, 1.0));
        let full = colour(&add_run(GAIN, 1.0));
        assert_ne!(
            full,
            half_gain,
            "gain {GAIN} and gain {} render the same colours, so the comparison below is \
     vacuous",
            GAIN * HALF
        );
        assert_eq!(
            half_fader, half_gain,
            "under `add`, a fader at {HALF} is not the multiply a gain at the same factor is"
        );

        // --- `over` and `max` share the numeric comparison, and it is inexact for
        // the single reason the other numeric tests here are: `f32` on the GPU,
        // rounded once to `f16` on write, against an expectation computed in `f32`
        // from values already rounded.
        const TOLERANCE: f32 = 1.0 / 1024.0;
        let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

        // --- `over`: the card contributes no colour, so a half fader has to leave
        // exactly half the hole a full one does.
        let over_run = |blend: Blend, opacity: f32| -> (Vec<f32>, Vec<f32>) {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, L4_CARD, SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_blend(karakuri_engine::DeckSlot(1), blend);
            deck.set_opacity(karakuri_engine::DeckSlot(1), opacity);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            (
                decode(&readback(&gpu, present.hdr_texture())),
                decode(&readback(
                    &gpu,
                    deck.slot_target(karakuri_engine::DeckSlot(1)),
                )),
            )
        };
        // `add` at full opacity is `A + 0`, which is `A`.
        let (bare, card) = over_run(Blend::Add, 1.0);
        let (half_hole, _) = over_run(Blend::Over, HALF);

        let mut misses = 0;
        let mut worst = 0.0f32;
        let mut moved = 0;
        for i in (0..bare.len()).filter(|i| i % 4 != 3) {
            let coverage = card[(i / 4) * 4 + 3];
            let expected = bare[i] * (1.0 - HALF * coverage);
            if !close(half_hole[i], expected) {
                misses += 1;
                worst = worst.max((half_hole[i] - expected).abs());
            }
            if !close(half_hole[i], bare[i]) {
                moved += 1;
            }
        }
        assert_eq!(
            misses,
            0,
            "under `over`, a fader at {HALF} is not `A*(1 - {HALF}*coverage)`: {misses} of {} \
     colour channels disagree, worst by {worst}",
            bare.len()
        );
        assert!(
            moved > 100,
            "a fader at {HALF} under `over` moved only {moved} colour channels, so this run \
     cannot see the fader at all"
        );

        // --- `max`: a crossfade *into* the maximum rather than a switch to it, so
        // a half fader is halfway between the layer under it and the maximum.
        let max_run = |blend: Blend, opacity: f32, off_air: Option<usize>| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_blend(karakuri_engine::DeckSlot(1), blend);
            deck.set_opacity(karakuri_engine::DeckSlot(1), opacity);
            if let Some(slot) = off_air {
                deck.set_residency(karakuri_engine::DeckSlot(slot as u8), Residency::Allocated);
            }
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            decode(&readback(&gpu, present.hdr_texture()))
        };
        let a = max_run(Blend::Add, 1.0, Some(1));
        let b = max_run(Blend::Add, 1.0, Some(0));
        let halfway = max_run(Blend::Max, HALF, None);

        let mut misses = 0;
        let mut worst = 0.0f32;
        let mut moved = 0;
        for i in (0..halfway.len()).filter(|i| i % 4 != 3) {
            let expected = a[i] + HALF * (a[i].max(b[i]) - a[i]);
            if !close(halfway[i], expected) {
                misses += 1;
                worst = worst.max((halfway[i] - expected).abs());
            }
            if !close(halfway[i], a[i]) {
                moved += 1;
            }
        }
        assert_eq!(
            misses,
            0,
            "under `max`, a fader at {HALF} is not halfway to the maximum: {misses} of {} \
     colour channels disagree, worst by {worst}",
            halfway.len()
        );
        assert!(
            moved > 100,
            "a fader at {HALF} under `max` moved only {moved} colour channels away from the \
     layer under it, so this run cannot see the fader"
        );
    }

    /// Verifies that zero gain silences layers in `add` and `max`, but acts as an opaque black card in `over`.
    #[test]
    fn zero_gain_silences_add_and_max_and_still_covers_under_over() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |residency: Residency, blend: Blend, gain: f32, opacity: f32| -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, L4_CARD, SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_residency(karakuri_engine::DeckSlot(1), residency);
            deck.set_blend(karakuri_engine::DeckSlot(1), blend);
            deck.set_gain(karakuri_engine::DeckSlot(1), gain);
            deck.set_opacity(karakuri_engine::DeckSlot(1), opacity);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            readback(&gpu, present.hdr_texture())
        };

        let parked = run(Residency::Allocated, Blend::Add, 1.0, 1.0);
        assert!(lit(&parked) > 100, "the surviving slot drew nothing");

        // The level silences where a layer contributing no colour contributes
        // nothing at all...
        for blend in [Blend::Add, Blend::Max] {
            assert_eq!(
                run(Residency::Live, blend, 0.0, 1.0),
                parked,
                "a slot at gain 0.0 under `{}` reached the mix",
                blend.name()
            );
        }
        // Under `over`, zero gain acts as an opaque black card that occludes underlying content.
        // Asserts on color channels only, as alpha represents coverage.
        let dark = decode(&run(Residency::Live, Blend::Over, 0.0, 1.0));
        let bright = decode(&parked);
        let darkened = (0..dark.len())
            .filter(|i| i % 4 != 3)
            .filter(|&i| dark[i] < bright[i])
            .count();
        assert!(
            darkened > 100,
            "a zero-gain `over` layer darkened only {darkened} colour channels, so it has \
     stopped covering — which would make gain and opacity the same control again"
        );
    }

    /// Verifies that a slot at gain 0.0 contributes nothing to the mix while remaining active in simulation.
    #[test]
    fn a_slot_at_zero_gain_contributes_nothing_to_the_mix() {
        let gpu = Gpu::headless().expect("no GPU available");

        let alone_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut alone = deck_of(&gpu, &[SEED_A]);
        for _ in 0..12 {
            frame(&gpu, &mut alone, &alone_present, 1);
        }
        let expected = readback(&gpu, alone_present.hdr_texture());

        let both_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut both = deck_of(&gpu, &[SEED_A, SEED_B]);
        both.set_gain(karakuri_engine::DeckSlot(1), 0.0);
        for _ in 0..12 {
            frame(&gpu, &mut both, &both_present, 1);
        }
        let mixed = readback(&gpu, both_present.hdr_texture());

        assert_eq!(
            mixed, expected,
            "a slot at zero gain reached the mix anyway"
        );
        // The silenced slot ran regardless: gain is not residency.
        assert_eq!(
            steps_taken(both.slot(karakuri_engine::DeckSlot(1)).set()),
            12
        );
        assert_eq!(both.live_slots(), 2);
    }

    /// Verifies that each Live slot renders into its own HDR target before compositing.
    #[test]
    fn each_live_slot_renders_into_its_own_target() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        let first = readback(&gpu, deck.slot_target(karakuri_engine::DeckSlot(0)));
        let second = readback(&gpu, deck.slot_target(karakuri_engine::DeckSlot(1)));

        assert!(
            lit(&first) > 100 && lit(&second) > 100,
            "a slot drew nothing"
        );
        assert_ne!(
            first, second,
            "two slots at different seeds hold the same target contents"
        );

        let solo_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut solo = deck_of(&gpu, &[SEED_A]);
        for _ in 0..12 {
            frame(&gpu, &mut solo, &solo_present, 1);
        }
        assert_eq!(
            first,
            readback(&gpu, solo_present.hdr_texture()),
            "slot 0's target holds something other than slot 0's own render"
        );
    }

    /// Asserts that resizing the deck reallocates targets and updates bind groups correctly.
    #[test]
    fn resizing_the_deck_reallocates_and_rebinds() {
        // 512 * 8 bytes is a 256-byte-aligned row, which the readback needs, and a
        // different aspect ratio from 256x256, which puts the Sets' cameras on the
        // new viewport as well as the targets.
        const WIDE: u32 = 512;
        const TALL: u32 = 256;

        let gpu = Gpu::headless().expect("no GPU available");

        let grown_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDE, TALL);
        let mut grown = deck_of(&gpu, &[SEED_A, SEED_B]);
        grown.resize(&gpu.device, WIDE, TALL);
        for _ in 0..12 {
            frame(&gpu, &mut grown, &grown_present, 1);
        }
        let after_resize = readback(&gpu, grown_present.hdr_texture());

        let native_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDE, TALL);
        let mut native = deck_of_at(&gpu, &[SEED_A, SEED_B], WIDE, TALL);
        for _ in 0..12 {
            frame(&gpu, &mut native, &native_present, 1);
        }

        assert!(lit(&after_resize) > 100, "the resized deck drew nothing");
        assert_eq!(
            after_resize,
            readback(&gpu, native_present.hdr_texture()),
            "a resized deck is not the deck it would have been at that size"
        );
        assert_eq!(
            grown.slot_target(karakuri_engine::DeckSlot(0)).width(),
            WIDE
        );
        assert_eq!(
            grown.slot_target(karakuri_engine::DeckSlot(1)).height(),
            TALL
        );
    }

    /// Verifies that gain scaling is linear and applied per-slot prior to compositing rather than to the sum.
    #[test]
    fn gain_is_linear_and_applied_before_the_composite() {
        let gpu = Gpu::headless().expect("no GPU available");
        const STEPS: usize = 12;

        let run = |gains: [f32; 2]| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_gain(karakuri_engine::DeckSlot(0), gains[0]);
            deck.set_gain(karakuri_engine::DeckSlot(1), gains[1]);
            for _ in 0..STEPS {
                frame(&gpu, &mut deck, &present, 1);
            }
            decode(&readback(&gpu, present.hdr_texture()))
        };

        let a = run([1.0, 0.0]);
        let b = run([0.0, 1.0]);
        let mixed = run([2.0, 1.0]);

        assert!(
            a.iter().all(|v| v.is_finite()) && b.iter().all(|v| v.is_finite()),
            "a slot rendered an infinity, which makes the arithmetic below meaningless"
        );

        // One f16 rounding of the result, and nothing else.
        const TOLERANCE: f32 = 1.0 / 1024.0;
        let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

        let mut before_misses = 0;
        let mut after_misses = 0;
        let mut worst = 0.0f32;
        // Asserts on color channels only; alpha represents coverage rather than color.
        for i in (0..mixed.len()).filter(|i| i % 4 != 3) {
            let before = 2.0 * a[i] + b[i];
            let after = 2.0 * (a[i] + b[i]);
            if !close(mixed[i], before) {
                before_misses += 1;
                worst = worst.max((mixed[i] - before).abs());
            }
            if !close(mixed[i], after) {
                after_misses += 1;
            }
        }

        assert_eq!(
            before_misses,
            0,
            "the mix is not 2*A + B: {before_misses} of {} channels disagree, worst by {worst}",
            mixed.len()
        );
        // The second half of the claim. Without this the first half would also
        // pass on an all-black frame, or on one where B never contributed
        // anything — in either case `2*A + B` and `2*(A + B)` are the same number
        // and the test would be asserting nothing.
        assert!(
            after_misses > 100,
            "only {after_misses} channels distinguish `2*A + B` from `2*(A + B)`, so this run \
     could not have detected gain being applied to the mix instead of to the slot"
        );
    }
}
