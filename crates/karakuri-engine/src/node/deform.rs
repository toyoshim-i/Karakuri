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
    /// **How many elements this node writes**, which is what reached it times
    /// its own factor and not the Set's capacity — every node below an
    /// amplifier is sized against the multiplied count. Kept because it is the
    /// denominator of this node's per-element figure and nothing else here
    /// holds it: [`Deform::amplify`] is one link of the product, not the
    /// product.
    out_capacity: u32,
    element_layout: ElementLayout,
    /// What this node's output carries: everything that reached it, plus its
    /// own `emit`. The next node in a chain widens this in turn.
    emits: Vec<Attr>,
    /// The engine-written slots this node's output carries. Travels beside
    /// [`Deform::emits`] because the next node addresses this node's buffer and
    /// has to name the same fields.
    synthetic: Synthetic,
    /// **What an amplifying node owns that an endomorphic one does not.**
    /// `None` for a node that keeps the element count, and its absence is the
    /// whole difference: without it this node hands its input's liveness and
    /// its input's counts straight on, which is what every L2 did before
    /// amplification existed.
    amplified: Option<Amplified>,
    /// **The declaration names, which are the uniform's own field names** —
    /// one `glow` for a `vec3`. [`super::write_params`] walks this, so it must
    /// not carry components: the packer finds a field by name and a `glow.x`
    /// is in no layout.
    param_names: Vec<String>,
    /// **The keys this node's params are *addressed* by** — `glow.x`,
    /// `glow.y`, `glow.z` for that same `vec3`.
    ///
    /// `Set::declared_names` walks this, which is what `Set::published` and
    /// `Set::bind` are built from, and it is the list the value map is keyed
    /// by. Two lists rather than one because the uniform and the address
    /// disagree about what a vector is
    /// ([ADR-0268](../../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)).
    param_keys: Vec<String>,
}

/// The three things a node that changes the element count has to own.
///
/// **A longer element buffer needs a longer alive buffer and a bigger set of
/// counts, and neither can be borrowed.** `docs/ir-spec.md` puts liveness
/// entirely upstream — an L2 cannot `kill()` — and this does not take that
/// back: what is written here is each parent's flag, repeated `factor` times.
/// The decision is still the L1's; only the indexing is this node's.
///
/// **Single-buffered and never compacted**, which is what makes amplification
/// cheaper than its position suggests: the output is rebuilt from the input
/// every frame, so nothing reads its previous value and there is no parity to
/// choose between.
struct Amplified {
    factor: u32,
    alive: wgpu::Buffer,
    counts: wgpu::Buffer,
    /// The one-invocation pass that turns the input's counts into this node's.
    derive: wgpu::ComputePipeline,
    derive_bg: wgpu::BindGroup,
}

impl Deform {
    /// Generate, compile and bind one L2 node against the geometry reaching it.
    ///
    /// `upstream` is the attribute list behind `input`'s layout — what is
    /// available at this position in the chain. It is passed alongside the
    /// geometry rather than derived from it because an `ElementLayout` is a
    /// list of slots and two of them carry no `Attr` at all.
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
        // **The output capacity, and it is what everything below this node is
        // sized and dispatched against.** Saturating rather than wrapping: the
        // checker caps a single factor, a Set caps its own capacity, and a chain
        // of amplifiers is still a product that a `u32` can be walked off the
        // end of. A wrapped capacity allocates a buffer that is too small and is
        // read past; a saturated one is caught by the check below.
        let out_capacity = capacity.saturating_mul(shader.amplify.unwrap_or(1));
        // **Sized where the figure is decided** — see `crate::storage`. Every
        // buffer this node is charged for comes off this, including the number
        // the limit below refuses, so what a device rejects and what a caller
        // with no device is told are one expression.
        let element_storage = DeformStorage::of(
            out_capacity,
            shader.element_layout.stride,
            shader.amplify.is_some(),
        );
        // **Asked of the device, here, rather than left to the allocation.**
        // wgpu does not return an error for a binding above the limit — its
        // uncaptured error handler panics the thread, which at startup takes the
        // process down and on the swap worker is a `SetError::Panicked`. The
        // checker's own ceiling on `amplify` cannot stand in for this: it sees
        // one declaration, and what is too large is the product of the Set's
        // capacity, every factor above this node, and the element stride, none
        // of which a single file knows.
        //
        // Checked for every node rather than only for amplifiers, because a
        // plain L2 *below* one inherits the amplified capacity and is exactly as
        // able to exceed the limit.
        let bytes = element_storage.element_buffer();
        // `u64` since wgpu 30 — the limit itself, not a widened `u32`.
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
        // **The far geometry joins the input group**, because that is what
        // it is: a second input edge, read and never written.
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
                // `expect` rather than a second `if`: this closure runs exactly
                // when `shader.amplify` is `Some`, which is the predicate the
                // storage was built with, so a `None` here would mean the two
                // had come apart — which is the thing worth stopping on.
                size: element_storage
                    .alive_buffer()
                    .expect("an amplifying node is charged for its own alive array"),
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            let counts = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{} counts", l2.name)),
                // INDIRECT because a renderer draws from it and a further L2
                // dispatches from it; COPY_SRC because nothing else can see
                // what is in it — the fields a picture depends on are checked
                // through the picture, and the ones it does not are only
                // checkable by reading them. COPY_DST is not needed: every
                // field is written by the derive pass and none by the host.
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
                // **The far source's own parity**, which is the same one:
                // both sources are static — no spawn, no kill, so no compaction
                // — and a static simulation's parity walks with the step count
                // exactly as this one's does.
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

