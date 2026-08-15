//! Transport, through a deck and a real GPU.
//!
//! `transport.rs`'s own tests cover the arithmetic — what mode maps the
//! session's clock to which step count, and which modes a Set may be given.
//! None of them touches a Set, which is the point of that split and also its
//! limit: they would all pass on a deck that computed a seek and then ignored
//! it.
//!
//! What is asserted here is the half that needs pixels:
//!
//! - a beat-synced slot **actually lands on the position**, rather than
//!   advancing towards it;
//! - scrubbing backwards **reproduces the earlier frame bit for bit**, which is
//!   the whole claim `closed_form` was introduced to make and the one that
//!   cannot be argued from arithmetic;
//! - tempo sync changes the rate and **nothing else**;
//! - and a free slot renders exactly what it rendered before the transport
//!   existed.

use karakuri_engine::deck::{Deck, Residency};
use karakuri_engine::swap::HotSwap;
use karakuri_engine::transport::{Refusal, Sync};
use karakuri_engine::{Gpu, Present, Set, Signals};
use karakuri_ir::typed::Checked;

const WIDTH: u32 = 128;
const HEIGHT: u32 = 128;
const CAPACITY: u32 = 1024;
const SEED: u32 = 19274;
const BPM: f32 = 120.0;

/// **Closed form, so it can be placed.** Nothing here reads an attribute it
/// emits, there is no `spawn` and no `kill()` — the three things
/// `is_closed_form` refuses on. `age = t` rather than `age + dt` for exactly
/// that reason, and it is not a dodge: a procedure with no `spawn` block has
/// every element alive from frame zero, so an element's age *is* `t`.
///
/// A ring whose angle is `t` alone, so the frame says where the clock is and
/// two frames at the same clock are the same frame.
const RING: &str = r#"
proc ring {
  kind     L1
  topology points
  capacity [1024, 262144] = 1024

  emit position, age

  element {
    let a = t * 3.1415927 + hash1(seed) * 1.2;
    let r = 1.2 + hash1(seed + 7u) * 0.8;
    position = vec3(cos(a) * r, sin(a) * r, 0.0);
    age      = t;
  }
}
"#;

/// **Accumulating**, so it cannot be placed: `position` reads what it emits,
/// which is the disqualifier. Used only to check that beat sync is refused.
const CREEP: &str = r#"
proc creep {
  kind     L1
  topology points
  capacity [1024, 262144] = 1024

  emit position, age

  element {
    let dir = sphere_point(hash1(seed), hash1(seed + 1000u));
    position = position + dir * dt * 2.0;
    age      = t;
  }
}
"#;

/// Closed form **and** written against the grid, which is the combination that
/// takes beat sync and refuses tempo sync.
const GRID_RING: &str = r#"
proc grid_ring {
  kind     L1
  topology points
  capacity [1024, 262144] = 1024

  emit position, age

  element {
    let a = beats * 3.1415927 + hash1(seed) * 1.2;
    position = vec3(cos(a) * 1.6, sin(a) * 1.6, 0.0);
    age      = t;
  }
}
"#;

const L4: &str = r#"
proc plain_points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(1.0, 0.9, 0.7, max(0.0, 1.0 - d));
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

fn build(gpu: &Gpu, l1: &str) -> Set {
    let mut set = Set::build(
        &gpu.device,
        &gpu.queue,
        &compile(l1),
        &compile(L4),
        CAPACITY,
        SEED,
    )
    .expect("the pair is compatible and the capacity is in range");
    set.resize(&gpu.device, WIDTH, HEIGHT);
    set
}

fn deck_of(gpu: &Gpu, l1: &str) -> Deck {
    let mut deck = Deck::new(
        &gpu.device,
        vec![HotSwap::fixed(build(gpu, l1))],
        WIDTH,
        HEIGHT,
    );
    deck.set_signals(Signals::new(BPM, u64::from(SEED)));
    deck
}

fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present) -> Vec<u16> {
    let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
    f.render(present.hdr_view(), present.size(), 1);
    f.finish();
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    readback(gpu, present.hdr_texture())
}

fn readback(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u16> {
    let (width, height) = (texture.width(), texture.height());
    let bytes_per_row = width * 8;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("transport readback"),
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

fn steps_taken(deck: &Deck) -> u64 {
    (deck.slot(0).set().time() * 60.0).round() as u64
}

// ---------------------------------------------------------------------------

/// **A free slot is what it was before the transport existed.** The default,
/// and the property that lets everything else be opt-in: a deck nobody has
/// arranged records exactly the frame it used to.
#[test]
fn a_free_slot_advances_by_the_sessions_own_steps() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, RING);
    assert_eq!(deck.transport(0).sync(), Sync::Free);

    for i in 1..=20 {
        frame(&gpu, &mut deck, &present);
        assert_eq!(steps_taken(&deck), i, "a free slot did not step once a frame");
    }
}

/// **Beat sync lands on the position rather than advancing towards it**, which
/// is the difference between a lock and a follow.
///
/// The slot starts thirty steps behind — half a second, one beat at 120 bpm —
/// and the very next frame is level with the session. A transport that stepped
/// towards the target would take thirty frames to close that, and would pass
/// every arithmetic test in `transport.rs` while doing so.
#[test]
fn beat_sync_lands_in_one_frame_rather_than_catching_up() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, RING);

    // Thirty frames off air: the session clock runs, the slot's does not.
    deck.set_residency(0, Residency::Allocated);
    for _ in 0..30 {
        frame(&gpu, &mut deck, &present);
    }
    assert_eq!(steps_taken(&deck), 0, "an allocated slot stepped");

    deck.set_residency(0, Residency::Live);
    deck.set_transport(0, Sync::Beat, BPM, 0.0)
        .expect("`ring` is closed form");
    frame(&gpu, &mut deck, &present);
    assert_eq!(
        steps_taken(&deck),
        31,
        "a beat-locked slot did not land on the session's position"
    );
}

