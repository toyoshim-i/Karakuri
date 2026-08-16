//! The L2 node: `Geometry -> Geometry`.

use karakuri_codegen::generate_l2;
use karakuri_codegen::layout::{binding, counts, group, ElementLayout, UniformLayout};
use karakuri_ir::typed::Checked;
use karakuri_ir::Attr;

use super::{Geometry, View};
use crate::uniforms::UniformScratch;

/// An L2 node: geometry in, geometry out.
///
/// **It owns one element buffer, not two.** An L1 double-buffers because it
/// reads what it wrote last frame; an L2 cannot — its output is rebuilt from its
/// input every frame, which is what "stateless by rule" means here and is
/// enforced by the lowering rather than by a check. One buffer is therefore
/// enough, and the same buffer serves both parities: what a reader picks by
/// parity is the *element* buffer this node writes, and there is only one of it.
///
/// **Materialised rather than fused**, which is the choice `docs/ir-spec.md`
/// makes for the layer: the deformation is paid once however many nodes read
/// its output, where fusing it into each reader would pay it per reader.
/// Fusing is what a graph compiler may do later; statelessness is what keeps
/// that legal rather than merely plausible.
pub(crate) struct Deform {
    pipeline: wgpu::ComputePipeline,
    uniforms: wgpu::Buffer,
    uniform_layout: UniformLayout,
    scratch: UniformScratch,
    uniform_bg: wgpu::BindGroup,
    /// Indexed by parity: which of the *input's* element buffers to read.
    src_bg: [wgpu::BindGroup; 2],
    dst_bg: wgpu::BindGroup,
    /// The buffer `dst_bg` names, kept so the node can hand out its own
    /// [`Geometry`].
    elements: wgpu::Buffer,
    element_layout: ElementLayout,
    /// What this node's output carries: everything that reached it, plus its
    /// own `emit`. The next node in a chain widens this in turn.
    emits: Vec<Attr>,
    param_names: Vec<String>,
}

impl Deform {
    /// Generate, compile and bind one L2 node against the geometry reaching it.
    ///
    /// `upstream` is the attribute list behind `input`'s layout — what is
    /// available at this position in the chain. It is passed alongside the
    /// geometry rather than derived from it because an `ElementLayout` is a
    /// list of slots and two of them carry no `Attr` at all.
    pub(crate) fn build(
        device: &wgpu::Device,
        l2: &Checked,
        upstream: &[Attr],
        input: &Geometry<'_>,
        capacity: u32,
    ) -> Deform {
        let shader = generate_l2(l2, upstream);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} (L2)", l2.name)),
            source: wgpu::ShaderSource::Wgsl(shader.source.as_str().into()),
        });

        let elements = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} elements", l2.name)),
            size: u64::from(capacity) * u64::from(shader.element_layout.stride),
            // No COPY_SRC: nothing reads this back. `Set::read_elements` is
            // about what the simulation holds, which is the L1's buffer and not
            // a derived one.
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} uniforms", l2.name)),
            size: u64::from(shader.uniform_layout.total_size),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let storage = |b: u32, read_only: bool| wgpu::BindGroupLayoutEntry {
            binding: b,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L2 uniforms"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: binding::UNIFORM,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage(binding::COUNTS, true),
            ],
        });
        // **The alive flags are read and never written.** An L2 cannot `kill()`,
        // so liveness passes through untouched — the node's output shares the
        // very buffer its input came with.
        let src_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L2 src"),
            entries: &[storage(binding::ELEMENT, true), storage(binding::ALIVE, true)],
        });
        let dst_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L2 dst"),
            entries: &[storage(binding::ELEMENT, false)],
        });

        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("L2 uniforms"),
            layout: &uniform_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: binding::UNIFORM, resource: uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: binding::COUNTS, resource: input.counts.as_entire_binding() },
            ],
        });
        let bind_src = |label: &str, parity: usize| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &src_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: binding::ELEMENT,
                        resource: input.elements[parity].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: binding::ALIVE,
                        resource: input.alive[parity].as_entire_binding(),
                    },
                ],
            })
        };
        let src_bg = [bind_src("L2 src0", 0), bind_src("L2 src1", 1)];
        let dst_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("L2 dst"),
            layout: &dst_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: binding::ELEMENT,
                resource: elements.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("L2"),
            bind_group_layouts: &[&uniform_bgl, &src_bgl, &dst_bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&l2.name),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("deform"),
            compilation_options: Default::default(),
            cache: None,
        });

        Deform {
            pipeline,
            uniforms,
            uniform_layout: shader.uniform_layout.clone(),
            scratch: UniformScratch::new(&shader.uniform_layout),
            uniform_bg,
            src_bg,
            dst_bg,
            elements,
            element_layout: shader.element_layout,
            emits: shader.emits,
            param_names: l2.params.iter().map(|p| p.name.clone()).collect(),
        }
    }

    /// The edge this node offers downstream.
    ///
    /// **One element buffer under both parities, and the alive flags belong to
    /// whoever made them.** The elements are this node's own and are rewritten
    /// each frame, so there is nothing for a parity to choose between; the
    /// liveness is the L1's and passes through every deformation untouched.
    pub(crate) fn geometry<'a>(&'a self, alive: [&'a wgpu::Buffer; 2], counts: &'a wgpu::Buffer) -> Geometry<'a> {
        Geometry {
            layout: &self.element_layout,
            elements: [&self.elements, &self.elements],
            alive,
            counts,
        }
    }

    /// What reaches the next node: everything upstream had, plus this node's
    /// own `emit`.
    pub(crate) fn emits(&self) -> &[Attr] {
        &self.emits
    }

    pub(crate) fn param_names(&self) -> &[String] {
        &self.param_names
    }

    /// This node's uniform block, from the grouping's view of the frame.
    ///
    /// Reuses [`View`] rather than [`super::Tick`]: an L2 runs once per frame
    /// at the instant the simulation reached, which is the same instant a
    /// renderer draws at — not the per-substep sequence a simulation walks.
    /// The camera and the viewport in that struct are simply unread here.
    pub(crate) fn write_uniforms(&mut self, queue: &wgpu::Queue, view: &View<'_>, dt: f32, capacity: u32) {
        let mut p = self.scratch.pack(&self.uniform_layout);
        p.f32("t", view.t)
            .f32("beats", view.beats)
            .f32("dt", dt)
            .u32("capacity", capacity)
            .u32("seed_salt", view.seed_salt);
        super::write_params(&mut p, &self.uniform_layout, &self.param_names, view.param);
        queue.write_buffer(&self.uniforms, 0, p.finish());
    }

    /// Record this node's pass. **Once per frame, after the simulation** — not
    /// once per substep: a deformation is a function of the instant the
    /// simulation reached, and running it between substeps would deform states
    /// nothing ever draws.
    pub(crate) fn record(&self, encoder: &mut wgpu::CommandEncoder, parity: usize, counts_buf: &wgpu::Buffer) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("deform"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(group::UNIFORMS, &self.uniform_bg, &[]);
        pass.set_bind_group(group::PREV, &self.src_bg[parity], &[]);
        pass.set_bind_group(group::NEXT, &self.dst_bg, &[]);
        // Indirect over the live range, the same number `element` dispatches
        // over — the host never learns it and never needs to.
        pass.dispatch_workgroups_indirect(counts_buf, counts::ELEM_XYZ);
    }
}
