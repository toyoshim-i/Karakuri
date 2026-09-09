//! The master chain: three fixed passes between the mix and the tone map.
//!
//! # What it is, and what it is not
//!
//! **Three built-in presets, always loaded, in one order** — feedback, then
//! bloom, then rgb shift — each with parameters a surface can move and none
//! that adds, removes or reorders a pass. It is not a writable L5, it is not a
//! node kind, and nothing in `docs/ir-spec.md` grows for it:
//! `docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md`
//! is where that was decided and what it leaves open.
//!
//! # Where it sits
//!
//! Between the two multiplications ADR-0224 separated. [`crate::mix`] applies
//! `out` where it **writes** the composited frame, this chain reads that frame,
//! and [`crate::present`] applies `exposure` where the tone mapper **reads**
//! what this chain wrote. That is the whole reason those are two levels: a
//! feedback trail fed at half level decays from half.
//!
//! Everything here is linear HDR (`Rgba16Float`), unclamped, upstream of the
//! one tone map and the one sRGB encode
//! (`docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md`).
//! Nothing in this module encodes anything, so P-0064's *one call site* is the
//! same one call site.
//!
//! # What it costs, before it is paid
//!
//! **Memory, always**: four frame-sized `Rgba16Float` targets — two the passes
//! ping-pong between, one bloom's blur lands in, and one holding the retained
//! frame. 8 bytes a texel, so 7.03 MB each and **28.1 MB at 1280x720**, which
//! is what a full deck of four slots costs and is allocated at build and at
//! resize, never when a parameter moves. Allocating them the first time an
//! amount leaves zero would be cheaper on a machine nobody turns the chain up
//! on and would put an unbounded stall on whichever frame that was
//! (`docs/principles/0091-cost-is-known-before-it-is-paid.md`).
//!
//! **Time, per pass, and only for a pass that runs.** Per output texel:
//!
//! - **feedback** — one fullscreen pass, 2 texel loads and a multiply-add.
//! - **bloom** — two fullscreen passes, 9 samples each, plus one load in the
//!   second: 19 fetches.
//! - **rgb shift** — one fullscreen pass, 1 load and 2 samples.
//! - **retention**, when feedback runs — one frame-sized texture copy, 7.03 MB
//!   read and written at 1280x720.
//!
//! **An amount of zero records no pass at all**, which is
//! [ADR-0040](../../../docs/adr/0040-a-gain-of-zero-means-no-contribution-so-the-slot-is-skipped.md)'s
//! rule where the composite already applies it: no contribution, no cost. With
//! all three at zero the chain records nothing, the mix writes straight into
//! the present pass's target, and the frame is the frame this program drew
//! before this module existed — bit for bit, and `mod gpu`'s
//! `a_chain_at_zero_is_the_frame_with_no_chain` is what holds it.
//!
//! # Determinism
//!
//! Feedback reads the previous frame, which makes the retained frame part of
//! the state a replay has to reproduce
//! (`docs/principles/0092-the-same-inputs-produce-the-same-frame.md`). It is,
//! and by construction rather than by care: the retained frame is a copy of a
//! target this chain wrote on the previous frame, the copy is a
//! `copy_texture_to_texture` and not an arithmetic pass, the parameters come
//! from records, and a freshly allocated target reads as zero — so a run and a
//! replay of the same records see the same history at every frame, including
//! the first. `a_feedback_run_is_bit_exact_across_two_runs` is that claim.
//!
//! **Nothing here reads a clock**, and the chain has no state beyond the
//! retained frame and the four numbers a record carries.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::present::Present;

