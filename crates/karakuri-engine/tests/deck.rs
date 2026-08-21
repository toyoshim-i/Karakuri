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
//! - `over` hides what is under it and `add` does not, which is the whole of
//!   what a blend mode buys, and `max` stacks without summing;
//! - the fader silences a slot under every mode and the level does not, which
//!   is the asymmetry `Blend::silent_at` records;
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
//! at unity gain and full opacity has no rounding in it anywhere, so "close enough"
//! would be hiding a real defect rather than tolerating a real error. The one
//! test that cannot be exact says why.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use karakuri_engine::binding::Curve;
use karakuri_engine::deck::{Blend, Deck, Mask, MaskKind, Residency};
use karakuri_engine::swap::{Event, HotSwap, Request};
use karakuri_engine::transition::{Control, Transition};
use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
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

/// **A layer that covers**: big black sprites at full coverage.
///
/// Black *and* opaque is the pair that separates the modes with nothing else
/// moving. Its colour contribution is zero under every mode — `blend additive`
/// multiplies colour by the sprite's own alpha on the way into the slot target,
/// and zero times anything is zero — so whatever the mix does with this layer
/// is entirely what it did with the coverage. Under `add` it is invisible;
/// under `over` it is a hole.
const L4_CARD: &str = r#"
proc opaque_card {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 48.0;
  }

  fragment {
    color = vec4(vec3(0.0, 0.0, 0.0) * exposure, 1.0);
  }
}
"#;

/// The ordinary material, drawn wide enough to be under the card everywhere it
/// covers. Used where a test needs the two layers to actually overlap rather
/// than to overlap wherever the seeds happened to put them.
const L4_WIDE: &str = r#"
proc wide_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 48.0;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

/// The same card, writing a coverage that is **not a coverage** — one per
/// spelling of "outside `[0, 1]`", named by what the alpha expression is.
///
/// Nothing in this pipeline stops any of them: the IR calls `color` linear RGB
/// with straight alpha, says values above 1.0 are expected, and no pass clamps
/// what a `fragment` block assigns. Each parses, type-checks, costs, compiles
/// and runs, and a generated L4 reaches all four by dividing by a parameter as
/// easily as by writing the constant. Black, so that whatever the mix does with
/// one of them is what it did with the coverage and not with the colour.
///
/// `exposure` defaults to 1.0 and every expression is written against it, so
/// none of them folds to a constant the compiler could reject before it runs.
const OVERDRAWN_ALPHA: [(&str, &str); 4] = [
    ("above one", "1.5 * exposure"),
    ("negative", "0.0 - exposure"),
    ("infinite", "exposure / 0.0"),
    ("NaN", "sqrt(0.0 - exposure)"),
];

fn overdrawn_card(alpha: &str) -> String {
    format!(
        r#"
proc overdrawn_card {{
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_size = 48.0;
  }}

  fragment {{
    color = vec4(vec3(0.0, 0.0, 0.0) * exposure, {alpha});
  }}
}}
"#
    )
}

/// **A wash over the whole frame**, for the mask tests.
///
/// Every other fixture here draws a sphere in the middle, and a mask's ends are
/// at the *edges*: a front that stops short of the corner, or a radial one that
/// stops at the inscribed circle, is invisible against material that never
/// reaches either. Four such defects walked past the first version of those
/// tests for exactly that reason.
///
/// The vertex stage ignores `position` and puts every sprite at the origin, so
/// one of them covers a target of [`MASK_SIZE`]. It still `consumes position`,
/// because an L4 is compiled against its L1's element layout and dropping the
/// attribute would change what is being tested.
const L4_WASH: &str = r#"
proc wash {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    let ignored = position;
    clip       = vec4(0.0, 0.0, 0.0, 1.0);
    point_size = 128.0;
  }

  fragment {
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, 1.0);
  }
}
"#;

/// The target the mask tests render at. Small, because [`L4_WASH`] overdraws
/// the whole frame once per element and the number that matters is coverage
/// rather than resolution.
const MASK_SIZE: u32 = 64;

/// A deck whose slot 1 covers the frame, at [`MASK_SIZE`].
fn wash_deck(gpu: &Gpu) -> Deck {
    let swaps = vec![
        HotSwap::fixed(build(gpu, SEED_A, CAPACITY)),
        HotSwap::fixed(build_with(gpu, L4_WASH, SEED_B, CAPACITY)),
    ];
    let mut deck = Deck::new(&gpu.device, swaps, MASK_SIZE, MASK_SIZE);
    deck.resize(&gpu.device, MASK_SIZE, MASK_SIZE);
    deck
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
    set.resize(&gpu.device, WIDTH, HEIGHT);
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
    // No bindings on these Sets, so the session clock is inert here and
    // nothing reads a signal; the real one belongs to the deck, and
    // `tests/binding.rs` is where it is asserted.
    set.prepare(&gpu.queue, steps, &Signals::default());
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
/// reads the texel under the fragment with `textureLoad`, adds it to a zeroed
/// accumulator at a gain and an opacity of exactly 1.0, and writes an `f16`
/// that came from an `f16`.
///
/// **The material here writes a coverage in `[0, 1]`, which is the one thing
/// this comparison assumes.** The mix saturates what it reads into that range
/// and a bare Set's target holds whatever L4 accumulated, so an L4 writing an
/// alpha of 1.5 makes the two disagree in alpha and nowhere else — see
/// `Blend::Add`. Every expectation this test exists to protect is about
/// colour.
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

/// **A slot faded to silence cannot take the mix with it, under any blend
/// mode.**
///
/// A fader at silence has to be a *skip*, not a blend at zero, for the same
/// reason `Allocated` is: `0.0 * x` is zero only for finite `x`. A slot's own
/// target is allowed to hold a NaN — `sqrt` of a negative is a procedure that
/// passes every stage of this pipeline — and one blended at a zero fader would
/// otherwise put a NaN in every channel of the composite, wiping out every
/// other slot. This is the property `docs/roadmap.md` records as the first
/// thing M2 taught, and it is **the** reason the operator has a fader at all.
///
/// The comparison is against the same deck with that slot `Allocated`, which is
/// the path that was already exact, so this asserts the two ways of silencing a
/// slot agree.
///
/// Opacity is the fader here, and every mode is tried, because
/// [`Blend::silent_at`] is the only thing standing between a NaN and the mix
/// and a mode it forgot would be a slot that cannot be turned off. Gain gets
/// the same treatment under the two modes where it silences at all —
/// `zero_gain_silences_add_and_max_and_still_covers_under_over` is where that
/// list comes from. **Under `over`, gain does not silence and a NaN gets
/// through**; that is what the fader is for and it is deliberately not asserted
/// here, because pinning it would read as a promise that NaN reaches the mix.
#[test]
fn a_slot_faded_to_silence_cannot_take_the_mix_with_it() {
    let gpu = Gpu::headless().expect("no GPU available");

    let run = |silence: Residency, blend: Blend, gain: f32, opacity: f32| -> Vec<u16> {
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
        deck.set_blend(1, blend);
        deck.set_gain(1, gain);
        deck.set_opacity(1, opacity);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        if silence == Residency::Live {
            // The faded slot is only interesting if its target really does
            // hold a NaN. (Parked, it renders nothing at all, so its target is
            // the transparent black it was cleared to and there is nothing to
            // check.)
            let own = readback(&gpu, deck.slot_target(1));
            assert!(
                decode(&own).iter().any(|v| v.is_nan()),
                "the NaN slot rendered no NaN, so this test is asserting nothing"
            );
        }
        readback(&gpu, present.hdr_texture())
    };

    // Off air: skipped by residency, and exact.
    let parked = run(Residency::Allocated, Blend::Add, 1.0, 1.0);
    assert!(lit(&parked) > 100, "the surviving slot drew nothing");

    let nans = |mix: &[u16]| decode(mix).iter().filter(|v| v.is_nan()).count();

    for blend in Blend::ALL {
        let faded = run(Residency::Live, blend, 1.0, 0.0);
        assert_eq!(
            faded,
            parked,
            "a slot at opacity 0.0 under `{}` reached the mix ({} NaN channels), while \
             the same slot taken off air did not",
            blend.name(),
            nans(&faded)
        );
    }
    for blend in [Blend::Add, Blend::Max] {
        let faded = run(Residency::Live, blend, 0.0, 1.0);
        assert_eq!(
            faded,
            parked,
            "a slot at gain 0.0 under `{}` reached the mix ({} NaN channels)",
            blend.name(),
            nans(&faded)
        );
    }
}

/// **A mask at either end is exact: nothing, or everything.**
///
/// Both matter and for different reasons. At the top, a wipe is a transition
/// carrying `position` to 1.0, so a corner left half-lit would be a wipe that
/// never finished. At the bottom, `Blend::silent_at` *skips* a layer whose mask
/// reveals nothing — which is only sound if it really is nothing, and skipping
/// is what keeps a NaN out of the mix.
///
/// Compared against the fader, which is the path that was already exact: a
/// masked-out slot must render exactly what the same slot at opacity 0 renders,
/// and a fully revealed one exactly what it renders with no mask at all. The
/// material is [`L4_WASH`], because a mask's ends are at the edges of the frame
/// and the sphere every other fixture draws never gets there.
#[test]
fn a_mask_at_either_end_is_exactly_nothing_or_exactly_everything() {
    let gpu = Gpu::headless().expect("no GPU available");

    let run = |mask: Option<Mask>, opacity: f32| -> Vec<u16> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, MASK_SIZE, MASK_SIZE);
        let mut deck = wash_deck(&gpu);
        deck.set_opacity(1, opacity);
        if let Some(mask) = mask {
            deck.set_mask(1, mask);
        }
        for _ in 0..4 {
            frame(&gpu, &mut deck, &present, 1);
        }
        readback(&gpu, present.hdr_texture())
    };

    let unmasked = run(None, 1.0);
    let silent = run(None, 0.0);
    assert!(lit(&unmasked) > 100, "the deck drew nothing");
    assert_ne!(unmasked, silent, "the two references are the same picture");
    // The wash has to reach every texel, or an end that is wrong at the edge
    // is an end nothing here can see.
    assert_eq!(
        lit(&unmasked),
        (MASK_SIZE * MASK_SIZE) as usize,
        "the wash does not cover the frame, so a mask's edges are untested"
    );

    for kind in [MaskKind::Linear, MaskKind::Radial] {
        // Every angle, because a linear front's normalisation is per-direction
        // and one that overshot would show at one angle and not another.
        for angle in [0.0, 0.7, std::f32::consts::FRAC_PI_2, 2.4, -0.7] {
            assert_eq!(
                run(Some(Mask::new(kind, angle, 1.0, 0.3)), 1.0),
                unmasked,
                "{} at {angle} rad, fully open, is not the unmasked frame",
                kind.name()
            );
            assert_eq!(
                run(Some(Mask::new(kind, angle, 0.0, 0.3)), 1.0),
                silent,
                "{} at {angle} rad, fully closed, is not a silent slot",
                kind.name()
            );
        }
    }
}

