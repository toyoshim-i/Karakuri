//! Unified render pass and image pass pipeline abstractions.
//!
//! # What it is
//!
//! A shared execution substrate connecting:
//! - **Image passes** (`ImagePass`): Fullscreen fragment shaders (`kind L5`) that transform
//!   or composite frames through uniforms, history buffers, and texture inputs.
//! - **Render pass execution** ([`RenderPassNode`]): A unified trait for any pass node
//!   that records draw commands into a target view (`fn record(&self, encoder, target)`).
//! - **History retention** ([`RetentionManager`]): Position-independent retention of
//!   retained frames (`Cut::Mix` and `Cut::Exit`), sanitized via `shaders/master.wgsl`'s
//!   `fs_keep` pass so that no NaN or infinity propagates across frames.
//!
//! Every fullscreen pass in this module is driven by a single three-vertex oversized
//! triangle covering `[-1, 1]` without requiring a vertex buffer or seam.

use std::fmt;

use karakuri_codegen::generate_l5;
use karakuri_codegen::layout::UniformLayout;
use karakuri_ir::typed::Checked;

use crate::present::Present;
use crate::uniforms::UniformScratch;

/// Standard L5 shader binding indices.
pub const BINDING_UNIFORM: u32 = 0;
pub const BINDING_SRC: u32 = 1;
pub const BINDING_HELD: u32 = 2;
pub const BINDING_SAMPLER: u32 = 3;

/// **A pass node that records draw commands into a target view.**
///
/// Implemented by both Set-level compositing nodes and Master-level image passes,
/// unifying execution across geometry, post-processing, and compositing layers.
pub trait RenderPassNode {
    /// Record commands targeting `target`.
    fn record(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView);
}

/// **Records a fullscreen pass over `target` using a 3-vertex oversized triangle.**
///
/// Executes `pass.draw(0..3, 0..1)` over `target` with linear HDR clear-to-transparent
/// and store operations.
pub fn record_fullscreen_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: Option<&str>,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_groups: &[&wgpu::BindGroup],
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
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
    for (i, bg) in bind_groups.iter().enumerate() {
        pass.set_bind_group(i as u32, *bg, &[]);
    }
    pass.draw(0..3, 0..1);
}

/// **Allocate a 2D linear HDR render target view.**
pub fn create_hdr_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: Option<&str>,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: Present::HDR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    texture.create_view(&Default::default())
}

/// **Which frame a slot reads back**, where its procedure declares `retains`.
///
/// The declaration is bare — *this reads a retained frame* — and **which one is
/// the slot's answer**, which is ADR-0317's decision carried forward unchanged:
/// the two cuts are different pictures and neither is the obvious one, so the
/// place that instantiates a procedure chooses rather than the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum Cut {
    /// **The frame as the mix wrote it**, before anything in this chain touched
    /// it — `out` applied, nothing added.
    #[default]
    Mix,
    /// **This chain's exit**, after the last slot and before the tone map.
    Exit,
}

impl Cut {
    /// Both cuts, in canonical order.
    pub const ALL: [Cut; 2] = [Cut::Mix, Cut::Exit];

    /// Lower-case identifier for this cut (`"mix"` or `"exit"`).
    pub fn name(self) -> &'static str {
        match self {
            Cut::Mix => "mix",
            Cut::Exit => "exit",
        }
    }

    /// Parse a cut name, returning `None` if unrecognized.
    pub fn parse(word: &str) -> Option<Cut> {
        Cut::ALL.into_iter().find(|c| c.name() == word)
    }

    /// Fixed index in `[0, 1]` for array indexing.
    pub fn index(self) -> usize {
        match self {
            Cut::Mix => 0,
            Cut::Exit => 1,
        }
    }
}

/// **The clock an image pass reads**, handed to the chain by the host.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Clock {
    pub t: f32,
    pub beats: f32,
    pub dt: f32,
    pub seed_salt: u32,
}

/// **Unified frame retention management.**
///
/// Manages retention pipelines, targets, and copy bind groups for `Cut::Mix` and `Cut::Exit`.
/// Sanitizes retained frames via `shaders/master.wgsl`'s `fs_keep` pass so that no NaN
/// or infinity propagates into history.
pub struct RetentionManager {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    retained: [Option<wgpu::TextureView>; 2],
    binds: [Option<wgpu::BindGroup>; 2],
}

