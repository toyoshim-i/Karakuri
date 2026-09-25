//! Single-frame composition and presentation coordination.
//!
//! Orchestrates the frame lifecycle across sinks: acquires render targets,
//! evaluates committed simulation steps and look settings, executes deck
//! rendering, and presents to active sinks.

use crate::deck::Deck;
#[cfg(test)]
use crate::deck::DeckSlot;
use crate::gpu::Gpu;
use crate::present::{Present, TonemapOp};

/// Output color grading and tone-mapping configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Active tonemapping operator.
    pub op: TonemapOp,
    /// Exposure adjustment applied during tonemapping.
    pub exposure: f32,
    /// White point parameter for Reinhard tonemapping.
    pub white_point: f32,
}

/// Reason a frame was not rendered to a specific sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skip {
    /// Transient acquisition failure (e.g., surface reconfiguration or minimized
    /// window).
    Transient,
    /// Persistent or diagnosed surface fault with a descriptive message.
    Fault(String),
}

/// Frame execution parameters determined by the commit closure.
pub struct Committed {
    /// Simulation steps to advance this frame.
    pub steps: u8,
    /// Output look applied to this frame.
    pub look: Look,
}

/// Destination sink for composited frames.
pub trait Sink {
    /// Acquires the render target for the upcoming frame.
    fn acquire(&mut self, gpu: &Gpu) -> Result<(), Skip>;

    /// Returns a view to the acquired render target texture.
    fn view(&self) -> &wgpu::TextureView;

    /// Returns the target texture dimensions in pixels `(width, height)`.
    fn size(&self) -> (u32, u32);

    /// Records post-draw commands into the frame command encoder.
    fn after_draw(&mut self, _encoder: &mut wgpu::CommandEncoder) {}

    /// Presents or finalizes the drawn frame after submission.
    fn present(&mut self, gpu: &Gpu) -> Result<(), String>;
}

/// Summary outcome of a composed frame across all candidate sinks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// Number of sinks successfully rendered and presented.
    pub reached: usize,
    /// Number of sinks that skipped or failed target acquisition.
    pub missed: usize,
}

/// Composes one frame across the provided sinks.
///
/// Acquires render targets from all sinks, invokes the commit closure, renders
/// the deck, draws to acquired sinks, executes optional final commands, and
/// presents results.
pub fn compose(
    gpu: &Gpu,
    deck: &mut Deck,
    present: &Present,
    sinks: &mut [&mut dyn Sink],
    refused: &mut dyn FnMut(usize, Skip),
    commit: impl FnOnce(&mut Deck) -> Committed,
    finally: impl FnOnce(&mut wgpu::CommandEncoder),
) -> Result<Outcome, String> {
    let mut reached = 0;
    for at in 0..sinks.len() {
        match sinks[at].acquire(gpu) {
            Ok(()) => {
                sinks[reached..=at].rotate_right(1);
                reached += 1;
            }
            Err(skip) => refused(at, skip),
        }
    }
    let missed = sinks.len() - reached;
    let (drawn, _) = sinks.split_at_mut(reached);

    let Committed { steps, look } = commit(deck);

    present.set_tonemap(&gpu.queue, look.op, look.exposure, look.white_point);
    // Synchronize session clock uniform and ops_per_fragment with master chain (ADR-0340).
    present.set_chain_clock(&gpu.queue, deck.chain_clock(steps));
    deck.set_chain_ops_per_fragment(present.chain_ops_per_fragment());

    {
        let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
        frame.render(present.mix_target(), present.size(), steps);
        present.draw_chain(frame.encoder());
        for sink in drawn.iter_mut() {
            present.draw(frame.encoder(), sink.view(), sink.size());
            sink.after_draw(frame.encoder());
        }
        finally(frame.encoder());
        frame.finish();
    }

    let mut failed = None;
    for sink in drawn.iter_mut() {
        if let Err(e) = sink.present(gpu) {
            failed.get_or_insert(e);
        }
    }
    match failed {
        Some(e) => Err(e),
        None => Ok(Outcome { reached, missed }),
    }
}

/// Standard presentation sink targeting a display window surface.
pub struct WindowSink {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    current: Option<(wgpu::SurfaceTexture, wgpu::TextureView)>,
    faulted: bool,
}

impl WindowSink {
    /// Creates a new WindowSink for the given surface and configuration.
    pub fn new(surface: wgpu::Surface<'static>, config: wgpu::SurfaceConfiguration) -> WindowSink {
        WindowSink {
            surface,
            config,
            current: None,
            faulted: false,
        }
    }

    /// Updates window dimensions and reconfigures the surface.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(device, &self.config);
    }
}

impl Sink for WindowSink {
    fn acquire(&mut self, gpu: &Gpu) -> Result<(), Skip> {
        let texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&gpu.device, &self.config);
                return Err(Skip::Transient);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Err(Skip::Transient)
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                if self.faulted {
                    return Err(Skip::Transient);
                }
                self.faulted = true;
                return Err(Skip::Fault(
                    "acquiring a texture raised a validation error \
                     — the window has stopped drawing"
                        .into(),
                ));
            }
        };
        self.faulted = false;
        let view = texture.texture.create_view(&Default::default());
        self.current = Some((texture, view));
        Ok(())
    }

    fn view(&self) -> &wgpu::TextureView {
        &self
            .current
            .as_ref()
            .expect("`view` before `acquire` — see the Sink contract")
            .1
    }

    fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    fn present(&mut self, gpu: &Gpu) -> Result<(), String> {
        if let Some((texture, _view)) = self.current.take() {
            gpu.queue.present(texture);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