/// **Which frame feedback reads back.**
///
/// The maintainer's decision on 2026-09-09 was *both, selectable* — the two
/// cuts are different pictures and neither is the obvious one, so the
/// parameter chooses rather than the design choosing. See ADR-0317.
///
/// **Only the chosen one is retained.** A cut is held by copying a
/// frame-sized target once a frame, so retaining both would cost a second
/// copy and a second 7.03 MB target for a picture nothing reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cut {
    /// **The frame as the mix wrote it**, before anything in this chain
    /// touched it — `out` applied, feedback not yet added.
    ///
    /// **One echo and not a trail**, and it is worth being plain about:
    /// nothing read back has ever been fed back, so the picture is this
    /// frame plus `amount` of the previous frame and no third layer exists.
    /// On material that is not moving it reaches `1 + amount` times the frame
    /// after exactly one frame and stops there — measured, in
    /// `tests/master.rs`'s `the_two_cuts_are_two_pictures`.
    #[default]
    Mix,
    /// **This chain's exit**, after rgb shift and before the tone map.
    ///
    /// **A trail, because it compounds**: what is read back already contains
    /// the trail, so every layer is re-bloomed and re-fringed on each pass
    /// round the loop and the picture keeps climbing until the geometric
    /// series settles. That is the look feedback is usually wanted for, and it
    /// is also why [`Chain::FEEDBACK_MAX`] is not 1.0.
    Exit,
}

impl Cut {
    /// Both, in the order a surface shows them. A list and not a cycle: the
    /// cycle belongs to whoever draws the control
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    pub const ALL: [Cut; 2] = [Cut::Mix, Cut::Exit];

    /// **The lower-case word for this cut**, which is what a record carries
    /// and what every surface spells it with. A match rather than a table, for
    /// `Blend::name`'s reason: a cut added to the enum does not compile until
    /// it has a name.
    pub fn name(self) -> &'static str {
        match self {
            Cut::Mix => "mix",
            Cut::Exit => "exit",
        }
    }

    /// The cut this word names, or `None`. The engine's, because the engine is
    /// what reads a record back.
    pub fn parse(word: &str) -> Option<Cut> {
        Cut::ALL.into_iter().find(|c| c.name() == word)
    }
}

/// **The whole of what the master chain is set to**: one amount per pass, and
/// feedback's cut.
///
/// One value rather than three because it is written to one uniform and read
/// back as one reading, and because a stream that moved one amount without the
/// others beside it would be describing a chain nobody can reconstruct — which
/// is `karakuri_store::record::Record::Look`'s argument, met here by four
/// numbers instead of three.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chain {
    /// **How much of the retained frame is added back**, `[0, 0.95]`.
    pub feedback: f32,
    /// Which frame that is. Meaningless while `feedback` is zero and carried
    /// anyway, so that turning the amount up puts back the cut that was last
    /// chosen rather than a default.
    pub cut: Cut,
    /// **How much of the blurred bright part is added**, `[0, 1]`.
    pub bloom: f32,
    /// **How far the channels are pulled apart**, `[0, 1]` of
    /// [`Chain::SHIFT_MAX`].
    pub rgb_shift: f32,
}

impl Default for Chain {
    /// **Nothing, and the frame is the frame with no chain in it.** Every
    /// amount is zero, so no pass is recorded — see this module's
    /// documentation.
    fn default() -> Chain {
        Chain {
            feedback: 0.0,
            cut: Cut::Mix,
            bloom: 0.0,
            rgb_shift: 0.0,
        }
    }
}

impl Chain {
    /// **The most feedback there is, and it is short of 1.0 on purpose.**
    ///
    /// Under [`Cut::Exit`] the pass is an accumulator — `frame + a * previous
    /// output` sums to `frame / (1 - a)` on material that is not moving — so
    /// an amount of 1.0 has no decay in it and a still frame runs away to
    /// infinity with nothing in the picture to warn anybody first. At 0.95 the
    /// ceiling is twenty times the frame, which a tone mapper has an answer
    /// for. Under [`Cut::Mix`] there is no recursion and the ceiling is twice
    /// the frame at any amount; the range is one range because the control is
    /// one control, and the cut is a parameter of it rather than a mode of it.
    pub const FEEDBACK_MAX: f32 = 0.95;

