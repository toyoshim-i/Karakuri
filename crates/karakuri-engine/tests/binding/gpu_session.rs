use super::common::*;

mod gpu {
    use super::*;

    /// One oscillator per deck, not one per Set: two Live slots carrying the same
    /// binding write the same value, on the same frame.
    #[test]
    fn every_live_slot_reads_the_same_session_phase() {
        let gpu = Gpu::headless().expect("no GPU");
        let sets: Vec<Set> = (0..2)
            .map(|_| {
                let mut set = build(&gpu);
                assert_eq!(
                    set.set_param("radius", 2.0),
                    1,
                    "the param must be declared for the write to mean anything"
                );
                assert!(set
                    .bind(Binding::new(
                        Kind::L1,
                        "radius",
                        "beat",
                        Curve::Pow2,
                        [1.0, 5.0]
                    ))
                    .attached());
                set
            })
            .collect();
        let (mut deck, present) = deck_of(&gpu, sets, 4);

        for _ in 0..17 {
            frame(&gpu, &mut deck, &present, 1);
            assert_eq!(
                value_of(&deck, 0, "radius"),
                value_of(&deck, 1, "radius"),
                "two slots disagreed about the session's phase"
            );
        }

        // Off-air slots continue reading phase and stepping alongside live slots.
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Allocated);
        let off_air_at = value_of(&deck, 1, "radius");
        let mut moved = false;
        for _ in 0..7 {
            frame(&gpu, &mut deck, &present, 1);
            assert_eq!(
                value_of(&deck, 0, "radius"),
                value_of(&deck, 1, "radius"),
                "an off-air slot read a different phase than the Live one beside it"
            );
            moved |= value_of(&deck, 1, "radius") != off_air_at;
        }
        assert!(
            moved,
            "the bound value never moved over seven frames, so the comparison above \
             holds against a slot that stopped resolving as much as against one that \
             did not"
        );
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Live);
        frame(&gpu, &mut deck, &present, 1);
        assert_eq!(
            value_of(&deck, 0, "radius"),
            value_of(&deck, 1, "radius"),
            "a slot brought back on air did not rejoin the session's phase"
        );
    }
    /// Verifies that the session clock advances by the same clamped simulation steps as the slots.
    #[test]
    fn the_session_clock_advances_by_the_same_clamped_steps_the_slots_do() {
        let gpu = Gpu::headless().expect("no GPU");

        let bound = |gpu: &Gpu| {
            let mut set = build(gpu);
            assert_eq!(
                set.set_param("radius", 2.0),
                1,
                "the param must be declared for the write to mean anything"
            );
            assert!(set
                .bind(Binding::new(
                    Kind::L1,
                    "radius",
                    "beat",
                    Curve::Lin,
                    [1.0, 5.0]
                ))
                .attached());
            set
        };

        // Over the cap and at it: the frames the two decks run are different
        // numbers, and the phase they reach has to be the same one.
        let over = karakuri_engine::set::MAX_STEPS + 3;
        let (mut fast, present_a) = deck_of(&gpu, vec![bound(&gpu)], 8);
        let (mut capped, present_b) = deck_of(&gpu, vec![bound(&gpu)], 8);
        for _ in 0..12 {
            frame(&gpu, &mut fast, &present_a, over);
            frame(
                &gpu,
                &mut capped,
                &present_b,
                karakuri_engine::set::MAX_STEPS,
            );
            assert_eq!(
                value_of(&fast, 0, "radius"),
                value_of(&capped, 0, "radius"),
                "a frame of {over} steps moved the session clock further than the {} \
             steps its slots took",
                karakuri_engine::set::MAX_STEPS
            );
        }

        // A zero-step frame renders and measures, and moves nothing.
        let held = value_of(&capped, 0, "radius");
        for _ in 0..5 {
            frame(&gpu, &mut capped, &present_b, 0);
            assert_eq!(
                value_of(&capped, 0, "radius"),
                held,
                "a zero-step frame advanced the session clock"
            );
        }
    }
    /// Verifies that parameters sharing identical names across multiple nodes maintain distinct values per node.
    #[test]
    fn a_param_name_two_nodes_declare_is_two_values_one_per_node() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(L1),
            &compile(L4_CLASHING),
            CAPACITY,
            SEED,
        )
        .expect("two nodes may each declare a `radius`");

        let mut declared: Vec<(Kind, f32)> = set
            .params()
            .filter(|(_, _, name, _)| *name == "radius")
            .map(|(layer, _, _, value)| (layer, value))
            .collect();
        declared.sort_by_key(|(layer, _)| format!("{layer:?}"));
        assert_eq!(
            declared,
            // Camera declares a radius as well, but unaddressed names do not modify it.
            vec![(Kind::L1, 2.5), (Kind::L3, 8.0), (Kind::L4, 7.5)],
            "the two declarations of `radius` did not survive as two values"
        );

        // A name with no address is every node that declares it — one knob, both
        // layers — which is what a `--param` and a `param` record ask for.
        assert_eq!(
            set.set_param("radius", 4.0),
            2,
            "a bare name must move both — and only both: the camera declares a `radius` and a \
             bare name is not a way to reach it"
        );
        assert_eq!(
            set.set_param("point_scale", 0.07),
            1,
            "only the L4 declares it"
        );
        assert_eq!(set.set_param("nothing_declares_this", 1.0), 0);

        // Back to two distinct values by rebuilding, since a bare name cannot set
        // them apart — which is the addressed write the record vocabulary still
        // owes, and deliberately not this commit's business.
        let mut set = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(L1),
            &compile(L4_CLASHING),
            CAPACITY,
            SEED,
        )
        .expect("two nodes may each declare a `radius`");
        for layer in [Kind::L1, Kind::L4] {
            assert!(
                set.bind(Binding::new(
                    layer,
                    "radius",
                    "nothing_measures_this",
                    Curve::Lin,
                    [0.0, 100.0]
                ))
                .attached(),
                "`radius` is declared on {layer:?}"
            );
        }

        let (mut deck, present) = deck_of(&gpu, vec![set], 1);
        frame(&gpu, &mut deck, &present, 1);

        let resolved = |layer: Kind| -> f32 {
            deck.slot(karakuri_engine::DeckSlot(0))
                .set()
                .bindings()
                .iter()
                .find(|b| b.layer == layer)
                .expect("both layers are bound")
                .value()
        };
        assert_eq!(
            (resolved(Kind::L1), resolved(Kind::L4)),
            (2.5, 7.5),
            "a binding blended from the other node's declaration of `radius`"
        );
    }
    /// `bind` refuses a param the layer does not declare, rather than attaching a
    /// binding that writes nowhere. Nothing here asks whether a *provider* exists
    /// — this is about the artifact's own declaration.
    #[test]
    fn binding_a_param_the_layer_does_not_declare_is_refused() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = build(&gpu);

        assert!(!set
            .bind(Binding::new(
                Kind::L1,
                "no_such_param",
                "beat",
                Curve::Lin,
                [0.0, 1.0]
            ))
            .attached());
        // `hue` is L4's, so an L1 binding to it must not attach.
        assert!(!set
            .bind(Binding::new(
                Kind::L1,
                "hue",
                "beat",
                Curve::Lin,
                [0.0, 1.0]
            ))
            .attached());
        assert!(set
            .bind(Binding::new(
                Kind::L4,
                "hue",
                "beat",
                Curve::Lin,
                [0.0, 1.0]
            ))
            .attached());
        assert_eq!(set.bindings().len(), 1);

        // A second binding on the same param replaces the first rather than
        // stacking behind it, so the write never depends on attachment order.
        assert!(set
            .bind(Binding::new(
                Kind::L4,
                "hue",
                "bar",
                Curve::Sqrt,
                [0.0, 1.0]
            ))
            .attached());
        assert_eq!(set.bindings().len(), 1);
        assert_eq!(set.bindings()[0].signal, "bar");
    }
    /// The measured path reaches a real uniform, not only the arithmetic: the same
    /// `energy` binding on a built Set writes a different value once a frame is
    /// measured, through `Set::prepare` and the deck, with nothing else changed.
    #[test]
    fn a_measured_signal_reaches_the_uniform_a_param_override_would_write() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = build(&gpu);
        assert!(set
            .bind(Binding::new(
                Kind::L1,
                "radius",
                "energy",
                Curve::Lin,
                [0.5, 8.0]
            ))
            .attached());
        let (mut deck, present) = deck_of(&gpu, vec![set], u64::from(SEED));

        frame(&gpu, &mut deck, &present, 1);
        let invented = value_of(&deck, 0, "radius");

        let mut signals = *deck.signals();
        signals.set_audio(Some(AudioFrame {
            energy: 1.0,
            onset: 0.0,
            bands: [0.0; 8],
            band_count: 8,
            confidence: 1.0,
        }));
        deck.set_signals(signals);
        frame(&gpu, &mut deck, &present, 1);
        let measured = value_of(&deck, 0, "radius");

        assert_eq!(
            measured, 8.0,
            "a measured energy of 1.0 should write the top of the range"
        );
        assert!(
            (measured - invented).abs() > 1.0,
            "the measured frame changed nothing: {invented} then {measured}"
        );
    }

    /// Verifies that live slot attachments persist and continue driving parameters across rebuilds.
    #[test]
    fn an_attachment_made_live_is_still_driving_after_the_next_rebuild() {
        use karakuri_engine::swap::{Event, Request, RequestNames};

        /// Frames stepped from the instant the rebuild landed. The incoming
        /// Set is cold, so this is the same simulation in both runs.
        const AFTER: usize = 4;
        /// Nothing here is a budget test, and a slot stopped for cost draws no
        /// frame at all (ADR-0316) — which would be this test failing for a
        /// reason it is not about.
        const NO_BUDGET: f32 = 10_000.0;

        let gpu = Gpu::headless().expect("no GPU");

        // Rebuilding the identical material currently executing in the slot.
        // save of an untouched `.kir` produces: nothing about the picture
        // changes across this rebuild except what the attachment does or does
        // not go on driving.
        let rebuild = || Request {
            id: 1,
            l1s: vec![(compile(L1), CAPACITY)],
            l2s: Vec::new(),
            l3s: Vec::new(),
            fields: Vec::new(),
            l4s: vec![compile(L4)],
            names: RequestNames::default(),
            edges: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            seed_salt: SEED,
            salts: Vec::new(),
            camera: karakuri_engine::camera::Orbit::default(),
            params: Vec::new(),
            published: Vec::new(),
            // Verifies parameter override remains active across set rebuild.
            // binding, because a pair carries none — so this is what every
            // rebuild of the panel's own slots hands the engine.
            bindings: Vec::new(),
            authorities: Vec::new(),
            label: String::from("a save of the same .kir"),
        };

        let measured = |energy: f32| AudioFrame {
            energy,
            onset: 0.0,
            bands: [0.0; 8],
            band_count: 8,
            confidence: 1.0,
        };

        // One run: attach `energy` to `radius` on the live slot, let a rebuild
        // land, and step `AFTER` frames of the room at `energy`.
        let rendered = |energy: f32| -> (f32, Vec<u16>) {
            let present =
                Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, WIDTH, HEIGHT);
            let (tx, rx) = std::sync::mpsc::channel::<Request>();
            let swap = HotSwap::new(
                &gpu.device,
                &gpu.queue,
                build(&gpu),
                NO_BUDGET,
                Box::new(rx),
            );
            let mut deck = Deck::new(&gpu.device, vec![swap], WIDTH, HEIGHT);
            deck.set_signals(Signals::new(BPM, u64::from(SEED)));

            // Tracing live parameter write path through to GPU uniform.
            assert!(
                deck.bind(
                    karakuri_engine::DeckSlot(0),
                    Binding::new(Kind::L1, "radius", "energy", Curve::Lin, [0.5, 8.0],)
                )
                .attached(),
                "`radius` is a declared L1 param"
            );
            let mut signals = *deck.signals();
            signals.set_audio(Some(measured(energy)));
            deck.set_signals(signals);
            frame(&gpu, &mut deck, &present, 1);
            assert_eq!(
                value_of(&deck, 0, "radius"),
                0.5 + 7.5 * energy,
                "the attachment is not driving before any rebuild, so this test would \
                 pass over the question it is about"
            );

            tx.send(rebuild()).expect("the build worker is listening");
            let mut landed = false;
            for _ in 0..600 {
                frame(&gpu, &mut deck, &present, 1);
                if deck
                    .events(karakuri_engine::DeckSlot(0))
                    .any(|e| matches!(e, Event::Swapped { .. }))
                {
                    landed = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            assert!(landed, "no rebuild landed, so nothing was asserted");
            assert_eq!(
                deck.slot(karakuri_engine::DeckSlot(0))
                    .set()
                    .bindings()
                    .len(),
                1,
                "the rebuild left the slot with no attachment at all: what an operator \
                 attached to `radius` was walked back by a save that states no binding"
            );

            for _ in 0..AFTER {
                let mut signals = *deck.signals();
                signals.set_audio(Some(measured(energy)));
                deck.set_signals(signals);
                frame(&gpu, &mut deck, &present, 1);
            }
            (
                value_of(&deck, 0, "radius"),
                readback(&gpu, present.hdr_texture()),
            )
        };

        let (quiet, quiet_frame) = rendered(0.0);
        let (loud, loud_frame) = rendered(1.0);

        assert_eq!(
            (quiet, loud),
            (0.5, 8.0),
            "the attachment did not survive the rebuild: `radius` is at its own value \
             rather than at what the room is driving it to"
        );
        assert_ne!(
            quiet_frame, loud_frame,
            "a silent room and a loud one drew the same picture after a rebuild, so the \
             attachment stopped reaching the shader"
        );
    }
}
