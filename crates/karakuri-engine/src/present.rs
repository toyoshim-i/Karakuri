//! The final output pass.
//!
//! Owns the linear HDR target that every `VideoSource` renders into, and the
//! one conversion to sRGB. Keeping both here is what makes "sRGB encoding
//! happens once, at final output" checkable rather than aspirational: there is
//! exactly one call site.
//!
//! Tone mapping — the transfer from unbounded linear HDR to something
//! displayable — lives in the same fragment shader, immediately before that
//! encode, for the identical reason: one call site, checkable rather than
//! aspirational. It is not a third thing bolted on beside sRGB encoding; the
//! two are the same pass because sRGB encoding is only ever correct on values
//! a tone mapper already brought into range.
//!
//! The operator is a uniform, not a shader variant. `set_tonemap` is a
//! `queue.write_buffer`, so switching Clamp for ACES for AgX live is the same
//! kind of change as a parameter move — no pipeline rebuild, no frame-boundary
//! swap, nothing for `HotSwap` to know about.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// The transfer from unbounded linear HDR to a displayable `[0, 1]`. Compared
/// side by side by `examples/tonemap_compare.rs`, on the material this project
/// actually renders: additive, saturated point sprites, which push channels far
/// past 1.0 unevenly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TonemapOp {
    /// `min(c, 1.0)`. Not a tone mapper — the control that shows what the
    /// other three are fixing. Kept so the comparison is honest, and kept as
    /// the default so a caller that never calls `set_tonemap` sees exactly
    /// what shipped before this module existed.
    Clamp = 0,
    /// Reinhard with a white point: a chosen input level maps to exactly 1.0
    /// rather than every level asymptotically approaching it. Per channel, so
    /// it shifts hue as it compresses.
    Reinhard = 1,
    /// The Narkowicz fitted approximation to the ACES reference rendering
    /// transform. Per channel, same hue-shifting consequence as Reinhard.
    Aces = 2,
    /// AgX, approximated: compresses in a rotated log-encoded working space
    /// with one shared curve rather than per channel, which is what keeps an
    /// overdriven channel from running away from its neighbours.
    AgX = 3,
}

/// Matches `Tonemap` in `shaders/present.wgsl`. 16 bytes, so no trailing pad
/// is needed to satisfy the uniform address space's alignment rules.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct TonemapUniform {
    op: u32,
    exposure: f32,
    white_point: f32,
    _pad: u32,
}

impl TonemapUniform {
    /// What `Present::new` uploads before anyone calls `set_tonemap`.
    ///
    /// **ACES**, chosen by looking at the four rendered side by side rather
    /// than from first principles — `examples/tonemap_compare.rs` is what
    /// produced the comparison, and it is worth rerunning rather than
    /// re-arguing. On this material, dense additive point sprites, ACES keeps
    /// colour strong and contrast defined where Reinhard flattens and AgX
    /// desaturates the whole frame on its way to a soft highlight.
    ///
    /// `Clamp` is what shipped before a tone mapper existed and is kept as the
    /// control, not as a fallback: it is the operator that turns a bright core
    /// into a featureless white disc, which is the thing being fixed.
    ///
    /// A default is only what happens when nobody chooses. Switching is a
    /// `queue.write_buffer`, so an operator can change mid-set.
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
    /// The operator, exposure, and (Reinhard's) white point, packed together
    /// because they are read together — see `shaders/present.wgsl`'s `Tonemap`
    /// struct. Exposure here is the operator's own control, distinct from a
    /// procedure's `param exposure` (how bright that material is) and from
    /// L5's future per-Set gain (how it balances against the others).
    tonemap: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
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
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
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
            multiview: None,
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

