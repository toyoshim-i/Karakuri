//! The L5 mix: several textures folded into one, each through its own edge.
//!
//! **One node kind, one shader, two roles** — `docs/ir-spec.md`, "L5". The
//! top-level mix is what an operator sees and rides, with a surface wired to
//! every input; a [`crate::node::Merge`] is the same thing inside a Set,
//! folding several renderers into the one `Texture` a Set has to produce.
//! Nothing about the compositing differs between them, and this module is the
//! part that is the same.
//!
//! # What belongs to the edge, and what belongs to the performance
//!
//! [`Input`] is the whole of an edge into an L5: `gain`, `opacity`, `blend`,
//! `mask`, and whether the input contributes at all. Those travel with the edge
//! wherever it is nested.
//!
//! `residency`, priming, hot swap, budget governance, `transport`, `preview` and
//! metering are properties of **a Set being played**. They sit beside the
//! top-level mix in [`crate::deck`] because that is where a performance happens,
//! not because they belong to L5 — which is why `live` here is a plain flag
//! rather than a `Residency`: the deck decides what silence and residency mean
//! and hands down the answer, and a nested merge has neither question.
//!
//! # Why a skip rather than a blend at zero
//!
//! An input that does not contribute is skipped. `0.0 * x` is only zero for
//! finite `x`, and an HDR target is allowed to hold an infinity or a NaN — a
//! generated L4 that divides by zero is a compiling procedure, not a broken
//! build. Blending that at zero would put a NaN in every channel, so a fader
//! pulled to silence would take the whole mix down with it. See
//! `shaders/composite.wgsl`, which says the same thing from the other side.

use crate::deck::{Blend, Mask, MAX_SLOTS};
use crate::present::Present;

/// Byte size of the `Mix` uniform: eight `vec4`s, one field per column and one
/// input per lane.
const UNIFORM_SIZE: u64 = 128;

/// **One edge into an L5.** Everything the mix knows about an input, and
/// nothing about where it came from.
#[derive(Debug, Clone, Copy)]
pub struct Input {
    /// The level the material arrives at. Colour only.
    pub gain: f32,
    /// The fader across the blend, `[0, 1]`. The only one of the two that
    /// touches what a layer covers.
    pub opacity: f32,
    pub blend: Blend,
    /// What shape of the frame this input reaches.
    pub mask: Mask,
    /// Whether it contributes to this frame at all. **The caller's answer, not
    /// this module's** — the deck folds residency and silence into it; a merge
    /// inside a Set has only silence to fold.
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
    /// **Unity: one term, at full level, under `add`.** `0.0 + 1.0 * src` is
    /// `src` exactly, so a mix of one input at unity hands on the material
    /// rather than a rendering of it — which is what makes an audition, and a
    /// merge nobody has touched, bit-exact.
    pub fn unity() -> Input {
        Input::default()
    }

    /// Whether this edge contributes, folding in the two ways a control can be
    /// silence. Separate from the flag because *which* settings count is the
    /// blend mode's answer: `opacity` at zero silences under every mode, and
    /// `gain` at zero does not silence `over` — a black card covers.
    fn contributes(&self) -> bool {
        self.live && !self.blend.silent_at(self.gain, self.opacity) && !self.mask.hides_everything()
    }
}

/// **Make one edge live and the rest not**, in place.
///
/// Selecting among alternatives, which is what a set of edges into one L5 is
/// for once several of them draw the same thing differently. `false` if `at`
/// names no edge, and **nothing is written when it does not**: a selection
/// nobody can honour must not leave a mix half-silenced on the way to finding
/// that out.
///
/// **A slice rather than a method on whatever owns the edges**, because the
/// rule is about the list and not about the owner: exactly one live, whoever
/// is holding them. [`crate::set::Set::select_renderer`] is the one caller,
/// and this is the half that can be read without a device.
///
/// **`live` and not `opacity`**, which is the difference between selecting and
/// fading. The flag is what `Input::contributes` folds and what the shader
/// skips on, so an unselected input is not read at all — where an opacity of
/// zero is still a texel fetch and a multiply. Neither of them saves the
/// *draw*: the renderer behind an unselected edge fills its own target this
/// frame like every other. See [`crate::set::Set::select_renderer`] for what
/// that costs.
pub fn select(edges: &mut [Input], at: usize) -> bool {
    if at >= edges.len() {
        return false;
    }
    for (i, edge) in edges.iter_mut().enumerate() {
        edge.live = i == at;
    }
    true
}

/// The mix pass: one fullscreen triangle folding up to [`MAX_SLOTS`] textures.
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
                    // Not filterable, because the mix does not sample: it
                    // loads the texel under the fragment. No sampler is bound
                    // here at all, which is what keeps a mix of one exact.
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
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
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
                    // Linear HDR out. The mix is folded in the shader, in input
                    // order, so there is no blend state here: hardware
                    // blending would put the order in the hands of whatever
                    // sequence the passes happened to be recorded in — and
                    // `over` makes that order visible in the picture rather
                    // than only in the last bits.
                    format: Present::HDR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
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

    /// The shader binds [`MAX_SLOTS`] textures whatever the caller's size is, so
    /// a mix of fewer inputs fills the spare bindings with input 0's view. The
    /// live flag for those is zero and the shader skips them, so nothing is
    /// read through them; binding a view twice is cheaper and simpler than a
    /// second pipeline per size, and far simpler than allocating four targets
    /// for a mix of one.
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

    /// Write this frame's edges.
    ///
    /// **Separate from [`Composite::record`]**, on the same terms every node in
    /// this engine separates them: a uniform is a queue write and a pass is an
    /// encoder recording, and a Set writes its uniforms in `prepare` and records
    /// in `draw`. The deck calls the two back to back because it has both in
    /// hand at once.
    ///
    /// The write is a fixed 128 bytes off the stack — the render thread does not
    /// allocate, and this is the render thread.
    pub(crate) fn write_uniform(&self, queue: &wgpu::Queue, inputs: &[Input]) {
        let mut bytes = [0u8; UNIFORM_SIZE as usize];
        for (i, input) in inputs.iter().enumerate().take(MAX_SLOTS) {
            // The blend mode is the shader's index rather than its name; the
            // name is what a record carries.
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
        queue.write_buffer(&self.uniform, 0, &bytes);
    }

    /// Fold the inputs into `target`, from whatever [`Composite::write_uniform`]
    /// last wrote.
    pub(crate) fn record(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // The triangle covers the whole target, so this only
                    // matters for a mix with nothing live in it — which mixes
                    // to black, and should say so rather than showing whatever
                    // was there last frame. `TRANSPARENT` rather than `BLACK`
                    // because the alpha channel is coverage: an empty mix
                    // covers nothing, and `BLACK` would claim it covered
                    // everything opaquely.
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Exactly one live, and it is the one asked for.** The whole claim of a
    /// selection: not "the chosen one is live", which a fold that turned
    /// nothing off would also satisfy, but that everything else stopped.
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

    /// **Everything else about an edge survives being unselected**, which is
    /// what makes a selection reversible in principle and what keeps it from
    /// being a fader in disguise: a renderer selected away and back is at the
    /// gain, opacity, blend and mask it was set to.
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

    /// **An edge that is not there writes nothing.** The refusal has to come
    /// before the loop rather than out of it: silencing two renderers on the
    /// way to discovering there is no third would take the picture away and
    /// report the mistake at the same time.
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
