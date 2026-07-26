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

use karakuri_engine::{Gpu, Present, Set, VideoSource};
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
    set.resize(WIDTH, HEIGHT);
    set
}

fn frame(gpu: &Gpu, set: &mut Set, steps: u8) -> Vec<u16> {
    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, WIDTH, HEIGHT);
    set.prepare(&gpu.queue, steps);

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
    assert_eq!(set.live_count(), CAPACITY, "a spawn-less procedure is full");

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

// ---------------------------------------------------------------------------
// Substepping: the simulation state at a given `t` must not depend on how many
// frames it took to get there. That is the entire reason `steps` exists.
// ---------------------------------------------------------------------------

#[test]
fn two_frames_of_one_step_land_where_one_frame_of_two_steps_does() {
    let gpu = Gpu::headless().expect("no GPU available");

    let mut split = build(&gpu, CAPACITY, 19274);
    frame(&gpu, &mut split, 1);
    let split = frame(&gpu, &mut split, 1);

    let mut merged = build(&gpu, CAPACITY, 19274);
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
