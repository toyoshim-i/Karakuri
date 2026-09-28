//! Integration tests for depth buffer, `blend opaque`, and draw-order verification.

#[path = "common/mod.rs"]
mod common;

mod refused {
    use karakuri_engine::set::{Layering, Set, SetError, Wiring};

    use super::common::compile;

    const L1: &str = r#"
proc point {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

    const ADDITIVE_L4: &str = r#"
proc add_sprite {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip = vec4(position.x, position.y, 0.5, 1.0);
    point_rate = 0.5;
  }

  fragment {
    color = vec4(1.0, 0.0, 0.0, 1.0);
  }
}
"#;

    const OPAQUE_L4: &str = r#"
proc opaque_sprite {
  kind  L4
  blend opaque

  consumes position

  vertex {
    clip = vec4(position.x, position.y, 0.2, 1.0);
    point_rate = 0.5;
  }

  fragment {
    color = vec4(0.0, 1.0, 0.0, 1.0);
  }
}
"#;

    const WEIGHTED_L4: &str = r#"
proc weighted_sprite {
  kind  L4
  blend weighted

  consumes position

  vertex {
    clip = vec4(position.x, position.y, 0.5, 1.0);
    point_rate = 0.5;
  }

  fragment {
    color = vec4(0.0, 0.0, 1.0, 0.5);
  }
}
"#;

    #[test]
    fn opaque_after_additive_is_refused() {
        let l1 = compile(L1);
        let add = compile(ADDITIVE_L4);
        let opaque = compile(OPAQUE_L4);

        let err = Set::validate(
            &[(&l1, 1)],
            &[],
            &[],
            &[],
            &[&add, &opaque],
            Layering::Overdraw,
            0,
            &[],
            Wiring::default(),
        )
        .err()
        .expect("opaque after additive must be refused");

        match err {
            SetError::OpaqueAfterNonOpaque {
                ref opaque,
                ref non_opaque,
            } => {
                assert_eq!(opaque, "opaque_sprite");
                assert_eq!(non_opaque, "add_sprite");
                assert!(err.to_string().contains("opaque geometry must draw first"));
            }
            other => panic!("expected OpaqueAfterNonOpaque, got {other:?}"),
        }
    }

    #[test]
    fn opaque_after_weighted_is_refused() {
        let l1 = compile(L1);
        let weighted = compile(WEIGHTED_L4);
        let opaque = compile(OPAQUE_L4);

        let err = Set::validate(
            &[(&l1, 1)],
            &[],
            &[],
            &[],
            &[&weighted, &opaque],
            Layering::Overdraw,
            0,
            &[],
            Wiring::default(),
        )
        .err()
        .expect("opaque after weighted must be refused");

        match err {
            SetError::OpaqueAfterNonOpaque {
                ref opaque,
                ref non_opaque,
            } => {
                assert_eq!(opaque, "opaque_sprite");
                assert_eq!(non_opaque, "weighted_sprite");
            }
            other => panic!("expected OpaqueAfterNonOpaque, got {other:?}"),
        }
    }
}

mod gpu {
    pub(super) use super::common::{compile, f16};
    use karakuri_engine::set::{Layering, Wiring};
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};

    const W: u32 = 64;
    const H: u32 = 64;

    const OPAQUE_L1: &str = r#"
proc center_opaque {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    // Center of screen, z = 0.5 in NDC
    position = vec3(0.0, 0.0, 0.5);
  }
}
"#;

    const OPAQUE_L4: &str = r#"
proc draw_opaque {
  kind  L4
  blend opaque

  consumes position

  vertex {
    clip = vec4(position.x, position.y, position.z, 1.0);
    // Large square covering the center
    point_rate = 0.5;
  }

  fragment {
    // Pure green
    color = vec4(0.0, 1.0, 0.0, 1.0);
  }
}
"#;

    const PARTICLES_L1: &str = r#"
proc two_particles {
  kind     L1
  topology points
  capacity [2, 2] = 2

  emit position

  element {
    // Slot 0: behind opaque (z = 0.8)
    // Slot 1: in front of opaque (z = 0.2)
    var z = 0.8;
    if seed == 1u {
      z = 0.2;
    }
    position = vec3(0.0, 0.0, z);
  }
}
"#;

    const PARTICLES_L4: &str = r#"