/// **A mask that reveals nothing is a skip, not a multiply by zero.**
///
/// The only way to see the difference, and the reason the skip is there: a
/// slot's target may hold a NaN — `sqrt` of a negative is a procedure that
/// passes every stage of this pipeline — and `0.0 * NaN` is NaN. With clean
/// material a mask at position 0 and a skipped layer are the same picture, so
/// this is the case that tells them apart, exactly as it does for the fader.
///
/// It is also what makes a mask a third escape from broken material, beside
/// residency and the fader.
#[test]
fn a_mask_that_reveals_nothing_keeps_a_nan_out_of_the_mix() {
    let gpu = Gpu::headless().expect("no GPU available");

    let run = |silence: Residency, mask: Mask| -> Vec<u16> {
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
        deck.set_mask(1, mask);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        if silence == Residency::Live {
            assert!(
                decode(&readback(&gpu, deck.slot_target(1)))
                    .iter()
                    .any(|v| v.is_nan()),
                "the NaN slot rendered no NaN, so this test is asserting nothing"
            );
        }
        readback(&gpu, present.hdr_texture())
    };

    let parked = run(Residency::Allocated, Mask::default());
    assert!(lit(&parked) > 100, "the surviving slot drew nothing");

    for kind in [MaskKind::Linear, MaskKind::Radial] {
        let closed = run(Residency::Live, Mask::new(kind, 0.4, 0.0, 0.1));
        let nans = decode(&closed).iter().filter(|v| v.is_nan()).count();
        assert_eq!(
            closed,
            parked,
            "a slot masked to nothing under `{}` reached the mix ({nans} NaN channels), \
             while the same slot taken off air did not",
            kind.name()
        );
    }
}

/// **A mask in the middle shapes the frame rather than dimming it.**
///
/// The difference between a mask and a fader, and the only assertion that can
/// tell them apart: half way across, part of the frame is exactly what it would
/// be with the slot present and part exactly what it would be without. A fader
/// at 0.5 is neither, everywhere.
#[test]
fn a_mask_half_way_leaves_one_part_untouched_and_removes_another() {
    let gpu = Gpu::headless().expect("no GPU available");

    let run = |mask: Option<Mask>, opacity: f32| -> Vec<f32> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, MASK_SIZE, MASK_SIZE);
        let mut deck = wash_deck(&gpu);
        deck.set_opacity(1, opacity);
        if let Some(mask) = mask {
            deck.set_mask(1, mask);
        }
        for _ in 0..4 {
            frame(&gpu, &mut deck, &present, 1);
        }
        decode(&readback(&gpu, present.hdr_texture()))
    };

    let silent = run(None, 0.0);
    // A hard front straight up the middle, left to right, on a slot at half
    // opacity — so "revealed" and "unmasked" are different pictures and the
    // mask cannot be mistaken for the fader that is also on.
    let halfway = run(Some(Mask::new(MaskKind::Linear, 0.0, 0.5, 0.0)), 0.5);
    let faded = run(None, 0.5);

    // **Only where the slot actually contributes.** Where it drew nothing, the
    // masked frame, the faded one and the silent one all agree, and counting
    // those would drown the claim in background.
    let mut hidden = 0;
    let mut revealed = 0;
    let mut between = 0;
    for i in (0..halfway.len()).filter(|i| i % 4 != 3) {
        if faded[i] == silent[i] {
            continue;
        }
        if halfway[i] == silent[i] {
            hidden += 1;
        } else if halfway[i] == faded[i] {
            revealed += 1;
        } else {
            between += 1;
        }
    }

    assert!(
        hidden > 100,
        "the mask removed the slot from {hidden} of the channels it drew, so the front \
         is not on the frame"
    );
    assert!(
        revealed > 100,
        "the mask left the slot in {revealed} of the channels it drew, so it is hiding \
         everything rather than shaping"
    );
    // A hard edge, so every contributing channel is on one side or the other.
    // A *fader* would put all of them in `between`, which is the difference
    // this test exists to see.
    assert!(
        between * 20 < hidden + revealed,
        "{between} channels are neither the revealed picture nor the hidden one, \
         against {} that are — a hard-edged mask is one or the other",
        hidden + revealed
    );
    // And the front is where it was asked for: half the covered frame, either
    // side. A wipe that finished early would still be "one or the other".
    let split = hidden as f32 / (hidden + revealed) as f32;
    assert!(
        (split - 0.5).abs() < 0.1,
        "the front left {split:.2} of the frame hidden rather than half, so it is not \
         where `position` says"
    );
}

