//! L2 nodes: a deformation between the simulation and the renderers.
//!
//! Three claims, and the middle one is the layer's whole design:
//!
//! - **A deformation reaches what is drawn.** It runs on the GPU between the
//!   simulation and the draw, so the only way to see it is in the elements a
//!   renderer read.
//! - **It is stateless.** Not by a rule the checker enforces but by how it is
//!   lowered — reads and writes address the output buffer, and the output is
//!   rebuilt from the input every frame, so there is nothing to accumulate onto.
//!   A modulator that drifted would be one that cannot be stacked, cannot be
//!   fused, and drags `closed_form` out of the L1's hands.
//! - **They chain**, and the second one sees the first one's work.
//!
//! A fourth arrived with the layer's masks: **a deformation can be partial**,
//! in two independent ways. `weight` is a declared `param` and scales the whole
//! modulation; a `mask` block computes a `strength` per element and decides
//! *where*. Both end at one `mix` between what reached the node and what the
//! body wrote.
//!
//! The material is a lattice with no motion of its own, so anything that moves
//! moved because a `deform` moved it.

use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

const W: u32 = 64;
const H: u32 = 64;
const CAPACITY: u32 = 1;

/// **One element, at the origin, still.** Every measurement below is a
/// displacement, so one element is the whole of what they need — and it is what
/// keeps them linear: a spread of elements walks off the top of a 64-texel frame
/// after a shift or two, and the mean of what is *left* then moves further than
/// the material did. That is what an earlier fixture measured, and it reported a
/// chain of two composing as three.
///
/// Displacement is along `y` rather than `x` for a related reason. The camera is
/// pinned so that a shift is measurable at all, and an `Orbit` at angle zero
/// sits on the `+x` axis — material laid along `x` is laid along the view
/// direction and projects to one point, which reads exactly like a deformation
/// that never ran.
const STILL: &str = r#"
proc still {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

/// Moves every element by a constant. The simplest thing that can be seen, and
/// the simplest thing that would drift if the layer were stateful: written as
/// `position + shift`, it lands one `shift` from where the input put it, every
/// frame, forever.
fn shift(name: &str, dy: f32) -> String {
    format!(
        r#"
proc {name} {{
  kind L2

  param amount : float [0.0, 8.0] = 1.0

  consumes position

  deform {{
    position = position + vec3(0.0, {dy:?}, 0.0) * amount;
  }}
}}
"#
    )
}

/// **Two elements, told apart by `seed` and separated on screen.**
///
/// A mask needs material it can treat differently, and a measurement needs to
/// see both halves: these sit either side of centre along `z`, which is the
/// screen's horizontal under the pinned camera below — `x` is the view
/// direction and would put one behind the other.
const PAIR: &str = r#"
proc pair {
  kind     L1
  topology points
  capacity [2, 2] = 2

  emit position

  element {
    position = vec3(0.0, 0.0, float(seed) * 2.0 - 1.0);
  }
}
"#;

/// Draws whatever reaches it, as one bright sprite per element.
const DOTS: &str = r#"
proc dots {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 5.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
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

fn build(gpu: &Gpu, l2s: &[&str]) -> Set {
    build_over(gpu, STILL, l2s, CAPACITY)
}

fn build_over(gpu: &Gpu, l1: &str, l2s: &[&str], capacity: u32) -> Set {
    let l2: Vec<Checked> = l2s.iter().map(|s| compile(s)).collect();
    let l2_refs: Vec<&Checked> = l2.iter().collect();
    let l4 = compile(DOTS);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&compile(l1), capacity)],
        &l2_refs,
        None,
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring::default(),
    )
    .expect("a chain of one L1, some L2s and one L4");
    set.resize(&gpu.device, W, H);
    // **The camera is pinned, which the measurements need rather than prefer.**
    // The default orbits at 0.15 rev/s from a height of two, so successive
    // frames look from different angles and the x axis these fixtures lay their
    // material along is foreshortened by an amount that changes every frame.
    // Head-on and still, a displacement in x is a displacement in texels.
    set.camera = karakuri_engine::camera::Orbit {
        radius: 5.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    };
    set
}

