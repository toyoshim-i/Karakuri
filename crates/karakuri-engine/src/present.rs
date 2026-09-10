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

use crate::master::{Chain, Clock, Cut, MasterChain};

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
    /// struct.
    ///
    /// **Exposure here is the level going into the transfer, and it is the
    /// fourth thing in this engine called a level.** It is not a procedure's
    /// `param exposure` (how bright that material is), not
    /// `Deck::set_gain`'s per-slot L5 gain (how one Set balances against the
    /// others), and not `Deck::set_out`, the master out — which is the same
    /// arithmetic as this one applied at the other end of the master chain,
    /// where the composited frame is *written* rather than read. That
    /// separation is
    /// `docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md`,
    /// and until the chain between them has an effect in it the two are
    /// indistinguishable in the picture.
    tonemap: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// **The master chain**, which is the ordered list of L5 slots between the
    /// mix's write and this pass's read — see [`crate::master`].
    ///
    /// **Here because this module already owns every frame-sized target
    /// between the fold and the surface**, and resizing them is one call: a
    /// chain owned by the program would have to be threaded through
    /// [`crate::frame::compose`] and every one of its callers, and would be a
    /// second thing to remember to resize beside `hdr`. It does not weaken
    /// what this module claims: the chain adds no encode and no transfer, so
    /// *sRGB is encoded once, here* is the same sentence it was.
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

    /// Reallocation, so never from the render thread mid-frame.
    ///
    /// **The queue is here because the chain's slots carry the frame's size in
    /// their uniforms**: `frame_step` converts a fraction of the frame's height
    /// into the coordinates `tap` takes, and a slot resized without that write
    /// would displace by the old frame's number.
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
        // The chain's targets are frame-sized for the same reason `hdr` is, so
        // they move with it and there is one call rather than two to forget.
        //
        // **After the target above is replaced, never before.** The chain binds
        // this view to copy the `exit` cut out of, so a chain resized first
        // would hold last size's frame — which wgpu accepts and draws wrong.
        self.chain
            .resize(device, queue, &self.hdr_view, width, height);
        self.width = width;
        self.height = height;
    }

    /// Selects the tone-mapping operator and its exposure, and — for
    /// `TonemapOp::Reinhard` only — the input level that maps to exactly 1.0.
    ///
    /// **`exposure` is the level at this pass's input**, which is the far end
    /// of the master chain from `Deck::set_out`'s. Anything the chain does to
    /// the frame has already happened when this multiply lands.
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

    /// The linear HDR target a `VideoSource` renders into, and what this pass
    /// reads.
    ///
    /// **Not necessarily where the mix writes** — see [`Present::mix_target`].
    /// With a master chain running, the mix writes into the chain's entry and
    /// the chain's last pass writes here.
    pub fn hdr_view(&self) -> &wgpu::TextureView {
        &self.hdr_view
    }

    /// **Where the composited frame is written**: the master chain's entry
    /// when the chain runs, and [`Present::hdr_view`] when it does not.
    ///
    /// The two are the same view for a chain at zero, which is what makes the
    /// default look bit-identical to the one this program drew before the
    /// chain existed: there is no pass in the way, not a pass that does
    /// nothing.
    pub fn mix_target(&self) -> &wgpu::TextureView {
        self.chain.entry().unwrap_or(&self.hdr_view)
    }

    /// **How many slots the chain is running**, and **what identifies them**.
    ///
    /// The shape is the pair a slot is recognised by — its procedure's content
    /// address and its cut — which is what a caller compares a record against
    /// before deciding whether it has a parameter move or a list to build. See
    /// [`Present::set_chain_params`].
    pub fn chain_len(&self) -> usize {
        self.chain.chain_len()
    }

    /// See [`Present::chain_len`].
    pub fn chain_shape(&self) -> Vec<(String, Option<Cut>)> {
        self.chain.shape()
    }

    /// **What the master chain is**, described the way a record carries it.
    ///
    /// One value, read back the way `Deck::out` is: this is the one writer of
    /// the chain (ADR-0317's *applied and not stored*), so the Master bay's
    /// rows draw from it and `karakuri-operation-record` completes a record
    /// from it, and there is nowhere else the two could disagree.
    pub fn chain_spec(&self) -> Vec<crate::master::SlotSpec> {
        self.chain.spec()
    }

    /// **Which cuts the running chain is holding**, and **how many frame-sized
    /// targets it has taken** — the entry, the ping-pong pair and the
    /// retentions.
    ///
    /// Read back rather than computed by a caller because it is what P-0091 is
    /// met by here: a retention is allocated only where a slot's answer names
    /// one, at most two ever, and an empty chain takes nothing.
    pub fn chain_retained(&self) -> Vec<Cut> {
        self.chain.retained()
    }

    /// See [`Present::chain_retained`].
    pub fn chain_targets(&self) -> usize {
        self.chain.targets()
    }

    /// **What the running chain costs per texel**, the sum over its slots —
    /// the number a governor spends against the frame's area beside the decks
    /// rather than against any one of them (ADR-0340 §5).
    pub fn chain_ops_per_fragment(&self) -> u32 {
        self.chain.ops_per_fragment()
    }

    /// **The chain's bind group layout**, so a slot can be compiled against it
    /// away from the render thread.
    ///
    /// Handed out rather than made twice: a bind group names the layout object
    /// it was created with, and a slot built against a second one would be a
    /// pipeline this `Present` cannot bind its own targets into. Cloning the
    /// handle is what lets [`crate::master::Slot::build`] run on a worker.
    pub fn chain_layout(&self) -> &wgpu::BindGroupLayout {
        self.chain.layout()
    }

    /// **Install a master chain**, which is a build and is spelled as one.
    ///
    /// This is where the chain's memory is taken and given back: the entry, at
    /// most two targets to ping-pong between, and one per retained cut some
    /// slot asked for. It is the shape [`Present::resize`] has and it belongs
    /// at a frame boundary for the same reason — never from the render thread
    /// mid-frame (P-0091, ADR-0033).
    ///
    /// **A parameter move does not come through here.** See
    /// [`Present::set_chain_params`], which is a `queue.write_buffer` per slot
    /// and allocates nothing.
    pub fn set_chain(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, chain: Chain) {
        let Present {
            chain: master,
            hdr_view,
            ..
        } = self;
        master.set(device, queue, hdr_view, chain);
    }

    /// **Move the running chain's parameters and nothing else**, or answer
    /// `false` because the list itself has changed.
    ///
    /// `Record::MasterChain` is written whole — a stream that moved one slot
    /// without saying where the others stood describes a chain a replay cannot
    /// put back — so the common case of applying one is a record whose *shape*
    /// is the shape already running with one number different. This is what
    /// keeps that from costing an allocation, and the caller that gets `false`
    /// resolves the addresses and builds a list instead.
    pub fn set_chain_params(
        &mut self,
        queue: &wgpu::Queue,
        shape: &[(String, Option<Cut>)],
        params: &[std::collections::BTreeMap<String, f32>],
    ) -> bool {
        self.chain.set_params(queue, shape, params)
    }

    /// **The clock the chain's `frame` blocks read**, which is the host's
    /// answer and not the engine's — see [`crate::master::Clock`]. One
    /// `queue.write_buffer` per slot into storage sized at build, so it is safe
    /// on the render thread for the reason [`Present::set_tonemap`] is.
    pub fn set_chain_clock(&mut self, queue: &wgpu::Queue, clock: Clock) {
        self.chain.set_clock(queue, clock);
    }

    /// **Record the master chain into this frame's encoder**, between the
    /// mix's write and this pass's read.
    ///
    /// Nothing at all for an empty chain. Called by [`crate::frame::compose`]
    /// immediately after `Frame::render` and before any sink is drawn into,
    /// which is what puts the chain in linear HDR and upstream of the one tone
    /// map.
    pub fn draw_chain(&self, encoder: &mut wgpu::CommandEncoder) {
        self.chain.record(encoder, &self.hdr_view);
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