/// **A wipe is a mask and one scheduled move**, and neither had to know about
/// the other.
///
/// The claim the whole design rests on: `Control::MaskPosition` carries the
/// front, the mask reads a number, and the picture between the two ends is
/// neither of them. Checked at three points, because a wipe that jumped would
/// pass a two-point test.
#[test]
fn a_wipe_is_a_transition_carrying_a_masks_front() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
    deck.set_signals(Signals::new(120.0, 1));
    deck.set_blend(1, Blend::Over);
    deck.set_mask(1, Mask::new(MaskKind::Linear, 0.0, 0.0, 0.02));

    let start = deck.signals().oscillator().beats();
    deck.schedule(Transition::new(
        1,
        Control::MaskPosition,
        0.0,
        1.0,
        start,
        4.0,
        Curve::Lin,
    ));

    let mut fronts = Vec::new();
    for i in 0..121 {
        frame(&gpu, &mut deck, &present, 1);
        if i % 30 == 0 {
            fronts.push(deck.mask(1).position());
        }
    }
    // Monotone and strictly moving, which a jump would not be.
    for pair in fronts.windows(2) {
        assert!(
            pair[1] > pair[0],
            "the front went backwards or stood still: {fronts:?}"
        );
    }
    assert_eq!(deck.mask(1).position(), 1.0, "the wipe did not finish");
    // The shape survived: a move carries the position and leaves the kind
    // alone, which is why `set_mask` does not cancel a transition.
    assert_eq!(deck.mask(1).kind(), MaskKind::Linear);
    assert_eq!(deck.transitions_on(1).count(), 0);
}

/// **A scheduled fade moves the fader on the beat grid and nowhere else.**
///
/// The claim that makes a transition reproducible: it is a function of the
/// session's beat count, so two runs given the same ticks fade identically —
/// and a run at a different frame rate reaching the same beat is at the same
/// point in the fade. Checked against the arithmetic rather than against a
/// second run of the deck, which would agree with any implementation.
#[test]
fn a_scheduled_fade_is_a_function_of_the_beat_count() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A]);
    // 120 bpm and dt of 1/60 is exactly two beats a second, so a frame is
    // 1/30 of a beat and the arithmetic below is exact.
    deck.set_signals(Signals::new(120.0, 1));

    let start = deck.signals().oscillator().beats();
    deck.schedule(Transition::new(
        0,
        Control::Opacity,
        1.0,
        0.0,
        start,
        4.0,
        Curve::Lin,
    ));

    // 120 frames is four beats: the whole fade.
    for i in 0..120 {
        let beats = deck.signals().oscillator().beats();
        let expected = 1.0 - (beats - start) as f32 / 4.0;
        assert!(
            (deck.opacity(0) - expected).abs() < 1e-6,
            "frame {i} at {beats} beats: the fader is {} rather than {expected}",
            deck.opacity(0)
        );
        frame(&gpu, &mut deck, &present, 1);
    }
    // Exactly at silence, and the transition gone rather than still writing.
    assert_eq!(deck.opacity(0), 0.0);
    assert_eq!(
        deck.transitions_on(0).count(),
        0,
        "a finished fade is still scheduled"
    );
}

/// **A hand on the fader wins.**
///
/// The one place an operator reaches when something is wrong is the one place
/// an automatic thing is writing, so a transition that kept going after a
/// manual move would be the worst control on the deck. Asserted for both ways
/// of touching it, since either is what a hand does.
#[test]
fn moving_a_control_by_hand_cancels_the_transition_moving_it() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A]);
    deck.set_signals(Signals::new(120.0, 1));

    let start = deck.signals().oscillator().beats();
    deck.schedule(Transition::new(
        0,
        Control::Opacity,
        1.0,
        0.0,
        start,
        8.0,
        Curve::Lin,
    ));
    for _ in 0..30 {
        frame(&gpu, &mut deck, &present, 1);
    }
    let mid = deck.opacity(0);
    assert!(mid > 0.0 && mid < 1.0, "the fade did not start: {mid}");

    deck.set_opacity(0, 0.75);
    for _ in 0..60 {
        frame(&gpu, &mut deck, &present, 1);
    }
    assert_eq!(
        deck.opacity(0),
        0.75,
        "the fade kept writing after the fader was moved by hand"
    );
    assert_eq!(deck.transitions_on(0).count(), 0);

    // And the other control's transition is untouched by the wrong fader:
    // cancelling has to be per control, or a gain move would stop an opacity
    // fade and an operator would never find out why.
    deck.schedule(Transition::new(
        0,
        Control::Gain,
        1.0,
        0.0,
        start,
        8.0,
        Curve::Lin,
    ));
    deck.set_opacity(0, 0.5);
    assert_eq!(
        deck.transitions_on(0).count(),
        1,
        "the gain fade was cancelled too"
    );

    // **The gain half of the rule, which this test claimed and did not check.**
    // `[`, `]` and `\` all end in `set_gain`, so a gain fade that kept writing
    // after one of them would be a control fighting the hand on it.
    let start = deck.signals().oscillator().beats();
    deck.schedule(Transition::new(
        0,
        Control::Gain,
        1.0,
        0.0,
        start,
        8.0,
        Curve::Lin,
    ));
    for _ in 0..30 {
        frame(&gpu, &mut deck, &present, 1);
    }
    let mid = deck.gain(0);
    assert!(mid > 0.0 && mid < 1.0, "the gain fade did not start: {mid}");
    deck.set_gain(0, 2.0);
    for _ in 0..60 {
        frame(&gpu, &mut deck, &present, 1);
    }
    assert_eq!(
        deck.gain(0),
        2.0,
        "the gain fade kept writing after the level was moved by hand"
    );
    assert_eq!(deck.transitions_on(0).count(), 0);
}

/// **A move onto a slot the deck does not have is refused where it is asked
/// for**, not three seconds later inside a frame.
///
/// `advance_transitions` indexes the slots directly, so an unchecked schedule
/// is a panic on the render thread at some unrelated moment. `set_gain` and
/// `set_opacity` panic at the call site; this joins them.
#[test]
#[should_panic(expected = "no slot 3")]
fn scheduling_a_move_onto_a_slot_that_is_not_there_is_refused_at_the_call() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut deck = deck_of(&gpu, &[SEED_A]);
    deck.schedule(Transition::new(
        3,
        Control::Opacity,
        1.0,
        0.0,
        0.0,
        4.0,
        Curve::Lin,
    ));
}

