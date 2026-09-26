use super::common::*;

mod gpu {
    use super::*;

    /// Verifies retention textures are allocated only when slots declare cuts (at most one per cut kind).
    #[test]
    fn a_cut_is_held_only_where_a_slot_asked_for_it() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);

        // Nothing retains: two targets, both of them the chain's own.
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &present, BLOOM, None, 0.5),
                slot(&gpu, &present, RGB_SHIFT, None, 0.5),
            ]),
        ));
        assert_eq!(present.chain_retained(), Vec::new());
        assert_eq!(present.chain_targets(), 2, "the entry and one intermediate");

        // One slot retaining: one cut held.
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &present, FEEDBACK, Some(Cut::Exit), 0.5),
                slot(&gpu, &present, RGB_SHIFT, None, 0.5),
            ]),
        ));
        assert_eq!(present.chain_retained(), vec![Cut::Exit]);
        assert_eq!(present.chain_targets(), 3);

        // Multiple slots reading the same cut share a single target buffer.
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.5),
                slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.25),
            ]),
        ));
        assert_eq!(present.chain_retained(), vec![Cut::Mix]);
        assert_eq!(present.chain_targets(), 3);

        // Two slots naming *different* cuts: both, and that is the ceiling.
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.5),
                slot(&gpu, &present, BLOOM, None, 0.5),
                slot(&gpu, &present, FEEDBACK, Some(Cut::Exit), 0.25),
            ]),
        ));
        assert_eq!(present.chain_retained(), vec![Cut::Mix, Cut::Exit]);
        assert_eq!(
            present.chain_targets(),
            5,
            "the entry, two intermediates and the two cuts"
        );
    }

    /// Verifies `Cut::Mix` and `Cut::Exit` yield visually distinct composited results.
    #[test]
    fn the_two_cuts_are_two_pictures() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |cut: Cut| {
            let mut present = present(&gpu);
            let chain = Chain::new(vec![
                slot(&gpu, &present, FEEDBACK, Some(cut), 0.8),
                slot(&gpu, &present, BLOOM, None, 0.8),
                slot(&gpu, &present, RGB_SHIFT, None, 0.5),
            ]);
            drop(present.set_chain(&gpu.device, &gpu.queue, chain));
            let mut points = hot_points(&gpu);
            let mut last = Vec::new();
            for _ in 0..4 {
                last = frame(&gpu, &present, &mut points);
            }
            last
        };

        let from_mix = run(Cut::Mix);
        let from_exit = run(Cut::Exit);
        assert_ne!(
            from_mix, from_exit,
            "the two cuts drew the same frame, so nothing chooses between them"
        );
        // The exit compounds — what is read back already contains the trail —
        // so four frames of it hold more light than four frames of a trail
        // that never feeds itself.
        assert!(
            light(&from_exit) > light(&from_mix),
            "the compounding cut held {} against the mix cut's {}",
            light(&from_exit),
            light(&from_mix)
        );
    }

    /// Verifies slots reading `Cut::Mix` correctly receive the previous frame's mix regardless of position in chain.
    #[test]
    fn the_mix_cut_is_the_previous_frame_wherever_the_slot_sits() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &present, RGB_SHIFT, None, 0.4),
                slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.8),
            ]),
        ));
        let mut points = hot_points(&gpu);
        let first = frame(&gpu, &present, &mut points);
        let second = frame(&gpu, &present, &mut points);
        assert_ne!(
            first, second,
            "a feedback slot second in the list retained nothing"
        );

        // And the same chain with nothing retained draws a different frame,
        // which is what says the trail is the retention rather than the shift.
        let mut plain = crate::common::present(&gpu);
        drop(plain.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![slot(&gpu, &plain, RGB_SHIFT, None, 0.4)]),
        ));
        let mut points = hot_points(&gpu);
        frame(&gpu, &plain, &mut points);
        assert_ne!(
            frame(&gpu, &plain, &mut points),
            second,
            "the second frame was the same with and without a feedback slot"
        );
    }

    /// Verifies that the initial frame of a retaining run reads empty (black) history textures.
    #[test]
    fn the_first_frame_of_a_retaining_run_reads_a_black_history() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = hot_points(&gpu);

        let plain = frame(&gpu, &present(&gpu), &mut points);

        let mut fed = present(&gpu);
        drop(fed.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![slot(&gpu, &fed, FEEDBACK, Some(Cut::Mix), 0.95)]),
        ));
        assert_eq!(
            frame(&gpu, &fed, &mut hot_points(&gpu)),
            plain,
            "the first frame of a feedback run added something to itself"
        );
    }

    /// Verifies bit-exact determinism across identical runs using retention textures.
    #[test]
    fn a_retaining_run_is_bit_exact_across_two_runs() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |cut: Cut| {
            let mut present = present(&gpu);
            let chain = Chain::new(vec![
                slot(&gpu, &present, FEEDBACK, Some(cut), 0.7),
                slot(&gpu, &present, BLOOM, None, 0.4),
                slot(&gpu, &present, RGB_SHIFT, None, 0.3),
            ]);
            drop(present.set_chain(&gpu.device, &gpu.queue, chain));
            let mut points = hot_points(&gpu);
            let mut frames = Vec::new();
            for _ in 0..5 {
                frames.push(frame(&gpu, &present, &mut points));
            }
            frames
        };

        for cut in Cut::ALL {
            assert_eq!(
                run(cut),
                run(cut),
                "two runs of the same records diverged with the {} cut",
                cut.name()
            );
        }
    }
}
