use karakuri_console::view::{picture_rect, preview_rects, Picture, DECKS};
use karakuri_console::{egui, egui_wgpu};
use karakuri_engine::{Gpu, Present, Sink, Skip};

#[allow(unused_imports)]
use super::*;

/// What the view `egui` samples is called on the device. The texture carries
/// the name that says which of the two it is — see [`Presented::label`] — and
/// this says which view of that texture it is, so the pair reads as one thing
/// in a validation message rather than as two with the same name.
pub(crate) const SAMPLED_LABEL: &str = "as egui reads it";

/// A GPU texture rendered into by the engine and sampled by egui (ADR-0156).
pub(crate) struct Presented {
    /// Underlying GPU texture backing the render and sampled views.
    #[allow(dead_code)]
    pub(crate) texture: wgpu::Texture,
    /// What [`Present::draw`] draws into: sRGB, so the encode is the hardware's.
    pub(crate) target: wgpu::TextureView,
    /// The registration `egui` draws by, of a view of this texture in
    /// [`Presented::format`]'s `remove_srgb_suffix()`.
    pub(crate) id: egui::TextureId,
    /// The texture's size in physical pixels — its region's, not the window's.
    pub(crate) size: (u32, u32),
    /// What the texture is called on the device. Kept rather than passed to
    /// [`Presented::fit`], so the texture a resize makes is called what the one it
    /// replaces was called; and its own per texture, so a device message about the
    /// preview does not read as one about the picture.
    pub(crate) label: &'static str,
    /// Texture color format, typically sRGB for single-pass hardware encoding (P-0064).
    pub(crate) format: wgpu::TextureFormat,
    /// Whether this sink has an active display rectangle on the current frame.
    pub(crate) aimed: bool,
}

impl Presented {
    /// Creates and aims a presented texture sink matching an optional initial viewport rectangle.
    pub(crate) fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        label: &'static str,
        format: wgpu::TextureFormat,
        at: Option<egui::Rect>,
        scale: f32,
    ) -> Presented {
        let mut presented = Presented::made(gpu, renderer, label, format, (1, 1));
        let mut construction = 0;
        presented.aim(gpu, renderer, at, scale, &mut construction);
        presented
    }

    /// Allocates a texture, engine render target view, and egui sampled registration for the given dimensions.
    pub(crate) fn made(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        label: &'static str,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Presented {
        // Egui expects non-sRGB sampled view formats to avoid double-decoding texels in gamma space.
        let sampled_format = format.remove_srgb_suffix();
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            // COPY_SRC is not for the frame path — nothing here ever copies
            // one of these — it is what lets a test read a cell back and say
            // that a pass really was recorded into it. `Deck::slot_target`
            // carries the same flag for the same reason and says so.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            // Declared so the sampled view below may reinterpret it — the two
            // formats differ only in whether the transfer function is applied.
            view_formats: &[sampled_format],
        });
        let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampled = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(SAMPLED_LABEL),
            format: Some(sampled_format),
            ..Default::default()
        });
        let id = renderer.register_native_texture(&gpu.device, &sampled, wgpu::FilterMode::Linear);
        Presented {
            texture,
            target,
            id,
            size,
            label,
            format,
            aimed: false,
        }
    }

    /// Configures destination viewport rectangle and resizes texture if physical dimensions changed.
    pub(crate) fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        at: Option<egui::Rect>,
        scale: f32,
        freed: &mut usize,
    ) -> Option<Picture> {
        self.aimed = at.is_some();
        // If the sink has no destination rectangle, retain existing texture allocation without rendering.
        let rect = at?;
        self.fit(gpu, renderer, physical(rect, scale), freed);
        Some(Picture { id: self.id, rect })
    }

    /// Reallocates texture resources if the requested physical pixel dimensions differ from current size (P-0091).
    pub(crate) fn fit(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        size: (u32, u32),
        freed: &mut usize,
    ) -> bool {
        if size == self.size {
            return false;
        }
        // Freed **before** the replacement is registered, which is the order
        // the leak is about rather than a tidiness: the atlas holds the old
        // bind group until this call and nothing else ever drops it.
        renderer.free_texture(&self.id);
        *freed += 1;
        // Carried across, because a resize is not an un-aiming: the rectangle
        // this was aimed at is the one it was just resized to.
        let aimed = self.aimed;
        *self = Presented::made(gpu, renderer, self.label, self.format, size);
        self.aimed = aimed;
        true
    }
}

/// Engine output sink implementation for textures sampled into egui UI panels.
impl Sink for Presented {
    fn acquire(&mut self, _gpu: &Gpu) -> Result<(), Skip> {
        match self.aimed {
            true => Ok(()),
            // **[`Skip::Transient`] and never a `Fault`.** A folded region is
            // an operator's choice and the next `f` over it undoes it, so
            // there is nothing to say about it — and a `Fault` here would be
            // a line printed the first frame of every fold.
            false => Err(Skip::Transient),
        }
    }

    fn view(&self) -> &wgpu::TextureView {
        &self.target
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Nothing. The picture and deck A's cell are textures the panel samples in the
    /// same submission; what reaches a display is the window, and the window is not
    /// a sink here — `karakuri-cli`'s window shows the canvas, and this one shows
    /// the panel.
    fn present(&mut self, _gpu: &Gpu) -> Result<(), String> {
        Ok(())
    }
}

/// Which rectangle each of the engine's sinks is sized from and drawn into.
pub(crate) fn aims(
    layout: &karakuri_layout::Layout,
    canvas: (u32, u32),
) -> (Option<egui::Rect>, Option<[egui::Rect; DECKS]>) {
    (picture_rect(layout, canvas), preview_rects(layout, canvas))
}

/// Determines engine compositing dimensions by selecting the maximum area among enabled outputs (ADR-0171, ADR-0247).
pub(crate) fn render_size(outputs: &[Option<(u32, u32)>]) -> Option<(u32, u32)> {
    // **A fold rather than `max_by_key`**, which returns the *last* of several
    // equal maxima and would give the tie to whichever output happened to be
    // drawn furthest right. `>` and not `>=` is the tie rule said once.
    outputs.iter().flatten().copied().fold(None, |best, at| {
        let area = |(w, h): (u32, u32)| (w as u64) * (h as u64);
        match best {
            Some(b) if area(b) >= area(at) => Some(b),
            _ => Some(at),
        }
    })
}

/// Renders individual deck preview cells via tonemapping pass before mixing (ADR-0258).
pub(crate) fn monitor(
    present: &Present,
    previews: &mut [Presented; DECKS],
    bind_groups: &[Option<wgpu::BindGroup>; DECKS],
    encoder: &mut wgpu::CommandEncoder,
) {
    for (slot, pres) in previews.iter_mut().enumerate() {
        if pres.aimed {
            if let Some(bg) = &bind_groups[slot] {
                present.draw_with_bind_group(encoder, bg, &pres.target, pres.size);
            }
        }
    }
}

/// Converts logical pixel bounds to physical pixel dimensions based on the window scale factor.
pub(crate) fn physical(rect: egui::Rect, scale: f32) -> (u32, u32) {
    (
        ((rect.width() * scale).round() as u32).max(1),
        ((rect.height() * scale).round() as u32).max(1),
    )
}