/// **The composite sees this frame's fader, not the last one's.**
///
/// A transition that ran *after* the mix was recorded would put every fade one
/// frame late — invisible in a four-beat fade and exactly wrong in a cut, which
/// is the case this uses. Compared against a deck whose fader was moved by hand
/// before the frame, which is the path that was already exact: the two are the
/// same picture if and only if the scheduled cut landed on the frame it was
/// scheduled for.
///
/// The comparison it replaces was `assert_ne!` against an earlier frame, which
/// this material passes with no transition scheduled at all — it rotates on `t`.
#[test]
fn a_scheduled_cut_lands_on_the_frame_it_was_scheduled_for() {
    let gpu = Gpu::headless().expect("no GPU available");
    const LEAD: usize = 12;

    // The reference: the same deck, the same ticks, the fader moved by hand
    // before the frame in question.
    let by_hand = {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_signals(Signals::new(120.0, 1));
        for _ in 0..LEAD {
            frame(&gpu, &mut deck, &present, 1);
        }
        deck.set_opacity(1, 0.0);
        frame(&gpu, &mut deck, &present, 1);
        readback(&gpu, present.hdr_texture())
    };

    // The same run, with the cut scheduled for the beat that frame lands on.
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
    deck.set_signals(Signals::new(120.0, 1));
    for _ in 0..LEAD {
        frame(&gpu, &mut deck, &present, 1);
    }
    // Where the clock stands. The next frame advances it first, so this
    // instant is already past by the time the transition is read — which is
    // the frame it is due on, and the frame the composite has to see it on.
    let cut_at = deck.signals().oscillator().beats();
    deck.schedule(Transition::new(
        1,
        Control::Opacity,
        1.0,
        0.0,
        cut_at,
        0.0,
        Curve::Lin,
    ));
    frame(&gpu, &mut deck, &present, 1);
    let scheduled = readback(&gpu, present.hdr_texture());

    assert_eq!(
        deck.opacity(1),
        0.0,
        "the cut did not land on the frame it was scheduled for"
    );
    assert_eq!(
        scheduled, by_hand,
        "the mix on the frame of a scheduled cut is not the mix of the same fader moved \
         by hand — the composite is reading a fader the transition has not written yet"
    );
}

/// **A scheduled move cannot reach a value a hand could not.**
///
/// It writes the slot's field directly rather than through `set_gain` and
/// `set_opacity`, because those cancel it — so the clamps they carry have to be
/// applied on the way past, or a `transition` record would be the one path into
/// the mix with no bound on it. An opacity above 1.0 makes an `over` layer
/// subtract more than it covers.
#[test]
fn a_scheduled_move_is_clamped_the_way_a_manual_one_is() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A]);
    deck.set_signals(Signals::new(120.0, 1));
    let start = deck.signals().oscillator().beats();

    deck.schedule(Transition::new(
        0,
        Control::Opacity,
        1.0,
        4.0,
        start,
        0.0,
        Curve::Lin,
    ));
    deck.schedule(Transition::new(
        0,
        Control::Gain,
        1.0,
        -3.0,
        start,
        0.0,
        Curve::Lin,
    ));
    frame(&gpu, &mut deck, &present, 1);

    assert_eq!(deck.opacity(0), 1.0, "a scheduled fader passed 1.0");
    assert_eq!(deck.gain(0), 0.0, "a scheduled level went negative");
}

/// **A crossfade is two scheduled moves**, and what makes that a crossfade
/// rather than two fades is that they share a start and a length.
///
/// The mix is checked rather than the fields: halfway through, the outgoing
/// slot is dimmer than it was and the incoming one is brighter, and the frame
/// carries both. That is the thing the roadmap asked for, and it needed no type
/// of its own.
#[test]
fn a_crossfade_is_two_moves_sharing_a_start_and_a_length() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
    deck.set_signals(Signals::new(120.0, 1));
    deck.set_opacity(1, 0.0);
    for _ in 0..12 {
        frame(&gpu, &mut deck, &present, 1);
    }

    let start = deck.signals().oscillator().beats();
    deck.schedule(Transition::new(
        0,
        Control::Opacity,
        1.0,
        0.0,
        start,
        4.0,
        Curve::Smooth,
    ));
    deck.schedule(Transition::new(
        1,
        Control::Opacity,
        0.0,
        1.0,
        start,
        4.0,
        Curve::Smooth,
    ));

    // Two beats: halfway, where both are somewhere in the middle.
    for _ in 0..60 {
        frame(&gpu, &mut deck, &present, 1);
    }
    let a = deck.opacity(0);
    let b = deck.opacity(1);
    assert!(a > 0.0 && a < 1.0 && b > 0.0 && b < 1.0, "{a} / {b}");
    assert!(
        (a + b - 1.0).abs() < 0.05,
        "a smooth crossfade should be near unity through the middle: {a} + {b}"
    );

    // Two more beats: the other end, exactly.
    for _ in 0..60 {
        frame(&gpu, &mut deck, &present, 1);
    }
    assert_eq!(deck.opacity(0), 0.0);
    assert_eq!(deck.opacity(1), 1.0);
    assert_eq!(
        deck.transitions_on(0).count() + deck.transitions_on(1).count(),
        0
    );
}

/// **An audition shows the slot's own target, bit for bit, at unity.**
///
/// Preview is not a second pass and not a copy — it is the mix with one term in
/// it — so the claim available is close to the strongest one: what reaches the
/// target is the previewed slot's texels and nothing else. `0.0 + 1.0 * src` is
/// `src`, in colour unconditionally and in alpha for material whose coverage is
/// in range, which the material here is. `Blend::Add` carries the caveat.
///
/// Two things it must ignore, and both are the point of auditioning. The
/// **faders**, because what is being judged is the level the material arrives
/// at rather than the setting somebody already gave it; and the **other
/// slots**, because a preview that summed anything would be a mix.
#[test]
fn an_audition_shows_that_slots_own_target_and_ignores_the_faders() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
    // Settings that would all be visible if any of them reached the preview:
    // slot 1 pulled down and put under a mode that hides, slot 0 turned up.
    deck.set_gain(0, 3.0);
    deck.set_gain(1, 0.3);
    deck.set_opacity(1, 0.4);
    deck.set_blend(1, Blend::Over);

    deck.set_preview(Some(1));
    for _ in 0..12 {
        frame(&gpu, &mut deck, &present, 1);
    }

    let shown = readback(&gpu, present.hdr_texture());
    let own = readback(&gpu, deck.slot_target(1));
    assert_eq!(
        shown, own,
        "the audition is not slot 1's own target — a fader, another slot, or a \
         resample reached it"
    );
    assert!(
        lit(&own) > 100,
        "slot 1 drew nothing, so this test compares two black frames"
    );

    // And the mix is still there to go back to: turning the preview off shows
    // something the audition did not.
    deck.set_preview(None);
    for _ in 0..12 {
        frame(&gpu, &mut deck, &present, 1);
    }
    let mixed = readback(&gpu, present.hdr_texture());
    assert_ne!(
        mixed,
        readback(&gpu, deck.slot_target(1)),
        "the mix and slot 1 alone are the same picture, so the comparison above \
         could not have failed"
    );
}

