//! Tests for nested L5 post-processing procedures running inside a Set (Deck).

#[path = "common/mod.rs"]
mod common;

use common::compile;
use karakuri_engine::set::{Layering, Wiring};
use karakuri_engine::Set;

const L1_GRID: &str = r#"
proc grid {
  kind     L1
  topology points
  capacity [8, 64] = 16

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

const L4_RED: &str = r#"
proc red_fill {
  kind  L4
  blend additive

  fragment {
    color = vec4(1.0, 0.0, 0.0, 1.0);
  }
}
"#;

const L5_INVERT: &str = r#"
proc invert {
  kind L5

  param amount : float [0.0, 1.0] = 1.0

  frame {
    let c = texel(src);
    let inv = vec3(1.0) - c.xyz;
    color = vec4(mix(c.xyz, inv, vec3(amount)), 1.0);
  }
}
"#;

const L5_TINT_GREEN: &str = r#"
proc tint_green {
  kind L5

  param gain : float [0.0, 2.0] = 0.5

  frame {
    let c = texel(src);
    color = vec4(c.x, c.y * gain, c.z, 1.0);
  }
}
"#;

#[test]
fn a_set_with_nested_l5_validates_and_exposes_parameters() {
    let l1 = compile(L1_GRID);
    let l4 = compile(L4_RED);
    let l5 = compile(L5_INVERT);

    let planned = Set::validate(
        &[(&l1, 16)],
        &[],
        &[],
        &[],
        &[&l4],
        &[&l5],
        Layering::Overdraw,
        1,
        &[],
        Wiring::default(),
    );
    assert!(planned.is_ok(), "validation failed: {:?}", planned.err());
}

mod gpu {
    pub(super) use super::common::{compile, f16};
    pub(super) use super::{L1_GRID, L4_RED, L5_INVERT, L5_TINT_GREEN};
    use karakuri_engine::set::{Layering, Wiring};
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::Kind;

    const W: u32 = 64;
    const H: u32 = 64;

    fn draw(gpu: &Gpu, set: &mut Set) -> Vec<[f32; 4]> {
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

    #[test]
    fn nested_l5_transforms_rendered_frame_on_gpu() {
        let gpu = Gpu::headless().expect("GPU available");
        let l1 = compile(L1_GRID);
        let l4 = compile(L4_RED);
        let l5 = compile(L5_INVERT);

        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&l1, 16)],
            &[],
            &[],
            &[],
            &[&l4],
            &[&l5],
            Layering::Overdraw,
            1,
            &[],
            Wiring::default(),
        )
        .expect("builds with L5");

        set.resize(&gpu.device, W, H);

        assert_eq!(set.node_named("invert"), Some((Kind::L5, 0)));

        // Default amount is 1.0 (full invert).
        // L4 draws red vec3(1.0, 0.0, 0.0).
        // L5 invert transforms it to vec3(0.0, 1.0, 1.0) (cyan).
        let pixels = draw(&gpu, &mut set);
        let center = pixels[(H / 2 * W + W / 2) as usize];
        assert!(
            center[0] < 0.1,
            "red channel should be inverted to ~0.0, got {}",
            center[0]
        );
        assert!(
            center[1] > 0.9,
            "green channel should be inverted to ~1.0, got {}",
            center[1]
        );
        assert!(
            center[2] > 0.9,
            "blue channel should be inverted to ~1.0, got {}",
            center[2]
        );

        // Modulate param to 0.0 (bypass invert).
        set.set_param("amount", 0.0);
        let pixels_uninverted = draw(&gpu, &mut set);
        let center_uninverted = pixels_uninverted[(H / 2 * W + W / 2) as usize];
        assert!(
            center_uninverted[0] > 0.9,
            "red channel should be ~1.0, got {}",
            center_uninverted[0]
        );
        assert!(
            center_uninverted[1] < 0.1,
            "green channel should be ~0.0, got {}",
            center_uninverted[1]
        );
        assert!(
            center_uninverted[2] < 0.1,
            "blue channel should be ~0.0, got {}",
            center_uninverted[2]
        );
    }

    #[test]
    fn sequential_nested_l5_passes_chain_together() {
        let gpu = Gpu::headless().expect("GPU available");
        let l1 = compile(L1_GRID);
        let l4 = compile(L4_RED);
        let l5_inv = compile(L5_INVERT);
        let l5_tint = compile(L5_TINT_GREEN);

        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&l1, 16)],
            &[],
            &[],
            &[],
            &[&l4],
            &[&l5_inv, &l5_tint],
            Layering::Overdraw,
            1,
            &[],
            Wiring::default(),
        )
        .expect("builds with multiple L5 passes");

        set.resize(&gpu.device, W, H);

        // L4: (1.0, 0.0, 0.0)
        // Pass 1 (invert amount 1.0): (0.0, 1.0, 1.0)
        // Pass 2 (gain 0.5 on green): (0.0, 0.5, 1.0)
        let pixels = draw(&gpu, &mut set);
        let center = pixels[(H / 2 * W + W / 2) as usize];
        assert!(center[0] < 0.1, "red should be ~0.0, got {}", center[0]);
        assert!(
            (center[1] - 0.5).abs() < 0.1,
            "green should be scaled by 0.5, got {}",
            center[1]
        );
        assert!(center[2] > 0.9, "blue should be ~1.0, got {}", center[2]);
    }
}
