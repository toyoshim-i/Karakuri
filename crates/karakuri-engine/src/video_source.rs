//! Trait for rendering linear HDR video frames.

/// Produces linear HDR color frames into an `Rgba16Float` render target.
pub trait VideoSource {
    /// Advances simulation state by `steps` and records rendering commands into `encoder`.
    fn render(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, steps: u8);

    /// Commits staged state transitions (such as simulation clock advancement
    /// and ping-pong parity) after command buffer submission.
    fn commit(&mut self) {}

    /// Discards staged state transitions when an open frame is abandoned without submission.
    fn discard(&mut self) {}

    /// Returns the render target texture format (defaults to `Rgba16Float`).
    fn format(&self) -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba16Float
    }
}
