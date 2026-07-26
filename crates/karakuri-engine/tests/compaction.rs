//! Headless correctness tests for order-preserving stream compaction.
//!
//! Every test goes through `Gpu::headless()` and reads results back off the
//! GPU — there is no CPU-side scan to fall back on, so these are the only
//! check that the multi-pass WGSL actually computes the exclusive prefix
//! sum the module doc promises.

use karakuri_codegen::layout::WORKGROUP_SIZE;
use karakuri_engine::{Compaction, Gpu};

/// Uploads `alive` (one flag per element) into `compaction`'s alive buffer,
/// matching the engine's `array<vec4<u32>>` attribute layout: one vec4 per
/// element, flag in `.x`, the rest padding.
fn upload_alive(gpu: &Gpu, compaction: &Compaction, alive: &[u32]) {
    let packed: Vec<[u32; 4]> = alive.iter().map(|&a| [a, 0, 0, 0]).collect();
    gpu.queue.write_buffer(compaction.alive_buffer(), 0, bytemuck::cast_slice(&packed));
}

/// Copies `buffer[..len_bytes]` into a mappable staging buffer and reads it
/// back synchronously. Only ever used from tests, never from `record`.
fn read_back(gpu: &Gpu, buffer: &wgpu::Buffer, len_bytes: u64) -> Vec<u8> {
    let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: len_bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, len_bytes);
    gpu.queue.submit([encoder.finish()]);

    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
    let out = data.to_vec();
    drop(data);
    staging.unmap();
    out
}

fn read_dest(gpu: &Gpu, compaction: &Compaction, capacity: u32) -> Vec<u32> {
    let bytes = read_back(gpu, compaction.dest_buffer(), u64::from(capacity) * 4);
    bytemuck::cast_slice::<u8, u32>(&bytes).to_vec()
}

/// `(workgroups_x, workgroups_y, workgroups_z, live_count)`.
fn read_indirect(gpu: &Gpu, compaction: &Compaction) -> (u32, u32, u32, u32) {
    let bytes = read_back(gpu, compaction.indirect_buffer(), 16);
    let words: &[u32] = bytemuck::cast_slice(&bytes);
    (words[0], words[1], words[2], words[3])
}

/// The obvious sequential computation this whole module exists to replace on
/// the GPU: `dest[i]` is how many alive flags precede `i`.
fn cpu_exclusive_scan(alive: &[u32]) -> (Vec<u32>, u32) {
    let mut dest = Vec::with_capacity(alive.len());
    let mut running = 0u32;
    for &a in alive {
        dest.push(running);
        if a != 0 {
            running += 1;
        }
    }
    (dest, running)
}

/// Runs a fresh `Compaction` over `alive` and returns `(dest[0..alive.len()],
/// live_count)`. A fresh instance's first `record` scans everything, since
/// `Compaction::new` seeds the previous-live-range bound at `capacity`.
fn run(gpu: &Gpu, alive: &[u32]) -> (Vec<u32>, u32) {
    let capacity = alive.len() as u32;
    let compaction = Compaction::new(&gpu.device, capacity);
    upload_alive(gpu, &compaction, alive);

    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    compaction.record(&mut encoder);
    gpu.queue.submit([encoder.finish()]);

    let dest = read_dest(gpu, &compaction, capacity);
    let (_, _, _, live_count) = read_indirect(gpu, &compaction);
    (dest, live_count)
}

/// xorshift32, seeded, so "pseudo-random" tests are reproducible rather than
/// clock-derived — the determinism invariant applies to the tests too.
fn xorshift32(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

fn assert_matches_cpu_reference(label: &str, gpu: &Gpu, alive: &[u32]) {
    let (expected_dest, expected_live) = cpu_exclusive_scan(alive);
    let (actual_dest, actual_live) = run(gpu, alive);
    assert_eq!(actual_live, expected_live, "{label}: live count");
    assert_eq!(actual_dest, expected_dest, "{label}: destinations");
}

#[test]
fn all_alive_matches_cpu_reference() {
    let gpu = Gpu::headless().expect("no GPU available");
    let alive = vec![1u32; 1000];
    assert_matches_cpu_reference("all alive", &gpu, &alive);
}

#[test]
fn all_dead_matches_cpu_reference() {
    let gpu = Gpu::headless().expect("no GPU available");
    let alive = vec![0u32; 1000];
    assert_matches_cpu_reference("all dead", &gpu, &alive);
}

#[test]
fn alternating_matches_cpu_reference() {
    let gpu = Gpu::headless().expect("no GPU available");
    let alive: Vec<u32> = (0..1000).map(|i| (i % 2) as u32).collect();
    assert_matches_cpu_reference("alternating", &gpu, &alive);
}

#[test]
fn single_survivor_at_end_matches_cpu_reference() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut alive = vec![0u32; 1000];
    *alive.last_mut().unwrap() = 1;
    assert_matches_cpu_reference("single survivor at end", &gpu, &alive);
}

