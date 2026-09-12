//! The L2 node: `Geometry -> Geometry`.

use karakuri_codegen::generate_l2;
use karakuri_codegen::layout::{binding, counts, group, UniformLayout, WORKGROUP_SIZE};
use karakuri_ir::layout::{ElementLayout, Synthetic};
use karakuri_ir::typed::Checked;
use karakuri_ir::Attr;

use super::{Geometry, View};
use crate::set::{ElementStorage, SetError};
use crate::storage::DeformStorage;
use crate::uniforms::UniformScratch;

/// An L2 compute node that transforms input geometry to output geometry.
///
/// An L2 node is stateless and rebuilt each frame from its input, allowing a single
/// output element buffer to serve both parities. Compute is evaluated once per frame,
/// and output is shared across all downstream readers.
pub(crate) struct Deform {
    pipeline: wgpu::ComputePipeline,
    uniforms: wgpu::Buffer,
    uniform_layout: UniformLayout,
    scratch: UniformScratch,
    uniform_bg: wgpu::BindGroup,
    /// Bind groups indexed by input parity, selecting which element buffer to read.
    src_bg: [wgpu::BindGroup; 2],
    dst_bg: wgpu::BindGroup,
    /// Output element buffer holding transformed vertex data.
    elements: wgpu::Buffer,
    /// Total elements written after applying any amplification factor.
    out_capacity: u32,
    element_layout: ElementLayout,
    /// Attributes output by this node, including inherited and emitted attributes.
    emits: Vec<Attr>,
    /// Engine-generated synthetic attributes carried by the output buffer.
    synthetic: Synthetic,
    /// Amplification resources (counts, alive buffers, derive pipeline) if factor > 1.
    amplified: Option<Amplified>,
    /// Uniform field names declared by this procedure.
    param_names: Vec<String>,
    /// Component-addressed parameter keys (e.g. `glow.x`, `glow.y`).
    param_keys: Vec<String>,
}

/// Resources owned by an amplifying L2 node.
///
/// Amplification expands the element buffer by `factor`, requiring dedicated
/// alive and counts buffers. Alive flags are repeated `factor` times from
/// upstream. These buffers are single-buffered because output is rebuilt every frame.
struct Amplified {
    factor: u32,
    alive: wgpu::Buffer,
    counts: wgpu::Buffer,
    /// The one-invocation pass that turns the input's counts into this node's.
    derive: wgpu::ComputePipeline,
    derive_bg: wgpu::BindGroup,
}

