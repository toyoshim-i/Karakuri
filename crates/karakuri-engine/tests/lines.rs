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

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
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

    /// A segment from `(x0, y)` to `(x1, y)` in NDC, `width_px` pixels across,
    /// with each endpoint scaled by its own `w` so the same picture can be asked
    /// for at more than one clip-space depth — and so one end can be put behind
    /// the eye while the other stays in front of it.
    ///
    /// Dividing by `w` is what makes the projected position independent of it:
    /// every `w` here describes the *same* NDC segment, which is what lets a depth
    /// be varied without varying anything else.
    ///
    /// **The width is given here in pixels and divided by [`H`] on the way in**,
    /// because `point_rate` is a fraction of the target's height and every claim
    /// below is about texels. The division belongs at this one boundary rather
    /// than at each of the eight call sites, and it is what lets those call sites
    /// keep saying "eight pixels" where "eight pixels" is the assertion.
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

    /// The same, given the rate itself rather than a pixel count.
    ///
    /// The three claims about what a rate *is* — that it is a share of the
    /// height, that the share survives a resize, and that widening the frame
    /// does not widen the sprite — are all about drawing one number at more than
    /// one target size, so none of them can go through a helper that divides by
    /// a fixed [`H`].
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

    /// RGBA f32 per texel, row-major, after one step of `l1` drawn by `l4`, at
    /// this file's own [`W`] x [`H`].
    fn draw(gpu: &Gpu, l1: &str, l4: &str) -> Vec<[f32; 4]> {
        draw_at(gpu, l1, l4, W, H)
    }

    /// The same, at a target size the caller picks.
    ///
    /// Separate from [`draw`] because two claims here are about what changes
    /// when the target does — that a sprite keeps its share of the frame's
    /// height, and that a wider frame does not widen it — and neither can be
    /// asked at one size.
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

    /// The load-bearing claim: a segment covers the rectangle its endpoints and
    /// its `point_rate` say it covers, to the texel. `point_rate` is a fraction
    /// of the target's height, so eight pixels on a 256-row target is `8 / 256`.
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

    /// `point_rate` is a width across the segment and nothing along it, and the
    /// number means what it says: the paired run at twice the width covers twice
    /// the rows and exactly the same columns.
    ///
    /// The pairing is the point. A single run cannot distinguish "a thirty-second
    /// of the height" from "a width the expansion happened to produce"; two runs
    /// whose only difference is the number pin the scale as well as the value.
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
            "a sprite is no longer `point_rate * H` pixels square"
        );
        assert_eq!(cells.len(), 8 * 8, "the sprite is not solid");
    }

    /// **A sprite covers the same fraction of the frame's height at any target
    /// size.** That is what `point_rate` means, and it is the whole reason the
    /// output stopped being a pixel count.
    ///
    /// One rate, 1/8, drawn at 256x256 and at 128x128. Thirty-two rows against
    /// sixteen: half the target, half the pixels, the same eighth of the frame.
    /// The bounds are exact rather than approximate because the quad's edges
    /// land on texel boundaries at both sizes — a sprite centred on NDC zero
    /// with a half-extent of 1/16 of the frame covers rows 112..=143 of 256 and
    /// 56..=71 of 128.
    ///
    /// **Both axes are asserted**, because the horizontal one is the axis that
    /// carries the aspect term and is therefore the one an arithmetic slip
    /// would break. The target is square here, so square in pixels and square
    /// in NDC agree; `a_wider_frame_does_not_widen_a_sprite` is what separates
    /// them.
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

    /// **A wider frame does not widen a sprite.** The rate is a fraction of the
    /// *height*, and that choice is exactly what this asserts: 256x256 against
    /// 512x256 puts the same 32x32 sprite in the middle of a frame that is twice
    /// as wide.
    ///
    /// Against the width instead, the same file at 21:9 would draw fatter
    /// material than at 16:9 — a second, unasked-for change to the picture every
    /// time somebody changed the canvas shape.
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

    /// **A stroke's width is the same fraction of the frame's height at any
    /// target size**, which is the same claim as the sprite's and a different
    /// piece of arithmetic: the segment expansion works in pixels and multiplies
    /// the rate by the target's height to get there.
    ///
    /// The length is asserted alongside the width so that a change to the one
    /// cannot be read as a change to the other: the segment runs NDC -0.5 to
    /// 0.5, which is half the frame's width whatever the frame is.
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

    /// **A sprite smaller than a texel is drawn at one texel and dimmed to
    /// compensate.** This is the sub-pixel rule where it reaches pixels.
    ///
    /// This is a quad pair and not a GL point, so no hardware minimum point size
    /// applies, and without MSAA a fragment exists only where the quad covers a
    /// pixel *centre*. `examples/soft_points.kir`'s default rate is 4/720; at
    /// 1280x720 that is the four-pixel sprite it was authored as, and at a tenth
    /// of that it is 0.4 pixels across.
    ///
    /// **This test asserted the absence, and the absence was the defect.** It
    /// read the empty cell as the rasterizer behaving and concluded that a
    /// preview cell has to be filled by downsampling — see `tests/deck.rs`,
    /// where that conclusion was cited. What the two numbers actually said is
    /// that a low-resolution *output* loses the same material, which no
    /// downsample rescues. The quad is now floored at one pixel and the
    /// fragment's alpha carries `s²`, the coverage the floor took.
    ///
    /// **128x72 rather than the console's own 112x63 cell**, because a readback
    /// needs its row length to be a multiple of 256 bytes and 112 texels at
    /// eight bytes is 896. Same tenth-scale, same conclusion, and the cell is
    /// smaller still.
    ///
    /// **Red carries the factor and alpha must not.** This file's fixture writes
    /// `color = vec4(1.0, …, 1.0)` into a `SrcAlpha, One` colour blend and a
    /// `One, OneMinusSrcAlpha` alpha blend, so red comes out as the compensated
    /// colour and alpha comes out as the coverage the sprite claimed. 0.4 across
    /// is 0.16 of a texel's area, and the coverage stays 1.0 — the compensation
    /// is paid in the light and not in the channel the mix's `over` hides
    /// behind. That is the whole of ADR-0245 in two numbers off one texel.
    ///
    /// **Both halves are the claim.** Sixteen texels at full brightness on the
    /// canvas says the mechanism is inert at or above a pixel; one dimmed texel
    /// in the cell says it is not inert below one.
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

    /// **A stroke thinner than a texel is compensated by its width and not by
    /// the square of it** — the one place the two topologies part company under
    /// the floor. A segment is short of coverage across its width and along none
    /// of its length, so squaring the factor would dim a thin stroke twice.
    ///
    /// The same rate and the same target as its sprite counterpart above, so the
    /// two numbers can be read against each other: 0.4 here, 0.16 there.
    ///
    /// The row count is asserted alongside the value, because a factor of 0.4
    /// applied to a stroke two rows tall would put the same light on screen and
    /// mean something else entirely.
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
