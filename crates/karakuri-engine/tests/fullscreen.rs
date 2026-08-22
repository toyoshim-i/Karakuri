//! `topology fullscreen`, asserted in pixels and in what does *not* run.
//!
//! Two kinds of claim here and the second is the one worth having. The first is
//! that a procedure with no `vertex` block covers the frame and can see where it
//! is on it — ordinary rendering claims. The second is that the paired L1's
//! simulation **does not happen**: a fullscreen L4 consumes nothing, the check
//! pass enforces that rather than assuming it, and the engine skips the compute
//! passes on the strength of it. That is a claim about absence, and absence is
//! what a test has to be built deliberately to see.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::typed::Checked;

    const W: u32 = 128;
    const H: u32 = 64;

    /// A spawning L1, so "did the simulation run" has a visible answer: a procedure
    /// with a `spawn` block starts empty and fills, so a live count that is still
    /// zero after several steps is a simulation that never happened.
    const SPAWNING_L1: &str = r#"
proc filler {
  kind     L1
  topology points
  capacity [8, 64] = 16

  param spawn_rate : float [0.0, 40000.0] = 1000.0

  emit position

  spawn {
    position = vec3(0.0, 0.0, 0.0);
  }

  element {
    position = position;
  }
}
"#;

    /// The control's L4: per-element, so the same L1 is stepped normally.
    const SPRITE_L4: &str = r#"
