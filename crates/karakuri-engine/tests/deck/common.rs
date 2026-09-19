pub use std::sync::mpsc;
pub use std::time::{Duration, Instant};

pub use karakuri_engine::binding::Curve;
pub use karakuri_engine::deck::{Blend, Deck, Mask, MaskKind, Residency};
pub use karakuri_engine::swap::{Event, HotSwap, Request};
pub use karakuri_engine::transition::{quantise, Control, Selection, Transition};
pub use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
pub use karakuri_ir::typed::Checked;

pub const WIDTH: u32 = 256;
pub const HEIGHT: u32 = 256;
pub const CAPACITY: u32 = 4096;

/// Two seeds, so that two slots hold visibly different material and a mix of
/// them is not the same picture twice.
pub const SEED_A: u32 = 19274;
pub const SEED_B: u32 = 88888;

/// The capacity a swapped-in Set is built at, differing from [`CAPACITY`] so
/// that which Set is live in a slot is observable from outside.
pub const SWAPPED: u32 = 8192;

/// A budget no frame in these tests will come near: they are about the deck,
/// not about the watchdog, and a slot stopping in the middle of one would
/// be measuring the host's mood.
pub const GENEROUS_MS: f32 = 10_000.0;

pub const PATIENCE: Duration = Duration::from_secs(30);

pub const L1: &str = r#"
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

/// The renderer every assertion in this file draws with.
///
/// **`point_rate` is a fraction of the target's height**, so the number
/// below is a sprite size only once a height is named: 0.015625 is four
/// texels at [`HEIGHT`]. The assertions render 128, 256 or 512 high and
/// compare pictures taken at one size against each other, so what a sprite
/// costs never enters them. **A test that renders somewhere else and cares
/// what a sprite costs scales the rate to its own height instead of reusing
/// this one** — 0.015625 is 11.25 texels at 720, which is 7.9 times the
/// area, and additive blending is paid by area.
/// [`the_cost_of_a_slot_and_of_the_composite_are_measured_and_reported`]
/// renders at 1280x720 and does that; it did not between 2026-09-02 and
/// 2026-09-07, and a quarter to two fifths of every figure it printed in
/// that window was this constant rather than the deck.
pub const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.015625;
  }

  fragment {
let d = length(point_coord * 2.0 - 1.0);
color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

/// A second renderer over the same geometry, differing only in its name.
///
/// Which is the whole point: two ways of drawing one simulation is what a
/// Set holds several L4s *for*, and what selecting between them is about.
/// A name of its own because a name addresses a node and two nodes cannot
/// share one.
pub const L4_B: &str = r#"
proc harder_points {
  kind  L4
  blend additive

  consumes position

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.0078125;
  }

  fragment {
color = vec4(1.0, 0.5, 0.25, 1.0);
  }
}
"#;

/// The same shape, rendering a NaN. `sqrt` of a negative is a procedure that
/// parses, type-checks, costs, compiles and runs — nothing in the pipeline
/// rejects it, and a generated L4 reaches this by dividing by a parameter or
/// normalizing a zero vector as easily as by this. What a slot holding one
/// must not be able to do is reach a mix it was faded out of.
pub const L4_NAN: &str = r#"
proc nan_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.015625;
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
pub const L4_CARD: &str = r#"
proc opaque_card {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.1875;
  }

  fragment {
color = vec4(vec3(0.0, 0.0, 0.0) * exposure, 1.0);
  }
}
"#;

/// The ordinary material, drawn wide enough to be under the card everywhere it
/// covers. Used where a test needs the two layers to actually overlap rather
/// than to overlap wherever the seeds happened to put them.
pub const L4_WIDE: &str = r#"
proc wide_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.1875;
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
pub const OVERDRAWN_ALPHA: [(&str, &str); 4] = [
    ("above one", "1.5 * exposure"),
    ("negative", "0.0 - exposure"),
    ("infinite", "exposure / 0.0"),
    ("NaN", "sqrt(0.0 - exposure)"),
];