/// One frame, returning the mean row of every lit texel weighted by brightness —
/// where the material sits on screen, in texels.
///
/// The elements themselves cannot be read back: `Set::read_elements` returns the
/// **simulation's** buffer, which a deformation never touches. That is correct
/// and is exactly why this measures the picture instead.
fn centre_y(gpu: &Gpu, set: &mut Set) -> f32 {
    let px = frame(gpu, set);
    let (mut sum, mut weight) = (0.0f64, 0.0f64);
    for (i, t) in px.chunks_exact(4).enumerate() {
        if t[0] > 0.01 {
            sum += f64::from(t[0]) * f64::from(i as u32 / W);
            weight += f64::from(t[0]);
        }
    }
    assert!(
        weight > 0.0,
        "nothing was drawn, so there is no position to measure"
    );
    (sum / weight) as f32
}

/// RGBA f32 per texel, after one frame.
fn frame(gpu: &Gpu, set: &mut Set) -> Vec<f32> {
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
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
    let out: Vec<f32> = data
        .chunks_exact(2)
        .map(|b| f16(u16::from_le_bytes([b[0], b[1]])))
        .collect();
    drop(data);
    readback.unmap();
    out
}

fn f16(bits: u16) -> f32 {
    let sign = f32::from_bits(u32::from(bits & 0x8000) << 16);
    let exp = (bits >> 10) & 0x1f;
    let mant = u32::from(bits & 0x3ff);
    let v = match exp {
        0 => f32::from_bits(mant << 13) * 2.0f32.powi(-112),
        0x1f => f32::from_bits(0x7f80_0000 | (mant << 13)),
        _ => f32::from_bits(((u32::from(exp) + 112) << 23) | (mant << 13)),
    };
    f32::from_bits(v.to_bits() | sign.to_bits())
}

// ---------------------------------------------------------------------------

/// **A deformation reaches what is drawn.** It runs between the simulation and
/// the draw and touches no buffer anything else reads, so the picture is the
/// only place its work exists.
#[test]
fn a_deformation_moves_what_the_renderer_draws() {
    let gpu = Gpu::headless().expect("no GPU available");
    let plain = centre_y(&gpu, &mut build(&gpu, &[]));
    let moved = centre_y(&gpu, &mut build(&gpu, &[&shift("push", 0.5)]));

    // **Upward, so the row number goes down**: `+y` in the world is toward the
    // top of the frame. Asserted as a distance rather than a direction, since
    // which way a screen axis runs is a fact about the projection and not about
    // the layer under test.
    assert!(
        (plain - moved) > 2.0,
        "the deformation did not reach the frame: {moved} against {plain}"
    );
}

/// **And it is stateless**, which is the decision the whole layer rests on.
///
/// `position = position + shift` reads the *input's* position, not this node's
/// own output from last frame, so it lands one `shift` from where the simulation
/// put the element — on frame one and on frame twenty alike. A stateful L2 would
/// walk the material off screen, and would also be one that cannot be stacked
/// freely, cannot be fused by a graph compiler, and drags `closed_form` out of
/// the L1's hands.
///
/// The lowering is what makes this true rather than a check refusing what breaks
/// it: reads and writes both address the output buffer, and the output buffer is
/// rebuilt from the input before the block runs. There is nothing to accumulate
/// onto.
#[test]
fn a_deformation_does_not_accumulate_over_frames() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu, &[&shift("push", 0.5)]);

    let first = centre_y(&gpu, &mut set);
    let mut last = first;
    for _ in 0..19 {
        last = centre_y(&gpu, &mut set);
    }
    assert!(
        (last - first).abs() <= 0.5,
        "twenty frames moved the material from {first} to {last} — the deformation \
         accumulated onto its own output"
    );
}

