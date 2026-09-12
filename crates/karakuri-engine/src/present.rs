//! Output presentation pass and final color conversion.
//!
//! Owns the linear HDR target that `VideoSource` instances render into, performs
//! tone mapping (Clamp, Reinhard, ACES, or AgX), and applies the single sRGB
//! conversion at final output.
//!
//! Tone mapping operator and parameters are updated via uniform buffer writes
//! rather than shader pipeline variants, allowing real-time parameter changes
//! without pipeline recreation.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::master::{Chain, Clock, Cut, MasterChain};

/// Transfer function from unbounded linear HDR to displayable `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TonemapOp {
    /// `min(c, 1.0)`. Clamps values exceeding 1.0 without tone compression.
    Clamp = 0,
    /// Reinhard operator with configurable white point.
    Reinhard = 1,
    /// Narkowicz approximation of the ACES reference rendering transform.
    Aces = 2,
    /// AgX approximation using logarithmic curve compression.
    AgX = 3,
}

/// Matches `Tonemap` in `shaders/present.wgsl`. Aligned to 16 bytes for uniform storage.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct TonemapUniform {
    op: u32,
    exposure: f32,
    white_point: f32,
    _pad: u32,
}

impl TonemapUniform {
    /// Returns the default tone-mapping configuration (ACES at exposure 1.0).
    fn default_op() -> TonemapUniform {
        TonemapUniform {
            op: TonemapOp::Aces as u32,
            exposure: 1.0,
            white_point: 1.0,
            _pad: 0,
        }
    }
}

pub struct Present {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    hdr: wgpu::Texture,
    hdr_view: wgpu::TextureView,
    /// Tone-mapping configuration buffer containing operator, exposure, and white point.
    tonemap: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// Master chain of L5 slots executed between composition and tone mapping.
    chain: MasterChain,
    width: u32,
    height: u32,
}

