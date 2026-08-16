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
//! The material is a lattice with no motion of its own, so anything that moves
//! moved because a `deform` moved it.

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
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
}

fn build(gpu: &Gpu, l2s: &[&str]) -> Set {
    let l2: Vec<Checked> = l2s.iter().map(|s| compile(s)).collect();
    let l2_refs: Vec<&Checked> = l2.iter().collect();
    let l4 = compile(DOTS);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &compile(STILL),
        &l2_refs,
        &[&l4],
        CAPACITY,
        7,
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
    assert!(weight > 0.0, "nothing was drawn, so there is no position to measure");
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
    assert_eq!(declared.len(), 2, "the two deformations did not get a map each");

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

    Set::build_many(&gpu.device, &gpu.queue, &l1, &[&l2], &[&l4], CAPACITY, 7)
        .expect("the renderer consumes what the deformation emits");

    // The same renderer without the deformation has nowhere to read `tint`
    // from, and the error has to name it.
    let err = Set::build_many(&gpu.device, &gpu.queue, &l1, &[], &[&l4], CAPACITY, 7)
        .err()
        .expect("`tint` is not available without the deformation that emits it");
    let message = err.to_string();
    assert!(message.contains("tint"), "the diagnostic does not name what was missing: {message}");
}