proc sprite {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = vec4(position, 1.0);
    point_size = 2.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

    fn fullscreen_l4(body: &str) -> String {
        format!(
            r#"
proc marcher {{
  kind  L4
  blend additive

  fragment {{
{body}
  }}
}}
"#
        )
    }

    fn compile(src: &str) -> Checked {
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        let checked =
            karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        checked
    }

    fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
        errs.iter()
            .map(|e| e.render(src))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn build(gpu: &Gpu, l1: &str, l4: &str) -> Set {
        let l1 = compile(l1);
        let l4 = compile(l4);
        let mut set =
            Set::build(&gpu.device, &gpu.queue, &l1, &l4, 16, 3).expect("a compatible pair");
        set.resize(&gpu.device, W, H);
        set
    }

    /// RGBA f32 per texel after `steps` steps.
    fn draw(gpu: &Gpu, set: &mut Set, steps: u8) -> Vec<[f32; 4]> {
        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, W, H);
        set.prepare(&gpu.queue, steps, &Signals::default());

        let bytes_per_row = W * 8;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(bytes_per_row * H),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        set.render(&mut encoder, present.hdr_view(), steps);
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

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device.poll(wgpu::PollType::Wait).expect("poll");
        let data = slice.get_mapped_range();
        let out: Vec<[f32; 4]> = data
            .chunks_exact(8)
            .map(|t| {
                let h = |i: usize| f16(u16::from_le_bytes([t[i], t[i + 1]]));
                [h(0), h(2), h(4), h(6)]
            })
            .collect();
        drop(data);
        readback.unmap();
        out
    }

    /// `f16` bits to `f32`. Written out rather than pulled in as a dependency, the
    /// same way `tests/deck.rs` and `tests/lines.rs` do it.
    fn f16(bits: u16) -> f32 {
        let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
        let exponent = (bits >> 10) & 0x1f;
        let mantissa = bits & 0x03ff;
        let magnitude = match exponent {
            0 => f32::from(mantissa) * 2.0f32.powi(-24),
            0x1f if mantissa == 0 => f32::INFINITY,
            0x1f => f32::NAN,
            e => (1.0 + f32::from(mantissa) / 1024.0) * 2.0f32.powi(i32::from(e) - 15),
        };
        sign * magnitude
    }

    fn at(px: &[[f32; 4]], x: u32, y: u32) -> [f32; 4] {
        px[(y * W + x) as usize]
    }

    /// Every texel, with no gaps: a fullscreen pass that covered most of the frame
    /// would look right in a thumbnail and be wrong at the corners, which is where
    /// a triangle that is not quite big enough shows first.
    #[test]
    fn a_fullscreen_procedure_covers_every_texel() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(
            &gpu,
            SPAWNING_L1,
            &fullscreen_l4("    color = vec4(1.0, 1.0, 1.0, 1.0);"),
        );
        let px = draw(&gpu, &mut set, 1);

        let dark = px.iter().filter(|t| t[0] < 0.5).count();
        assert_eq!(dark, 0, "{dark} texels of {} were not covered", px.len());
    }

    /// **The claim about absence.** A `spawn` block fills an empty procedure, so a
    /// live count still at zero after several steps is a simulation that did not
    /// run — which is what a fullscreen L4 is supposed to save.
    ///
    /// Paired with the same L1 under a per-element L4, because "the count is zero"
    /// is also what a broken fixture produces.
    #[test]
    fn the_paired_l1_is_not_stepped_at_all() {
        let gpu = Gpu::headless().expect("no GPU available");

        let mut marched = build(
            &gpu,
            SPAWNING_L1,
            &fullscreen_l4("    color = vec4(1.0, 1.0, 1.0, 1.0);"),
        );
        let mut sprites = build(&gpu, SPAWNING_L1, SPRITE_L4);
        for _ in 0..4 {
            draw(&gpu, &mut marched, 1);
            draw(&gpu, &mut sprites, 1);
        }

        assert!(
            sprites.live_count(&gpu.device, &gpu.queue) > 0,
            "the control never spawned, so the fixture cannot tell the two apart"
        );
        assert_eq!(
            marched.live_count(&gpu.device, &gpu.queue),
            0,
            "a fullscreen Set stepped its L1, which nothing reads"
        );
    }

    /// `t` still advances, which is the other half of the sentence above: the
    /// simulation is skipped and the clock is not, because a marcher reads it.
    #[test]
    fn time_still_advances_for_a_fullscreen_set() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(
            &gpu,
            SPAWNING_L1,
            &fullscreen_l4("    color = vec4(t, t, t, 1.0);"),
        );

        let early = draw(&gpu, &mut set, 1);
        for _ in 0..20 {
            draw(&gpu, &mut set, 4);
        }
        let late = draw(&gpu, &mut set, 1);

        assert!(
            at(&early, 64, 32)[0] < at(&late, 64, 32)[0],
            "`t` did not move"
        );
    }

    /// `point_coord` runs across the frame, in the orientation a fragment can act
    /// on: x to the right, y down the framebuffer.
    #[test]
    fn point_coord_spans_the_frame_left_to_right_and_top_to_bottom() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(
            &gpu,
            SPAWNING_L1,
            &fullscreen_l4("    color = vec4(point_coord.x, point_coord.y, 0.0, 1.0);"),
        );
        let px = draw(&gpu, &mut set, 1);

        assert!(
            at(&px, 1, 32)[0] < 0.05,
            "x is {} at the left edge",
            at(&px, 1, 32)[0]
        );
        assert!(
            at(&px, W - 2, 32)[0] > 0.95,
            "x is {} at the right edge",
            at(&px, W - 2, 32)[0]
        );
        assert!(
            at(&px, 64, 1)[1] < 0.05,
            "y is {} at the top",
            at(&px, 64, 1)[1]
        );
        assert!(
            at(&px, 64, H - 2)[1] > 0.95,
            "y is {} at the bottom",
            at(&px, 64, H - 2)[1]
        );
    }

    /// **`ray` is a unit vector and it fans out across the frame.** Both halves
    /// matter: a constant direction would still be unit, and an unnormalised one
    /// would still fan.
    #[test]
    fn the_ray_is_a_unit_vector_that_differs_across_the_frame() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(
            &gpu,
            SPAWNING_L1,
            &fullscreen_l4("    color = vec4(ray * 0.5 + vec3(0.5, 0.5, 0.5), 1.0);"),
        );
        let px = draw(&gpu, &mut set, 1);

        let dir = |x: u32, y: u32| {
            let t = at(&px, x, y);
            [t[0] * 2.0 - 1.0, t[1] * 2.0 - 1.0, t[2] * 2.0 - 1.0]
        };
        for (x, y) in [(2, 2), (W - 3, 2), (2, H - 3), (W - 3, H - 3), (64, 32)] {
            let d = dir(x, y);
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            assert!((len - 1.0).abs() < 0.02, "|ray| at ({x}, {y}) is {len}");
        }

        let left = dir(2, 32);
        let right = dir(W - 3, 32);
        assert!(
            (left[0] - right[0]).abs() + (left[2] - right[2]).abs() > 0.2,
            "the ray does not fan across the frame: {left:?} against {right:?}"
        );
    }

    /// `eye` is the camera's position, and it is the *same* position the projection
    /// matrix puts it at — two derivations of one orbit would agree until one of
    /// them was edited.
    ///
    /// Checked by marching nothing: a fullscreen pass that reports `length(eye)`
    /// should report the orbit's radius, which is `Orbit::default()`'s 8.0.
    #[test]
    fn the_eye_is_where_the_camera_is() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(
            &gpu,
            SPAWNING_L1,
            &fullscreen_l4("    let r = length(eye) * 0.1;\n    color = vec4(r, r, r, 1.0);"),
        );
        let px = draw(&gpu, &mut set, 1);

        // radius 8.0 and height 2.0 — the orbit is at sqrt(8^2 + 2^2) from the
        // origin, scaled by 0.1 to stay inside f16's comfortable range.
        let expected = (8.0f32 * 8.0 + 2.0 * 2.0).sqrt() * 0.1;
        let got = at(&px, 64, 32)[0];
        assert!(
            (got - expected).abs() < 0.02,
            "|eye| * 0.1 is {got}, expected {expected}"
        );
    }
}
