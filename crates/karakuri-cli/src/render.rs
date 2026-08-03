//! Offscreen rendering to a PNG.
//!
//! The same path the window takes, minus the window: the deck composites its
//! Live slots into the linear HDR target, the present pass tone maps once and
//! encodes to sRGB exactly once — on write, because the destination texture
//! carries the transfer function. That makes this a preview of what is actually
//! on screen rather than a second rendering path that might disagree with it.
//!
//! **With several Sets this renders the mix**, for that reason and no other: it
//! is what the window shows. A flag that rendered each slot to its own file
//! would be a different feature — auditioning one candidate on its own is M2's
//! per-slot preview, it wants a default renderer per topology to be worth
//! having, and the deck already keeps every slot's target separate so that it
//! can be added without changing anything here. What the mix must not become is
//! a fourth definition of "the output"; there is one, and this is it.

use std::path::Path;

use karakuri_engine::{Deck, Gpu, Present};

use crate::Look;

/// Rows in a texture-to-buffer copy must be a multiple of this.
const COPY_ALIGN: u32 = 256;

/// A single frame at simulation frame `frames`.
pub fn to_png(
    gpu: &Gpu,
    deck: &mut Deck,
    look: Look,
    width: u32,
    height: u32,
    frames: u32,
    path: &Path,
) -> Result<(), String> {
    sequence(gpu, deck, look, width, height, frames, |i| {
        (i + 1 == frames).then(|| path.to_path_buf())
    })
}

/// Every frame from 0 to `frames`, into `dir/%05d.png`. The metadata format
/// carries a preview path per artifact, so rendering a sequence offline is a
/// thing the store will want; it is also the only way to look at motion without
/// a window.
pub fn to_sequence(
    gpu: &Gpu,
    deck: &mut Deck,
    look: Look,
    width: u32,
    height: u32,
    frames: u32,
    dir: &Path,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    sequence(gpu, deck, look, width, height, frames, |i| {
        Some(dir.join(format!("{i:05}.png")))
    })
}

/// Render a session's frames, driven by the stream rather than by a clock.
///
/// `drive` is called once before each frame with its index and the deck, and
/// returns the step count that frame's `tick` recorded.
#[allow(clippy::too_many_arguments)]
pub fn replay(
    gpu: &Gpu,
    deck: &mut Deck,
    look: Look,
    width: u32,
    height: u32,
    frames: u32,
    wanted: impl Fn(u32) -> Option<std::path::PathBuf>,
    drive: impl FnMut(u32, &mut Deck) -> u8,
) -> Result<(), String> {
    sequence_driven(gpu, deck, look, width, height, frames, wanted, drive)
}

fn sequence(
    gpu: &Gpu,
    deck: &mut Deck,
    look: Look,
    width: u32,
    height: u32,
    frames: u32,
    wanted: impl Fn(u32) -> Option<std::path::PathBuf>,
) -> Result<(), String> {
    sequence_driven(gpu, deck, look, width, height, frames, wanted, |_, _| 1)
}

/// [`sequence`] with the frame's step count, and whatever else has to happen,
/// decided per frame by the caller.
///
/// The seam a replay needs and the only thing it needs: a live run measures
/// `steps` from a clock and a replay reads it from a `tick`, and everything
/// else about rendering a frame is the same. `drive` is handed the frame index
/// and the deck, applies whatever the stream says belongs before that frame,
/// and returns what to advance by.
#[allow(clippy::too_many_arguments)]
fn sequence_driven(
    gpu: &Gpu,
    deck: &mut Deck,
    look: Look,
    width: u32,
    height: u32,
    frames: u32,
    wanted: impl Fn(u32) -> Option<std::path::PathBuf>,
    mut drive: impl FnMut(u32, &mut Deck) -> u8,
) -> Result<(), String> {
    assert_eq!(
        (width * 4) % COPY_ALIGN,
        0,
        "width {width} gives a {}-byte row, which is not a multiple of {COPY_ALIGN}",
        width * 4
    );

    // Rgba8UnormSrgb, so the hardware does the linear-to-sRGB encode on write.
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let present = Present::new(&gpu.device, format, width, height);
    present.set_tonemap(&gpu.queue, look.op, look.exposure, look.white_point);

    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("png target"),
        size: wgpu::Extent3d {
            width,
            height,
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

    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("png readback"),
        size: u64::from(width * height * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Simulation is advanced by whole steps, so frame N here is the same image
    // the window shows at frame N — no clock is involved anywhere, which is
    // what makes an offline preview and a live run agree.
    for i in 0..frames {
        // One `begin_frame` per encoder, and the guard is what says so: the
        // readback copy is recorded through `Frame::encoder` into the frame it
        // belongs to rather than into an encoder of this function's own.
        // Before the frame opens: `drive` may move a fader or a residency, and
        // a `Frame` holds the only `&mut Deck` there is while it is alive.
        let steps = drive(i, deck);
        let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
        frame.render(present.hdr_view(), present.size(), steps);

        let Some(path) = wanted(i) else {
            frame.finish();
            continue;
        };

        present.draw(frame.encoder(), &view);
        frame.encoder().copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        frame.finish();

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::Wait)
            .map_err(|e| format!("{e}"))?;
        let pixels = slice.get_mapped_range().to_vec();
        readback.unmap();

        let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut png = png::Encoder::new(std::io::BufWriter::new(file), width, height);
        png.set_color(png::ColorType::Rgba);
        png.set_depth(png::BitDepth::Eight);
        png.write_header()
            .and_then(|mut w| w.write_image_data(&pixels))
            .map_err(|e| format!("{e}"))?;
    }
    Ok(())
}