#[test]
fn pseudo_random_matches_cpu_reference() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut state = 88172645463325252u64 as u32; // fixed seed, not a clock
    let alive: Vec<u32> = (0..50_000).map(|_| xorshift32(&mut state) & 1).collect();
    assert_matches_cpu_reference("pseudo-random", &gpu, &alive);
}

// --- multi-pass boundary sizes ---
// WORKGROUP_SIZE is 64: one workgroup covers 64 elements, one level of block
// sums covers 64*64 = 4096, and the full 3-level pyramid the spec's default
// capacity exercises covers 64^3 = 262144 exactly.

fn pseudo_random_pattern(n: u32, seed: u32) -> Vec<u32> {
    let mut state = seed;
    (0..n).map(|_| xorshift32(&mut state) & 1).collect()
}

#[test]
fn exactly_one_workgroup() {
    let gpu = Gpu::headless().expect("no GPU available");
    let alive = pseudo_random_pattern(WORKGROUP_SIZE, 1);
    assert_matches_cpu_reference("exactly one workgroup", &gpu, &alive);
}

#[test]
fn one_more_than_one_workgroup() {
    let gpu = Gpu::headless().expect("no GPU available");
    let alive = pseudo_random_pattern(WORKGROUP_SIZE + 1, 2);
    assert_matches_cpu_reference("one more than one workgroup", &gpu, &alive);
}

#[test]
fn exactly_a_full_block_sum_tile() {
    let gpu = Gpu::headless().expect("no GPU available");
    let alive = pseudo_random_pattern(WORKGROUP_SIZE * WORKGROUP_SIZE, 3);
    assert_matches_cpu_reference("exactly a full block-sum tile", &gpu, &alive);
}

#[test]
fn two_hundred_sixty_two_thousand_one_hundred_forty_four() {
    let gpu = Gpu::headless().expect("no GPU available");
    let alive = pseudo_random_pattern(262_144, 4);
    assert_matches_cpu_reference("262144", &gpu, &alive);
}

// --- order preservation ---

#[test]
fn surviving_destinations_are_strictly_increasing() {
    // The property the whole module exists for: the sequence of surviving
    // indices' destinations must be strictly increasing, so the live set is
    // always a stable, order-preserving subsequence.
    let gpu = Gpu::headless().expect("no GPU available");
    let alive = pseudo_random_pattern(50_000, 99);
    let (dest, live_count) = run(&gpu, &alive);

    let mut last_dest: Option<u32> = None;
    let mut survivors_seen = 0u32;
    for (i, &a) in alive.iter().enumerate() {
        if a == 0 {
            continue;
        }
        let d = dest[i];
        assert!(d < live_count, "destination {d} out of range at index {i}");
        if let Some(prev) = last_dest {
            assert!(d > prev, "destinations not strictly increasing at index {i}: {prev} -> {d}");
        }
        last_dest = Some(d);
        survivors_seen += 1;
    }
    assert_eq!(survivors_seen, live_count, "every survivor must get a distinct destination");
}

// --- live count into the indirect args buffer ---

#[test]
fn indirect_args_workgroup_count_covers_the_live_count() {
    let gpu = Gpu::headless().expect("no GPU available");
    let alive = pseudo_random_pattern(10_000, 7);
    let expected_live: u32 = alive.iter().sum();

    let capacity = alive.len() as u32;
    let compaction = Compaction::new(&gpu.device, capacity);
    upload_alive(&gpu, &compaction, &alive);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    compaction.record(&mut encoder);
    gpu.queue.submit([encoder.finish()]);

    let (wg_x, wg_y, wg_z, live_count) = read_indirect(&gpu, &compaction);
    assert_eq!(live_count, expected_live, "live count must match the number of alive flags");
    assert_eq!(wg_y, 1);
    assert_eq!(wg_z, 1);
    assert_eq!(wg_x, live_count.div_ceil(WORKGROUP_SIZE), "workgroup count must cover live_count exactly");
}

// --- cost, measured on the GPU ---
//
// "Any change touching performance comes with a GPU-timestamp measurement"
// (README, Working style). `karakuri-engine::Probe` is another agent's
// work in progress and not this module's to depend on, so this brackets
// `Compaction::record` with the same two-no-op-compute-pass timestamp
// technique `probe.rs` uses, independently, to get a real number for the
// scan at full capacity rather than asserting a budget with nothing behind
// it.

