//! The camera edge, from the producer to the picture.
//!
//! The camera is no longer six numbers the host packs into each renderer's
//! uniform: it is a node with a state buffer, a derivation pass, and a bind
//! group every L4 reads — see `karakuri_engine::node::Camera`. Everything
//! between the `Orbit` a caller assigns and the texels that come out is GPU
//! work, so the picture is the only place to check that it arrived.
//!
//! Three claims, and the third is the one that survived a defect injection
//! before this file existed:
//!
//! - **The camera reaches the frame**, so moving it moves the material.
//! - **It is re-derived every frame**, so a camera that turns keeps turning.
//! - **The aspect ratio reaches the projection.** It belongs to the canvas
//!   rather than to the camera, which is exactly why it is the piece that can
//!   go missing without any of the above noticing: it enters at the derivation,
//!   from a different buffer, written by a different call.
//!
//! `node::camera`'s own unit tests hold the derivation against the host's copy
//! of the same arithmetic. These hold the *plumbing* against the picture, which
//! is a different question: a perfect derivation nothing binds draws nothing.

use karakuri_engine::camera::Orbit;
use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

/// **One element, off the view axis and away from what the camera looks at.**
///
/// The `Orbit` below is pinned at angle zero, which puts the eye on `+x` looking
/// back at the origin — so screen-right is world `-z` and screen-up is world
/// `+y`. `(0, 1, -1.5)` therefore lands up and to the right of centre, and every
/// measurement here is that offset.
///
/// **Away from the origin is the part that took two tries.** An orbit turns
/// *about* the point it looks at, so material near that point stays near the
/// centre of the frame however far the camera swings — a first fixture at
/// `(0, 0, -1)` moved four texels over a fifth of a revolution, which reads
/// exactly like a camera that never reached the draw.
const MARK: &str = r#"
proc mark {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = vec3(0.0, 1.0, -1.5);
  }
}
"#;

const DOT: &str = r#"
proc dot {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 3.0;
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

/// A Set of one element and one renderer, at `w` by `h`, seen from `camera`.
fn build(gpu: &Gpu, w: u32, h: u32, camera: Orbit) -> Set {
    let l4 = compile(DOT);
    let mut set =
        Set::build_many(&gpu.device, &gpu.queue, &compile(MARK), &[], None, &[&l4], Layering::Overdraw, 1, 7)
            .expect("one L1 and one L4");
    set.resize(&gpu.device, w, h);
    set.camera = camera;
    set
}

/// The camera these tests measure against: **still**, so a frame is a frame and
/// not a moment in a sweep.
fn pinned() -> Orbit {
    Orbit { radius: 5.0, speed: 0.0, height: 0.0, ..Default::default() }
}

/// Brightness-weighted mean column and row of the lit texels, in texels.
fn centroid(gpu: &Gpu, set: &mut Set, w: u32, h: u32) -> (f32, f32) {
    let px = frame(gpu, set, w, h);
    let (mut sx, mut sy, mut weight) = (0.0f64, 0.0f64, 0.0f64);
    for (i, t) in px.chunks_exact(4).enumerate() {
        if t[0] > 0.01 {
            let (x, y) = ((i as u32 % w) as f64, (i as u32 / w) as f64);
            sx += f64::from(t[0]) * x;
            sy += f64::from(t[0]) * y;
            weight += f64::from(t[0]);
        }
    }
    assert!(weight > 0.0, "nothing was drawn, so there is nowhere to measure");
    ((sx / weight) as f32, (sy / weight) as f32)
}

/// RGBA f32 per texel, after one frame.
fn frame(gpu: &Gpu, set: &mut Set, w: u32, h: u32) -> Vec<f32> {
    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, w, h);
    set.prepare(&gpu.queue, 1, &Signals::default());

    let bytes_per_row = w * 8;
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(bytes_per_row * h),
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
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
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

/// **The camera reaches the frame.** Nothing about it is on the host any more —
/// the state goes into a buffer, a pass derives the matrix, and a bind group
/// carries it to the vertex stage — so a picture that moves when the camera does
/// is the only proof that all three happened.
///
/// Dollying in rather than pitching up, because a pitch rotates about the point
/// the camera looks at and this material is close to it: raising the eye by 1.5
/// moves the material half a texel, which is a fact about the geometry and not
/// about the camera. Distance scales the whole offset and cannot be cancelled by
/// where the material happens to sit.
#[test]
fn moving_the_camera_moves_the_material() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;
    let offset = |radius: f32| {
        let mut set = build(&gpu, W, H, Orbit { radius, ..pinned() });
        centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0
    };

    let far = offset(5.0);
    let near = offset(3.0);
    assert!(far.abs() > 4.0, "the material is on the centre column; nothing to measure");
    // Two thirds the distance, so five thirds the offset — asserted as a
    // direction and a lower bound rather than a ratio, since what is under test
    // is that the camera arrives at all.
    assert!(
        near.abs() > far.abs() * 1.3,
        "closing from 5 to 3 moved the material from {far} texels off centre to \
         {near} — the camera did not reach the draw"
    );
}

