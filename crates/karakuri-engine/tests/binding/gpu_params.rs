use super::common::*;

mod gpu {
    use super::*;

    /// **An attachment made through the deck rides, and taking it back leaves
    /// the parameter at its own value** — the two writers *Attach a signal to
    /// a parameter* and *Take a parameter back* land on, both reaching the
    /// live `Set` by `Deck::write_param`'s road
    /// (`docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`).
    ///
    /// **Asserted at the texel**, on `a_bound_param_reaches_the_shader_by_the_same_path_a_param_override_takes`'s
    /// terms and for its reason: a binding table that is right and a slot
    /// still writing what it was built with are indistinguishable anywhere
    /// else. Three decks, each stepped the same number of frames from cold, so
    /// the pictures are comparable — a bound one, a taken-back one, and one
    /// that was never bound at all.
    ///
    /// **The take-back is asserted as *the manual picture* and not as an empty
    /// table.** That is the whole of the decision: there is no suspended state
    /// for a binding to be in, so what *not driving this parameter* means is
    /// the parameter's own value, exactly — the same arithmetic the blend
    /// already does at confidence 0.0 (P-0084) rather than a case added for
    /// it. A `retain` that left the last written value behind would pass an
    /// assertion about the table and fail this one.
    #[test]
    fn an_attachment_through_the_deck_rides_and_a_take_back_returns_the_manual_value() {
        let gpu = Gpu::headless().expect("no GPU");

        // A steady spawn rate so there is material on screen, and a constant
        // range so the value written does not depend on the phase the frame
        // was taken at — the neighbouring test's two guards.
        const RATE: f32 = 20_000.0;
        const MANUAL: f32 = 2.5;
        const DRIVEN: f32 = 4.5;
        let material = || {
            let mut set = build(&gpu);
            assert_eq!(
                set.set_param("spawn_rate", RATE),
                1,
                "the param must be declared for the write to mean anything"
            );
            assert_eq!(
                set.set_param("radius", MANUAL),
                1,
                "the param must be declared for the write to mean anything"
            );
            set
        };
        let driven = || Binding::new(Kind::L1, "radius", "beat", Curve::Lin, [DRIVEN, DRIVEN]);
        let rendered = |attach: &dyn Fn(&mut Deck)| {
            let (mut deck, present) = deck_of(&gpu, vec![material()], 6);
            attach(&mut deck);
            for _ in 0..4 {
                frame(&gpu, &mut deck, &present, 1);
            }
            readback(&gpu, present.hdr_texture())
        };

        let untouched = rendered(&|_deck| {});
        let attached = rendered(&|deck| {
            assert!(
                deck.bind(karakuri_engine::DeckSlot(0), driven()).attached(),
                "`radius` is a declared L1 param"
            );
        });
        let taken_back = rendered(&|deck| {
            assert!(deck.bind(karakuri_engine::DeckSlot(0), driven()).attached());
            assert!(
                deck.unbind(karakuri_engine::DeckSlot(0), Kind::L1, None, "radius"),
                "there was an attachment at that address to remove"
            );
        });

        assert_ne!(
            attached, untouched,
            "an attachment made through the deck changed nothing on screen"
        );
        assert_eq!(
            taken_back, untouched,
            "a parameter taken back is not at its own value: the attachment is still \
             driving it, or the value it last wrote was left behind"
        );

        // **And a take-back on a knob nobody is holding says so rather than
        // failing**, which is the caller's cue and not an error: a rebuild may
        // no longer declare the name.
        let (mut deck, _present) = deck_of(&gpu, vec![material()], 6);
        assert!(!deck.unbind(karakuri_engine::DeckSlot(0), Kind::L1, None, "radius"));
        assert!(deck.bind(karakuri_engine::DeckSlot(0), driven()).attached());
        assert!(deck.unbind(karakuri_engine::DeckSlot(0), Kind::L1, None, "radius"));
        assert!(
            !deck.unbind(karakuri_engine::DeckSlot(0), Kind::L1, None, "radius"),
            "a second take-back claimed to remove something"
        );
        // **And it is addressed**: the attachment above is the layer's, so a
        // take-back naming one node of it is a different address and removes
        // nothing.
        assert!(deck.bind(karakuri_engine::DeckSlot(0), driven()).attached());
        assert!(
            !deck.unbind(karakuri_engine::DeckSlot(0), Kind::L1, Some(0), "radius"),
            "an addressed take-back removed the layer's attachment"
        );
    }