/// **Looking at a slot does not run it.**
///
/// The property every residency level rests on is that `t` advances through
/// `Set::prepare` and nowhere else, which is what lets a slot be taken off air
/// and put back where it stopped. An audition that stepped what it was looking
/// at would break that in the least visible way possible: the operator sees a
/// running image, puts it on air, and it is somewhere other than where they
/// left it.
///
/// Asserted against `Allocated`, where the claim is absolute — no step at all,
/// however many frames it is watched for — and the still it holds is what an
/// audition of a stopped slot is *supposed* to show.
///
/// **The slot is warmed by priming rather than by being on air**, and that is
/// what makes the draw half of this testable at all: priming steps and does not
/// draw, so the target has never been written and is still the transparent
/// black it was cleared to. Run it Live first and the target holds a picture
/// from when it was, so an audition that drew nothing would show that picture
/// and pass — which is what this test did until the defect was injected. It is
/// also the composition `Deck::set_preview` describes: park it, prime it, look
/// at it.
#[test]
fn an_audition_draws_an_allocated_slot_without_stepping_it() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);

    // Warm it out of sight, so it has element state and its target has never
    // been drawn into.
    deck.set_residency(1, Residency::Priming);
    for _ in 0..12 {
        frame(&gpu, &mut deck, &present, 1);
    }
    assert_eq!(
        lit(&readback(&gpu, deck.slot_target(1))),
        0,
        "priming drew into the slot's target, so this test cannot tell a draw from \
         a leftover"
    );

    deck.set_residency(1, Residency::Allocated);
    let parked_steps = steps_taken(deck.slot(1).set());
    let parked_t = deck.slot(1).set().time();
    assert!(
        parked_steps > 0,
        "the slot never warmed, so it has nothing to draw"
    );

    deck.set_preview(Some(1));
    for _ in 0..30 {
        frame(&gpu, &mut deck, &present, 1);
    }

    assert_eq!(
        steps_taken(deck.slot(1).set()),
        parked_steps,
        "an audition stepped an Allocated slot, so looking at material moves it"
    );
    assert_eq!(deck.slot(1).set().time(), parked_t);
    // Drawn all the same, or an audition of a parked slot would be a black
    // frame and there would be nothing to audition.
    let shown = readback(&gpu, present.hdr_texture());
    assert!(
        lit(&shown) > 100,
        "an audition of an Allocated slot showed nothing, so the draw did not happen"
    );
    assert_eq!(
        shown,
        readback(&gpu, deck.slot_target(1)),
        "the audition is not the parked slot's target"
    );
}

/// **Auditioning a Priming slot shows what it is warming into, on every frame.**
///
/// This is the workflow the whole feature is for: a candidate warms out of
/// sight, the operator looks at it before deciding, and the deciding does not
/// disturb the warming. Priming draws nothing on its own — that is the point of
/// it — so the target starts as the transparent black it was cleared to and
/// anything on screen came from the audition's draw.
///
/// **The second half is why it draws on every frame and not only on the frames
/// the slot steps**, and it took an injected defect to find a version of this
/// that could tell the two apart. A slot target persists, so between steps the
/// two behave identically and no readback can separate them. Where they
/// separate is a resize, which reallocates the target: draw only on step frames
/// and a slot priming one frame in eight shows black until its next step. The
/// resize below is timed to land on a frame that does not step.
#[test]
fn auditioning_a_priming_slot_survives_the_frame_its_target_is_reallocated() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
    deck.set_residency(1, Residency::Priming);
    deck.set_prime_one_in(1, 8);
    deck.set_preview(Some(1));

    // Nine frames: indices 0 and 8 stepped, and the phase now stands at 9.
    for _ in 0..9 {
        frame(&gpu, &mut deck, &present, 1);
    }
    let warmed = steps_taken(deck.slot(1).set());
    assert_eq!(
        warmed, 2,
        "the slot stepped {warmed} times in nine frames rather than 2, so the phase is \
         not where the rest of this test assumes"
    );
    assert!(
        lit(&readback(&gpu, present.hdr_texture())) > 100,
        "the audition showed nothing before the resize, so what follows proves nothing"
    );

    // The one moment a persisted target stops being an answer.
    present.resize(&gpu.device, WIDTH / 2, HEIGHT / 2);
    deck.resize(&gpu.device, WIDTH / 2, HEIGHT / 2);
    // Phase 9, so this frame does not step.
    frame(&gpu, &mut deck, &present, 1);
    assert_eq!(
        steps_taken(deck.slot(1).set()),
        warmed,
        "the frame after the resize stepped, so it cannot show whether the draw is \
         tied to stepping"
    );

    let shown = readback(&gpu, present.hdr_texture());
    assert!(
        lit(&shown) > 100,
        "the audition went black on the frame after its target was reallocated, so a \
         warming slot at a slow rate disappears when the window is resized"
    );
    // And it is still warming: looking at it did not take over its clock.
    for _ in 0..16 {
        frame(&gpu, &mut deck, &present, 1);
    }
    assert!(
        steps_taken(deck.slot(1).set()) > warmed,
        "the slot stopped warming while it was being looked at"
    );
}

/// **An audition survives a resize that changes the aspect ratio.**
///
/// The viewport and the camera's aspect ratio live in the L4 uniform block, and
/// that block is written by `Set::prepare` and by nothing else — while
/// `Set::resize` moves only the host-side value. A parked slot never prepares,
/// so an audition of one drew at the aspect it had before the resize, and went
/// on doing so for as long as it stayed off air. Which is permanently: going
/// off air is what stops it being prepared.
///
/// **A square resize cannot see this**, which is why the sibling test above
/// could not: halving both dimensions leaves the aspect ratio alone and the
/// stale matrix is the right matrix. The comparison here is against the same
/// Set prepared at the same size through the ordinary path, which is the only
/// reference that is not a second run of the thing under test.
#[test]
fn an_audition_redraws_at_the_aspect_ratio_it_was_resized_to() {
    let gpu = Gpu::headless().expect("no GPU available");
    const WIDE: u32 = 256;
    const SHORT: u32 = 64;

    // The reference: a deck that was this shape all along, so its Set was
    // prepared at this aspect on every frame it ran.
    let reference = {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDE, SHORT);
        let mut deck = deck_of_at(&gpu, &[SEED_A], WIDE, SHORT);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        deck.set_residency(0, Residency::Allocated);
        deck.set_preview(Some(0));
        frame(&gpu, &mut deck, &present, 1);
        readback(&gpu, present.hdr_texture())
    };
    assert!(lit(&reference) > 100, "the reference drew nothing");

    // The same Set, run square, parked, and then resized to that shape while
    // it was parked — so nothing has prepared it at the new aspect.
    let mut present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A]);
    for _ in 0..12 {
        frame(&gpu, &mut deck, &present, 1);
    }
    deck.set_residency(0, Residency::Allocated);
    present.resize(&gpu.device, WIDE, SHORT);
    deck.resize(&gpu.device, WIDE, SHORT);
    deck.set_preview(Some(0));
    frame(&gpu, &mut deck, &present, 1);
    let shown = readback(&gpu, present.hdr_texture());

    assert_eq!(
        shown, reference,
        "the audition drew at the aspect ratio the slot had before the resize, so a \
         parked slot is auditioned at the wrong shape until it goes back on air"
    );
}

/// **The audition is metered; a slot nobody is looking at is not.**
///
/// This is what separates an audition from a look. The number an operator wants
/// before putting a slot on air is what level it will arrive at, and `deck.rs`
/// retires the meter for a slot that is off air precisely because its target is
/// stale — which stops being true the moment something is drawing it.
#[test]
fn an_auditioned_slot_is_metered_and_an_unwatched_one_is_not() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B, SEED_A]);
    deck.enable_meters(&gpu.device);
    // **Warmed by priming, never by being on air**, for the same reason as
    // `an_audition_draws_an_allocated_slot_without_stepping_it`: a slot that
    // has run Live leaves a picture in its target, and a meter reading that
    // picture is exactly the stale number `Meters::retire` exists to prevent —
    // so the test would pass with no audition draw at all. A Set that has never
    // been stepped has nothing to draw either, so priming is the only way to
    // get element state into a slot whose target is still the black it was
    // cleared to.
    deck.set_residency(1, Residency::Priming);
    deck.set_residency(2, Residency::Priming);
    for _ in 0..12 {
        frame(&gpu, &mut deck, &present, 1);
    }
    deck.set_residency(1, Residency::Allocated);
    deck.set_residency(2, Residency::Allocated);
    deck.set_preview(Some(1));

    // Long enough for a reading to make the round trip. The meter never waits,
    // so this is the harness standing in for the frames a real run would have.
    for _ in 0..60 {
        frame(&gpu, &mut deck, &present, 1);
    }

    let watched = deck.level(1);
    assert!(
        watched.is_some_and(|l| l.mean > 0.0),
        "the auditioned slot reported {watched:?}, so its level cannot be read before \
         it goes on air"
    );
    assert_eq!(
        deck.level(2),
        None,
        "an Allocated slot nobody is looking at reported a level, which would be a \
         reading of whatever its target last held"
    );
}

