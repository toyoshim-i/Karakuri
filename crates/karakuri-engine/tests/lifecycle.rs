//! The element lifecycle, end to end: spawn, kill, compaction, and the
//! properties the record-stream design rests on.
//!
//! `compaction.rs` proves the scan computes an exclusive prefix sum.
//! `generated.rs` proves `.kir` text reaches the screen. Neither says
//! anything about the seam between them — that a killed element actually
//! leaves, that survivors keep their order through the compaction, that
//! spawning stops at capacity rather than writing past it, and that all of
//! that stays bit-exact under replay. That is what this file is for.
//!
//! Everything here reads state back off the GPU, which is a stall. That is
//! fine in a test and is exactly why `Set::live_count` documents itself as
//! one; nothing in this file is a model for what the frame path does.

use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

const WIDTH: u32 = 128;
const HEIGHT: u32 = 128;

/// The L4 half of every pair below. Deliberately minimal: these tests are
/// about which elements exist, not about how they look.
const L4: &str = r#"
proc plain_points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 3.0;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(1.0, 0.9, 0.7, max(0.0, 1.0 - d));
  }
}
"#;

/// Spawns at a fixed rate and kills on age. Nothing in it reads `t`, which
/// is what lets the substepping test compare a two-step frame against two
/// one-step frames without the two disagreeing about the clock — see that
/// test for why that matters.
fn emitter(spawn_rate: f32, lifetime: f32) -> String {
    format!(
        r#"
proc emitter {{
  kind     L1
  topology points
  capacity [64, 262144] = 4096

  param spawn_rate : float [0.0, 100000.0] = {spawn_rate:?}
  param lifetime   : float [0.01, 100.0]   = {lifetime:?}

  emit position, velocity, age

  spawn {{
    let u = hash1(seed);
    let v = hash1(seed + 1u);
    position = vec3(0.0, 0.0, 0.0);
    velocity = sphere_point(u, v) * 1.5;
    age      = 0.0;
  }}

  element {{
    velocity = velocity;
    position = position + velocity * dt;
    age      = age + dt;

    if age > lifetime {{
      kill();
    }}
  }}
}}
"#
    )
}

fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|errs| panic!("{}", render(&errs, src)));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|errs| panic!("{}", render(&errs, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|errs| panic!("{}", render(&errs, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
}

fn build(gpu: &Gpu, l1_src: &str, capacity: u32) -> Set {
    let l1 = compile(l1_src);
    let l4 = compile(L4);
    let mut set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, capacity, 19274).expect("compatible pair");
    set.resize(&gpu.device, WIDTH, HEIGHT);
    set
}

/// One frame, discarding the pixels. `Present` is rebuilt per call the same
/// way `generated.rs` does it — wasteful, and irrelevant to what is being
/// measured.
fn step(gpu: &Gpu, set: &mut Set, steps: u8) {
    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, WIDTH, HEIGHT);
    set.prepare(&gpu.queue, steps, &Signals::default());
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    set.render(&mut encoder, present.hdr_view(), steps);
    gpu.queue.submit([encoder.finish()]);
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
}

/// One frame, returning the rendered `Rgba16Float` texels.
fn frame(gpu: &Gpu, set: &mut Set, steps: u8) -> Vec<u16> {
    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, WIDTH, HEIGHT);
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
        wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
    let out = data.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
    drop(data);
    readback.unmap();
    out
}

/// The `seed` of every slot in `[0, live_count)`, in slot order. `seed` is
/// element identity — the one thing that survives compaction moving an
/// element between slots — so this is what an order claim has to be made
/// about.
fn live_seeds(gpu: &Gpu, set: &Set) -> Vec<u32> {
    let live = set.live_count(&gpu.device, &gpu.queue) as usize;
    let bytes = set.read_elements(&gpu.device, &gpu.queue);
    let layout = set.element_layout();
    let stride = layout.stride as usize;
    let off = layout.offset_of("seed") as usize;
    (0..live)
        .map(|i| {
            let at = i * stride + off;
            u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes of seed"))
        })
        .collect()
}

/// Every element's `position.x` in `[0, live_count)`, in slot order.
fn live_position_x(gpu: &Gpu, set: &Set) -> Vec<f32> {
    let live = set.live_count(&gpu.device, &gpu.queue) as usize;
    let bytes = set.read_elements(&gpu.device, &gpu.queue);
    let layout = set.element_layout();
    let stride = layout.stride as usize;
    let off = layout.offset_of("position") as usize;
    (0..live)
        .map(|i| {
            let at = i * stride + off;
            f32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes of position.x"))
        })
        .collect()
}

