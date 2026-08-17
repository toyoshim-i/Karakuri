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
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
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
    let capacity = l1.capacity.expect("an L1 declares a capacity range").default;
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &l1,
        &l2_refs,
        None,
        &[&compile(l4)],
        Layering::Overdraw,
        capacity,
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
    assert!(weight > 0.0, "nothing was drawn, so there is no position to measure");
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
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
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
    let l4 = reader("by_speed", "position, velocity", "length(velocity) * 4.0 - 1.0");

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
/// Sixteen bytes per element per rule is what an unconditional slot would cost
/// every Set in the library, and the two rules together are more than the whole
/// of `position`. The stride is the observable.
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

    let stride = |s: &Set| s.element_layout().stride;
    assert_eq!(stride(&aged), stride(&plain) + 16, "`birth_t`");
    assert_eq!(stride(&moving), stride(&plain) + 16, "`velocity`");
    assert_eq!(stride(&both), stride(&plain) + 32, "one slot each, not one between them");
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
