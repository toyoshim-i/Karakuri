//! The master chain: three fixed passes between the mix's write and the tone
//! map.
//!
//! Uses `Points` — the hand-written vertical slice — for the same reason
//! `tests/tonemap.rs` does: it produces strongly saturated additive content
//! well past 1.0, which is the only material bloom has anything to do and the
//! material a feedback trail is actually played on.
//!
//! **Every readback here is the linear HDR target and not the surface**, so
//! nothing in these numbers has been through a tone mapper or an sRGB encode.
//! The chain runs upstream of both, and a test that read the encoded frame
//! would be asserting about the tone mapper as much as about the chain.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::{Chain, Cut, Gpu, Points, Present, VideoSource};

    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 64;
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    /// One frame: the source into wherever the mix writes, the chain, and the
    /// linear HDR target read back.
    ///
    /// **`mix_target` and not `hdr_view`**, which is the whole seam this file
    /// is about: with a chain running the two are different targets, and a
    /// test that rendered into `hdr_view` would be feeding the chain nothing
    /// and reading its own source back.
    fn frame(gpu: &Gpu, present: &Present, points: &mut Points) -> Vec<u8> {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        points.render(&mut encoder, present.mix_target(), 1);
        present.draw_chain(&mut encoder);

        let bytes_per_row = WIDTH * 8;
        assert_eq!(bytes_per_row % 256, 0, "readback rows must stay aligned");
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("master test readback"),
            size: u64::from(bytes_per_row * HEIGHT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            present.hdr_texture().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
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

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let pixels = slice.get_mapped_range().expect("map").to_vec();
        readback.unmap();
        pixels
    }

    /// `tests/tonemap.rs`'s `hot_points`: an exposure high enough that the core
    /// blows every channel past 1.0, which is what bloom's knee is at.
    fn hot_points(gpu: &Gpu) -> Points {
        let mut points = Points::new(&gpu.device, 4096, 19274);
        points.resize(WIDTH, HEIGHT);
        points.params.exposure = 6.0;
        points.params.point_scale = 10.0;
        points.prepare(&gpu.queue, 1);
        points
    }

    fn present(gpu: &Gpu) -> Present {
        Present::new(&gpu.device, FORMAT, WIDTH, HEIGHT)
    }

    /// `tests/deck.rs`'s decoder, and it is a copy for that file's own reason:
    /// an `Rgba16Float` readback is bytes, and the workspace has no `f16` type
    /// in it.
    fn f16(bits: u16) -> f32 {
        let sign = f32::from_bits(u32::from(bits & 0x8000) << 16);
        let exponent = (bits >> 10) & 0x1f;
        let mantissa = bits & 0x03ff;
        let magnitude = match exponent {
            0 => f32::from(mantissa) * 2.0f32.powi(-24),
            0x1f if mantissa == 0 => f32::INFINITY,
            0x1f => f32::NAN,
            e => (1.0 + f32::from(mantissa) / 1024.0) * 2.0f32.powi(i32::from(e) - 15),
        };
        if sign.is_sign_negative() {
            -magnitude
        } else {
            magnitude
        }
    }

    /// The total light in a frame, summed over every colour channel. A blunt
    /// instrument on purpose: bloom and feedback both *add* light, and this is
    /// the one number that says so whatever else moved.
    fn light(pixels: &[u8]) -> f64 {
        pixels
            .chunks_exact(8)
            .map(|texel| {
                texel[..6]
                    .chunks_exact(2)
                    .map(|c| f64::from(f16(u16::from_le_bytes([c[0], c[1]]))))
                    .sum::<f64>()
            })
            .sum()
    }

    /// **The default look is what it was before this chain existed**, bit for
    /// bit, and it is the claim the whole design rests on: an amount of zero
    /// records no pass, so there is nothing in the way rather than something in
    /// the way that does nothing.
    ///
    /// Three ways of having no chain are compared: one never told about a
    /// chain, one told a chain of zeros, and one turned up and put back. The
    /// third is the one that could fail on its own — it has allocated,
    /// recorded and retained, and the picture still has to be the same
    /// picture.
    #[test]
    fn a_chain_at_zero_is_the_frame_with_no_chain() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = hot_points(&gpu);

        let untouched = frame(&gpu, &present(&gpu), &mut points);

        let mut zeroed = present(&gpu);
        zeroed.set_chain(&gpu.queue, Chain::default());
        assert_eq!(
            frame(&gpu, &zeroed, &mut points),
            untouched,
            "a chain explicitly set to zero drew a different frame from no chain at all"
        );

        let mut returned = present(&gpu);
        returned.set_chain(
            &gpu.queue,
            Chain {
                feedback: 0.5,
                cut: Cut::Exit,
                bloom: 0.5,
                rgb_shift: 0.5,
            },
        );
        frame(&gpu, &returned, &mut points);
        frame(&gpu, &returned, &mut points);
        returned.set_chain(&gpu.queue, Chain::default());
        assert_eq!(
            frame(&gpu, &returned, &mut points),
            untouched,
            "a chain turned up and put back to zero did not put the frame back"
        );
    }

    /// **Bloom at an amount changes the frame, and the change is more light.**
    ///
    /// The knee is 1.0 — the top of the range the sRGB encode is honest about
    /// — so what blooms is what is already over it, and what the pass does is
    /// spread that into the texels beside it. So the total goes *up*: nothing
    /// here is a redistribution.
    #[test]
    fn bloom_at_an_amount_adds_light_and_zero_adds_none() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = hot_points(&gpu);
        let mut present = present(&gpu);

        present.set_chain(&gpu.queue, Chain::default());
        let none = frame(&gpu, &present, &mut points);

        present.set_chain(
            &gpu.queue,
            Chain {
                bloom: 0.6,
                ..Chain::default()
            },
        );
        let bloomed = frame(&gpu, &present, &mut points);

        assert_ne!(
            bloomed, none,
            "bloom at 0.6 drew the same frame as bloom at 0"
        );
        assert!(
            light(&bloomed) > light(&none) * 1.001,
            "bloom added {} where the frame had {}",
            light(&bloomed) - light(&none),
            light(&none)
        );
    }

    /// **RGB shift at an amount changes the frame**, and at zero it does not —
    /// which is the pass not running rather than a displacement of nothing.
    #[test]
    fn rgb_shift_at_an_amount_changes_the_frame() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = hot_points(&gpu);
        let mut present = present(&gpu);

        present.set_chain(&gpu.queue, Chain::default());
        let straight = frame(&gpu, &present, &mut points);

        present.set_chain(
            &gpu.queue,
            Chain {
                rgb_shift: 1.0,
                ..Chain::default()
            },
        );
        assert_ne!(
            frame(&gpu, &present, &mut points),
            straight,
            "the channels were pulled 2% of the frame apart and nothing moved"
        );
    }

    /// **The first frame of a feedback run reads a black history.**
    ///
    /// Which is what makes a run and its replay agree on frame one: nothing
    /// has been retained yet, and a freshly allocated target reads as zero. If
    /// it did not, this frame would carry whatever the driver left in that
    /// memory and no two runs would agree.
    #[test]
    fn the_first_frame_of_a_feedback_run_reads_a_black_history() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = hot_points(&gpu);

        let plain = frame(&gpu, &present(&gpu), &mut points);

        let mut fed = present(&gpu);
        fed.set_chain(
            &gpu.queue,
            Chain {
                feedback: Chain::FEEDBACK_MAX,
                cut: Cut::Mix,
                ..Chain::default()
            },
        );
        assert_eq!(
            frame(&gpu, &fed, &mut points),
            plain,
            "the first frame of a feedback run added something to itself"
        );
    }

    /// **Feedback accumulates, and the two cuts accumulate differently.**
    ///
    /// `Cut::Mix` reads the frame as the mix wrote it, so what comes back has
    /// been through nothing; `Cut::Exit` reads this chain's own output, so
    /// what comes back has already been bloomed and shifted and is bloomed and
    /// shifted again. The maintainer's decision was both, selectable, and this
    /// is the frame that says why it had to be a parameter rather than a
    /// choice taken here.
    #[test]
    fn the_two_cuts_are_two_pictures() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = hot_points(&gpu);

        let run = |cut: Cut| {
            let mut present = present(&gpu);
            present.set_chain(
                &gpu.queue,
                Chain {
                    feedback: 0.8,
                    cut,
                    bloom: 0.8,
                    rgb_shift: 0.5,
                },
            );
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
        // And the trail is a trail: with a history in it, a later frame is not
        // the first one.
        let mut present = present(&gpu);
        present.set_chain(
            &gpu.queue,
            Chain {
                feedback: 0.8,
                cut: Cut::Mix,
                ..Chain::default()
            },
        );
        let first = frame(&gpu, &present, &mut points);
        let second = frame(&gpu, &present, &mut points);
        assert_ne!(first, second, "nothing was retained between two frames");
    }

    /// **A feedback run is bit-exact across two runs**, which is the whole of
    /// what makes the retained frame part of the state a replay reproduces
    /// rather than a thing the picture picked up along the way
    /// (`docs/principles/0092-…`).
    ///
    /// Both cuts, because they retain from different targets and only one of
    /// them is downstream of the passes.
    #[test]
    fn a_feedback_run_is_bit_exact_across_two_runs() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |cut: Cut| {
            let mut present = present(&gpu);
            present.set_chain(
                &gpu.queue,
                Chain {
                    feedback: 0.7,
                    cut,
                    bloom: 0.4,
                    rgb_shift: 0.3,
                },
            );
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
