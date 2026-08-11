//! Fitting the canvas into a window that is not its shape.
//!
//! The window is a preview of what leaves by some other route, so the property
//! under test is that it never *disagrees* with that route about framing: the
//! canvas keeps its aspect ratio and the leftover is black. Stretching would be
//! invisible — a full frame either way — which is exactly why it needs a test
//! rather than an eye.
//!
//! **The fixture is a full-frame wash, and that is the whole of why these tests
//! can see anything.** A letterbox defect lives at the *edges* of the output:
//! bars that should be there and are not, bars on the wrong axis, a viewport
//! one texel out. Material that draws in the middle of the frame — every other
//! fixture in this crate — leaves the edges black whether the fit is right or
//! wrong, and would pass with `set_viewport` deleted.

use karakuri_engine::{letterbox, Gpu, Present, TonemapOp};

/// A square canvas, so a target wider than it and a target taller than it are
/// the same case with the axes exchanged. An axis swap anywhere in the fit is
/// then a failure in one of the two rather than a pair of compensating errors.
const CANVAS: u32 = 64;

/// Fill the canvas edge to edge with full white and draw it into a
/// `target_w x target_h` attachment, returning the attachment as RGBA bytes.
///
/// The wash is a render pass that clears and draws nothing — the cheapest way
/// to get material that reaches all four edges, which is the one thing that
/// matters here. `Clamp` at exposure 1.0 so a lit texel is exactly 255 and a
/// bar is exactly 0: this is a test about *where* the light is, and an operator
/// that compressed the highlight would turn it into a question about how much.
fn fit(gpu: &Gpu, target_w: u32, target_h: u32) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let present = Present::new(&gpu.device, format, CANVAS, CANVAS);
    present.set_tonemap(&gpu.queue, TonemapOp::Clamp, 1.0, 1.0);

    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("preview target"),
        size: wgpu::Extent3d {
            width: target_w,
            height: target_h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());

    let bytes_per_row = target_w * 4;
    assert_eq!(bytes_per_row % 256, 0, "readback rows must stay aligned");
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("preview readback"),
        size: u64::from(bytes_per_row * target_h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    // Clears and draws nothing: the clear *is* the wash.
    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("wash"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: present.hdr_view(),
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    }));

    present.draw(&mut encoder, &view, (target_w, target_h));
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(target_h),
            },
        },
        wgpu::Extent3d {
            width: target_w,
            height: target_h,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let pixels = slice.get_mapped_range().to_vec();
    readback.unmap();
    pixels
}

/// The red channel of one texel of a `target_w`-wide readback.
fn texel(pixels: &[u8], target_w: u32, x: u32, y: u32) -> u8 {
    pixels[((y * target_w + x) * 4) as usize]
}

/// A square canvas in a target twice as wide: a quarter of black, a half of
/// canvas, a quarter of black, and the canvas reaching both of its own edges.
///
/// The last clause is the one that catches a viewport that is right about the
/// bars and wrong about the fill.
#[test]
fn a_wide_window_gets_bars_at_the_sides() {
    let gpu = Gpu::headless().expect("no GPU");
    let (w, h) = (CANVAS * 2, CANVAS);
    let pixels = fit(&gpu, w, h);

    let mid = h / 2;
    assert_eq!(texel(&pixels, w, 0, mid), 0, "the left edge is a bar");
    assert_eq!(
        texel(&pixels, w, CANVAS / 2 - 1, mid),
        0,
        "the texel before the canvas starts is still a bar"
    );
    assert_eq!(
        texel(&pixels, w, CANVAS / 2, mid),
        255,
        "the canvas starts at a quarter of the way across"
    );
    assert_eq!(
        texel(&pixels, w, CANVAS + CANVAS / 2 - 1, mid),
        255,
        "and reaches its own right edge"
    );
    assert_eq!(
        texel(&pixels, w, CANVAS + CANVAS / 2, mid),
        0,
        "the texel after it is a bar again"
    );
    assert_eq!(texel(&pixels, w, w - 1, mid), 0, "the right edge is a bar");

    // Full height, because only the width had to give: bars on the wrong axis
    // is a whole class of defect and this is what refuses it.
    assert_eq!(texel(&pixels, w, w / 2, 0), 255, "the top is canvas");
    assert_eq!(texel(&pixels, w, w / 2, h - 1), 255, "so is the bottom");
}

