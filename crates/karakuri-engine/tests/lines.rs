//! Pixel-level integration tests for line topology expansion.
//!
//! Asserts exact bounding boxes, pixel counts, and vertex attribute interpolation
//! across various depths, projections, and line rate settings.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
#[path = "common/mod.rs"]
mod common;

mod gpu {
    pub(super) use super::common::{compile, f16};
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};

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

    /// Returns an L4 renderer emitting a horizontal line segment with given NDC coordinates and pixel width.
    fn segment_l4(x0: f32, x1: f32, y: f32, width_px: f32, w: f32, w_b: f32) -> String {
        segment_rate_l4(x0, x1, y, width_px / H as f32, w, w_b)
    }

    /// The same, given the rate itself — for the one claim that draws a single
    /// number at two target sizes. See [`sprite_rate_l4`].
    fn segment_rate_l4(x0: f32, x1: f32, y: f32, width: f32, w: f32, w_b: f32) -> String {
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
    point_rate = {width:?};
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
    /// `width_px` converts the same way [`segment_l4`]'s does.
    fn sprite_l4(x0: f32, y: f32, width_px: f32) -> String {
        sprite_rate_l4(x0, y, width_px / H as f32)
    }

    /// Returns an L4 renderer emitting a point sprite with given rate.
    fn sprite_rate_l4(x0: f32, y: f32, width: f32) -> String {
        format!(
            r#"
proc sprite {{
  kind  L4
  blend additive

  consumes position

  vertex {{
    clip       = vec4({x0:?} + position.x, {y:?}, 0.0, 1.0);
    point_rate = {width:?};
  }}

  fragment {{
    color = vec4(1.0, point_coord.x, point_coord.y, 1.0);
  }}
}}
"#
        )
    }

    /// RGBA f32 per texel, row-major, after one step of `l1` drawn by `l4`, at
    /// this file's own [`W`] x [`H`].
    fn draw(gpu: &Gpu, l1: &str, l4: &str) -> Vec<[f32; 4]> {
        draw_at(gpu, l1, l4, W, H)
    }

    /// Renders the L1/L4 pair at the specified target dimensions and reads back RGBA f32 pixels.
    fn draw_at(gpu: &Gpu, l1: &str, l4: &str, w: u32, h: u32) -> Vec<[f32; 4]> {
        let l1 = compile(l1);
        let l4 = compile(l4);
        let mut set =
            Set::build(&gpu.device, &gpu.queue, &l1, &l4, 1, 7).expect("a compatible pair");
        // Without this the viewport uniform is 1x1 and every extent on screen is
        // meaningless — the one piece of engine state the expansion depends on.
        set.resize(&gpu.device, w, h);

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
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);
        set.commit();

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("map");
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
    fn at(px: &[[f32; 4]], x: u32, y: u32) -> [f32; 4] {
        px[(y * W + x) as usize]
    }

    /// Every texel the draw touched, as `(x, y)`. Red is 1.0 wherever the fragment
    /// ran and the target clears to zero, so this is coverage rather than a
    /// brightness threshold.
    fn covered(px: &[[f32; 4]]) -> Vec<(u32, u32)> {
        covered_wide(px, W)
    }

    /// The same, for a readback whose row length is not [`W`].
    fn covered_wide(px: &[[f32; 4]], w: u32) -> Vec<(u32, u32)> {
        px.iter()
            .enumerate()
            .filter(|(_, t)| t[0] > 0.0)
            .map(|(i, _)| (i as u32 % w, i as u32 / w))
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

    /// Verifies that a segment rasterizes to the exact bounding box and pixel count predicted by endpoints and width.
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

    /// Verifies that point_rate scales width perpendicularly without affecting segment length.
    #[test]
    fn width_is_across_the_segment_and_nothing_along_it() {
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

    /// Verifies that point_coord.x interpolates monotonically from clip to clip_b along the segment.
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

    /// Verifies that segments straddling the eye plane are culled rather than producing clipped artifacts.
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
            "a sprite is no longer `point_rate * H` pixels square"
        );
        assert_eq!(cells.len(), 8 * 8, "the sprite is not solid");
    }

    /// Verifies that point_rate maintains a constant fraction of viewport height across resolutions.
    #[test]
    fn a_sprite_is_the_same_fraction_of_the_frames_height_at_any_target_size() {
        let gpu = Gpu::headless().expect("no GPU available");
        let l4 = sprite_rate_l4(0.0, 0.0, 0.125);

        let big = bounds(&covered_wide(&draw_at(&gpu, L1, &l4, 256, 256), 256));
        let small = bounds(&covered_wide(&draw_at(&gpu, L1, &l4, 128, 128), 128));

        assert_eq!(
            big,
            (112, 143, 112, 143),
            "an eighth of 256 is not 32 texels"
        );
        assert_eq!(small, (56, 71, 56, 71), "an eighth of 128 is not 16 texels");

        let share = |b: (u32, u32, u32, u32), h: u32| f64::from(b.3 - b.2 + 1) / f64::from(h);
        assert_eq!(
            share(big, 256),
            share(small, 128),
            "the sprite is not the same share of the frame at the two sizes"
        );
    }

    /// Verifies that widening the frame aspect ratio does not distort or widen point sprite geometry.
    #[test]
    fn a_wider_frame_does_not_widen_a_sprite() {
        let gpu = Gpu::headless().expect("no GPU available");
        let l4 = sprite_rate_l4(0.0, 0.0, 0.125);

        let square = bounds(&covered_wide(&draw_at(&gpu, L1, &l4, 256, 256), 256));
        let wide = bounds(&covered_wide(&draw_at(&gpu, L1, &l4, 512, 256), 512));

        assert_eq!(
            (square.1 - square.0, square.3 - square.2),
            (wide.1 - wide.0, wide.3 - wide.2),
            "doubling the width changed the sprite's size in texels"
        );
        assert_eq!(
            (wide.0, wide.1, wide.2, wide.3),
            (240, 271, 112, 143),
            "the sprite is not 32 texels square in the middle of the wide frame"
        );
    }

    /// Verifies that stroke width maintains a constant fraction of viewport height across target sizes.
    #[test]
    fn a_strokes_width_is_the_same_fraction_of_the_frames_height_at_any_target_size() {
        let gpu = Gpu::headless().expect("no GPU available");
        let l4 = segment_rate_l4(-0.5, 0.5, 0.0, 0.125, 1.0, 1.0);

        let big = bounds(&covered_wide(&draw_at(&gpu, L1, &l4, 256, 256), 256));
        let small = bounds(&covered_wide(&draw_at(&gpu, L1, &l4, 128, 128), 128));

        assert_eq!(
            (big.3 - big.2 + 1, small.3 - small.2 + 1),
            (32, 16),
            "the stroke's width is not an eighth of the height at both sizes"
        );
        assert_eq!(
            (big.1 - big.0 + 1, small.1 - small.0 + 1),
            (128, 64),
            "the stroke's length is not half the frame's width at both sizes"
        );
    }

    /// Verifies that sub-pixel point sprites clamp to one texel and dim by area coverage (s²).
    #[test]
    fn a_sprite_below_a_texel_is_drawn_at_one_texel_and_dimmed_to_compensate() {
        let gpu = Gpu::headless().expect("no GPU available");
        // The rate `examples/soft_points.kir` carries, spelled as the division
        // it came from rather than as the decimal it rounds to.
        let l4 = sprite_rate_l4(0.0, 0.0, 4.0 / 720.0);

        let canvas_px = draw_at(&gpu, L1, &l4, 1280, 720);
        let canvas = covered_wide(&canvas_px, 1280);
        let cell_px = draw_at(&gpu, L1, &l4, 128, 72);
        let cell = covered_wide(&cell_px, 128);

        assert_eq!(
            canvas.len(),
            16,
            "four pixels square is not sixteen texels at 1280x720"
        );
        for &(x, y) in &canvas {
            let red = canvas_px[(y * 1280 + x) as usize][0];
            assert!(
                (red - 1.0).abs() < 1e-3,
                "a four-pixel sprite was compensated at all: texel ({x}, {y}) carries {red}"
            );
        }

        assert_eq!(
            cell.len(),
            1,
            "a 0.4-pixel sprite did not land on exactly one texel in a 128x72 cell, but on {:?}",
            cell
        );
        let (x, y) = cell[0];
        let texel = cell_px[(y * 128 + x) as usize];
        assert!(
            (texel[0] - 0.16).abs() < 1e-3,
            "a 0.4-pixel sprite carries {} rather than 0.4 squared",
            texel[0]
        );
        assert!(
            (texel[3] - 1.0).abs() < 1e-3,
            "the compensation reached the coverage channel: alpha is {} rather than 1.0",
            texel[3]
        );
    }

    /// Verifies that sub-pixel line strokes clamp to one texel and dim linearly by width rather than area.
    #[test]
    fn a_stroke_thinner_than_a_texel_is_compensated_by_its_width_rather_than_its_square() {
        let gpu = Gpu::headless().expect("no GPU available");
        let l4 = segment_rate_l4(-0.5, 0.5, 0.0, 4.0 / 720.0, 1.0, 1.0);

        let px = draw_at(&gpu, L1, &l4, 128, 72);
        let cells = covered_wide(&px, 128);

        let (_, _, y0, y1) = bounds(&cells);
        assert_eq!(y0, y1, "a one-texel stroke covered rows {y0}..={y1}");
        for &(x, y) in &cells {
            let texel = px[(y * 128 + x) as usize];
            assert!(
                (texel[0] - 0.4).abs() < 1e-3,
                "texel ({x}, {y}) carries {} rather than the stroke's 0.4 of a texel",
                texel[0]
            );
            assert!(
                (texel[3] - 1.0).abs() < 1e-3,
                "texel ({x}, {y}) has coverage {} rather than 1.0, so the factor reached alpha",
                texel[3]
            );
        }
    }
}
