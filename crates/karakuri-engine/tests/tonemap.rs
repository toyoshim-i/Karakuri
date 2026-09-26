//! Integration tests for tone mapping operators in the present pass.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::{Gpu, Points, Present, TonemapOp, VideoSource};

    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 64;

    /// Renders points through the HDR target and reads back the sRGB presentation output.
    fn capture(gpu: &Gpu, present: &Present, points: &mut Points) -> Vec<u8> {
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tonemap test target"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());

        let bytes_per_row = WIDTH * 4;
        assert_eq!(bytes_per_row % 256, 0, "readback rows must stay aligned");
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tonemap test readback"),
            size: u64::from(bytes_per_row * HEIGHT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        points.render(&mut encoder, present.hdr_view(), 1);
        present.draw(&mut encoder, &view, (WIDTH, HEIGHT));
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
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

    /// Creates a high-exposure points source that blows out channels past 1.0 to test tone compression.
    fn hot_points(gpu: &Gpu) -> Points {
        let mut points = Points::new(&gpu.device, 4096, 19274);
        points.resize(WIDTH, HEIGHT);
        points.params.exposure = 6.0;
        points.params.point_scale = 10.0;
        points.prepare(&gpu.queue, 1);
        points
    }

    fn brightest_channel(pixels: &[u8]) -> u8 {
        pixels.iter().copied().max().unwrap_or(0)
    }

    fn fully_saturated_texels(pixels: &[u8]) -> usize {
        pixels
            .chunks_exact(4)
            .filter(|p| p[0] == 255 && p[1] == 255 && p[2] == 255)
            .count()
    }

    /// Verifies that the default tonemap operator initialized by Present is ACES at unit exposure.
    #[test]
    fn the_constructor_default_is_aces_at_unit_exposure() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            WIDTH,
            HEIGHT,
        );
        let mut points = hot_points(&gpu);

        let default_pixels = capture(&gpu, &present, &mut points);
        present.set_tonemap(&gpu.queue, TonemapOp::Aces, 1.0, 1.0);
        let explicit_pixels = capture(&gpu, &present, &mut points);
        assert_eq!(
            default_pixels, explicit_pixels,
            "the constructor default must equal an explicit ACES at exposure 1.0"
        );

        // And it is not Clamp, which is what the default used to be: a test that
        // only compared the default against itself would pass whatever it was.
        present.set_tonemap(&gpu.queue, TonemapOp::Clamp, 1.0, 1.0);
        let clamped = capture(&gpu, &present, &mut points);
        assert_ne!(
            default_pixels, clamped,
            "the default is still Clamp, so nothing is being tone mapped"
        );

        // The material drives channels past 1.0 somewhere, which is the condition
        // that makes the choice of operator matter at all.
        assert_eq!(brightest_channel(&clamped), 255);
    }

    #[test]
    fn switching_operators_on_one_present_changes_the_output() {
        // The whole point of a uniform-driven operator: one `Present`, one
        // pipeline, never rebuilt, and four visibly different results.
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            WIDTH,
            HEIGHT,
        );
        let mut points = hot_points(&gpu);

        let mut render_with = |op, exposure, white_point| {
            present.set_tonemap(&gpu.queue, op, exposure, white_point);
            capture(&gpu, &present, &mut points)
        };
        let clamp = render_with(TonemapOp::Clamp, 1.0, 1.0);
        let reinhard = render_with(TonemapOp::Reinhard, 1.0, 4.0);
        let aces = render_with(TonemapOp::Aces, 1.0, 1.0);
        let agx = render_with(TonemapOp::AgX, 1.0, 1.0);

        assert_ne!(
            clamp, reinhard,
            "Reinhard must differ from the Clamp baseline"
        );
        assert_ne!(clamp, aces, "ACES must differ from the Clamp baseline");
        assert_ne!(clamp, agx, "AgX must differ from the Clamp baseline");
        assert_ne!(
            reinhard, aces,
            "Reinhard and ACES must not coincide on saturated input"
        );
        assert_ne!(
            aces, agx,
            "ACES and AgX must not coincide on saturated input"
        );
    }

    /// Verifies that continuous tone operators retain highlight variation where Clamp clips.
    #[test]
    fn a_real_tonemapper_recovers_detail_that_clamp_destroys() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            WIDTH,
            HEIGHT,
        );
        let mut points = hot_points(&gpu);

        present.set_tonemap(&gpu.queue, TonemapOp::Clamp, 1.0, 1.0);
        let clamp_flat = fully_saturated_texels(&capture(&gpu, &present, &mut points));
        assert!(
            clamp_flat > 0,
            "the probe must actually blow out somewhere under Clamp"
        );

        present.set_tonemap(&gpu.queue, TonemapOp::Aces, 1.0, 1.0);
        assert!(
            fully_saturated_texels(&capture(&gpu, &present, &mut points)) < clamp_flat,
            "ACES should compress rather than clip the blown-out core"
        );

        present.set_tonemap(&gpu.queue, TonemapOp::AgX, 1.0, 1.0);
        assert!(
            fully_saturated_texels(&capture(&gpu, &present, &mut points)) < clamp_flat,
            "AgX should compress rather than clip the blown-out core"
        );
    }

    #[test]
    fn exposure_is_the_operators_own_control_not_a_recompile() {
        // `set_tonemap`'s `exposure` scales the input before the curve, same as
        // any other tone-mapping exposure control — distinct from `Points`'s own
        // `params.exposure`, which stands in for a procedure's `param exposure`.
        // Turning it down on a fixed HDR source should darken the ACES result.
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            WIDTH,
            HEIGHT,
        );
        let mut points = hot_points(&gpu);

        present.set_tonemap(&gpu.queue, TonemapOp::Aces, 1.0, 1.0);
        let bright = capture(&gpu, &present, &mut points);
        present.set_tonemap(&gpu.queue, TonemapOp::Aces, 0.1, 1.0);
        let dim = capture(&gpu, &present, &mut points);

        let sum = |pixels: &[u8]| pixels.iter().map(|&b| u64::from(b)).sum::<u64>();
        assert!(
            sum(&dim) < sum(&bright),
            "lowering exposure must darken the tonemapped result"
        );
    }
}
