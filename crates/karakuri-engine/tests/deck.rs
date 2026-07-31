//! The deck: several Sets resident, one to four composited.
//!
//! What is asserted here is what `docs/roadmap.md`'s M2 promises and what the
//! rest of the system will be built on top of, in the order the module doc on
//! `deck.rs` argues them:
//!
//! - a deck of one is a bare Set, **bit for bit**, so every single-Set
//!   expectation elsewhere in this repository still means something;
//! - gain is linear, per slot, and applied to that slot's own target before
//!   the sum rather than to the sum;
//! - `Allocated` keeps its state — a slot taken off air does not advance and
//!   resumes where it stopped;
//! - the same ticks and the same seeds composite to the same pixels;
//! - a hot swap in one slot is invisible to every other slot.
//!
//! The frame guard is asserted where it lives: a `compile_fail` doc test on
//! `Deck::begin_frame`, because a claim about what the borrow checker refuses
//! is worth exactly what a compiler says about it and nothing that can be
//! written here is the same claim.
//!
//! Everything is a pixel comparison against a readback of the mix, and most of
//! them are exact. That is deliberate: `Rgba16Float` in and `Rgba16Float` out
//! with a weight of 1.0 has no rounding in it anywhere, so "close enough"
//! would be hiding a real defect rather than tolerating a real error. The one
//! test that cannot be exact says why.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use karakuri_engine::deck::{Deck, Residency};
use karakuri_engine::swap::{Event, HotSwap, Request};
use karakuri_engine::{Gpu, Present, Set, VideoSource};
use karakuri_ir::typed::Checked;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
const CAPACITY: u32 = 4096;

/// Two seeds, so that two slots hold visibly different material and a mix of
/// them is not the same picture twice.
const SEED_A: u32 = 19274;
const SEED_B: u32 = 88888;

/// The capacity a swapped-in Set is built at, differing from [`CAPACITY`] so
/// that which Set is live in a slot is observable from outside.
const SWAPPED: u32 = 8192;

/// A budget no frame in these tests will come near: they are about the deck,
/// not about the watchdog, and a rollback firing in the middle of one would
/// be measuring the host's mood.
const GENEROUS_MS: f32 = 10_000.0;

const PATIENCE: Duration = Duration::from_secs(30);

const L1: &str = r#"
proc static_shell {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5

  emit position, age

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = rot_y(sphere_point(u, v) * radius, t * 0.3);
    age      = age + dt;
  }
}
"#;

const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

/// The same shape, rendering a NaN. `sqrt` of a negative is a procedure that
/// parses, type-checks, costs, compiles and runs — nothing in the pipeline
/// rejects it, and a generated L4 reaches this by dividing by a parameter or
/// normalizing a zero vector as easily as by this. What a slot holding one
/// must not be able to do is reach a mix it was faded out of.
const L4_NAN: &str = r#"
proc nan_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    let d   = length(point_coord * 2.0 - 1.0);
    let bad = sqrt(0.0 - exposure);
    color = vec4(vec3(bad, bad, bad), max(0.0, 1.0 - d));
  }
}
"#;

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

fn build(gpu: &Gpu, seed: u32, capacity: u32) -> Set {
    build_with(gpu, L4, seed, capacity)
}

fn build_with(gpu: &Gpu, l4: &str, seed: u32, capacity: u32) -> Set {
    let mut set = Set::build(
        &gpu.device,
        &gpu.queue,
        &compile(L1),
        &compile(l4),
        capacity,
        seed,
    )
    .expect("the pair is compatible and the capacity is in range");
    set.resize(WIDTH, HEIGHT);
    set
}

/// A deck of fixed Sets — no worker, so nothing can ever swap and the run is a
/// pure function of the ticks it is given. Every test but the last one uses
/// this, because a build landing halfway through would be a second variable.
fn deck_of(gpu: &Gpu, seeds: &[u32]) -> Deck {
    deck_of_at(gpu, seeds, WIDTH, HEIGHT)
}

fn deck_of_at(gpu: &Gpu, seeds: &[u32], width: u32, height: u32) -> Deck {
    let swaps = seeds
        .iter()
        .map(|&seed| HotSwap::fixed(build(gpu, seed, CAPACITY)))
        .collect();
    Deck::new(&gpu.device, swaps, width, height)
}