/// **They chain**, and the second sees the first one's work rather than the
/// simulation's.
#[test]
fn two_deformations_compose_in_chain_order() {
    let gpu = Gpu::headless().expect("no GPU available");
    let one = shift("first", 0.5);
    let two = shift("second", 0.5);

    let plain = centre_y(&gpu, &mut build(&gpu, &[]));
    let once = centre_y(&gpu, &mut build(&gpu, &[&one]));
    let twice = centre_y(&gpu, &mut build(&gpu, &[&one, &two]));

    let step = plain - once;
    assert!(step > 1.0, "one deformation did nothing measurable");
    assert!(
        ((plain - twice) - 2.0 * step).abs() <= 0.5 * step,
        "two deformations moved the material by {} where twice {step} was due — \
         the second read the simulation rather than the first",
        plain - twice
    );
}

/// **A deformation's params are its own**, addressed like any other node's. The
/// two nodes here declare one `amount` and hold two values.
#[test]
fn each_deformation_holds_its_own_parameters() {
    let gpu = Gpu::headless().expect("no GPU available");
    let one = shift("first", 0.5);
    let two = shift("second", 0.5);
    let mut set = build(&gpu, &[&one, &two]);

    let declared: Vec<(u32, f32)> = set
        .params()
        .filter(|(layer, _, name, _)| *layer == karakuri_ir::Kind::L2 && *name == "amount")
        .map(|(_, index, _, value)| (index, value))
        .collect();
    assert_eq!(
        declared.len(),
        2,
        "the two deformations did not get a map each"
    );

    let plain = centre_y(&gpu, &mut build(&gpu, &[&one, &two]));
    assert!(
        set.set_param_at(karakuri_ir::Kind::L2, 1, "amount", 0.0),
        "the second deformation declares `amount`"
    );
    let muted = centre_y(&gpu, &mut set);
    let single = centre_y(&gpu, &mut build(&gpu, &[&one]));
    assert!(
        (muted - single).abs() < (plain - single).abs() / 2.0,
        "silencing the second deformation left the picture at {muted}, where one \
         deformation alone gives {single} and both give {plain}"
    );
}

/// **An L2 may widen the element**, and a renderer downstream can consume what
/// it added — which the same renderer over the bare L1 cannot. The composition
/// check therefore has to walk the chain rather than compare against the L1.
#[test]
fn a_renderer_may_consume_what_a_deformation_added() {
    let gpu = Gpu::headless().expect("no GPU available");
    let tinter = r#"
proc tinter {
  kind L2
  consumes position
  emit tint
  deform {
    tint = vec3(1.0, 0.2, 0.2);
  }
}
"#;
    let tinted = r#"
proc tinted_dots {
  kind  L4
  blend additive
  consumes position, tint
  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 5.0;
  }
  fragment {
    color = vec4(tint, 1.0);
  }
}
"#;
    let l2 = compile(tinter);
    let l4 = compile(tinted);
    let l1 = compile(STILL);

    Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&l1, CAPACITY)],
        &[&l2],
        None,
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring::default(),
    )
    .expect("the renderer consumes what the deformation emits");

    // The same renderer without the deformation has nowhere to read `tint`
    // from, and the error has to name it.
    let err = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&l1, CAPACITY)],
        &[],
        None,
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring::default(),
    )
    .err()
    .expect("`tint` is not available without the deformation that emits it");
    let message = err.to_string();
    assert!(
        message.contains("tint"),
        "the diagnostic does not name what was missing: {message}"
    );
}

// ---------------------------------------------------------------------------
// Partial deformation: `weight` and `mask`
// ---------------------------------------------------------------------------

