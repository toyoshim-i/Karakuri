use super::common::*;

mod gpu {
    use super::*;

    /// Encodes canvas output through the tone mapper and sRGB present pass into a target.
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

    /// Evaluates deck rendering across test steps under specified master output and exposure.
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

    /// Verifies that master output scales RGB channels while leaving coverage alpha bit-for-bit unchanged.
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

    /// Verifies separation of master output (mix write) and tonemap exposure (present read).
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

    /// Verifies that an allocated slot continues simulation off-air to remain synchronized (ADR-0269).
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