/// One frame, shaped the way a caller has to shape it: the guard owns the
/// encoder, so there is no other shape available.
///
/// The `poll` afterwards is the harness standing in for vsync, exactly as in
/// `tests/hot_swap.rs`: it bounds a headless loop that would otherwise queue
/// thousands of command buffers ahead of the GPU. It is the submit-and-wait
/// the render thread must never do, and it is not inside the frame.
fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present, steps: u8) {
    let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
    f.render(present.hdr_view(), present.size(), steps);
    f.finish();
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
}

/// One frame of a bare Set, the way `karakuri-cli` and every earlier test
/// record one. The comparison in
/// [`a_deck_of_one_is_a_bare_set_bit_for_bit`] is only worth something if this
/// is the *old* path rather than a second spelling of the new one.
fn bare_frame(gpu: &Gpu, set: &mut Set, present: &Present, steps: u8) {
    set.prepare(&gpu.queue, steps);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    set.render(&mut encoder, present.hdr_view(), steps);
    gpu.queue.submit([encoder.finish()]);
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
}

/// The raw `f16` bits of an `Rgba16Float` texture, four per texel. Raw rather
/// than decoded so that an exact comparison is a comparison of bits and not of
/// two float expressions that happen to agree.
fn readback(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u16> {
    let (width, height) = (texture.width(), texture.height());
    let bytes_per_row = width * 8;
    assert_eq!(bytes_per_row % 256, 0, "row pitch must be 256-byte aligned");

    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("deck readback"),
        size: u64::from(bytes_per_row * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
    let out = data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    drop(data);
    buffer.unmap();
    out
}

/// `f16` bits to `f32`, for the one test that has to do arithmetic on what it
/// read back rather than compare it. Written out rather than pulled in as a
/// dependency: it is fifteen lines and this is the only caller.
fn f16(bits: u16) -> f32 {
    let sign = f32::from_bits(u32::from(bits & 0x8000) << 16);
    let exponent = (bits >> 10) & 0x1f;
    let mantissa = bits & 0x03ff;
    let magnitude = match exponent {
        // Subnormal, including zero.
        0 => f32::from(mantissa) * 2.0f32.powi(-24),
        // Infinity or NaN, told apart by the mantissa. An HDR target may
        // legitimately hold either; the tests that call this either assert it
        // does not, or are about what happens when it does.
        0x1f if mantissa == 0 => f32::INFINITY,
        0x1f => f32::NAN,
        e => (1.0 + f32::from(mantissa) / 1024.0) * 2.0f32.powi(i32::from(e) - 15),
    };
    if sign.is_sign_negative() {
        -magnitude
    } else {
        magnitude
    }
}

fn decode(pixels: &[u16]) -> Vec<f32> {
    pixels.iter().copied().map(f16).collect()
}

fn lit(pixels: &[u16]) -> usize {
    pixels
        .chunks_exact(4)
        .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
        .count()
}

/// Simulation steps a Set has taken, recovered as an integer. `Set::time` is
/// `steps * dt`, and comparing that against a float expression meaning the
/// same thing is exactly the last-bit trap `Set` derives `t` from an integer
/// counter to avoid — see `tests/hot_swap.rs`, which does this for the same
/// reason.
fn steps_taken(set: &Set) -> u64 {
    (set.time() * 60.0).round() as u64
}

// ---------------------------------------------------------------------------

/// **A deck of one behaves exactly as a bare Set does today.**
///
/// The load-bearing test of the whole slice, and the reason it asserts bit
/// equality rather than similarity: every single-Set expectation in
/// `tests/generated.rs`, `tests/lifecycle.rs` and `tests/hot_swap.rs` is about
/// the path a bare Set takes, and they only keep meaning anything about the
/// deck if the deck reproduces that path exactly. It can be exact — the mix
/// reads the texel under the fragment with `textureLoad`, multiplies by a
/// weight of exactly 1.0, and writes an `f16` that came from an `f16`.
///
/// A failure here is not a tolerance to widen. It means the mix is filtering,
/// or resampling, or applying something it should not.
#[test]
fn a_deck_of_one_is_a_bare_set_bit_for_bit() {
    let gpu = Gpu::headless().expect("no GPU available");

    let bare_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut bare = build(&gpu, SEED_A, CAPACITY);
    for _ in 0..12 {
        bare_frame(&gpu, &mut bare, &bare_present, 1);
    }
    let expected = readback(&gpu, bare_present.hdr_texture());

    let deck_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A]);
    for _ in 0..12 {
        frame(&gpu, &mut deck, &deck_present, 1);
    }
    let mixed = readback(&gpu, deck_present.hdr_texture());

    assert!(
        lit(&expected) > 100,
        "the bare Set drew nothing, so this test would pass on two black frames"
    );
    // And it has to be a *bright* frame, not merely a non-empty one. `README`'s
    // Color invariant is that values above 1.0 are expected and are what feeds
    // bloom, so a mix that only agreed on [0, 1] would be agreeing on the
    // uninteresting half. Asserted rather than assumed, because the material
    // this test renders is free to get dimmer later and take the property with
    // it silently.
    let brightest = decode(&expected).into_iter().fold(0.0f32, f32::max);
    assert!(
        brightest > 1.0,
        "the bare Set peaked at {brightest}, so bit equality was only checked \
         below 1.0 — the range this pipeline is HDR for is untested"
    );
    assert_eq!(
        mixed, expected,
        "a deck of one slot at unity gain is not the bare Set it composites"
    );
    assert_eq!(steps_taken(deck.slot(0).set()), 12);
}