/// **`over` hides what is under it and `add` does not**, which is the whole of
/// what the blend vocabulary buys.
///
/// Slot 1 is [`L4_CARD`] — black, opaque, and contributing no colour at all —
/// so the two modes differ by exactly one thing: whether the coverage it drew
/// is allowed to take the layer under it away. The expectation is not a
/// direction but a number, read from the card's own target:
///
/// ```text
///   add:   A + 0        = A
///   over:  0 + A*(1 - c)         c = the card's coverage at that texel
/// ```
///
/// Inexact for the same single reason as `gain_is_linear_...`: the GPU works in
/// `f32` and rounds once to `f16` on write, while the expectation is computed
/// in `f32` from values already rounded to `f16`.
#[test]
fn over_hides_what_is_under_it_and_add_does_not() {
    let gpu = Gpu::headless().expect("no GPU available");

    let run = |blend: Blend| -> (Vec<f32>, Vec<f32>) {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = Deck::new(
            &gpu.device,
            vec![
                HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                HotSwap::fixed(build_with(&gpu, L4_CARD, SEED_B, CAPACITY)),
            ],
            WIDTH,
            HEIGHT,
        );
        deck.set_blend(1, blend);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        (
            decode(&readback(&gpu, present.hdr_texture())),
            decode(&readback(&gpu, deck.slot_target(1))),
        )
    };

    let (added, card) = run(Blend::Add);
    let (overed, _) = run(Blend::Over);

    // The card has to actually cover something, or every assertion below is
    // `A == A` and this test says nothing. Alpha is the fourth channel.
    let covered = card.iter().skip(3).step_by(4).filter(|c| **c > 0.5).count();
    assert!(
        covered > 100,
        "the card covered only {covered} texels, so there is nothing for `over` to hide"
    );

    const TOLERANCE: f32 = 1.0 / 1024.0;
    let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

    let mut misses = 0;
    let mut worst = 0.0f32;
    let mut hidden = 0;
    // Colour only: the fourth channel is coverage, and what the mix does with
    // coverage is the same under every mode by construction.
    for i in (0..added.len()).filter(|i| i % 4 != 3) {
        let coverage = card[(i / 4) * 4 + 3];
        let expected = added[i] * (1.0 - coverage);
        if !close(overed[i], expected) {
            misses += 1;
            worst = worst.max((overed[i] - expected).abs());
        }
        if !close(overed[i], added[i]) {
            hidden += 1;
        }
    }

    assert_eq!(
        misses,
        0,
        "`over` is not `A*(1 - coverage)`: {misses} of {} colour channels disagree, \
         worst by {worst}",
        added.len()
    );
    // The other half, and the half that fails if `over` silently stayed `add`:
    // the two modes have to differ somewhere, or the first assertion passed
    // only because the coverage was zero everywhere it looked.
    assert!(
        hidden > 100,
        "only {hidden} channels distinguish `over` from `add`, so this run cannot tell \
         the two modes apart"
    );
}

/// **`max` stacks without summing.**
///
/// Two lit slots. Under `add` the mix is `A + B`; under `max` it is the larger
/// of the two per channel, which is what makes four layers of the same bright
/// material stay that bright instead of reaching four times it. `A` and `B` are
/// measured on their own — same deck, same seeds, same ticks, one slot off air
/// each time — so both are sampled at the same `t` as the mix.
#[test]
fn max_takes_the_larger_of_two_layers_rather_than_their_sum() {
    let gpu = Gpu::headless().expect("no GPU available");

    let run = |blend: Blend, off_air: Option<usize>| -> Vec<f32> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_blend(1, blend);
        if let Some(slot) = off_air {
            deck.set_residency(slot, Residency::Allocated);
        }
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        decode(&readback(&gpu, present.hdr_texture()))
    };

    let a = run(Blend::Add, Some(1));
    let b = run(Blend::Add, Some(0));
    let summed = run(Blend::Add, None);
    let maxed = run(Blend::Max, None);

    const TOLERANCE: f32 = 1.0 / 1024.0;
    let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

    let mut misses = 0;
    let mut worst = 0.0f32;
    let mut distinct = 0;
    for i in (0..maxed.len()).filter(|i| i % 4 != 3) {
        let expected = a[i].max(b[i]);
        if !close(maxed[i], expected) {
            misses += 1;
            worst = worst.max((maxed[i] - expected).abs());
        }
        // Where both layers are lit, `max` is strictly less than `add`. If
        // nowhere is, the two slots never overlap and the comparison above is
        // `A + 0` against `max(A, 0)`, which agree.
        if !close(maxed[i], summed[i]) {
            distinct += 1;
        }
    }

    assert_eq!(
        misses,
        0,
        "`max` is not the per-channel maximum: {misses} of {} colour channels disagree, \
         worst by {worst}",
        maxed.len()
    );
    assert!(
        distinct > 100,
        "only {distinct} channels distinguish `max` from `add`, so the two slots barely \
         overlap and this run cannot tell them apart"
    );
}

/// **An opacity outside `[0, 1]` is clamped where the engine takes it**, not
/// where a key press produces it.
///
/// `karakuri-cli` clamps at the key so that the `opacity` record carries the
/// value that took effect, but a record is also how a *replay* drives the deck,
/// and a stream is allowed to say anything. Past 1.0 an `over` layer subtracts
/// more than it covers; below 0.0 it adds what it should have hidden. Unlike
/// gain — a level into an HDR mix, deliberately open above 1.0 — every value
/// outside this range has exactly one sensible reading, so it is clamped rather
/// than refused.
///
/// NaN silences, which is the third value a fader can carry and the one with no
/// obvious reading: the two available are "this slot goes dark" and "the whole
/// mix goes dark".
#[test]
fn an_opacity_a_record_could_carry_is_clamped_to_a_fader() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut deck = deck_of(&gpu, &[SEED_A]);

    for (asked, expected) in [
        (3.0, 1.0),
        (1.0, 1.0),
        (0.5, 0.5),
        (0.0, 0.0),
        (-2.0, 0.0),
        (f32::INFINITY, 1.0),
        (f32::NEG_INFINITY, 0.0),
        (f32::NAN, 0.0),
    ] {
        deck.set_opacity(0, asked);
        assert_eq!(
            deck.opacity(0),
            expected,
            "an opacity of {asked} reached the mix as {}",
            deck.opacity(0)
        );
    }
}

/// **A gain a record could carry is floored at zero, and a NaN reads as zero.**
///
/// The same hole as the one above and it needed the same answer: `karakuri-cli`
/// floors at the key press, which says plainly that a negative gain is wrong,
/// but a replayed `{"t":"gain","slot":1,"value":-2.0}` does not go through a
/// key press. Unbounded *above*, unlike opacity, because gain is a level into
/// an HDR mix and 4.0 is an ordinary thing to want.
///
/// The NaN case is the one that cannot be recovered from. A NaN gain puts a NaN
/// in every channel of the mix from one slot, and unlike the material's own
/// NaN — which the fader skips past — no fader undoes a gain that has already
/// multiplied by one.
#[test]
fn a_gain_a_record_could_carry_is_floored_but_not_ceilinged() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut deck = deck_of(&gpu, &[SEED_A]);

    for (asked, expected) in [
        (4.0, 4.0),
        (1.0, 1.0),
        (0.0, 0.0),
        (-2.0, 0.0),
        (f32::INFINITY, f32::INFINITY),
        (f32::NEG_INFINITY, 0.0),
        (f32::NAN, 0.0),
    ] {
        deck.set_gain(0, asked);
        assert_eq!(
            deck.gain(0),
            expected,
            "a gain of {asked} reached the mix as {}",
            deck.gain(0)
        );
    }
}

