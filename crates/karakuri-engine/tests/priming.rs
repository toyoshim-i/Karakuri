//! Priming: a deck slot warming its element buffers out of sight.
//!
//! What is asserted here is the claim `deck.rs` makes in "Priming steps; it
//! does not draw", in the order that module argues it:
//!
//! - a Priming slot **steps** — its `t` advances — and reaches the mix in no
//!   way at all;
//! - it may step on only some frames, and then its `t` advances slower than
//!   wall time, which is what "reduced rate" means;
//! - **a slot primed for thirty frames and then put on air is bit-for-bit the
//!   slot that was on air for thirty-one**, which is the whole justification
//!   for skipping the render rather than shrinking it;
//! - and it is visibly different from the same slot taken from Allocated
//!   straight to Live, or none of the above would be worth anything;
//! - determinism survives all of it, including a slot that primes for a while
//!   and then goes Live.
//!
//! The material is deliberately an *accumulating* procedure — elements drift
//! outward from the origin at a fixed rate — because a closed-form one would
//! look identical warm and cold and every test below would pass on a deck that
//! never primed anything. Cold, every element is at the origin and the frame is
//! one small blob; warm, they are on a sphere. That gap is what "shows warmed
//! state" is measured by.

use karakuri_engine::deck::{Deck, Residency};
use karakuri_engine::swap::HotSwap;
use karakuri_engine::{Gpu, Present, Set};
use karakuri_ir::typed::Checked;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
const CAPACITY: u32 = 4096;

const SEED_A: u32 = 19274;
const SEED_B: u32 = 88888;