/// **And it is derived every frame**, not once at build.
///
/// An orbit that turns is the case a cached derivation gets wrong, and it gets
/// it wrong silently: the first frame is correct, so a still fixture and a
/// single-frame test both pass. This one lets the same Set run on.
#[test]
fn a_turning_camera_keeps_turning() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;
    // Thirty frames at sixty steps a second is half a second; at a quarter
    // revolution a second that is an eighth of a turn, which swings the material
    // a good way across the frame without carrying it off the edge.
    let mut set = build(&gpu, W, H, Orbit { speed: 0.25, ..pinned() });

    let first = centroid(&gpu, &mut set, W, H).0;
    let mut last = first;
    for _ in 0..30 {
        last = centroid(&gpu, &mut set, W, H).0;
    }
    assert!(
        (last - first).abs() > 6.0,
        "thirty frames of a turning camera left the material at column {last}, \
         where it started at {first} — the derivation ran once and was reused"
    );
}

/// **The aspect ratio reaches the projection**, and it is the piece most easily
/// lost: it belongs to the canvas rather than to the camera, so it arrives at
/// the derivation from its own buffer, written by its own call, and every test
/// above passes with it stuck at 1.
///
/// Same width, twice the height. The field of view is vertical, so a taller
/// frame at a fixed width is a *narrower* one horizontally — the same world
/// spreads over twice as many texels across, and the material's distance from
/// the centre column doubles.
#[test]
fn the_canvas_shape_reaches_the_projection() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 128;

    let wide = centroid(&gpu, &mut build(&gpu, W, 64, pinned()), W, 64).0 - W as f32 / 2.0;
    let square = centroid(&gpu, &mut build(&gpu, W, 128, pinned()), W, 128).0 - W as f32 / 2.0;

    assert!(wide.abs() > 4.0, "the material is on the centre column; nothing to measure");
    assert!(
        (square - 2.0 * wide).abs() < 0.25 * wide.abs(),
        "at aspect 2 the material sits {wide} texels from the centre and at aspect 1 \
         it sits {square}, where twice {wide} was due — the canvas did not reach the \
         projection"
    );
}

// ---------------------------------------------------------------------------
// L3 — a camera that is a procedure
// ---------------------------------------------------------------------------

/// A camera on the clock alone, parameterised so a test can move it. Writes two
/// of the six outputs and leaves the other four to their defaults, which is what
/// the simplest camera anyone writes looks like.
fn sweep(dist: f32) -> String {
    format!(
        r#"
proc sweep {{
  kind L3
  param dist : float [1.0, 40.0] = {dist:?}
  camera {{
    eye    = vec3(dist, 0.0, 0.0);
    target = vec3(0.0, 0.0, 0.0);
  }}
}}
"#
    )
}

/// **Sized by its own param**, so a picture that changes when the param does
/// proves three things at once: the L3's pass ran, its uniform reached it, and
/// the state it wrote was what the derivation read.
const GAIN_DOT: &str = r#"
proc gain_dot {
  kind  L4
  blend additive

  param gain : float [0.0, 4.0] = 1.0

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 3.0;
  }

  fragment {
    color = vec4(gain, gain, gain, 1.0);
  }
}
"#;

