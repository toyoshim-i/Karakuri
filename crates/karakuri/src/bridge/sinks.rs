use karakuri_console::view::{picture_rect, preview_rects, Picture, DECKS};
use karakuri_console::{egui, egui_wgpu};
use karakuri_engine::{Gpu, Present, Sink, Skip};

#[allow(unused_imports)]
use super::*;

/// What the picture and every preview are rendered in: an sRGB format, so the
/// hardware does the one encode `Present`'s shader relies on ([P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)).
pub(crate) const PICTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// **The same texels, read as if they were not sRGB, which is how `egui` wants
/// them.**
///
/// `egui`'s fragment shader says so outright — *"We expect 'normal' textures
/// that are NOT sRGB-aware"* — and multiplies the sample by the vertex tint in
/// gamma space. A view in [`PICTURE_FORMAT`] would have the hardware decode to
/// linear on the way in and `egui` would then write those linear values into a
/// gamma-space surface, which is a picture that comes out visibly dark with no
/// error anywhere. So the render target is sRGB, the sampled view is this, and
/// the encode still happens exactly once — in the present pass.
pub(crate) const PICTURE_SAMPLED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// What the view `egui` samples is called on the device. The **texture**
/// carries the name that says which of the two it is — see
/// [`Presented::label`] — and this says which view of that texture it is, so
/// the pair reads as one thing in a validation message rather than as two with
/// the same name.
pub(crate) const SAMPLED_LABEL: &str = "as egui reads it";

/// **A texture the engine presents into and the panel samples**: the texture
/// itself, the view [`Present::draw`] draws into, the registration `egui`
/// reads it by, and its size in physical pixels.
///
/// **Two call sites on the day it is written**, which is this repository's
/// rule about an abstraction and is the same standing
/// [`karakuri_console::view::bay_head`] has: the Program bay's picture, and
/// deck A's preview cell under it. The four fields were `Engine`'s own until
/// the second one needed them, and every argument written on them then is
/// written on them here, because all of it is still true of both.
///
/// # It is a [`Sink`], and the aiming is the half a `Sink` cannot carry
///
/// [`Sink::acquire`] is handed a `&Gpu` and nothing else, and sizing one of
/// these needs the region's rectangle, the scale factor and the
/// [`egui_wgpu::Renderer`] the registration lives in. So the frame does that
/// first, through [`Presented::aim`], and `acquire` answers from what it was
/// aimed at: a rectangle means a target, no rectangle means [`Skip::Transient`]
/// and nothing at all is drawn into it.
///
/// **The split is where it is because of what a test can call.** Deciding
/// *which rectangle, at what size* inside `window_event` is deciding it
/// somewhere `winit` will not let a test reach — and sizing deck A's texture
/// from the picture's rectangle was injected there once and every test still
/// passed. [`aims`] and [`Presented::aim`] are that decision, whole, outside
/// the event handler; `mod gpu` calls them the way the frame does.
///
/// **Neither of these is presented anywhere**, which is why
/// [`Sink::present`] is `Ok(())` for both: the picture and the preview cell
/// are textures the panel samples, and the thing that reaches a display is the
/// window — which is not a sink here. `karakuri-cli`'s window shows the
/// canvas; this window shows the panel, and the panel goes in `finally`.
pub(crate) struct Presented {
    /// The texture. Held because the views below are of it, and because
    /// freeing the `egui` registration does not free this — `egui-wgpu` stores
    /// a bind group for a registered native texture and no texture at all, so
    /// dropping this is the only thing that releases the memory. Read only by
    /// `mod gpu`, which asks it what size it came out, and that is the whole
    /// of why the lint has to be told: the window does not read it and the
    /// window is not what it is for.
    #[allow(dead_code)]
    pub(crate) texture: wgpu::Texture,
    /// What [`Present::draw`] draws into: sRGB, so the encode is the
    /// hardware's.
    pub(crate) target: wgpu::TextureView,
    /// The registration `egui` draws by, of a view in
    /// [`PICTURE_SAMPLED_FORMAT`].
    pub(crate) id: egui::TextureId,
    /// The texture's size in physical pixels — **its region's**, not the
    /// window's.
    pub(crate) size: (u32, u32),
    /// What the texture is called on the device. Kept rather than passed to
    /// [`Presented::fit`], so the texture a resize makes is called what the
    /// one it replaces was called; and its own per texture, so a device
    /// message about the preview does not read as one about the picture.
    pub(crate) label: &'static str,
    /// **Whether this sink has a rectangle on screen this frame**, written by
    /// [`Presented::aim`] and read by nothing but [`Sink::acquire`].
    ///
    /// It is a field rather than an argument because the two questions are
    /// asked at different moments and by different callers: the frame aims
    /// every sink before it composes, and `compose` then asks each one for
    /// itself. A `Presented` that has never been aimed answers no, which is
    /// the right answer — it has a 1x1 placeholder texture and nothing has
    /// said where it goes.
    pub(crate) aimed: bool,
}