/// **`weight` scales the whole modulation**, and it is an ordinary declared
/// `param` — which is the point of it being one rather than a number inside the
/// mask. It goes on a fader, takes a binding, moves under a transition, and is
/// saved in a Set file, none of which an expression could.
///
/// Asserted as arithmetic and not as an inequality: half the weight is half the
/// displacement, exactly, because the lowering blends between what reached the
/// node and what the body wrote. An inequality would pass for any monotone
/// wrong answer, and the whole point of a fader is that its middle is the
/// middle.
#[test]
fn a_weight_scales_how_much_of_the_deformation_lands() {
    let gpu = Gpu::headless().expect("no GPU available");
    let weighted = r#"
proc weighted {
  kind L2
  param weight : float [0.0, 1.0] = 1.0
  consumes position
  deform {
    position = position + vec3(0.0, 1.0, 0.0);
  }
}
"#;
    let plain = centre_y(&gpu, &mut build(&gpu, &[]));
    let full = centre_y(&gpu, &mut build(&gpu, &[weighted]));
    let step = plain - full;
    assert!(step > 2.0, "the deformation did not reach the frame");

    let mut half = build(&gpu, &[weighted]);
    assert!(
        half.set_param_at(karakuri_ir::Kind::L2, 0, "weight", 0.5),
        "the modulator declares `weight`"
    );
    let moved = plain - centre_y(&gpu, &mut half);
    assert!(
        (moved - step * 0.5).abs() < step * 0.1,
        "half the weight moved the material {moved} texels where half of {step} was due"
    );

    // And zero is the identity, which is what makes a fader able to take a
    // modulator out of the chain without rebuilding it.
    let mut off = build(&gpu, &[weighted]);
    assert!(off.set_param_at(karakuri_ir::Kind::L2, 0, "weight", 0.0));
    assert!(
        (centre_y(&gpu, &mut off) - plain).abs() < 0.25,
        "a weight of zero still moved the material"
    );
}

/// **A mask decides where.** The two elements differ only in `seed`, so a mask
/// on `seed` deforms one and leaves the other exactly where the simulation put
/// it — which shows as the centroid moving half as far as it does when both go.
///
/// **The anchor is the half**, not the direction. A mask that let both through
/// would move it the full distance and a mask that let neither through would
/// move it none, so an inequality against zero would pass for the first of
/// those — which is the mask doing nothing at all.
#[test]
fn a_mask_applies_the_deformation_to_some_elements_and_not_others() {
    let gpu = Gpu::headless().expect("no GPU available");
    let both = r#"
proc lift {
  kind L2
  consumes position
  deform {
    position = position + vec3(0.0, 1.0, 0.0);
  }
}
"#;
    // `seed` is the spawn ordinal, so element 0 gets 0 and element 1 gets 1.
    // `step` turns that into the mask's `[0, 1]` scalar.
    let one = r#"
proc lift_one {
  kind L2
  consumes position
  mask {
    strength = step(0.5, float(seed));
  }
  deform {
    position = position + vec3(0.0, 1.0, 0.0);
  }
}
"#;
    let plain = centre_y(&gpu, &mut build_over(&gpu, PAIR, &[], 2));
    let all = centre_y(&gpu, &mut build_over(&gpu, PAIR, &[both], 2));
    let masked = centre_y(&gpu, &mut build_over(&gpu, PAIR, &[one], 2));

    let step = plain - all;
    assert!(
        step > 2.0,
        "the unmasked deformation did not reach the frame"
    );
    assert!(
        ((plain - masked) - step * 0.5).abs() < step * 0.15,
        "the mask moved the centroid {} texels where half of {step} was due — it let \
         through both elements or neither",
        plain - masked
    );
}