impl RetentionManager {
    /// Compile the retention pipeline and bind group layout.
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("retention layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("retention shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/master.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("retention pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("retention pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_keep"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
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
        });
        RetentionManager {
            pipeline,
            layout,
            retained: [None, None],
            binds: [None, None],
        }
    }

    /// The bind group layout used by the retention pass.
    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    /// Access the retained view for `cut`, if currently allocated.
    pub fn held(&self, cut: Cut) -> Option<&wgpu::TextureView> {
        self.retained[cut.index()].as_ref()
    }

    /// Whether a target for `cut` is currently retained.
    pub fn is_held(&self, cut: Cut) -> bool {
        self.retained[cut.index()].is_some()
    }

    /// Which cuts are currently retained.
    pub fn active_cuts(&self) -> Vec<Cut> {
        Cut::ALL
            .into_iter()
            .filter(|c| self.retained[c.index()].is_some())
            .collect()
    }

    /// How many retained frame targets are currently allocated (0..=2).
    pub fn count(&self) -> usize {
        self.active_cuts().len()
    }

    /// Release all allocated retention targets and bind groups.
    pub fn clear(&mut self) {
        self.retained = [None, None];
        self.binds = [None, None];
    }

    /// Allocate targets and bind groups for the requested cuts.
    pub fn allocate(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        cuts: &[Cut],
        mix_source: Option<&wgpu::TextureView>,
        exit_source: Option<&wgpu::TextureView>,
    ) {
        self.retained = [None, None];
        for &cut in cuts {
            let label = format!("retention {} cut", cut.name());
            self.retained[cut.index()] =
                Some(create_hdr_target(device, width, height, Some(&label)));
        }

        let make_bind = |from: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("retention bind group"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(from),
                }],
            })
        };

        self.binds = [
            if self.retained[Cut::Mix.index()].is_some() {
                mix_source.map(make_bind)
            } else {
                None
            },
            if self.retained[Cut::Exit.index()].is_some() {
                exit_source.map(make_bind)
            } else {
                None
            },
        ];
    }

    /// Execute retention passes for all active cuts.
    pub fn record(&self, encoder: &mut wgpu::CommandEncoder) {
        for cut in Cut::ALL {
            if let (Some(target), Some(bind)) =
                (&self.retained[cut.index()], &self.binds[cut.index()])
            {
                let label = match cut {
                    Cut::Mix => "master chain retain mix",
                    Cut::Exit => "master chain retain exit",
                };
                record_fullscreen_pass(encoder, Some(label), target, &self.pipeline, &[bind]);
            }
        }
    }
}

/// **A compiled fullscreen image pass.**
///
/// Encapsulates:
/// - Pipeline execution (`pass.draw(0..3, 0..1)`).
/// - Uniform buffer storage, layout, and packing scratch.
/// - Standard L5 bind group construction (`uniform`, `src`, `held`, `sampler`).
pub struct ImagePass {
    name: String,
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    uniform_layout: UniformLayout,
    scratch: UniformScratch,
}

impl fmt::Debug for ImagePass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImagePass")
            .field("name", &self.name)
            .field("uniform_layout", &self.uniform_layout)
            .finish_non_exhaustive()
    }
}

impl ImagePass {
    /// Construct an `ImagePass` from compiled wgpu primitives.
    pub fn new(
        name: impl Into<String>,
        pipeline: wgpu::RenderPipeline,
        uniforms: wgpu::Buffer,
        uniform_layout: UniformLayout,
    ) -> Self {
        let scratch = UniformScratch::new(&uniform_layout);
        Self {
            name: name.into(),
            pipeline,
            uniforms,
            uniform_layout,
            scratch,
        }
    }

    /// Name of the pass for labeling and diagnostics.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The underlying render pipeline.
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    /// The uniform buffer for this pass.
    pub fn uniforms(&self) -> &wgpu::Buffer {
        &self.uniforms
    }

    /// Uniform buffer layout descriptor.
    pub fn uniform_layout(&self) -> &UniformLayout {
        &self.uniform_layout
    }