    /// The whole range of the other two, which is `[0, 1]` at both ends: an
    /// amount above 1.0 would add more blurred light than there was light, and
    /// a fringe wider than [`Chain::SHIFT_MAX`] stops reading as one picture.
    pub const AMOUNT_MAX: f32 = 1.0;

    /// **What `rgb_shift` of 1.0 is**, in fractions of the frame's *height*:
    /// 2%, which is 14.4 texels either way at 720. In the shader as
    /// `SHIFT_MAX`, and here so that a surface can say what a number means
    /// without reading WGSL.
    pub const SHIFT_MAX: f32 = 0.02;

    /// **Bloom's radius**, in the same units and fixed for the reason
    /// `shaders/master.wgsl` gives at its own constant: the tap count is what a
    /// radius costs, and a cost is known before it is paid.
    pub const BLOOM_RADIUS: f32 = 0.012;

    /// **Every amount brought into range, and it is the engine that does it.**
    ///
    /// A surface asks; the wall is here, where the record is applied, so every
    /// route in meets the same one
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). A NaN
    /// floors to zero, which is the pass not running — the same answer
    /// `clamp_gain` gives one bay up.
    pub fn clamped(self) -> Chain {
        fn amount(v: f32, high: f32) -> f32 {
            if v.is_nan() {
                0.0
            } else {
                v.clamp(0.0, high)
            }
        }
        Chain {
            feedback: amount(self.feedback, Chain::FEEDBACK_MAX),
            cut: self.cut,
            bloom: amount(self.bloom, Chain::AMOUNT_MAX),
            rgb_shift: amount(self.rgb_shift, Chain::AMOUNT_MAX),
        }
    }

    /// **How many of the three passes run at these settings**, which is the
    /// number of ping-pong steps and is zero for a chain nobody has turned up.
    ///
    /// Bloom is one step and two passes: its first half writes the blur buffer
    /// rather than the frame, so it does not advance the ping-pong.
    fn steps(&self) -> usize {
        usize::from(self.feedback > 0.0)
            + usize::from(self.bloom > 0.0)
            + usize::from(self.rgb_shift > 0.0)
    }

    /// Whether this chain records anything at all.
    pub fn runs(&self) -> bool {
        self.steps() > 0
    }
}

/// Matches `Chain` in `shaders/master.wgsl`. 16 bytes, so no trailing pad is
/// needed to satisfy the uniform address space's alignment rules.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ChainUniform {
    feedback: f32,
    bloom: f32,
    shift: f32,
    _pad: f32,
}

/// The four frame-sized targets and the four pipelines that read them.
pub(crate) struct MasterChain {
    /// The two the passes ping-pong between. `[0]` is where the mix writes
    /// when the chain runs at all.
    ping: [wgpu::Texture; 2],
    ping_views: [wgpu::TextureView; 2],
    /// Bloom's horizontal half lands here and its vertical half reads it. Not
    /// one of the two above, because bloom's second pass reads the frame
    /// *and* this at once and writes a third target.
    blur: wgpu::TextureView,
    /// The retained cut of the previous frame. Read by feedback, written by a
    /// copy at the end of the frame, and zero until something has been copied
    /// into it — a freshly created texture reads as zero, which is what makes
    /// the first frame of a run and the first frame of its replay the same
    /// frame.
    history: wgpu::Texture,
    /// `(src, aux)` for every pass shape, per side of the ping-pong.
    feedback_bind: [wgpu::BindGroup; 2],
    /// `(src, src)` — the bright pass and rgb shift, neither of which reads a
    /// second texture. Binding a view twice is the composite's own trick, and
    /// it is why there is one layout here rather than three.
    self_bind: [wgpu::BindGroup; 2],
    /// `(src, blur)` — bloom's second half.
    blend_bind: [wgpu::BindGroup; 2],
    feedback: wgpu::RenderPipeline,
    bloom_bright: wgpu::RenderPipeline,
    bloom_blend: wgpu::RenderPipeline,
    rgb_shift: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    chain: Chain,
    width: u32,
    height: u32,
}