impl Presented {
    /// **A sink aimed at the rectangle its region has**, or at nothing where
    /// the region is folded away.
    ///
    /// It goes through [`Presented::aim`] rather than sizing the texture here,
    /// so that *which rectangle, at what size* has exactly one derivation in
    /// this file and the window's first frame cannot disagree with the window
    /// it opened at. The 1x1 below is never drawn into and never on screen: it
    /// is what `aim` replaces on the same statement.
    ///
    /// The `freed` it counts into is discarded, and that is the one place in
    /// this file where it may be. [`Engine::freed`] is a tally of registrations
    /// leaked by a **resize**, and this is construction — a caller that saw a
    /// 1 here would be told a leak had already happened before the window drew.
    pub(crate) fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        label: &'static str,
        at: Option<egui::Rect>,
        scale: f32,
    ) -> Presented {
        let mut presented = Presented::made(gpu, renderer, label, (1, 1));
        let mut construction = 0;
        presented.aim(gpu, renderer, at, scale, &mut construction);
        presented
    }

    /// The texture, the view the engine draws into, and the registration
    /// `egui` reads it by, at a size in physical pixels. One constructor
    /// because the three are made together and are replaced together, and
    /// private to this type because a caller aims at a rectangle — turning one
    /// into a size is [`Presented::aim`]'s and [`Presented::fit`]'s alone.
    pub(crate) fn made(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        label: &'static str,
        size: (u32, u32),
    ) -> Presented {
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
            format: PICTURE_FORMAT,
            // COPY_SRC is not for the frame path — nothing here ever copies
            // one of these — it is what lets a test read a cell back and say
            // that a pass really was recorded into it. `Deck::slot_target`
            // carries the same flag for the same reason and says so.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            // Declared so the sampled view below may reinterpret it — the two
            // formats differ only in whether the transfer function is applied.
            view_formats: &[PICTURE_SAMPLED_FORMAT],
        });
        let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampled = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(SAMPLED_LABEL),
            format: Some(PICTURE_SAMPLED_FORMAT),
            ..Default::default()
        });
        let id = renderer.register_native_texture(&gpu.device, &sampled, wgpu::FilterMode::Linear);
        Presented {
            texture,
            target,
            id,
            size,
            label,
            aimed: false,
        }
    }

    /// **Where this sink goes this frame, and how big its texture therefore
    /// is** — one statement, and it is the whole of what [`Sink::acquire`]
    /// then answers from.
    ///
    /// Returns what the console should draw, or `None` where there is no
    /// rectangle. **The returned [`Picture`] carries the same rectangle the
    /// texture was just sized from and the id the sizing may have just
    /// replaced**, which is the pairing `karakuri_console::view::Picture`'s own
    /// documentation asks for: whoever sized the texture and whoever placed it
    /// are one statement, so a texture sized from the window and drawn into the
    /// picture's region cannot be written by accident, and a resize cannot
    /// leave a freed id in the view.
    ///
    /// **Called before [`compose`], and it has to be**: `acquire` takes only a
    /// `&Gpu`, and this needs the rectangle, the scale factor and the
    /// renderer. It is also a reallocation on the frames a size changed, so it
    /// belongs at the top of the frame rather than mid-pass — see
    /// [`Presented::fit`].
    pub(crate) fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        at: Option<egui::Rect>,
        scale: f32,
        freed: &mut usize,
    ) -> Option<Picture> {
        self.aimed = at.is_some();
        // **No rectangle, so nothing to fit.** The texture is left exactly as
        // it was rather than shrunk: a folded region is one an operator
        // unfolds, and remaking it small and large again would put two
        // reallocations on a fold that costs none. Nothing is drawn into it
        // meanwhile — `acquire` refuses, which is the whole of what a fold
        // has to mean.
        let rect = at?;
        self.fit(gpu, renderer, physical(rect, scale), freed);
        Some(Picture { id: self.id, rect })
    }

    /// **The region is a different size, so the texture is remade at that size
    /// and the registration it replaces is freed.**
    ///
    /// Returns whether anything was remade, which is `false` on all but a
    /// handful of frames — every frame of a drag on the program's height is
    /// one of them, and that is exactly the case the free is for. A
    /// `register_native_texture` per dragged frame with no `free_texture`
    /// beside it is a bind group and a sampler leaked per frame, for as long
    /// as somebody keeps hold of a divider.
    ///
    /// A reallocation, so it is the frame's first act and not something done
    /// mid-pass ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
    /// is about the render thread, and this is that thread; what it costs is
    /// paid on the frames a size changed and on no others).
    ///
    /// **`freed` is handed in rather than kept here** because the tally is the
    /// whole engine's — one number over both textures, which is what
    /// [`Engine::freed`] is and what `mod gpu` asserts on. Taking it as an
    /// argument is what makes it impossible for a call site to remake a
    /// texture and forget to count what it freed.
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
        *self = Presented::made(gpu, renderer, self.label, size);
        self.aimed = aimed;
        true
    }
}

