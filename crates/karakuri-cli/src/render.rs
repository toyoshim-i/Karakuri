//! Offscreen rendering to a PNG.
//!
//! The same path the window takes, minus the window: the Set renders into the
//! linear HDR target and the present pass encodes to sRGB exactly once, on
//! write, because the destination texture carries the transfer function. That
//! makes this a preview of what is actually on screen rather than a second
//! rendering path that might disagree with it.

use std::path::Path;

use karakuri_engine::{Gpu, Present, Set, VideoSource};

/// Rows in a texture-to-buffer copy must be a multiple of this.
const COPY_ALIGN: u32 = 256;

/// A single frame at simulation frame `frames`.
pub fn to_png(
    gpu: &Gpu,
    set: &mut Set,
    width: u32,
    height: u32,
    frames: u32,
    path: &Path,
) -> Result<(), String> {
    sequence(gpu, set, width, height, frames, |i| {
        (i + 1 == frames).then(|| path.to_path_buf())
    })
}

/// Every frame from 0 to `frames`, into `dir/%05d.png`. The metadata format
/// carries a preview path per artifact, so rendering a sequence offline is a
/// thing the store will want; it is also the only way to look at motion without
/// a window.
pub fn to_sequence(
    gpu: &Gpu,
    set: &mut Set,
    width: u32,
    height: u32,
    frames: u32,
    dir: &Path,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    sequence(gpu, set, width, height, frames, |i| {
        Some(dir.join(format!("{i:05}.png")))
    })
}

fn sequence(
    gpu: &Gpu,
    set: &mut Set,
    width: u32,
    height: u32,
    frames: u32,
    wanted: impl Fn(u32) -> Option<std::path::PathBuf>,
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
    set.resize(width, height);

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
        set.prepare(&gpu.queue, 1);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        set.render(&mut encoder, present.hdr_view(), 1);

        let Some(path) = wanted(i) else {
            gpu.queue.submit([encoder.finish()]);
            continue;
        };

        present.draw(&mut encoder, &view);
        encoder.copy_texture_to_buffer(
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
        gpu.queue.submit([encoder.finish()]);

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