impl MasterChain {
    pub(crate) fn new(device: &wgpu::Device, width: u32, height: u32) -> MasterChain {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("master chain"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/master.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("master chain"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("master chain"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let pipeline = |label: &str, entry: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        // Linear HDR out, and no blend state: every pass here
                        // folds in its own shader, so what reaches the target
                        // is the whole answer rather than an answer combined
                        // with whatever the target held.
                        format: Present::HDR_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("master chain"),
            // Clamped at the edges, which is `SamplerDescriptor`'s default and
            // is the one that matters here: a bloom tap or a shifted channel
            // that walks off the frame reads the edge texel rather than
            // wrapping the far side of the picture into it.
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("master chain"),
            contents: bytemuck::bytes_of(&ChainUniform {
                feedback: 0.0,
                bloom: 0.0,
                shift: 0.0,
                _pad: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let targets = Targets::allocate(device, &layout, &sampler, &uniform, width, height);
        MasterChain {
            ping: targets.ping,
            ping_views: targets.ping_views,
            blur: targets.blur,
            history: targets.history,
            feedback_bind: targets.feedback_bind,
            self_bind: targets.self_bind,
            blend_bind: targets.blend_bind,
            feedback: pipeline("master feedback", "fs_feedback"),
            bloom_bright: pipeline("master bloom bright", "fs_bloom_bright"),
            bloom_blend: pipeline("master bloom blend", "fs_bloom_blend"),
            rgb_shift: pipeline("master rgb shift", "fs_rgb_shift"),
            layout,
            sampler,
            uniform,
            chain: Chain::default(),
            width,
            height,
        }
    }

    /// Reallocation, so never from the render thread mid-frame — the same
    /// shape as [`Present::resize`], which is its one caller.
    ///
    /// **The retained frame does not survive it**, and it is stated rather
    /// than fixed: the new one is a new texture and reads as zero, so a trail
    /// starts again from the frame after a resize. Scaling the old one would
    /// be inventing texels a replay would have to reproduce exactly.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let targets = Targets::allocate(
            device,
            &self.layout,
            &self.sampler,
            &self.uniform,
            width,
            height,
        );
        self.ping = targets.ping;
        self.ping_views = targets.ping_views;
        self.blur = targets.blur;
        self.history = targets.history;
        self.feedback_bind = targets.feedback_bind;
        self.self_bind = targets.self_bind;
        self.blend_bind = targets.blend_bind;
        self.width = width;
        self.height = height;
    }

    pub(crate) fn chain(&self) -> Chain {
        self.chain
    }

    /// Set the chain and write its uniform. Clamped here rather than at any
    /// surface — see [`Chain::clamped`].
    pub(crate) fn set(&mut self, queue: &wgpu::Queue, chain: Chain) {
        self.chain = chain.clamped();
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&ChainUniform {
                feedback: self.chain.feedback,
                bloom: self.chain.bloom,
                shift: self.chain.rgb_shift,
                _pad: 0.0,
            }),
        );
    }

    /// **Where the mix writes**: this chain's entry when anything runs, and
    /// `None` when nothing does — in which case the caller hands the mix the
    /// present pass's own target and this module is not in the frame at all.
    pub(crate) fn entry(&self) -> Option<&wgpu::TextureView> {
        self.chain.runs().then(|| &self.ping_views[0])
    }

    /// **Record the chain**, from `entry` into `out`.
    ///
    /// Nothing is recorded for a chain that does not run, so the caller's
    /// `out` is what the mix already wrote into. Nothing here allocates: the
    /// bind groups and the targets were made at build, and the uniform was
    /// written when the chain was set.
    pub(crate) fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        out: &wgpu::TextureView,
        out_texture: &wgpu::Texture,
    ) {
        let total = self.chain.steps();
        if total == 0 {
            return;
        }
        // Which side of the ping-pong the next pass reads. The mix wrote into
        // `[0]`, so that is where the first one starts.
        let mut src = 0usize;
        let mut done = 0usize;
        // The last step writes into the caller's target and every other one
        // into the other side of the ping-pong.
        let dst = |src: usize, done: usize| {
            if done + 1 == total {
                out
            } else {
                &self.ping_views[1 - src]
            }
        };

        if self.chain.feedback > 0.0 {
            self.pass(
                encoder,
                "master feedback",
                &self.feedback,
                &self.feedback_bind[src],
                dst(src, done),
            );
            // **The mix cut is retained here and not at the end of the
            // frame**, and the position is the whole of it. `ping[0]` holds
            // the frame as the mix wrote it *until the second step writes into
            // it*, so the copy has to be recorded while it is still true. It
            // is recorded after the pass that reads the history rather than
            // before, so what feedback added this frame is last frame's cut
            // and not this one's — commands run in the order they are
            // recorded, which is what makes that a fact rather than a hope.
            if self.chain.cut == Cut::Mix {
                self.retain(encoder, &self.ping[0]);
            }
            done += 1;
            src = 1 - src;
        }
        if self.chain.bloom > 0.0 {
            // The bright half writes the blur buffer rather than the frame, so
            // it is not a step: the ping-pong does not advance across it.
            self.pass(
                encoder,
                "master bloom bright",
                &self.bloom_bright,
                &self.self_bind[src],
                &self.blur,
            );
            self.pass(
                encoder,
                "master bloom blend",
                &self.bloom_blend,
                &self.blend_bind[src],
                dst(src, done),
            );
            done += 1;
            src = 1 - src;
        }
        if self.chain.rgb_shift > 0.0 {
            self.pass(
                encoder,
                "master rgb shift",
                &self.rgb_shift,
                &self.self_bind[src],
                dst(src, done),
            );
        }

        // **The exit cut is retained once every pass has run**, which is what
        // makes it the exit: the last thing written before the tone mapper
        // reads it.
        if self.chain.feedback > 0.0 && self.chain.cut == Cut::Exit {
            self.retain(encoder, out_texture);
        }
    }