proc draw_particles {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip = vec4(position.x, position.y, position.z, 1.0);
    point_rate = 0.5;
  }

  fragment {
    // Pure red
    color = vec4(1.0, 0.0, 0.0, 1.0);
  }
}
"#;

    fn draw_and_read(gpu: &Gpu, set: &mut Set) -> Vec<[f32; 4]> {
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

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let bytes = slice.get_mapped_range().expect("map");

        let mut out = Vec::with_capacity((W * H) as usize);
        for row in 0..H {
            let row_offset = (row * bytes_per_row) as usize;
            for col in 0..W {
                let at = row_offset + (col * 8) as usize;
                let r = f16(u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()));
                let g = f16(u16::from_le_bytes(
                    bytes[at + 2..at + 4].try_into().unwrap(),
                ));
                let b = f16(u16::from_le_bytes(
                    bytes[at + 4..at + 6].try_into().unwrap(),
                ));
                let a = f16(u16::from_le_bytes(
                    bytes[at + 6..at + 8].try_into().unwrap(),
                ));
                out.push([r, g, b, a]);
            }
        }
        drop(bytes);
        readback.unmap();
        out
    }

    #[test]
    fn opaque_writes_depth_and_occludes_additive_behind() {
        let gpu = Gpu::headless().expect("no GPU available");
        let op_l1 = compile(OPAQUE_L1);
        let op_l4 = compile(OPAQUE_L4);
        let pt_l1 = compile(PARTICLES_L1);
        let pt_l4 = compile(PARTICLES_L4);

        // Opaque renderer first, additive particles second.
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&op_l1, 1), (&pt_l1, 2)],
            &[],
            &[],
            &[],
            &[&op_l4, &pt_l4],
            Layering::Overdraw,
            0,
            &[],
            Wiring::default(),
        )
        .expect("build");

        set.resize(&gpu.device, W, H);
        let pixels = draw_and_read(&gpu, &mut set);

        // At center (32, 32):
        // Opaque quad is green (z = 0.5).
        // Particle 0 (seed 0, z = 0.8) is behind: occluded by depth test!
        // Particle 1 (seed 1, z = 0.2) is in front: adds red (1, 0, 0) over green (0, 1, 0) = yellow!
        let center_idx = (H / 2 * W + W / 2) as usize;
        let [r, g, b, a] = pixels[center_idx];

        assert!(
            (r - 1.0).abs() < 0.05,
            "expected red from in-front particle: got {r}"
        );
        assert!(
            (g - 1.0).abs() < 0.05,
            "expected green from opaque quad: got {g}"
        );
        assert_eq!(b, 0.0);
        assert!((a - 1.0).abs() < 0.05);

        // Outside corner (2, 2): neither covers -> transparent black
        let corner_idx = (2 * W + 2) as usize;
        let [cr, cg, cb, ca] = pixels[corner_idx];
        assert_eq!(cr, 0.0);
        assert_eq!(cg, 0.0);
        assert_eq!(cb, 0.0);
        assert_eq!(ca, 0.0);
    }

    #[test]
    fn depth_test_disabled_allows_overlay_behind_opaque() {
        let gpu = Gpu::headless().expect("no GPU available");
        let op_l1 = compile(OPAQUE_L1);
        let op_l4 = compile(OPAQUE_L4);
        let pt_l1 = compile(PARTICLES_L1);
        let pt_l4 = compile(PARTICLES_L4);

        // Explicitly disable depth test on the second renderer (particles)
        let depth_tests = [None, Some(false)];
        let wiring = Wiring {
            depth_tests: &depth_tests,
            ..Default::default()
        };

        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&op_l1, 1), (&pt_l1, 2)],
            &[],
            &[],
            &[],
            &[&op_l4, &pt_l4],
            Layering::Overdraw,
            0,
            &[],
            wiring,
        )
        .expect("build");

        set.resize(&gpu.device, W, H);
        let pixels = draw_and_read(&gpu, &mut set);

        // With depth_test = false, BOTH particles (seed 0 behind and seed 1 in front) draw!
        // Red accumulates twice (1.0 + 1.0 = 2.0) over green (1.0)
        let center_idx = (H / 2 * W + W / 2) as usize;
        let [r, g, _, _] = pixels[center_idx];
        assert!(
            r >= 1.9,
            "both particles should accumulate when depth test is disabled, got r={r}"
        );
        assert!((g - 1.0).abs() < 0.05);
    }

    #[test]
    fn fullscreen_at_far_plane_sits_behind_opaque_geometry() {
        let gpu = Gpu::headless().expect("no GPU available");

        // Fullscreen background with blend opaque: writes depth = 1.0 (far plane)
        const SKY_L4: &str = r#"
proc blue_sky {
  kind  L4
  blend opaque

  fragment {
    // Pure blue sky
    color = vec4(0.0, 0.0, 1.0, 1.0);
  }
}
"#;

        let sky_l4 = compile(SKY_L4);
        let op_l1 = compile(OPAQUE_L1);
        let op_l4 = compile(OPAQUE_L4);

        // Sky drawn first (opaque, at z = 1.0), then green mesh (opaque, at z = 0.5)
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&op_l1, 1)],
            &[],
            &[],
            &[],
            &[&sky_l4, &op_l4],
            Layering::Overdraw,
            0,
            &[],
            Wiring::default(),
        )
        .expect("build");

        set.resize(&gpu.device, W, H);
        let pixels = draw_and_read(&gpu, &mut set);

        // At center: green opaque quad (z = 0.5) occludes the sky (z = 1.0)
        let center_idx = (H / 2 * W + W / 2) as usize;
        let [r, g, b, _] = pixels[center_idx];
        assert_eq!(r, 0.0);
        assert!((g - 1.0).abs() < 0.05, "expected green foreground, got {g}");
        assert_eq!(b, 0.0);

        // At corner: background sky is visible (pure blue)
        let corner_idx = (2 * W + 2) as usize;
        let [cr, cg, cb, _] = pixels[corner_idx];
        assert_eq!(cr, 0.0);
        assert_eq!(cg, 0.0);
        assert!((cb - 1.0).abs() < 0.05, "expected blue sky, got {cb}");
    }

    #[test]
    fn triangles_topology_renders_mesh_with_depth() {
        let gpu = Gpu::headless().expect("no GPU available");

        const TRI_L1: &str = r#"
proc single_triangle {
  kind     L1
  topology triangles
  capacity [3, 3] = 3

  emit position

  element {
    var p = vec3(0.0, 0.8, 0.4);
    if seed == 1u {
      p = vec3(-0.8, -0.8, 0.4);
    }
    if seed == 2u {
      p = vec3(0.8, -0.8, 0.4);
    }
    position = p;
  }
}
"#;

        const TRI_L4: &str = r#"
proc draw_tri {
  kind  L4
  blend opaque

  consumes position

  vertex {
    clip = vec4(position.x, position.y, position.z, 1.0);
    point_rate = 1.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 0.0, 1.0);
  }
}
"#;

        let l1 = compile(TRI_L1);
        let l4 = compile(TRI_L4);

        let mut set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 3, 0).expect("build");
        set.resize(&gpu.device, W, H);
        let pixels = draw_and_read(&gpu, &mut set);

        // Center of screen inside triangle: yellow (1, 1, 0, 1)
        let center_idx = (H / 2 * W + W / 2) as usize;
        let [r, g, b, a] = pixels[center_idx];
        assert!((r - 1.0).abs() < 0.05);
        assert!((g - 1.0).abs() < 0.05);
        assert_eq!(b, 0.0);
        assert!((a - 1.0).abs() < 0.05);

        // Outside top-left corner: transparent black
        let corner_idx = (2 * W + 2) as usize;
        let [cr, cg, cb, ca] = pixels[corner_idx];
        assert_eq!(cr, 0.0);
        assert_eq!(cg, 0.0);
        assert_eq!(cb, 0.0);
        assert_eq!(ca, 0.0);
    }
}
