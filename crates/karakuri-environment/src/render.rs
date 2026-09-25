//! Offscreen rendering to PNG.
//! Composites live slots into a linear HDR target, tone-maps, and encodes to sRGB PNG (ADR-0258).

use std::path::Path;

use karakuri_engine::frame::{self, Committed, Sink, Skip};
use karakuri_engine::{Deck, Gpu, Look, Present};

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

/// Renders every frame from 0 to `frames` into `dir/%05d.png`.
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

/// Renders a session driven frame-by-frame by a stream playback closure rather than a clock.
///
/// `drive` provides the step count, look, and optional updated master chain per frame.
#[allow(clippy::too_many_arguments)]
pub fn replay(
    gpu: &Gpu,
    deck: &mut Deck,
    width: u32,
    height: u32,
    frames: u32,
    wanted: impl Fn(u32) -> Option<std::path::PathBuf>,
    resolve: &dyn Fn(&str) -> Option<String>,
    drive: impl FnMut(u32, &mut Deck) -> (u8, Look, Option<Vec<karakuri_engine::SlotSpec>>),
) -> Result<(), String> {
    sequence_driven(gpu, deck, width, height, frames, wanted, resolve, drive)
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
    // 1 step/frame, fixed look, no master chain.
    sequence_driven(
        gpu,
        deck,
        width,
        height,
        frames,
        wanted,
        &|_| None,
        move |_, _| (1, look, None),
    )
}

/// A [`Sink`] that writes selected frames to PNG files without stalling non-captured frames.
pub struct PngSink {
    format: wgpu::TextureFormat,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    readback: wgpu::Buffer,
    width: u32,
    height: u32,
    unpadded_row: u32,
    padded_row: u32,
    /// The path for the frame in flight, `None` when this one is not kept. Set by
    /// the loop before each frame — see [`PngSink::want`].
    writing: Option<std::path::PathBuf>,
}

impl PngSink {
    pub fn new(gpu: &Gpu, width: u32, height: u32) -> PngSink {
        // Texture-to-buffer copies require row pitch aligned to COPY_ALIGN.
        let unpadded_row = width * 4;
        let padded_row = unpadded_row.div_ceil(COPY_ALIGN) * COPY_ALIGN;

        // Rgba8UnormSrgb handles linear-to-sRGB hardware encoding on write.
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
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

        PngSink {
            format,
            target,
            view,
            readback,
            width,
            height,
            unpadded_row,
            padded_row,
            writing: None,
        }
    }

    /// Sets the destination path for the next frame, or `None` to skip disk write.
    pub fn want(&mut self, path: Option<std::path::PathBuf>) {
        self.writing = path;
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}

impl Sink for PngSink {
    fn acquire(&mut self, _gpu: &Gpu) -> Result<(), Skip> {
        Ok(())
    }

    fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn after_draw(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if self.writing.is_none() {
            return;
        }
        encoder.copy_texture_to_buffer(
            self.target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn present(&mut self, gpu: &Gpu) -> Result<(), String> {
        let Some(path) = self.writing.take() else {
            return Ok(());
        };
        let slice = self.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("{e}"))?;
        let mapped = slice.get_mapped_range().map_err(|e| format!("{e}"))?;
        let pixels = unpad_rows(&mapped, self.padded_row, self.unpadded_row);
        drop(mapped);
        self.readback.unmap();

        let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut png = png::Encoder::new(std::io::BufWriter::new(file), self.width, self.height);
        png.set_color(png::ColorType::Rgba);
        png.set_depth(png::BitDepth::Eight);
        png.write_header()
            .and_then(|mut w| w.write_image_data(&pixels))
            .map_err(|e| format!("{e}"))
    }
}

/// Drives sequence rendering frame-by-frame via `frame::compose`.
#[allow(clippy::too_many_arguments)]
fn sequence_driven(
    gpu: &Gpu,
    deck: &mut Deck,
    width: u32,
    height: u32,
    frames: u32,
    wanted: impl Fn(u32) -> Option<std::path::PathBuf>,
    resolve: &dyn Fn(&str) -> Option<String>,
    mut drive: impl FnMut(u32, &mut Deck) -> (u8, Look, Option<Vec<karakuri_engine::SlotSpec>>),
) -> Result<(), String> {
    let mut sink = PngSink::new(gpu, width, height);
    let mut present = Present::new(&gpu.device, sink.format(), width, height);

    // Simulation is advanced by whole steps, so frame N here is the same image
    // the window shows at frame N — no clock is involved anywhere, which is
    // what makes an offline preview and a live run agree.
    for i in 0..frames {
        sink.want(wanted(i));
        // Drive before compose to ensure chain lands on Present first.
        let (steps, look, chain) = drive(i, deck);
        if let Some(slots) = chain {
            if let Err(refusal) =
                crate::mix::install_chain(&mut present, &gpu.device, &gpu.queue, &slots, resolve)
            {
                eprintln!("  {refusal} — the chain keeps what it had");
            }
        }
        let mut refusal = None;
        let outcome = {
            let mut sinks: [&mut dyn Sink; 1] = [&mut sink];
            frame::compose(
                gpu,
                deck,
                &present,
                &mut sinks,
                &mut |at, skip| refusal = Some((at, skip)),
                |_| Committed { steps, look },
                |_| {},
            )?
        };
        // PngSink never skips; verify frame reached sink.
        if outcome.reached != 1 {
            return Err(format!(
                "the offscreen sink skipped a frame: {outcome:?} {refusal:?}"
            ));
        }
    }
    Ok(())
}

/// Removes row copy alignment padding from a mapped readback buffer.
fn unpad_rows(mapped: &[u8], padded_row: u32, unpadded_row: u32) -> Vec<u8> {
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

    /// Verifies row stride calculation rounds up to `COPY_ALIGN` only when necessary.
    #[test]
    fn a_row_is_padded_only_when_it_needs_to_be() {
        let stride = |width: u32| (width * 4).div_ceil(COPY_ALIGN) * COPY_ALIGN;
        assert_eq!(stride(1920), 7680, "already aligned");
        assert_eq!(stride(1280), 5120, "already aligned");
        assert_eq!(stride(800), 3328, "3200 rounds up to 3328");
        assert_eq!(stride(1366), 5632, "5464 rounds up to 5632");
        assert_eq!(stride(1), 256, "one texel still costs a whole alignment");
    }

    /// Verifies unpad_rows removes padding while preserving ordered pixel data across all rows.
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