pub fn overdrawn_card(alpha: &str) -> String {
    format!(
        r#"
proc overdrawn_card {{
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {{
clip       = camera * vec4(position, 1.0);
point_rate = 0.1875;
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
pub const L4_WASH: &str = r#"
proc wash {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
let ignored = position;
clip       = vec4(0.0, 0.0, 0.0, 1.0);
// Twice the frame's height, so one sprite covers a square target with
// margin to spare. `point_rate` is a fraction of the height, so this is
// the one fixture here whose number does not depend on [`MASK_SIZE`].
point_rate = 2.0;
  }

  fragment {
color = vec4(vec3(1.0, 1.0, 1.0) * exposure, 1.0);
  }
}
"#;

/// The target the mask tests render at. Small, because [`L4_WASH`] overdraws
/// the whole frame once per element and the number that matters is coverage
/// rather than resolution.
pub const MASK_SIZE: u32 = 64;

/// A deck whose slot 1 covers the frame, at [`MASK_SIZE`].
pub fn wash_deck(gpu: &Gpu) -> Deck {
    let swaps = vec![
        HotSwap::fixed(build(gpu, SEED_A, CAPACITY)),
        HotSwap::fixed(build_with(gpu, L4_WASH, SEED_B, CAPACITY)),
    ];
    let mut deck = Deck::new(&gpu.device, swaps, MASK_SIZE, MASK_SIZE);
    deck.resize(&gpu.device, MASK_SIZE, MASK_SIZE);
    deck
}

pub fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    let checked =
        karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

pub fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build(gpu: &Gpu, seed: u32, capacity: u32) -> Set {
    build_with(gpu, L4, seed, capacity)
}

pub fn build_with(gpu: &Gpu, l4: &str, seed: u32, capacity: u32) -> Set {
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
pub fn deck_of(gpu: &Gpu, seeds: &[u32]) -> Deck {
    deck_of_at(gpu, seeds, WIDTH, HEIGHT)
}

pub fn deck_of_at(gpu: &Gpu, seeds: &[u32], width: u32, height: u32) -> Deck {
    let swaps = seeds
        .iter()
        .map(|&seed| HotSwap::fixed(build(gpu, seed, CAPACITY)))
        .collect();
    Deck::new(&gpu.device, swaps, width, height)
}

/// One build request for the pair this file compiles, at [`SWAPPED`] so that
/// which Set a slot is holding is observable from outside.
pub fn candidate(id: u64) -> Request {
    Request {
        names: karakuri_engine::swap::RequestNames::default(),
        edges: Vec::new(),
        id,
        l1s: vec![(compile(L1), SWAPPED)],
        l2s: Vec::new(),
        l3s: Vec::new(),
        fields: Vec::new(),
        layering: karakuri_engine::set::Layering::Overdraw,
        live: None,
        published: Vec::new(),
        l4s: vec![compile(L4)],
        seed_salt: SEED_B,
        camera: karakuri_engine::camera::Orbit::default(),
        salts: Vec::new(),
        params: Vec::new(),
        bindings: Vec::new(),
        authorities: Vec::new(),
        label: "candidate".to_string(),
    }
}

/// **A verdict, as the three things it is**: whether the candidate was kept,
/// the number it was decided on, and which of the slot's two numbers that
/// was. `None` for every other event.
///
/// `cost_ms` is an `Option` on the favourable side and not on the other,
/// which is `swap::Event`'s own asymmetry: a candidate nothing could measure
/// is kept, run, and reported as unjudged, and a slot can only be stopped
/// through a number.
///
/// **The `bool` is whether the slot goes on running, not whether the
/// candidate stayed.** Since ADR-0316 the candidate stays either way.
pub fn said_verdict(e: Event) -> Option<(bool, Option<f32>, karakuri_engine::Basis)> {
    match e {
        Event::Accepted { cost_ms, basis, .. } => Some((true, cost_ms, basis)),
        Event::Overloaded { cost_ms, basis, .. } => Some((false, Some(cost_ms), basis)),
        _ => None,
    }
}

/// One frame, shaped the way a caller has to shape it: the guard owns the
/// encoder, so there is no other shape available.
///
/// The `poll` afterwards is the harness standing in for vsync, exactly as in
/// `tests/hot_swap.rs`: it bounds a headless loop that would otherwise queue
/// thousands of command buffers ahead of the GPU. It is the submit-and-wait
/// the render thread must never do, and it is not inside the frame.
pub fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present, steps: u8) {
    let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
    f.render(present.hdr_view(), present.size(), steps);
    f.finish();
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
}

/// One frame of a bare Set, the way `karakuri-cli` and every earlier test
/// record one. The comparison in
/// [`a_deck_of_one_is_a_bare_set_bit_for_bit`] is only worth something if this
/// is the *old* path rather than a second spelling of the new one.
pub fn bare_frame(gpu: &Gpu, set: &mut Set, present: &Present, steps: u8) {
    // No bindings on these Sets, so the session clock is inert here and
    // nothing reads a signal; the real one belongs to the deck, and
    // `tests/binding.rs` is where it is asserted.
    set.prepare(&gpu.queue, steps, &Signals::default());
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    set.render(&mut encoder, present.hdr_view(), steps);
    gpu.queue.submit([encoder.finish()]);
    set.commit();
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
}

/// The raw `f16` bits of an `Rgba16Float` texture, four per texel. Raw rather
/// than decoded so that an exact comparison is a comparison of bits and not of
/// two float expressions that happen to agree.
pub fn readback(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u16> {
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
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = slice.get_mapped_range().expect("map");
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
pub fn f16(bits: u16) -> f32 {
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

pub fn decode(pixels: &[u16]) -> Vec<f32> {
    pixels.iter().copied().map(f16).collect()
}

pub fn lit(pixels: &[u16]) -> usize {
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
pub fn steps_taken(set: &Set) -> u64 {
    (set.time() * 60.0).round() as u64
}
