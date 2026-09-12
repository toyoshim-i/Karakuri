//! Nested L5 compositing node: `[Texture] -> Texture`.
//!
//! Composites multiple renderer outputs into a single texture within a Set,
//! applying gain, opacity, blend modes, and masks per input.

use crate::mix::{Composite, Input};
use crate::present::Present;

/// A merge node holding per-input render targets and a composite pass.
pub(crate) struct Merge {
    /// Render targets for each input texture in draw order.
    targets: Vec<wgpu::Texture>,
    views: Vec<wgpu::TextureView>,
    composite: Composite,
}

impl Merge {
    /// Constructs a merge node with `inputs` intermediate render targets.
    pub(crate) fn build(device: &wgpu::Device, inputs: usize, width: u32, height: u32) -> Merge {
        let (targets, views) = allocate(device, inputs, width, height);
        let refs: Vec<&wgpu::TextureView> = views.iter().collect();
        let composite = Composite::new(device, &refs);
        Merge {
            targets,
            views,
            composite,
        }
    }

    /// Returns the render target texture view for input `i`.
    pub(crate) fn target(&self, i: usize) -> &wgpu::TextureView {
        &self.views[i]
    }

    /// Reallocates intermediate render targets to the new dimensions.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (targets, views) = allocate(device, self.targets.len(), width, height);
        self.targets = targets;
        self.views = views;
        let refs: Vec<&wgpu::TextureView> = self.views.iter().collect();
        self.composite.rebind(device, &refs);
    }

    /// Writes input blend parameters to the composite uniform buffer.
    pub(crate) fn write_uniform(&self, queue: &wgpu::Queue, inputs: &[Input]) {
        self.composite.write_uniform(queue, inputs, 1.0);
    }

    /// Records the composite pass folding all inputs into `target`.
    pub(crate) fn record(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.composite.record(encoder, target);
    }
}

impl crate::pass::RenderPassNode for Merge {
    fn record(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.record(encoder, target);
    }
}

fn allocate(
    device: &wgpu::Device,
    inputs: usize,
    width: u32,
    height: u32,
) -> (Vec<wgpu::Texture>, Vec<wgpu::TextureView>) {
    let targets: Vec<wgpu::Texture> = (0..inputs)
        .map(|i| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("merge input {i}")),
                size: wgpu::Extent3d {
                    width: width.max(1),
                    height: height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: Present::HDR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        })
        .collect();
    let views = targets
        .iter()
        .map(|t| t.create_view(&Default::default()))
        .collect();
    (targets, views)
}