    /// **What this node allocated to hold elements** — see [`ElementStorage`]
    /// for what counts and why the figure is read off the buffers.
    ///
    /// **One element buffer, never two.** An L2's output is rebuilt from its
    /// input every frame and nothing reads back what it wrote, so there is no
    /// second direction to pay for — the doubling is the L1's alone.
    ///
    /// **And an alive array only where it amplifies.** A deform that emits the
    /// elements that reached it emits them under the flags they arrived with
    /// and shares that buffer; an amplifier's outputs are new elements nothing
    /// upstream has a flag for, so it owns one `factor` times as long. The
    /// counts block it owns beside that is one block per node however large the
    /// capacity, and is not part of a per-element figure.
    ///
    /// **The stride here is the chain's, not this procedure's.** What an L2
    /// writes carries everything that reached it as well as its own `emit`, so
    /// a node that names one attribute over an L1 that emits four is sized for
    /// five. That is the input no per-procedure estimate can have and the whole
    /// reason this figure is asked of the node.
    pub(crate) fn element_storage(&self) -> ElementStorage {
        ElementStorage {
            bytes: self.elements.size() + self.amplified.as_ref().map_or(0, |a| a.alive.size()),
            capacity: self.out_capacity,
        }
    }

    /// How many elements this node's output holds per element reaching it — the
    /// number the next node down allocates against.
    pub(crate) fn amplify(&self) -> u32 {
        self.amplified.as_ref().map_or(1, |a| a.factor)
    }

    /// Whether this node owns the liveness and counts below it.
    ///
    /// Asked rather than derived from [`Deform::amplify`]: the two agree today
    /// only because the checker refuses a factor below two, which is a rule in
    /// another crate. What the chain needs to know is whose buffers it is on,
    /// and that is this question rather than an arithmetic one about the count.
    pub(crate) fn amplifies(&self) -> bool {
        self.amplified.is_some()
    }

    /// The engine-written slots this node's output carries.
    pub(crate) fn synthetic(&self) -> Synthetic {
        self.synthetic
    }

    /// The counts everything below this node runs on, or `None` where this node
    /// changed nothing and whatever reached it still applies.
    ///
    /// A `Counts` is three numbers at once — the workgroup count a compute pass
    /// dispatches over, the live range a pass bounds itself by, and the instance
    /// count a renderer draws — and an amplifier multiplies all three. Handing a
    /// node below one the simulation's instead is a chain that computes four
    /// elements and draws one of them.
    pub(crate) fn counts(&self) -> Option<&wgpu::Buffer> {
        self.amplified.as_ref().map(|a| &a.counts)
    }

    /// The edge this node offers downstream.
    ///
    /// **One element buffer under both parities, and the alive flags belong to
    /// whoever made them.** The elements are this node's own and are rewritten
    /// each frame, so there is nothing for a parity to choose between; the
    /// liveness is the L1's and passes through every deformation untouched.
    pub(crate) fn geometry<'a>(
        &'a self,
        alive: [&'a wgpu::Buffer; 2],
        counts: &'a wgpu::Buffer,
    ) -> Geometry<'a> {
        // **An amplifier answers with its own liveness and its own counts**, and
        // that is the one place the doc above stops being the whole story: the
        // flags it hands on are still the L1's decision, re-indexed onto a
        // buffer `factor` times as long, and the counts are the input's
        // multiplied out. A node that does not amplify passes both straight
        // through, unlooked-at.
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

    /// What reaches the next node: everything upstream had, plus this node's
    /// own `emit`.
    pub(crate) fn emits(&self) -> &[Attr] {
        &self.emits
    }

    /// **The addressable keys, and there is deliberately no accessor for the
    /// other list.** `param_names` is read at the one place it means anything
    /// — this node's own uniform write — and handing it out would be handing
    /// out a list of names a `--param` cannot use.
    pub(crate) fn param_keys(&self) -> &[String] {
        &self.param_keys
    }

    /// This node's uniform block, from the grouping's view of the frame.
    ///
    /// Reuses [`View`] rather than [`super::Tick`]: an L2 runs once per frame
    /// at the instant the simulation reached, which is the same instant a
    /// renderer draws at — not the per-substep sequence a simulation walks.
    /// The camera and the viewport in that struct are simply unread here.
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
        // **The spliced field's params, written by every caller.** A field has
        // no node and therefore no uniform of its own; each procedure that
        // evaluates it carries them in its own and writes the same answer.
        super::write_field_params(
            &mut p,
            &self.uniform_layout,
            view.field_params,
            view.field_value,
        );
        // **The Source slots this node declared**, each holding the identity
        // of the geometry its edge named — see [`View::source_value`].
        super::write_source_slots(&mut p, &self.uniform_layout, view.source_value);
        queue.write_buffer(&self.uniforms, 0, p.finish());
    }

    /// Record this node's pass. **Once per frame, after the simulation** — not
    /// once per substep: a deformation is a function of the instant the
    /// simulation reached, and running it between substeps would deform states
    /// nothing ever draws.
    /// Derive this node's counts from the ones its input came with.
    ///
    /// **Separate from [`Deform::record`], because a frame that steps nothing
    /// still needs it.** A deck draws an `Allocated` slot without stepping it —
    /// that is what an audition is — and the counts this writes are the
    /// renderer's indirect draw arguments. Left to the step, an amplified Set
    /// that had never been stepped drew from a freshly allocated buffer, which
    /// is zeroed: no vertices, no instances, a black audition of a Set that
    /// works. The camera node is here for the identical reason and one function
    /// along.
    ///
    /// Idempotent and one invocation, so running it in both places costs a
    /// dispatch and cannot disagree with itself.
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
        // Indirect over the live range, the same number `element` dispatches
        // over — the host never learns it and never needs to.
        pass.dispatch_workgroups_indirect(counts_buf, counts::ELEM_XYZ);
    }
}