/// **A slot faded to silence cannot take the mix with it.**
///
/// Zero gain has to be a *skip*, not a multiply by zero, for the same reason
/// `Allocated` is: `0.0 * x` is zero only for finite `x`. A slot's own target
/// is allowed to hold a NaN — `sqrt` of a negative is a procedure that passes
/// every stage of this pipeline — and one multiplied by a zero fader would
/// otherwise put a NaN in every channel of the composite, wiping out every
/// other slot.
///
/// The comparison is against the same deck with that slot `Allocated`, which
/// is the path that was already exact, so this asserts the two ways of
/// silencing a slot agree.
#[test]
fn a_slot_faded_to_silence_cannot_take_the_mix_with_it() {
    let gpu = Gpu::headless().expect("no GPU available");

    let run = |silence: Residency, gain: f32| -> Vec<u16> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = Deck::new(
            &gpu.device,
            vec![
                HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                HotSwap::fixed(build_with(&gpu, L4_NAN, SEED_B, CAPACITY)),
            ],
            WIDTH,
            HEIGHT,
        );
        deck.set_residency(1, silence);
        deck.set_gain(1, gain);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        if silence == Residency::Live {
            // The faded slot is only interesting if its target really does
            // hold a NaN. (Parked, it renders nothing at all, so its target is
            // the black it was cleared to and there is nothing to check.)
            let own = readback(&gpu, deck.slot_target(1));
            assert!(
                decode(&own).iter().any(|v| v.is_nan()),
                "the NaN slot rendered no NaN, so this test is asserting nothing"
            );
        }
        readback(&gpu, present.hdr_texture())
    };

    // Off air: skipped by residency, and exact.
    let parked = run(Residency::Allocated, 1.0);
    // On air at a zero fader: must be the same picture.
    let faded = run(Residency::Live, 0.0);

    assert!(lit(&parked) > 100, "the surviving slot drew nothing");
    let nans = decode(&faded).iter().filter(|v| v.is_nan()).count();
    assert_eq!(
        faded, parked,
        "a slot at gain 0.0 reached the mix ({nans} NaN channels), while the same \
         slot taken off air did not"
    );
}

/// **Two slots at gain 1.0 and 0.0 render what slot 0 alone renders.**
///
/// Exact, for the same reason as above with one addition: `acc + 0.0 * x` is
/// `acc` for every finite `x`, so a silenced slot contributes nothing at all
/// rather than something below a threshold.
///
/// Note what is *not* silenced: the slot is still `Live`, so it is still
/// stepped and still rendered into its own target. Gain is a mixer fader, not
/// a residency level, and conflating the two is how a fader move would come to
/// cost a simulation.
#[test]
fn a_slot_at_zero_gain_contributes_nothing_to_the_mix() {
    let gpu = Gpu::headless().expect("no GPU available");

    let alone_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut alone = deck_of(&gpu, &[SEED_A]);
    for _ in 0..12 {
        frame(&gpu, &mut alone, &alone_present, 1);
    }
    let expected = readback(&gpu, alone_present.hdr_texture());

    let both_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut both = deck_of(&gpu, &[SEED_A, SEED_B]);
    both.set_gain(1, 0.0);
    for _ in 0..12 {
        frame(&gpu, &mut both, &both_present, 1);
    }
    let mixed = readback(&gpu, both_present.hdr_texture());

    assert_eq!(
        mixed, expected,
        "a slot at zero gain reached the mix anyway"
    );
    // The silenced slot ran regardless: gain is not residency.
    assert_eq!(steps_taken(both.slot(1).set()), 12);
    assert_eq!(both.live_slots(), 2);
}

