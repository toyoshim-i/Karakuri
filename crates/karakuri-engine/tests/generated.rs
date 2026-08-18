//! The V1 assumption, minus the LLM and the hot swap.
//!
//! > Can an LLM generate constrained IR, can we validate it and compile it to
//! > WGSL, and can we hot-swap it without dropping a frame?
//!
//! This exercises the middle of that: `.kir` text goes in, and pixels come out
//! of a real GPU, through every stage in between — parse, check, cost, generate
//! WGSL, build pipelines against the published binding layout, dispatch
//! compute, render. Nothing here is hand-written shader code.
//!
//! It is deliberately the same shape as `offscreen.rs`, which guards the
//! hand-written stand-in: when the generated path can do everything the
//! stand-in does, the stand-in can go.

use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
const CAPACITY: u32 = 4096;

/// A procedure with no `spawn` block: every element is live from frame zero and
/// `seed` is its slot index, which is the simplest form the spec describes and
/// the one that needs no compaction to run.
const L1: &str = r#"
proc static_shell {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5
  param swirl  : float [0.0, 4.0] = 1.0

  emit position, velocity, age

  element {
    let u    = hash1(seed);
    let v    = hash1(seed + 1000u);
    let base = sphere_point(u, v) * radius;
    let p    = rot_y(base, t * swirl * 0.2);

    position = p;
    velocity = p * 0.1;
    age      = age + dt;
  }
}
"#;

const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position, velocity, age

  param point_scale : float [0.5, 40.0] = 8.0
  param hue         : float [0.0, 1.0]  = 0.6
  param exposure    : float [0.0, 8.0]  = 1.4
  param falloff     : float [0.5, 8.0]  = 2.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = point_scale;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    let a = pow(max(0.0, 1.0 - d), falloff);
    let c = hsv_to_rgb(vec3(hue + hash1(seed) * 0.05, 0.7, 1.0));
    color = vec4(c * exposure, a);
  }
}
"#;

/// Everything stages 1 through 4 do, with the diagnostics rendered against the
/// source if any stage refuses.
fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src)
        .unwrap_or_else(|errs| panic!("{}", render(&errs, src)));
    let checked = karakuri_ir::check::check(&proc)
        .unwrap_or_else(|errs| panic!("{}", render(&errs, src)));
    karakuri_ir::cost::estimate(&checked)
        .unwrap_or_else(|errs| panic!("{}", render(&errs, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build(gpu: &Gpu, capacity: u32, seed: u32) -> Set {
    let l1 = compile(L1);
    let l4 = compile(L4);
    let mut set = Set::build(
        &gpu.device,
        &gpu.queue,
        &l1,
        &l4,
        capacity,
        seed,
    )
    .expect("the pair is compatible and the capacity is in range");
    set.resize(&gpu.device, WIDTH, HEIGHT);
    set
}

fn frame(gpu: &Gpu, set: &mut Set, steps: u8) -> Vec<u16> {
    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, WIDTH, HEIGHT);
    // No bindings on these Sets, so the session clock is inert here and
    // nothing reads a signal; the real one belongs to the deck, and
    // `tests/binding.rs` is where it is asserted.
    set.prepare(&gpu.queue, steps, &Signals::default());

    let bytes_per_row = WIDTH * 8;
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(bytes_per_row * HEIGHT),
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
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
    let out = data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    drop(data);
    readback.unmap();
    out
}

fn lit(pixels: &[u16]) -> usize {
    pixels
        .chunks_exact(4)
        .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
        .count()
}

#[test]
fn kir_source_reaches_the_screen() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu, CAPACITY, 19274);
    assert_eq!(set.live_count(&gpu.device, &gpu.queue), CAPACITY, "a spawn-less procedure is full");

    let pixels = frame(&gpu, &mut set, 1);
    let n = lit(&pixels);
    assert!(n > 100, "the generated pair drew nothing: {n} lit texels");
    assert!(
        n < (WIDTH * HEIGHT) as usize,
        "the whole frame is lit, so the camera or the blend is wrong"
    );
}

