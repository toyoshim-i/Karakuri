//! The machinery behind `blend weighted`: two accumulation targets and the
//! pass that folds them back into one.
//!
//! # Why two targets
//!
//! Weighted blended OIT needs three running quantities per texel and a slot
//! target has room for two. `sum(c * a * w)` and `sum(a * w)` fit in one
//! `Rgba16Float` — three channels and the alpha — and `prod(1 - a)` needs a
//! second attachment of its own, because it composes by multiplication where
//! the others compose by addition, and a colour attachment has one blend state.
//!
//! # Why the Set owns them and not the deck
//!
//! One pair shared across every slot would be smaller — the passes are
//! sequential, so no two Sets are accumulating at once — and it would have to
//! arrive through [`crate::video_source::VideoSource::render`], which takes one
//! target and nothing else. That interface exists so that a second
//! implementation is an addition rather than a change; threading one
//! implementation's scratch buffers through it would make the interface
//! describe how a `Set` happens to draw. The cost of keeping it clean is
//! `10` bytes a texel per *weighted* slot — nothing at all for the additive
//! ones, which is most of them.
//!
//! # What it costs
//!
//! At 1280x720: 7.03 MB of accumulation plus 1.76 MB of revealage, so 8.79 MB
//! for each *renderer* that declares `weighted`, on top of the 7.03 MB slot
//! target every slot has. A deck of four, one weighted renderer apiece, is 63 MB
//! of render target — against 28 MB for four additive ones.
//!
//! **Per renderer, and a slot may hold several.** A Set draws with a list of L4
//! nodes over one geometry and each weighted one owns its own pair, so a stack
//! of three weighted renderers in one slot is 26 MB rather than 8.79. Sharing
//! one pair across a stack is available — the passes are sequential, so no two
//! nodes accumulate at once — and is the same trade as sharing across slots
//! below: it would put one implementation's scratch buffers into an interface
//! that is deliberately about targets.
//!
//! **Per Set and not per slot, which is the other number that surprises.** A
//! [`HotSwap`](crate::swap::HotSwap) holds the live Set, the candidate on trial
//! behind it, and up to `GRAVEYARD_CAPACITY` more waiting for the GPU to be done
//! with them — every one of which owns its own pair while it exists. A weighted
//! slot mid-swap is 17.6 MB and a `--watch` session churning builds can be
//! several times that before the graveyard drains. `swap::measure` also resizes
//! a Set to the probe resolution and back, so each weighted build allocates its
//! pair twice on the way in.
//!
//! Worth knowing before a fifth slot is ever considered; not worth an allocator.

use crate::present::Present;

/// `sum(c * a * w)` in rgb, `sum(a * w)` in alpha. The same format as a slot
/// target, and for the same reason: the values are unbounded linear HDR.
const ACCUM_FORMAT: wgpu::TextureFormat = Present::HDR_FORMAT;

/// `prod(1 - a)`, one channel.
///
/// **`R16Float` rather than the `R8Unorm` the published technique uses.** A
/// product of many terms below 1 falls away fast, and 8 bits of it quantises
/// coverage to 1/255 — visible as banding exactly where soft material is
/// thinnest, which is the material this mode is for. Two bytes a texel is the
/// cheaper half of what this already costs.
const REVEAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

/// What the L4 pipeline's fragment stage writes to, under `blend weighted`.
///
/// **Two attachments, two blend states, and the second one is the whole trick.**
/// The accumulation adds, which is ordinary. The revealage is `dst * (1 - src)`
/// with no source term at all — the fragment's own contribution is discarded
/// and the destination is scaled — so what the target holds after *n* fragments
/// is `prod(1 - a_i)` regardless of the order they arrived in.
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
                // The same, and it has to be spelled out: the alpha channel here
                // is `sum(a * w)`, a fourth accumulator rather than a coverage.
                // Coverage is the *other* target's business under this mode.
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
                // `OneMinusSrc`, not `OneMinusSrcAlpha`: this target has one
                // channel and the fragment writes the opacity into it, so the
                // factor has to be taken from the colour and not from an alpha
                // that does not exist.
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

/// The accumulation targets and the pass that resolves them.
pub(crate) struct Oit {
    resolve: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    accum: wgpu::TextureView,
    reveal: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    size: (u32, u32),
}

impl Oit {
    /// Built with the Set, at 1×1.
    ///
    /// A Set comes up before anyone has told it what size it is —
    /// `Set::build` leaves the viewport at 1×1 and a caller's `resize` is what
    /// sets it — so allocating here at the eventual size is not available. One
    /// texel of each is what the pipeline needs to exist against, and the first
    /// `resize` replaces both.
    pub(crate) fn new(device: &wgpu::Device) -> Oit {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("oit resolve"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/oit_resolve.wgsl").into()),
        });
        let texture = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                // Not filterable, for `composite.wgsl`'s reason: the resolve
                // loads the texel under the fragment. No sampler is bound here
                // at all.
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
                // **`over`, premultiplied**, and the same state whether this
                // node is the first to reach the target or the fifth. The
                // resolve emits colour already multiplied by coverage with
                // coverage in alpha, so `src + dst * (1 - src.a)` is the
                // composite — and onto the transparent black the first node
                // clears to, `over` gives back exactly `src`. One blend state
                // rather than two paths that have to agree about the first one.
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

    /// Reallocation, so never from the render thread mid-frame — the same terms
    /// as [`Present::resize`].
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

    /// The L4 pass's attachments.
    ///
    /// **The revealage clears to white and the accumulation to nothing.** One is
    /// a product and one is a sum, so their identities differ, and a revealage
    /// cleared to black would mean "every texel is already fully hidden" —
    /// which resolves to an empty frame however much material drew.
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

    /// Draws the resolve into `target`, which is the slot target the additive
    /// path would have written directly.
    ///
    /// `first` says whether this node is the first to reach that target. The
    /// first clears it and the rest load what is already there — see
    /// [`Renderer::draw`](crate::node::Renderer::draw), which makes the same
    /// choice for the same reason.
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
                    // The triangle covers every texel, so the clear is a
                    // formality on an immediate-mode backend and a saved read
                    // on a tiler — for the first node. For any later one the
                    // load is not a formality at all: it is what the `over`
                    // above composites against.
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
    // The textures themselves are dropped here and the views keep them alive:
    // nothing outside this module ever needs to name one, and a bind group
    // holds its views.
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
