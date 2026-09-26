//! Integration tests for L5 composite layering versus direct overdraw.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
#[path = "common/mod.rs"]
mod common;

mod gpu {
    pub(super) use super::common::{compile, f16};
    use karakuri_engine::mix::Input;
    use karakuri_engine::set::Layering;
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::typed::Checked;

    /// A lattice with no motion of its own, so the picture is a function of the
    /// renderers alone.
    const GRID: &str = r#"
proc grid {
  kind     L1
  topology points
  capacity [16, 16] = 16

  emit position

  element {
    let u = hash1(seed);
    let v = hash1(seed + 977u);
    position = vec3(u * 4.0 - 2.0, v * 4.0 - 2.0, 0.0);
  }
}
"#;

    /// One flat colour per renderer, so what each contributes to the mix is a
    /// number a test can name. Additive, which is the mode the two layerings agree
    /// under.
    fn dots(name: &str, rgb: (f32, f32, f32)) -> String {
        let (r, g, b) = rgb;
        format!(
            r#"
proc {name} {{
  kind  L4
  blend additive

  consumes position

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.09375;
  }}

  fragment {{
    color = vec4({r:?}, {g:?}, {b:?}, 1.0);
  }}
}}
"#
        )
    }

    // Non-square dimensions to expose any width/height transposition issues.
    const W: u32 = 96;
    const H: u32 = 64;

    fn build(gpu: &Gpu, l4s: &[String], layering: Layering) -> Set {
        let compiled: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
        let refs: Vec<&Checked> = compiled.iter().collect();
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&compile(GRID), 16)],
            &[],
            &[],
            &[],
            &refs,
            layering,
            5,
            &[],
            karakuri_engine::set::Wiring::default(),
        )
        .expect("one L1 and some L4s");
        set.resize(&gpu.device, W, H);
        // Still and level, so a frame is a frame rather than a moment in a sweep.
        set.aim_camera(karakuri_engine::camera::Orbit {
            radius: 6.0,
            speed: 0.0,
            height: 0.0,
            ..Default::default()
        });
        set
    }

    /// RGBA f32 per texel, after one frame.
    fn frame(gpu: &Gpu, set: &mut Set) -> Vec<f32> {
        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, W, H);
        set.prepare(&gpu.queue, 1, &Signals::default());

        let bytes_per_row = W * 8;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(bytes_per_row * H),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        set.render(&mut encoder, present.hdr_view(), 1);
        encoder.copy_texture_to_buffer(
            present.hdr_texture().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(H),
                },
            },
            wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);
        set.commit();

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("map");
        let out: Vec<f32> = data
            .chunks_exact(2)
            .map(|b| f16(u16::from_le_bytes([b[0], b[1]])))
            .collect();
        drop(data);
        readback.unmap();
        out
    }

    /// Sum of one channel over the frame — how much of a colour reached the mix.
    fn total(px: &[f32], channel: usize) -> f64 {
        px.chunks_exact(4).map(|t| f64::from(t[channel])).sum()
    }

    // ---------------------------------------------------------------------------

    /// Verifies that compositing a single input is bit-exact with direct overdraw.
    #[test]
    fn a_merge_of_one_input_hands_the_material_on_unchanged() {
        let gpu = Gpu::headless().expect("no GPU available");
        let one = [dots("only", (0.4, 0.7, 0.2))];
        let over = frame(&gpu, &mut build(&gpu, &one, Layering::Overdraw));
        let composited = frame(&gpu, &mut build(&gpu, &one, Layering::Composite));

        assert_eq!(over.len(), composited.len());
        let differing = over
            .iter()
            .zip(&composited)
            .filter(|(a, b)| a.to_bits() != b.to_bits())
            .count();
        assert_eq!(
            differing,
            0,
            "{differing} of {} channels differ",
            over.len()
        );
    }

    /// Verifies that multiple additive renderers produce matching visual output under compositing and overdraw.
    #[test]
    fn additive_renderers_composite_to_what_they_overdraw_to() {
        let gpu = Gpu::headless().expect("no GPU available");
        let two = [dots("warm", (0.5, 0.2, 0.0)), dots("cool", (0.0, 0.2, 0.5))];
        let over = frame(&gpu, &mut build(&gpu, &two, Layering::Overdraw));
        let composited = frame(&gpu, &mut build(&gpu, &two, Layering::Composite));

        for channel in 0..3 {
            let (a, b) = (total(&over, channel), total(&composited, channel));
            assert!(a > 1.0, "channel {channel} is empty in the overdrawn frame");
            assert!(
                (a - b).abs() < a * 0.01,
                "channel {channel}: overdraw totals {a} and compositing totals {b}"
            );
        }
    }

    /// Each merge input connects via an independent edge buffer.
    /// being what separates a per-input fader from a fader on the Set.
    #[test]
    fn an_inputs_own_fader_reaches_only_that_input() {
        let gpu = Gpu::headless().expect("no GPU available");
        let two = [dots("warm", (0.5, 0.0, 0.0)), dots("cool", (0.0, 0.0, 0.5))];
        let mut set = build(&gpu, &two, Layering::Composite);
        let full = frame(&gpu, &mut set);
        let (warm, cool) = (total(&full, 0), total(&full, 2));
        assert!(
            warm > 1.0 && cool > 1.0,
            "both renderers must reach the frame first"
        );

        assert!(
            set.set_input(
                1,
                Input {
                    opacity: 0.0,
                    ..Input::unity()
                }
            ),
            "this Set draws with two renderers"
        );
        let muted = frame(&gpu, &mut set);
        assert!(
            total(&muted, 2) < cool * 0.01,
            "the silenced input still reached the mix"
        );
        assert!(
            (total(&muted, 0) - warm).abs() < warm * 0.01,
            "silencing one input moved the other"
        );

        // And half is half: a fader's middle is the middle, which an on/off test
        // cannot see.
        assert!(set.set_input(
            1,
            Input {
                opacity: 0.5,
                ..Input::unity()
            }
        ));
        let half = total(&frame(&gpu, &mut set), 2);
        assert!(
            (half - cool * 0.5).abs() < cool * 0.05,
            "half opacity gave {half} where half of {cool} was due"
        );
    }

    /// Verifies that solo renderer selection isolates the chosen renderer while preserving its contribution level.
    #[test]
    fn selecting_one_renderer_leaves_only_that_renderer_in_the_frame() {
        let gpu = Gpu::headless().expect("no GPU available");
        let two = [dots("warm", (0.5, 0.0, 0.0)), dots("cool", (0.0, 0.0, 0.5))];
        let mut set = build(&gpu, &two, Layering::Composite);
        let full = frame(&gpu, &mut set);
        let (warm, cool) = (total(&full, 0), total(&full, 2));
        assert!(
            warm > 1.0 && cool > 1.0,
            "both renderers must reach the frame first"
        );

        assert!(set.select_renderer(1), "this Set draws with two renderers");
        let only_cool = frame(&gpu, &mut set);
        assert!(
            total(&only_cool, 0) < warm * 0.01,
            "the renderer that was not selected still reached the fold"
        );
        assert!(
            (total(&only_cool, 2) - cool).abs() < cool * 0.01,
            "the selected renderer did not arrive at the level it was already at"
        );

        // And the other way, which is what makes it a selection rather than a
        // one-shot silencing: choosing renderer 0 brings it back and takes 1
        // out.
        assert!(set.select_renderer(0));
        let only_warm = frame(&gpu, &mut set);
        assert!(total(&only_warm, 2) < cool * 0.01);
        assert!((total(&only_warm, 0) - warm).abs() < warm * 0.01);

        // A renderer this Set does not have changes nothing at all — the
        // picture is still the one the last selection asked for, rather than a
        // mix silenced on the way to finding out.
        assert!(!set.select_renderer(2));
        let after = frame(&gpu, &mut set);
        assert!(
            (total(&after, 0) - warm).abs() < warm * 0.01 && total(&after, 2) < cool * 0.01,
            "a refused selection moved the picture"
        );
    }

    /// Verifies that renderer selection on an overdrawing Set is a safe no-op.
    #[test]
    fn selecting_a_renderer_of_an_overdrawing_set_changes_nothing() {
        let gpu = Gpu::headless().expect("no GPU available");
        let two = [dots("warm", (0.5, 0.0, 0.0)), dots("cool", (0.0, 0.0, 0.5))];
        let mut set = build(&gpu, &two, Layering::Overdraw);
        let before = frame(&gpu, &mut set);
        assert!(
            set.select_renderer(1),
            "the edges exist under both layerings"
        );
        let after = frame(&gpu, &mut set);
        for channel in [0, 2] {
            let (a, b) = (total(&before, channel), total(&after, channel));
            assert!(a > 1.0, "channel {channel} is empty before the selection");
            assert!(
                (a - b).abs() < a * 0.01,
                "channel {channel} moved from {a} to {b} on a Set with no L5 to fold"
            );
        }
    }

    /// Merge targets resize dynamically following viewport size changes.
    #[test]
    fn the_merge_targets_follow_a_resize() {
        let gpu = Gpu::headless().expect("no GPU available");
        let two = [dots("warm", (0.5, 0.0, 0.0)), dots("cool", (0.0, 0.0, 0.5))];
        let mut set = build(&gpu, &two, Layering::Composite);
        let small = total(&frame(&gpu, &mut set), 0);
        assert!(small > 1.0, "nothing was drawn at the first size");

        // `frame` reads back at W by H, so the readback still matches; what changes
        // is what the merge allocated between the two.
        set.resize(&gpu.device, W * 2, H * 2);
        set.resize(&gpu.device, W, H);
        let again = total(&frame(&gpu, &mut set), 0);
        assert!(
            (again - small).abs() < small * 0.01,
            "after a resize and back the frame totals {again} where it totalled {small}"
        );
    }

    /// Verifies that compositing rejects more input renderers than the L5 shader supports.
    #[test]
    fn compositing_refuses_more_renderers_than_an_l5_can_fold() {
        let gpu = Gpu::headless().expect("no GPU available");
        let five: Vec<String> = (0..5)
            .map(|i| dots(&format!("r{i}"), (0.1 * i as f32, 0.0, 0.0)))
            .collect();

        let err = {
            let compiled: Vec<Checked> = five.iter().map(|s| compile(s)).collect();
            let refs: Vec<&Checked> = compiled.iter().collect();
            Set::build_many(
                &gpu.device,
                &gpu.queue,
                &[(&compile(GRID), 16)],
                &[],
                &[],
                &[],
                &refs,
                Layering::Composite,
                5,
                &[],
                karakuri_engine::set::Wiring::default(),
            )
            .err()
            .expect("five inputs is one more than an L5 folds")
        };
        let message = err.to_string();
        assert!(message.contains('5') && message.contains('4'), "{message}");
        assert!(
            message.contains("overdraw"),
            "the hint does not offer the layering that has no limit: {message}"
        );

        // And the same five overdraw without complaint.
        let mut set = build(&gpu, &five, Layering::Overdraw);
        let px = frame(&gpu, &mut set);
        assert!(
            total(&px, 0) > 1.0,
            "five renderers drew nothing when overdrawing"
        );
    }
}
