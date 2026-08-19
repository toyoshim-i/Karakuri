//! Attributes a consumer asks for and no procedure emits.
//!
//! `age` and `velocity` are the two `docs/ir-spec.md` settled a rule for. Until
//! now the rule was design only and an unmet `consumes` was an unconditional
//! error — so a renderer that wanted a motion streak could only be paired with
//! an L1 that had thought to emit `velocity`, which is the coupling the slot
//! interface contract exists to remove.
//!
//! **Every test here is a comparison against a procedure that emits the
//! attribute by hand**, and that anchor is the point. A test that only asserted
//! the derived value was *non-zero* would pass on any number at all, and one
//! that only asserted the pair *builds* would pass on zeros — which is exactly
//! the failure the old unconditional rejection was written to prevent, so
//! reintroducing it here would be undoing the reason the rejection existed.
//!
//! Each comparison carries a second assertion beside it: that the anchor and
//! the derived Set both differ from the picture the attribute's *absence* would
//! give. Two agreeing wrong answers look exactly like two agreeing right ones.

use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

const W: u32 = 64;
const H: u32 = 64;

/// One element at the origin, drifting along `y` at a known rate.
///
/// `emit` and the body are parameters because the whole method here is to run
/// the same motion twice — once with the attribute written by hand and once
/// with nothing writing it — and see the same picture.
fn mover(name: &str, emit: &str, extra: &str) -> String {
    format!(
        r#"
proc {name} {{
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit {emit}

  element {{
    position = position + vec3(0.0, 0.25, 0.0) * dt;
{extra}  }}
}}
"#
    )
}

/// Draws one sprite, placed by whatever it is told to read.
///
/// The attribute under test decides *where* rather than how bright, because a
/// position is what this file can measure to a texel — see `deform.rs` for the
/// same reasoning about the pinned camera below.
fn reader(name: &str, consumes: &str, y: &str) -> String {
    format!(
        r#"
proc {name} {{
  kind  L4
  blend additive

  consumes {consumes}

  vertex {{
    clip       = camera * vec4(0.0, {y}, 0.0, 1.0);
    point_size = 2.0;
  }}

  fragment {{
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }}
}}
"#
    )
}

/// The same motion, but with elements **spawned over time** rather than alive
/// from frame zero.
///
/// **This is what a `birth_t` of zero survives.** With no `spawn` block every
/// element exists at `t = 0`, so "the clock" and "the clock since this element
/// was born" are the same number and a rule that forgot to record the spawn
/// instant is exactly right by accident. Only a population with a spread of
/// ages can tell them apart.
fn spawner(name: &str, emit: &str, spawn_extra: &str, element_extra: &str) -> String {
    format!(
        r#"
proc {name} {{
  kind     L1
  topology points
  capacity [64, 64] = 64

  param spawn_rate : float [0.0, 4000.0] = 192.0

  emit {emit}

  spawn {{
    position = vec3(0.0, 0.0, 0.0);
{spawn_extra}  }}

  element {{
    position = position + vec3(0.0, 0.25, 0.0) * dt;
{element_extra}  }}
}}
"#
    )
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

fn build(gpu: &Gpu, l1: &str, l4: &str) -> Set {
    build_chain(gpu, l1, &[], l4).expect("a pair whose `consumes` is satisfied")
}

fn build_chain(
    gpu: &Gpu,
    l1: &str,
    l2s: &[&str],
    l4: &str,
) -> Result<Set, karakuri_engine::set::SetError> {
    let compiled: Vec<Checked> = l2s.iter().map(|s| compile(s)).collect();
    let l2_refs: Vec<&Checked> = compiled.iter().collect();
    let l1 = compile(l1);
    let capacity = l1
        .capacity
        .expect("an L1 declares a capacity range")
        .default;
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&l1, capacity)],
        &l2_refs,
        None,
        None,
        &[&compile(l4)],
        Layering::Overdraw,
        7,
    )?;
    set.resize(&gpu.device, W, H);
    set.camera = karakuri_engine::camera::Orbit {
        radius: 5.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    };
    Ok(set)
}

