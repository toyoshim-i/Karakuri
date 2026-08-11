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
    // **The row a texture-to-buffer copy needs is padded; the row a PNG needs is
    // not.** This used to assert the two were the same, which made every width
    // that is not a multiple of 64 a panic — and once the canvas became a
    // *record*, that stopped being a limitation of `--render` and became a way
    // to record a session nothing could replay: `--canvas 800x600` runs
    // perfectly live, and `--canvas` is refused with `--replay`, so there was
    // no way back. Padding here costs one copy per written frame and removes
    // the constraint instead of reporting it.
    let unpadded_row = width * 4;
    let padded_row = unpadded_row.div_ceil(COPY_ALIGN) * COPY_ALIGN;

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
        size: u64::from(padded_row * height),
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

        // The attachment is the canvas here: an offscreen render has no window
        // to fit into, so the viewport is the whole of it and no bars exist.
        present.draw(frame.encoder(), &view, (width, height));
        frame.encoder().copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
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
        let mapped = slice.get_mapped_range();
        let pixels = unpad_rows(&mapped, padded_row, unpadded_row);
        drop(mapped);
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

/// Drop the copy alignment padding from a mapped readback, leaving the rows a
/// PNG encoder expects.
///
/// **Extracted because the failure is silent and looks like art.** A row stride
/// that is one texel out does not error, does not change the file size, and
/// does not produce anything an eye reads as broken on generative material — it
/// produces a picture sheared by one texel per row, which on a soft point cloud
/// is indistinguishable from the material. The only way to catch it is to feed
/// it rows whose contents say which row they are.
fn unpad_rows(mapped: &[u8], padded_row: u32, unpadded_row: u32) -> Vec<u8> {
    // Nothing to strip on an aligned width, which is every default and most of
    // what anyone types — so the common case stays the copy it always was.
    if padded_row == unpadded_row {
        return mapped.to_vec();
    }
    mapped
        .chunks(padded_row as usize)
        .flat_map(|row| &row[..unpadded_row as usize])
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The padded stride is the next multiple of [`COPY_ALIGN`], and an already
    /// aligned width is left alone rather than pushed to the next one.
    ///
    /// `1920 * 4` is 7680, exactly thirty alignments — so the default canvas
    /// takes the untouched path, and a test that only used the default would
    /// never run the padding at all.
    #[test]
    fn a_row_is_padded_only_when_it_needs_to_be() {
        let stride = |width: u32| (width * 4).div_ceil(COPY_ALIGN) * COPY_ALIGN;
        assert_eq!(stride(1920), 7680, "already aligned");
        assert_eq!(stride(1280), 5120, "already aligned");
        assert_eq!(stride(800), 3328, "3200 rounds up to 3328");
        assert_eq!(stride(1366), 5632, "5464 rounds up to 5632");
        assert_eq!(stride(1), 256, "one texel still costs a whole alignment");
    }

    /// Every row comes back whole, in order, with the padding gone.
    ///
    /// The fixture numbers each row, so a stride that is out by one alignment —
    /// or by one byte — produces different bytes rather than a different length,
    /// which is the whole point: a length check passes on a sheared image.
    #[test]
    fn stripping_the_padding_keeps_every_row_where_it_was() {
        let (unpadded, padded, height) = (12u32, 16u32, 4u32);
        let mut mapped = Vec::new();
        for row in 0..height {
            mapped.extend(std::iter::repeat_n(row as u8, unpadded as usize));
            mapped.extend(std::iter::repeat_n(0xff, (padded - unpadded) as usize));
        }

        let out = unpad_rows(&mapped, padded, unpadded);
        assert_eq!(out.len(), (unpadded * height) as usize);
        for row in 0..height {
            let start = (row * unpadded) as usize;
            assert!(
                out[start..start + unpadded as usize]
                    .iter()
                    .all(|&b| b == row as u8),
                "row {row} came back as {:?}",
                &out[start..start + unpadded as usize]
            );
        }
        assert!(!out.contains(&0xff), "no padding survived");
    }

    /// An aligned readback is passed through untouched, which is the path every
    /// default size takes.
    #[test]
    fn an_aligned_readback_is_copied_rather_than_walked() {
        let mapped: Vec<u8> = (0..64).collect();
        assert_eq!(unpad_rows(&mapped, 16, 16), mapped);
    }
}
