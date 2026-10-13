//! Asynchronous frame capture execution and background PNG encoding for MCP inspection (ADR-0393).

use karakuri_console::view::DECKS;
use karakuri_engine::Present;

/// A GPU frame readback and encoding job sent to a background worker.
pub(crate) struct CaptureJob {
    pub(crate) readback_buffer: wgpu::Buffer,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) unpadded_row: u32,
    pub(crate) padded_row: u32,
    pub(crate) is_bgra: bool,
    pub(crate) description: String,
    pub(crate) reply: karakuri_mcp::CaptureReply,
}

/// A target texture and drawing parameters recorded within the frame command encoder.
pub(crate) struct CaptureDraw {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) bind_group: Option<wgpu::BindGroup>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) padded_row: u32,
}

/// Prepares GPU targets, readback buffers, and worker jobs for pending frame capture requests.
pub(crate) fn prepare_captures(
    captures: Vec<karakuri_mcp::CaptureRequest>,
    present: &Present,
    slot_bind_groups: &[Option<wgpu::BindGroup>; DECKS],
    format: wgpu::TextureFormat,
    device: &wgpu::Device,
) -> (Vec<CaptureDraw>, Vec<CaptureJob>) {
    let mut draws = Vec::new();
    let mut jobs = Vec::new();

    let is_bgra = format!("{format:?}")
        .to_ascii_lowercase()
        .starts_with("bgra");

    for req in captures {
        let (canvas_w, canvas_h) = present.size();
        let max_dim = req.max_dimension;
        let (width, height) = if canvas_w >= canvas_h {
            let w = max_dim;
            let h = ((max_dim as f64 * canvas_h as f64 / canvas_w as f64).round() as u32).max(1);
            (w, h)
        } else {
            let h = max_dim;
            let w = ((max_dim as f64 * canvas_w as f64 / canvas_h as f64).round() as u32).max(1);
            (w, h)
        };

        let target_bg = match req.target {
            karakuri_mcp::CaptureTarget::Program => Ok(None),
            karakuri_mcp::CaptureTarget::Slot(slot) => {
                if slot < DECKS {
                    if let Some(ref bg) = slot_bind_groups[slot] {
                        Ok(Some(bg.clone()))
                    } else {
                        Err(format!("deck slot {slot} has no allocated texture view"))
                    }
                } else {
                    Err(format!("deck slot {slot} is out of bounds"))
                }
            }
        };

        let bind_group = match target_bg {
            Ok(bg) => bg,
            Err(err) => {
                req.reply.settled(Err(err));
                continue;
            }
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mcp capture target"),
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
        let view = texture.create_view(&Default::default());

        let unpadded_row = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row = (unpadded_row + align - 1) & !(align - 1);
        let buffer_size = u64::from(padded_row * height);

        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mcp capture readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let target_desc = match req.target {
            karakuri_mcp::CaptureTarget::Program => "program".to_string(),
            karakuri_mcp::CaptureTarget::Slot(slot) => format!("deck slot {slot}"),
        };
        let description = format!("captured {target_desc} frame ({width}x{height})");

        draws.push(CaptureDraw {
            texture,
            view,
            bind_group,
            width,
            height,
            padded_row,
        });
        jobs.push(CaptureJob {
            readback_buffer,
            width,
            height,
            unpadded_row,
            padded_row,
            is_bgra,
            description,
            reply: req.reply,
        });
    }

    (draws, jobs)
}

/// Renders capture targets and copies them into readback staging buffers within the frame encoder.
pub(crate) fn render_captures(
    encoder: &mut wgpu::CommandEncoder,
    present: &Present,
    draws: &[CaptureDraw],
    jobs: &[CaptureJob],
) {
    for (i, draw) in draws.iter().enumerate() {
        match &draw.bind_group {
            Some(bg) => {
                present.draw_with_bind_group(encoder, bg, &draw.view, (draw.width, draw.height));
            }
            None => present.draw(encoder, &draw.view, (draw.width, draw.height)),
        }
        encoder.copy_texture_to_buffer(
            draw.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &jobs[i].readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(draw.padded_row),
                    rows_per_image: Some(draw.height),
                },
            },
            wgpu::Extent3d {
                width: draw.width,
                height: draw.height,
                depth_or_array_layers: 1,
            },
        );
    }
}

/// Dispatches frame capture jobs to asynchronous worker threads or tasks (P-0094).
pub(crate) fn dispatch_capture_jobs(jobs: Vec<CaptureJob>, _device: wgpu::Device) {
    for job in jobs {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let device = _device.clone();
            std::thread::spawn(move || {
                let slice = job.readback_buffer.slice(..);
                slice.map_async(wgpu::MapMode::Read, |r| {
                    if let Err(e) = r {
                        eprintln!("frame capture map error: {e:?}");
                    }
                });
                if let Err(e) = device.poll(wgpu::PollType::wait_indefinitely()) {
                    job.reply
                        .settled(Err(format!("GPU device poll error during capture: {e}")));
                    return;
                }
                let mapped = match slice.get_mapped_range() {
                    Ok(m) => m,
                    Err(e) => {
                        job.reply
                            .settled(Err(format!("buffer get_mapped_range failed: {e}")));
                        return;
                    }
                };
                let mut pixels = karakuri_environment::render::unpad_rows(
                    &mapped,
                    job.padded_row,
                    job.unpadded_row,
                );
                drop(mapped);
                job.readback_buffer.unmap();

                if job.is_bgra {
                    for chunk in pixels.chunks_exact_mut(4) {
                        chunk.swap(0, 2);
                    }
                }

                match karakuri_environment::render::encode_png_rgba8(&pixels, job.width, job.height)
                {
                    Ok(png_bytes) => {
                        let base64_png = karakuri_environment::render::encode_base64(&png_bytes);
                        job.reply.settled(Ok(karakuri_mcp::CapturedImage {
                            description: job.description,
                            data: base64_png,
                            mime_type: "image/png".into(),
                            width: job.width,
                            height: job.height,
                        }));
                    }
                    Err(e) => {
                        job.reply.settled(Err(format!("PNG encode error: {e}")));
                    }
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            let CaptureJob {
                readback_buffer,
                width,
                height,
                unpadded_row,
                padded_row,
                is_bgra,
                description,
                reply,
            } = job;
            let buffer = std::sync::Arc::new(readback_buffer);
            let buffer_cb = buffer.clone();
            buffer.slice(..).map_async(wgpu::MapMode::Read, move |res| {
                if let Err(e) = res {
                    reply.settled(Err(format!("buffer map error: {e:?}")));
                    return;
                }
                let mapped = match buffer_cb.slice(..).get_mapped_range() {
                    Ok(m) => m,
                    Err(e) => {
                        reply.settled(Err(format!("buffer get_mapped_range failed: {e}")));
                        return;
                    }
                };
                let mut pixels =
                    karakuri_environment::render::unpad_rows(&mapped, padded_row, unpadded_row);
                drop(mapped);
                buffer_cb.unmap();

                if is_bgra {
                    for chunk in pixels.chunks_exact_mut(4) {
                        chunk.swap(0, 2);
                    }
                }

                match karakuri_environment::render::encode_png_rgba8(&pixels, width, height) {
                    Ok(png_bytes) => {
                        let base64_png = karakuri_environment::render::encode_base64(&png_bytes);
                        reply.settled(Ok(karakuri_mcp::CapturedImage {
                            description,
                            data: base64_png,
                            mime_type: "image/png".into(),
                            width,
                            height,
                        }));
                    }
                    Err(e) => {
                        reply.settled(Err(format!("PNG encode error: {e}")));
                    }
                }
            });
        }
    }
}
