//! Compositing for L5 mix and merge passes.
//!
//! Combines multiple texture inputs into a single output texture according to
//! per-edge properties (gain, opacity, blend mode, and mask).
//!
//! Inputs that do not contribute (for example, silenced inputs or masked-out regions)
//! are skipped to avoid propagating NaNs or infinities from HDR targets.

use crate::deck::{Blend, Mask, MAX_SLOTS};
use crate::present::Present;

/// Byte size of the `Mix` uniform buffer: eight vec4s for input columns and one scalar for master out.
const UNIFORM_SIZE: u64 = 144;

/// Byte offset where the master output level is stored in the uniform buffer.
const MASTER_OUT_AT: usize = 128;

/// Configuration for a single input edge into an L5 compositing pass.
#[derive(Debug, Clone, Copy)]
pub struct Input {
    /// Input gain factor.
    pub gain: f32,
    /// Blend opacity in the range [0, 1].
    pub opacity: f32,
    pub blend: Blend,
    /// Spatial mask configuration for this input.
    pub mask: Mask,
    /// Whether this input contributes to the current frame.
    pub live: bool,
}

impl Default for Input {
    fn default() -> Input {
        Input {
            gain: 1.0,
            opacity: 1.0,
            blend: Blend::Add,
            mask: Mask::default(),
            live: true,
        }
    }
}

impl Input {
    /// Returns an input configured with unity gain and opacity under additive blend.
    pub fn unity() -> Input {
        Input::default()
    }

    /// Returns whether this edge actively contributes to the composite output.
    fn contributes(&self) -> bool {
        self.live && !self.blend.silent_at(self.gain, self.opacity) && !self.mask.hides_everything()
    }
}

/// Sets the edge at index `at` to live and all other edges to inactive in place.
///
/// Returns `true` if `at` is valid, or `false` if `at` is out of bounds (leaving edges unchanged).
pub fn select(edges: &mut [Input], at: usize) -> bool {
    if at >= edges.len() {
        return false;
    }
    for (i, edge) in edges.iter_mut().enumerate() {
        edge.live = i == at;
    }
    true
}

/// Fullscreen render pass for compositing up to [`MAX_SLOTS`] input textures.
pub(crate) struct Composite {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Composite {
    pub(crate) fn new(device: &wgpu::Device, views: &[&wgpu::TextureView]) -> Composite {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/composite.wgsl").into()),
        });

        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        for slot in 0..MAX_SLOTS {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1 + slot as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    // Non-filterable: texels are loaded directly per fragment without sampling.
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("composite"),
            entries: &entries,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composite"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("composite"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    // Linear HDR output with shader-level compositing order.
                    format: Present::HDR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("composite mix"),
            size: UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = Composite::bind(device, &layout, &uniform, views);
        Composite {
            pipeline,
            layout,
            uniform,
            bind_group,
        }
    }

    /// Binds textures to pipeline slots, padding spare bindings with the first view.
    fn bind(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniform: &wgpu::Buffer,
        views: &[&wgpu::TextureView],
    ) -> wgpu::BindGroup {
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }];
        for slot in 0..MAX_SLOTS {
            entries.push(wgpu::BindGroupEntry {
                binding: 1 + slot as u32,
                resource: wgpu::BindingResource::TextureView(views[slot.min(views.len() - 1)]),
            });
        }
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite"),
            layout,
            entries: &entries,
        })
    }

    pub(crate) fn rebind(&mut self, device: &wgpu::Device, views: &[&wgpu::TextureView]) {
        self.bind_group = Composite::bind(device, &self.layout, &self.uniform, views);
    }

    /// Writes uniform data for input edges and master output level.
    pub(crate) fn write_uniform(&self, queue: &wgpu::Queue, inputs: &[Input], out: f32) {
        let mut bytes = [0u8; UNIFORM_SIZE as usize];
        for (i, input) in inputs.iter().enumerate().take(MAX_SLOTS) {
            let fields = [
                input.gain.to_le_bytes(),
                input.opacity.to_le_bytes(),
                input.blend.index().to_le_bytes(),
                u32::from(input.contributes()).to_le_bytes(),
                input.mask.kind().index().to_le_bytes(),
                input.mask.angle().to_le_bytes(),
                input.mask.position().to_le_bytes(),
                input.mask.softness().to_le_bytes(),
            ];
            for (field, value) in fields.iter().enumerate() {
                let at = field * 16 + i * 4;
                bytes[at..at + 4].copy_from_slice(value);
            }
        }
        bytes[MASTER_OUT_AT..MASTER_OUT_AT + 4].copy_from_slice(&out.to_le_bytes());
        queue.write_buffer(&self.uniform, 0, &bytes);
    }

    /// Records the compositing pass into the command encoder.
    pub(crate) fn record(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        crate::pass::record_fullscreen_pass(
            encoder,
            Some("composite"),
            target,
            &self.pipeline,
            &[&self.bind_group],
        );
    }
}

impl crate::pass::RenderPassNode for Composite {
    fn record(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.record(encoder, target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selecting_one_edge_leaves_exactly_one_live() {
        let mut edges = vec![Input::unity(); 3];
        for at in 0..edges.len() {
            assert!(select(&mut edges, at));
            let live: Vec<usize> = edges
                .iter()
                .enumerate()
                .filter(|(_, e)| e.live)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(live, vec![at], "selecting {at} left {live:?} live");
        }
    }

    #[test]
    fn a_selection_moves_the_flag_and_nothing_else() {
        let quiet = Input {
            gain: 0.25,
            opacity: 0.5,
            ..Input::unity()
        };
        let mut edges = vec![quiet, Input::unity()];
        assert!(select(&mut edges, 1));
        assert!(!edges[0].live);
        assert_eq!((edges[0].gain, edges[0].opacity), (0.25, 0.5));
        assert!(select(&mut edges, 0));
        assert!(edges[0].live);
        assert_eq!((edges[0].gain, edges[0].opacity), (0.25, 0.5));
    }

    #[test]
    fn selecting_an_edge_that_is_not_there_leaves_every_edge_alone() {
        let mut edges = vec![Input::unity(); 2];
        assert!(!select(&mut edges, 2));
        assert!(
            edges.iter().all(|e| e.live),
            "an unhonourable selection silenced the mix"
        );
        // And the boundary either side of it.
        assert!(select(&mut edges, 1));
        assert!(!select(&mut edges, usize::MAX));
        assert!(edges[1].live);
        // An empty list has nothing to select and says so rather than panicking.
        assert!(!select(&mut [], 0));
    }
}