/// **Accumulating on purpose.** `position = position + ...` reads what it
/// emits, so the state at `t` is the sum of every step taken to get there and
/// nothing but running it forward produces it. `karakuri-ir` classifies this
/// as not closed form, which is what makes it a legitimate thing to prime;
/// `tests/governor.rs` asserts the other side.
///
/// Cold, every element sits at the origin — `Set::initialize` zeroes every
/// attribute — so the frame is one blob. After thirty steps they are on a
/// sphere of radius `30 * dt * 4`, about two units.
const CREEP: &str = r#"
proc creep {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param speed : float [0.0, 16.0] = 4.0

  emit position, age

  element {
    let dir = sphere_point(hash1(seed), hash1(seed + 1000u));
    position = position + dir * dt * speed;
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

fn build(gpu: &Gpu, seed: u32) -> Set {
    let mut set = Set::build(
        &gpu.device,
        &gpu.queue,
        &compile(CREEP),
        &compile(L4),
        CAPACITY,
        seed,
    )
    .expect("the pair is compatible and the capacity is in range");
    set.resize(WIDTH, HEIGHT);
    set
}

/// A deck of fixed Sets: no worker, so nothing can swap and a run is a pure
/// function of the ticks it is given.
fn deck_of(gpu: &Gpu, seeds: &[u32]) -> Deck {
    let swaps = seeds
        .iter()
        .map(|&seed| HotSwap::fixed(build(gpu, seed)))
        .collect();
    Deck::new(&gpu.device, swaps, WIDTH, HEIGHT)
}

fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present, steps: u8) {
    let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
    f.render(present.hdr_view(), present.size(), steps);
    f.finish();
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
}

/// Raw `f16` bits, four per texel — raw so that an exact comparison compares
/// bits rather than two float expressions that happen to agree.
fn readback(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u16> {
    let (width, height) = (texture.width(), texture.height());
    let bytes_per_row = width * 8;
    assert_eq!(bytes_per_row % 256, 0, "row pitch must be 256-byte aligned");

    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("priming readback"),
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

fn lit(pixels: &[u16]) -> usize {
    pixels
        .chunks_exact(4)
        .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
        .count()
}

/// Simulation steps taken, recovered as an integer. `Set::time` is
/// `steps * dt`, and comparing that against a float expression meaning the same
/// thing is the last-bit trap `Set` derives `t` from an integer counter to
/// avoid.
fn steps_taken(set: &Set) -> u64 {
    (set.time() * 60.0).round() as u64
}

// ---------------------------------------------------------------------------

/// **A Priming slot steps, and contributes nothing to the mix.**
///
/// Both halves matter and each would pass alone on a broken implementation.
/// "Steps" alone passes on a slot that is also being composited — which is just
/// Live under another name. "Contributes nothing" alone passes on a slot that
/// is doing nothing at all — which is Allocated under another name. The mix is
/// compared bit for bit against a deck holding only the other slot, on the same
/// ticks, so the priming slot's absence from it is exact rather than small.
#[test]
fn a_priming_slot_advances_its_t_and_contributes_nothing_to_the_mix() {
    let gpu = Gpu::headless().expect("no GPU available");
    const FRAMES: usize = 20;

    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
    deck.set_residency(1, Residency::Priming);
    assert_eq!(deck.live_slots(), 1);
    assert_eq!(deck.priming_slots(), 1);

    for _ in 0..FRAMES {
        frame(&gpu, &mut deck, &present, 1);
    }
    let mixed = readback(&gpu, present.hdr_texture());

    assert_eq!(
        steps_taken(deck.slot(1).set()),
        FRAMES as u64,
        "a Priming slot did not advance its `t`; nothing is calling `prepare` on it \
         and it is Allocated under another name"
    );
    assert_eq!(
        steps_taken(deck.slot(0).set()),
        FRAMES as u64,
        "the Live slot did not step normally alongside a priming one"
    );

    let solo_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut solo = deck_of(&gpu, &[SEED_A]);
    for _ in 0..FRAMES {
        frame(&gpu, &mut solo, &solo_present, 1);
    }
    let expected = readback(&gpu, solo_present.hdr_texture());

    assert!(
        lit(&expected) > 100,
        "the remaining slot drew nothing, so this comparison is two black frames"
    );
    assert_eq!(
        mixed, expected,
        "a Priming slot reached the mix — it is stepped, and it must not be drawn"
    );
    // And it drew nothing into its own target either, which is the sharper
    // version of the same claim: the composite could have been skipping it
    // while a render pass still ran and still cost.
    assert_eq!(
        lit(&readback(&gpu, deck.slot_target(1))),
        0,
        "a Priming slot rendered into its own target; priming skips the render \
         entirely, it does not render somewhere nobody looks"
    );
    // No level, for the same reason an off-air slot has none: the meter cannot
    // vouch that what it measured is what the slot is showing, because the slot
    // is not showing anything.
    assert!(deck.level(1).is_none());
}

/// **Reduced rate.** A slot priming one frame in three has taken a third of the
/// steps after thirty frames, and its `t` is a third of wall time. That is
/// correct rather than a defect: it is catching up, not keeping time.
#[test]
fn a_priming_slot_at_a_reduced_rate_advances_slower_than_wall_time() {
    let gpu = Gpu::headless().expect("no GPU available");
    const FRAMES: usize = 30;
    const ONE_IN: u32 = 3;

    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
    deck.set_residency(1, Residency::Priming);
    deck.set_prime_one_in(1, ONE_IN);

    for _ in 0..FRAMES {
        frame(&gpu, &mut deck, &present, 1);
    }

    assert_eq!(
        steps_taken(deck.slot(1).set()),
        (FRAMES as u64) / u64::from(ONE_IN),
        "a slot priming one frame in {ONE_IN} did not take a third of the steps"
    );
    assert_eq!(
        steps_taken(deck.slot(0).set()),
        FRAMES as u64,
        "the rate leaked onto the Live slot"
    );
}

/// **A rate change, and a return to priming, both start from a defined
/// frame.**
///
/// The rate is a modulus over a counter, and a counter left running across a
/// change makes "one frame in four" mean "one frame in four, offset by however
/// long the slot happened to have been doing something else". That offset is
/// unobservable from any single run and reproduces only by accident — two
/// record streams that reach the same state by different routes would prime on
/// different frames.
///
/// Observed at the frame after each change rather than by counting steps over a
/// window: over a long enough window a stale phase produces the same *number*
/// of steps, just at different instants, so a count would not see it.
#[test]
fn a_rate_change_takes_effect_from_a_defined_frame() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, &[SEED_A]);
    deck.set_residency(0, Residency::Priming);

    // Five frames at full rate leaves the counter at five, which is not a
    // multiple of four.
    for _ in 0..5 {
        frame(&gpu, &mut deck, &present, 1);
    }
    assert_eq!(steps_taken(deck.slot(0).set()), 5);

    deck.set_prime_one_in(0, 4);
    frame(&gpu, &mut deck, &present, 1);
    assert_eq!(
        steps_taken(deck.slot(0).set()),
        6,
        "the first frame after a rate change did not step; the counter carried \
         over from before the change instead of starting from it"
    );

    // Same again across a residency change: park it at a counter that is not a
    // multiple of the rate, bring it back, and the first frame back must step.
    for _ in 0..2 {
        frame(&gpu, &mut deck, &present, 1);
    }
    assert_eq!(steps_taken(deck.slot(0).set()), 6, "one in four stepped early");
    deck.set_residency(0, Residency::Allocated);
    frame(&gpu, &mut deck, &present, 1);
    deck.set_residency(0, Residency::Priming);
    frame(&gpu, &mut deck, &present, 1);
    assert_eq!(
        steps_taken(deck.slot(0).set()),
        7,
        "the first frame back in Priming did not step; the counter survived a trip \
         through Allocated"
    );
}

/// **A Priming slot that goes Live shows warmed state**, and the warming is
/// exactly the warming being on air would have done.
///
/// Three runs of the same material, same seed, same ticks, differing only in
/// what the slot was doing for the first thirty frames:
///
/// ```text
///   primed   30 frames Priming,   then 1 Live   →  t = 31/60
///   always   31 frames Live                     →  t = 31/60
///   cold     30 frames Allocated, then 1 Live   →  t =  1/60
/// ```
///
/// `primed == always`, **bit for bit**. That is the claim that justifies
/// skipping the render rather than shrinking it: all of a Set's state is L1's,
/// so running L1 and not drawing reaches the identical state, and nothing about
/// the frames a slot spent priming is recoverable from the picture afterwards.
/// It can be exact because it is the same arithmetic in the same order — a
/// tolerance here would be hiding a real difference rather than absorbing a
/// rounding one.
///
/// `primed != cold`, by a lot and in the direction warmth predicts: cold, every
/// element is still at the origin and the frame is one blob; warm, they are
/// spread over a sphere. Asserted as a ratio of lit pixels rather than as
/// inequality, because two frames of an accumulating procedure differ at *some*
/// bit almost whatever happens, and the point is that the difference is the
/// warming.
#[test]
fn a_priming_slot_that_goes_live_shows_warmed_state() {
    let gpu = Gpu::headless().expect("no GPU available");
    const WARM: usize = 30;

    // One slot, so the mix is that slot and there is nothing else in the frame
    // to explain a difference.
    let run = |before: Residency, live_frames: usize| -> (Vec<u16>, u64) {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A]);
        deck.set_residency(0, before);
        for _ in 0..WARM {
            frame(&gpu, &mut deck, &present, 1);
        }
        deck.set_residency(0, Residency::Live);
        for _ in 0..live_frames {
            frame(&gpu, &mut deck, &present, 1);
        }
        (
            readback(&gpu, present.hdr_texture()),
            steps_taken(deck.slot(0).set()),
        )
    };

    let (primed, primed_t) = run(Residency::Priming, 1);
    let (cold, cold_t) = run(Residency::Allocated, 1);
    // The control: never off air at all.
    let (always, always_t) = run(Residency::Live, 1);

    assert_eq!(primed_t, WARM as u64 + 1);
    assert_eq!(always_t, WARM as u64 + 1);
    assert_eq!(cold_t, 1, "the Allocated slot stepped while it was parked");

    assert!(
        lit(&always) > 200,
        "the warmed material lit only {} pixels, so the ratio below is measuring \
         noise",
        lit(&always)
    );
    assert_eq!(
        primed, always,
        "a slot primed for {WARM} frames and then put on air is not the slot that \
         was on air for {} — priming is not reaching the same state that being \
         live reaches",
        WARM + 1
    );

    // And the warming is visible, or the equality above would be satisfied by a
    // Priming slot that did nothing at all in a deck where being Live also did
    // nothing.
    let (warm_px, cold_px) = (lit(&primed), lit(&cold));
    assert!(
        warm_px > cold_px * 4,
        "warmed lit {warm_px} pixels and cold lit {cold_px}; cold is supposed to be \
         every element still stacked at the origin and warm a sphere, so this run \
         could not tell a primed slot from an unprimed one"
    );
}

