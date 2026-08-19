//! `topology lines`, asserted in pixels.
//!
//! Everything here draws **one** element, whose two clip-space endpoints the
//! `vertex` block writes as literals rather than deriving through `camera`.
//! That is the whole point of the fixture: a segment placed by the camera can
//! only be checked for looking plausible, and a segment placed at NDC
//! (-0.5, 0) to (0.5, 0) has a bounding box arithmetic can predict to the
//! texel. Every assertion below is an exact pixel count or an exact
//! rectangle — the class of claim that survives someone changing the
//! expansion and keeping it "roughly right".
//!
//! The alpha is 1.0 everywhere and the blend is `SrcAlpha, One`, so what lands
//! in the target is the fragment's colour unmultiplied. That lets the channels
//! carry information: red says a texel was drawn at all, green carries
//! `point_coord.x` (along the segment) and blue `point_coord.y` (across it).

use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

const W: u32 = 256;
const H: u32 = 256;

/// One element, placed at the origin and never moved. Only its existence
/// matters — every coordinate in these tests comes from the L4.
const L1: &str = r#"
proc one_element {
  kind     L1
  topology lines
  capacity [1, 64] = 1

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

/// A segment from `(x0, y)` to `(x1, y)` in NDC, `width` pixels across, with
/// each endpoint scaled by its own `w` so the same picture can be asked for at
/// more than one clip-space depth — and so one end can be put behind the eye
/// while the other stays in front of it.
///
/// Dividing by `w` is what makes the projected position independent of it:
/// every `w` here describes the *same* NDC segment, which is what lets a depth
/// be varied without varying anything else.
fn segment_l4(x0: f32, x1: f32, y: f32, width: f32, w: f32, w_b: f32) -> String {
    format!(
        r#"
proc segment {{
  kind  L4
  blend additive

  consumes position

  vertex {{
    // `position` is zero, and reading it is what keeps `consumes` honest:
    // the element buffer is bound and read, exactly as in a real procedure.
    clip       = vec4({x0:?} * {w:?} + position.x, {y:?} * {w:?}, 0.0, {w:?});
    clip_b     = vec4({x1:?} * {w_b:?}, {y:?} * {w_b:?}, 0.0, {w_b:?});
    point_size = {width:?};
  }}

  fragment {{
    color = vec4(1.0, point_coord.x, point_coord.y, 1.0);
  }}
}}
"#
    )
}

/// The same procedure with the second endpoint removed: a sprite, not a
/// segment. The control for every claim below that names the lines path.
fn sprite_l4(x0: f32, y: f32, width: f32) -> String {
    format!(
        r#"
proc sprite {{
  kind  L4
  blend additive

  consumes position

  vertex {{
    clip       = vec4({x0:?} + position.x, {y:?}, 0.0, 1.0);
    point_size = {width:?};
  }}

  fragment {{
    color = vec4(1.0, point_coord.x, point_coord.y, 1.0);
  }}
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

/// RGBA f32 per texel, row-major, after one step of `l1` drawn by `l4`.
fn draw(gpu: &Gpu, l1: &str, l4: &str) -> Vec<[f32; 4]> {
    let l1 = compile(l1);
    let l4 = compile(l4);
    let mut set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 1, 7).expect("a compatible pair");
    // Without this the viewport uniform is 1x1 and every width in pixels is
    // meaningless — the one piece of engine state the expansion depends on.
    set.resize(&gpu.device, W, H);

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
/// same way `tests/deck.rs` does it and for the same reason.
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

/// Every texel the draw touched, as `(x, y)`. Red is 1.0 wherever the fragment
/// ran and the target clears to zero, so this is coverage rather than a
/// brightness threshold.
fn covered(px: &[[f32; 4]]) -> Vec<(u32, u32)> {
    px.iter()
        .enumerate()
        .filter(|(_, t)| t[0] > 0.0)
        .map(|(i, _)| (i as u32 % W, i as u32 / W))
        .collect()
}

fn bounds(cells: &[(u32, u32)]) -> (u32, u32, u32, u32) {
    let xs: Vec<u32> = cells.iter().map(|c| c.0).collect();
    let ys: Vec<u32> = cells.iter().map(|c| c.1).collect();
    (
        *xs.iter().min().expect("nothing was drawn"),
        *xs.iter().max().expect("nothing was drawn"),
        *ys.iter().min().expect("nothing was drawn"),
        *ys.iter().max().expect("nothing was drawn"),
    )
}

/// The load-bearing claim: a segment covers the rectangle its endpoints and
/// its `point_size` say it covers, to the texel.
///
/// NDC -0.5 and 0.5 are pixel columns 64 and 192 on a 256-wide target, and a
/// pixel is covered when its *centre* is, so the columns run 64..=191 — 128 of
/// them. Eight pixels of width centred on row 128 covers rows 124..=131. That
/// is 1024 texels and no others.
#[test]
fn a_segment_covers_exactly_the_rectangle_its_endpoints_and_width_describe() {
    let gpu = Gpu::headless().expect("no GPU available");
    let px = draw(&gpu, L1, &segment_l4(-0.5, 0.5, 0.0, 8.0, 1.0, 1.0));
    let cells = covered(&px);

    assert_eq!(
        bounds(&cells),
        (64, 191, 124, 131),
        "the segment is not where its endpoints put it"
    );
    assert_eq!(cells.len(), 128 * 8, "the rectangle has holes or spills");
}

/// `point_size` is a width in pixels, and the number means what it says: the
/// paired run at twice the width covers twice the rows and exactly the same
/// columns.
///
/// The pairing is the point. A single run cannot distinguish "eight pixels
/// wide" from "a width the expansion happened to produce"; two runs whose only
/// difference is the number pin the scale as well as the value.
#[test]
fn width_is_pixels_across_the_segment_and_nothing_along_it() {
    let gpu = Gpu::headless().expect("no GPU available");
    let thin = bounds(&covered(&draw(
        &gpu,
        L1,
        &segment_l4(-0.5, 0.5, 0.0, 8.0, 1.0, 1.0),
    )));
    let thick = bounds(&covered(&draw(
        &gpu,
        L1,
        &segment_l4(-0.5, 0.5, 0.0, 24.0, 1.0, 1.0),
    )));

    assert_eq!(
        (thin.0, thin.1),
        (thick.0, thick.1),
        "width changed the segment's length"
    );
    assert_eq!(thin.3 - thin.2 + 1, 8, "8 pixels of width is not 8 rows");
    assert_eq!(
        thick.3 - thick.2 + 1,
        24,
        "24 pixels of width is not 24 rows"
    );
}

/// `point_coord.x` runs from `clip` to `clip_b`, not the other way and not
/// across.
///
/// Read at the two ends of the same stroke, on its centre row. Without a
/// direction the two would agree; with the wrong one they would swap, which is
/// why both ends are asserted rather than the difference between them.
#[test]
fn point_coord_runs_along_the_segment_from_clip_to_clip_b() {
    let gpu = Gpu::headless().expect("no GPU available");
    let px = draw(&gpu, L1, &segment_l4(-0.5, 0.5, 0.0, 8.0, 1.0, 1.0));

    let head = at(&px, 64, 128)[1];
    let tail = at(&px, 191, 128)[1];
    assert!(
        head < 0.02,
        "point_coord.x at the `clip` end is {head}, not ~0"
    );
    assert!(
        tail > 0.98,
        "point_coord.x at the `clip_b` end is {tail}, not ~1"
    );

    // And a stroke drawn the other way round reports the reverse, which is
    // what says the coordinate follows the endpoints rather than the screen.
    let flipped = draw(&gpu, L1, &segment_l4(0.5, -0.5, 0.0, 8.0, 1.0, 1.0));
    assert!(
        at(&flipped, 64, 128)[1] > 0.98,
        "reversing the endpoints did not reverse point_coord.x"
    );
}

/// `point_coord.y` runs across the segment, spanning its full width.
#[test]
fn point_coord_runs_across_the_segment_edge_to_edge() {
    let gpu = Gpu::headless().expect("no GPU available");
    let px = draw(&gpu, L1, &segment_l4(-0.5, 0.5, 0.0, 8.0, 1.0, 1.0));

    let near = at(&px, 128, 124)[2];
    let far = at(&px, 128, 131)[2];
    assert!(
        (near - far).abs() > 0.8,
        "point_coord.y barely moves across the stroke: {near} to {far}"
    );
    assert!(
        near.min(far) < 0.1 && near.max(far) > 0.9,
        "point_coord.y does not reach both edges"
    );
}

/// The expansion divides by `w` to work in pixels and multiplies by it again
/// on the way out. Two clip-space depths describing the same NDC segment must
/// therefore produce the same picture — which is the round trip, asserted
/// rather than argued.
#[test]
fn the_same_segment_at_two_clip_depths_draws_the_same_pixels() {
    let gpu = Gpu::headless().expect("no GPU available");
    let near = draw(&gpu, L1, &segment_l4(-0.5, 0.5, 0.0, 8.0, 1.0, 1.0));
    let far = draw(&gpu, L1, &segment_l4(-0.5, 0.5, 0.0, 8.0, 4.0, 4.0));

    assert_eq!(
        covered(&near),
        covered(&far),
        "w cancelled incorrectly: the depth moved the stroke"
    );
}

/// A segment that **straddles** the eye is dropped rather than drawn through
/// the camera — the deliberate limitation named in `SEGMENT_EXPANSION`.
///
/// Straddling, and not simply behind, and the difference is the whole test.
/// With *both* endpoints behind the eye the rasterizer discards the triangles
/// on its own, so the first version of this passed with the guard deleted: it
/// asserted the hardware's behaviour and called it the guard's. One end in
/// front and one behind is the case the guard exists for — the manual divide
/// puts the far end at a plausible-looking pixel position, `w` crosses zero
/// along the quad, and the corners where it is still positive rasterize into a
/// wedge that has nothing to do with the geometry.
///
/// Paired with the identical segment wholly in front, because "nothing was
/// drawn" is what a broken fixture also produces.
#[test]
fn a_segment_straddling_the_eye_is_dropped_and_the_same_one_in_front_is_not() {
    let gpu = Gpu::headless().expect("no GPU available");
    let front = draw(&gpu, L1, &segment_l4(-0.5, 0.5, 0.0, 8.0, 1.0, 1.0));
    let straddling = draw(&gpu, L1, &segment_l4(-0.5, 0.5, 0.0, 8.0, 1.0, -1.0));

    assert!(
        !covered(&front).is_empty(),
        "the control drew nothing, so the fixture is broken"
    );
    assert!(
        covered(&straddling).is_empty(),
        "a segment straddling the eye reached the screen"
    );
}

/// The control for the whole file: the same L1, the same width, the same
/// position, drawn by an L4 that does *not* assign `clip_b`, is still a
/// sprite. If this drifted, every claim above would be about a path nothing
/// else takes.
#[test]
fn an_l4_without_clip_b_still_draws_a_square_sprite() {
    let gpu = Gpu::headless().expect("no GPU available");
    let cells = covered(&draw(&gpu, L1, &sprite_l4(0.0, 0.0, 8.0)));

    assert_eq!(
        bounds(&cells),
        (124, 131, 124, 131),
        "a sprite is no longer its point_size square"
    );
    assert_eq!(cells.len(), 8 * 8, "the sprite is not solid");
}