    /// **Hold one frame for the next one.** A copy and not a pass: it moves
    /// the texels and cannot change one, which is what makes the history the
    /// same on a replay as it was on the run.
    fn retain(&self, encoder: &mut wgpu::CommandEncoder, source: &wgpu::Texture) {
        encoder.copy_texture_to_texture(
            source.as_image_copy(),
            self.history.as_image_copy(),
            wgpu::Extent3d {
                width: self.width.max(1),
                height: self.height.max(1),
                depth_or_array_layers: 1,
            },
        );
    }

    fn pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        label: &str,
        pipeline: &wgpu::RenderPipeline,
        bind: &wgpu::BindGroup,
        target: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // The triangle covers the whole target, so nothing of the
                    // clear survives; it is `Clear` rather than `Load` because
                    // loading would ask the driver to fetch a target this pass
                    // overwrites in full.
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The four targets and the six bind groups over them, made together because
/// a bind group is invalid the moment the view it names is replaced.
struct Targets {
    ping: [wgpu::Texture; 2],
    ping_views: [wgpu::TextureView; 2],
    blur: wgpu::TextureView,
    history: wgpu::Texture,
    feedback_bind: [wgpu::BindGroup; 2],
    self_bind: [wgpu::BindGroup; 2],
    blend_bind: [wgpu::BindGroup; 2],
}

impl Targets {
    fn allocate(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        uniform: &wgpu::Buffer,
        width: u32,
        height: u32,
    ) -> Targets {
        let target = |label: &str, copy_src: bool| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: width.max(1),
                    height: height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // Always linear HDR, never the surface's: this whole chain is
                // upstream of the one encode.
                format: Present::HDR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | if copy_src {
                        wgpu::TextureUsages::COPY_SRC
                    } else {
                        wgpu::TextureUsages::COPY_DST
                    },
                view_formats: &[],
            })
        };
        // `COPY_SRC` on the entry because [`Cut::Mix`] is retained by copying
        // it; `COPY_DST` on the history because that is where the copy lands.
        let ping = [
            target("master chain a", true),
            target("master chain b", true),
        ];
        let ping_views = [
            ping[0].create_view(&Default::default()),
            ping[1].create_view(&Default::default()),
        ];
        let blur_texture = target("master chain blur", true);
        let blur = blur_texture.create_view(&Default::default());
        let history = target("master chain history", false);
        let history_view = history.create_view(&Default::default());

        let bind = |label: &str, src: &wgpu::TextureView, aux: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(aux),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            })
        };
        let feedback_bind = [
            bind("master feedback a", &ping_views[0], &history_view),
            bind("master feedback b", &ping_views[1], &history_view),
        ];
        let self_bind = [
            bind("master self a", &ping_views[0], &ping_views[0]),
            bind("master self b", &ping_views[1], &ping_views[1]),
        ];
        let blend_bind = [
            bind("master blend a", &ping_views[0], &blur),
            bind("master blend b", &ping_views[1], &blur),
        ];
        Targets {
            ping,
            ping_views,
            blur,
            history,
            feedback_bind,
            self_bind,
            blend_bind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A chain nobody has turned up records nothing**, which is the whole of
    /// what makes the default look unchanged: not a pass that multiplies by
    /// zero, but no pass.
    #[test]
    fn every_amount_at_zero_is_no_pass_at_all() {
        assert_eq!(Chain::default().steps(), 0);
        assert!(!Chain::default().runs());
    }

    /// **One step per pass that has an amount, and bloom's two passes are one
    /// step** — the bright half writes the blur buffer rather than the frame.
    #[test]
    fn a_pass_with_an_amount_is_a_step_and_one_without_is_not() {
        let all = Chain {
            feedback: 0.5,
            cut: Cut::Exit,
            bloom: 0.5,
            rgb_shift: 0.5,
        };
        assert_eq!(all.steps(), 3);
        assert_eq!(Chain { bloom: 0.0, ..all }.steps(), 2);
        assert_eq!(
            Chain {
                feedback: 0.0,
                rgb_shift: 0.0,
                ..all
            }
            .steps(),
            1
        );
    }

    /// **The clamp is the engine's**, and a NaN floors to zero rather than
    /// reaching a uniform — the pass then does not run at all, which is the
    /// answer that cannot draw a broken frame.
    #[test]
    fn the_engine_brings_every_amount_into_range() {
        let wild = Chain {
            feedback: 4.0,
            cut: Cut::Exit,
            bloom: -1.0,
            rgb_shift: f32::NAN,
        }
        .clamped();
        assert_eq!(wild.feedback, Chain::FEEDBACK_MAX);
        assert_eq!(wild.bloom, 0.0);
        assert_eq!(wild.rgb_shift, 0.0);
        // The cut is not a range and is carried through whatever the amounts
        // do.
        assert_eq!(wild.cut, Cut::Exit);
    }

    /// **Feedback stops short of 1.0**, because under [`Cut::Exit`] an amount
    /// of 1.0 is an accumulator with no decay. The number is the ceiling on
    /// still material: `1 / (1 - a)`.
    #[test]
    fn the_feedback_ceiling_is_twenty_times_the_frame() {
        const { assert!(Chain::FEEDBACK_MAX < 1.0) };
        let ceiling = 1.0 / (1.0 - Chain::FEEDBACK_MAX);
        assert!((ceiling - 20.0).abs() < 0.001, "{ceiling}");
    }

    /// **Every cut has a name and the name round-trips**, which is what a
    /// record carries.
    #[test]
    fn a_cut_is_spelled_one_way_and_read_back() {
        for cut in Cut::ALL {
            assert_eq!(Cut::parse(cut.name()), Some(cut));
        }
        assert_eq!(Cut::parse("previous"), None);
    }
}