    /// **An authority set through the deck is the level the node is under**,
    /// which is the writer ADR-0211 said the engine owed.
    ///
    /// It changes nothing on screen and is not meant to: nothing writes a
    /// parameter on an agent's behalf, so what is asserted is that the level
    /// lands at the node it names and that a node this Set has not got is said
    /// rather than silently taken.
    #[test]
    fn an_authority_set_through_the_deck_lands_on_the_node_it_names() {
        let gpu = Gpu::headless().expect("no GPU");
        let (mut deck, _present) = deck_of(&gpu, vec![build(&gpu)], 1);

        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(0))
                .set()
                .authority(Kind::L1, 0),
            Some(karakuri_engine::set::Authority::Manual),
            "a node nobody has spoken for is manual"
        );
        assert!(deck.set_authority(
            karakuri_engine::DeckSlot(0),
            Kind::L1,
            0,
            karakuri_engine::set::Authority::Automatic
        ));
        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(0))
                .set()
                .authority(Kind::L1, 0),
            Some(karakuri_engine::set::Authority::Automatic),
            "the level did not land on the node it named"
        );
        // The L4 beside it is untouched, which is what *per node* means.
        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(0))
                .set()
                .authority(Kind::L4, 0),
            Some(karakuri_engine::set::Authority::Manual),
            "setting one node's authority moved another node's"
        );
        assert!(
            !deck.set_authority(
                karakuri_engine::DeckSlot(0),
                Kind::L1,
                7,
                karakuri_engine::set::Authority::Manual
            ),
            "a node this Set has not got was taken rather than reported"
        );
    }

    /// A binding reaches the parameter, through a real frame, and moves it with
    /// the session oscillator's phase rather than merely over time.
    #[test]
    fn a_beat_binding_moves_a_param_in_time_with_the_deck_oscillator() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = build(&gpu);
        assert_eq!(
            set.set_param("radius", 2.5),
            1,
            "the param must be declared for the write to mean anything"
        );
        assert!(
            set.bind(Binding::new(
                Kind::L1,
                "radius",
                "beat",
                Curve::Lin,
                [1.0, 5.0]
            ))
            .attached(),
            "`radius` is a declared L1 param"
        );
        let (mut deck, present) = deck_of(&gpu, vec![set], 1);

        let mut values = Vec::new();
        for _ in 0..(FRAMES_PER_BEAT * 3) {
            frame(&gpu, &mut deck, &present, 1);
            values.push(value_of(&deck, 0, "radius"));
        }

        // It moves at all, and across most of the range it was given.
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            min < 1.5 && max > 4.5,
            "a bound param spanned only {min}..{max} of [1, 5]"
        );

        // And it moves at the *oscillator's* period. A param drifting for any
        // other reason would fail this while passing the span check above.
        for i in 0..FRAMES_PER_BEAT {
            let a = values[i];
            let b = values[i + FRAMES_PER_BEAT];
            assert!(
                (a - b).abs() < 1e-3,
                "frame {i} and one beat later differ: {a} vs {b}"
            );
        }

        // It is the deck's own phase, not a second clock: the value equals what
        // the deck's `beat` signal says at the instant of the frame just rendered.
        let beat = deck.signals().sample("beat");
        assert_eq!(beat.confidence, 1.0, "beat comes off the local oscillator");
        let expected = blend(2.5, 1.0 + 4.0 * beat.value, 1.0);
        assert_eq!(*values.last().expect("frames were rendered"), expected);
    }
    /// **What a binding writes reaches the shader**, and reaches it by the same
    /// path a `--param` override takes.
    ///
    /// Every other test here reads the value a binding resolved to, which is one
    /// step short of the claim: a `prepare` that resolved bindings correctly and
    /// then packed the manual values into the uniform would pass all of them.
    /// So this one compares rendered pixels, in both directions and in both
    /// uniform buffers — L1's `radius` and L4's `hue`:
    ///
    /// - a Set with a param bound to a constant renders **bit for bit** the same
    ///   as a Set with that param simply set to the same number, which is what
    ///   "the same path a `--param` takes" means;
    /// - and both differ from the same Set left at its manual value, which is
    ///   what stops the first comparison from passing on two identical blanks.
    #[test]
    fn a_bound_param_reaches_the_shader_by_the_same_path_a_param_override_takes() {
        let gpu = Gpu::headless().expect("no GPU");

        // A constant range and a certain signal, so the written value does not
        // depend on the phase the comparison happened to be taken at.
        // A steady spawn rate, so there is material on screen to compare at all.
        const RATE: f32 = 20_000.0;
        let bound_to = |key: &str, layer, value: f32, manual: f32| {
            let mut set = build(&gpu);
            assert_eq!(
                set.set_param("spawn_rate", RATE),
                1,
                "the param must be declared for the write to mean anything"
            );
            assert_eq!(
                set.set_param(key, manual),
                1,
                "the param must be declared for the write to mean anything"
            );
            assert!(set
                .bind(Binding::new(layer, key, "beat", Curve::Lin, [value, value]))
                .attached());
            set
        };
        let set_to = |key: &str, value: f32| {
            let mut set = build(&gpu);
            assert_eq!(
                set.set_param("spawn_rate", RATE),
                1,
                "the param must be declared for the write to mean anything"
            );
            assert_eq!(
                set.set_param(key, value),
                1,
                "the param must be declared for the write to mean anything"
            );
            set
        };
        let render_of = |set: Set| {
            let (mut deck, present) = deck_of(&gpu, vec![set], 6);
            for _ in 0..4 {
                frame(&gpu, &mut deck, &present, 1);
            }
            readback(&gpu, present.hdr_texture())
        };

        for (key, layer, bound, manual) in
            [("radius", Kind::L1, 5.5, 2.5), ("hue", Kind::L4, 0.05, 0.6)]
        {
            let by_binding = render_of(bound_to(key, layer, bound, manual));
            let by_override = render_of(set_to(key, bound));
            let unbound = render_of(set_to(key, manual));

            assert_ne!(
                by_binding, unbound,
                "`{key}` bound to {bound} rendered the same as `{key}` left at {manual}: \
             the binding never reached the uniform"
            );
            assert_eq!(
                by_binding, by_override,
                "`{key}` bound to {bound} and `{key}` set to {bound} rendered differently: \
             a binding is supposed to be the same uniform write an override is"
            );
        }
    }
    /// `spawn_rate` has a **second** consumer — the spawn accumulator, which is
    /// not a uniform — and a binding has to reach that one too. A `spawn_rate`
    /// whose uniform took the bound value while its accumulator took the manual
    /// one would be the same param meaning two things in one frame, and it is the
    /// exact case `docs/ir-spec.md`'s Spawn timing rests on.
    #[test]
    fn a_binding_reaches_the_spawn_accumulator_and_not_only_the_uniform() {
        let gpu = Gpu::headless().expect("no GPU");

        // Nothing spawns at all without the binding: `spawn_rate` is zero by hand.
        let mut unbound = build(&gpu);
        assert_eq!(
            unbound.set_param("spawn_rate", 0.0),
            1,
            "the param must be declared for the write to mean anything"
        );
        let (mut deck, present) = deck_of(&gpu, vec![unbound], 5);
        for _ in 0..30 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(0))
                .set()
                .live_count(&gpu.device, &gpu.queue),
            0,
            "a manual spawn_rate of zero spawned something"
        );

        // The same Set, the same manual zero, plus a certain binding pinned to a
        // constant rate: the range's two ends are equal, so the count depends on
        // the binding being read rather than on the phase it was read at.
        let mut bound = build(&gpu);
        assert_eq!(
            bound.set_param("spawn_rate", 0.0),
            1,
            "the param must be declared for the write to mean anything"
        );
        assert!(bound
            .bind(Binding::new(
                Kind::L1,
                "spawn_rate",
                "beat",
                Curve::Lin,
                [3000.0, 3000.0]
            ))
            .attached());
        let (mut deck, present) = deck_of(&gpu, vec![bound], 5);
        for _ in 0..30 {
            frame(&gpu, &mut deck, &present, 1);
        }
        // 3000 per second for half a second, give or take the accumulator's carry.
        let live = deck
            .slot(karakuri_engine::DeckSlot(0))
            .set()
            .live_count(&gpu.device, &gpu.queue);
        assert!(
            (1400..=1600).contains(&live),
            "the spawn accumulator produced {live} elements, not the bound rate's ~1500"
        );
    }
    /// A param that is bound **and** given a manual value. The manual value is the
    /// base of the blend and is never overwritten, so the answer does not depend
    /// on which of the two happened last.
    #[test]
    fn a_manual_value_is_kept_and_blended_from_rather_than_overwritten() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = build(&gpu);
        // What `--param radius=7.0` does.
        assert_eq!(
            set.set_param("radius", 7.0),
            1,
            "the param must be declared for the write to mean anything"
        );
        // An *invented* signal, so the manual value keeps 90% of the weight and
        // its survival is observable in the written value rather than only in the
        // map it came from.
        assert!(set
            .bind(Binding::new(
                Kind::L1,
                "radius",
                "energy",
                Curve::Lin,
                [0.0, 1.0]
            ))
            .attached());
        let (mut deck, present) = deck_of(&gpu, vec![set], 2);

        for _ in 0..10 {
            frame(&gpu, &mut deck, &present, 1);
        }

        let manual = deck
            .slot(karakuri_engine::DeckSlot(0))
            .set()
            .param("radius")
            .expect("declared");
        assert_eq!(manual, 7.0, "the binding overwrote the manual value");

        let energy = deck.signals().sample("energy");
        assert_eq!(energy.confidence, 0.1, "energy is supposed to be invented");
        assert_eq!(
            value_of(&deck, 0, "radius"),
            blend(7.0, energy.value, energy.confidence)
        );
        // Ten percent of the way from 7.0 towards a value in [0, 1]: still far
        // nearer the manual value than the signal, which is what confidence 0.1
        // has to mean for this not to be theatre.
        assert!(value_of(&deck, 0, "radius") > 6.2);
    }
    /// A signal no provider has ever heard of. No failure, no panic, and the param
    /// stands exactly where it was put — bit for bit, across a real frame.
    #[test]
    fn a_signal_with_no_provider_leaves_the_param_where_it_was_put() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = build(&gpu);
        assert_eq!(
            set.set_param("radius", 3.25),
            1,
            "the param must be declared for the write to mean anything"
        );
        assert!(set
            .bind(Binding::new(
                Kind::L1,
                "radius",
                "mic_level",
                Curve::Pow2,
                [100.0, 200.0]
            ))
            .attached());
        let (mut deck, present) = deck_of(&gpu, vec![set], 3);

        for _ in 0..20 {
            frame(&gpu, &mut deck, &present, 1);
            assert_eq!(
                value_of(&deck, 0, "radius"),
                3.25,
                "a signal with no provider moved a param"
            );
        }
        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(0))
                .set()
                .param("radius")
                .expect("declared"),
            3.25
        );
    }
    /// The same tick sequence and the same seed reproduce every bound value bit
    /// for bit, through a noise binding — so a binding is inside the determinism
    /// invariant rather than beside it.
    #[test]
    fn the_same_ticks_and_seed_reproduce_a_noise_binding_bit_for_bit() {
        let gpu = Gpu::headless().expect("no GPU");
        // Ragged on purpose: the tick sequence, not the frame count, is what has
        // to be reproduced.
        let ticks = [1u8, 2, 1, 1, 3, 1, 2, 4, 1];

        let run = |seed: u64| {
            let mut set = build(&gpu);
            set.bind(
                Binding::new(
                    Kind::L1,
                    "spawn_rate",
                    "noise",
                    Curve::Lin,
                    [4000.0, 16000.0],
                )
                .with_noise(NoiseConfig {
                    kind: NoiseKind::Fbm { octaves: 4 },
                    rate: 0.5,
                    stream: 3,
                }),
            )
            .attached();
            set.bind(Binding::new(
                Kind::L4,
                "hue",
                "bar",
                Curve::Smooth,
                [0.0, 1.0],
            ))
            .attached();
            let (mut deck, present) = deck_of(&gpu, vec![set], seed);
            let mut out = Vec::new();
            for steps in ticks {
                frame(&gpu, &mut deck, &present, steps);
                out.push(value_of(&deck, 0, "spawn_rate").to_bits());
                out.push(value_of(&deck, 0, "hue").to_bits());
            }
            out
        };

        assert_eq!(run(7), run(7), "two identical runs disagreed");
        assert_ne!(run(7), run(8), "changing the seed changed nothing");
    }
}