    /// Reallocation, so never from the render thread mid-frame.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if (width, height) == (self.width, self.height) {
            return;
        }
        let (hdr, view, bind_group) =
            Self::make_target(device, &self.layout, &self.sampler, &self.tonemap, width, height);
        self.hdr = hdr;
        self.hdr_view = view;
        self.bind_group = bind_group;
        self.width = width;
        self.height = height;
    }

    /// Selects the tone-mapping operator and its exposure, and — for
    /// `TonemapOp::Reinhard` only — the input level that maps to exactly 1.0.
    /// `white_point` is ignored by the other three operators; pass whatever is
    /// convenient, since a uniform write is all this costs regardless.
    ///
    /// A `queue.write_buffer`, not a pipeline rebuild: the fragment shader
    /// already contains all four operators and branches on this value, which
    /// is what "an operator change must not be a recompile" means in practice.
    /// Safe to call from the render thread for the same reason every other
    /// per-frame uniform write in this codebase is — it writes into storage
    /// sized once, at construction.
    pub fn set_tonemap(&self, queue: &wgpu::Queue, op: TonemapOp, exposure: f32, white_point: f32) {
        let uniform = TonemapUniform {
            op: op as u32,
            exposure,
            white_point,
            _pad: 0,
        };
        queue.write_buffer(&self.tonemap, 0, bytemuck::bytes_of(&uniform));
    }

    /// The linear HDR target a `VideoSource` renders into.
    pub fn hdr_view(&self) -> &wgpu::TextureView {
        &self.hdr_view
    }

    pub fn hdr_texture(&self) -> &wgpu::Texture {
        &self.hdr
    }

    /// The canvas: what every `VideoSource` renders at, and what a deck must
    /// be resized to match. **Not the size of whatever this is drawn into** —
    /// see [`Present::draw`].
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Draws the canvas into `target`, which may be a different size and a
    /// different shape.
    ///
    /// `target_size` is the attachment's, in texels. When it matches the canvas
    /// — every offscreen render — the viewport is the whole attachment and this
    /// is what it always was. When it does not, the canvas is centred and
    /// scaled to fit, and the bars are the clear.
    ///
    /// **Fitted rather than stretched, and that is a decision about honesty.**
    /// A window is a preview of what leaves by some other route, so the one
    /// thing it must not do is disagree with that route about framing: a
    /// stretched preview puts the material somewhere it will not be. Bars are
    /// visible and mean something; a wrong aspect ratio is invisible and means
    /// the operator composes for the wrong frame.
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
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
        });
        let (x, y, w, h) = letterbox((self.width, self.height), target_size);
        pass.set_viewport(x, y, w, h, 0.0, 1.0);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The largest centred rectangle inside `target` with `canvas`'s aspect ratio,
/// as `(x, y, width, height)` in texels.
///
/// Free rather than a method, and public, because it is the whole of the
/// fitting decision and the only part of it that can be checked without a GPU.
///
/// The clamps are not defensive tidiness, and they are **not** there to keep a
/// render pass legal. wgpu validates a viewport against the device's texture
/// limits and not against the attachment, so a rectangle that hangs outside is
/// accepted and simply draws wrong — checked on this machine by submitting a
/// 256x256 viewport into a 64x64 attachment, which passed. Silence is the
/// reason to be exact here rather than a reason to relax.
///
/// What they fix is the arithmetic. The exact-fit axis computes as
/// `c * (t / c)`, which floating point does not promise is `t`: over
/// `c, t` in `1..=4096` that product exceeds `t` for 936k of the 16.7M pairs,
/// the tightest being `c = 21, t = 3` giving `3.0000002`. Clamping the extent
/// first and deriving the offset from the clamped extent keeps `x + w <= t`
/// true by construction.
///
/// The lower clamp is the same argument at the other end: a canvas fitted into
/// a window shaped nothing like it — 4096x1 into 1x4096 — scales to less than
/// one texel tall, and a sub-texel viewport draws *nothing*. One row of the
/// picture beats an empty preview and a puzzled operator.
pub fn letterbox(canvas: (u32, u32), target: (u32, u32)) -> (f32, f32, f32, f32) {
    let (cw, ch) = (canvas.0.max(1) as f32, canvas.1.max(1) as f32);
    let (tw, th) = (target.0.max(1) as f32, target.1.max(1) as f32);
    let scale = (tw / cw).min(th / ch);
    // `min` before `max`: the target is at least 1 in each axis, so the result
    // lands in `[1, t]` and `x + w = (t + w) / 2 <= t` still holds.
    let w = (cw * scale).min(tw).max(1.0);
    let h = (ch * scale).min(th).max(1.0);
    (((tw - w) * 0.5).max(0.0), ((th - h) * 0.5).max(0.0), w, h)
}