/// **Each Live slot renders into its own HDR target**, and the mix is a sum of
/// exactly those targets.
///
/// This is the decision the module doc defends — additive-only would allow one
/// shared target, blend modes and masks will not — so it is worth an assertion
/// rather than only a comment. Slot 0's own target is checked against what a
/// deck of one holding the same Set mixes, which is the same picture by
/// definition if and only if the slot rendered alone into somewhere of its
/// own; two Sets sharing a target would have summed there instead, and slot
/// 0's target would hold the sum.
#[test]
fn each_live_slot_renders_into_its_own_target() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
    for _ in 0..12 {
        frame(&gpu, &mut deck, &present, 1);
    }
    let first = readback(&gpu, deck.slot_target(0));
    let second = readback(&gpu, deck.slot_target(1));

    assert!(lit(&first) > 100 && lit(&second) > 100, "a slot drew nothing");
    assert_ne!(
        first, second,
        "two slots at different seeds hold the same target contents"
    );

    let solo_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut solo = deck_of(&gpu, &[SEED_A]);
    for _ in 0..12 {
        frame(&gpu, &mut solo, &solo_present, 1);
    }
    assert_eq!(
        first,
        readback(&gpu, solo_present.hdr_texture()),
        "slot 0's target holds something other than slot 0's own render"
    );
}

/// A resize reallocates every slot target and rebinds the mix, and the result
/// is the deck it would have been at that size all along.
///
/// The failure this catches is a bind group left pointing at the old,
/// differently sized textures: `textureLoad` out of range is defined to return
/// zero rather than to fault, so the symptom would be a mix that is correct in
/// one corner and black everywhere else. Silent, and only visible on a window
/// that has been dragged.
#[test]
fn resizing_the_deck_reallocates_and_rebinds() {
    // 512 * 8 bytes is a 256-byte-aligned row, which the readback needs, and a
    // different aspect ratio from 256x256, which puts the Sets' cameras on the
    // new viewport as well as the targets.
    const WIDE: u32 = 512;
    const TALL: u32 = 256;

    let gpu = Gpu::headless().expect("no GPU available");

    let grown_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDE, TALL);
    let mut grown = deck_of(&gpu, &[SEED_A, SEED_B]);
    grown.resize(&gpu.device, WIDE, TALL);
    for _ in 0..12 {
        frame(&gpu, &mut grown, &grown_present, 1);
    }
    let after_resize = readback(&gpu, grown_present.hdr_texture());

    let native_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDE, TALL);
    let mut native = deck_of_at(&gpu, &[SEED_A, SEED_B], WIDE, TALL);
    for _ in 0..12 {
        frame(&gpu, &mut native, &native_present, 1);
    }

    assert!(lit(&after_resize) > 100, "the resized deck drew nothing");
    assert_eq!(
        after_resize,
        readback(&gpu, native_present.hdr_texture()),
        "a resized deck is not the deck it would have been at that size"
    );
    assert_eq!(grown.slot_target(0).width(), WIDE);
    assert_eq!(grown.slot_target(1).height(), TALL);
}

