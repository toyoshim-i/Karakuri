//! Auxiliary output sinks (Projector, WebXR spatial world, output plugins).

use super::*;

impl App {
    /// Sets active WebXR stereo camera matrices for Side-by-Side spatial world rendering.
    pub fn set_stereo_matrices(
        &mut self,
        matrices: Option<(
            karakuri_engine::StereoMatrices,
            karakuri_engine::StereoMatrices,
        )>,
    ) {
        self.stereo_matrices = matrices;
    }

    /// Sets one WebXR eye's size in pixels: while the stereo world is drawn
    /// it is the render size, and the XR canvas holds two of it side by side.
    #[cfg(target_arch = "wasm32")]
    pub fn set_stereo_eye_size(&mut self, size: Option<(u32, u32)>) {
        self.stereo_eye_size = size;
        if let (Some(gfx), Some((w, h))) = (self.gfx.as_mut(), size) {
            handler::stereo::fit_canvas(gfx, (w * 2, h));
        }
    }

    /// Returns true if the active stereo set in the deck mix is purely Fullscreen topology.
    pub fn is_fullscreen_stereo(&self) -> bool {
        self.gfx
            .as_ref()
            .map(|g| g.engine.deck.is_fullscreen_stereo())
            .unwrap_or(false)
    }

    /// Configures and attaches secondary WebXR canvas surface for SBS spatial world rendering.
    #[cfg(target_arch = "wasm32")]
    pub fn attach_xr_surface(&mut self, surface: wgpu::Surface<'static>) {
        if let Some(ref mut gfx) = self.gfx {
            gfx.xr_surface = Some(surface);
            let at = gfx.xr_canvas;
            gfx.xr_canvas = (0, 0);
            handler::stereo::fit_canvas(gfx, at);
            // Eye targets are sized to the drawn Set, which can change; they
            // are made on the first stereo frame (`stereo::draw_world`).
            gfx.stereo_target = None;
        }
    }

    /// Sets custom output plugin override state (available, display name, and active toggle).
    pub fn set_plugin_override(&mut self, available: bool, name: Option<&'static str>, on: bool) {
        self.custom_plugin = Some((available, name, on));
        self.readout.view.plugin_available = available;
        self.readout.view.plugin_name = name;
        self.readout.view.plugin = on;
        if let Some(ref gfx) = self.gfx {
            gfx.window.request_redraw();
        }
    }

    /// Registers a callback invoked when an output plugin sink is toggled (e.g. WebXR session).
    pub fn set_plugin_route_hook<
        F: FnMut(u8, bool, karakuri_console::view::XrSessionMode) + 'static,
    >(
        &mut self,
        hook: F,
    ) {
        self.plugin_route_hook = Some(Box::new(hook));
    }

    /// Triggers plugin route hook directly using disjoint field borrow.
    pub(crate) fn trigger_plugin_route(
        hook: &mut Option<PluginRouteHook>,
        n: u8,
        on: bool,
        mode: karakuri_console::view::XrSessionMode,
    ) {
        if let Some(ref mut h) = hook {
            h(n, on, mode);
        }
    }

    /// Sets callback invoked when the Projector output is toggled on wasm32.
    pub fn set_projector_route_hook<F: FnMut(bool) + 'static>(&mut self, hook: F) {
        self.projector_route_hook = Some(Box::new(hook));
    }

    /// Attaches an external projector surface on wasm32 (e.g. from an auxiliary window canvas).
    #[cfg(target_arch = "wasm32")]
    pub fn attach_projector(&mut self, surface: wgpu::Surface<'static>, size: (u32, u32)) {
        if let Some(ref mut gfx) = self.gfx {
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: gfx.picture_format,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: size.0.max(1),
                height: size.1.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&gfx.gpu.device, &config);
            gfx.projector = Some(crate::gfx::Projector {
                window: None,
                sink: karakuri_engine::WindowSink::new(surface, config),
                size: (size.0.max(1), size.1.max(1)),
            });
            self.readout.view.projector = true;
            gfx.window.request_redraw();
        }
    }

    /// Detaches the projector surface on wasm32 and turns off the Projector chip.
    #[cfg(target_arch = "wasm32")]
    pub fn detach_projector(&mut self) {
        if let Some(ref mut gfx) = self.gfx {
            gfx.projector = None;
            gfx.window.request_redraw();
        }
        self.readout.view.projector = false;
    }

    /// Resizes the projector surface on wasm32 when the auxiliary window is resized.
    #[cfg(target_arch = "wasm32")]
    pub fn resize_projector(&mut self, size: (u32, u32)) {
        if let Some(ref mut gfx) = self.gfx {
            if let Some(projector) = gfx.projector.as_mut() {
                let at = (size.0.max(1), size.1.max(1));
                projector.sink.resize(&gfx.gpu.device, at.0, at.1);
                projector.size = at;
                gfx.window.request_redraw();
            }
        }
    }
}
