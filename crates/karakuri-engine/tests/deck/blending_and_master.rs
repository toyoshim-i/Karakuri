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

    /// A frame the way a sink sees it: the canvas through the present pass —
    /// tone mapped, then sRGB encoded by the hardware — into a target of the
    /// canvas's own size, so `Present::draw`'s letterbox is the whole
    /// attachment exactly as it is on every offscreen render.
    ///
    /// Only the master-out tests below need it. Everything else in this file
    /// reads the HDR target, because everything else is about the mix; these
    /// are about the difference between the mix's output and the picture drawn
    /// from it, and that difference is this pass.
    fn shown(gpu: &Gpu, present: &Present) -> Vec<u8> {
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shown"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());

        let bytes_per_row = WIDTH * 4;
        assert_eq!(bytes_per_row % 256, 0, "row pitch must be 256-byte aligned");
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shown readback"),
            size: u64::from(bytes_per_row * HEIGHT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        present.draw(&mut encoder, &view, (WIDTH, HEIGHT));
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("map");
        let out = data.to_vec();
        drop(data);
        buffer.unmap();
        out
    }

    /// One run of a fixed deck under one master out and one exposure: the
    /// composited frame as written, and the picture drawn from it.
    ///
    /// A `Present` per run, at a **window's** surface format rather than
    /// [`Present::HDR_FORMAT`] — every other test in this file renders and
    /// never draws, so this is the only place the encode's own target has to
    /// exist. The HDR target is the same either way; the format only decides
    /// what `Present::draw` may be pointed at.
    fn under(gpu: &Gpu, out: f32, exposure: f32) -> (Vec<u16>, Vec<u8>) {
        const STEPS: usize = 12;
        let present = Present::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            WIDTH,
            HEIGHT,
        );
        present.set_tonemap(&gpu.queue, karakuri_engine::TonemapOp::Aces, exposure, 1.0);
        let mut deck = deck_of(gpu, &[SEED_A, SEED_B]);
        deck.set_out(out);
        for _ in 0..STEPS {
            frame(gpu, &mut deck, &present, 1);
        }
        (readback(gpu, present.hdr_texture()), shown(gpu, &present))
    }

    /// How many values two readbacks of the same shape disagree on. Counted
    /// rather than reported as a boolean, because every assertion below is
    /// about *how much* of the frame moved: one texel differing is a driver
    /// having a bad day and a hundred thousand is a level.
    fn differing<T: PartialEq>(a: &[T], b: &[T]) -> usize {
        assert_eq!(a.len(), b.len(), "two readbacks of different shapes");
        a.iter().zip(b).filter(|(x, y)| x != y).count()
    }

    /// **The master out is a level on the composited frame, and coverage is not
    /// a level.**
    ///
    /// Half the master out is half the colour, exactly: the mix folds in `f32`
    /// and rounds once on write, and scaling by a power of two commutes with
    /// that rounding for every normal `f16`, so the tolerance here is one
    /// subnormal step rather than a relative one. The fourth channel is
    /// coverage — `1 - prod(1 - a_i)`, composed as `over` under every blend
    /// mode — and it is asserted **bit for bit unchanged**, which is the same
    /// asymmetry `gain` has: turning a level down dims what a frame draws and
    /// does not change what it covers.
    #[test]
    fn the_master_out_scales_the_composited_frame_and_leaves_its_coverage_alone() {
        let gpu = Gpu::headless().expect("no GPU available");

        let (full, _) = under(&gpu, 1.0, 1.0);
        let (half, _) = under(&gpu, 0.5, 1.0);

        assert!(
            lit(&full) > 100,
            "the deck drew nothing, so this test would pass on two black frames"
        );
        let bright = decode(&full);
        let brightest = (0..bright.len())
            .filter(|i| i % 4 != 3)
            .fold(0.0f32, |m, i| m.max(bright[i]));
        assert!(
            brightest > 1.0,
            "the deck peaked at {brightest}, so the master out was only checked below 1.0 — \
         the range this pipeline is HDR for is untested"
        );

        // Half of a normal `f16` is exact; a value already in the subnormal
        // range can round by half a subnormal ulp, which is 2^-25.
        const TOLERANCE: f32 = 1e-7;
        let scaled = decode(&half);
        let mut misses = 0;
        let mut worst = 0.0f32;
        for i in (0..scaled.len()).filter(|i| i % 4 != 3) {
            let want = 0.5 * bright[i];
            if (scaled[i] - want).abs() > TOLERANCE {
                misses += 1;
                worst = worst.max((scaled[i] - want).abs());
            }
        }
        assert_eq!(
            misses, 0,
            "the master out is not a multiply on the folded colour: {misses} channels disagree, \
         worst by {worst}"
        );

        // Both halves of the claim need a witness. Without this one, a master
        // out that did nothing at all would satisfy the arithmetic above on
        // every black texel and be caught only where the frame is lit.
        let moved = (0..scaled.len())
            .filter(|i| i % 4 != 3 && scaled[*i] != bright[*i])
            .count();
        assert!(
            moved > 100,
            "only {moved} colour channels moved when the master out was halved, so this run \
         could not have detected it being ignored"
        );

        let coverage: Vec<u16> = full.iter().skip(3).step_by(4).copied().collect();
        let after: Vec<u16> = half.iter().skip(3).step_by(4).copied().collect();
        assert_eq!(
            differing(&coverage, &after),
            0,
            "the master out reached the alpha channel, which is coverage rather than a level"
        );
    }

    /// **The two levels multiply in different places, and this is where the
    /// difference is visible today.** The master out is applied where the mix
    /// *writes* the composited frame; the tone mapper's exposure is applied
    /// where the present pass *reads* it. So halving the master out moves the
    /// HDR target and halving the exposure leaves it bit for bit identical —
    /// while both move the picture drawn from it.
    ///
    /// That last assertion is the one that keeps this test honest. Without it,
    /// an exposure that had been quietly disconnected would pass the middle
    /// assertion perfectly, and the test would be reporting "the two are in
    /// different places" on the strength of one of them doing nothing at all.
    ///
    /// **This is the whole of what `docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md`
    /// can assert until the master chain exists**, and it is enough: a build
    /// that folded the two into one multiplication — either of the rejected
    /// options — fails here, whichever end it folded them at.
    #[test]
    fn the_master_out_is_at_the_chains_entry_and_exposure_is_at_the_tonemaps_input() {
        let gpu = Gpu::headless().expect("no GPU available");

        let (base_frame, base_shown) = under(&gpu, 1.0, 1.0);
        let (out_frame, out_shown) = under(&gpu, 0.5, 1.0);
        let (exposed_frame, exposed_shown) = under(&gpu, 1.0, 0.5);

        assert!(
            lit(&base_frame) > 100,
            "the deck drew nothing, so every comparison below is between two black frames"
        );

        let by_out = differing(&base_frame, &out_frame);
        assert!(
            by_out > 100,
            "the master out moved only {by_out} of {} values in the composited frame, so it is \
         not being applied where that frame is written",
            base_frame.len()
        );
        assert_eq!(
            differing(&base_frame, &exposed_frame),
            0,
            "the tone mapper's exposure changed the composited frame, so it is being applied \
         upstream of where the present pass reads it"
        );

        let shown_by_exposure = differing(&base_shown, &exposed_shown);
        assert!(
            shown_by_exposure > 100,
            "the exposure moved only {shown_by_exposure} of {} bytes of the picture, so the \
         frame it left untouched proves nothing",
            base_shown.len()
        );
        let shown_by_out = differing(&base_shown, &out_shown);
        assert!(
            shown_by_out > 100,
            "the master out moved only {shown_by_out} of {} bytes of the picture",
            base_shown.len()
        );
    }

    /// Asserts that with an empty master chain, master output level and exposure
    /// scaling commute and yield equivalent rendered output.
    #[test]
    fn with_nothing_in_the_master_chain_the_two_levels_are_the_same_picture() {
        let gpu = Gpu::headless().expect("no GPU available");

        let (_, base_shown) = under(&gpu, 1.0, 1.0);
        let (_, out_shown) = under(&gpu, 0.5, 1.0);
        let (_, exposed_shown) = under(&gpu, 1.0, 0.5);

        let moved = differing(&base_shown, &out_shown);
        assert!(
            moved > 100,
            "neither level changed the picture, so the agreement below is between two frames \
         nothing happened to"
        );

        let worst = out_shown
            .iter()
            .zip(&exposed_shown)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .expect("a picture has bytes in it");
        let differ = differing(&out_shown, &exposed_shown);
        assert!(
            worst <= 1,
            "a master out of 0.5 and an exposure of 0.5 drew different pictures — {differ} bytes \
         apart, worst by {worst}. Either something now sits between the two multiplications, \
         in which case this test has done its job and goes, or one of them is not a level."
        );
    }

    /// **`Allocated` leaves the mix and keeps running.** A slot taken off air
    /// goes on advancing at the room's tempo, and comes back on the beat the
    /// rest of the deck is on rather than at the `t` it left on.
    ///
    /// `t` is the sharpest witness available: simulation time only moves through
    /// `Set::prepare`, and every slot is handed one on every frame
    /// (ADR-0269) — so what going off air changes is the mix and nothing else.
    ///
    /// **It used to assert the opposite**, and the sentence it asserted —
    /// *`Allocated` keeps its state; a slot taken off air does not advance while
    /// it is off, and resumes where it stopped* — is the behaviour ADR-0269
    /// removed. What made it wrong is the cell: a slot that stands still while
    /// it is off air is a slot whose preview is a still, and the operator is
    /// deciding from that preview. The property it leaned on is still true
    /// somewhere, and that somewhere is `swap.rs`, which parks an *outgoing
    /// Set* across a watchdog window — a Set held outside the deck, which no
    /// residency reaches.
    ///
    /// Substepped while it is off air, so that "kept up" is a claim about steps
    /// rather than about frames: three steps a frame for ten frames is thirty,
    /// and a slot stepping once a frame regardless would read ten.
    #[test]
    fn a_slot_taken_off_air_keeps_running_and_comes_back_on_the_beat() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);

        for _ in 0..10 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(0)).set()),
            10
        );
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            10
        );

        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Allocated);
        assert_eq!(deck.live_slots(), 1);
        assert_eq!(deck.slot_count(), 2, "going off air does not free the slot");

        for _ in 0..10 {
            frame(&gpu, &mut deck, &present, 3);
        }
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(0)).set()),
            40,
            "the on-air slot did not step normally while the other was off air"
        );
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            40,
            "an off-air slot did not keep the room's tempo: nothing is calling \
         `prepare` on it, so its cell is a still rather than a preview"
        );

        // What the mix shows while it is away is what the remaining slot shows.
        let off_air = readback(&gpu, present.hdr_texture());
        let solo_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut solo = deck_of(&gpu, &[SEED_A]);
        for _ in 0..10 {
            frame(&gpu, &mut solo, &solo_present, 1);
        }
        for _ in 0..10 {
            frame(&gpu, &mut solo, &solo_present, 3);
        }
        assert_eq!(
            off_air,
            readback(&gpu, solo_present.hdr_texture()),
            "an Allocated slot was still reaching the mix"
        );

        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Live);
        for _ in 0..5 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(1)).set()),
            45,
            "the returning slot did not come back where the room is"
        );
        assert_eq!(
            steps_taken(deck.slot(karakuri_engine::DeckSlot(0)).set()),
            45
        );
        assert_eq!(deck.live_slots(), 2);
    }
}
