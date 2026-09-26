//! Verification of L2 amplification stages (`amplify <factor>`).
//!
//! Asserts element multiplication, `copy` index progression, parent liveness propagation,
//! and buffer chaining across amplifier stages.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
#[path = "common/mod.rs"]
mod common;

mod gpu {
    pub(super) use super::common::{compile, f16};
    use karakuri_engine::set::Layering;
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::typed::Checked;

    const W: u32 = 64;
    const H: u32 = 64;

    /// **One element, at the origin, still.** Every count below is of things the
    /// amplifier made, so one parent is the whole of what they need — and it keeps
    /// the arithmetic exact: a band count is the factor rather than the factor
    /// times something.
    const STILL: &str = r#"
proc still {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

    /// Element crowd fixture exceeding workgroup size to verify multi-workgroup dispatch.
    const CROWD: &str = r#"
proc crowd {
  kind     L1
  topology points
  capacity [64, 64] = 64

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

    /// Fixture with two elements where one is explicitly killed mid-run to test liveness propagation.
    const ONE_DIES: &str = r#"
proc one_dies {
  kind     L1
  topology points
  capacity [2, 2] = 2

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
    if seed == 1u {
      if t > 0.05 {
        kill();
      }
    }
  }
}
"#;

    /// An amplifier that fans its copies out along `y`, centred on the parent.
    fn fan(name: &str, factor: u32, spread: f32) -> String {
        let half = (factor as f32 - 1.0) / 2.0;
        format!(
            r#"
proc {name} {{
  kind    L2
  amplify {factor}

  consumes position

  deform {{
    position = position + vec3(0.0, (float(copy) - {half:?}) * {spread:?}, 0.0);
  }}
}}
"#
        )
    }

    /// An amplifier that performs identity pass-through, preserving input positions.
    fn silent(name: &str, factor: u32) -> String {
        format!(
            r#"
proc {name} {{
  kind    L2
  amplify {factor}

  consumes position

  deform {{
    position = position;
  }}
}}
"#
        )
    }

    /// An L2 deformation that scales positions away from the origin by `by`.
    fn grow(name: &str, by: f32) -> String {
        format!(
            r#"
proc {name} {{
  kind L2

  consumes position

  deform {{
    position = position * {by:?};
  }}
}}
"#
        )
    }

    /// Draws whatever reaches it, as one small sprite per element. Small so that
    /// neighbouring copies do not merge into one band.
    const DOTS: &str = r#"
proc dots {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

    fn build(gpu: &Gpu, l1: &str, l2s: &[&str]) -> Set {
        // The Set's capacity is the L1's own declared default; what an amplifier
        // allocates is that times its factor, and nothing here has to say so.
        let capacity = compile(l1)
            .capacity
            .expect("an L1 declares a capacity range")
            .default;
        let l2: Vec<Checked> = l2s.iter().map(|s| compile(s)).collect();
        let l2_refs: Vec<&Checked> = l2.iter().collect();
        let l4 = compile(DOTS);
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&compile(l1), capacity)],
            &l2_refs,
            &[],
            &[],
            &[&l4],
            Layering::Overdraw,
            7,
            &[],
            karakuri_engine::set::Wiring::default(),
        )
        .expect("a chain of one L1, some L2s and one L4");
        set.resize(&gpu.device, W, H);
        // Pinned, for the reason `deform.rs` gives at length: an orbiting camera
        // foreshortens by a different amount every frame, and these fixtures lay
        // their material along an axis that has to stay in the picture plane.
        set.aim_camera(karakuri_engine::camera::Orbit {
            radius: 5.0,
            speed: 0.0,
            height: 0.0,
            ..Default::default()
        });
        set
    }

    /// Counts the number of distinct horizontal lit bands across the frame.
    fn bands(gpu: &Gpu, set: &mut Set) -> usize {
        let px = frame(gpu, set);
        let mut lit = vec![false; H as usize];
        for (i, t) in px.chunks_exact(4).enumerate() {
            if t[0] > 0.01 {
                lit[i / W as usize] = true;
            }
        }
        assert!(
            lit.iter().any(|&b| b),
            "nothing was drawn, so there are no bands to count"
        );
        lit.iter()
            .enumerate()
            .filter(|&(y, &on)| on && (y == 0 || !lit[y - 1]))
            .count()
    }

    /// Calculates total red-channel luminance across the rendered frame.
    fn total_light(gpu: &Gpu, set: &mut Set) -> f64 {
        frame(gpu, set)
            .chunks_exact(4)
            .map(|t| f64::from(t[0]))
            .sum()
    }

    /// Total brightness after a frame that **draws without stepping** — the audition
    /// path, where a deck shows an `Allocated` slot the still it stopped at.
    fn draw_only(gpu: &Gpu, set: &mut Set) -> f64 {
        render_frame(gpu, set, false)
            .chunks_exact(4)
            .map(|t| f64::from(t[0]))
            .sum()
    }

    /// RGBA f32 per texel, after one frame.
    fn frame(gpu: &Gpu, set: &mut Set) -> Vec<f32> {
        render_frame(gpu, set, true)
    }

    fn render_frame(gpu: &Gpu, set: &mut Set, step: bool) -> Vec<f32> {
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
        if step {
            set.render(&mut encoder, present.hdr_view(), 1);
        } else {
            set.draw(&mut encoder, present.hdr_view());
        }
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
        if step {
            set.commit();
        }

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

    // ---------------------------------------------------------------------------

    /// Verifies that an amplifying L2 stage renders multiple distinct copies of an input element.
    #[test]
    fn an_amplifying_l2_draws_one_element_as_many() {
        let gpu = Gpu::headless().expect("a GPU");

        let mut plain = build(&gpu, STILL, &[]);
        assert_eq!(bands(&gpu, &mut plain), 1, "one element is one band");

        let fan4 = fan("fan4", 4, 0.9);
        let mut amplified = build(&gpu, STILL, &[&fan4]);
        assert_eq!(
            bands(&gpu, &mut amplified),
            4,
            "`amplify 4` makes four elements of one, at four positions"
        );
    }

    /// **The factor is what decides the count**, so a different one gives a
    /// different picture. Without this a hardcoded four anywhere in the lowering
    /// would pass the test above.
    #[test]
    fn the_declared_factor_is_the_number_of_copies() {
        let gpu = Gpu::headless().expect("a GPU");
        for factor in [2u32, 3, 5] {
            let f = fan("fanned", factor, 0.8);
            let mut set = build(&gpu, STILL, &[&f]);
            assert_eq!(
                bands(&gpu, &mut set),
                factor as usize,
                "`amplify {factor}` should make {factor} copies"
            );
        }
    }

    /// Verifies that stacked amplifier stages multiply element counts and compose copy indices.
    #[test]
    fn stacked_amplifiers_multiply_and_compose_the_index() {
        let gpu = Gpu::headless().expect("a GPU");
        let first = silent("first", 2);
        let second = fan("second", 6, 0.55);
        // The second stage's factor is 3, but it spreads over the *composed* range,
        // so it is written as a fan of six over a chain whose product is six.
        let second = second.replace("amplify 6", "amplify 3");
        let mut set = build(&gpu, STILL, &[&first, &second]);
        assert_eq!(
            bands(&gpu, &mut set),
            6,
            "two by three is six distinct elements, not three pairs at three places"
        );
    }

    /// Verifies that killed parent elements generate no live amplified copies.
    #[test]
    fn a_dead_parent_leaves_no_live_copies() {
        let gpu = Gpu::headless().expect("a GPU");

        // Step plain and amplified Sets in lockstep to verify 3x light invariant across element death.
        let f = fan("fan3", 3, 0.9);
        let mut plain = build(&gpu, ONE_DIES, &[]);
        let mut amplified = build(&gpu, ONE_DIES, &[&f]);

        let mut saw_the_death = false;
        let mut first = None;
        for frame in 0..10 {
            let one = total_light(&gpu, &mut plain);
            let many = total_light(&gpu, &mut amplified);
            let base = *first.get_or_insert(one);
            assert!(
                (many - one * 3.0).abs() < one * 0.05,
                "frame {frame}: {many} lit against {one} for the same geometry unamplified — \
             three copies of every live parent and no copies of a dead one"
            );
            if one < base * 0.75 {
                saw_the_death = true;
            }
        }
        assert!(
            saw_the_death,
            "one of the two parents has to actually die inside the window, or the \
         invariant above was never put under any strain"
        );
    }

    /// Verifies that stages downstream of an amplifier process the amplified buffer range.
    #[test]
    fn every_stage_below_an_amplifier_still_sees_every_copy() {
        let gpu = Gpu::headless().expect("a GPU");

        let f = fan("fan4", 4, 0.8);
        let after = grow("after", 1.4);
        let again = grow("again", 1.15);

        let mut one = build(&gpu, CROWD, &[&f, &after]);
        assert_eq!(
            bands(&gpu, &mut one),
            4,
            "a stage below the amplifier deforms every copy, not the one parent that reached it"
        );

        let mut two = build(&gpu, CROWD, &[&f, &after, &again]);
        assert_eq!(
            bands(&gpu, &mut two),
            4,
            "and so does the stage below that one — a fifth band is the elements a \
         short dispatch never wrote, sitting at the origin"
        );
    }

    /// Verifies that overlapping amplified copies contribute additive light proportional to copy count.
    #[test]
    fn stacked_copies_are_counted_by_the_light_they_add() {
        let gpu = Gpu::headless().expect("a GPU");

        // The copies land on their parent, deliberately: with nothing separating
        // them the only thing that can differ between the two frames is how much
        // light each element contributed, which is what the claim is about.
        let still = fan("still4", 4, 0.0);
        let mut plain = build(&gpu, STILL, &[]);
        let mut amplified = build(&gpu, STILL, &[&still]);

        let one = total_light(&gpu, &mut plain);
        let four = total_light(&gpu, &mut amplified);
        assert!(
            (four - one * 4.0).abs() < one * 0.05,
            "four copies of one element is four times the light, not sixteen: {four} against {one}"
        );
    }

    /// Asserts that an amplified Set can be drawn before its first step without
    /// rendering empty instances.
    #[test]
    fn an_amplified_set_draws_without_having_been_stepped() {
        let gpu = Gpu::headless().expect("a GPU");

        let f = fan("fan4", 4, 0.0);
        let mut plain = build(&gpu, STILL, &[]);
        let mut amplified = build(&gpu, STILL, &[&f]);

        let one = draw_only(&gpu, &mut plain);
        let four = draw_only(&gpu, &mut amplified);
        assert!(
            one > 0.0,
            "the unamplified Set draws its initial state without a step"
        );
        assert!(
            (four - one * 4.0).abs() < one * 0.05,
            "and the amplified one draws four copies of it: {four} against {one}"
        );
    }

    /// Verifies that amplifier chains exceeding device buffer limits return descriptive refusal errors.
    #[test]
    fn an_amplified_chain_too_large_for_the_device_is_refused_rather_than_fatal() {
        let gpu = Gpu::headless().expect("a GPU");
        let huge = r#"
proc huge {
  kind     L1
  topology points
  capacity [1, 1048576] = 1048576

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;
        // Minimal deformation body to test device buffer limits independently of cost ceiling.
        let l2 = silent("enormous", 1024);
        let l2 = compile(&l2);
        let l4 = compile(DOTS);
        let built = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&compile(huge), 1_048_576)],
            &[&l2],
            &[],
            &[],
            &[&l4],
            Layering::Overdraw,
            7,
            &[],
            karakuri_engine::set::Wiring::default(),
        );
        let Err(err) = built else {
            panic!("a billion elements is past every device, and building it should have said so");
        };
        let text = err.to_string();
        assert!(
            text.contains("enormous") && text.contains("amplifies to"),
            "the refusal has to name the node and the size: {text}"
        );
    }
}