impl Present {
    pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Present {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("present"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/present.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("present"),
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
            label: Some("present"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("present"),
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
                targets: &[Some(surface_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("present"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let tonemap = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tonemap"),
            contents: bytemuck::bytes_of(&TonemapUniform::default_op()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let (hdr, hdr_view, bind_group) =
            Self::make_target(device, &layout, &sampler, &tonemap, width, height);

        Present {
            pipeline,
            layout,
            sampler,
            hdr,
            hdr_view,
            tonemap,
            bind_group,
            chain: MasterChain::new(device, width, height),
            width,
            height,
        }
    }

    fn make_target(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        tonemap: &wgpu::Buffer,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup) {
        let hdr = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hdr"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = hdr.create_view(&Default::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("present"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: tonemap.as_entire_binding(),
                },
            ],
        });
        (hdr, view, bind_group)
    }

    /// Resizes the HDR target and associated master chain passes to `(width, height)`.
    ///
    /// Reallocates GPU textures and updates the master chain dimensions. Must be
    /// called outside render passes.
    pub fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        if (width, height) == (self.width, self.height) {
            return;
        }
        let (hdr, view, bind_group) = Self::make_target(
            device,
            &self.layout,
            &self.sampler,
            &self.tonemap,
            width,
            height,
        );
        self.hdr = hdr;
        self.hdr_view = view;
        self.bind_group = bind_group;
        self.chain
            .resize(device, queue, &self.hdr_view, width, height);
        self.width = width;
        self.height = height;
    }

    /// Selects the tone-mapping operator, exposure, and white point.
    ///
    /// Writes the parameters into the uniform buffer via `queue.write_buffer`
    /// without rebuilding the shader pipeline. `white_point` is only used by
    /// `TonemapOp::Reinhard`.
    pub fn set_tonemap(&self, queue: &wgpu::Queue, op: TonemapOp, exposure: f32, white_point: f32) {
        let uniform = TonemapUniform {
            op: op as u32,
            exposure,
            white_point,
            _pad: 0,
        };
        queue.write_buffer(&self.tonemap, 0, bytemuck::bytes_of(&uniform));
    }

    /// Returns the linear HDR texture view rendered into and read by this pass.
    pub fn hdr_view(&self) -> &wgpu::TextureView {
        &self.hdr_view
    }

    /// Returns the texture view that composited frames should be written to.
    ///
    /// Returns the entry view of the master chain if active, or `hdr_view` if the
    /// chain is empty.
    pub fn mix_target(&self) -> &wgpu::TextureView {
        self.chain.entry().unwrap_or(&self.hdr_view)
    }

    /// Returns the number of slots in the running master chain.
    pub fn chain_len(&self) -> usize {
        self.chain.chain_len()
    }

    /// Returns the sequence of `(procedure_id, cut)` identifying each slot in the chain.
    pub fn chain_shape(&self) -> Vec<(String, Option<Cut>)> {
        self.chain.shape()
    }

    /// Returns specifications for all slots in the master chain.
    pub fn chain_spec(&self) -> Vec<crate::master::SlotSpec> {
        self.chain.spec()
    }

    /// Returns the cuts currently retained by the running master chain.
    pub fn chain_retained(&self) -> Vec<Cut> {
        self.chain.retained()
    }

    /// Returns the count of frame-sized intermediate targets allocated by the chain.
    pub fn chain_targets(&self) -> usize {
        self.chain.targets()
    }

    /// Returns the total estimated shader operations per fragment across the chain.
    pub fn chain_ops_per_fragment(&self) -> u32 {
        self.chain.ops_per_fragment()
    }

    /// Returns the bind group layout used to compile master chain slots.
    pub fn chain_layout(&self) -> &wgpu::BindGroupLayout {
        self.chain.layout()
    }

    /// Installs an ordered master chain, allocating required intermediate textures.
    pub fn set_chain(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, chain: Chain) {
        let Present {
            chain: master,
            hdr_view,
            ..
        } = self;
        master.set(device, queue, hdr_view, chain);
    }

    /// Updates running chain slot parameters in place without reallocating textures.
    ///
    /// Returns `false` if the provided shape does not match the active chain structure.
    pub fn set_chain_params(
        &mut self,
        queue: &wgpu::Queue,
        shape: &[(String, Option<Cut>)],
        params: &[std::collections::BTreeMap<String, f32>],
    ) -> bool {
        self.chain.set_params(queue, shape, params)
    }

    /// Updates the host clock uniform passed to master chain frame passes.
    pub fn set_chain_clock(&mut self, queue: &wgpu::Queue, clock: Clock) {
        self.chain.set_clock(queue, clock);
    }

    /// Records master chain render passes into `encoder`.
    pub fn draw_chain(&self, encoder: &mut wgpu::CommandEncoder) {
        self.chain.record(encoder, &self.hdr_view);
    }

    pub fn hdr_texture(&self) -> &wgpu::Texture {
        &self.hdr
    }

    /// Returns the canvas dimensions `(width, height)` in texels.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Draws the presentation pass into `target`, letterboxing to fit `target_size`.
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_size: (u32, u32),
    ) {
        self.draw_with_bind_group(encoder, &self.bind_group, target, target_size);
    }

    /// Creates a bind group mapping `source` through this tone-mapping pipeline.
    pub fn create_bind_group_for(
        &self,
        device: &wgpu::Device,
        source: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("present source"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.tonemap.as_entire_binding(),
                },
            ],
        })
    }

    /// Draws `bind_group`'s source into `target`, letterboxed to fit `target_size`.
    pub fn draw_with_bind_group(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        target: &wgpu::TextureView,
        target_size: (u32, u32),
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("present"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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
        let (x, y, w, h) = letterbox((self.width, self.height), target_size);
        pass.set_viewport(x, y, w, h, 0.0, 1.0);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Computes the largest centered rectangle inside `target` matching `canvas` aspect ratio.
///
/// Returns `(x, y, width, height)` in texels. Clamps extents and offsets so that
/// `x + width <= target.width` and `y + height <= target.height`.
pub fn letterbox(canvas: (u32, u32), target: (u32, u32)) -> (f32, f32, f32, f32) {
    let (cw, ch) = (canvas.0.max(1) as f32, canvas.1.max(1) as f32);
    let (tw, th) = (target.0.max(1) as f32, target.1.max(1) as f32);
    let scale = (tw / cw).min(th / ch);
    let w = (cw * scale).min(tw).max(1.0);
    let h = (ch * scale).min(th).max(1.0);
    (((tw - w) * 0.5).max(0.0), ((th - h) * 0.5).max(0.0), w, h)
}