/// **The two textures are sinks, and this is the whole of what that costs.**
///
/// `acquire` answers from [`Presented::aim`] and nothing else; `present` is
/// `Ok(())` because neither of these is presented anywhere — they are sampled
/// by the panel, and the panel is drawn in [`compose`]'s `finally` rather than
/// by a sink. See the type's own documentation.
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

    /// **Nothing.** The picture and deck A's cell are textures the panel
    /// samples in the same submission; what reaches a display is the window,
    /// and the window is not a sink here — `karakuri-cli`'s window shows the
    /// canvas, and this one shows the panel.
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

/// **The one size the frame is composited at: the largest enabled output's.**
///
/// Each entry is one output — `Some(size)` for one that is on, `None` for one
/// that is off — in the order the Outputs row draws them, which is the program
/// view first. `None` back means **no output is enabled at all**, and the
/// caller leaves the size where it is rather than picking one.
///
/// # Largest by area, and the answer is always some output's own size
///
/// [ADR-0247](../../../docs/adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md)
/// says *the largest enabled output's size*, so that every output is a
/// downscale and none is ever upscaled. With two outputs of different shapes
/// that needs a rule, and there are two candidates. **A componentwise maximum
/// is refused**: 1920x1080 beside 1024x1280 would give 1920x1280, which is a
/// size no output is and nobody asked for, and it upscales both of them in one
/// axis while claiming to prevent upscaling. **Largest by pixel count wins**,
/// and it hands back one output's own size — so the frame is always exactly
/// right for at least one destination and a downscale for the rest. The tie
/// goes to the earlier entry, which is the program view, because the picture
/// is the output an operator is looking at while they decide.
///
/// # Every output off leaves the size where it was
///
/// **An output going off is not a statement about what the frame should be
/// rendered at.** Turning the last one off stops the publishing and not the
/// instrument
/// ([ADR-0171](../../../docs/adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md)),
/// and re-deriving a size there would reallocate every slot target, the mix
/// and the master chain in order to change a picture nobody is looking at —
/// and reallocate them again on the way back. So the fallback to
/// [`CANVAS`] is the run's **starting** value rather than a re-derivation, and
/// this function says *nothing to say* instead of guessing.
///
/// **The rejected reading is ADR-0247's own sentence taken literally** —
/// *"the session's `canvas` record is what it falls back to when no output
/// says anything"* — which is right about a run that has never had an output
/// and wrong about one whose outputs are all momentarily off, and cannot tell
/// the two apart from inside this function. The starting value is where that
/// sentence is honoured; see [`Engine::aim`].
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

/// **One tone-mapping pass per deck preview cell, off that slot's own target,
/// whatever the slot's residency.**
///
/// The engine drew every slot into its own target a moment before this — Live
/// ones stepped and drawn, off-air ones drawn and not stepped — so this reads
/// [`DECKS`] fresh images and never the composite. Through the same [`Present`]
/// the picture goes through, so a cell is the material under the transfer curve
/// the room gets, with no fader on it, because a fader is an edge property
/// applied in the mix and a slot's target is upstream of the mix. That is
/// what [ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
/// asks of a monitor, in one loop.
///
/// **Gated on `aimed` and on there being a slot to sample, and on nothing
/// else.** Residency was the third gate and is not: a cell that goes dark when
/// its deck goes off air is dark at exactly the moment an operator is deciding
/// whether to bring it back. [`Engine::aim`] asks the same two questions, so a
/// cell cannot be aimed and not drawn.
///
/// A function rather than the body of the closure it is called from, for
/// [`live`]'s reason: `window_event` cannot be called from a test, so the part
/// worth asserting is lifted to where one can reach it — see
/// `every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency`.
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

/// A rectangle in logical pixels, in physical ones. **Both of the engine's
/// textures are sized from this and from nothing else** — the picture from its
/// region, deck A's preview from its cell — which is what makes each of them
/// the size of what it is drawn into rather than of the window. Which
/// rectangle each one gets is [`aims`]; this is the *at what size* half, and
/// [`Presented::aim`] is the one place the two meet.
pub(crate) fn physical(rect: egui::Rect, scale: f32) -> (u32, u32) {
    (
        ((rect.width() * scale).round() as u32).max(1),
        ((rect.height() * scale).round() as u32).max(1),
    )
}
