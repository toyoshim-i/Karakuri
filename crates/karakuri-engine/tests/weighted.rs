//! `blend weighted`, asserted in pixels.
//!
//! The claims worth having here are all comparisons against `additive`, because
//! almost everything that could go wrong with weighted blended OIT produces a
//! picture rather than an error: a revealage cleared to the wrong value, a
//! resolve that forgot to normalise, a depth weight measured against nothing.
//! Each of those is a plausible frame. So every test below draws the *same*
//! material both ways and names what has to differ, or what has to not.
//!
//! Two sprites overlapping is the smallest fixture that can tell the modes
//! apart at all: one sprite alone is a case where the two agree exactly, which
//! is itself one of the claims.

use karakuri_engine::{Gpu, Present, Set, SetError, Signals, VideoSource};
use karakuri_ir::typed::Checked;

const W: u32 = 64;
const H: u32 = 64;

/// Two elements at fixed positions, no spawning, no motion — the picture is a
/// pure function of `seed` so both modes get literally identical geometry.
///
/// `seed 0` sits at the origin and `seed 1` three units toward the eye and a
/// little to the side, so the two sprites partly overlap while their depths
/// differ by a third of the orbit's radius. Everything the weighted path
/// computes per fragment is a function of that depth.
///
/// The camera orbits, so this only holds near `t = 0` — every test here draws
/// one step and no more.
const PAIR_L1: &str = r#"
proc pair {
  kind     L1
  topology points
  capacity [2, 2] = 2

  emit position

  element {
    var x = 0.0;
    var z = 0.0;
    if seed == 1u {
      x = 3.0;
      z = 0.6;
    }
    position = vec3(x, 0.0, z);
  }
}
"#;

/// A flat, opaque-ish sprite: a constant colour over the whole quad, so a texel
/// inside the overlap is covered by exactly two fragments of known alpha.
///
/// `alpha` and the two colours are params so a test can vary them without a
/// second fixture.
fn sprite_l4(blend: &str) -> String {
    format!(
        r#"
proc flat_sprite {{
  kind  L4
  blend {blend}

  param alpha : float [0.0, 2.0] = 0.5

  consumes position

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_size = 40.0;
  }}

  fragment {{
    var c = vec3(1.0, 0.0, 0.0);
    if seed == 1u {{
      c = vec3(0.0, 0.0, 1.0);
    }}
    color = vec4(c, alpha);
  }}
}}
"#
    )
}

fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
}

fn try_build(gpu: &Gpu, l1: &str, l4: &str) -> Result<Set, SetError> {
    Set::build(&gpu.device, &gpu.queue, &compile(l1), &compile(l4), 2, 3)
}

fn build(gpu: &Gpu, l4: &str) -> Set {
    let mut set = try_build(gpu, PAIR_L1, l4).expect("a compatible pair");
    set.resize(&gpu.device, W, H);
    set
}

/// RGBA f32 per texel, after one step.
fn draw(gpu: &Gpu, set: &mut Set) -> Vec<[f32; 4]> {
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
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
    let out: Vec<[f32; 4]> = data
        .chunks_exact(8)
        .map(|t| {
            let h = |i: usize| f16(u16::from_le_bytes([t[i], t[i + 1]]));
            [h(0), h(2), h(4), h(6)]
        })
        .collect();
    drop(data);
    readback.unmap();
    out
}

/// `f16` bits to `f32`. Written out rather than pulled in as a dependency, the
/// same way `tests/deck.rs`, `tests/lines.rs` and `tests/fullscreen.rs` do it.
fn f16(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = (bits >> 10) & 0x1f;
    let mantissa = bits & 0x03ff;
    let magnitude = match exponent {
        0 => f32::from(mantissa) * 2.0f32.powi(-24),
        0x1f if mantissa == 0 => f32::INFINITY,
        0x1f => f32::NAN,
        e => (1.0 + f32::from(mantissa) / 1024.0) * 2.0f32.powi(i32::from(e) - 15),
    };
    sign * magnitude
}

fn at(px: &[[f32; 4]], x: u32, y: u32) -> [f32; 4] {
    px[(y * W + x) as usize]
}

/// The brightest texel in the frame, by luminance-ish sum. Used where the claim
/// is about the material as a whole rather than about a chosen coordinate — a
/// test that reads one hard-coded pixel of a camera-projected sprite is a test
/// that breaks when the default orbit moves.
fn brightest(px: &[[f32; 4]]) -> [f32; 4] {
    *px.iter()
        .max_by(|a, b| {
            let s = |t: &[f32; 4]| t[0] + t[1] + t[2];
            s(a).partial_cmp(&s(b)).expect("no NaN in the frame")
        })
        .expect("a non-empty frame")
}

/// Every texel any material covered, in both frames. The overlap is what the
/// two modes disagree about, and it is where the interesting texels are.
fn covered(px: &[[f32; 4]]) -> Vec<usize> {
    px.iter().enumerate().filter(|(_, t)| t[3] > 0.01).map(|(i, _)| i).collect()
}

