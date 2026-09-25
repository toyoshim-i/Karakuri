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

/// What to do about a frame that could not be acquired.
///
/// Every outcome gets a decision, because ignoring one is invisible. This loop
/// waits for events rather than spinning, so a `return` that neither
/// reconfigures nor asks for another frame is a window that stops drawing and
/// never starts again — and there is nothing on screen or on stdout to say why.
/// `karakuri-cli`'s window sink carries the same table with the same argument,
/// and says that returning silently is what left it with a frozen window once
/// already.
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

/// The projector window: the second `Sink` this repository owns.
///
/// A second `winit` window with a surface of its own and a [`WindowSink`] over
/// it, opened on `RouteFrame { output: Projector(0), on: true }` and dropped on
/// the same operation with `on: false`. It takes the composited frame beside
/// the picture — `Present::draw` letterboxes the one canvas into each target,
/// so a window and a picture of different shapes need one `Present` between
/// them rather than one each
/// ([ADR-0247](../../../docs/adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md)).
///
/// It never delays anything. `compose` asks every sink to acquire before the
/// frame is committed, draws into the ones that answered, and presents all of
/// them before returning any error
/// ([ADR-0171](../../../docs/adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md))
/// — so a projector that is resizing, minimised or wedged loses its own frame
/// and costs the picture nothing, which is what `docs/plugins.md` promises of a
/// sink and what the console page's chip says.
///
/// Its size is the window the operating system gave it, which is
/// [ADR-0246](../../../docs/adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)'s
/// *the destination window decides*: `WindowEvent::Resized` on this window
/// reconfigures the swapchain and the next frame's [`render_size`] follows it.
/// No operation names a display. The window is made fullscreen by the
/// operating system's own gesture, on the display it is on when the gesture is
/// made, and a picture for another application goes out through a plugin sink
/// (`docs/plugins.md`) rather than through a window
/// ([ADR-0358](../../../docs/adr/0358-the-projector-is-fullscreened-by-the-operating-system-on-the-display-it-is-on-and-another-application-is-reached-through-a-plugin.md)).
pub(crate) struct Projector {
    /// Held because the surface borrows it for `'static` and because the window is
    /// what a close event and a resize arrive on. Read by [`App::window_event`],
    /// which routes by [`WindowId`].
    pub(crate) window: Arc<Window>,
    pub(crate) sink: WindowSink,
    /// Its inner size in physical pixels — this output's size, and the second term
    /// [`render_size`] takes a maximum over.
    ///
    /// Kept here rather than asked of the window every frame: `Window:: inner_size`
    /// is a platform call on the frame path, and the answer only changes on an
    /// event this program already handles.
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
    /// Where the presets root is, copied from [`App::presets`] when the device was
    /// made, or `None` on a machine with no library.
    ///
    /// Here for [`Gfx::store`]'s reason word for word: a procedure load reaches a
    /// disk on a press and is handed nothing but a device, and the shipped tier is
    /// one of the two places a library row's file can be (ADR-0227, ADR-0338).
    pub(crate) presets: Option<std::path::PathBuf>,
    /// Where the plugins root is, copied from [`App::plugins`] when the device was made.
    #[allow(dead_code)]
    pub(crate) plugins: Option<std::path::PathBuf>,
    /// Out-of-process output plugins discovered in the plugins directory.
    pub(crate) discovered_plugins: Vec<karakuri_environment::output_plugin::DiscoveredPlugin>,
    /// Self-reported display name of the primary plugin (e.g. "Spout"), interned as static.
    pub(crate) plugin_name: Option<&'static str>,
    /// Where the library is, copied from [`App::store`] when the device was made.
    ///
    /// It is on both because the two readers are on both sides of the window:
    /// `resumed` lists the bay before there is a `Gfx` at all, and [`played`] and
    /// `performed` reach a disk on a key press and are handed nothing but this. A
    /// `Path::new(STORE)` at each of those four call sites is what this replaces,
    /// and the reason it can no longer be one is the whole of the change — the
    /// directory is a thing the operator said, so it has to be carried from where
    /// they said it.
    pub(crate) store: std::path::PathBuf,
}