fn is_subsequence(sub: &[u32], of: &[u32]) -> bool {
    let mut it = of.iter();
    sub.iter().all(|x| it.any(|y| y == x))
}

// ---------------------------------------------------------------------------
// Kill
// ---------------------------------------------------------------------------

/// A `kill()` has to actually remove an element, and the survivors around it
/// have to keep their relative order — that is what "the live set is a
/// stable subsequence" means, and it is what makes the additive blend order
/// reproducible.
#[test]
fn a_killed_element_leaves_and_the_survivors_keep_their_order() {
    let gpu = Gpu::headless().expect("no GPU available");
    // 600 per second at dt = 1/60 is exactly 10 per step, and a two-step
    // lifetime, so after a few steps the population is in steady state and
    // something dies every step.
    let mut set = build(&gpu, &emitter(600.0, 2.0 / 60.0), 4096);

    for _ in 0..6 {
        step(&gpu, &mut set, 1);
    }
    let before = live_seeds(&gpu, &set);
    assert!(before.len() > 10, "the emitter never filled up: {} live", before.len());

    step(&gpu, &mut set, 1);
    let after = live_seeds(&gpu, &set);

    // Some of `before` must be gone — the lifetime is two steps.
    let survivors: Vec<u32> = after.iter().copied().filter(|s| before.contains(s)).collect();
    assert!(
        survivors.len() < before.len(),
        "nothing died: {} before, {} of them still live",
        before.len(),
        survivors.len()
    );
    assert!(!survivors.is_empty(), "everything died at once, so this proves nothing about order");

    // ...and the ones that stayed are still in the order they were in.
    assert!(
        is_subsequence(&survivors, &before),
        "compaction reordered the survivors\nbefore: {before:?}\nafter:  {after:?}"
    );

    // Seeds are a monotone spawn ordinal, so a slot-ordered live set is
    // ascending whether or not anything died.
    assert!(after.windows(2).all(|w| w[0] < w[1]), "live seeds are not ascending: {after:?}");
}