/// **Gain is linear, and applied per slot before the sum rather than to the
/// sum.**
///
/// The two are only distinguishable with more than one slot at more than one
/// gain, and they differ by an entire slot's contribution:
///
/// ```text
///   before (what this asserts):   2*A + B
///   after  (what it must not be): 2*(A + B)
/// ```
///
/// So `A` and `B` are measured on their own — same deck, same seeds, same
/// ticks, one slot silenced each time, so both are sampled at the same `t` as
/// the mix is — and the mix is checked against the first expression and
/// against the second.
///
/// This is the one comparison here that cannot be exact. The GPU sums in `f32`
/// and rounds once, to `f16`, on write; the expectation is computed in `f32`
/// from values that are already `f16`. The gap is that single rounding, which
/// is 2^-11 relative, so the tolerance is 2^-10 — tight enough that the
/// alternative hypothesis misses it by three orders of magnitude, which the
/// second half of the test asserts rather than assumes.
#[test]
fn gain_is_linear_and_applied_before_the_composite() {
    let gpu = Gpu::headless().expect("no GPU available");
    const STEPS: usize = 12;

    let run = |gains: [f32; 2]| -> Vec<f32> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_gain(0, gains[0]);
        deck.set_gain(1, gains[1]);
        for _ in 0..STEPS {
            frame(&gpu, &mut deck, &present, 1);
        }
        decode(&readback(&gpu, present.hdr_texture()))
    };

    let a = run([1.0, 0.0]);
    let b = run([0.0, 1.0]);
    let mixed = run([2.0, 1.0]);

    assert!(
        a.iter().all(|v| v.is_finite()) && b.iter().all(|v| v.is_finite()),
        "a slot rendered an infinity, which makes the arithmetic below meaningless"
    );

    // One f16 rounding of the result, and nothing else.
    const TOLERANCE: f32 = 1.0 / 1024.0;
    let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

    let mut before_misses = 0;
    let mut after_misses = 0;
    let mut worst = 0.0f32;
    for i in 0..mixed.len() {
        let before = 2.0 * a[i] + b[i];
        let after = 2.0 * (a[i] + b[i]);
        if !close(mixed[i], before) {
            before_misses += 1;
            worst = worst.max((mixed[i] - before).abs());
        }
        if !close(mixed[i], after) {
            after_misses += 1;
        }
    }

    assert_eq!(
        before_misses, 0,
        "the mix is not 2*A + B: {before_misses} of {} channels disagree, worst by {worst}",
        mixed.len()
    );
    // The second half of the claim. Without this the first half would also
    // pass on an all-black frame, or on one where B never contributed
    // anything — in either case `2*A + B` and `2*(A + B)` are the same number
    // and the test would be asserting nothing.
    assert!(
        after_misses > 100,
        "only {after_misses} channels distinguish `2*A + B` from `2*(A + B)`, so this run \
         could not have detected gain being applied to the mix instead of to the slot"
    );
}

/// **`Allocated` keeps its state.** A slot taken off air does not advance while
/// it is off, and resumes where it stopped when it comes back — it does not
/// restart, and it does not quietly catch up.
///
/// `t` is the sharpest witness available: simulation time only moves through
/// `Set::prepare`, and a slot that is not `Live` is never handed one. That is
/// the same property `swap.rs` already leans on to park an outgoing Set across
/// a watchdog window, which is why it costs nothing to have here.
///
/// This is two of the roadmap's three residency levels. `Priming` — stepping
/// hidden at a reduced rate — is the next slice and needs the budget governor
/// to be worth having.
#[test]
fn a_slot_taken_off_air_keeps_its_t_and_resumes_where_it_stopped() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);

    for _ in 0..10 {
        frame(&gpu, &mut deck, &present, 1);
    }
    assert_eq!(steps_taken(deck.slot(0).set()), 10);
    assert_eq!(steps_taken(deck.slot(1).set()), 10);

    deck.set_residency(1, Residency::Allocated);
    assert_eq!(deck.live_slots(), 1);
    assert_eq!(deck.slot_count(), 2, "going off air does not free the slot");

    // Substepped while it is away, so that "did not advance" is a claim about
    // steps rather than about frames.
    for _ in 0..10 {
        frame(&gpu, &mut deck, &present, 3);
    }
    assert_eq!(
        steps_taken(deck.slot(0).set()),
        40,
        "the on-air slot did not step normally while the other was parked"
    );
    assert_eq!(
        steps_taken(deck.slot(1).set()),
        10,
        "an Allocated slot advanced: something is calling `prepare` on it"
    );

    // What the mix shows while it is away is what the remaining slot shows.
    let off_air = readback(&gpu, present.hdr_texture());
    let solo_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut solo = deck_of(&gpu, &[SEED_A]);
    for _ in 0..10 {
        frame(&gpu, &mut solo, &solo_present, 1);
    }
    for _ in 0..10 {
        frame(&gpu, &mut solo, &solo_present, 3);
    }
    assert_eq!(
        off_air,
        readback(&gpu, solo_present.hdr_texture()),
        "an Allocated slot was still reaching the mix"
    );

    deck.set_residency(1, Residency::Live);
    for _ in 0..5 {
        frame(&gpu, &mut deck, &present, 1);
    }
    assert_eq!(
        steps_taken(deck.slot(1).set()),
        15,
        "the returning slot did not resume from where it was parked"
    );
    assert_eq!(steps_taken(deck.slot(0).set()), 45);
    assert_eq!(deck.live_slots(), 2);
}