#[test]
fn the_compute_pass_actually_runs() {
    // The shell rotates with `t`, and `t` only advances through `prepare`. If
    // the compute pass were skipped, or if L4 were reading a buffer the
    // compute pass never wrote, every frame would be identical.
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu, CAPACITY, 19274);
    let early = frame(&gpu, &mut set, 1);
    for _ in 0..30 {
        frame(&gpu, &mut set, 4);
    }
    let late = frame(&gpu, &mut set, 1);
    assert_ne!(early, late, "the geometry never moved");
}

#[test]
fn the_same_seed_and_the_same_steps_reproduce_the_same_frame() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut a = build(&gpu, CAPACITY, 19274);
    let mut b = build(&gpu, CAPACITY, 19274);
    for _ in 0..5 {
        frame(&gpu, &mut a, 1);
        frame(&gpu, &mut b, 1);
    }
    assert_eq!(frame(&gpu, &mut a, 1), frame(&gpu, &mut b, 1));
}

#[test]
fn reseeding_changes_the_result_without_changing_the_procedure() {
    // `{"t":"seed"}` salts the hash builtins. If this fails, a seed record
    // does nothing and the artifact cannot be re-rolled.
    let gpu = Gpu::headless().expect("no GPU available");
    let mut a = build(&gpu, CAPACITY, 19274);
    let mut b = build(&gpu, CAPACITY, 88888);
    assert_ne!(frame(&gpu, &mut a, 1), frame(&gpu, &mut b, 1));
}

#[test]
fn capacity_is_a_set_level_dial() {
    // Same artifact, two capacities, no recompilation of the IR: the whole
    // reason capacity was moved out of the procedure's identity.
    let gpu = Gpu::headless().expect("no GPU available");
    let sparse = lit(&frame(&gpu, &mut build(&gpu, 1024, 19274), 1));
    let dense = lit(&frame(&gpu, &mut build(&gpu, 65536, 19274), 1));
    assert!(dense > sparse, "sparse {sparse}, dense {dense}");
}

#[test]
fn a_capacity_outside_the_declared_range_is_refused() {
    let gpu = Gpu::headless().expect("no GPU available");
    let l1 = compile(L1);
    let l4 = compile(L4);
    let result = Set::build(
        &gpu.device,
        &gpu.queue,
        &l1,
        &l4,
        999_999,
        1,
    );
    let msg = match result {
        Ok(_) => panic!("999999 is above the declared maximum and was accepted"),
        Err(e) => e.to_string(),
    };
    assert!(msg.contains("999999") && msg.contains("262144"), "{msg}");
}

/// An L1 emitting exactly `attrs` and writing nothing else. Narrowing `L1`'s
/// own `emit` will not do: an attribute is writable only where it is emitted,
/// so dropping one from the list makes the body itself illegal and the failure
/// lands in the checker rather than at composition.
fn narrow_l1(attrs: &str) -> String {
    let writes: String = attrs
        .split(", ")
        .map(|a| match a {
            "position" => "    position = sphere_point(hash1(seed), hash1(seed + 1u)) * 2.0;\n",
            "age" => "    age = age + dt;\n",
            "normal" => "    normal = vec3(0.0, 1.0, 0.0);\n",
            other => panic!("narrow_l1 has no body for `{other}`"),
        })
        .collect();
    format!(
        "proc narrow {{\n  kind L1\n  topology points\n  capacity [1024, 262144] = 4096\n\
         \n  emit {attrs}\n\n  element {{\n{writes}  }}\n}}\n"
    )
}