/// The narrow version of the same claim, with no spawning to confuse it: an
/// element that kills itself is gone from the live count on a later step.
/// A procedure with `kill()` and no `spawn` block is compacted too — its
/// population can only shrink.
#[test]
fn an_element_that_kills_itself_is_gone_from_the_live_count() {
    let gpu = Gpu::headless().expect("no GPU available");
    // Every fourth element dies on its second step. `seed` is the slot index
    // for a spawn-less procedure, so this is exactly 25% of 256.
    let l1 = r#"
proc decimate {
  kind     L1
  topology points
  capacity [64, 4096] = 256

  emit position, age

  element {
    position = vec3(0.0, 0.0, 0.0);
    age      = age + dt;

    if age > 0.025 {
      if seed % 4u == 0u {
        kill();
      }
    }
  }
}
"#;
    let l4 = r#"
proc plain {
  kind  L4
  blend additive
  consumes position
  vertex   { clip = camera * vec4(position, 1.0); point_size = 2.0; }
  fragment { color = vec4(1.0, 1.0, 1.0, 1.0); }
}
"#;
    let mut set = Set::build(&gpu.device, &gpu.queue, &compile(l1), &compile(l4), 256, 1).expect("pair");
    set.resize(&gpu.device, WIDTH, HEIGHT);

    assert_eq!(set.live_count(&gpu.device, &gpu.queue), 256, "everything starts alive");

    // Reading an attribute yields the *previous* frame's value, so `age`
    // inside step k is `(k - 1) * dt`. It first exceeds 0.025 (one and a
    // half steps at dt = 1/60) during step 3, which is when `kill()` runs.
    // Step 3 still writes those elements into their compacted slot with a
    // dead flag; step 4's scan is what reclaims the slot.
    for expected in [256, 256, 256, 192] {
        step(&gpu, &mut set, 1);
        assert_eq!(
            set.live_count(&gpu.device, &gpu.queue),
            expected,
            "a killed element occupies its slot for exactly one more step"
        );
    }

    // And what is left is exactly the seeds that did not kill themselves,
    // still in ascending order.
    let seeds = live_seeds(&gpu, &set);
    assert_eq!(seeds, (0..256u32).filter(|s| s % 4 != 0).collect::<Vec<_>>());
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

/// Spawning has to fill toward capacity and then stop, not wrap around or
/// write past the end of the buffer. The GPU is what clamps — `spawn`
/// returns when its slot is at or past `capacity` — so this is the check
/// that the clamp is real rather than an assumption about the host's count.
#[test]
fn spawning_fills_toward_capacity_and_stops_there() {
    let gpu = Gpu::headless().expect("no GPU available");
    let capacity = 512u32;
    // 6000/s at dt = 1/60 is 100 per step, and nothing ever dies, so this
    // reaches 512 during the sixth step and then has nowhere to put the
    // seventh's hundred.
    let mut set = build(&gpu, &emitter(6000.0, 1000.0), capacity);

    let mut counts = Vec::new();
    for _ in 0..12 {
        step(&gpu, &mut set, 1);
        counts.push(set.live_count(&gpu.device, &gpu.queue));
    }

    assert_eq!(&counts[..5], &[100, 200, 300, 400, 500], "the fill rate is 100 per step: {counts:?}");
    assert!(
        counts[5..].iter().all(|&c| c == capacity),
        "the count must stop at capacity, not overshoot or wrap: {counts:?}"
    );

    // Every slot in the live range holds a distinct, ascending seed. A write
    // past the end that wrapped to slot 0 would show up here as a seed out
    // of order.
    let seeds = live_seeds(&gpu, &set);
    assert_eq!(seeds.len(), capacity as usize);
    assert!(seeds.windows(2).all(|w| w[0] < w[1]), "seeds are not ascending after the clamp");
}

/// **The spawn accumulator's fractional carry, asserted as a rate rather than
/// as a comparison between two runs.**
///
/// `spawn_rate * dt` is rarely a whole number of elements, so the remainder has
/// to carry into the next substep — within a frame and across frames alike — for
/// the long-run rate to come out right. Everything else in this file that
/// touches spawning compares one run against another (same ticks, same frame;
/// two frames of one step against one frame of two), and every such comparison
/// is blind to the carry being dropped, because both sides drop it. Deleting
/// `spawn_carry -= whole` in favour of `spawn_carry = 0.0` passed all 43 suites.
///
/// The sharpest case is a rate **below one element per substep**, where the
/// absence of a carry is not an inaccuracy but a total failure: `floor(0.5)` is
/// zero, every substep, forever, so a Set asked for thirty elements a second
/// emits none at all and does so silently. `1.667` is the other regime, where
/// the loss is a fifth of the material rather than all of it.
///
/// The tolerance is one element, which is the carry still in flight at the
/// moment the count is read — that is what "exact in the long run" means here,
/// and it is not slack for a rate that is merely close.
#[test]
fn the_spawn_rate_is_exact_over_many_substeps_because_the_fraction_carries() {
    let gpu = Gpu::headless().expect("no GPU available");
    const SUBSTEPS: u32 = 60;

    // Frames of two steps, so the carry is exercised both between substeps of
    // one frame and between frames — the two places a naive reset would put it.
    let live_after = |rate: f32| -> u32 {
        let mut set = build(&gpu, &emitter(rate, 1000.0), 4096);
        for _ in 0..SUBSTEPS / 2 {
            step(&gpu, &mut set, 2);
        }
        set.live_count(&gpu.device, &gpu.queue)
    };

    // Half an element per substep. Without a carry this is zero, always.
    let slow = live_after(30.0);
    assert!(
        slow > 0,
        "a rate below one element per substep emitted nothing in a whole second — \
         the fractional carry is gone, and no rate under 60/s can ever spawn"
    );
    assert!(
        slow.abs_diff(30) <= 1,
        "30 elements a second gave {slow} after one second, not 30 give or take the carry"
    );

    // Five thirds of an element per substep: the other regime, where dropping
    // the carry truncates to one and loses two elements in five.
    let fast = live_after(100.0);
    assert!(
        fast.abs_diff(100) <= 1,
        "100 elements a second gave {fast} after one second, not 100 give or take the carry"
    );
}

/// The birth-fraction correction exists so that a frame's worth of elements
/// do not all start at the same phase, and it must apply exactly once: the
/// element's *first* update runs with `dt * birth_frac`, and every update
/// after that with unscaled `dt`.
///
/// This measures it directly. The elements move at a constant velocity along
/// a known axis, so `position.x` after one step is `v * dt * frac` and after
/// two is `v * dt * (frac + 1)` — a ratio that pins the correction to one
/// step without needing to read `birth_frac` itself.
#[test]
fn the_birth_fraction_correction_expires_after_one_step() {
    let gpu = Gpu::headless().expect("no GPU available");
    // One element per step, so its birth fraction is (0 + 0.5) / 1 = 0.5,
    // exactly, with no floating-point argument to make.
    let l1 = r#"
proc drifter {
  kind     L1
  topology points
  capacity [64, 4096] = 64

  param spawn_rate : float [0.0, 1000.0] = 60.0

  emit position, velocity

  spawn {
    position = vec3(0.0, 0.0, 0.0);
    velocity = vec3(1.0, 0.0, 0.0);
  }

  element {
    velocity = velocity;
    position = position + velocity * dt;
  }
}
"#;
    let l4 = r#"
proc plain {
  kind  L4
  blend additive
  consumes position
  vertex   { clip = camera * vec4(position, 1.0); point_size = 2.0; }
  fragment { color = vec4(1.0, 1.0, 1.0, 1.0); }
}
"#;
    let mut set = Set::build(&gpu.device, &gpu.queue, &compile(l1), &compile(l4), 64, 1).expect("pair");
    set.resize(&gpu.device, WIDTH, HEIGHT);

    let dt = 1.0f32 / 60.0;
    for _ in 0..4 {
        step(&gpu, &mut set, 1);
    }

    // Four steps, one element born per step, oldest first: the element born
    // in step 1 has had three updates, the one born in step 4 has had none.
    // The one with n updates sits at `dt * (0.5 + (n - 1))`.
    let xs = live_position_x(&gpu, &set);
    assert_eq!(xs.len(), 4, "one element per step for four steps: {xs:?}");

    let expected = [dt * 2.5, dt * 1.5, dt * 0.5, 0.0];
    for (i, (&got, &want)) in xs.iter().zip(expected.iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-6,
            "element {i}: position.x is {got}, expected {want}\nall: {xs:?}\n\
             (a second scaled step would give {}, no scaling at all {})",
            dt * 2.0,
            dt * 3.0,
        );
    }
}