/// **The same tick sequence and the same seeds composite to the same pixels.**
///
/// The ticks are deliberately uneven. A run of identical steps would pass even
/// if the deck were advancing slots by whatever each one felt like, since they
/// would all feel like the same thing; varying `steps` frame to frame is what
/// makes "every Live slot advances by the same `steps` from the same tick" the
/// thing being tested.
///
/// Bit equality, not similarity. Floating-point addition is not associative,
/// so a composite whose order depended on a `HashMap`, or on which slot last
/// had a build land on it, would show up here — which is the whole reason the
/// order is the slot index and the shader's sum is unrolled.
#[test]
fn the_same_ticks_and_seeds_composite_bit_identically() {
    let gpu = Gpu::headless().expect("no GPU available");
    const TICKS: [u8; 12] = [1, 2, 1, 3, 1, 1, 4, 2, 1, 3, 2, 1];

    let run = || -> Vec<u16> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B, SEED_A + 1]);
        deck.set_gain(0, 1.5);
        deck.set_gain(1, 0.75);
        deck.set_opacity(2, 0.5);
        for steps in TICKS {
            frame(&gpu, &mut deck, &present, steps);
        }
        readback(&gpu, present.hdr_texture())
    };

    let first = run();
    let second = run();
    assert!(lit(&first) > 100, "the deck drew nothing to compare");
    assert_eq!(
        first, second,
        "two runs of the same ticks and the same seeds composited differently"
    );
}

/// **Per-slot hot swap still works, and a swap in one slot does not disturb
/// another.**
///
/// A slot is the unit that gets replaced — that is what it means for each slot
/// to own its own `HotSwap` rather than for the deck to own one over all of
/// them. The neighbouring slot must come through with its `t`, its element
/// buffers and its live count untouched, exactly as the running Set does
/// through a failed build in `tests/hot_swap.rs`.
#[test]
fn a_swap_in_one_slot_leaves_the_other_slot_alone() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

    let (tx, rx) = mpsc::channel();
    let swapping = HotSwap::new(
        &gpu.device,
        &gpu.queue,
        build(&gpu, SEED_A, CAPACITY),
        GENEROUS_MS,
        Box::new(rx),
    );
    let steady = HotSwap::fixed(build(&gpu, SEED_B, CAPACITY));
    let mut deck = Deck::new(&gpu.device, vec![swapping, steady], WIDTH, HEIGHT);

    let mut frames = 0u64;
    for _ in 0..10 {
        frame(&gpu, &mut deck, &present, 1);
        frames += 1;
    }
    let neighbour_live_before = deck.slot(1).set().live_count(&gpu.device, &gpu.queue);

    tx.send(Request {
        l1: compile(L1),
        l4: compile(L4),
        capacity: SWAPPED,
        seed_salt: SEED_A,
        params: Vec::new(),
        label: "slot 0, second".to_string(),
    })
    .expect("worker alive");

    let started = Instant::now();
    let mut swapped = false;
    while !swapped {
        frame(&gpu, &mut deck, &present, 1);
        frames += 1;
        swapped = deck.events(0).any(|e| matches!(e, Event::Swapped { .. }));
        assert!(
            started.elapsed() < PATIENCE,
            "waited {PATIENCE:?} for the swap and it never landed"
        );
    }
    // Frames kept coming while the build was in flight, which is what "the
    // worker does not block the render loop" looks like from outside — and it
    // has to keep being true with N slots, since `Deck::begin_frame` polls
    // every one of them.
    assert!(
        frames > 11,
        "only {frames} frames were produced; the deck waited for the build"
    );

    assert_eq!(
        deck.slot(0).set().capacity(),
        SWAPPED,
        "the swap reported success but slot 0 is still the old Set"
    );
    // Cold, as every V1 swap is: a new procedure means new buffers. Warming
    // one out of sight is Priming, and it is not in this slice.
    assert_eq!(
        steps_taken(deck.slot(0).set()),
        1,
        "the swapped-in Set inherited a `t`"
    );

    assert_eq!(
        deck.slot(1).set().capacity(),
        CAPACITY,
        "the swap reached the neighbouring slot"
    );
    assert_eq!(
        steps_taken(deck.slot(1).set()),
        frames,
        "the neighbouring slot's clock did not advance normally across the swap"
    );
    assert_eq!(
        deck.slot(1).set().live_count(&gpu.device, &gpu.queue),
        neighbour_live_before,
        "the swap disturbed the neighbouring slot's element buffers"
    );
    assert_eq!(deck.live_slots(), 2, "the swap changed which slots are live");
    assert!(
        deck.events(1).next().is_none(),
        "the untouched slot reported an event"
    );
}