impl Deform {
    /// Generates, compiles, and binds an L2 compute node for input geometry.
    ///
    /// `upstream` contains the attribute list for `input`'s layout. It is passed
    /// separately from `input` because `ElementLayout` slots do not all correspond
    /// to an [`Attr`].
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn build(
        device: &wgpu::Device,
        l2: &Checked,
        upstream: &[Attr],
        synthetic: Synthetic,
        derived: &[Attr],
        // The far geometry, for an L2 that declares a `uses` slot — its
        // attribute list and the edge it reads. `None` for every other L2, and
        // *which* geometry it is was settled where the Set resolved the edge.
        far: Option<(&[Attr], &Geometry<'_>)>,
        fields: karakuri_codegen::Bound<'_>,
        input: &Geometry<'_>,
        capacity: u32,
    ) -> Result<Deform, SetError> {
        let shader = generate_l2(
            l2,
            upstream,
            synthetic,
            derived,
            far.map(|(a, _)| a),
            fields,
        );
        // Saturating rather than wrapping: a chain of amplifiers is a product a
        // `u32` can be walked off the end of, and a wrapped capacity allocates a
        // buffer that is too small and is then read past. A saturated one is
        // caught by the limit check below.
        let out_capacity = capacity.saturating_mul(shader.amplify.unwrap_or(1));
        // Every buffer this node is charged for comes off this, including the
        // number the limit below refuses, so what a device rejects and what a
        // caller with no device is told are one expression.
        let element_storage = DeformStorage::of(
            out_capacity,
            shader.element_layout.stride,
            shader.amplify.is_some(),
        );
        // Check buffer size against device limit before allocation; wgpu panics on
        // over-limit bindings rather than returning an error. Plain L2 nodes below
        // an amplifying node inherit the amplified capacity and also require checking.
        let bytes = element_storage.element_buffer();
        let limit = device.limits().max_storage_buffer_binding_size;
        if bytes > limit {
            return Err(SetError::TooManyElements {
                l2: l2.name.clone(),
                elements: u64::from(out_capacity),
                bytes,
                limit,
            });
        }
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} (L2)", l2.name)),
            source: wgpu::ShaderSource::Wgsl(shader.source.as_str().into()),
        });

        let elements = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} elements", l2.name)),
            size: element_storage.element_buffer(),
            // Element buffer is consumed by downstream GPU passes; no host readback is needed.
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
        // Non-amplifying L2 nodes share the input alive buffer since L2 cannot kill elements.
        // Amplifying nodes generate new alive flags and bind them writable in dst.
        // Far geometry provides a read-only secondary input edge if present.
        let mut src_entries = vec![
            storage(binding::ELEMENT, true),
            storage(binding::ALIVE, true),
        ];
        if far.is_some() {
            src_entries.push(storage(binding::FAR, true));
        }
        let src_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L2 src"),
            entries: &src_entries,
        });
        let dst_entries: Vec<wgpu::BindGroupLayoutEntry> = if shader.amplify.is_some() {
            vec![
                storage(binding::ELEMENT, false),
                storage(binding::ALIVE, false),
            ]
        } else {
            vec![storage(binding::ELEMENT, false)]
        };
        let dst_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L2 dst"),
            entries: &dst_entries,
        });

        // Allocated before the bind group that names it, and only for a node
        // that amplifies — a node that does not shares its input's, which is
        // the same buffer under both parities and belongs to the L1.
        let amplified_buffers = shader.amplify.map(|factor| {
            let alive = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{} alive", l2.name)),
                // DeformStorage was constructed with amplification enabled, so alive_buffer is present.
                size: element_storage
                    .alive_buffer()
                    .expect("an amplifying node is charged for its own alive array"),
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            let counts = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{} counts", l2.name)),
                // Buffer is used for indirect dispatch/draw arguments and read back in tests.
                size: counts::SIZE,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::INDIRECT
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            (factor, alive, counts)
        });

        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("L2 uniforms"),
            layout: &uniform_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: binding::UNIFORM,
                    resource: uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: binding::COUNTS,
                    resource: input.counts.as_entire_binding(),
                },
            ],
        });
        let bind_src = |label: &str, parity: usize| {
            let mut entries = vec![
                wgpu::BindGroupEntry {
                    binding: binding::ELEMENT,
                    resource: input.elements[parity].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: binding::ALIVE,
                    resource: input.alive[parity].as_entire_binding(),
                },
            ];
            if let Some((_, geometry)) = far {
                // Static sources advance parity with step count; match the current parity.
                entries.push(wgpu::BindGroupEntry {
                    binding: binding::FAR,
                    resource: geometry.elements[parity].as_entire_binding(),
                });
            }
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &src_bgl,
                entries: &entries,
            })
        };
        let src_bg = [bind_src("L2 src0", 0), bind_src("L2 src1", 1)];
        let mut dst_bg_entries = vec![wgpu::BindGroupEntry {
            binding: binding::ELEMENT,
            resource: elements.as_entire_binding(),
        }];
        if let Some((_, alive, _)) = &amplified_buffers {
            dst_bg_entries.push(wgpu::BindGroupEntry {
                binding: binding::ALIVE,
                resource: alive.as_entire_binding(),
            });
        }
        let dst_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("L2 dst"),
            layout: &dst_bgl,
            entries: &dst_bg_entries,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("L2"),
            bind_group_layouts: &[Some(&uniform_bgl), Some(&src_bgl), Some(&dst_bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&l2.name),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("deform"),
            compilation_options: Default::default(),
            cache: None,
        });

        let amplified = amplified_buffers.map(|(factor, alive, counts)| {
            let source = include_str!("../shaders/amplify.wgsl")
                .replace("{{COUNTS_STRUCT}}", counts::WGSL)
                .replace("{{FACTOR}}", &factor.to_string())
                .replace("{{WG}}", &WORKGROUP_SIZE.to_string());
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("{} (amplify counts)", l2.name)),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("amplify counts"),
                entries: &[storage(0, true), storage(1, false)],
            });
            let derive_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("amplify counts"),
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input.counts.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: counts.as_entire_binding(),
                    },
                ],
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("amplify counts"),
                bind_group_layouts: &[Some(&bgl)],
                immediate_size: 0,
            });
            let derive = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("amplify counts"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("derive"),
                compilation_options: Default::default(),
                cache: None,
            });
            Amplified {
                factor,
                alive,
                counts,
                derive,
                derive_bg,
            }
        });

        Ok(Deform {
            pipeline,
            uniforms,
            uniform_layout: shader.uniform_layout.clone(),
            scratch: UniformScratch::new(&shader.uniform_layout),
            uniform_bg,
            src_bg,
            dst_bg,
            elements,
            out_capacity,
            element_layout: shader.element_layout,
            emits: shader.emits,
            synthetic: shader.synthetic,
            amplified,
            param_names: l2.params.iter().map(|p| p.name.clone()).collect(),
            param_keys: crate::set::declared_keys(l2),
        })
    }

    /// Returns the element storage allocated by this node; see [`ElementStorage`].
    ///
    /// Includes the element buffer and, if amplified, the alive buffer.
    pub(crate) fn element_storage(&self) -> ElementStorage {
        ElementStorage {
            bytes: self.elements.size() + self.amplified.as_ref().map_or(0, |a| a.alive.size()),
            capacity: self.out_capacity,
        }
    }

    /// Returns the amplification factor (output elements per input element).
    pub(crate) fn amplify(&self) -> u32 {
        self.amplified.as_ref().map_or(1, |a| a.factor)
    }

    /// Returns true if this node owns its liveness and indirect counts buffers.
    pub(crate) fn amplifies(&self) -> bool {
        self.amplified.is_some()
    }

    /// Returns synthetic attributes generated for this node's output.
    pub(crate) fn synthetic(&self) -> Synthetic {
        self.synthetic
    }

    /// Returns the indirect counts buffer if amplified, or `None` if upstream applies.
    pub(crate) fn counts(&self) -> Option<&wgpu::Buffer> {
        self.amplified.as_ref().map(|a| &a.counts)
    }

    /// Returns the geometry edge offered downstream.
    ///
    /// A single element buffer serves both parities since output is rewritten
    /// each frame. If amplified, returns this node's own alive and counts buffers;
    /// otherwise passes through the caller's.
    pub(crate) fn geometry<'a>(
        &'a self,
        alive: [&'a wgpu::Buffer; 2],
        counts: &'a wgpu::Buffer,
    ) -> Geometry<'a> {
        let (alive, counts) = match &self.amplified {
            None => (alive, counts),
            Some(a) => ([&a.alive, &a.alive], &a.counts),
        };
        Geometry {
            layout: &self.element_layout,
            elements: [&self.elements, &self.elements],
            alive,
            counts,
        }
    }

    /// Returns all attributes available downstream (upstream attributes plus emitted).
    pub(crate) fn emits(&self) -> &[Attr] {
        &self.emits
    }

    /// Returns the addressable parameter keys for this node.
    pub(crate) fn param_keys(&self) -> &[String] {
        &self.param_keys
    }

    /// Writes this node's uniform block using the frame view.
    ///
    /// Deformations run once per frame at the end of the simulation step,
    /// rather than per simulation substep.
    pub(crate) fn write_uniforms(
        &mut self,
        queue: &wgpu::Queue,
        view: &View<'_>,
        dt: f32,
        capacity: u32,
    ) {
        let mut p = self.scratch.pack(&self.uniform_layout);
        p.f32("t", view.t)
            .f32("beats", view.beats)
            .f32("dt", dt)
            .u32("capacity", capacity)
            .u32("seed_salt", view.seed_salt);
        super::write_params(
            &mut p,
            &self.uniform_layout,
            &self.param_names,
            view.param,
            view.param_value,
        );
        super::write_field_params(
            &mut p,
            &self.uniform_layout,
            view.field_params,
            view.field_value,
        );
        super::write_source_slots(&mut p, &self.uniform_layout, view.source_value);
        queue.write_buffer(&self.uniforms, 0, p.finish());
    }

    /// Records the compute pass to derive amplified dispatch and draw counts.
    ///
    /// Runs separately from [`Deform::record`] so unstepped frames (such as
    /// during audition) still populate valid indirect draw arguments.
    pub(crate) fn record_counts(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(a) = &self.amplified else { return };
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("amplify counts"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&a.derive);
        pass.set_bind_group(0, &a.derive_bg, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    /// Records this node's compute deformation pass.
    pub(crate) fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        parity: usize,
        counts_buf: &wgpu::Buffer,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("deform"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(group::UNIFORMS, &self.uniform_bg, &[]);
        pass.set_bind_group(group::PREV, &self.src_bg[parity], &[]);
        pass.set_bind_group(group::NEXT, &self.dst_bg, &[]);
        // Indirect dispatch over the live element range.
        pass.dispatch_workgroups_indirect(counts_buf, counts::ELEM_XYZ);
    }
}