/// The same case with the axes exchanged. Present because a fit that returns
/// its offsets or its extents in the wrong order passes the test above.
#[test]
fn a_tall_window_gets_bars_at_the_top_and_bottom() {
    let gpu = Gpu::headless().expect("no GPU");
    let (w, h) = (CANVAS, CANVAS * 2);
    let pixels = fit(&gpu, w, h);

    let mid = w / 2;
    assert_eq!(texel(&pixels, w, mid, 0), 0, "the top edge is a bar");
    assert_eq!(
        texel(&pixels, w, mid, CANVAS / 2 - 1),
        0,
        "the texel before the canvas starts is still a bar"
    );
    assert_eq!(
        texel(&pixels, w, mid, CANVAS / 2),
        255,
        "the canvas starts a quarter of the way down"
    );
    assert_eq!(
        texel(&pixels, w, mid, CANVAS + CANVAS / 2 - 1),
        255,
        "and reaches its own bottom edge"
    );
    assert_eq!(
        texel(&pixels, w, mid, CANVAS + CANVAS / 2),
        0,
        "the texel after it is a bar again"
    );
    assert_eq!(texel(&pixels, w, mid, h - 1), 0, "the bottom edge is a bar");

    assert_eq!(texel(&pixels, w, 0, h / 2), 255, "full width");
    assert_eq!(texel(&pixels, w, w - 1, h / 2), 255, "to both edges");
}

/// A window the canvas's own shape has no bars anywhere — the case every
/// offscreen render is in, and the one a fit must not disturb.
#[test]
fn a_window_the_canvas_shape_is_all_canvas() {
    let gpu = Gpu::headless().expect("no GPU");
    let pixels = fit(&gpu, CANVAS, CANVAS);
    for (i, corner) in [
        (0, 0),
        (CANVAS - 1, 0),
        (0, CANVAS - 1),
        (CANVAS - 1, CANVAS - 1),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            texel(&pixels, CANVAS, corner.0, corner.1),
            255,
            "corner {i} is canvas, not a bar"
        );
    }
}

/// The arithmetic on its own, over shapes a GPU test would take a minute to
/// cover.
///
/// **Nothing else checks this.** wgpu validates a viewport against the device's
/// texture limits, not against the attachment, so a rectangle that hangs
/// outside is accepted and draws wrong — there is no error to notice and no
/// pixel comparison above that would catch a one-ulp overhang. The exact-fit
/// axis is where floating point puts it there: `c * (t / c)` is not promised to
/// be `t`.
#[test]
fn the_fitted_rectangle_never_leaves_the_attachment() {
    let sizes = [1u32, 2, 3, 7, 64, 65, 720, 1080, 1920, 4096];
    for &cw in &sizes {
        for &ch in &sizes {
            for &tw in &sizes {
                for &th in &sizes {
                    let (x, y, w, h) = letterbox((cw, ch), (tw, th));
                    assert!(x >= 0.0 && y >= 0.0, "{cw}x{ch} in {tw}x{th}: {x},{y}");
                    // At least one whole texel in each axis. `> 0.0` would
                    // admit a viewport that rounds away to nothing: 4096x1
                    // fitted into 1x4096 is 0.00024 tall, which draws an empty
                    // preview rather than a thin one.
                    assert!(w >= 1.0 && h >= 1.0, "{cw}x{ch} in {tw}x{th}: {w}x{h}");
                    assert!(
                        x + w <= tw as f32 && y + h <= th as f32,
                        "{cw}x{ch} in {tw}x{th} gave {x},{y} {w}x{h}, which leaves it"
                    );
                }
            }
        }
    }
}

/// One axis always fills, and it is the one that ran out first. A fit that
/// scaled by the *larger* ratio would overflow the attachment; one that scaled
/// by neither would leave bars on all four sides.
#[test]
fn the_tighter_axis_fills_exactly() {
    let (x, y, w, h) = letterbox((16, 9), (1600, 1200));
    assert_eq!((x, w), (0.0, 1600.0), "width is the tighter axis here");
    assert_eq!(h, 900.0);
    assert_eq!(y, 150.0, "and the leftover height is split in two");

    let (x, y, w, h) = letterbox((9, 16), (1600, 1200));
    assert_eq!((y, h), (0.0, 1200.0), "height is the tighter axis here");
    assert_eq!(w, 675.0);
    assert_eq!(x, 462.5, "and the leftover width is split in two");
}
