//! The L4 node: `(Geometry, Camera) -> Texture`.

use karakuri_codegen::generate_l4;
use karakuri_codegen::layout::{binding, counts, group, UniformLayout};
use karakuri_ir::typed::Checked;

use super::{Camera, Geometry, View};
use crate::oit::Oit;
use crate::uniforms::UniformScratch;

/// An L4 node: `(Geometry, Camera) -> Texture`.
pub(crate) struct Renderer {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    uniform_layout: UniformLayout,
    scratch: UniformScratch,
    uniform_bg: wgpu::BindGroup,
    /// Indexed by parity. This is the edge, resolved: the two bind groups name
    /// the [`Geometry`] this node was built against.
    attr_bg: [wgpu::BindGroup; 2],
    /// The other edge, resolved the same way: the [`Camera`] this node was built
    /// against, and the group index its shader reads it at.
    ///
    /// **`None` for a shader that reads no camera**, which is a real case rather
    /// than a defensive one: an L4 whose vertex block writes `clip` without
    /// projecting reads nothing from it. See
    /// [`karakuri_codegen::L4Shader::camera_group`].
    camera_bg: Option<(u32, wgpu::BindGroup)>,
    /// Present only under `blend weighted` — see [`crate::oit`].
    oit: Option<Oit>,
    /// **What this node draws**, carried from the checked procedure rather
    /// than reduced to a flag.
    ///
    /// Two readers. The draw asks whether there is a primitive to size at all
    /// — a fullscreen L4 consumes nothing, which the check pass enforces, so a
    /// Set whose only renderer is one has nothing reading its element buffers
    /// and skips the L1 passes entirely. [`crate::estimate`] asks the same
    /// question for a different reason: a procedure with no `vertex` block
    /// emits no `point_rate`, so there is no primitive that can fall under a
    /// pixel and ADR-0245's sub-pixel floor does not apply to it.
    ///
    /// **Neither reader asks a three-way question**, and this comment claimed
    /// one did until ADR-0266: *`Fullscreen` and `Lines` cost the target's area
    /// and `Points` does not, so a small draw extrapolates one way or the other
    /// by this field*. Nothing ever branched that way, and the rule that
    /// replaced the extrapolation says it never should — what decides whether a
    /// procedure's cost tracks the target's area is its coverage,
    /// `capacity × rate²`, which a param moves at any time. The full value is
    /// still carried rather than reduced to a `bool` because
    /// [`crate::estimate::Estimate`] reports it, so a number can be read
    /// against what was drawn.
    topology: karakuri_ir::Topology,
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

impl Renderer {
    /// Generate, compile and bind one L4 node against `geometry`.
    ///
    /// **Infallible.** The one refusal that used to live here — `blend weighted`
    /// on a procedure that draws the whole frame — turned out to be a rule about
    /// the *Set*: it holds only while this node is the only one drawing, which
    /// is not something a node can know about itself. It moved to
    /// `Set::build_many`, where the count is. See
    /// [`SetError::WeightedFullscreen`].
    pub(crate) fn build(
        device: &wgpu::Device,
        l4: &Checked,
        geometry: &Geometry<'_>,
        camera: &Camera,
        fields: karakuri_codegen::Bound<'_>,
    ) -> Renderer {
        // **`Points` where a procedure somehow declared nothing.** `check`
        // infers a topology for every L4 (`drawn_topology`), so the `None` arm
        // is unreachable; it is spelled out rather than unwrapped because the
        // conservative answer for a cost extrapolation and the conservative
        // answer for the draw are the same one — a per-element renderer.
        let topology = l4.topology.unwrap_or(karakuri_ir::Topology::Points);
        let fullscreen = topology == karakuri_ir::Topology::Fullscreen;
        let weighted = l4.blend == Some(karakuri_ir::Blend::Weighted);

        let shader = generate_l4(l4, geometry.layout, fields);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} (L4)", l4.name)),
            source: wgpu::ShaderSource::Wgsl(shader.source.as_str().into()),
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("L4 uniforms"),
            size: u64::from(shader.uniform_layout.total_size),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L4 uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: binding::UNIFORM,
                visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let storage_entry = |binding_num: u32| wgpu::BindGroupLayoutEntry {
            binding: binding_num,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        // Both buffers, not just the elements: the draw range holds elements
        // killed during the step that just ran, scattered among the survivors
        // rather than gathered at either end, and the vertex stage skips them
        // per instance by reading the flag.
        let attr_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("attrs"),
            entries: &[
                storage_entry(binding::ELEMENT),
                storage_entry(binding::ALIVE),
            ],
        });

        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("L4 uniforms"),
            layout: &uniform_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: binding::UNIFORM,
                resource: uniforms.as_entire_binding(),
            }],
        });
        // L4 reads what L1 last wrote. The frame's compute pass writes "next",
        // then parity flips, so L4 binds the same physical element buffer that
        // is "prev" under the flipped parity — which is what `Geometry`'s arrays
        // are already indexed by.
        let bind_attrs = |label: &str, parity: usize| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &attr_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: binding::ELEMENT,
                        resource: geometry.elements[parity].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: binding::ALIVE,
                        resource: geometry.alive[parity].as_entire_binding(),
                    },
                ],
            })
        };
        let attr_bg = [bind_attrs("l4attrs0", 0), bind_attrs("l4attrs1", 1)];

        // **No attribute group for a fullscreen shader**, which declares none:
        // it consumes nothing, so it binds nothing. wgpu would accept the extra
        // group — a layout may name one the module does not use — so this is
        // about the layout saying what the shader is, and about leaving the
        // *number* free for the camera below, which does need a group index with
        // nothing missing under it.
        let mut groups: Vec<Option<&wgpu::BindGroupLayout>> = if fullscreen {
            vec![Some(&uniform_bgl)]
        } else {
            vec![Some(&uniform_bgl), Some(&attr_bgl)]
        };
        // **The generated source names the index and this asserts it**, rather
        // than a constant in two crates that agree by convention: which group
        // the camera lands in depends on whether the shader bound attributes
        // below it, and that is the generator's decision. What the assertion
        // catches is a *hole* — group 2 declared with group 1 missing — which is
        // the shape wgpu refuses; a trailing group nothing uses it accepts.
        if let Some(g) = shader.camera_group {
            assert_eq!(
                g as usize,
                groups.len(),
                "the camera's group index must follow the groups below it"
            );
            groups.push(Some(camera.layout()));
        }
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("L4"),
            bind_group_layouts: &groups,
            immediate_size: 0,
        });
        // **What the fragment stage writes to, which the blend mode chooses.**
        // The generated shader returns one `vec4` or a two-field struct — see
        // `karakuri_codegen`'s `WEIGHTED_FS_OUT` — and a pipeline whose targets
        // did not match would be a validation error rather than a wrong picture.
        let weighted_targets = crate::oit::colour_targets();
        let additive_target = [Some(wgpu::ColorTargetState {
            // Always the linear HDR format, never the surface's. A
            // `VideoSource` renders into the HDR target and the present
            // pass is the one place that encodes to sRGB; taking this
            // as a parameter would let a caller quietly break "the
            // pipeline is linear and HDR end to end".
            format: crate::present::Present::HDR_FORMAT,
            // `blend additive`, no depth write.
            //
            // **Colour adds; alpha accumulates coverage.** The two
            // components answer different questions and this is the
            // only pairing that answers both: colour is emissive and
            // sums past what any coverage would allow, which is what
            // `blend additive` is for, while alpha comes out as
            // `1 - prod(1 - a_i)` — the probability that *something*
            // drew at this texel, and order-independent because
            // `a_s + a_d(1 - a_s)` is symmetric in the two.
            //
            // **Not bounded at 1, and the mix does not assume it is.**
            // Nothing clamps what a fragment block assigns to alpha —
            // the IR calls it straight alpha and says values above 1.0
            // are expected — so this accumulates whatever the material
            // wrote. `composite.wgsl` saturates on the way in rather
            // than L4 clamping on the way out, because clamping here
            // would change the colour too: additive blending
            // multiplies colour by this same alpha.
            //
            // Nothing in this pass reads it back. It exists for L5:
            // `Blend::Over` needs to know what an input covers, and
            // before this the channel was written by nothing and held
            // the clear value forever. Colour is premultiplied by
            // coverage on the way out, which is what makes the mix's
            // `over` a multiply-add rather than a divide by an alpha
            // that is allowed to be zero.
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&l4.name),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: if weighted {
                    &weighted_targets
                } else {
                    &additive_target
                },
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Renderer {
            pipeline,
            uniforms,
            uniform_layout: shader.uniform_layout.clone(),
            scratch: UniformScratch::new(&shader.uniform_layout),
            uniform_bg,
            attr_bg,
            camera_bg: shader
                .camera_group
                .map(|g| (g, camera.bind_group().clone())),
            oit: weighted.then(|| Oit::new(device)),
            topology,
            param_names: l4.params.iter().map(|p| p.name.clone()).collect(),
            param_keys: crate::set::declared_keys(l4),
        }
    }

    pub(crate) fn is_fullscreen(&self) -> bool {
        self.topology == karakuri_ir::Topology::Fullscreen
    }

    /// What this node draws — see [`Renderer::topology`].
    pub(crate) fn topology(&self) -> karakuri_ir::Topology {
        self.topology
    }

    /// **The addressable keys, and there is deliberately no accessor for the
    /// other list.** `param_names` is read at the one place it means anything
    /// — this node's own uniform write — and handing it out would be handing
    /// out a list of names a `--param` cannot use.
    pub(crate) fn param_keys(&self) -> &[String] {
        &self.param_keys
    }

    /// Reallocation, so never from the render thread mid-frame. A node with no
    /// targets of its own has nothing to do here.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if let Some(oit) = &mut self.oit {
            oit.resize(device, width, height);
        }
    }

    /// This node's uniform block, from the grouping's view of the frame.
    ///
    /// **Nothing about the camera is packed here**, and that is the whole of
    /// what changed when it became a node: the matrix, the ray basis and
    /// `depth_range` used to be written from an `Orbit` the host owned, and are
    /// now derived on the GPU into a buffer this node binds. What is left is the
    /// clock, the salt, the canvas, and this node's own params.
    pub(crate) fn write_uniforms(&mut self, queue: &wgpu::Queue, view: &View<'_>) {
        let fullscreen = self.is_fullscreen();
        let mut p = self.scratch.pack(&self.uniform_layout);
        p.f32("t", view.t)
            .f32("beats", view.beats)
            .u32("seed_salt", view.seed_salt);
        // Two shapes of uniform, because the two shaders need different things:
        // a per-element one expands sprites and strokes and needs the viewport
        // in pixels; a fullscreen one has no primitive to size. Writing a field
        // the layout does not declare is a panic in the packer, which is the
        // right way round — it means the two halves cannot drift.
        if !fullscreen {
            p.vec2("viewport", view.viewport);
        }
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

    /// **Two shapes of draw, and the geometry is the same in both.** Under
    /// `blend additive` the pass writes straight into `target`. Under `weighted`
    /// it writes into two accumulation targets instead, and a second pass
    /// resolves those into `target` — which comes out holding exactly what the
    /// additive path would have left there, colour premultiplied by coverage and
    /// coverage in alpha, so nothing downstream can tell which mode ran.
    ///
    /// **`first` says whether this node is the first to reach `target`.** Several
    /// renderers over one geometry run in order over the one attachment — the
    /// first clears it, the rest load what is there — which is what makes a
    /// stack of them *overdraw* rather than compositing. It costs one target
    /// however many nodes there are; a target apiece is what an L5 is for. Both
    /// blend modes already know how to meet what is under them: `additive`'s
    /// blend state accumulates into whatever is there, and the weighted resolve
    /// composites `over` — see [`crate::oit`].
    pub(crate) fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        parity: usize,
        counts_buf: &wgpu::Buffer,
        first: bool,
    ) {
        if let Some(oit) = &self.oit {
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("L4 (weighted)"),
                    color_attachments: &oit.attachments(),
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                self.record(&mut pass, parity, counts_buf);
            }
            oit.resolve_into(encoder, target, first);
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("L4"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // `TRANSPARENT`, not `BLACK`: alpha in this target is
                    // coverage, accumulated by the blend state the pipeline
                    // carries, and it has to start at "nothing drew here".
                    // `BLACK` is opaque black and would hand the L5 mix a slot
                    // that covers the frame before a single sprite has run.
                    load: if first {
                        wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.record(&mut pass, parity, counts_buf);
    }

    /// The draw itself, into whatever pass the caller opened.
    ///
    /// One copy for the same reason [`Renderer::draw`] is one copy: the blend
    /// mode changes what the fragments are written *into* and nothing about
    /// which primitives run, so two transcriptions of "a triangle, or every
    /// element indirectly" is how the two modes would come to draw different
    /// geometry.
    fn record(&self, pass: &mut wgpu::RenderPass<'_>, parity: usize, counts_buf: &wgpu::Buffer) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(group::UNIFORMS, &self.uniform_bg, &[]);
        if let Some((at, bg)) = &self.camera_bg {
            pass.set_bind_group(*at, bg, &[]);
        }
        if self.is_fullscreen() {
            // Three vertices, one instance, and no indirect read: the count
            // is a property of the shape rather than of how many elements
            // survived. See `FULLSCREEN_VS` for why it is a triangle and
            // not a quad.
            pass.draw(0..3, 0..1);
            return;
        }
        pass.set_bind_group(group::ATTRS, &self.attr_bg[parity], &[]);
        // The instance count is GPU state now, so this is indirect even
        // for a static procedure whose count the host does know — one
        // render path rather than two, at the cost of one buffer read
        // the command processor was going to do anyway.
        pass.draw_indirect(counts_buf, counts::DRAW);
    }
}