/// **The watchdog does not judge a Set that is not on screen.**
///
/// `Deck::begin_frame` gives every slot its frame boundary, off-air ones
/// included — a build has to be able to land on a slot that is not showing,
/// and retired Sets have to keep reaching the worker. What an off-air slot
/// must *not* get is a watchdog sample: it renders nothing, so the frame
/// interval the deck is producing is entirely the other slots' cost. Judging
/// against it accepts a candidate on a budget it never spent, and — with a
/// tight budget and busy neighbours, which is what this asserts because it is
/// the deterministic direction — rolls one back for cost it never caused.
///
/// The budget here is zero, so nothing can pass it. While the slot is parked
/// no verdict may arrive at all; the moment it goes Live, one must.
#[test]
fn an_off_air_slot_is_not_judged_against_its_neighbours_frames() {
    /// `WARMUP_FRAMES + JUDGE_FRAMES` in `swap.rs`, and slack. Private there,
    /// so this is a duplicate — if it drifts, the test gets weaker rather than
    /// wrong, because it would stop being enough frames for a verdict and the
    /// second half would catch that.
    const A_FULL_WINDOW: usize = 8 + 30 + 12;

    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

    let (tx, rx) = mpsc::channel();
    let parked = HotSwap::new(
        &gpu.device,
        &gpu.queue,
        build(&gpu, SEED_B, CAPACITY),
        0.0,
        Box::new(rx),
    );
    let mut deck = Deck::new(
        &gpu.device,
        vec![HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)), parked],
        WIDTH,
        HEIGHT,
    );
    deck.set_residency(1, Residency::Allocated);

    tx.send(Request {
        l1: compile(L1),
        l4: compile(L4),
        capacity: SWAPPED,
        seed_salt: SEED_B,
        params: Vec::new(),
        label: "off air".to_string(),
    })
    .expect("worker alive");

    let verdict = |deck: &mut Deck| -> Option<String> {
        deck.events(1).find_map(|e| match e {
            Event::Accepted { label, median_ms, .. } => {
                Some(format!("Accepted `{label}` at {median_ms:.3} ms"))
            }
            Event::RolledBack { label, median_ms, .. } => {
                Some(format!("RolledBack `{label}` at {median_ms:.3} ms"))
            }
            _ => None,
        })
    };

    // The build still lands on the parked slot: that is wanted, and the rest
    // of the test is about nothing else happening to it.
    let started = Instant::now();
    while deck.slot(1).set().capacity() != SWAPPED {
        frame(&gpu, &mut deck, &present, 1);
        assert!(
            verdict(&mut deck).is_none(),
            "a verdict arrived before the build even landed"
        );
        assert!(
            started.elapsed() < PATIENCE,
            "waited {PATIENCE:?} for the off-air build and it never installed"
        );
    }

    for _ in 0..A_FULL_WINDOW {
        frame(&gpu, &mut deck, &present, 1);
        if let Some(v) = verdict(&mut deck) {
            panic!(
                "the watchdog reached a verdict — {v} — on a slot that rendered nothing; \
                 the interval it measured is the neighbouring slot's cost"
            );
        }
    }
    assert_eq!(
        steps_taken(deck.slot(1).set()),
        0,
        "the parked slot was stepped"
    );

    // On air, it is judged — against a budget of zero, so it goes.
    deck.set_residency(1, Residency::Live);
    let mut on_air = None;
    for _ in 0..A_FULL_WINDOW {
        frame(&gpu, &mut deck, &present, 1);
        if let Some(v) = verdict(&mut deck) {
            on_air = Some(v);
            break;
        }
    }
    assert!(
        on_air.is_some_and(|v| v.starts_with("RolledBack")),
        "a Live slot's candidate was never judged, so the first half of this test \
         would pass on a watchdog that had simply stopped working"
    );
}

// ---------------------------------------------------------------------------
// Measured, reported.
// ---------------------------------------------------------------------------

