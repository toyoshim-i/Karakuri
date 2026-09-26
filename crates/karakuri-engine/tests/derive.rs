//! Integration tests for automatic attribute derivation rules (`age`, `velocity`).
//!
//! Validates derived attributes against hand-emitted procedure baselines and
//! verifies slot allocation behavior.

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

    /// Generator for an L1 procedure drifting along y at a known rate.
    fn mover(name: &str, emit: &str, extra: &str) -> String {
        format!(
            r#"
proc {name} {{
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit {emit}

  element {{
    position = position + vec3(0.0, 0.25, 0.0) * dt;
{extra}  }}
}}
"#
        )
    }

    /// Generator for an L4 renderer placing sprites according to consumed attributes.
    pub(super) fn reader(name: &str, consumes: &str, y: &str) -> String {
        format!(
            r#"
proc {name} {{
  kind  L4
  blend additive

  consumes {consumes}

  vertex {{
    clip       = camera * vec4(0.0, {y}, 0.0, 1.0);
    point_rate = 0.03125;
  }}

  fragment {{
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }}
}}
"#
        )
    }

    /// Generator for an L1 procedure with dynamic element spawning over time.
    fn spawner(name: &str, emit: &str, spawn_extra: &str, element_extra: &str) -> String {
        format!(
            r#"
proc {name} {{
  kind     L1
  topology points
  capacity [64, 64] = 64

  param spawn_rate : float [0.0, 4000.0] = 192.0

  emit {emit}

  spawn {{
    position = vec3(0.0, 0.0, 0.0);
{spawn_extra}  }}

  element {{
    position = position + vec3(0.0, 0.25, 0.0) * dt;
{element_extra}  }}
}}
"#
        )
    }

    fn build(gpu: &Gpu, l1: &str, l4: &str) -> Set {
        build_chain(gpu, l1, &[], l4).expect("a pair whose `consumes` is satisfied")
    }

    fn build_chain(
        gpu: &Gpu,
        l1: &str,
        l2s: &[&str],
        l4: &str,
    ) -> Result<Set, karakuri_engine::set::SetError> {
        let compiled: Vec<Checked> = l2s.iter().map(|s| compile(s)).collect();
        let l2_refs: Vec<&Checked> = compiled.iter().collect();
        let l1 = compile(l1);
        let capacity = l1
            .capacity
            .expect("an L1 declares a capacity range")
            .default;
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&l1, capacity)],
            &l2_refs,
            &[],
            &[],
            &[&compile(l4)],
            Layering::Overdraw,
            7,
            &[],
            karakuri_engine::set::Wiring::default(),
        )?;
        set.resize(&gpu.device, W, H);
        set.aim_camera(karakuri_engine::camera::Orbit {
            radius: 5.0,
            speed: 0.0,
            height: 0.0,
            ..Default::default()
        });
        Ok(set)
    }

    /// Where the material sits on screen after `frames` frames, in texels.
    fn centre_y(gpu: &Gpu, set: &mut Set, frames: u32) -> f32 {
        let mut px = Vec::new();
        for _ in 0..frames {
            px = frame(gpu, set);
        }
        let (mut sum, mut weight) = (0.0f64, 0.0f64);
        for (i, t) in px.chunks_exact(4).enumerate() {
            if t[0] > 0.01 {
                sum += f64::from(t[0]) * f64::from(i as u32 / W);
                weight += f64::from(t[0]);
            }
        }
        assert!(
            weight > 0.0,
            "nothing was drawn, so there is no position to measure"
        );
        (sum / weight) as f32
    }

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

    // ---------------------------------------------------------------------------

    const FRAMES: u32 = 20;

    /// Asserts that automatically derived `age` matches an explicitly accumulated `age`
    /// attribute across frames.
    #[test]
    fn a_derived_age_matches_one_the_procedure_accumulates() {
        let gpu = Gpu::headless().expect("a GPU");
        let l4 = reader("by_age", "position, age", "age * 3.0 - 1.0");

        let mut anchored = build(
            &gpu,
            &mover("anchored", "position, age", "    age = age + dt;\n"),
            &l4,
        );
        let mut derived = build(&gpu, &mover("bare", "position", ""), &l4);

        let a = centre_y(&gpu, &mut anchored, FRAMES);
        let d = centre_y(&gpu, &mut derived, FRAMES);
        assert!(
            (a - d).abs() < 1.5,
            "an accumulated age put the sprite at row {a} and a derived one at row {d}"
        );

        // What age zero looks like, so that "they agree" cannot be "they are both
        // nothing". The renderer's own arithmetic decides this row, not the test.
        let mut still = build(
            &gpu,
            &mover("still", "position", ""),
            &reader("at_zero", "position", "0.0 * 3.0 - 1.0"),
        );
        let zero = centre_y(&gpu, &mut still, FRAMES);
        assert!(
        (d - zero).abs() > 3.0,
        "a derived age of {FRAMES} frames should not draw where age zero does: {d} against {zero}"
    );
    }

    /// Verifies that derived age is measured from each element's individual spawn instant.
    #[test]
    fn a_derived_age_is_measured_from_the_spawn_instant() {
        let gpu = Gpu::headless().expect("a GPU");
        let l4 = reader("by_age", "position, age", "age * 6.0 - 1.0");

        let mut anchored = build(
            &gpu,
            &spawner(
                "anchored",
                "position, age",
                "    age = 0.0;\n",
                "    age = age + dt;\n",
            ),
            &l4,
        );
        let mut derived = build(&gpu, &spawner("bare", "position", "", ""), &l4);

        let a = centre_y(&gpu, &mut anchored, FRAMES);
        let d = centre_y(&gpu, &mut derived, FRAMES);
        assert!(
            (a - d).abs() < 1.5,
            "a spawning population's accumulated ages sat at row {a} and its derived ages at {d}"
        );
    }

    /// Verifies that derived velocity matches hand-written element motion.
    #[test]
    fn a_derived_velocity_matches_one_the_procedure_writes() {
        let gpu = Gpu::headless().expect("a GPU");
        let l4 = reader(
            "by_speed",
            "position, velocity",
            "length(velocity) * 4.0 - 1.0",
        );

        let mut anchored = build(
            &gpu,
            &mover(
                "anchored",
                "position, velocity",
                "    velocity = vec3(0.0, 0.25, 0.0);\n",
            ),
            &l4,
        );
        let mut derived = build(&gpu, &mover("bare", "position", ""), &l4);

        let a = centre_y(&gpu, &mut anchored, FRAMES);
        let d = centre_y(&gpu, &mut derived, FRAMES);
        assert!(
            (a - d).abs() < 1.5,
            "a written velocity put the sprite at row {a} and a derived one at row {d}"
        );

        let mut still = build(
            &gpu,
            &mover("still", "position", ""),
            &reader("at_zero", "position", "0.0 * 4.0 - 1.0"),
        );
        let zero = centre_y(&gpu, &mut still, FRAMES);
        assert!(
            (d - zero).abs() > 3.0,
            "a derived velocity should not draw where a speed of zero does: {d} against {zero}"
        );
    }

    /// Verifies that derivation slots are only allocated when consumed by downstream nodes.
    #[test]
    fn a_derivations_slot_exists_only_where_something_consumes_it() {
        let gpu = Gpu::headless().expect("a GPU");
        let bare = mover("bare", "position", "");

        let plain = build(&gpu, &bare, &reader("plain", "position", "0.0"));
        let aged = build(&gpu, &bare, &reader("aged", "position, age", "age"));
        let moving = build(
            &gpu,
            &bare,
            &reader("moving", "position, velocity", "velocity.y"),
        );
        let both = build(
            &gpu,
            &bare,
            &reader("both", "position, age, velocity", "age + velocity.y"),
        );

        let has = |s: &Set, name: &str| s.element_layout().slots.iter().any(|f| f.name == name);
        for (set, label) in [
            (&plain, "plain"),
            (&aged, "aged"),
            (&moving, "moving"),
            (&both, "both"),
        ] {
            assert_eq!(
                has(set, "birth_t"),
                label == "aged" || label == "both",
                "{label}: `birth_t`"
            );
            assert_eq!(
                has(set, "velocity"),
                label == "moving" || label == "both",
                "{label}: `velocity`"
            );
        }

        // And a stored derivation still costs bytes, because `velocity` is a `vec3`
        // and there is no padding upstream of it to hide in. Its companion flag is
        // the one that is free — it lands in the four bytes the `vec3` leaves.
        let stride = |s: &Set| s.element_layout().stride;
        assert!(
            stride(&moving) > stride(&plain),
            "a stored derivation is not free"
        );
        assert_eq!(
            stride(&both),
            stride(&moving),
            "`birth_t` fits in what was already padding"
        );
    }

    /// Verifies that explicit procedure attributes take precedence over automatic derivation.
    #[test]
    fn an_emitted_attribute_is_read_rather_than_derived() {
        let gpu = Gpu::headless().expect("a GPU");
        let l4 = reader("by_age", "position, age", "age * 3.0 - 1.0");

        let bare = build(&gpu, &mover("bare", "position", ""), &l4);
        // The emitter deliberately writes something the clock never would: age
        // counting *down*. If the derivation ran anyway, the sprite would go the
        // other way.
        let wrong_way = build(
            &gpu,
            &mover("counts_down", "position, age", "    age = age - dt;\n"),
            &l4,
        );

        assert_eq!(
            bare.element_layout().stride,
            wrong_way.element_layout().stride,
            "one slot either way — `birth_t` for the derived Set, `age` for the emitting one"
        );

        let mut bare = bare;
        let mut wrong_way = wrong_way;
        let up = centre_y(&gpu, &mut bare, FRAMES);
        let down = centre_y(&gpu, &mut wrong_way, FRAMES);
        assert!(
            (up - down).abs() > 3.0,
            "the emitting procedure's own `age` should be what is drawn: {up} against {down}"
        );

        // An L2 above the emitter asks for `age`, and a second L2 below it emits
        // one. Deriving here would give the first node a substitution and the
        // second a slot, for one name, in one chain.
        let wants = r#"
proc wants_age {
  kind L2
  consumes position, age
  deform { position = position + vec3(0.0, age, 0.0); }
}
"#;
        let emits = r#"
proc emits_age {
  kind L2
  emit age
  consumes position
  deform { age = length(position); }
}
"#;
        let err = build_chain(
            &gpu,
            &mover("bare", "position", ""),
            &[wants, emits],
            &reader("plain", "position", "0.0"),
        )
        .err()
        .expect("`age` is emitted below the node that consumes it, so it is not derived there");
        assert!(
            err.to_string().contains("age"),
            "the refusal has to name the attribute: {err}"
        );
    }

    /// Verifies that L1 procedures consuming derived attributes receive their previous frame's values.
    #[test]
    fn an_l1_reading_a_derived_age_matches_one_that_accumulates_its_own() {
        let gpu = Gpu::headless().expect("a GPU");
        let l4 = reader("plain", "position", "position.y");

        // The L1 writes its own age into `position.y`, so the renderer needs to
        // know nothing about the rule.
        let anchored = r#"
proc anchored {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position, age

  element {
    age      = age + dt;
    position = vec3(0.0, age * 3.0 - 1.0, 0.0);
  }
}
"#;
        let derived = r#"
proc derives {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit     position
  consumes age

  element {
    position = vec3(0.0, age * 3.0 - 1.0, 0.0);
  }
}
"#;
        let mut a = build(&gpu, anchored, &l4);
        let mut d = build(&gpu, derived, &l4);
        let (ay, dy) = (
            centre_y(&gpu, &mut a, FRAMES),
            centre_y(&gpu, &mut d, FRAMES),
        );
        assert!(
            (ay - dy).abs() < 1.5,
            "an L1's own accumulated age put the sprite at row {ay} and a derived one at {dy}"
        );
    }

    /// The same claim for an L2, which reads through a different resolver and would
    /// fail in a different file.
    #[test]
    fn an_l2_reading_a_derived_age_matches_one_the_l1_accumulates() {
        let gpu = Gpu::headless().expect("a GPU");
        let l4 = reader("plain", "position", "position.y");
        let lift = r#"
proc lift {
  kind L2

  consumes position, age

  deform {
    position = vec3(0.0, age * 3.0 - 1.0, 0.0);
  }
}
"#;
        let anchored = build_chain(
            &gpu,
            &mover("anchored", "position, age", "    age = age + dt;\n"),
            &[lift],
            &l4,
        )
        .expect("the L1 emits `age`");
        let derived = build_chain(&gpu, &mover("bare", "position", ""), &[lift], &l4)
            .expect("nothing emits `age`, so it is derived");

        let mut anchored = anchored;
        let mut derived = derived;
        let (ay, dy) = (
            centre_y(&gpu, &mut anchored, FRAMES),
            centre_y(&gpu, &mut derived, FRAMES),
        );
        assert!(
            (ay - dy).abs() < 1.5,
            "an L2 over an emitted age drew at row {ay} and over a derived one at {dy}"
        );
    }

    /// Verifies that newly spawned elements have zero derived velocity during their initial update.
    #[test]
    fn a_first_update_has_no_derived_velocity() {
        let gpu = Gpu::headless().expect("a GPU");
        const SPEED: f32 = 0.25;

        // Closed form, so nothing about the body scales with the step: every
        // element jumps to where `t` says it should be, whatever fraction of a step
        // it has lived.
        let l1 = format!(
            r#"
proc jumps {{
  kind     L1
  topology points
  capacity [64, 64] = 64

  param spawn_rate : float [0.0, 4000.0] = 600.0

  emit position

  spawn {{
    position = vec3(0.0, 0.0, 0.0);
  }}

  element {{
    position = vec3(0.0, {SPEED:?} * t, 0.0);
  }}
}}
"#
        );
        let mut set = build(
            &gpu,
            &l1,
            &reader("by_speed", "position, velocity", "length(velocity)"),
        );
        for _ in 0..3 {
            frame(&gpu, &mut set);
        }

        let layout = set.element_layout().clone();
        let offset = layout.offset_of("velocity") as usize;
        let stride = layout.stride as usize;
        let bytes = set.read_elements(&gpu.device, &gpu.queue);
        let speeds: Vec<f32> = bytes
            .chunks_exact(stride)
            .map(|e| {
                let y = f32::from_le_bytes(e[offset + 4..offset + 8].try_into().unwrap());
                y.abs()
            })
            .collect();

        // Every element is either still waiting for its first whole step, or moving
        // at the one speed this procedure has. Nothing in between, and nothing
        // above — an inflated first update reads as a multiple of the true speed,
        // and the multiple is up to twice the batch size.
        for (i, &v) in speeds.iter().enumerate() {
            assert!(
            v < 1e-4 || (v - SPEED).abs() < SPEED * 0.05,
            "element {i} has a derived speed of {v}, and this procedure only ever moves at {SPEED}"
        );
        }
        assert!(
        speeds.iter().any(|&v| (v - SPEED).abs() < SPEED * 0.05),
        "no element reached the true speed at all, so the assertion above was vacuous: {speeds:?}"
    );
    }
}

