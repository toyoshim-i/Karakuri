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

    /// Sets callback invoked when Document Picture-in-Picture is toggled on wasm32.
    pub fn set_pip_route_hook<F: FnMut(bool) + 'static>(&mut self, hook: F) {
        self.pip_route_hook = Some(Box::new(hook));
    }

    /// Sets callback invoked when Share pill is pressed in Outputs row.
    pub fn set_share_hook<F: FnMut() + 'static>(&mut self, hook: F) {
        self.share_hook = Some(Box::new(hook));
    }

    /// Sets callback invoked when Logs pill is pressed in Outputs row.
    pub fn set_log_toggle_hook<F: FnMut() + 'static>(&mut self, hook: F) {
        self.log_toggle_hook = Some(Box::new(hook));
    }

    /// Attaches an external projector surface on wasm32 (e.g. from an auxiliary window canvas).
    #[cfg(target_arch = "wasm32")]
    pub fn attach_projector(&mut self, surface: wgpu::Surface<'static>, size: (u32, u32)) {
        if let Some(ref mut gfx) = self.gfx {
            gfx.engine
                .deck
                .grant_watchdog_grace(std::time::Duration::from_secs(3));
            for slot in 0..gfx.engine.deck.slot_count() {
                gfx.engine
                    .deck
                    .set_overloaded(karakuri_engine::DeckSlot(slot as u8), false);
            }
            let caps = surface.get_capabilities(&gfx.gpu.adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let view_formats = if format != gfx.picture_format {
                vec![gfx.picture_format]
            } else {
                vec![]
            };
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: size.0.max(1),
                height: size.1.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: caps.alpha_modes[0],
                view_formats,
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
            gfx.engine
                .deck
                .grant_watchdog_grace(std::time::Duration::from_secs(3));
            for slot in 0..gfx.engine.deck.slot_count() {
                gfx.engine
                    .deck
                    .set_overloaded(karakuri_engine::DeckSlot(slot as u8), false);
            }
            gfx.projector = None;
            gfx.window.request_redraw();
        }
        self.readout.view.projector = false;
    }

    /// Resizes the projector surface on wasm32 when the auxiliary window is resized.
    #[cfg(target_arch = "wasm32")]
    pub fn resize_projector(&mut self, size: (u32, u32)) {
        if let Some(ref mut gfx) = self.gfx {
            gfx.engine
                .deck
                .grant_watchdog_grace(std::time::Duration::from_secs(2));
            if let Some(projector) = gfx.projector.as_mut() {
                let at = (size.0.max(1), size.1.max(1));
                projector.sink.resize(&gfx.gpu.device, at.0, at.1);
                projector.size = at;
                gfx.window.request_redraw();
            }
        }
    }

    /// Sets cast plugin override state (available, display name, and active toggle).
    pub fn set_cast_override(&mut self, available: bool, on: bool) {
        self.readout.view.plugin_1_available = available;
        self.readout.view.plugin_1_name = Some("Cast");
        self.readout.view.plugin_1 = on;
        if let Some(ref gfx) = self.gfx {
            gfx.window.request_redraw();
        }
    }

    /// Attaches an offscreen Cast canvas surface for WebRTC stream capture on wasm32.
    #[cfg(target_arch = "wasm32")]
    pub fn attach_cast_surface(&mut self, surface: wgpu::Surface<'static>, size: (u32, u32)) {
        if let Some(ref mut gfx) = self.gfx {
            let caps = surface.get_capabilities(&gfx.gpu.adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let view_formats = if format != gfx.picture_format {
                vec![gfx.picture_format]
            } else {
                vec![]
            };
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: size.0.max(1),
                height: size.1.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: caps.alpha_modes[0],
                view_formats,
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&gfx.gpu.device, &config);
            gfx.cast_sink = Some(karakuri_engine::WindowSink::new(surface, config));
            self.readout.view.plugin_1 = true;
            gfx.window.request_redraw();
        }
    }

    /// Detaches the cast surface on wasm32 and turns off the Cast chip.
    #[cfg(target_arch = "wasm32")]
    pub fn detach_cast_surface(&mut self) {
        if let Some(ref mut gfx) = self.gfx {
            gfx.cast_sink = None;
            gfx.window.request_redraw();
        }
        self.readout.view.plugin_1 = false;
    }

    /// Sets Document Picture-in-Picture state (available and active toggle).
    pub fn set_pip_override(&mut self, available: bool, on: bool) {
        self.readout.view.pip_available = available;
        self.readout.view.pip = on;
        if let Some(ref gfx) = self.gfx {
            gfx.window.request_redraw();
        }
    }

    /// Attaches a Document Picture-in-Picture surface on wasm32.
    #[cfg(target_arch = "wasm32")]
    pub fn attach_pip_surface(&mut self, surface: wgpu::Surface<'static>, size: (u32, u32)) {
        if let Some(ref mut gfx) = self.gfx {
            gfx.engine
                .deck
                .grant_watchdog_grace(std::time::Duration::from_secs(3));
            for slot in 0..gfx.engine.deck.slot_count() {
                gfx.engine
                    .deck
                    .set_overloaded(karakuri_engine::DeckSlot(slot as u8), false);
            }
            let caps = surface.get_capabilities(&gfx.gpu.adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let view_formats = if format != gfx.picture_format {
                vec![gfx.picture_format]
            } else {
                vec![]
            };
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: size.0.max(1),
                height: size.1.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: caps.alpha_modes[0],
                view_formats,
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&gfx.gpu.device, &config);
            gfx.pip_sink = Some(karakuri_engine::WindowSink::new(surface, config));
            self.readout.view.pip = true;
            gfx.window.request_redraw();
        }
    }

    /// Detaches the PiP surface on wasm32 and turns off the PiP chip.
    #[cfg(target_arch = "wasm32")]
    pub fn detach_pip_surface(&mut self) {
        if let Some(ref mut gfx) = self.gfx {
            gfx.engine
                .deck
                .grant_watchdog_grace(std::time::Duration::from_secs(3));
            for slot in 0..gfx.engine.deck.slot_count() {
                gfx.engine
                    .deck
                    .set_overloaded(karakuri_engine::DeckSlot(slot as u8), false);
            }
            gfx.pip_sink = None;
            gfx.window.request_redraw();
        }
        self.readout.view.pip = false;
    }

    /// Resizes the PiP surface on wasm32 when the floating window is resized.
    #[cfg(target_arch = "wasm32")]
    pub fn resize_pip_surface(&mut self, size: (u32, u32)) {
        if let Some(ref mut gfx) = self.gfx {
            gfx.engine
                .deck
                .grant_watchdog_grace(std::time::Duration::from_secs(2));
            if let Some(sink) = gfx.pip_sink.as_mut() {
                sink.resize(&gfx.gpu.device, size.0.max(1), size.1.max(1));
                gfx.window.request_redraw();
            }
        }
    }

    /// Attaches an offscreen clean visual recorder surface on wasm32 for video loop capture without UI overlays.
    #[cfg(target_arch = "wasm32")]
    pub fn attach_recorder_surface(&mut self, surface: wgpu::Surface<'static>, size: (u32, u32)) {
        if let Some(ref mut gfx) = self.gfx {
            let caps = surface.get_capabilities(&gfx.gpu.adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let view_formats = if format != gfx.picture_format {
                vec![gfx.picture_format]
            } else {
                vec![]
            };
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: size.0.max(1),
                height: size.1.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: caps.alpha_modes[0],
                view_formats,
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&gfx.gpu.device, &config);
            gfx.recorder_sink = Some(karakuri_engine::WindowSink::new(surface, config));
            gfx.window.request_redraw();
        }
    }

    /// Detaches the offscreen recorder surface on wasm32 after video capture completes.
    #[cfg(target_arch = "wasm32")]
    pub fn detach_recorder_surface(&mut self) {
        if let Some(ref mut gfx) = self.gfx {
            gfx.recorder_sink = None;
            gfx.window.request_redraw();
        }
    }
}
