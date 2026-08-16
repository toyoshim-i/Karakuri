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
    covered_above(px, 0.01)
}

/// The same, with the threshold named — a sweep down to an opacity of 0.002 has
/// to look below the coverage that counts as "anything at all" at 0.5.
fn covered_above(px: &[[f32; 4]], floor: f32) -> Vec<usize> {
    px.iter().enumerate().filter(|(_, t)| t[3] > floor).map(|(i, _)| i).collect()
}

/// **The blend mode does not change what a texel is covered by.** Two sprites of
/// opacity 0.5 over one another cover three quarters under both modes, because
/// `1 - prod(1 - a)` is what both accumulate — weighted in a revealage target
/// and additive in its alpha channel, by different arithmetic reaching the same
/// number.
///
/// That agreement is what lets L5 stay out of this: `composite.wgsl` reads a
/// slot's alpha as coverage and does not care which pass wrote it.
///
/// **An earlier version of this called itself a saturation test** — "weighted
/// saturates where additive sums" — which is false and was never what it
/// asserted. Additive's alpha blend is `One` / `OneMinusSrcAlpha`, so it
/// produces exactly the same probability; the two only part company for an
/// alpha above 1, which is `an_alpha_above_one_is_clamped`'s business and not
/// this test's.
#[test]
fn the_blend_mode_does_not_change_what_a_texel_is_covered_by() {
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
/// **Swept across three orders of magnitude of opacity**, because the agreement
/// is not scale-free and the first version of this test — at alpha 0.5 alone —
/// missed that. The resolve divides the accumulated colour by the accumulated
/// weight and guards that divide against zero; a guard set too high is a floor
/// under `a * w`, and since `w` is itself proportional to `a`, the alpha it
/// starts eating is the *square root* of it. Thin material is what this mode is
/// for, so a floor there is a floor on the mode.
///
/// **The colour and the coverage are asserted separately, and they have to be**,
/// because only one of them is the resolve's arithmetic. `weighted.rgb / alpha`
/// is what the divide produced and it must match additive's colour outright.
/// The coverage is a read of the `R16Float` revealage, whose spacing just below
/// 1.0 is one part in 2048 — so at an opacity of 0.005 the coverage is quantised
/// to a tenth of itself no matter what the resolve does. Folding the two
/// together would make a tolerance loose enough to hide the divide.
#[test]
fn a_lone_sprite_resolves_to_what_additive_accumulates_at_every_opacity() {
    let gpu = Gpu::headless().expect("no GPU available");
    // The spacing of `f16` immediately below 1.0. Coverage comes out of
    // `1 - revealage`, so this is the finest coverage the mode can express, and
    // the error bar on every coverage below it.
    const REVEAL_QUANTUM: f32 = 1.0 / 2048.0;

    for alpha in [0.5, 0.05, 0.005, 0.003, 0.002] {
        let mut additive = build(&gpu, &sprite_l4("additive"));
        assert_eq!(additive.set_param("alpha", alpha), 1, "the param must be declared for the write to mean anything");
        let mut weighted = build(&gpu, &sprite_l4("weighted"));
        assert_eq!(weighted.set_param("alpha", alpha), 1, "the param must be declared for the write to mean anything");
        let a = draw(&gpu, &mut additive);
        let w = draw(&gpu, &mut weighted);

        // The fixture writes a flat alpha over the whole quad, so a covered
        // texel holds exactly `alpha` or exactly `1 - (1 - alpha)^2` and nothing
        // between.
        let both = 1.0 - (1.0 - alpha) * (1.0 - alpha);
        let lone: Vec<usize> = covered_above(&a, alpha * 0.5)
            .into_iter()
            .filter(|&i| a[i][3] < (alpha + both) * 0.5)
            .collect();
        assert!(
            lone.len() > 100,
            "at alpha {alpha}, only {} texels are covered by exactly one sprite",
            lone.len()
        );

        for i in lone {
            assert!(
                (a[i][3] - w[i][3]).abs() < (alpha * 0.05).max(REVEAL_QUANTUM),
                "at alpha {alpha}, coverage at texel {i} is {} against additive's {}",
                w[i][3],
                a[i][3]
            );
            for c in 0..3 {
                // Both are premultiplied by their own coverage, so dividing it
                // back out is what leaves the colour the resolve computed. The
                // fixture's colours are 1.0 and 0.0 exactly; skip the zeroes.
                if a[i][c] < 1e-4 {
                    continue;
                }
                let (colour_a, colour_w) = (a[i][c] / a[i][3], w[i][c] / w[i][3]);
                let error = (colour_a - colour_w).abs() / colour_a;
                assert!(
                    error < 0.05,
                    "at alpha {alpha}, channel {c} of texel {i} resolved {:.0}% off: \
                     additive {:?} against weighted {:?}",
                    error * 100.0,
                    a[i],
                    w[i]
                );
            }
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
    assert_eq!(bright.set_param("alpha", 2.0), 1, "the param must be declared for the write to mean anything");
    let mut normal = build(&gpu, &sprite_l4("additive"));
    assert_eq!(normal.set_param("alpha", 1.0), 1, "the param must be declared for the write to mean anything");
    let bright = brightest(&draw(&gpu, &mut bright));
    let normal = brightest(&draw(&gpu, &mut normal));
    assert!(
        bright[0] + bright[1] + bright[2] > (normal[0] + normal[1] + normal[2]) * 1.5,
        "the `alpha` param never reached the additive shader: {bright:?} against {normal:?}"
    );

    let mut over = build(&gpu, &sprite_l4("weighted"));
    assert_eq!(over.set_param("alpha", 2.0), 1, "the param must be declared for the write to mean anything");
    let mut unit = build(&gpu, &sprite_l4("weighted"));
    assert_eq!(unit.set_param("alpha", 1.0), 1, "the param must be declared for the write to mean anything");
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

/// Two strokes that land on **the same screen line** while running through depth
/// in opposite directions.
///
/// Both lie in a plane through the eye, so each one's projection runs between
/// the same two screen points: any point at `eye + s * v` projects where `v`
/// does, whatever `s` is. `near_strand` starts three units from the eye at the
/// right-hand end and finishes twelve away at the left; `far_strand` does the
/// reverse. So they overlap along their whole length, and which of them is
/// nearer *changes sign half way across*.
///
/// That is the only arrangement in which a stroke's depth has to vary **along**
/// it for the picture to be right. The numbers are literals because they are
/// computed against a stationary camera the test installs — see the test.
const CROSSING_STRANDS_L1: &str = r#"
proc strands {
  kind     L1
  topology lines
  capacity [2, 2] = 2

  emit position, velocity

  element {
    var head = vec3(5.127, 0.0, -0.862);
    var tail = vec3(-3.494, 0.0, 3.448);
    if seed == 1u {
      head = vec3(-3.494, 0.0, -3.448);
      tail = vec3(5.127, 0.0, 0.862);
    }
    position = head;
    velocity = tail - head;
  }
}
"#;

/// A segment renderer: `clip_b` is the whole declaration. Flat colour, red for
/// the near-at-the-right strand and blue for the other.
fn strand_l4(blend: &str) -> String {
    format!(
        r#"
proc flat_strand {{
  kind  L4
  blend {blend}

  param alpha : float [0.0, 2.0] = 0.5

  consumes position, velocity

  vertex {{
    clip       = camera * vec4(position, 1.0);
    clip_b     = camera * vec4(position + velocity, 1.0);
    point_size = 12.0;
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

/// **A stroke is weighted along its length, not at one end.**
///
/// The claim `SEGMENT_EXPANSION`'s `w = mix(_clip.w, _clip_b.w, corner.x)` makes
/// and the only one in this change that had no picture behind it: a review
/// replaced that `w` with `_clip.w` — every corner taking the head's depth — and
/// the whole suite stayed green, because everything else here draws sprites,
/// where all six corners are at one depth anyway and the two are the same
/// expression.
///
/// The camera is stopped for this test, at the origin's height, because the
/// fixture's coordinates are worked out against a particular eye. `Orbit`'s
/// default speed would have the answer depend on which instant the frame is.
#[test]
fn a_weighted_stroke_is_weighted_along_its_length() {
    let gpu = Gpu::headless().expect("no GPU available");
    let stationary = karakuri_engine::Orbit { speed: 0.0, height: 0.0, ..Default::default() };

    let mut set = try_build(&gpu, CROSSING_STRANDS_L1, &strand_l4("weighted")).expect("a pair");
    set.camera = stationary;
    set.resize(&gpu.device, W, H);
    let w = draw(&gpu, &mut set);

    // The control: the same two strands added rather than resolved. Additive
    // cannot tell them apart at any point along the overlap, so a picture that
    // flips is the weighting and not the geometry.
    let mut control = try_build(&gpu, CROSSING_STRANDS_L1, &strand_l4("additive")).expect("a pair");
    control.camera = stationary;
    control.resize(&gpu.device, W, H);
    let a = draw(&gpu, &mut control);

    let lit = covered(&a);
    assert!(lit.len() > 200, "the strands cover only {} texels", lit.len());
    let x = |i: &usize| (i % W as usize) as u32;
    let (left, right) = (
        lit.iter().map(x).min().expect("a covered texel"),
        lit.iter().map(x).max().expect("a covered texel"),
    );
    assert!(right - left > 20, "the strands run only {} texels across", right - left);

    // A quarter in from each end, so neither sample is on a cap.
    let quarter = (right - left) / 4;
    let sample = |px: &[[f32; 4]], band: u32| -> [f32; 4] {
        let mut sum = [0.0; 4];
        let mut n = 0.0;
        for &i in &lit {
            if x(&i).abs_diff(band) <= 2 {
                for c in 0..4 {
                    sum[c] += px[i][c];
                }
                n += 1.0;
            }
        }
        assert!(n > 0.0, "nothing covered at x = {band}");
        [sum[0] / n, sum[1] / n, sum[2] / n, sum[3] / n]
    };

    let near_right = sample(&w, right - quarter);
    let near_left = sample(&w, left + quarter);
    let control_right = sample(&a, right - quarter);
    let control_left = sample(&a, left + quarter);

    assert!(
        (control_right[0] - control_right[2]).abs() < 0.05
            && (control_left[0] - control_left[2]).abs() < 0.05,
        "additive already favours one strand ({control_left:?} against {control_right:?}), so it \
         is not the control it is being used as"
    );
    assert!(
        near_right[0] > near_right[2] * 1.05,
        "the strand that is near at the right did not dominate there: {near_right:?}"
    );
    assert!(
        near_left[2] > near_left[0] * 1.05,
        "the strand that is near at the left did not dominate there: {near_left:?}"
    );
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
