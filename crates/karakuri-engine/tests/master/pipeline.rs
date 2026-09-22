use super::common::*;

mod gpu {
    use super::*;

    /// **The default look is what it was before this chain existed**, bit for
    /// bit, and it is the claim the whole design rests on: an empty chain is no
    /// pass at all rather than a pass that does nothing.
    ///
    /// Three ways of having no chain are compared: one never told about a
    /// chain, one told an empty one, and one that held three slots and had them
    /// taken out again. The third is the one that could fail on its own — it
    /// has allocated, recorded and retained, and the picture still has to be
    /// the same picture.
    #[test]
    fn an_empty_chain_is_the_frame_with_no_chain() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = hot_points(&gpu);

        let untouched = frame(&gpu, &present(&gpu), &mut points);

        let mut emptied = present(&gpu);
        drop(emptied.set_chain(&gpu.device, &gpu.queue, Chain::default()));
        assert_eq!(emptied.chain_targets(), 0, "an empty chain took a target");
        assert_eq!(
            frame(&gpu, &emptied, &mut points),
            untouched,
            "a chain explicitly set to empty drew a different frame from no chain at all"
        );

        let mut returned = present(&gpu);
        let full = Chain::new(vec![
            slot(&gpu, &returned, FEEDBACK, Some(Cut::Exit), 0.5),
            slot(&gpu, &returned, BLOOM, None, 0.5),
            slot(&gpu, &returned, RGB_SHIFT, None, 0.5),
        ]);
        drop(returned.set_chain(&gpu.device, &gpu.queue, full));
        frame(&gpu, &returned, &mut points);
        frame(&gpu, &returned, &mut points);
        drop(returned.set_chain(&gpu.device, &gpu.queue, Chain::default()));
        assert_eq!(
            frame(&gpu, &returned, &mut points),
            untouched,
            "a chain filled and emptied again did not put the frame back"
        );
    }

    /// **`examples/rgb_shift.kir` is `master.wgsl`'s `fs_rgb_shift`**, bit for
    /// bit, on a frame that is not square — so the conversion from a fraction
    /// of the frame's *height* into a step along x is part of what is compared.
    #[test]
    fn the_shipped_rgb_shift_is_the_hand_written_pass_bit_for_bit() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let hand = Hand::new(&gpu, &present);

        for amount in [0.25f32, 1.0] {
            let mix = mixes(&gpu, 1).pop().expect("one frame");
            let out = target(&gpu, "hand rgb shift");
            hand.set(&gpu, 0.0, 0.0, amount);
            let src = mix.create_view(&Default::default());
            let bind = hand.bind(&gpu, &src, &src);
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            hand.pass(
                &mut encoder,
                &hand.rgb_shift,
                &bind,
                &out.create_view(&Default::default()),
            );
            gpu.queue.submit([encoder.finish()]);
            let wanted = readback(&gpu, &out);

            drop(present.set_chain(
                &gpu.device,
                &gpu.queue,
                Chain::new(vec![slot(&gpu, &present, RGB_SHIFT, None, amount)]),
            ));
            let mut points = hot_points(&gpu);
            let got = frame(&gpu, &present, &mut points);
            assert_eq!(
                got, wanted,
                "the shipped rgb shift at {amount} is not the pass it replaced"
            );
        }
    }

    /// **`examples/feedback.kir` is `master.wgsl`'s `fs_feedback`**, bit for
    /// bit, with a history that is a picture rather than a black frame.
    ///
    /// The history is the previous frame's mix, which is what `Cut::Mix` means,
    /// so the comparison is run on the *second* frame: the chain reads what it
    /// retained from the first, and the hand-written pass is handed the same
    /// frame from a copy of it.
    #[test]
    fn the_shipped_feedback_is_the_hand_written_pass_bit_for_bit() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let hand = Hand::new(&gpu, &present);
        const AMOUNT: f32 = 0.7;

        // Two frames of the mix: the first is what the chain retains, the
        // second is what it then adds to.
        let mix = mixes(&gpu, 2);
        let out = target(&gpu, "hand feedback");
        hand.set(&gpu, AMOUNT, 0.0, 0.0);
        let bind = hand.bind(
            &gpu,
            &mix[1].create_view(&Default::default()),
            &mix[0].create_view(&Default::default()),
        );
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        hand.pass(
            &mut encoder,
            &hand.feedback,
            &bind,
            &out.create_view(&Default::default()),
        );
        gpu.queue.submit([encoder.finish()]);
        let wanted = readback(&gpu, &out);

        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), AMOUNT)]),
        ));
        let mut points = hot_points(&gpu);
        frame(&gpu, &present, &mut points);
        let got = frame(&gpu, &present, &mut points);
        assert_eq!(
            got, wanted,
            "the shipped feedback is not the pass it replaced"
        );
    }

    /// **`examples/bloom.kir` is not the pass it replaces, and this is where
    /// the difference is measured.**
    ///
    /// The hand-written bloom is two passes with an intermediate target — a
    /// bright-and-blur-x, then a blur-y-and-add that needs **both** that buffer
    /// and the frame it was taken from. A chain slot's output replaces the
    /// frame, so a second slot could not name the earlier value: letting it is
    /// a graph and the chain is a list. So the shipped procedure is one 9x9
    /// kernel where the pair was two 9-tap passes — 81 fetches against 19 — at
    /// the same radius, the same weights and the same knee (ADR-0340's
    /// *Bloom as two chain slots*).
    ///
    /// **What differs is the filtering and not the arithmetic.** The pair's
    /// second half samples an already-blurred buffer with a bilinear tap; the
    /// single pass taps the frame itself at eighty-one offsets. So this asserts
    /// a *tolerance* and prints the number, rather than asserting an equality
    /// that is not true — and the tolerance is relative to the light the frame
    /// holds, because these are unbounded HDR values.
    #[test]
    fn the_shipped_bloom_is_the_hand_written_pair_within_a_stated_tolerance() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let hand = Hand::new(&gpu, &present);
        const AMOUNT: f32 = 0.6;

        let mix = mixes(&gpu, 1).pop().expect("one frame");
        let blur = target(&gpu, "hand bloom blur");
        let out = target(&gpu, "hand bloom");
        hand.set(&gpu, 0.0, AMOUNT, 0.0);
        let src = mix.create_view(&Default::default());
        let blur_view = blur.create_view(&Default::default());
        let self_bind = hand.bind(&gpu, &src, &src);
        let blend_bind = hand.bind(&gpu, &src, &blur_view);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        hand.pass(&mut encoder, &hand.bloom_bright, &self_bind, &blur_view);
        hand.pass(
            &mut encoder,
            &hand.bloom_blend,
            &blend_bind,
            &out.create_view(&Default::default()),
        );
        gpu.queue.submit([encoder.finish()]);
        let two_pass = channels(&readback(&gpu, &out));

        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![slot(&gpu, &present, BLOOM, None, AMOUNT)]),
        ));
        let mut points = hot_points(&gpu);
        let one_pass = channels(&frame(&gpu, &present, &mut points));

        let plain = channels(&readback(&gpu, &mix));
        let peak = plain.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1.0);
        let (worst, mean) = two_pass
            .iter()
            .zip(&one_pass)
            .fold((0.0f32, 0.0f64), |(worst, sum), (a, b)| {
                ((a - b).abs().max(worst), sum + f64::from((a - b).abs()))
            });
        let mean = mean / two_pass.len() as f64;
        println!(
            "bloom: one 9x9 pass against the separable pair at amount {AMOUNT} — \
             worst channel {worst:.4}, mean {mean:.6}, against a frame peaking at {peak:.3}"
        );
        // **The same picture, and the tolerance says how nearly.** 2% of the
        // frame's peak: enough to admit a bilinear tap's difference and far
        // too tight to admit a different radius, a different knee or a
        // different normalisation, each of which moves the blurred light by
        // tens of percent.
        assert!(
            worst < 0.02 * peak,
            "the shipped bloom differs from the pair it replaces by {worst} \
             against a peak of {peak} — that is not a filter's difference"
        );
        // And it is bloom rather than a copy: the light went up.
        assert!(
            light(&frame(&gpu, &present, &mut hot_points(&gpu)))
                > light(&readback(&gpu, &mix)) * 1.001
        );
    }
}
