//! The guard on the vertical slice.
//!
//! "Get a triangle on screen, then never break it" needs something that fails
//! when it breaks. A window cannot do that in CI or in a subagent, but an
//! offscreen `Rgba16Float` target and a readback can, and they check more than a
//! screenshot would: that the HDR format survives the round trip, that additive
//! blending accumulates, and that simulation time advances from steps rather
//! than from a clock.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::{Gpu, Points, Present, VideoSource};

    const WIDTH: u32 = 256;
    const HEIGHT: u32 = 256;

    /// Render one frame offscreen and read back the HDR target as raw f16 bits.
    fn render(gpu: &Gpu, points: &mut Points, steps: u8) -> Vec<u16> {
        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, WIDTH, HEIGHT);
        points.resize(WIDTH, HEIGHT);
        points.prepare(&gpu.queue, steps);

        let bytes_per_row = WIDTH * 8; // Rgba16Float
        assert_eq!(bytes_per_row % 256, 0, "readback rows must stay aligned");

        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(bytes_per_row * HEIGHT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        points.render(&mut encoder, present.hdr_view(), steps);
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
        let data = slice.get_mapped_range().expect("map");
        let out = data
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        drop(data);
        readback.unmap();
        out
    }

    fn lit_texels(pixels: &[u16]) -> usize {
        // Non-zero in any of R, G, B. f16 zero is all-zero bits, so this needs no
        // decoding.
        pixels
            .chunks_exact(4)
            .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
            .count()
    }

    #[test]
    fn the_slice_puts_elements_on_screen() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = Points::new(&gpu.device, 4096, 19274);
        let pixels = render(&gpu, &mut points, 1);

        assert_eq!(pixels.len() as u32, WIDTH * HEIGHT * 4);
        let lit = lit_texels(&pixels);
        assert!(lit > 100, "expected a visible shell, got {lit} lit texels");
        assert!(
            lit < (WIDTH * HEIGHT) as usize,
            "the whole frame is lit, which means the camera or the blend is wrong"
        );
    }

    #[test]
    fn more_elements_light_more_of_the_frame() {
        // Additive blending with no depth write: adding elements can only add
        // light. If this ever fails, the blend state stopped being additive.
        let gpu = Gpu::headless().expect("no GPU available");
        let sparse = lit_texels(&render(&gpu, &mut Points::new(&gpu.device, 256, 19274), 1));
        let dense = lit_texels(&render(&gpu, &mut Points::new(&gpu.device, 8192, 19274), 1));
        assert!(dense > sparse, "sparse {sparse}, dense {dense}");
    }

    #[test]
    fn the_same_seed_and_the_same_steps_reproduce_the_same_frame() {
        // The determinism invariant, at the only place it can currently be
        // observed: same records plus same seeds, bit for bit.
        let gpu = Gpu::headless().expect("no GPU available");
        let a = render(&gpu, &mut Points::new(&gpu.device, 2048, 19274), 3);
        let b = render(&gpu, &mut Points::new(&gpu.device, 2048, 19274), 3);
        assert_eq!(a, b, "identical inputs produced different frames");
    }

    #[test]
    fn a_different_seed_produces_a_different_frame() {
        // Re-seeding changes randomness. If this fails, the seed salt is not
        // reaching the hash builtins and `{"t":"seed"}` records do nothing.
        let gpu = Gpu::headless().expect("no GPU available");
        let a = render(&gpu, &mut Points::new(&gpu.device, 2048, 19274), 1);
        let b = render(&gpu, &mut Points::new(&gpu.device, 2048, 88888), 1);
        assert_ne!(a, b);
    }

    #[test]
    fn time_advances_by_steps_not_by_a_clock() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = Points::new(&gpu.device, 64, 1);
        assert_eq!(points.time(), 0.0);

        points.prepare(&gpu.queue, 1);
        let after_one = points.time();
        points.prepare(&gpu.queue, 3);
        let after_four = points.time();

        assert!(
            (after_four - after_one * 4.0).abs() < 1e-6,
            "steps must be linear"
        );
        // Two frames of one step and one frame of two steps must land in the same
        // place, which is what makes a tick record replayable.
        let mut other = Points::new(&gpu.device, 64, 1);
        other.prepare(&gpu.queue, 2);
        other.prepare(&gpu.queue, 2);
        assert!((other.time() - after_four).abs() < 1e-6);
    }
}
