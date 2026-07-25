//! What L5 consumes.
//!
//! A `Set` is the only implementation in V1. The interface exists now so that a
//! second implementation is an addition rather than a change — the same move as
//! declaring `blend additive` before there is a second blend mode.

/// A source of one frame of linear HDR colour.
///
/// Implementations render into an `Rgba16Float` target. sRGB encoding happens
/// once, at final output, and values above 1.0 are expected — they are what
/// feeds bloom.
pub trait VideoSource {
    /// Advance by `steps` simulation steps and render.
    ///
    /// `steps` comes from a `tick` record, never from a measurement: emitted
    /// from real time when live, read back verbatim on replay. That is what
    /// keeps substepping compatible with deterministic reproduction.
    fn render(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, steps: u8);

    /// The format every implementation renders into.
    fn format(&self) -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba16Float
    }
}
