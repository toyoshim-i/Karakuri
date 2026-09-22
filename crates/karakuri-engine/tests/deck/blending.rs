use super::common::*;

mod gpu {
    use super::*;

    /// **`over` hides what is under it and `add` does not**, which is the whole of
    /// what the blend vocabulary buys.
    ///
    /// Slot 1 is [`L4_CARD`] — black, opaque, and contributing no colour at all —
    /// so the two modes differ by exactly one thing: whether the coverage it drew
    /// is allowed to take the layer under it away. The expectation is not a
    /// direction but a number, read from the card's own target:
    ///
    /// ```text
    ///   add:   A + 0        = A
    ///   over:  0 + A*(1 - c)         c = the card's coverage at that texel
    /// ```
    ///
    /// Inexact for the same single reason as `gain_is_linear_...`: the GPU works in
    /// `f32` and rounds once to `f16` on write, while the expectation is computed
    /// in `f32` from values already rounded to `f16`.
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

    /// **`max` stacks without summing.**
    ///
    /// Two lit slots. Under `add` the mix is `A + B`; under `max` it is the larger
    /// of the two per channel, which is what makes four layers of the same bright
    /// material stay that bright instead of reaching four times it. `A` and `B` are
    /// measured on their own — same deck, same seeds, same ticks, one slot off air
    /// each time — so both are sampled at the same `t` as the mix.
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

    /// **An opacity outside `[0, 1]` is clamped where the engine takes it**, not
    /// where a key press produces it.
    ///
    /// `karakuri-cli` clamps at the key so that the `opacity` record carries the
    /// value that took effect, but a record is also how a *replay* drives the deck,
    /// and a stream is allowed to say anything. Past 1.0 an `over` layer subtracts
    /// more than it covers; below 0.0 it adds what it should have hidden. Unlike
    /// gain — a level into an HDR mix, deliberately open above 1.0 — every value
    /// outside this range has exactly one sensible reading, so it is clamped rather
    /// than refused.
    ///
    /// NaN silences, which is the third value a fader can carry and the one with no
    /// obvious reading: the two available are "this slot goes dark" and "the whole
    /// mix goes dark".
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

    /// **A gain a record could carry is floored at zero, and a NaN reads as zero.**
    ///
    /// The same hole as the one above and it needed the same answer: `karakuri-cli`
    /// floors at the key press, which says plainly that a negative gain is wrong,
    /// but a replayed `{"t":"gain","slot":1,"value":-2.0}` does not go through a
    /// key press. Unbounded *above*, unlike opacity, because gain is a level into
    /// an HDR mix and 4.0 is an ordinary thing to want.
    ///
    /// The NaN case is the one that cannot be recovered from. A NaN gain puts a NaN
    /// in every channel of the mix from one slot, and unlike the material's own
    /// NaN — which the fader skips past — no fader undoes a gain that has already
    /// multiplied by one.
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