    /// Standard bind group layout for L5 image passes:
    /// - Binding 0: Uniform buffer
    /// - Binding 1: `src` texture view
    /// - Binding 2: `held` texture view
    /// - Binding 3: Sampler (linear filtering)
    pub fn create_bind_group_layout(
        device: &wgpu::Device,
        label: Option<&str>,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: BINDING_UNIFORM,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: BINDING_SRC,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: BINDING_HELD,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: BINDING_SAMPLER,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    /// Create the linear-filtering edge-clamping sampler for image passes.
    pub fn create_sampler(device: &wgpu::Device, label: Option<&str>) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        })
    }

    /// Construct an `ImagePass` by lowering a checked `kind L5` AST.
    pub fn from_l5(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        checked: &Checked,
    ) -> (Self, karakuri_codegen::L5Shader) {
        let shader = generate_l5(checked);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} (L5)", checked.name)),
            source: wgpu::ShaderSource::Wgsl(shader.source.as_str().into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image pass pipeline layout"),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&checked.name),
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
                targets: &[Some(wgpu::ColorTargetState {
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
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} uniforms", checked.name)),
            size: u64::from(shader.uniform_layout.total_size),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pass = Self::new(
            checked.name.clone(),
            pipeline,
            uniforms,
            shader.uniform_layout.clone(),
        );
        (pass, shader)
    }

    /// Bind source, held, and sampler into an L5 bind group.
    pub fn bind(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        src: &wgpu::TextureView,
        held: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        label: Option<&str>,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: BINDING_UNIFORM,
                    resource: self.uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_SRC,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_HELD,
                    resource: wgpu::BindingResource::TextureView(held),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_SAMPLER,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// Pack and upload the uniform block: clock, viewport, and custom parameters.
    pub fn write_uniform<'a>(
        &mut self,
        queue: &wgpu::Queue,
        clock: Clock,
        viewport: [f32; 2],
        params: impl IntoIterator<Item = (&'a str, f32)>,
    ) {
        let mut p = self.scratch.pack(&self.uniform_layout);
        p.f32("t", clock.t)
            .f32("beats", clock.beats)
            .f32("dt", clock.dt)
            .u32("seed_salt", clock.seed_salt)
            .vec2("viewport", viewport);
        for (name, val) in params {
            p.f32(name, val);
        }
        queue.write_buffer(&self.uniforms, 0, p.finish());
    }

    /// Write clock fields at the head of the uniform block without repacking parameters.
    pub fn write_clock(&self, queue: &wgpu::Queue, clock: Clock) {
        let mut head = [0u8; 16];
        head[0..4].copy_from_slice(&clock.t.to_le_bytes());
        head[4..8].copy_from_slice(&clock.beats.to_le_bytes());
        head[8..12].copy_from_slice(&clock.dt.to_le_bytes());
        head[12..16].copy_from_slice(&clock.seed_salt.to_le_bytes());
        debug_assert!(self
            .uniform_layout
            .fields
            .iter()
            .zip(["t", "beats", "dt", "seed_salt"])
            .all(|(f, n)| f.name == n));
        debug_assert_eq!(self.uniform_layout.fields[0].offset, 0);
        queue.write_buffer(&self.uniforms, 0, &head);
    }

    /// Record a fullscreen pass using this pipeline and the given bind group.
    pub fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        bind_group: &wgpu::BindGroup,
    ) {
        record_fullscreen_pass(
            encoder,
            Some(&self.name),
            target,
            &self.pipeline,
            &[bind_group],
        );
    }

    /// Pair this pass with an active bind group to form an executable [`RenderPassNode`].
    pub fn bound<'a>(&'a self, bind_group: &'a wgpu::BindGroup) -> BoundImagePass<'a> {
        BoundImagePass {
            pass: self,
            bind_group,
        }
    }
}

/// **An [`ImagePass`] bound to its runtime resources, ready to execute.**
pub struct BoundImagePass<'a> {
    pub pass: &'a ImagePass,
    pub bind_group: &'a wgpu::BindGroup,
}

impl<'a> RenderPassNode for BoundImagePass<'a> {
    fn record(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.pass.record(encoder, target, self.bind_group);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_round_trips_canonical_names() {
        for cut in Cut::ALL {
            assert_eq!(Cut::parse(cut.name()), Some(cut));
        }
        assert_eq!(Cut::parse("invalid"), None);
    }

    #[test]
    fn cut_indices_are_contiguous() {
        assert_eq!(Cut::Mix.index(), 0);
        assert_eq!(Cut::Exit.index(), 1);
    }

    #[test]
    fn clock_defaults_to_zero() {
        let clock = Clock::default();
        assert_eq!(clock.t, 0.0);
        assert_eq!(clock.beats, 0.0);
        assert_eq!(clock.dt, 0.0);
        assert_eq!(clock.seed_salt, 0);
    }
}
