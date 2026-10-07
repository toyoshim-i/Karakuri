//! A 2D affine resample between two HDR pictures of the same size.
//!
//! The WebXR stereo world uses it to carry each eye's picture onto a virtual
//! screen fixed a short way in front of the head, run the screen-space L5s
//! there, and carry the result back: an L5 knows only its picture's UV, so
//! running it on a shared screen gives both eyes the same effect at that
//! screen's depth rather than at infinity.

use crate::present::Present;

const SHADER: &str = r#"
struct Affine {
    // src_ndc = scale * dst_ndc + offset, per axis; NDC is y-up.
    scale: vec2<f32>,
    offset: vec2<f32>,
};
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
@group(0) @binding(2) var<uniform> affine: Affine;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> Out {
    let xy = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0;
    var out: Out;
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.ndc = xy;
    return out;
}

@fragment
fn fs(in: Out) -> @location(0) vec4<f32> {
    let s = affine.scale * in.ndc + affine.offset;
    let uv = vec2<f32>(s.x * 0.5 + 0.5, 0.5 - s.y * 0.5);
    return textureSampleLevel(src, smp, uv, 0.0);
}
"#;

/// The resample pipeline; each use is a [`PlaneWarp::bind`]ing of a source
/// picture and its own affine.
pub struct PlaneWarp {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

/// One source picture and the affine it is read through.
pub struct WarpBinding {
    bind_group: wgpu::BindGroup,
    affine: wgpu::Buffer,
}

impl WarpBinding {
    /// Sets the affine: a destination pixel at NDC `d` reads the source at
    /// `scale * d + offset`. Lands at the next submit.
    pub fn set(&self, queue: &wgpu::Queue, scale: [f32; 2], offset: [f32; 2]) {
        let words = [scale[0], scale[1], offset[0], offset[1]];
        queue.write_buffer(&self.affine, 0, bytemuck::cast_slice(&words));
    }
}

impl PlaneWarp {
    pub fn new(device: &wgpu::Device) -> PlaneWarp {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("plane warp"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("plane warp"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("plane warp"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("plane warp"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: Present::HDR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        // Clamped: where the other picture does not reach, its edge repeats.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("plane warp"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        PlaneWarp {
            pipeline,
            layout,
            sampler,
        }
    }

    /// A binding of `src` (an [`Present::HDR_FORMAT`] picture) with an
    /// identity affine.
    pub fn bind(&self, device: &wgpu::Device, src: &wgpu::TextureView) -> WarpBinding {
        use wgpu::util::DeviceExt;
        let affine = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("plane warp affine"),
            contents: bytemuck::cast_slice(&[1.0f32, 1.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("plane warp"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: affine.as_entire_binding(),
                },
            ],
        });
        WarpBinding { bind_group, affine }
    }

    /// Resamples `binding`'s source into all of `dst`.
    pub fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        binding: &WarpBinding,
        dst: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("plane warp"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: dst,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &binding.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