/// **Coverage never exceeds 1, which additive's does not promise.** This is the
/// mode's whole point restated as a number: two half-covering sprites over one
/// another cover three quarters, not one and a half, because `1 - prod(1 - a)`
/// is a probability where a sum is not.
///
/// Asserted on the alpha channel because that is what L5 composites with. An
/// additive slot at the same alpha hands the mix a coverage above 1, which
/// `composite.wgsl` then has to saturate.
#[test]
fn weighted_coverage_saturates_where_additive_coverage_sums() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut additive = build(&gpu, &sprite_l4("additive"));
    let mut weighted = build(&gpu, &sprite_l4("weighted"));
    let a = draw(&gpu, &mut additive);
    let w = draw(&gpu, &mut weighted);

    let overlap: Vec<usize> = covered(&a)
        .into_iter()
        .filter(|&i| a[i][3] > 0.7)
        .collect();
    assert!(
        !overlap.is_empty(),
        "the fixture's sprites do not overlap, so there is nothing to tell the modes apart"
    );

    for i in overlap {
        assert!(
            (a[i][3] - 0.75).abs() < 0.02,
            "additive coverage at {i} is {}, expected 1 - 0.5^2 under two sprites of alpha 0.5",
            a[i][3]
        );
        assert!(
            (w[i][3] - 0.75).abs() < 0.02,
            "weighted coverage at {i} is {}, and it has to be the same probability",
            w[i][3]
        );
    }
}

/// **The near sprite wins the overlap, and under `additive` neither does.**
///
/// The two sprites are red at the origin and blue three units nearer the eye.
/// Additive sums them, so the overlap is the same magenta whichever is in front.
/// Weighted resolves toward the nearer one, so blue has to come out ahead of red
/// — which is the whole reason the weight is a function of depth rather than a
/// constant.
#[test]
fn the_nearer_sprite_dominates_the_overlap_only_under_weighted() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut additive = build(&gpu, &sprite_l4("additive"));
    let mut weighted = build(&gpu, &sprite_l4("weighted"));
    let a = draw(&gpu, &mut additive);
    let w = draw(&gpu, &mut weighted);

    let overlap: Vec<usize> = covered(&a).into_iter().filter(|&i| a[i][3] > 0.7).collect();
    assert!(!overlap.is_empty(), "the sprites do not overlap");
    let mid = overlap[overlap.len() / 2];

    assert!(
        (a[mid][0] - a[mid][2]).abs() < 0.05,
        "additive put the two sprites at different levels ({:?}), so it is not the control it \
         is being used as",
        a[mid]
    );
    assert!(
        w[mid][2] > w[mid][0] * 1.05,
        "the nearer (blue) sprite did not dominate the weighted overlap: {:?}",
        w[mid]
    );
}

/// **One layer per texel is where the two modes agree exactly**, and this is the
/// claim `SetError::WeightedFullscreen` rests on: for a single fragment the
/// resolve gives back `c * a`, which is what additive blending into a cleared
/// target leaves. Outside the overlap, this fixture is that case.
///
/// Not asserted bit for bit: the weighted path multiplies by a weight and then
/// divides it out again, so the two differ by whatever `f32` rounding that costs
/// before both are stored as `f16`.
#[test]
fn a_lone_sprite_resolves_to_what_additive_accumulates() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut additive = build(&gpu, &sprite_l4("additive"));
    let mut weighted = build(&gpu, &sprite_l4("weighted"));
    let a = draw(&gpu, &mut additive);
    let w = draw(&gpu, &mut weighted);

    // Covered by exactly one sprite: coverage is a single alpha of 0.5 rather
    // than the overlap's 0.75.
    let lone: Vec<usize> = covered(&a).into_iter().filter(|&i| a[i][3] < 0.6).collect();
    assert!(lone.len() > 100, "only {} texels are covered by one sprite", lone.len());

    for i in lone {
        for c in 0..4 {
            assert!(
                (a[i][c] - w[i][c]).abs() < 0.01,
                "channel {c} at texel {i}: additive {:?} against weighted {:?}",
                a[i],
                w[i]
            );
        }
    }
}