// ---------------------------------------------------------------------------
// Determinism and substepping, with spawning and killing in play
// ---------------------------------------------------------------------------

/// The property the whole record-stream design rests on. Compaction is
/// order-preserving precisely so that this survives spawning and killing:
/// floating-point addition is not associative, so the additive blend has to
/// combine the same elements in the same order on every run.
#[test]
fn the_same_seed_and_the_same_ticks_reproduce_the_same_frame_with_spawn_and_kill() {
    let gpu = Gpu::headless().expect("no GPU available");
    let src = emitter(5000.0, 0.5);
    let mut a = build(&gpu, &src, 4096);
    let mut b = build(&gpu, &src, 4096);

    // Long enough for the population to reach steady state, so both spawning
    // and killing are happening every step by the time the frames compared.
    for _ in 0..40 {
        step(&gpu, &mut a, 1);
        step(&gpu, &mut b, 1);
    }

    assert_eq!(
        a.live_count(&gpu.device, &gpu.queue),
        b.live_count(&gpu.device, &gpu.queue),
        "two runs of the same record stream disagree about how many elements exist"
    );
    assert_eq!(live_seeds(&gpu, &a), live_seeds(&gpu, &b), "the live sets differ");
    assert_eq!(frame(&gpu, &mut a, 1), frame(&gpu, &mut b, 1), "the same record stream produced two images");
}

/// Substepping must not change the simulation, only the frame count — and
/// that has to keep holding once spawning is in the loop. The accumulator
/// therefore advances once per *substep*, not once per frame: a frame of two
/// steps spawns the same two batches, at the same two points in the
/// integration, that two frames of one step do. Advancing it once per frame
/// would put both batches in before the second element pass, and the
/// positions below would come out short by one step's worth of travel.
///
/// The comparison covers the element buffer *and* the rendered image. `t` is
/// `steps_taken * dt` from an integer counter rather than a running float
/// sum, so twenty steps taken one at a time and ten taken two at a time reach
/// the same `t` bit for bit — and the built-in orbit camera, which is the one
/// thing here that reads `t`, therefore produces the same matrix either way.
/// An f32 sum would land the two orders two ULP apart at `dt = 1/60`, which
/// is why the pixel comparison used to be impossible at this horizon.
#[test]
fn two_frames_of_one_step_land_where_one_frame_of_two_steps_does_with_spawning() {
    let gpu = Gpu::headless().expect("no GPU available");
    let src = emitter(5000.0, 0.25);

    let mut split = build(&gpu, &src, 4096);
    for _ in 0..20 {
        step(&gpu, &mut split, 1);
    }

    let mut merged = build(&gpu, &src, 4096);
    for _ in 0..10 {
        step(&gpu, &mut merged, 2);
    }

    let split_seeds = live_seeds(&gpu, &split);
    assert!(!split_seeds.is_empty(), "nothing is alive, so this proves nothing");
    assert_eq!(split_seeds, live_seeds(&gpu, &merged), "the live sets diverged under substepping");
    assert_eq!(
        split.live_count(&gpu.device, &gpu.queue),
        merged.live_count(&gpu.device, &gpu.queue),
    );
    assert_eq!(
        split.read_elements(&gpu.device, &gpu.queue),
        merged.read_elements(&gpu.device, &gpu.queue),
        "a frame rate drop changed the simulation rather than the frame count"
    );
    assert_eq!(
        split.time(),
        merged.time(),
        "`t` is derived from a step count, so twenty steps must land exactly where ten pairs do"
    );
    assert_eq!(
        frame(&gpu, &mut split, 1),
        frame(&gpu, &mut merged, 1),
        "the pixels diverged even though the geometry did not — that would be `t`"
    );
}