/// Validation refusal tests for attribute derivation requirements.
mod refused {
    use super::gpu::{compile, reader};
    use karakuri_engine::set::{Layering, SetError, Wiring};
    use karakuri_engine::Set;
    use karakuri_ir::typed::Checked;

    /// The Set `build_chain` above describes — one L1 at its declared default
    /// capacity, this chain of L2s, one renderer — minus the device and
    /// everything downstream of it.
    fn validate_chain(l1: &str, l2s: &[&str], l4: &str) -> Result<(), SetError> {
        let compiled: Vec<Checked> = l2s.iter().map(|s| compile(s)).collect();
        let l2_refs: Vec<&Checked> = compiled.iter().collect();
        let l1 = compile(l1);
        let capacity = l1
            .capacity
            .expect("an L1 declares a capacity range")
            .default;
        Set::validate(
            &[(&l1, capacity)],
            &l2_refs,
            &[],
            &[],
            &[&compile(l4)],
            Layering::Overdraw,
            7,
            &[],
            Wiring::default(),
        )
        .map(|_| ())
    }

    /// Verifies that derivation refusal diagnostics name the missing source attribute.
    #[test]
    fn a_blocked_rule_names_the_attribute_it_needed() {
        let no_position = r#"
proc no_position {
  kind     L1
  topology points
  capacity [8, 8] = 8

  emit tint

  element {
    tint = vec3(1.0, 1.0, 1.0);
  }
}
"#;
        let err = validate_chain(
            no_position,
            &[],
            &reader("wants_speed", "velocity", "velocity.y"),
        )
        .expect_err("`velocity` derives from `position`, which this L1 does not emit");

        let text = err.to_string();
        assert!(
            text.contains("position"),
            "the refusal has to name what the rule wanted: {text}"
        );
        assert!(
            !text.contains("nothing else is"),
            "and must not fall through to the hint that says `velocity` is synthesised: {text}"
        );
    }
}