fn with_camera(gpu: &Gpu, l3: Option<&str>, l4: &str, w: u32, h: u32) -> Set {
    let l3 = l3.map(compile);
    let l4 = compile(l4);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &compile(MARK),
        &[],
        l3.as_ref(),
        &[&l4],
        Layering::Overdraw,
        1,
        7,
    )
    .expect("one L1, an optional camera, and one L4");
    set.resize(&gpu.device, w, h);
    set.camera = pinned();
    set
}

/// **A camera procedure produces the view**, and its params reach it. The whole
/// path is on the GPU — a uniform write, a compute pass writing six numbers, a
/// second deriving a matrix, and a bind group — so moving the eye and watching
/// the material move is the only end-to-end proof there is.
#[test]
fn a_camera_procedure_produces_the_view() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;

    let offset = |dist: f32| {
        let mut set = with_camera(&gpu, Some(&sweep(dist)), GAIN_DOT, W, H);
        centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0
    };
    let far = offset(5.0);
    let near = offset(3.0);

    assert!(far.abs() > 4.0, "the material is on the centre column; nothing to measure");
    assert!(
        near.abs() > far.abs() * 1.3,
        "closing the camera's own `dist` from 5 to 3 moved the material from {far} texels off \
         centre to {near} — the procedure did not reach the frame"
    );
}

/// **And it can be addressed after the build**, like any other node's params.
#[test]
fn a_cameras_parameters_are_addressed_as_a_nodes() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;
    let mut set = with_camera(&gpu, Some(&sweep(5.0)), GAIN_DOT, W, H);

    let declared: Vec<(u32, &str, f32)> = set
        .params()
        .filter(|(layer, ..)| *layer == karakuri_ir::Kind::L3)
        .map(|(_, index, name, value)| (index, name, value))
        .collect();
    assert_eq!(
        declared,
        vec![(0, "dist", 5.0)],
        "the camera's params are not reported as the camera's"
    );

    let far = centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0;
    assert!(set.set_param_at(karakuri_ir::Kind::L3, 0, "dist", 3.0), "the camera declares `dist`");
    let near = centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0;
    assert!(
        near.abs() > far.abs() * 1.3,
        "writing the camera's `dist` left the material at {near}, against {far}"
    );
}

/// **A camera between the deformations and the renderers does not shift what a
/// renderer reads.** This is a regression: the L4 uniform pass spelled out its
/// own slot arithmetic instead of asking [`Set::slot_of`], so inserting an L3
/// gave every renderer the node before it — and `soft_points` drew a black
/// frame, because its `exposure` resolved against the camera's parameter map
/// and came back missing.
///
/// The reading is a brightness rather than a position, on purpose: a shifted map
/// leaves a declared param with no value, which the uniform path writes as
/// `0.0`. A renderer whose colour *is* its param then goes black — which is
/// exactly what happened, and is the one symptom a picture can show.
#[test]
fn a_camera_does_not_shift_the_parameters_a_renderer_reads() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 64;
    const H: u32 = 64;

    let peak = |l3: Option<&str>| {
        let mut set = with_camera(&gpu, l3, GAIN_DOT, W, H);
        frame(&gpu, &mut set, W, H)
            .chunks_exact(4)
            .map(|t| t[0])
            .fold(0.0f32, f32::max)
    };
    // The same camera either way, so the only difference between the two Sets
    // is whether a node sits between the geometry and the renderer.
    let built_in = peak(None);
    let procedure = peak(Some(&sweep(5.0)));

    assert!(built_in > 0.5, "the renderer's own default never reached the frame: {built_in}");
    assert!(
        (procedure - built_in).abs() < 0.01,
        "with a camera procedure the renderer drew at {procedure}, and without one at \
         {built_in} — a node was inserted and the renderer read the map beside its own"
    );
}

