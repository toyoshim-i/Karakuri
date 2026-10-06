//! The WebXR stereo world: the aimed Set drawn once per eye into the
//! side-by-side XR canvas, which the WebGL2 XR layer samples behind the HUD.
//!
//! Each eye is drawn into its own HDR target at the Set's own viewport, never
//! into a shared larger one: the Set's depth, weighted-transparency and merge
//! targets are all that size, and attaching targets of different sizes to one
//! render pass is a validation error that silently drops the whole frame.
//!
//! Each eye is also its own submission. The eye's camera is written with
//! `queue.write_buffer`, which lands at the start of the *next* submit; two
//! writes before one submit would leave both eyes looking through the second.

use std::sync::Mutex;

use karakuri_engine::{DeckSlot, Present, StereoMatrices};

use crate::gfx::{EyeTarget, Gfx, StereoTarget, XR_CANVAS};

/// Draws the stereo world for one frame, reporting what happened through
/// `log` when it differs from the previous frame (so a stuck state is said
/// once, not sixty times a second).
pub(crate) fn draw_world(gfx: &mut Gfx, slot: DeckSlot, eyes: &(StereoMatrices, StereoMatrices)) {
    let said = match draw(gfx, slot, eyes) {
        Ok(size) => format!("drawing slot {} at {}x{} per eye", slot.0, size.0, size.1),
        Err(why) => format!("not drawn: {why}"),
    };
    static LAST: Mutex<String> = Mutex::new(String::new());
    if let Ok(mut last) = LAST.lock() {
        if *last != said {
            log::info!("Karakuri WebXR stereo world: {said}");
            *last = said;
        }
    }
}

fn draw(
    gfx: &mut Gfx,
    slot: DeckSlot,
    (left, right): &(StereoMatrices, StereoMatrices),
) -> Result<(u32, u32), String> {
    if gfx.xr_surface.is_none() {
        return Err("no XR canvas surface".into());
    }
    let size = gfx
        .engine
        .deck
        .stereo_viewport(Some(slot))
        .ok_or("the deck has no Set to draw")?;
    if size.0 == 0 || size.1 == 0 {
        return Err(format!("the Set's viewport is {}x{}", size.0, size.1));
    }
    if gfx.stereo_target.as_ref().map(|t| t.size) != Some(size) {
        gfx.stereo_target = Some(stereo_target(gfx, size));
    }

    let surface = gfx.xr_surface.as_ref().expect("checked above");
    let frame = match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
        other => return Err(format!("the XR canvas gave no texture: {other:?}")),
    };
    // The canvas is configured in its plain format with the sRGB variant as a
    // view format (`App::attach_xr_surface`); the present pipeline writes sRGB.
    let canvas = frame.texture.create_view(&wgpu::TextureViewDescriptor {
        format: Some(gfx.picture_format),
        ..Default::default()
    });

    let target = gfx.stereo_target.as_ref().expect("made above");
    let full = (0.0, 0.0, size.0 as f32, size.1 as f32);
    for (eye, matrices) in target.eyes.iter().zip([left, right]) {
        let mut encoder = gfx
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("WebXR stereo eye"),
            });
        // `None`: the Set's own depth, which is its viewport's size.
        gfx.engine.deck.draw_stereo_eye(
            &gfx.gpu.queue,
            &mut encoder,
            &eye.hdr_view,
            None,
            Some(slot),
            matrices,
            true,
            full,
        );
        gfx.gpu.queue.submit(std::iter::once(encoder.finish()));
    }

    let mut encoder = gfx
        .gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("WebXR stereo composite"),
        });
    let half = XR_CANVAS.0 as f32 * 0.5;
    let height = XR_CANVAS.1 as f32;
    for (i, eye) in target.eyes.iter().enumerate() {
        gfx.engine.present.draw_into_viewport(
            &mut encoder,
            &eye.bind_group,
            &canvas,
            (half * i as f32, 0.0, half, height),
            i == 0,
        );
    }
    gfx.gpu.queue.submit(std::iter::once(encoder.finish()));
    gfx.gpu.queue.present(frame);
    Ok(size)
}

fn stereo_target(gfx: &Gfx, size: (u32, u32)) -> StereoTarget {
    let eye = |label| {
        let hdr = gfx.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Present::HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let hdr_view = hdr.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = gfx
            .engine
            .present
            .create_bind_group_for(&gfx.gpu.device, &hdr_view);
        EyeTarget {
            hdr,
            hdr_view,
            bind_group,
        }
    };
    StereoTarget {
        size,
        eyes: [eye("WebXR left eye"), eye("WebXR right eye")],
    }
}