/// **An alpha that is not a coverage cannot invert the mix or NaN it.**
///
/// `over` is `A*(1 - covered)`, so a coverage of 1.5 turns hiding into
/// *subtracting*, a coverage of 2 or more turns it into amplifying with the
/// sign flipped, and a NaN takes every channel of the frame. Nothing in the
/// pipeline bounds what an L4 writes to alpha — see [`OVERDRAWN_ALPHA`] — and
/// before blend modes existed that did not matter, because the channel was
/// written by nothing and read by nothing. It is load-bearing now, which is why
/// the mix saturates on the way in rather than trusting the material.
///
/// **All four spellings of "not a coverage", not only the one that motivated
/// the fix.** Above one is the case that reads as a hiding layer subtracting;
/// negative and infinite are the same arithmetic further along; and NaN is the
/// one the saturation catches only because it is written as a comparison rather
/// than as `clamp`, whose behaviour on a NaN operand WGSL leaves to the
/// backend. A test that ran only the finite case would pass on a backend where
/// the NaN case renders a blank frame.
///
/// Three claims per spelling. No colour channel of the mix is a NaN, none is
/// negative, and the mix's own coverage stays in range — the last one being
/// what says the saturation is where it belongs, since that value is what the
/// slot above this one is composited against.
#[test]
fn an_alpha_that_is_not_a_coverage_cannot_invert_the_mix_or_nan_it() {
    let gpu = Gpu::headless().expect("no GPU available");

    for (name, alpha) in OVERDRAWN_ALPHA {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = Deck::new(
            &gpu.device,
            vec![
                HotSwap::fixed(build_with(&gpu, L4_WIDE, SEED_A, CAPACITY)),
                HotSwap::fixed(build_with(&gpu, &overdrawn_card(alpha), SEED_B, CAPACITY)),
            ],
            WIDTH,
            HEIGHT,
        );
        deck.set_blend(1, Blend::Over);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }

        // The material has to actually be out of range, or this is a test of
        // ordinary coverage under a frightening name.
        let own = decode(&readback(&gpu, deck.slot_target(1)));
        let bad = own
            .iter()
            .skip(3)
            .step_by(4)
            .filter(|a| !(0.0..=1.0).contains(*a))
            .count();
        assert!(
            bad > 100,
            "the {name} card left only {bad} texels outside a coverage of [0, 1], so \
             nothing here is being saturated"
        );

        let mixed = decode(&readback(&gpu, present.hdr_texture()));
        let nan = (0..mixed.len())
            .filter(|i| i % 4 != 3)
            .filter(|&i| mixed[i].is_nan())
            .count();
        assert_eq!(
            nan, 0,
            "{nan} colour channels of the mix are NaN under the {name} card, so an alpha \
             nothing draws with reached every channel of the frame"
        );
        let negative = (0..mixed.len())
            .filter(|i| i % 4 != 3)
            .filter(|&i| mixed[i] < 0.0)
            .count();
        assert_eq!(
            negative, 0,
            "{negative} colour channels of the mix are negative under the {name} card, so \
             the coverage turned `over` from hiding into subtracting"
        );
        let unbounded = mixed
            .iter()
            .skip(3)
            .step_by(4)
            .filter(|a| !(0.0..=1.0).contains(*a))
            .count();
        assert_eq!(
            unbounded, 0,
            "{unbounded} texels of the mix carry a coverage outside [0, 1] under the \
             {name} card, which is what the next slot in the stack would be composited \
             against"
        );
    }
}

/// **Opacity moves the mix at settings between silence and full, under every
/// mode.**
///
/// Every other test here pins the fader at 0.0 or 1.0, where a composite that
/// ignored `opacity` outright is indistinguishable from one that honours it —
/// 0.0 is the skip, which [`Blend::silent_at`] decides on the host, and 1.0 is
/// the identity. So without this test the one control this whole slice exists
/// to make real has nothing saying it does anything.
///
/// Each mode gets its own reference, and none of them is a second run of the
/// composite at a different fader:
///
/// ```text
///   add:   a half fader at gain g is bit-identical to a full fader at g/2
///   over:  A*(1 - o*c)              c = the card's coverage
///   max:   mix(A, max(A, B), o)     A and B measured on their own
/// ```
#[test]
fn opacity_moves_the_mix_at_settings_between_zero_and_one() {
    let gpu = Gpu::headless().expect("no GPU available");
    const HALF: f32 = 0.5;
    const GAIN: f32 = 1.4;

    // --- `add`: **in colour**, opacity is the same multiply gain is, so it can
    // be checked against gain exactly. This is the collapse the deck's two
    // numbers used to be justified by, asserted rather than asserted about.
    //
    // Colour and not the whole texel, because the collapse stops at the alpha
    // channel: opacity scales coverage and gain does not, so the same picture
    // under the two settings carries a different coverage. That is the
    // difference between a fader and a level, showing up in the one channel
    // where `add` cannot hide it.
    let add_run = |gain: f32, opacity: f32| -> Vec<f32> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_gain(1, gain);
        deck.set_opacity(1, opacity);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        decode(&readback(&gpu, present.hdr_texture()))
    };
    let colour = |mix: &[f32]| -> Vec<f32> {
        (0..mix.len())
            .filter(|i| i % 4 != 3)
            .map(|i| mix[i])
            .collect()
    };

    let half_fader = colour(&add_run(GAIN, HALF));
    let half_gain = colour(&add_run(GAIN * HALF, 1.0));
    let full = colour(&add_run(GAIN, 1.0));
    assert_ne!(
        full,
        half_gain,
        "gain {GAIN} and gain {} render the same colours, so the comparison below is \
         vacuous",
        GAIN * HALF
    );
    assert_eq!(
        half_fader, half_gain,
        "under `add`, a fader at {HALF} is not the multiply a gain at the same factor is"
    );

    // --- `over` and `max` share the numeric comparison, and it is inexact for
    // the single reason the other numeric tests here are: `f32` on the GPU,
    // rounded once to `f16` on write, against an expectation computed in `f32`
    // from values already rounded.
    const TOLERANCE: f32 = 1.0 / 1024.0;
    let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

    // --- `over`: the card contributes no colour, so a half fader has to leave
    // exactly half the hole a full one does.
    let over_run = |blend: Blend, opacity: f32| -> (Vec<f32>, Vec<f32>) {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = Deck::new(
            &gpu.device,
            vec![
                HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                HotSwap::fixed(build_with(&gpu, L4_CARD, SEED_B, CAPACITY)),
            ],
            WIDTH,
            HEIGHT,
        );
        deck.set_blend(1, blend);
        deck.set_opacity(1, opacity);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        (
            decode(&readback(&gpu, present.hdr_texture())),
            decode(&readback(&gpu, deck.slot_target(1))),
        )
    };
    // `add` at full opacity is `A + 0`, which is `A`.
    let (bare, card) = over_run(Blend::Add, 1.0);
    let (half_hole, _) = over_run(Blend::Over, HALF);

    let mut misses = 0;
    let mut worst = 0.0f32;
    let mut moved = 0;
    for i in (0..bare.len()).filter(|i| i % 4 != 3) {
        let coverage = card[(i / 4) * 4 + 3];
        let expected = bare[i] * (1.0 - HALF * coverage);
        if !close(half_hole[i], expected) {
            misses += 1;
            worst = worst.max((half_hole[i] - expected).abs());
        }
        if !close(half_hole[i], bare[i]) {
            moved += 1;
        }
    }
    assert_eq!(
        misses,
        0,
        "under `over`, a fader at {HALF} is not `A*(1 - {HALF}*coverage)`: {misses} of {} \
         colour channels disagree, worst by {worst}",
        bare.len()
    );
    assert!(
        moved > 100,
        "a fader at {HALF} under `over` moved only {moved} colour channels, so this run \
         cannot see the fader at all"
    );

    // --- `max`: a crossfade *into* the maximum rather than a switch to it, so
    // a half fader is halfway between the layer under it and the maximum.
    let max_run = |blend: Blend, opacity: f32, off_air: Option<usize>| -> Vec<f32> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_blend(1, blend);
        deck.set_opacity(1, opacity);
        if let Some(slot) = off_air {
            deck.set_residency(slot, Residency::Allocated);
        }
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        decode(&readback(&gpu, present.hdr_texture()))
    };
    let a = max_run(Blend::Add, 1.0, Some(1));
    let b = max_run(Blend::Add, 1.0, Some(0));
    let halfway = max_run(Blend::Max, HALF, None);

    let mut misses = 0;
    let mut worst = 0.0f32;
    let mut moved = 0;
    for i in (0..halfway.len()).filter(|i| i % 4 != 3) {
        let expected = a[i] + HALF * (a[i].max(b[i]) - a[i]);
        if !close(halfway[i], expected) {
            misses += 1;
            worst = worst.max((halfway[i] - expected).abs());
        }
        if !close(halfway[i], a[i]) {
            moved += 1;
        }
    }
    assert_eq!(
        misses,
        0,
        "under `max`, a fader at {HALF} is not halfway to the maximum: {misses} of {} \
         colour channels disagree, worst by {worst}",
        halfway.len()
    );
    assert!(
        moved > 100,
        "a fader at {HALF} under `max` moved only {moved} colour channels away from the \
         layer under it, so this run cannot see the fader"
    );
}