    /// **Opacity moves the mix at settings between silence and full, under every
    /// mode.**
    ///
    /// Every other test here pins the fader at 0.0 or 1.0, where a composite that
    /// ignored `opacity` outright is indistinguishable from one that honours it —
    /// 0.0 is the skip, which [`Blend::silent_at`] decides on the host, and 1.0 is
    /// the identity. So without this test the one control this whole slice exists
    /// to make real has nothing saying it does anything.
    ///
    /// Each mode gets its own reference, and none of them is a second run of the
    /// composite at a different fader:
    ///
    /// ```text
    ///   add:   a half fader at gain g is bit-identical to a full fader at g/2
    ///   over:  A*(1 - o*c)              c = the card's coverage
    ///   max:   mix(A, max(A, B), o)     A and B measured on their own
    /// ```
    #[test]
    fn opacity_moves_the_mix_at_settings_between_zero_and_one() {
        let gpu = Gpu::headless().expect("no GPU available");
        const HALF: f32 = 0.5;
        const GAIN: f32 = 1.4;

        // --- `add`: **in colour**, opacity is the same multiply gain is, so it can
        // be checked against gain exactly. This is the collapse the deck's two
        // numbers used to be justified by, asserted rather than asserted about.
        //
        // Colour and not the whole texel, because the collapse stops at the alpha
        // channel: opacity scales coverage and gain does not, so the same picture
        // under the two settings carries a different coverage. That is the
        // difference between a fader and a level, showing up in the one channel
        // where `add` cannot hide it.
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

    /// **Zero gain silences `add` and `max`, and still covers under `over`.**
    ///
    /// The other half of [`Blend::silent_at`] — the fader's half is asserted
    /// against material that has gone NaN, in
    /// `a_slot_faded_to_silence_cannot_take_the_mix_with_it`, because that is where
    /// a skip and a multiply by zero stop agreeing.
    ///
    /// Here the material is clean and the asymmetry is what is being pinned: a
    /// layer at zero level contributes no colour, so under `add` and `max` it is
    /// not there at all — and under `over` it is a black card, which covers. Slot 1
    /// is [`L4_CARD`], so that difference is most of the frame rather than a few
    /// bits.
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
        // ...and does not under `over`, where zero gain is a black card and a black
        // card covers. Asserted rather than left as a comment, because it is the
        // one place the two faders stop being interchangeable and an operator
        // reaching for the wrong one gets a frame that goes dark instead of a
        // layer that goes away.
        //
        // **Colour channels only.** A whole-buffer `assert_ne!` would pass on the
        // alpha channel alone — coverage composes whatever the colour mode does, so
        // a slot that reached the mix and changed nothing visible still moves it —
        // and this claim is about what the picture does.
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

    /// **Two slots at gain 1.0 and 0.0 render what slot 0 alone renders.**
    ///
    /// Exact, for the same reason as above with one addition: `acc + 0.0 * x` is
    /// `acc` for every finite `x`, so a silenced slot contributes nothing at all
    /// rather than something below a threshold.
    ///
    /// Note what is *not* silenced: the slot is still `Live`, so it is still
    /// stepped and still rendered into its own target. Gain is a mixer fader, not
    /// a residency level, and conflating the two is how a fader move would come to
    /// cost a simulation.
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

    /// **Each Live slot renders into its own HDR target**, and the mix is a sum of
    /// exactly those targets.
    ///
    /// This is the decision the module doc defends — additive-only would allow one
    /// shared target, blend modes and masks will not — so it is worth an assertion
    /// rather than only a comment. Slot 0's own target is checked against what a
    /// deck of one holding the same Set mixes, which is the same picture by
    /// definition if and only if the slot rendered alone into somewhere of its
    /// own; two Sets sharing a target would have summed there instead, and slot
    /// 0's target would hold the sum.
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

    /// **Gain is linear, and applied per slot before the sum rather than to the
    /// sum.**
    ///
    /// The two are only distinguishable with more than one slot at more than one
    /// gain, and they differ by an entire slot's contribution:
    ///
    /// ```text
    ///   before (what this asserts):   2*A + B
    ///   after  (what it must not be): 2*(A + B)
    /// ```
    ///
    /// So `A` and `B` are measured on their own — same deck, same seeds, same
    /// ticks, one slot silenced each time, so both are sampled at the same `t` as
    /// the mix is — and the mix is checked against the first expression and
    /// against the second.
    ///
    /// This is the one comparison here that cannot be exact. The GPU sums in `f32`
    /// and rounds once, to `f16`, on write; the expectation is computed in `f32`
    /// from values that are already `f16`. The gap is that single rounding, which
    /// is 2^-11 relative, so the tolerance is 2^-10 — tight enough that the
    /// alternative hypothesis misses it by three orders of magnitude, which the
    /// second half of the test asserts rather than assumes.
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
        // **Colour only.** The fourth channel is coverage rather than a colour —
        // `1 - prod(1 - a_i)`, composed as `over` under every blend mode — and gain
        // deliberately does not reach it: turning a layer's level down dims what it
        // draws and does not change what it covers. So alpha is neither `2*A + B`
        // nor `2*(A + B)`, and including it here would be asserting linearity of a
        // channel this deck promises is not linear.
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