/// **Scrubbing backwards reproduces the earlier frame, bit for bit.**
///
/// This is the claim `closed_form` exists to make, and the reason a rewind is
/// possible at all: the state at `t` does not depend on how the clock arrived
/// there, so putting the clock back and evaluating once gives the same image —
/// not a similar one.
///
/// The comparison is against a frame captured on the way past rather than
/// against a second run, so what is asserted is that *this* slot returned to
/// where *it* was, and not merely that two runs of the same procedure agree.
#[test]
fn scrubbing_back_a_beat_reproduces_the_frame_from_a_beat_ago() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = deck_of(&gpu, RING);
    deck.set_transport(0, Sync::Beat, BPM, 0.0)
        .expect("`ring` is closed form");

    // Every frame kept, so the assertion can name the step it expects rather
    // than the frame number it happens to be.
    let mut seen: Vec<(u64, Vec<u16>)> = Vec::new();
    for _ in 0..60 {
        let pixels = frame(&gpu, &mut deck, &present);
        seen.push((steps_taken(&deck), pixels));
    }
    assert!(lit(&seen[59].1) > 50, "the material drew nothing");

    // One beat at 120 bpm is half a second: thirty steps. The session advances
    // one more step on the frame below, so the slot lands at 61 - 30.
    deck.set_transport(0, Sync::Beat, BPM, -1.0)
        .expect("`ring` is closed form");
    let rewound = frame(&gpu, &mut deck, &present);
    let landed = steps_taken(&deck);
    assert_eq!(landed, 31, "the scrub did not land where a beat back is");

    let expected = seen
        .iter()
        .find(|(step, _)| *step == landed)
        .map(|(_, pixels)| pixels)
        .expect("step 31 was rendered on the way past");
    assert_eq!(
        &rewound, expected,
        "a rewound slot did not reproduce the frame it had drawn at that step"
    );
    // And it is a real rewind rather than a frame that happened to match:
    // the two neighbours differ, so the material is moving at this scale.
    assert_ne!(&rewound, &seen[59].1);
}

/// **Tempo sync changes the rate and nothing else.** The room at double the
/// anchor advances the slot twice a frame, and the picture is the picture that
/// slot would have drawn at that step under any other mode — the transport
/// decides *when*, never *what*.
#[test]
fn tempo_sync_scales_the_rate_and_leaves_the_material_alone() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

    // Anchored at half the session tempo, so the room is 2x and every frame is
    // two steps.
    let mut fast = deck_of(&gpu, RING);
    fast.set_transport(0, Sync::Tempo, BPM / 2.0, 0.0)
        .expect("`ring` takes tempo sync");
    let mut last = Vec::new();
    for i in 1..=20 {
        last = frame(&gpu, &mut fast, &present);
        assert_eq!(steps_taken(&fast), i * 2, "a 2x slot did not take two steps");
    }

    // The same forty steps reached free-running, in forty frames. Bit for bit:
    // a rate is a rate, and forty steps is forty steps however they were
    // spread over wall time.
    let mut free = deck_of(&gpu, RING);
    let mut plain = Vec::new();
    for _ in 0..40 {
        plain = frame(&gpu, &mut free, &present);
    }
    assert_eq!(steps_taken(&free), 40);
    assert!(lit(&plain) > 50, "the material drew nothing");
    assert_eq!(
        last, plain,
        "tempo sync changed the material rather than only its rate"
    );
}

/// **Both refusals, at the point the operator asks and against a real Set.**
///
/// `transport.rs` asserts the rule over the two flags; this asserts that the
/// flags reaching it are the ones the check pass put on the material. A rule
/// that is right about the wrong Set refuses nothing.
#[test]
fn a_set_is_refused_the_mode_its_material_cannot_take() {
    let gpu = Gpu::headless().expect("no GPU available");

    // Accumulating: a rate, but not a position.
    let mut creep = deck_of(&gpu, CREEP);
    assert_eq!(creep.sync_allowed(0, Sync::Tempo), Ok(()));
    assert_eq!(
        creep.set_transport(0, Sync::Beat, BPM, 0.0),
        Err(Refusal::NotClosedForm)
    );
    assert_eq!(
        creep.transport(0).sync(),
        Sync::Free,
        "a refused mode was applied anyway"
    );

    // Closed form and written against the grid: a position, but not a rate —
    // it already follows the room, and scaling its clock would make it follow
    // twice.
    let mut grid = deck_of(&gpu, GRID_RING);
    assert_eq!(grid.sync_allowed(0, Sync::Beat), Ok(()));
    assert_eq!(
        grid.set_transport(0, Sync::Tempo, BPM, 0.0),
        Err(Refusal::AlreadyOnTheGrid)
    );
    assert_eq!(grid.transport(0).sync(), Sync::Free);
    // And the mode it does take is applied.
    assert_eq!(grid.set_transport(0, Sync::Beat, BPM, 0.0), Ok(()));
    assert_eq!(grid.transport(0).sync(), Sync::Beat);
}
