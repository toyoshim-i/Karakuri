use super::common::*;

mod gpu {
    use super::*;

    /// **A retention is allocated only where a slot's answer names one, and at
    /// most two ever** — one per cut, however many slots read them.
    ///
    /// The rule ADR-0317 wrote for one fixed pass, at a list's width: *a cut
    /// that is read has to be held, so holding one nothing reads is the cost
    /// nobody would pay* (P-0091).
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

        // **Two slots reading the same cut read one frame**, so it is still one
        // target.
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

    /// **The two cuts are two pictures**, which is why the cut is the slot's
    /// answer rather than a decision the design took.
    ///
    /// `Cut::Mix` reads the frame as the mix wrote it, so what comes back has
    /// been through nothing; `Cut::Exit` reads the chain's own output, so what
    /// comes back has already been bloomed and shifted and is bloomed and
    /// shifted again.
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

    /// **A slot reading the mix cut reads the previous frame's mix wherever it
    /// sits in the list**, which is the position-independence the entry target
    /// buys.
    ///
    /// `master.rs` used to copy the mix cut *between* two passes, because the
    /// mix's own target was also the second pass's destination. A list cannot
    /// honour that: the slot that reads the cut may be anywhere. So the entry
    /// is held apart from the ping-pong pair and the copy happens at the end,
    /// and this is the frame that says it worked — feedback second in the list
    /// still trails.
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

    /// **The first frame of a run with a retaining slot reads a black
    /// history.**
    ///
    /// Which is what makes a run and its replay agree on frame one: nothing has
    /// been retained yet, and a freshly allocated target reads as zero. If it
    /// did not, this frame would carry whatever the driver left in that memory
    /// and no two runs would agree.
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

    /// **A run with a retention is bit-exact across two runs**, which is the
    /// whole of what makes the retained frame part of the state a replay
    /// reproduces rather than a thing the picture picked up along the way
    /// (`docs/principles/0092-…`).
    ///
    /// Both cuts, because they are held from different targets and only one of
    /// them is downstream of the passes.
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