/// An L4 consuming attributes with no derivation rule, for the composition
/// tests below.
///
/// **`velocity` and `age` cannot be used for this any more**, and that is the
/// point rather than an inconvenience: both have a rule now, so a pair missing
/// one of them composes instead of failing. Two tests here used exactly those
/// two as their "unsatisfiable" fixture and started passing for the wrong
/// reason — a refusal test whose fixture became satisfiable asserts nothing and
/// says nothing about it.
const L4_UNSATISFIABLE: &str = r#"
proc wants_shape {
  kind  L4
  blend additive

  consumes position, normal, uv

  vertex {
    clip       = camera * vec4(position + normal * 0.0, 1.0);
    point_size = 4.0 + uv.x * 0.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

/// Stage 6, the one check that needs both procedures at once. An L4 reads the
/// element struct its paired L1 wrote, so consuming an attribute that L1 never
/// emitted — and that nothing knows how to synthesise — has no field to read.
/// Without this check it surfaces as a WGSL parse failure from inside
/// `create_shader_module`, which is an internal error where the contract calls
/// for a diagnostic naming what to fix.
#[test]
fn an_l4_consuming_what_the_l1_never_emits_is_refused() {
    let gpu = Gpu::headless().expect("no GPU available");
    let l1 = compile(&narrow_l1("position, normal"));
    let l4 = compile(L4_UNSATISFIABLE);
    let result = Set::build(&gpu.device, &gpu.queue, &l1, &l4, CAPACITY, 1);
    let msg = match result {
        Ok(_) => panic!("`uv` is consumed, never emitted, and has no rule — and was accepted"),
        Err(e) => e.to_string(),
    };
    assert!(msg.contains("uv"), "{msg}");
    assert!(msg.contains("emit"), "no hint at what to do: {msg}");
}

/// Every missing attribute at once, not just the first — a regeneration should
/// be able to fix all of them in one pass. Same rule the IR checker follows.
#[test]
fn a_composition_error_names_every_missing_attribute() {
    let gpu = Gpu::headless().expect("no GPU available");
    let l1 = compile(&narrow_l1("position"));
    let l4 = compile(L4_UNSATISFIABLE);
    let msg = match Set::build(&gpu.device, &gpu.queue, &l1, &l4, CAPACITY, 1) {
        Ok(_) => panic!("two attributes are missing and the pair was accepted"),
        Err(e) => e.to_string(),
    };
    assert!(msg.contains("normal") && msg.contains("uv"), "{msg}");
}

/// **And the pair that used to fail now composes**, which is the other half of
/// the change and the half a refusal test cannot state. An L1 emitting only
/// `position` paired with a renderer wanting `velocity` and `age` builds, and
/// the two attributes are the engine's to provide.
#[test]
fn an_l4_consuming_a_derivable_attribute_composes_with_an_l1_that_emits_neither() {
    let gpu = Gpu::headless().expect("no GPU available");
    let l1 = compile(&narrow_l1("position"));
    let l4 = compile(L4);
    Set::build(&gpu.device, &gpu.queue, &l1, &l4, CAPACITY, 1)
        .expect("`velocity` and `age` both have a derivation rule");
}

// ---------------------------------------------------------------------------
// Substepping: the simulation state at a given `t` must not depend on how many
// frames it took to get there. That is the entire reason `steps` exists.
// ---------------------------------------------------------------------------

#[test]
fn two_frames_of_one_step_land_where_one_frame_of_two_steps_does() {
    let gpu = Gpu::headless().expect("no GPU available");

    // Twenty steps, not two: the interesting failures are the ones that need
    // a while to show. A `t` accumulated as a running float sum agrees with a
    // computed one for the first step or two and drifts apart after that, so a
    // short horizon here passes while the property does not hold.
    let mut split = build(&gpu, CAPACITY, 19274);
    for _ in 0..19 {
        frame(&gpu, &mut split, 1);
    }
    let split = frame(&gpu, &mut split, 1);

    let mut merged = build(&gpu, CAPACITY, 19274);
    for _ in 0..9 {
        frame(&gpu, &mut merged, 2);
    }
    let merged = frame(&gpu, &mut merged, 2);

    assert_eq!(
        split, merged,
        "a frame rate drop changed the simulation rather than the frame count"
    );
}

#[test]
fn zero_steps_renders_the_previous_frame_unchanged() {
    // A paused frame, or simply a display faster than the step rate: `t` does
    // not advance, so nothing may move.
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu, CAPACITY, 19274);

    frame(&gpu, &mut set, 1);
    let before = frame(&gpu, &mut set, 1);
    let t_before = set.time();

    let paused = frame(&gpu, &mut set, 0);
    assert_eq!(set.time(), t_before, "`t` advanced on a zero-step frame");
    assert_eq!(before, paused, "the simulation advanced while paused");
}

/// The substepping invariant, on the shape that can actually detect a
/// violation of it.
///
/// `static_shell` computes `position` as a closed-form function of `t`, so
/// only the final `t` reaches the buffer and any intermediate value is
/// overwritten. That makes it blind to *when* the substeps happened: it passes
/// whether `t` advances once per frame or once per step. This procedure
/// accumulates instead, and the term it accumulates depends on `t`, so the sum
/// records every instant it was evaluated at.
///
/// Two steps in one frame with `t` held constant across them gives `2·t₂·dt`;
/// two frames of one step gives `t₁·dt + t₂·dt`. Those differ, visibly, and no
/// amount of care elsewhere recovers it — which is why `t` is per substep.
#[test]
fn an_accumulating_procedure_reading_t_is_substep_invariant() {
    const ACCUM: &str = r#"
proc accumulate {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  emit position, velocity, age

  element {
    position = position + vec3(t, 0.5, -t) * dt * 0.05;
    velocity = vec3(0.0, 0.0, 0.0);
    age      = age + dt;
  }
}
"#;
    let gpu = Gpu::headless().expect("no GPU available");
    let l1 = compile(ACCUM);
    let l4 = compile(L4);
    let make = || {
        let mut set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, CAPACITY, 19274)
            .expect("the pair is compatible");
        set.resize(&gpu.device, WIDTH, HEIGHT);
        set
    };

    // 21 steps each, reached two ways: twenty-one frames of one, and seven
    // frames of three.
    let mut split = make();
    for _ in 0..20 {
        frame(&gpu, &mut split, 1);
    }
    let split = frame(&gpu, &mut split, 1);

    let mut merged = make();
    for _ in 0..6 {
        frame(&gpu, &mut merged, 3);
    }
    let merged = frame(&gpu, &mut merged, 3);

    assert_eq!(
        split.len(),
        merged.len(),
        "the two runs did not even render the same size"
    );
    assert_eq!(
        split, merged,
        "an accumulating procedure saw a different clock under a different frame rate"
    );
}