/// **Determinism survives priming**, including the transition out of it.
///
/// Uneven ticks, a slot that primes at a reduced rate for a while and then goes
/// on air, and a second slot live throughout. Two runs, bit for bit. A rate
/// counter that started from wherever it happened to be, or a priming decision
/// that read a clock, shows up here — verified by making the step decision
/// genuinely random, which this fails on.
///
/// What it does **not** cover, despite being the file's determinism test: the
/// governor's index order. `govern` is not called here and is not on the frame
/// path, and randomising the order slots are *stepped* in is invisible to a
/// pixel comparison anyway — slots do not interact, each renders into its own
/// target, and the composite reads them by index afterwards. The governor's
/// order is asserted directly in `tests/governor.rs`, where it is observable.
#[test]
fn priming_then_going_live_reproduces_bit_identically() {
    let gpu = Gpu::headless().expect("no GPU available");
    const TICKS: [u8; 12] = [1, 2, 1, 3, 1, 1, 4, 2, 1, 3, 2, 1];

    let run = || -> Vec<u16> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_residency(1, Residency::Priming);
        deck.set_prime_one_in(1, 2);
        for steps in TICKS {
            frame(&gpu, &mut deck, &present, steps);
        }
        // On air part-way through, at whatever `t` it warmed to.
        deck.set_residency(1, Residency::Live);
        deck.set_gain(1, 0.75);
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
        "two runs of the same ticks, the same seeds and the same priming schedule \
         composited differently"
    );
}

