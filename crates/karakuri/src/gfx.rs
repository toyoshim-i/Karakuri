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
/// **Every outcome gets a decision, because ignoring one is invisible.** This
/// loop waits for events rather than spinning, so a `return` that neither
/// reconfigures nor asks for another frame is a window that stops drawing and
/// never starts again — and there is nothing on screen or on stdout to say
/// why. `karakuri-cli`'s window sink carries the same table with the same
/// argument, and says that returning silently is what left it with a frozen
/// window once already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Missed {
    /// The swapchain needs remaking: reconfigure, then ask for another frame.
    Remake,
    /// Ordinary jitter. Ask again.
    Again,
    /// Nobody can see the window. Doing nothing is right — it is the OS that
    /// says when it is back, and asking for frames meanwhile is a spin.
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

/// **The projector window: the second `Sink` this repository owns.**
///
/// A second `winit` window with a surface of its own and a
/// [`WindowSink`] over it, opened on `RouteFrame { output: Projector(0), on:
/// true }` and dropped on the same operation with `on: false`. It takes the
/// composited frame beside the picture — `Present::draw` letterboxes the one
/// canvas into each target, so a window and a picture of different shapes need
/// one `Present` between them rather than one each
/// ([ADR-0247](../../../docs/adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md)).
///
/// **It never delays anything.** `compose` asks every sink to acquire before
/// the frame is committed, draws into the ones that answered, and presents all
/// of them before returning any error
/// ([ADR-0171](../../../docs/adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md))
/// — so a projector that is resizing, minimised or wedged loses its own frame
/// and costs the picture nothing, which is what `docs/plugins.md` promises of
/// a sink and what the console page's chip says.
///
/// **Its size is the window the operating system gave it**, which is
/// [ADR-0246](../../../docs/adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)'s
/// *the destination window decides*: `WindowEvent::Resized` on this window
/// reconfigures the swapchain and the next frame's [`render_size`] follows it.
/// **Fullscreen on a chosen display is not built and is not one line** — the
/// operator makes this window fullscreen the way they make any window
/// fullscreen, which is the console page's *fullscreen is just what an
/// application does*; naming *which* display wants a list of monitors this
/// program does not read, and a chip that carried one would carry a label that
/// changes when the cable does.
pub(crate) struct Projector {
    /// Held because the surface borrows it for `'static` and because the
    /// window is what a close event and a resize arrive on. Read by
    /// [`App::window_event`], which routes by [`WindowId`].
    pub(crate) window: Arc<Window>,
    pub(crate) sink: WindowSink,
    /// Its inner size in physical pixels — **this output's size**, and the
    /// second term [`render_size`] takes a maximum over.
    ///
    /// Kept here rather than asked of the window every frame: `Window::
    /// inner_size` is a platform call on the frame path, and the answer only
    /// changes on an event this program already handles.
    pub(crate) size: (u32, u32),
}

pub(crate) struct Gfx {
    pub(crate) window: Arc<Window>,
    /// **The projector window, while it is open.** `None` is the ordinary
    /// state — a run with one output, the picture — and it is the state every
    /// run starts in. See [`Projector`].
    pub(crate) projector: Option<Projector>,
    pub(crate) gpu: Gpu,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) egui: egui_winit::State,
    pub(crate) renderer: egui_wgpu::Renderer,
    /// **The engine, on the same device as the panel.** It lives beside the
    /// renderer rather than beside the model because everything in it takes a
    /// device: that is the seam `karakuri-console` keeps, and this is the side
    /// of it that is allowed one.
    pub(crate) engine: Engine,
    /// **The room this window is listening to**, or `None` for a machine with
    /// no input — see [`listening`], where all three of that decision's cases
    /// are argued.
    ///
    /// It is here beside the engine rather than on [`App`] because the two
    /// halves of what it is for are both here: the deck's signal bus is what a
    /// measurement is written into, and the *output lag* a beat correction
    /// leads by is this display's frame queue. A window remade is a display
    /// remade, and the input is re-opened with it.
    pub(crate) audio: Option<audio::Audio>,
    /// **The control surface this window opened**, or `None` for a run with
    /// nothing plugged in — see [`surfaced`], where the three cases are
    /// argued, and [`App::mapped`], which is the drain.
    ///
    /// **Beside `audio` and for its reason.** Both are doors this window opens
    /// at startup and neither is a flag; a window remade re-opens both, which
    /// is right for the microphone and harmless for the port.
    pub(crate) midi: Option<midi::Surface>,
    /// Scratch for [`midi::Surface::take`], owned so the drain allocates
    /// nothing on a frame — `karakuri-cli` keeps the same buffer for the same
    /// reason. Sized once at construction; a frame's worth of a surface's
    /// fastest gesture is single figures.
    pub(crate) performed_by_hand: Vec<Operation>,
    /// **What a frame has to fit in on this window**, read from the display
    /// once when the window opened — see [`budget_ms`].
    pub(crate) budget_ms: Option<f32>,
    /// **What the mixer strip calls what each slot is playing**, in slot
    /// order — see [`Sources::material`]. Kept rather than recomputed because
    /// a name is a string and the frame path is budgeted.
    ///
    /// **One per slot rather than one for the deck**, and the difference only
    /// began to matter when a load did. Both slots open on the pair this
    /// program was launched with, so one name was every slot's name and could
    /// not become wrong; a load moves one slot's material and leaves the other
    /// where it was, and a single name would then have both strips reading the
    /// launch pair with the picture showing something else. That is a readout
    /// that is wrong and silent, which is the one thing P-0094 refuses.
    ///
    /// Rewritten where the slot is: [`played`], on the press, which is also
    /// where the store is read. Nothing on the frame path touches it.
    pub(crate) material: Vec<String>,
    /// **What a slot that has never been loaded is playing**, which is the pair
    /// this run was launched with — [`Sources::material`], said once because
    /// every slot opens on it.
    ///
    /// **It is the base a procedure load reads**, and it is the one case
    /// `watch::Aim::set` cannot answer: a slot running a Set is filed under
    /// that id and a slot running the launch pair is filed under nothing, so
    /// the strip's `<base> + <kir>` needs this where the aim says `None`
    /// (ADR-0338). It never moves — a load writes [`Gfx::material`], which is
    /// what the strip reads.
    pub(crate) launch: String,
    /// **Where the presets root is**, copied from [`App::presets`] when the
    /// device was made, or `None` on a machine with no library.
    ///
    /// Here for [`Gfx::store`]'s reason word for word: a procedure load reaches
    /// a disk on a press and is handed nothing but a device, and the shipped
    /// tier is one of the two places a library row's file can be
    /// (ADR-0227, ADR-0338).
    pub(crate) presets: Option<std::path::PathBuf>,
    /// **Where the library is**, copied from [`App::store`] when the device
    /// was made.
    ///
    /// It is on both because the two readers are on both sides of the window:
    /// `resumed` lists the bay before there is a `Gfx` at all, and [`played`]
    /// and `performed` reach a disk on a key press and are handed nothing but
    /// this. A `Path::new(STORE)` at each of those four call sites is what
    /// this replaces, and the reason it can no longer be one is the whole of
    /// the change — the directory is a thing the operator said, so it has to
    /// be carried from where they said it.
    pub(crate) store: std::path::PathBuf,
}
