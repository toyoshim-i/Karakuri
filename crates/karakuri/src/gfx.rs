//! Graphics context, surfaces, and projector window.

use std::sync::Arc;

use karakuri_console::{egui_wgpu, egui_winit};
use karakuri_engine::{Gpu, WindowSink};
use karakuri_environment::{audio, midi};
use karakuri_operation::Operation;
use winit::window::Window;

use crate::engine_bridge::Engine;

// ---------------------------------------------------------------------------
// The window
// ---------------------------------------------------------------------------

/// Decision on how to handle an unacquired surface frame.
///
/// Avoids silent returns so the event loop does not freeze rendering indefinitely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Missed {
    /// The swapchain needs remaking: reconfigure, then ask for another frame.
    Remake,
    /// Ordinary jitter. Ask again.
    Again,
    /// Nobody can see the window. Doing nothing is right — it is the OS that says
    /// when it is back, and asking for frames meanwhile is a spin.
    Idle,
    /// Not self-correcting, and not something a retry mends. Say so.
    Fault,
}

/// `None` where a texture was handed over; a decision for every other case.
pub(crate) fn missed(outcome: &wgpu::CurrentSurfaceTexture) -> Option<Missed> {
    match outcome {
        // `Suboptimal` draws correctly and asks to be reconfigured for
        // performance, which the next resize does anyway.
        wgpu::CurrentSurfaceTexture::Success(_) | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
            None
        }
        wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
            Some(Missed::Remake)
        }
        wgpu::CurrentSurfaceTexture::Timeout => Some(Missed::Again),
        wgpu::CurrentSurfaceTexture::Occluded => Some(Missed::Idle),
        wgpu::CurrentSurfaceTexture::Validation => Some(Missed::Fault),
    }
}

/// Projector window sink for secondary display output.
///
/// Rendered and letterboxed alongside the main canvas (ADR-0247) without blocking
/// other sinks (ADR-0171). Window size dictates render size (ADR-0246, ADR-0358).
pub(crate) struct Projector {
    /// Held because the surface borrows it for `'static` and because the window is
    /// what a close event and a resize arrive on. Read by [`App::window_event`],
    /// which routes by [`WindowId`].
    pub(crate) window: Arc<Window>,
    pub(crate) sink: WindowSink,
    /// Inner size in physical pixels, cached to avoid per-frame platform queries.
    pub(crate) size: (u32, u32),
}

pub(crate) struct Gfx {
    pub(crate) window: Arc<Window>,
    /// The projector window, while it is open. `None` is the ordinary state — a run
    /// with one output, the picture — and it is the state every run starts in. See
    /// [`Projector`].
    pub(crate) projector: Option<Projector>,
    /// Out-of-process video output plugin sink (e.g. Syphon on macOS).
    pub(crate) plugin: Option<crate::bridge::PluginSink>,
    pub(crate) gpu: Gpu,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) config: wgpu::SurfaceConfiguration,
    /// Target texture format for rendering pictures, selected as the first supported sRGB format on the surface (P-0064, ADR-0162).
    pub(crate) picture_format: wgpu::TextureFormat,
    pub(crate) egui: egui_winit::State,
    pub(crate) renderer: egui_wgpu::Renderer,
    /// The engine instance managing GPU pipelines, passes, and deck state.
    pub(crate) engine: Engine,
    /// Audio input stream handle, or `None` if audio input was not opened.
    pub(crate) audio: Option<audio::Audio>,
    /// Connected MIDI control surface handle, or `None` if none was opened.
    pub(crate) midi: Option<midi::Surface>,
    /// Pre-allocated buffer for draining hand-performed operations from MIDI without allocating per frame.
    pub(crate) performed_by_hand: Vec<Operation>,
    /// Display frame budget in milliseconds, measured on window initialization.
    pub(crate) budget_ms: Option<f32>,
    /// Display names of currently loaded material per slot for mixer readout (P-0094).
    pub(crate) material: Vec<String>,
    /// Base name of the launch material, used when slot aim has no explicit set id (ADR-0338).
    pub(crate) launch: String,
    /// Presets root directory if available (ADR-0227, ADR-0338).
    pub(crate) presets: Option<std::path::PathBuf>,
    /// Where the plugins root is, copied from [`App::plugins`] when the device was made.
    #[allow(dead_code)]
    pub(crate) plugins: Option<std::path::PathBuf>,
    /// Out-of-process output plugins discovered in the plugins directory.
    pub(crate) discovered_plugins: Vec<karakuri_environment::output_plugin::DiscoveredPlugin>,
    /// Self-reported display name of the primary plugin (e.g. "Spout"), interned as static.
    pub(crate) plugin_name: Option<&'static str>,
    /// Configured library directory path, propagated from application options.
    pub(crate) store: std::path::PathBuf,
}