/// **A mask reads what reached the node, not what the body wrote.** Running it
/// after the deformation would decide where to apply a deformation from a
/// position that deformation had already moved — which for a mask written
/// against `position` is a different set of elements every frame, and for a
/// stationary one is the wrong set once.
///
/// The fixture makes the two answers differ by a whole element: the mask admits
/// what is on the far side of the origin along `z`, and the deformation moves
/// everything across it. Evaluated on the input, exactly one element passes;
/// evaluated on the output, the other one does.
#[test]
fn a_mask_reads_the_input_rather_than_the_deformed_element() {
    let gpu = Gpu::headless().expect("no GPU available");
    let crossing = r#"
proc crossing {
  kind L2
  consumes position
  mask {
    strength = step(0.0, position.z);
  }
  deform {
    position = vec3(position.x, position.y + 1.0, 0.0 - position.z);
  }
}
"#;
    let px = frame(&gpu, &mut build_over(&gpu, PAIR, &[crossing], 2));
    // The element that was at `z = +1` is the one the mask admits, and it is the
    // one that moves — up, and across to `z = -1`. Screen-right is world `-z`
    // under this camera, so the lifted element ends up on the right.
    let mut lifted_on = None;
    for (i, t) in px.chunks_exact(4).enumerate() {
        if t[0] > 0.01 {
            let (x, y) = (i as u32 % W, i as u32 / W);
            // The lifted one is above the middle row; record which half it is in.
            if y < H / 2 - 2 {
                lifted_on = Some(x > W / 2);
            }
        }
    }
    assert_eq!(
        lifted_on,
        Some(true),
        "the element that moved is not the one the mask admitted on the input"
    );
}

/// **The gate reaches every attribute the body wrote, and `weight` multiplies
/// the mask rather than replacing it.**
///
/// Two holes a review found with surviving mutations, and they are the same
/// hole seen twice: every other test here measures `position`, and every one of
/// them uses a weight or a mask but never both. Restricting the blend to
/// `position` passed the whole workspace, and so did `strength = weight`, which
/// destroys the formula the layer is built on.
///
/// `tint` is the probe because it is not `position`: the material does not move,
/// so what is measured is the colour of a sprite that stayed where it was.
#[test]
fn the_gate_reaches_every_attribute_and_weight_multiplies_the_mask() {
    let gpu = Gpu::headless().expect("no GPU available");
    let tinted = r#"
proc tinted_dots {
  kind  L4
  blend additive
  consumes position, tint
  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 5.0;
  }
  fragment {
    color = vec4(tint, 1.0);
  }
}
"#;
    // Emits `tint` and then paints it, so the gate has an attribute other than
    // `position` to blend — and the pass-through zeroes a slot the input did
    // not have, which is what the blend runs from.
    let paint = |weight: &str, mask: &str| {
        format!(
            r#"
proc paint {{
  kind L2
  {weight}
  consumes position
  emit tint
  mask {{ strength = {mask}; }}
  deform {{ tint = vec3(1.0, 1.0, 1.0); }}
}}
"#
        )
    };
    let brightness = |src: &str| {
        let l2 = compile(src);
        let l4 = compile(tinted);
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&compile(STILL), CAPACITY)],
            &[&l2],
            None,
            &[],
            &[&l4],
            Layering::Overdraw,
            7,
            &[],
            karakuri_engine::set::Wiring::default(),
        )
        .expect("builds");
        set.resize(&gpu.device, W, H);
        set.camera = karakuri_engine::camera::Orbit {
            radius: 5.0,
            speed: 0.0,
            height: 0.0,
            ..Default::default()
        };
        frame(&gpu, &mut set)
            .chunks_exact(4)
            .map(|t| t[0])
            .fold(0.0f32, f32::max)
    };

    let full = brightness(&paint("", "1.0"));
    assert!(
        full > 0.5,
        "the deformation's own attribute never reached the frame: {full}"
    );

    // The mask alone. Half the strength is half the way from the zero the
    // pass-through left to the white the body wrote.
    let masked = brightness(&paint("", "0.5"));
    assert!(
        (masked - full * 0.5).abs() < full * 0.1,
        "a strength of 0.5 painted {masked} where half of {full} was due — the gate does not \
         reach an attribute other than `position`"
    );

    // **Both, and they multiply.** `weight` replacing the mask would give the
    // full 0.5 here rather than a quarter, which is the mutation that survived.
    let both = brightness(&paint("param weight : float [0.0, 1.0] = 0.5", "0.5"));
    assert!(
        (both - full * 0.25).abs() < full * 0.1,
        "weight 0.5 and strength 0.5 painted {both} where a quarter of {full} was due — \
         `weight` replaced the mask instead of multiplying it"
    );
}

