//! Weighted blended order-independent transparency (OIT).
//!
//! Implements two-target accumulation (color/weight and revealage) and resolves
//! them onto HDR slot render targets using a full-screen pass.

use crate::present::Present;

/// Accumulation texture format: RGB holds `sum(c * a * w)`, Alpha holds `sum(a * w)`.
const ACCUM_FORMAT: wgpu::TextureFormat = Present::HDR_FORMAT;

/// Revealage texture format storing `prod(1 - a)`.
const REVEAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

/// Returns color target states for the weighted transparency rendering pass.
///
/// Target 0 accumulates weighted color and alpha.
/// Target 1 computes revealage using multiplicative blending (`dst * (1 - src)`).
pub(crate) fn colour_targets() -> [Option<wgpu::ColorTargetState>; 2] {
    [
        Some(wgpu::ColorTargetState {
            format: ACCUM_FORMAT,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                // Target 0 alpha accumulates sum(a * w) rather than standard coverage.
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            write_mask: wgpu::ColorWrites::ALL,
        }),
        Some(wgpu::ColorTargetState {
            format: REVEAL_FORMAT,
            blend: Some(wgpu::BlendState {
                // OneMinusSrc scales destination by fragment opacity.
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::OneMinusSrc,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::OneMinusSrc,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            write_mask: wgpu::ColorWrites::ALL,
        }),
    ]
}

/// Manages OIT accumulation targets and the full-screen resolve pass.
pub(crate) struct Oit {
    resolve: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    accum: wgpu::TextureView,
    reveal: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    size: (u32, u32),
}

impl Oit {
    /// Creates a new OIT resolver initialized with 1x1 placeholder targets.
    pub(crate) fn new(device: &wgpu::Device) -> Oit {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("oit resolve"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/oit_resolve.wgsl").into()),
        });
        let texture = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                // Unfiltered texture bindings for full-screen resolve.
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("oit resolve"),
            entries: &[texture(0), texture(1)],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("oit resolve"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let resolve = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("oit resolve"),
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
                // Premultiplied over blend: resolve emits color * coverage, composite computes src + dst * (1 - src.a).
                targets: &[Some(wgpu::ColorTargetState {
                    format: Present::HDR_FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let (accum, reveal, bind_group) = make_targets(device, &layout, 1, 1);
        Oit {
            resolve,
            layout,
            accum,
            reveal,
            bind_group,
            size: (1, 1),
        }
    }

    /// Reallocates accumulation and revealage targets for the given dimensions.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let size = (width.max(1), height.max(1));
        if size == self.size {
            return;
        }
        let (accum, reveal, bind_group) = make_targets(device, &self.layout, size.0, size.1);
        self.accum = accum;
        self.reveal = reveal;
        self.bind_group = bind_group;
        self.size = size;
    }

    /// Returns the render pass color attachments for accumulation and revealage.
    ///
    /// Accumulation clears to transparent black; revealage clears to 1.0 (white).
    pub(crate) fn attachments(&self) -> [Option<wgpu::RenderPassColorAttachment<'_>>; 2] {
        [
            Some(wgpu::RenderPassColorAttachment {
                view: &self.accum,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.reveal,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            }),
        ]
    }

    /// Resolves accumulated OIT buffers into the destination render target.
    ///
    /// When `first` is true, the destination target is cleared before composite.
    pub(crate) fn resolve_into(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        first: bool,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("oit resolve"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Clear target on first pass; load existing content on subsequent passes.
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
        pass.set_pipeline(&self.resolve);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn make_targets(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
) -> (wgpu::TextureView, wgpu::TextureView, wgpu::BindGroup) {
    let make = |label: &str, format: wgpu::TextureFormat| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default())
    };
    // Dropping parent textures; TextureViews retain underlying GPU resources.
    let accum = make("oit accum", ACCUM_FORMAT);
    let reveal = make("oit reveal", REVEAL_FORMAT);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("oit resolve"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&accum),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&reveal),
            },
        ],
    });
    (accum, reveal, bind_group)
}