/// **The built-in orbit is not a second producer.** A Set whose camera is a
/// procedure has the `camera` field still on it — a `camera` record and a Set
/// file both set one — and it must reach nothing, because two producers writing
/// one edge would resolve by whichever ran last.
#[test]
fn an_orbit_assigned_beside_a_camera_procedure_reaches_nothing() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;

    let mut set = with_camera(&gpu, Some(&sweep(5.0)), GAIN_DOT, W, H);
    let before = centroid(&gpu, &mut set, W, H);
    // A camera nowhere near the procedure's, and pointed from above rather than
    // level, so anything of it that leaked would move the material a long way.
    set.camera = Orbit { radius: 20.0, height: 18.0, speed: 0.0, ..Default::default() };
    let after = centroid(&gpu, &mut set, W, H);

    assert!(
        (before.0 - after.0).abs() < 0.5 && (before.1 - after.1).abs() < 0.5,
        "assigning an orbit moved the material from {before:?} to {after:?} — the built-in \
         reached a Set whose camera is a procedure"
    );
}

/// **An address past a layer's last node reaches nothing**, rather than the
/// first node of the layer after it.
///
/// The parameter maps are laid end to end in node order, so `slot_of(layer) +
/// index` is a position and says nothing about whose it is. A Set with no
/// camera makes that concrete: with nothing between the deformations and the
/// renderers, `L3` and `L4` start at the same slot, and `--param
/// L3:0:exposure=0.0` reached renderer 0 and blacked out the frame — silently,
/// because the caller only reports an address that reached *zero* nodes.
///
/// Two addresses, and the second is the same defect without an L3 in it:
/// `L4:1:` on a Set of one renderer. That one is safe today only because the
/// renderers are last and their range runs to the end of the list, which is a
/// property of the ordering rather than of the check.
#[test]
fn an_address_past_a_layers_last_node_reaches_nothing() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = with_camera(&gpu, None, GAIN_DOT, 64, 64);

    assert_eq!(set.param("gain"), Some(1.0), "the renderer's declared default");
    assert!(
        !set.set_param_at(karakuri_ir::Kind::L3, 0, "gain", 0.0),
        "a Set with no camera has no L3 node to address"
    );
    assert_eq!(set.param("gain"), Some(1.0), "the L3 address reached the renderer");
    assert!(
        !set.set_param_at(karakuri_ir::Kind::L4, 1, "gain", 0.0),
        "this Set draws with one renderer, so index 1 addresses nothing"
    );
    assert_eq!(set.param("gain"), Some(1.0), "an out-of-range renderer address wrote anyway");
    // And the address that does exist still works, so the bound is a bound and
    // not a refusal.
    assert!(set.set_param_at(karakuri_ir::Kind::L4, 0, "gain", 0.25));
    assert_eq!(set.param("gain"), Some(0.25));
}

/// **A vector param is declared and not driven, and used to panic the render
/// thread for it.**
///
/// The language allows `param centre : vec3 …`; the engine has never driven one
/// — `Set::default_scalar` reads a scalar out of a declaration and skips
/// anything else, so a vector param never enters a node's value map. Every
/// node's uniform path nonetheless wrote *every* declared name as an `f32`, and
/// the packer panics on a field its layout says is a `vec3<f32>`. So a `.kir`
/// that parses, checks and costs took the render thread down on the first
/// `prepare` — not the swap worker, so not caught as `SetError::Panicked`.
///
/// Building and preparing is the whole test: the panic was unconditional.
#[test]
fn a_vector_param_is_left_undriven_rather_than_packed_as_a_scalar() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mixed = r#"
proc mixed {
  kind  L4
  blend additive

  param centre : vec3  [0.0, 1.0] = vec3(0.5, 0.5, 0.5)
  param gain   : float [0.0, 4.0] = 1.0

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 3.0;
  }

  fragment {
    color = vec4(gain, gain, gain, 1.0);
  }
}
"#;
    let mut set = with_camera(&gpu, Some(&sweep(5.0)), mixed, 64, 64);
    let peak = frame(&gpu, &mut set, 64, 64)
        .chunks_exact(4)
        .map(|t| t[0])
        .fold(0.0f32, f32::max);

    // And the scalar beside it still arrives, so the filter is a filter rather
    // than a node that gave up on its params.
    assert!(peak > 0.5, "the scalar param never reached the frame: {peak}");
    assert_eq!(set.param("gain"), Some(1.0));
    assert_eq!(set.param("centre"), None, "a vector param has no scalar value to hold");
}