#[test]
fn full_capacity_scan_gpu_timestamp() {
    let gpu = Gpu::headless().expect("no GPU available");
    if !gpu.timestamps {
        eprintln!("skipping: adapter has no TIMESTAMP_QUERY support");
        return;
    }

    // The spec's default capacity, and the point the module doc calls out
    // as needing a real number: "Measure the scan at full capacity rather
    // than assuming it is free" (ir-spec.md, Dispatch).
    let capacity = 262_144u32;
    let alive = pseudo_random_pattern(capacity, 42);
    let compaction = Compaction::new(&gpu.device, capacity);
    upload_alive(&gpu, &compaction, &alive);

    let query_set = gpu.device.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("compaction timestamps"),
        ty: wgpu::QueryType::Timestamp,
        count: 2,
    });
    let resolve = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("compaction timestamp resolve"),
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("compaction timestamp readback"),
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let period_ns = gpu.queue.get_timestamp_period();

    // 8 back-to-back samples, first discarded as the cold-cache outlier,
    // median of the rest reported — same rationale as `Probe::SAMPLES`.
    const SAMPLES: u32 = 8;
    let mut deltas_ns = Vec::with_capacity(SAMPLES as usize - 1);
    for i in 0..SAMPLES {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("timestamp begin"),
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                query_set: &query_set,
                beginning_of_pass_write_index: Some(0),
                end_of_pass_write_index: None,
            }),
        });
        compaction.record(&mut encoder);
        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("timestamp end"),
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                query_set: &query_set,
                beginning_of_pass_write_index: None,
                end_of_pass_write_index: Some(1),
            }),
        });
        encoder.resolve_query_set(&query_set, 0..2, &resolve, 0);
        encoder.copy_buffer_to_buffer(&resolve, 0, &readback, 0, 16);
        gpu.queue.submit([encoder.finish()]);

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device.poll(wgpu::PollType::Wait).expect("poll");
        let data = slice.get_mapped_range();
        let ticks: Vec<u64> =
            data.chunks_exact(8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).collect();
        drop(data);
        readback.unmap();

        if i > 0 {
            deltas_ns.push(ticks[1].saturating_sub(ticks[0]) as f64 * f64::from(period_ns));
        }
    }
    deltas_ns.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_ns = deltas_ns[deltas_ns.len() / 2];
    eprintln!(
        "compaction scan at capacity={capacity}: median {:.3} ms over {} samples (period_ns={period_ns})",
        median_ns / 1_000_000.0,
        SAMPLES - 1,
    );

    assert!(median_ns.is_finite() && median_ns >= 0.0, "median_ns = {median_ns}");
}

// --- previous live range, not whole capacity ---

#[test]
fn a_second_scan_ignores_stale_data_past_the_previous_live_count() {
    // The compaction restricts itself to the previous frame's live range.
    // Simulate that: after the first scan shrinks the live count, poke
    // "alive" flags into the alive buffer *past* the new live count without
    // telling compaction the range grew — a second `record` call must still
    // ignore them, because that memory is exactly the kind of stale data
    // the module doc says this restriction exists for.
    let gpu = Gpu::headless().expect("no GPU available");
    let capacity = 1000u32;
    // Only the first 200 are alive; the rest start dead.
    let mut alive = vec![0u32; capacity as usize];
    for a in alive.iter_mut().take(200) {
        *a = 1;
    }

    let compaction = Compaction::new(&gpu.device, capacity);
    upload_alive(&gpu, &compaction, &alive);

    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    compaction.record(&mut encoder);
    gpu.queue.submit([encoder.finish()]);
    let (_, _, _, live_after_first) = read_indirect(&gpu, &compaction);
    assert_eq!(live_after_first, 200);

    // Now mark elements at [500, 600) alive too — well past the live range
    // compaction now believes in (200) — without updating that belief.
    for a in alive.iter_mut().take(600).skip(500) {
        *a = 1;
    }
    upload_alive(&gpu, &compaction, &alive);

    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    compaction.record(&mut encoder);
    gpu.queue.submit([encoder.finish()]);
    let (_, _, _, live_after_second) = read_indirect(&gpu, &compaction);

    // If the scan had (wrongly) covered the whole capacity, this would be
    // 300. Restricted to the previous live range (200), the newly-alive
    // [500, 600) entries are outside it and must not be counted.
    assert_eq!(live_after_second, 200, "scan must ignore flags past the previous live range");
}
