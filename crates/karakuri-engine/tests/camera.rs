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
        Set::build_many(&gpu.device, &gpu.queue, &compile(MARK), &[], &[&l4], 1, 7)
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