/// What a slot costs, and what the composite costs. **Printed, not asserted.**
///
/// `README.md`'s Working style asks for a GPU-timestamp measurement on any
/// change touching the frame path, and says in the same breath that timestamps
/// do not work on the machine this was developed on — `probe.rs` documents an
/// enormous workload resolving to zero. So this is a host clock around
/// submit-and-wait, on the same terms as every other number in this
/// repository: coarse, biased high, and real. Run with
/// `cargo test -p karakuri-engine --test deck -- --nocapture --ignored`.
///
/// Three configurations at the CLI's own defaults, so the numbers are
/// comparable with the ones in `README.md` rather than being a measurement of
/// a toy: a bare Set, a deck of one, and a deck of four. Bare against deck-of-
/// one isolates the composite pass, since the simulation either side of it is
/// identical. Deck-of-one against deck-of-four is what a slot costs, which is
/// dominated by the Set and not by the mix.
///
/// `#[ignore]`d because four simulations at capacity 262144 is a real
/// workload, and `cargo test` should not be one.
#[test]
#[ignore = "a measurement, not a check; run with --ignored --nocapture"]
fn the_cost_of_a_slot_and_of_the_composite_are_measured_and_reported() {
    const CAP: u32 = 262_144;
    const W: u32 = 1280;
    const H: u32 = 720;
    const WARMUP: usize = 60;
    const MEASURED: usize = 120;

    let gpu = Gpu::headless().expect("no GPU available");

    let summarize = |label: &str, mut xs: Vec<f32>| {
        xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        eprintln!(
            "  {label:<24} n={:<4} median {:.3} ms   worst {:.3} ms",
            xs.len(),
            xs[xs.len() / 2],
            xs[xs.len() - 1]
        );
    };

    let present = Present::new(&gpu.device, Present::HDR_FORMAT, W, H);

    // Bare Set, no deck: the path every earlier test measures.
    let mut bare = Set::build(
        &gpu.device,
        &gpu.queue,
        &compile(L1),
        &compile(L4),
        CAP,
        SEED_A,
    )
    .expect("the pair is compatible");
    bare.resize(W, H);
    let mut bare_ms = Vec::new();
    for i in 0..WARMUP + MEASURED {
        let at = Instant::now();
        bare_frame(&gpu, &mut bare, &present, 1);
        if i >= WARMUP {
            bare_ms.push(at.elapsed().as_secs_f32() * 1_000.0);
        }
    }

    let deck_ms = |seeds: &[u32]| -> Vec<f32> {
        let swaps = seeds
            .iter()
            .map(|&seed| {
                let mut set = Set::build(
                    &gpu.device,
                    &gpu.queue,
                    &compile(L1),
                    &compile(L4),
                    CAP,
                    seed,
                )
                .expect("the pair is compatible");
                set.resize(W, H);
                HotSwap::fixed(set)
            })
            .collect();
        let mut deck = Deck::new(&gpu.device, swaps, W, H);
        let mut out = Vec::new();
        for i in 0..WARMUP + MEASURED {
            let at = Instant::now();
            frame(&gpu, &mut deck, &present, 1);
            if i >= WARMUP {
                out.push(at.elapsed().as_secs_f32() * 1_000.0);
            }
        }
        out
    };
    let one = deck_ms(&[SEED_A]);
    let four = deck_ms(&[SEED_A, SEED_B, SEED_A + 1, SEED_B + 1]);

    let deck_mb = 4.0 * f64::from(W) * f64::from(H) * 8.0 / 1_048_576.0;
    eprintln!(
        "\nframe times at capacity {CAP}, {W}x{H}, host clock around submit-and-wait \
         (four slot targets at 8 bytes a texel is {deck_mb:.1} MB):"
    );
    summarize("bare Set, no deck", bare_ms);
    summarize("deck of one", one);
    summarize("deck of four", four);
    eprintln!();
}

/// A mix target that is not the deck's size is refused rather than mixed.
///
/// The composite reads its sources with `textureLoad`, and an out-of-range
/// `textureLoad` is *defined* to return zero — so resizing `Present` and
/// forgetting the deck would produce a black frame, every frame, with nothing
/// logged. A panic at the call is louder than a picture that is quietly wrong,
/// and this is a programming error rather than an input error: no `.kir` and
/// no record stream can reach it.
#[test]
#[should_panic(expected = "resize both")]
fn a_mix_target_of_the_wrong_size_is_refused() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut deck = deck_of(&gpu, &[1]);
    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba8UnormSrgb, WIDTH, HEIGHT);

    // The deck moves, the target does not — the direction a window resize
    // takes if only one of the two handlers is wired.
    deck.resize(&gpu.device, WIDTH * 2, HEIGHT * 2);

    let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
    frame.render(present.hdr_view(), present.size(), 1);
}