/// **A validation error is a diagnostic, not a dead process.**
///
/// wgpu's default answer to one is an uncaptured-error handler that panics the
/// thread that made the call — which on the swap worker is recoverable and at
/// startup is the process. `Set::build_many` runs inside a validation error
/// scope, and per the WebGPU rules an error a scope captures is not reported to
/// the uncaptured handler, so it comes back as a value.
///
/// **The fixture asks for something no `.kir` can express**, deliberately: this
/// is the net under the check pass, and a test that reached it through a
/// checker hole would stop testing the net the moment the hole was closed. Five
/// such holes were found in one milestone, each a process death before it was a
/// refusal, and the net is what makes the sixth cost a message instead.
#[test]
fn a_validation_error_at_build_is_returned_rather_than_fatal() {
    let gpu = Gpu::headless().expect("no GPU available");

    // A capacity beyond any device's buffer limit. The checker cannot refuse
    // this — `capacity` is a Set-level dial and the limit is the device's — and
    // it is exactly the shape a driver answers with a validation error.
    let l1 = compile(&narrow_l1("position").replace("[1024, 262144] = 4096", "[1, 4294967295] = 4096"));
    let l4 = compile(
        &L4_UNSATISFIABLE
            .replace("consumes position, normal, uv", "consumes position")
            .replace("position + normal * 0.0", "position")
            .replace("4.0 + uv.x * 0.0", "4.0"),
    );

    let result = Set::build(&gpu.device, &gpu.queue, &l1, &l4, u32::MAX, 1);
    let err = match result {
        Ok(_) => panic!("a capacity of u32::MAX is past every device and was accepted"),
        Err(e) => e.to_string(),
    };
    assert!(
        err.contains("this device refused") || err.contains("outside the range"),
        "a device refusal has to arrive as a message: {err}"
    );
}