/// **Alpha above 1 is clamped**, which `additive` does not do and the ir-spec
/// says so beside the declaration. Under additive an alpha of 2.0 doubles what
/// the fragment adds; under weighted it is opacity, and a revealage of
/// `prod(1 - 2)` is negative light.
///
/// The control is what makes this a test of the clamp rather than of the fixture:
/// additive at 2.0 has to be visibly brighter than additive at 1.0, or the
/// parameter never reached the shader.
#[test]
fn an_alpha_above_one_is_clamped_under_weighted_and_not_under_additive() {
    let gpu = Gpu::headless().expect("no GPU available");

    let mut bright = build(&gpu, &sprite_l4("additive"));
    bright.params.insert("alpha".to_string(), 2.0);
    let mut normal = build(&gpu, &sprite_l4("additive"));
    normal.params.insert("alpha".to_string(), 1.0);
    let bright = brightest(&draw(&gpu, &mut bright));
    let normal = brightest(&draw(&gpu, &mut normal));
    assert!(
        bright[0] + bright[1] + bright[2] > (normal[0] + normal[1] + normal[2]) * 1.5,
        "the `alpha` param never reached the additive shader: {bright:?} against {normal:?}"
    );

    let mut over = build(&gpu, &sprite_l4("weighted"));
    over.params.insert("alpha".to_string(), 2.0);
    let mut unit = build(&gpu, &sprite_l4("weighted"));
    unit.params.insert("alpha".to_string(), 1.0);
    let over = draw(&gpu, &mut over);
    let unit = draw(&gpu, &mut unit);

    assert!(
        over.iter().all(|t| t.iter().all(|c| c.is_finite() && *c >= -0.001)),
        "a weighted frame at alpha 2.0 has negative or non-finite light in it"
    );
    for i in covered(&unit) {
        for c in 0..4 {
            assert!(
                (over[i][c] - unit[i][c]).abs() < 0.02,
                "alpha 2.0 differs from alpha 1.0 at texel {i} channel {c}: {:?} against {:?}",
                over[i],
                unit[i]
            );
        }
    }
}

/// **A texel nothing drew on stays exactly nothing**, which the resolve has to
/// arrange rather than inherit: its accumulation target is zero there and it
/// divides by that sum. A missing guard is a NaN, and `tests/deck.rs` has a
/// whole test about what one NaN does to a mix.
#[test]
fn an_untouched_texel_resolves_to_transparent_black() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu, &sprite_l4("weighted"));
    let px = draw(&gpu, &mut set);

    let empty = at(&px, 1, 1);
    assert_eq!(empty, [0.0, 0.0, 0.0, 0.0], "the corner is not empty: {empty:?}");
    assert!(
        px.iter().all(|t| t.iter().all(|c| c.is_finite())),
        "the resolve put a NaN or an infinity in the frame"
    );
}

/// **A resize reallocates the accumulation targets**, asserted against what the
/// additive path covers rather than against the weighted path's own earlier
/// self.
///
/// The first version of this compared a Set taken through a resize round trip
/// against one resized straight to the frame, and **it passed with
/// `Oit::resize` stubbed out to do nothing** — because both Sets then
/// accumulated into the 1×1 targets `Set::build` leaves, agreed with each other
/// perfectly, and covered one texel. A comparison between two copies of the same
/// mistake is not a test.
///
/// Coverage is geometry, and the blend mode is not geometry: whichever mode ran,
/// exactly the texels the sprites' quads landed on have material on them. So the
/// additive frame is the control, and it is a control the weighted path cannot
/// accidentally agree with.
#[test]
fn the_accumulation_targets_follow_a_resize_and_cover_what_additive_covers() {
    let gpu = Gpu::headless().expect("no GPU available");

    let mut control = build(&gpu, &sprite_l4("additive"));
    let mut round_trip = try_build(&gpu, PAIR_L1, &sprite_l4("weighted")).expect("a pair");
    // Out, back to what `build` leaves, and out again. Only the last call can
    // make this draw the frame it is about to be asked for.
    round_trip.resize(&gpu.device, W, H);
    round_trip.resize(&gpu.device, 1, 1);
    round_trip.resize(&gpu.device, W, H);

    let control = covered(&draw(&gpu, &mut control));
    let weighted = covered(&draw(&gpu, &mut round_trip));
    assert!(control.len() > 100, "the control covered {} texels", control.len());
    assert_eq!(control, weighted, "the two modes did not cover the same texels");
}

/// A fullscreen L4 declaring `weighted` is refused, and the same procedure under
/// `additive` builds. See `SetError::WeightedFullscreen` for why this is a rule
/// about the Set rather than about the procedure.
#[test]
fn a_weighted_fullscreen_l4_is_refused_and_an_additive_one_builds() {
    let gpu = Gpu::headless().expect("no GPU available");
    let marcher = |blend: &str| {
        format!(
            r#"
proc marcher {{
  kind  L4
  blend {blend}

  fragment {{
    color = vec4(ray * 0.5 + vec3(0.5, 0.5, 0.5), 0.5);
  }}
}}
"#
        )
    };

    try_build(&gpu, PAIR_L1, &marcher("additive")).expect("the control must build");

    match try_build(&gpu, PAIR_L1, &marcher("weighted")) {
        Err(SetError::WeightedFullscreen { l4 }) => assert_eq!(l4, "marcher"),
        Err(other) => panic!("refused for the wrong reason: {other}"),
        Ok(_) => panic!("a fullscreen weighted Set built, and would have paid for the identity"),
    }
}
