//! The L5 node, nested: `[Texture] -> Texture`.
//!
//! **The same node the deck mixes on**, one shader and one set of per-input
//! controls — `docs/ir-spec.md`, "L5". What differs between the two roles is
//! only whether a surface is wired to it: the top-level one is what an operator
//! rides, and this one folds several renderers into the single `Texture` a Set
//! has to produce.
//!
//! # Compositing is not overdraw with extra memory
//!
//! Placing this node is what turns several renderers from **overdraw** into
//! **compositing**, and the two are different operations rather than one with a
//! dial:
//!
//! - **Overdraw** runs the renderers in order over *one* attachment — the first
//!   clears, the rest load — so each meets what is already there through its own
//!   blend state. One target, whatever the count.
//! - **Compositing** gives each renderer a cleared target of its own and folds
//!   them here, through a gain, an opacity, a blend mode and a mask *per input*.
//!   One target per input, at 7.03 MB each for 1280x720.
//!
//! **They do not agree, and that is why the node is asked for explicitly.** For
//! renderers under `blend additive` they nearly do — addition is addition, and
//! the fold order is the draw order either way. Under `blend weighted` they do
//! not: a weighted renderer resolves `over` onto whatever its target holds, so
//! overdraw composites it against the picture so far while this composites it
//! against a clear and then mixes the result. Neither is wrong; they are two
//! things, and the memory is the smaller half of the difference.
//!
//! # What it does not have
//!
//! No residency, no priming, no preview, no metering. Those are properties of a
//! Set being *played* and live beside the top-level mix in [`crate::deck`]; an
//! [`crate::mix::Input`] is the whole of what an edge into an L5 carries.

use crate::mix::{Composite, Input};
use crate::present::Present;

/// A merge node: one target per input, and the mix that folds them.
pub(crate) struct Merge {
    /// One per input, in draw order. Frame-sized, so a resize reallocates.
    targets: Vec<wgpu::Texture>,
    views: Vec<wgpu::TextureView>,
    composite: Composite,
}

impl Merge {
    /// **`inputs` is how many renderers feed it**, and a merge of one is legal
    /// and exact: `0.0 + 1.0 * src` hands the material on unchanged, which is
    /// what makes a Set that grows a second renderer later not change what the
    /// first one looked like.
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

    /// Where input `i` draws. **The renderer clears it**, which is what makes
    /// this compositing rather than overdraw: every input starts from nothing
    /// and meets the others only here.
    pub(crate) fn target(&self, i: usize) -> &wgpu::TextureView {
        &self.views[i]
    }

    /// Reallocation, so never from the render thread mid-frame — the same shape
    /// as [`crate::node::Renderer::resize`] and every other owner of a
    /// frame-sized target in this engine.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (targets, views) = allocate(device, self.targets.len(), width, height);
        self.targets = targets;
        self.views = views;
        let refs: Vec<&wgpu::TextureView> = self.views.iter().collect();
        self.composite.rebind(device, &refs);
    }

    /// This frame's edges, written where a Set writes every other uniform.
    pub(crate) fn write_uniform(&self, queue: &wgpu::Queue, inputs: &[Input]) {
        self.composite.write_uniform(queue, inputs);
    }

    /// Fold the inputs into `target`, after every one of them has drawn.
    pub(crate) fn record(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.composite.record(encoder, target);
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
                // Always the linear HDR format, never the surface's — the
                // pipeline is linear and HDR end to end and the one encode
                // happens in the present pass.
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