/// The pixel half of the claim above, at a two-step horizon where `t` is
/// bit-identical either way (`dt + dt` and `2 * dt` round the same) — so a
/// difference here would be the simulation, not the camera. Spawning is
/// active: at this rate each of the two steps creates its own batch.
#[test]
fn substepping_leaves_the_rendered_frame_alone_where_time_agrees_exactly() {
    let gpu = Gpu::headless().expect("no GPU available");
    let src = emitter(5000.0, 0.25);

    let mut split = build(&gpu, &src, 4096);
    frame(&gpu, &mut split, 1);
    let split_frame = frame(&gpu, &mut split, 1);

    let mut merged = build(&gpu, &src, 4096);
    let merged_frame = frame(&gpu, &mut merged, 2);

    assert_eq!(split.time(), merged.time(), "the premise of this test is that `t` agrees");
    assert!(split.live_count(&gpu.device, &gpu.queue) > 100, "spawning is not active");
    assert_eq!(split_frame, merged_frame, "substepping changed the image");
}

/// A dead element keeps its slot for one step — the range still covers it
/// until the next scan reclaims it — so the draw range contains elements
/// that must not appear. If the vertex stage did not collapse them, this
/// would render the whole population rather than the live part of it.
#[test]
fn elements_killed_this_step_are_not_drawn() {
    let gpu = Gpu::headless().expect("no GPU available");
    // Everything is born at the origin and dies on its very first update, so
    // in steady state exactly half the draw range is corpses — and every one
    // of the corpses has drifted somewhere the newborns have not.
    let l1 = r#"
proc mayfly {
  kind     L1
  topology points
  capacity [64, 4096] = 4096

  param spawn_rate : float [0.0, 100000.0] = 6000.0

  emit position, age

  element {
    position = position + vec3(60.0, 0.0, 0.0) * dt;
    age      = age + dt;
    kill();
  }

  spawn {
    position = vec3(0.0, 0.0, 0.0);
    age      = 0.0;
  }
}
"#;
    let l4 = r#"
proc plain {
  kind  L4
  blend additive
  consumes position
  vertex   { clip = camera * vec4(position, 1.0); point_size = 2.0; }
  fragment { color = vec4(1.0, 1.0, 1.0, 1.0); }
}
"#;
    let mut set = Set::build(&gpu.device, &gpu.queue, &compile(l1), &compile(l4), 4096, 1).expect("pair");
    set.resize(&gpu.device, WIDTH, HEIGHT);

    for _ in 0..4 {
        step(&gpu, &mut set, 1);
    }
    // Half the range is the batch that just died one unit to the right of
    // the origin, half is the batch just spawned at it.
    assert!(set.live_count(&gpu.device, &gpu.queue) > 100, "the emitter is not running");

    let pixels = frame(&gpu, &mut set, 1);
    // The origin is dead centre; the corpses are a world unit along +x,
    // which at this camera is well to one side of it. Count lit columns
    // rather than reasoning about the exact projection: a live-only render
    // has one cluster, a render that includes the dead has two.
    let mut columns: Vec<usize> = Vec::new();
    for x in 0..WIDTH as usize {
        let lit = (0..HEIGHT as usize)
            .filter(|y| {
                let at = (y * WIDTH as usize + x) * 4;
                pixels[at] != 0
            })
            .count();
        if lit > 0 {
            columns.push(x);
        }
    }
    assert!(!columns.is_empty(), "nothing was drawn at all");
    let span = columns.last().unwrap() - columns.first().unwrap();
    assert!(
        span < 20,
        "the render spans {span} columns, which is two clusters — the elements killed this step \
         were drawn alongside the ones just spawned"
    );
}
