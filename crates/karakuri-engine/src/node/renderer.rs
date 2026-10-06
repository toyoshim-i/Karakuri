//! The L4 node: `(Geometry, Camera) -> Texture`.

use karakuri_codegen::generate_l4_for_topology;
use karakuri_codegen::layout::{binding, counts, group, UniformLayout};
use karakuri_ir::typed::Checked;

use super::{Camera, Geometry, View};
use crate::oit::Oit;
use crate::uniforms::UniformScratch;

/// An L4 rendering node that draws geometry to a target texture.
pub(crate) struct Renderer {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    uniform_layout: UniformLayout,
    scratch: UniformScratch,
    uniform_bg: wgpu::BindGroup,
    /// Attribute bind groups indexed by input geometry parity.
    attr_bg: [wgpu::BindGroup; 2],
    /// Camera bind group and layout index, if the shader samples a camera.
    camera_bg: Option<(u32, wgpu::BindGroup)>,
    /// Order-independent transparency resources for weighted blending.
    oit: Option<Oit>,
    /// Drawing topology declared by this procedure.
    topology: karakuri_ir::Topology,
    /// Whether this renderer configures depth testing / writing.
    has_depth_stencil: bool,
    /// Parameter names declared in procedure definition.
    param_names: Vec<String>,
    /// Addressable parameter keys (e.g. `color.r`, `size.x`).
    param_keys: Vec<String>,
}

impl Renderer {
    /// Compiles and binds an L4 renderer node for the specified geometry and camera.
    pub(crate) fn build(
        device: &wgpu::Device,
        l4: &Checked,
        geometry: &Geometry<'_>,
        camera: &Camera,
        fields: karakuri_codegen::Bound<'_>,
        has_depth: bool,
        depth_test: bool,
    ) -> Renderer {
        let inferred = l4.topology.unwrap_or(karakuri_ir::Topology::Points);
        let topology = if inferred == karakuri_ir::Topology::Points && geometry.topology.is_shared()
        {
            geometry.topology
        } else {
            inferred
        };
        let fullscreen = topology == karakuri_ir::Topology::Fullscreen;
        let weighted = l4.blend == Some(karakuri_ir::Blend::Weighted);
        let is_opaque = l4.blend == Some(karakuri_ir::Blend::Opaque);

        let shader = generate_l4_for_topology(l4, geometry.layout, fields, topology);
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
        // Bind both element and alive buffers; dead elements are skipped in the vertex shader.
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

        // Fullscreen shaders consume no attributes and bind only uniforms.
        let mut groups: Vec<Option<&wgpu::BindGroupLayout>> = if fullscreen {
            vec![Some(&uniform_bgl)]
        } else {
            vec![Some(&uniform_bgl), Some(&attr_bgl)]
        };
        // Verify camera bind group index immediately follows preceding groups.
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
        let (has_depth_stencil, depth_stencil) = if is_opaque {
            (
                true,
                Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
            )
        } else if has_depth && depth_test {
            (
                true,
                Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
            )
        } else {
            (false, None)
        };

        let opaque_target = [Some(wgpu::ColorTargetState {
            format: crate::present::Present::HDR_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let weighted_targets = crate::oit::colour_targets();
        let additive_target = [Some(wgpu::ColorTargetState {
            format: crate::present::Present::HDR_FORMAT,
            // Additive blending: color is emissive and accumulates via addition,
            // while alpha tracks coverage: 1 - prod(1 - a_i).
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
                targets: if is_opaque {
                    &opaque_target
                } else if weighted {
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
            depth_stencil,
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
            has_depth_stencil,
            param_names: l4.params.iter().map(|p| p.name.clone()).collect(),
            param_keys: crate::set::declared_keys(l4),
        }
    }

    /// Returns true if this renderer uses fullscreen topology.
    pub(crate) fn is_fullscreen(&self) -> bool {
        self.topology == karakuri_ir::Topology::Fullscreen
    }

    /// Returns the draw topology used by this renderer.
    pub(crate) fn topology(&self) -> karakuri_ir::Topology {
        self.topology
    }

    /// Returns addressable parameter keys for this node.
    pub(crate) fn param_keys(&self) -> &[String] {
        &self.param_keys
    }

    /// Resizes OIT accumulation targets if present.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if let Some(oit) = &mut self.oit {
            oit.resize(device, width, height);
        }
    }

    /// Writes this node's uniform block using the frame view.
    pub(crate) fn write_uniforms(&mut self, queue: &wgpu::Queue, view: &View<'_>) {
        let fullscreen = self.is_fullscreen();
        let mut p = self.scratch.pack(&self.uniform_layout);
        p.f32("t", view.t)
            .f32("beats", view.beats)
            .u32("seed_salt", view.seed_salt);
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
        super::write_field_params(
            &mut p,
            &self.uniform_layout,
            view.field_params,
            view.field_value,
        );
        super::write_source_slots(&mut p, &self.uniform_layout, view.source_value);
        queue.write_buffer(&self.uniforms, 0, p.finish());
    }

    /// Renders geometry into `target`.
    ///
    /// Under weighted blending, renders to OIT accumulation targets and resolves
    /// into `target`. `first` indicates whether `target` should be cleared to transparent.
    /// Renders geometry into `target`.
    pub(crate) fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        parity: usize,
        counts_buf: &wgpu::Buffer,
        first: bool,
    ) {
        self.draw_viewport(encoder, target, depth_view, parity, counts_buf, first, None);
    }

    /// Renders geometry into `target` constrained to an optional viewport rectangle `(x, y, width, height)`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_viewport(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        parity: usize,
        counts_buf: &wgpu::Buffer,
        first: bool,
        viewport: Option<(f32, f32, f32, f32)>,
    ) {
        let depth_stencil_attachment = if self.has_depth_stencil {
            let view = depth_view.expect("depth view required when renderer has depth stencil");
            Some(wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: if first {
                        wgpu::LoadOp::Clear(1.0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            })
        } else {
            None
        };

        if let Some(oit) = &self.oit {
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("L4 (weighted)"),
                    color_attachments: &oit.attachments(),
                    depth_stencil_attachment: depth_stencil_attachment.as_ref().map(|d| {
                        wgpu::RenderPassDepthStencilAttachment {
                            view: d.view,
                            depth_ops: Some(wgpu::Operations {
                                load: if first {
                                    wgpu::LoadOp::Clear(1.0)
                                } else {
                                    wgpu::LoadOp::Load
                                },
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                if let Some((x, y, w, h)) = viewport {
                    pass.set_viewport(x, y, w, h, 0.0, 1.0);
                }
                self.record(&mut pass, parity, counts_buf);
            }
            oit.resolve_into(encoder, target, first, viewport);
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("L4"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: if first {
                        wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some((x, y, w, h)) = viewport {
            pass.set_viewport(x, y, w, h, 0.0, 1.0);
        }
        self.record(&mut pass, parity, counts_buf);
    }

    /// Records draw commands into the given render pass.
    fn record(&self, pass: &mut wgpu::RenderPass<'_>, parity: usize, counts_buf: &wgpu::Buffer) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(group::UNIFORMS, &self.uniform_bg, &[]);
        if let Some((at, bg)) = &self.camera_bg {
            pass.set_bind_group(*at, bg, &[]);
        }
        if self.is_fullscreen() {
            // Fullscreen quad rendered as a single triangle covering NDC (-1..1).
            pass.draw(0..3, 0..1);
            return;
        }
        pass.set_bind_group(group::ATTRS, &self.attr_bg[parity], &[]);
        // Indirect draw using element counts calculated upstream.
        pass.draw_indirect(counts_buf, counts::DRAW);
    }
}