/// **Zero gain silences `add` and `max`, and still covers under `over`.**
///
/// The other half of [`Blend::silent_at`] — the fader's half is asserted
/// against material that has gone NaN, in
/// `a_slot_faded_to_silence_cannot_take_the_mix_with_it`, because that is where
/// a skip and a multiply by zero stop agreeing.
///
/// Here the material is clean and the asymmetry is what is being pinned: a
/// layer at zero level contributes no colour, so under `add` and `max` it is
/// not there at all — and under `over` it is a black card, which covers. Slot 1
/// is [`L4_CARD`], so that difference is most of the frame rather than a few
/// bits.
#[test]
fn zero_gain_silences_add_and_max_and_still_covers_under_over() {
    let gpu = Gpu::headless().expect("no GPU available");

    let run = |residency: Residency, blend: Blend, gain: f32, opacity: f32| -> Vec<u16> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = Deck::new(
            &gpu.device,
            vec![
                HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                HotSwap::fixed(build_with(&gpu, L4_CARD, SEED_B, CAPACITY)),
            ],
            WIDTH,
            HEIGHT,
        );
        deck.set_residency(1, residency);
        deck.set_blend(1, blend);
        deck.set_gain(1, gain);
        deck.set_opacity(1, opacity);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        readback(&gpu, present.hdr_texture())
    };

    let parked = run(Residency::Allocated, Blend::Add, 1.0, 1.0);
    assert!(lit(&parked) > 100, "the surviving slot drew nothing");

    // The level silences where a layer contributing no colour contributes
    // nothing at all...
    for blend in [Blend::Add, Blend::Max] {
        assert_eq!(
            run(Residency::Live, blend, 0.0, 1.0),
            parked,
            "a slot at gain 0.0 under `{}` reached the mix",
            blend.name()
        );
    }
    // ...and does not under `over`, where zero gain is a black card and a black
    // card covers. Asserted rather than left as a comment, because it is the
    // one place the two faders stop being interchangeable and an operator
    // reaching for the wrong one gets a frame that goes dark instead of a
    // layer that goes away.
    //
    // **Colour channels only.** A whole-buffer `assert_ne!` would pass on the
    // alpha channel alone — coverage composes whatever the colour mode does, so
    // a slot that reached the mix and changed nothing visible still moves it —
    // and this claim is about what the picture does.
    let dark = decode(&run(Residency::Live, Blend::Over, 0.0, 1.0));
    let bright = decode(&parked);
    let darkened = (0..dark.len())
        .filter(|i| i % 4 != 3)
        .filter(|&i| dark[i] < bright[i])
        .count();
    assert!(
        darkened > 100,
        "a zero-gain `over` layer darkened only {darkened} colour channels, so it has \
         stopped covering — which would make gain and opacity the same control again"
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

    assert!(
        lit(&first) > 100 && lit(&second) > 100,
        "a slot drew nothing"
    );
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
    // **Colour only.** The fourth channel is coverage rather than a colour —
    // `1 - prod(1 - a_i)`, composed as `over` under every blend mode — and gain
    // deliberately does not reach it: turning a layer's level down dims what it
    // draws and does not change what it covers. So alpha is neither `2*A + B`
    // nor `2*(A + B)`, and including it here would be asserting linearity of a
    // channel this deck promises is not linear.
    for i in (0..mixed.len()).filter(|i| i % 4 != 3) {
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
        before_misses,
        0,
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
        names: karakuri_engine::swap::RequestNames::default(),
        edges: Vec::new(),
        id: 1,
        l1s: vec![(compile(L1), SWAPPED)],
        l2s: Vec::new(),
        l3s: Vec::new(),
        fields: Vec::new(),
        layering: karakuri_engine::set::Layering::Overdraw,
        published: Vec::new(),
        l4s: vec![compile(L4)],
        seed_salt: SEED_A,
        salts: Vec::new(),
        params: Vec::new(),
        bindings: Vec::new(),
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
    assert_eq!(
        deck.live_slots(),
        2,
        "the swap changed which slots are live"
    );
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
        names: karakuri_engine::swap::RequestNames::default(),
        edges: Vec::new(),
        id: 1,
        l1s: vec![(compile(L1), SWAPPED)],
        l2s: Vec::new(),
        l3s: Vec::new(),
        fields: Vec::new(),
        layering: karakuri_engine::set::Layering::Overdraw,
        published: Vec::new(),
        l4s: vec![compile(L4)],
        seed_salt: SEED_B,
        salts: Vec::new(),
        params: Vec::new(),
        bindings: Vec::new(),
        label: "off air".to_string(),
    })
    .expect("worker alive");

    let verdict = |deck: &mut Deck| -> Option<String> {
        deck.events(1).find_map(|e| match e {
            Event::Accepted {
                label, median_ms, ..
            } => Some(format!("Accepted `{label}` at {median_ms:.3} ms")),
            Event::RolledBack {
                label, median_ms, ..
            } => Some(format!("RolledBack `{label}` at {median_ms:.3} ms")),
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
    bare.resize(&gpu.device, W, H);
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
                set.resize(&gpu.device, W, H);
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
    let present = Present::new(
        &gpu.device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        WIDTH,
        HEIGHT,
    );

    // The deck moves, the target does not — the direction a window resize
    // takes if only one of the two handlers is wired.
    deck.resize(&gpu.device, WIDTH * 2, HEIGHT * 2);

    let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
    frame.render(present.hdr_view(), present.size(), 1);
}