/// **A slot the input did not have is zeroed, not left as it was** — which is
/// the statelessness claim applied to an attribute the L2 *added*, and the one
/// half of it nothing was watching.
///
/// An L2 owns one element buffer and rewrites it every frame, so whatever a
/// newly emitted slot holds before the body runs is **this node's own output
/// from the previous frame**. That is exactly the accumulation the layer is not
/// allowed to have, and it hides from every test that writes the attribute in
/// full: a stale value overwritten completely is a stale value nobody can see.
///
/// A *partial* write is what makes it visible. At half strength the gate lands
/// the tint half way from what the slot held to what the body wrote — from zero
/// that is 0.5 every frame, and from last frame's 0.5 it is 0.75, then 0.875,
/// walking to white. Twenty frames is plenty.
#[test]
fn an_attribute_a_deformation_adds_starts_from_zero_every_frame() {
    let gpu = Gpu::headless().expect("no GPU available");
    let tinted = r#"
proc tinted_dots {
  kind  L4
  blend additive
  consumes position, tint
  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 5.0;
  }
  fragment {
    color = vec4(tint, 1.0);
  }
}
"#;
    let half = r#"
proc half_paint {
  kind L2
  consumes position
  emit tint
  mask { strength = 0.5; }
  deform { tint = vec3(1.0, 1.0, 1.0); }
}
"#;
    let l2 = compile(half);
    let l4 = compile(tinted);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&compile(STILL), CAPACITY)],
        &[&l2],
        None,
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring::default(),
    )
    .expect("builds");
    set.resize(&gpu.device, W, H);
    set.camera = karakuri_engine::camera::Orbit {
        radius: 5.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    };

    let brightest = |set: &mut Set| {
        frame(&gpu, set)
            .chunks_exact(4)
            .map(|t| t[0])
            .fold(0.0f32, f32::max)
    };
    let first = brightest(&mut set);
    assert!(
        first > 0.1,
        "the added attribute never reached the frame: {first}"
    );
    let mut last = first;
    for _ in 0..19 {
        last = brightest(&mut set);
    }
    assert!(
        (last - first).abs() < first * 0.05,
        "twenty frames took the tint from {first} to {last} — the emitted slot kept this \
         node's own output from the frame before"
    );
}

/// **The gate is clamped, because `mix` extrapolates.**
///
/// Nothing stops a `mask` block writing a `strength` of 2, and nothing should:
/// it is an expression over attributes and a generated one will land outside
/// `[0, 1]` sooner or later. What must not happen is the arithmetic taking it
/// literally — `mix(a, b, 2)` applies the deformation *twice over*, and a
/// negative one applies its inverse. Both are a wrong picture rather than a
/// missing one, which is the kind this suite exists to catch.
#[test]
fn a_strength_outside_the_unit_range_is_clamped_rather_than_extrapolated() {
    let gpu = Gpu::headless().expect("no GPU available");
    let lift = |strength: &str| {
        format!(
            r#"
proc lift {{
  kind L2
  consumes position
  mask {{ strength = {strength}; }}
  deform {{ position = position + vec3(0.0, 1.0, 0.0); }}
}}
"#
        )
    };
    let plain = centre_y(&gpu, &mut build(&gpu, &[]));
    let full = centre_y(&gpu, &mut build(&gpu, &[&lift("1.0")]));
    let over = centre_y(&gpu, &mut build(&gpu, &[&lift("2.5")]));
    let under = centre_y(&gpu, &mut build(&gpu, &[&lift("0.0 - 1.5")]));

    assert!(
        plain - full > 2.0,
        "the deformation did not reach the frame"
    );
    assert!(
        (over - full).abs() < 0.25,
        "a strength of 2.5 put the material at {over} where 1.0 gives {full} — the \
         deformation was applied more than once"
    );
    assert!(
        (under - plain).abs() < 0.25,
        "a strength of -1.5 put the material at {under} where 0.0 gives {plain} — the \
         deformation was applied backwards"
    );
}