/// Where the material sits on screen after `frames` frames, in texels.
fn centre_y(gpu: &Gpu, set: &mut Set, frames: u32) -> f32 {
    let mut px = Vec::new();
    for _ in 0..frames {
        px = frame(gpu, set);
    }
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

const FRAMES: u32 = 20;

/// **A derived `age` is the same number an accumulating one is.**
///
/// The anchor emits `age` and adds `dt` to it every step, which is what every
/// L1 in `examples/` does by hand; the subject emits only `position` and lets
/// the rule answer. The renderer places the sprite by `age`, so agreement is
/// agreement to a texel.
///
/// The third figure is what the picture would be at age zero, and it is here
/// because two Sets that both returned zero would agree perfectly.
#[test]
fn a_derived_age_matches_one_the_procedure_accumulates() {
    let gpu = Gpu::headless().expect("a GPU");
    let l4 = reader("by_age", "position, age", "age * 3.0 - 1.0");

    let mut anchored = build(
        &gpu,
        &mover("anchored", "position, age", "    age = age + dt;\n"),
        &l4,
    );
    let mut derived = build(&gpu, &mover("bare", "position", ""), &l4);

    let a = centre_y(&gpu, &mut anchored, FRAMES);
    let d = centre_y(&gpu, &mut derived, FRAMES);
    assert!(
        (a - d).abs() < 1.5,
        "an accumulated age put the sprite at row {a} and a derived one at row {d}"
    );

    // What age zero looks like, so that "they agree" cannot be "they are both
    // nothing". The renderer's own arithmetic decides this row, not the test.
    let mut still = build(
        &gpu,
        &mover("still", "position", ""),
        &reader("at_zero", "position", "0.0 * 3.0 - 1.0"),
    );
    let zero = centre_y(&gpu, &mut still, FRAMES);
    assert!(
        (d - zero).abs() > 3.0,
        "a derived age of {FRAMES} frames should not draw where age zero does: {d} against {zero}"
    );
}

/// **The same claim over a population with a spread of ages**, which is the
/// only shape that can see whether the spawn instant was recorded at all.
///
/// Everything alive from frame zero has one age, and it is the clock — so a
/// rule that measured from zero instead of from birth is right by accident and
/// stays right forever. Spawning over time separates the two: the mean age of a
/// population filling up is about half the elapsed time, and a rule reading the
/// clock puts every element at the oldest one's row.
#[test]
fn a_derived_age_is_measured_from_the_spawn_instant() {
    let gpu = Gpu::headless().expect("a GPU");
    let l4 = reader("by_age", "position, age", "age * 6.0 - 1.0");

    let mut anchored = build(
        &gpu,
        &spawner(
            "anchored",
            "position, age",
            "    age = 0.0;\n",
            "    age = age + dt;\n",
        ),
        &l4,
    );
    let mut derived = build(&gpu, &spawner("bare", "position", "", ""), &l4);

    let a = centre_y(&gpu, &mut anchored, FRAMES);
    let d = centre_y(&gpu, &mut derived, FRAMES);
    assert!(
        (a - d).abs() < 1.5,
        "a spawning population's accumulated ages sat at row {a} and its derived ages at {d}"
    );
}

/// **A derived `velocity` is the motion the simulation actually had.**
///
/// The anchor emits `velocity` and writes the same constant the body moves by;
/// the subject emits only `position`. The renderer places the sprite by the
/// velocity's length, so the two agree only if the difference and the division
/// are both right — a factor of `dt` either way moves the sprite off the frame.
#[test]
fn a_derived_velocity_matches_one_the_procedure_writes() {
    let gpu = Gpu::headless().expect("a GPU");
    let l4 = reader(
        "by_speed",
        "position, velocity",
        "length(velocity) * 4.0 - 1.0",
    );

    let mut anchored = build(
        &gpu,
        &mover(
            "anchored",
            "position, velocity",
            "    velocity = vec3(0.0, 0.25, 0.0);\n",
        ),
        &l4,
    );
    let mut derived = build(&gpu, &mover("bare", "position", ""), &l4);

    let a = centre_y(&gpu, &mut anchored, FRAMES);
    let d = centre_y(&gpu, &mut derived, FRAMES);
    assert!(
        (a - d).abs() < 1.5,
        "a written velocity put the sprite at row {a} and a derived one at row {d}"
    );

    let mut still = build(
        &gpu,
        &mover("still", "position", ""),
        &reader("at_zero", "position", "0.0 * 4.0 - 1.0"),
    );
    let zero = centre_y(&gpu, &mut still, FRAMES);
    assert!(
        (d - zero).abs() > 3.0,
        "a derived velocity should not draw where a speed of zero does: {d} against {zero}"
    );
}

/// **The slot is allocated because something asked**, and a Set nothing asks in
/// pays nothing.
///
/// A slot per element per rule is what an unconditional one would cost every Set
/// in the library. **The observable is the slot list and not the stride**, which
/// it used to be: every slot was a padded sixteen bytes then, so "one more slot"
/// and "sixteen more bytes" were the same sentence. They are not any more —
/// `birth_t` is a `f32` and lands in the four bytes `seed` and `birth_frac`
/// leave, so it is now free — and a stride assertion would have read that as the
/// slot not existing.
#[test]
fn a_derivations_slot_exists_only_where_something_consumes_it() {
    let gpu = Gpu::headless().expect("a GPU");
    let bare = mover("bare", "position", "");

    let plain = build(&gpu, &bare, &reader("plain", "position", "0.0"));
    let aged = build(&gpu, &bare, &reader("aged", "position, age", "age"));
    let moving = build(
        &gpu,
        &bare,
        &reader("moving", "position, velocity", "velocity.y"),
    );
    let both = build(
        &gpu,
        &bare,
        &reader("both", "position, age, velocity", "age + velocity.y"),
    );

    let has = |s: &Set, name: &str| s.element_layout().slots.iter().any(|f| f.name == name);
    for (set, label) in [
        (&plain, "plain"),
        (&aged, "aged"),
        (&moving, "moving"),
        (&both, "both"),
    ] {
        assert_eq!(
            has(set, "birth_t"),
            label == "aged" || label == "both",
            "{label}: `birth_t`"
        );
        assert_eq!(
            has(set, "velocity"),
            label == "moving" || label == "both",
            "{label}: `velocity`"
        );
    }

    // And a stored derivation still costs bytes, because `velocity` is a `vec3`
    // and there is no padding upstream of it to hide in. Its companion flag is
    // the one that is free — it lands in the four bytes the `vec3` leaves.
    let stride = |s: &Set| s.element_layout().stride;
    assert!(
        stride(&moving) > stride(&plain),
        "a stored derivation is not free"
    );
    assert_eq!(
        stride(&both),
        stride(&moving),
        "`birth_t` fits in what was already padding"
    );
}

/// **An attribute somebody emits is never derived**, wherever in the chain the
/// emitter sits.
///
/// Both at once would mean a slot and a substitution for one name, and every
/// reader would have to know which applied where. So an L1 that emits `age`
/// keeps its own — the stride says so — and the renderer reads what the
/// procedure wrote rather than what the clock would have said.
///
/// The second half is the one the first cannot reach. Where the *L1* emits it,
/// the attribute is available from the start and the rule is never even
/// considered; the guard that matters is for an emitter **below** a consumer,
/// where "nobody has emitted this yet" and "nobody emits this" come apart. That
/// stays a composition error naming the position, because the alternative is a
/// chain in which one name is a slot at one node and a substitution at another.
#[test]
fn an_emitted_attribute_is_read_rather_than_derived() {
    let gpu = Gpu::headless().expect("a GPU");
    let l4 = reader("by_age", "position, age", "age * 3.0 - 1.0");

    let bare = build(&gpu, &mover("bare", "position", ""), &l4);
    // The emitter deliberately writes something the clock never would: age
    // counting *down*. If the derivation ran anyway, the sprite would go the
    // other way.
    let wrong_way = build(
        &gpu,
        &mover("counts_down", "position, age", "    age = age - dt;\n"),
        &l4,
    );

    assert_eq!(
        bare.element_layout().stride,
        wrong_way.element_layout().stride,
        "one slot either way — `birth_t` for the derived Set, `age` for the emitting one"
    );

    let mut bare = bare;
    let mut wrong_way = wrong_way;
    let up = centre_y(&gpu, &mut bare, FRAMES);
    let down = centre_y(&gpu, &mut wrong_way, FRAMES);
    assert!(
        (up - down).abs() > 3.0,
        "the emitting procedure's own `age` should be what is drawn: {up} against {down}"
    );

    // An L2 above the emitter asks for `age`, and a second L2 below it emits
    // one. Deriving here would give the first node a substitution and the
    // second a slot, for one name, in one chain.
    let wants = r#"
proc wants_age {
  kind L2
  consumes position, age
  deform { position = position + vec3(0.0, age, 0.0); }
}
"#;
    let emits = r#"
proc emits_age {
  kind L2
  emit age
  consumes position
  deform { age = length(position); }
}
"#;
    let err = build_chain(
        &gpu,
        &mover("bare", "position", ""),
        &[wants, emits],
        &reader("plain", "position", "0.0"),
    )
    .err()
    .expect("`age` is emitted below the node that consumes it, so it is not derived there");
    assert!(
        err.to_string().contains("age"),
        "the refusal has to name the attribute: {err}"
    );
}

/// **An L1 may consume a derived attribute, and the substitution it gets is its
/// own previous frame's.**
///
/// Nothing here read one before, and that was the file's largest hole: deleting
/// the derived branch of either the L1 or the L2 resolver left the whole
/// workspace green, and what it produces is a shader naming a field the struct
/// does not have — a wgpu validation panic at build, from a `.kir` the checker
/// accepted.
///
/// The anchor is the same procedure emitting `age` and accumulating it. Both
/// place the sprite by the age they read, so agreement is agreement to a texel.
#[test]
fn an_l1_reading_a_derived_age_matches_one_that_accumulates_its_own() {
    let gpu = Gpu::headless().expect("a GPU");
    let l4 = reader("plain", "position", "position.y");

    // The L1 writes its own age into `position.y`, so the renderer needs to
    // know nothing about the rule.
    let anchored = r#"
proc anchored {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position, age

  element {
    age      = age + dt;
    position = vec3(0.0, age * 3.0 - 1.0, 0.0);
  }
}
"#;
    let derived = r#"
proc derives {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit     position
  consumes age

  element {
    position = vec3(0.0, age * 3.0 - 1.0, 0.0);
  }
}
"#;
    let mut a = build(&gpu, anchored, &l4);
    let mut d = build(&gpu, derived, &l4);
    let (ay, dy) = (
        centre_y(&gpu, &mut a, FRAMES),
        centre_y(&gpu, &mut d, FRAMES),
    );
    assert!(
        (ay - dy).abs() < 1.5,
        "an L1's own accumulated age put the sprite at row {ay} and a derived one at {dy}"
    );
}

/// The same claim for an L2, which reads through a different resolver and would
/// fail in a different file.
#[test]
fn an_l2_reading_a_derived_age_matches_one_the_l1_accumulates() {
    let gpu = Gpu::headless().expect("a GPU");
    let l4 = reader("plain", "position", "position.y");
    let lift = r#"
proc lift {
  kind L2

  consumes position, age

  deform {
    position = vec3(0.0, age * 3.0 - 1.0, 0.0);
  }
}
"#;
    let anchored = build_chain(
        &gpu,
        &mover("anchored", "position, age", "    age = age + dt;\n"),
        &[lift],
        &l4,
    )
    .expect("the L1 emits `age`");
    let derived = build_chain(&gpu, &mover("bare", "position", ""), &[lift], &l4)
        .expect("nothing emits `age`, so it is derived");

    let mut anchored = anchored;
    let mut derived = derived;
    let (ay, dy) = (
        centre_y(&gpu, &mut anchored, FRAMES),
        centre_y(&gpu, &mut derived, FRAMES),
    );
    assert!(
        (ay - dy).abs() < 1.5,
        "an L2 over an emitted age drew at row {ay} and over a derived one at {dy}"
    );
}

/// **An element's first update has no velocity**, and the alternative measured
/// twenty times the true speed.
///
/// A newly spawned element's first pass runs over a *fraction* of a step, and
/// the step it is divided by is scaled to that fraction — right for a body that
/// integrates, wrong for one that computes position from `t` and jumps a whole
/// step regardless. An element of a procedure with no `spawn` block has the
/// same problem from the other end: it starts at the origin because that is
/// what an unwritten buffer holds, and its first pass is a difference against a
/// state it was never in.
///
/// Read off the buffer rather than out of the picture, because the artifact is
/// one frame of one batch and a mean position cannot see it.
#[test]
fn a_first_update_has_no_derived_velocity() {
    let gpu = Gpu::headless().expect("a GPU");
    const SPEED: f32 = 0.25;

    // Closed form, so nothing about the body scales with the step: every
    // element jumps to where `t` says it should be, whatever fraction of a step
    // it has lived.
    let l1 = format!(
        r#"
proc jumps {{
  kind     L1
  topology points
  capacity [64, 64] = 64

  param spawn_rate : float [0.0, 4000.0] = 600.0

  emit position

  spawn {{
    position = vec3(0.0, 0.0, 0.0);
  }}

  element {{
    position = vec3(0.0, {SPEED:?} * t, 0.0);
  }}
}}
"#
    );
    let mut set = build(
        &gpu,
        &l1,
        &reader("by_speed", "position, velocity", "length(velocity)"),
    );
    for _ in 0..3 {
        frame(&gpu, &mut set);
    }

    let layout = set.element_layout().clone();
    let offset = layout.offset_of("velocity") as usize;
    let stride = layout.stride as usize;
    let bytes = set.read_elements(&gpu.device, &gpu.queue);
    let speeds: Vec<f32> = bytes
        .chunks_exact(stride)
        .map(|e| {
            let y = f32::from_le_bytes(e[offset + 4..offset + 8].try_into().unwrap());
            y.abs()
        })
        .collect();

    // Every element is either still waiting for its first whole step, or moving
    // at the one speed this procedure has. Nothing in between, and nothing
    // above — an inflated first update reads as a multiple of the true speed,
    // and the multiple is up to twice the batch size.
    for (i, &v) in speeds.iter().enumerate() {
        assert!(
            v < 1e-4 || (v - SPEED).abs() < SPEED * 0.05,
            "element {i} has a derived speed of {v}, and this procedure only ever moves at {SPEED}"
        );
    }
    assert!(
        speeds.iter().any(|&v| (v - SPEED).abs() < SPEED * 0.05),
        "no element reached the true speed at all, so the assertion above was vacuous: {speeds:?}"
    );
}

/// **A rule that could not run says why**, rather than repeating advice that
/// contradicts it.
///
/// `velocity` is synthesised from `position`, so an L1 emitting neither cannot
/// have it. The refusal for that used to drop the reason and fall through to
/// the generic hint — which asserts, on a refusal *of* `velocity`, that
/// `velocity` is synthesised where nothing emits it. A regenerating model
/// reading that is being told the specification is wrong, and the one thing
/// that would fix the file is the one thing the message does not name.
#[test]
fn a_blocked_rule_names_the_attribute_it_needed() {
    let gpu = Gpu::headless().expect("a GPU");
    let no_position = r#"
proc no_position {
  kind     L1
  topology points
  capacity [8, 8] = 8

  emit tint

  element {
    tint = vec3(1.0, 1.0, 1.0);
  }
}
"#;
    let err = build_chain(
        &gpu,
        no_position,
        &[],
        &reader("wants_speed", "velocity", "velocity.y"),
    )
    .err()
    .expect("`velocity` derives from `position`, which this L1 does not emit");

    let text = err.to_string();
    assert!(
        text.contains("position"),
        "the refusal has to name what the rule wanted: {text}"
    );
    assert!(
        !text.contains("nothing else is"),
        "and must not fall through to the hint that says `velocity` is synthesised: {text}"
    );
}
