use super::common::*;

mod gpu {
    use super::*;

    /// Verifies parameter-only updates modify uniforms in place without reallocation and match full rebuild results.
    #[test]
    fn moving_a_parameter_draws_what_rebuilding_the_list_draws() {
        let gpu = Gpu::headless().expect("no GPU available");
        let shape = |present: &Present| -> Vec<(String, Option<Cut>)> { present.chain_shape() };

        let mut built = present(&gpu);
        drop(built.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &built, BLOOM, None, 0.9),
                slot(&gpu, &built, RGB_SHIFT, None, 0.2),
            ]),
        ));
        let wanted = frame(&gpu, &built, &mut hot_points(&gpu));

        let mut moved = present(&gpu);
        drop(moved.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &moved, BLOOM, None, 0.1),
                slot(&gpu, &moved, RGB_SHIFT, None, 0.7),
            ]),
        ));
        let before = moved.chain_targets();
        assert!(
            moved.set_chain_params(&gpu.queue, &shape(&moved), &[params(0.9), params(0.2)]),
            "the shape that is running was refused as a parameter move"
        );
        assert_eq!(
            moved.chain_targets(),
            before,
            "a parameter move allocated a target"
        );
        assert_eq!(
            frame(&gpu, &moved, &mut hot_points(&gpu)),
            wanted,
            "the cheap path drew a different frame from the built one"
        );

        // Mismatched cut lists are rejected atomically without partial application.
        assert!(
            !moved.set_chain_params(&gpu.queue, &[("elsewhere".into(), None)], &[params(0.5)]),
            "a shape that is not running was taken as a parameter move"
        );
    }

    /// Verifies that chain fragment cost calculates as the sum of per-slot operations per fragment.
    #[test]
    fn a_chains_price_is_the_sum_of_its_slots() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        assert_eq!(
            present.chain_ops_per_fragment(),
            0,
            "an empty chain is free"
        );

        let one = Chain::new(vec![slot(&gpu, &present, RGB_SHIFT, None, 0.5)]);
        let alone = one.ops_per_fragment();
        drop(present.set_chain(&gpu.device, &gpu.queue, one));
        assert_eq!(present.chain_ops_per_fragment(), alone);

        let three = Chain::new(vec![
            slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.5),
            slot(&gpu, &present, BLOOM, None, 0.5),
            slot(&gpu, &present, RGB_SHIFT, None, 0.5),
        ]);
        let sum: u32 = three.slots().iter().map(|s| s.ops_per_fragment()).sum();
        assert_eq!(three.ops_per_fragment(), sum);
        println!(
            "chain cost: feedback {}, bloom {}, rgb shift {} — {sum} ops per fragment together",
            three.slots()[0].ops_per_fragment(),
            three.slots()[1].ops_per_fragment(),
            three.slots()[2].ops_per_fragment(),
        );
        drop(present.set_chain(&gpu.device, &gpu.queue, three));
        assert_eq!(present.chain_ops_per_fragment(), sum);
    }

    /// Verifies chain slot procedures receive frame uniforms `(t, beats, dt)` correctly.
    #[test]
    fn a_chain_slot_reads_the_clock_the_frame_hands_it() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let mut points = hot_points(&gpu);
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![slot(&gpu, &present, CLOCK, None, 0.0)]),
        ));

        let read = |present: &Present, points: &mut Points, clock: Clock| {
            present.set_chain_clock(&gpu.queue, clock);
            assert_eq!(
                present.chain_clock(),
                clock,
                "the chain did not keep the clock it was written"
            );
            let pixels = frame(&gpu, present, points);
            let texel = channels(&pixels[..8]);
            (texel[0], texel[1], texel[2])
        };

        let first = Clock {
            t: 2.5,
            beats: 5.25,
            dt: 0.25,
            seed_salt: 0,
        };
        let (t, beats, dt) = read(&present, &mut points, first);
        assert!(
            (t - first.t).abs() < 1e-3 && (beats - first.beats).abs() < 1e-3,
            "the slot read t {t} and beats {beats} against {first:?}"
        );
        assert!((dt - first.dt).abs() < 1e-4, "the slot read dt {dt}");

        let second = Clock {
            t: 7.0,
            beats: 14.0,
            dt: 0.125,
            seed_salt: 0,
        };
        let (t, beats, dt) = read(&present, &mut points, second);
        assert!(
            (t - second.t).abs() < 1e-3 && (beats - second.beats).abs() < 1e-3,
            "the slot read t {t} and beats {beats} against {second:?}"
        );
        assert!((dt - second.dt).abs() < 1e-4, "the slot read dt {dt}");
    }

    /// A parameter write leaves the running clock where it is.
    /// `set_chain_params` repacks a whole uniform block, and the clock it
    /// repacks is the one that is running.
    #[test]
    fn moving_a_parameter_leaves_the_clock_where_it_is() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let mut points = hot_points(&gpu);
        let built = Chain::new(vec![slot(&gpu, &present, CLOCK, None, 0.0)]);
        let shape: Vec<(String, Option<Cut>)> = built
            .slots()
            .iter()
            .map(|s| (s.proc().to_string(), s.cut()))
            .collect();
        drop(present.set_chain(&gpu.device, &gpu.queue, built));

        let clock = Clock {
            t: 3.0,
            beats: 6.0,
            dt: 0.25,
            seed_salt: 0,
        };
        present.set_chain_clock(&gpu.queue, clock);
        assert!(
            present.set_chain_params(&gpu.queue, &shape, &[params(1.0)]),
            "the shape handed back is the shape that is running"
        );
        let pixels = frame(&gpu, &present, &mut points);
        let texel = channels(&pixels[..8]);
        assert!(
            (texel[0] - clock.t).abs() < 1e-3,
            "a parameter write moved the clock to {}",
            texel[0]
        );
    }

    /// Verifies reference execution cost estimates for the three shipped master chain procedures.
    #[test]
    fn the_three_shipped_procedures_price_the_chains_rate() {
        use karakuri_engine::estimate::{
            chain_ms, CHAIN_REFERENCE_MS, CHAIN_REFERENCE_OPS, CHAIN_REFERENCE_SIZE,
        };

        let gpu = Gpu::headless().expect("no GPU available");
        let present = present(&gpu);
        let three = Chain::new(vec![
            slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.5),
            slot(&gpu, &present, BLOOM, None, 0.5),
            slot(&gpu, &present, RGB_SHIFT, None, 0.5),
        ]);
        assert_eq!(
            three.ops_per_fragment(),
            CHAIN_REFERENCE_OPS,
            "the shipped three no longer cost what the chain's rate was calibrated on"
        );

        let ms = chain_ms(three.ops_per_fragment(), CHAIN_REFERENCE_SIZE);
        assert!(
            (ms - CHAIN_REFERENCE_MS).abs() < 0.01,
            "the shipped three price at {ms:.3} ms against a measured {CHAIN_REFERENCE_MS:.2}"
        );

        assert_eq!(
            chain_ms(0, CHAIN_REFERENCE_SIZE),
            0.0,
            "an empty chain is free"
        );
        let doubled = chain_ms(three.ops_per_fragment(), (2560, 720));
        assert!(
            (doubled - 2.0 * ms).abs() < 1e-3,
            "twice the area priced at {doubled:.3} ms against {:.3}",
            2.0 * ms
        );
    }

    /// Refusal classifications for invalid post-processing chain configurations.
    /// Texture slot the chain has no `edge` to bind.
    #[test]
    fn a_procedure_that_cannot_be_a_chain_slot_is_refused_with_the_reason() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = present(&gpu);
        let build = |source: &str, cut: Option<Cut>| {
            Slot::build(
                &gpu.device,
                present.chain_layout(),
                "test:refused",
                &checked(source),
                cut,
                params(0.5),
            )
            .err()
            .map(|e| e.to_string())
        };

        assert!(build(FEEDBACK, Some(Cut::Mix)).is_none());
        let bare = build(FEEDBACK, None).expect("`retains` with no cut is refused");
        assert!(bare.contains("mix") && bare.contains("exit"), "{bare}");
        let extra = build(BLOOM, Some(Cut::Exit)).expect("a cut with no `retains` is refused");
        assert!(extra.contains("retains"), "{extra}");
    }

    /// Unified image pass and retention manager abstractions.
    #[test]
    fn unified_image_pass_and_retention_abstractions_record() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = present(&gpu);
        let s = slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.5);

        assert_eq!(s.pass().name(), "feedback");
        assert_eq!(s.name(), "feedback");

        let mut retention = RetentionManager::new(&gpu.device);
        assert_eq!(retention.count(), 0);
        let mix_src = target(&gpu, "mix src").create_view(&Default::default());
        let exit_src = target(&gpu, "exit src").create_view(&Default::default());
        retention.allocate(
            &gpu.device,
            WIDTH,
            HEIGHT,
            &[Cut::Mix, Cut::Exit],
            Some(&mix_src),
            Some(&exit_src),
        );
        assert_eq!(retention.count(), 2);
        assert!(retention.is_held(Cut::Mix));
        assert!(retention.is_held(Cut::Exit));
        assert!(retention.held(Cut::Mix).is_some());

        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        retention.record(&mut encoder);
        gpu.queue.submit([encoder.finish()]);

        // Verify bound pass implements RenderPassNode
        let dst = target(&gpu, "dst target").create_view(&Default::default());
        let held = target(&gpu, "held target").create_view(&Default::default());
        let sampler = gpu.device.create_sampler(&Default::default());
        let bg = s.pass().bind(
            &gpu.device,
            present.chain_layout(),
            &mix_src,
            &held,
            &sampler,
            Some("test bg"),
        );
        let bound = s.bound(&bg);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        bound.record(&mut encoder, &dst);
        gpu.queue.submit([encoder.finish()]);
    }

    /// Verifies that chains built via background workers produce identical frames and install at frame boundaries.
    #[test]
    fn a_chain_built_on_the_worker_draws_what_building_it_here_draws() {
        let gpu = Gpu::headless().expect("no GPU available");

        let mut here = present(&gpu);
        drop(here.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &here, FEEDBACK, Some(Cut::Mix), 0.7),
                slot(&gpu, &here, BLOOM, None, 0.4),
                slot(&gpu, &here, RGB_SHIFT, None, 0.3),
            ]),
        ));
        let wanted = frame(&gpu, &here, &mut hot_points(&gpu));

        let mut there = present(&gpu);
        let mut swap = ChainSwap::new(&gpu.device, &gpu.queue);
        let id = swap.request(
            &there,
            vec![
                asked(FEEDBACK, Some(Cut::Mix), 0.7),
                asked(BLOOM, None, 0.4),
                asked(RGB_SHIFT, None, 0.3),
            ],
        );
        assert_eq!(
            there.chain_len(),
            0,
            "asking for a chain installed one on the calling thread"
        );

        until("the worker never finished the build", || swap.built() == 1);
        assert_eq!(
            there.chain_len(),
            0,
            "a finished build installed itself without a frame boundary"
        );
        let meanwhile = frame(&gpu, &there, &mut hot_points(&gpu));
        assert_ne!(
            meanwhile, wanted,
            "the frame before the boundary was already the built chain's"
        );

        swap.begin_frame(&mut there, &gpu.device, &gpu.queue);
        assert_eq!(swap.installs(), 1, "the frame boundary installed nothing");
        assert_eq!(there.chain_len(), 3);
        assert_eq!(
            there.chain_shape(),
            vec![
                (address(FEEDBACK), Some(Cut::Mix)),
                (address(BLOOM), None),
                (address(RGB_SHIFT), None),
            ]
        );
        let ids: Vec<u64> = swap
            .events()
            .map(|event| match event {
                ChainEvent::Installed { id, slots } => {
                    assert_eq!(slots, 3);
                    id
                }
                other => panic!("{other}"),
            })
            .collect();
        assert_eq!(ids, vec![id]);

        assert_eq!(
            frame(&gpu, &there, &mut hot_points(&gpu)),
            wanted,
            "the chain built on the worker drew a different frame from the one built here"
        );
    }

    /// Verifies that when multiple chain builds are pending at a frame boundary, only the newest installs.
    #[test]
    fn the_newest_of_two_pending_builds_wins() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let mut swap = ChainSwap::new(&gpu.device, &gpu.queue);

        let stale = swap.request(&present, vec![asked(BLOOM, None, 0.4)]);
        let newest = swap.request(
            &present,
            vec![asked(RGB_SHIFT, None, 0.3), asked(BLOOM, None, 0.9)],
        );
        assert_ne!(stale, newest);
        until("the worker never finished both builds", || {
            swap.built() == 2
        });

        swap.begin_frame(&mut present, &gpu.device, &gpu.queue);
        assert_eq!(
            swap.installs(),
            1,
            "one frame boundary installed more than one chain"
        );
        assert_eq!(
            present.chain_shape(),
            vec![(address(RGB_SHIFT), None), (address(BLOOM), None)],
            "the boundary installed the build the newer one superseded"
        );
        let ids: Vec<u64> = swap
            .events()
            .map(|event| match event {
                ChainEvent::Installed { id, .. } => id,
                other => panic!("{other}"),
            })
            .collect();
        assert_eq!(
            ids,
            vec![newest],
            "the superseded build was announced as well as retired"
        );

        // Frame boundary without pending requests installs nothing.
        swap.begin_frame(&mut present, &gpu.device, &gpu.queue);
        assert_eq!(swap.installs(), 1);
        assert!(swap.pending_events().is_empty());
    }
}