/// **The identity holds at a reduced rate too**, which the test above does not
/// say: it primes at one frame in one, where "primed then Live" and "always
/// Live" run the same number of steps on the same frames and an implementation
/// that ignored the rate entirely would still pass.
///
/// Sixty frames at one-in-two is thirty steps, and thirty-one frames Live is
/// thirty-one — the same `t`, reached by two different routes through wall
/// clock. Bit for bit.
///
/// **The material has no signal binding, and that is load-bearing rather than
/// incidental.** A Priming slot is handed the *session's* signals, at the
/// current frame's phase, however few steps it has taken; so a bound parameter
/// on a slot priming at one frame in `n` is sampled at a phase its own `t`
/// never reaches, and the equality below does not hold for it. See "Priming
/// steps; it does not draw" in `deck.rs` for what the claim is narrowed to.
#[test]
fn priming_at_a_reduced_rate_reaches_the_same_state_as_being_live() {
    let gpu = Gpu::headless().expect("no GPU available");
    const PRIME_FRAMES: usize = 60;
    const ONE_IN: u32 = 2;
    const STEPS: u64 = (PRIME_FRAMES as u64) / (ONE_IN as u64) + 1;

    let run = |primed: bool| -> Vec<u16> {
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A]);
        if primed {
            deck.set_residency(0, Residency::Priming);
            deck.set_prime_one_in(0, ONE_IN);
            for _ in 0..PRIME_FRAMES {
                frame(&gpu, &mut deck, &present, 1);
            }
            deck.set_residency(0, Residency::Live);
            frame(&gpu, &mut deck, &present, 1);
        } else {
            for _ in 0..STEPS {
                frame(&gpu, &mut deck, &present, 1);
            }
        }
        assert_eq!(steps_taken(deck.slot(0).set()), STEPS);
        readback(&gpu, present.hdr_texture())
    };

    let (primed, always) = (run(true), run(false));
    assert!(lit(&always) > 200, "the material drew nothing to compare");
    assert_eq!(
        primed, always,
        "a slot primed at one frame in {ONE_IN} and then put on air is not the slot \
         that was on air throughout, at the same `t`"
    );
}
