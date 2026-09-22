pub use std::collections::BTreeMap;

pub use karakuri_engine::{
    Chain, ChainEvent, ChainSlot, ChainSwap, Clock, Cut, Gpu, Points, Present, RenderPassNode,
    RetentionManager, Slot, SlotSpec, VideoSource,
};
pub use karakuri_ir::typed::Checked;

pub const WIDTH: u32 = 128;
pub const HEIGHT: u32 = 64;
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// The three shipped procedures, compiled in so that a test does not
/// depend on a working directory.
pub const FEEDBACK: &str = include_str!("../../../../examples/feedback.kir");
pub const BLOOM: &str = include_str!("../../../../examples/bloom.kir");
pub const RGB_SHIFT: &str = include_str!("../../../../examples/rgb_shift.kir");

/// **The four fragment entry points `master.wgsl` held until 2026-09-10**,
/// term for term, over the chain's own bind group layout.
///
/// Kept here and nowhere else: the engine no longer contains them, and a
/// replacement that nothing ever ran against what it replaced is a claim
/// rather than a fact.
pub const HAND_WRITTEN: &str = r#"
struct Chain {
    feedback: f32,
    bloom: f32,
    shift: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> chain: Chain;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var aux: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

const BLOOM_RADIUS: f32 = 0.012;
const W0: f32 = 0.2041631;
const W1: f32 = 0.1801737;
const W2: f32 = 0.1238322;
const W3: f32 = 0.0662825;
const W4: f32 = 0.0276303;
const SHIFT_MAX: f32 = 0.02;
const BLOOM_KNEE: f32 = 1.0;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

fn step_uv(r: f32) -> vec2<f32> {
    let dim = vec2<f32>(textureDimensions(src, 0));
    return vec2<f32>(r * dim.y / dim.x, r);
}

fn keepable(v: vec3<f32>) -> vec3<f32> {
    return select(vec3<f32>(0.0), v, (v == v) & (abs(v) < vec3<f32>(3.4e38)));
}

@fragment
fn fs_feedback(in: VsOut) -> @location(0) vec4<f32> {
    let at = vec2<i32>(in.clip.xy);
    let s = textureLoad(src, at, 0);
    let h = keepable(textureLoad(aux, at, 0).rgb);
    return vec4<f32>(s.rgb + chain.feedback * h, s.a);
}

fn bright(uv: vec2<f32>) -> vec3<f32> {
    return max(textureSampleLevel(src, samp, uv, 0.0).rgb - vec3<f32>(BLOOM_KNEE), vec3<f32>(0.0));
}

@fragment
fn fs_bloom_bright(in: VsOut) -> @location(0) vec4<f32> {
    let d = vec2<f32>(step_uv(BLOOM_RADIUS).x * 0.25, 0.0);
    var sum = vec3<f32>(0.0);
    sum += W4 * bright(in.uv - 4.0 * d);
    sum += W3 * bright(in.uv - 3.0 * d);
    sum += W2 * bright(in.uv - 2.0 * d);
    sum += W1 * bright(in.uv - d);
    sum += W0 * bright(in.uv);
    sum += W1 * bright(in.uv + d);
    sum += W2 * bright(in.uv + 2.0 * d);
    sum += W3 * bright(in.uv + 3.0 * d);
    sum += W4 * bright(in.uv + 4.0 * d);
    return vec4<f32>(sum, 1.0);
}

@fragment
fn fs_bloom_blend(in: VsOut) -> @location(0) vec4<f32> {
    let d = vec2<f32>(0.0, BLOOM_RADIUS * 0.25);
    var sum = vec3<f32>(0.0);
    sum += W4 * textureSampleLevel(aux, samp, in.uv - 4.0 * d, 0.0).rgb;
    sum += W3 * textureSampleLevel(aux, samp, in.uv - 3.0 * d, 0.0).rgb;
    sum += W2 * textureSampleLevel(aux, samp, in.uv - 2.0 * d, 0.0).rgb;
    sum += W1 * textureSampleLevel(aux, samp, in.uv - d, 0.0).rgb;
    sum += W0 * textureSampleLevel(aux, samp, in.uv, 0.0).rgb;
    sum += W1 * textureSampleLevel(aux, samp, in.uv + d, 0.0).rgb;
    sum += W2 * textureSampleLevel(aux, samp, in.uv + 2.0 * d, 0.0).rgb;
    sum += W3 * textureSampleLevel(aux, samp, in.uv + 3.0 * d, 0.0).rgb;
    sum += W4 * textureSampleLevel(aux, samp, in.uv + 4.0 * d, 0.0).rgb;
    let s = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    return vec4<f32>(s.rgb + chain.bloom * sum, s.a);
}

@fragment
fn fs_rgb_shift(in: VsOut) -> @location(0) vec4<f32> {
    let d = vec2<f32>(step_uv(SHIFT_MAX * chain.shift).x, 0.0);
    let centre = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    let r = textureSampleLevel(src, samp, in.uv + d, 0.0).r;
    let b = textureSampleLevel(src, samp, in.uv - d, 0.0).b;
    return vec4<f32>(r, centre.g, b, centre.a);
}
"#;

/// A chain slot that is nothing but the clock it was handed. Every texel of
/// the frame it writes is `(t, beats, dt)`, so a readback is a direct
/// reading of the uniform the pass ran under.
///
/// The `amount` parameter is declared and unused: it is what
/// `moving_a_parameter_leaves_the_clock_where_it_is` writes.
pub const CLOCK: &str = r#"
proc clock_probe {
  kind L5

  param amount : float [0.0, 1.0] = 0.0

  frame {
    color = vec4(t, beats, dt, 1.0);
  }
}
"#;

pub fn checked(source: &str) -> Checked {
    let proc = karakuri_ir::parse(source).expect("a shipped procedure parses");
    karakuri_ir::check::check(&proc).expect("a shipped procedure checks")
}

pub fn present(gpu: &Gpu) -> Present {
    Present::new(&gpu.device, FORMAT, WIDTH, HEIGHT)
}

pub fn params(amount: f32) -> BTreeMap<String, f32> {
    [("amount".to_string(), amount)].into_iter().collect()
}

/// What a slot of the shipped chain is called. A function of the source, so
/// the address a spec names and the address a built slot reports are one
/// answer.
pub fn address(source: &str) -> String {
    format!("test:{:x}", source.len())
}

/// One slot of the shipped chain as a description, for the worker to build.
pub fn spec(source: &str, cut: Option<Cut>, amount: f32) -> SlotSpec {
    SlotSpec {
        procedure: address(source),
        cut,
        params: params(amount),
    }
}

/// One slot of the shipped chain as `ChainSwap` takes it: the description
/// and the checked source behind it.
pub fn asked(source: &str, cut: Option<Cut>, amount: f32) -> ChainSlot {
    ChainSlot {
        spec: spec(source, cut, amount),
        checked: checked(source),
    }
}

/// Spins until `done`, or fails after 30 seconds saying what never
/// happened. Nothing here sleeps on a fixed interval: the wait is on a
/// worker thread's progress and the failure is a deadline, not a guess.
pub fn until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !done() {
        assert!(std::time::Instant::now() < deadline, "{what}");
        std::thread::yield_now();
    }
}

/// One slot of the shipped chain, built against the `Present` that will run
/// it — see `Present::chain_layout`.
pub fn slot(gpu: &Gpu, present: &Present, source: &str, cut: Option<Cut>, amount: f32) -> Slot {
    Slot::build(
        &gpu.device,
        present.chain_layout(),
        format!("test:{:x}", source.len()),
        &checked(source),
        cut,
        params(amount),
    )
    .expect("a shipped procedure is a legal chain slot")
}

/// `tests/tonemap.rs`'s `hot_points`: an exposure high enough that the core
/// blows every channel past 1.0, which is what bloom's knee is at.
pub fn hot_points(gpu: &Gpu) -> Points {
    let mut points = Points::new(&gpu.device, 4096, 19274);
    points.resize(WIDTH, HEIGHT);
    points.params.exposure = 6.0;
    points.params.point_scale = 10.0;
    points.prepare(&gpu.queue, 1);
    points
}

pub fn target(gpu: &Gpu, label: &str) -> wgpu::Texture {
    gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
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
    })
}

/// The linear HDR bytes of one texture.
pub fn readback(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u8> {
    let bytes_per_row = WIDTH * 8;
    assert_eq!(bytes_per_row % 256, 0, "readback rows must stay aligned");
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("master test readback"),
        size: u64::from(bytes_per_row * HEIGHT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let pixels = slice.get_mapped_range().expect("map").to_vec();
    buffer.unmap();
    pixels
}

/// One frame: the source into wherever the mix writes, the chain, and the
/// linear HDR target read back.
///
/// **`mix_target` and not `hdr_view`**, which is the whole seam this file
/// is about: with a chain running the two are different targets, and a
/// test that rendered into `hdr_view` would be feeding the chain nothing
/// and reading its own source back.
pub fn frame(gpu: &Gpu, present: &Present, points: &mut Points) -> Vec<u8> {
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    points.render(&mut encoder, present.mix_target(), 1);
    present.draw_chain(&mut encoder);
    gpu.queue.submit([encoder.finish()]);
    readback(gpu, present.hdr_texture())
}

/// **The mix, as many frames of it as asked for**, each kept in a texture
/// of its own.
///
/// A `Present` with an empty chain writes the mix straight into the target
/// the present pass reads, so this is exactly what the first slot of a
/// chain would be handed — which is what lets the hand-written pass below
/// be run on the same input the shipped procedure sees.
pub fn mixes(gpu: &Gpu, count: usize) -> Vec<wgpu::Texture> {
    let present = present(gpu);
    let mut points = hot_points(gpu);
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let kept = target(gpu, "mix");
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        points.render(&mut encoder, present.mix_target(), 1);
        let _ = i;
        encoder.copy_texture_to_texture(
            present.hdr_texture().as_image_copy(),
            kept.as_image_copy(),
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);
        out.push(kept);
    }
    out
}

/// The four hand-written passes, compiled against the chain's own layout.
pub struct Hand {
    pub feedback: wgpu::RenderPipeline,
    pub bloom_bright: wgpu::RenderPipeline,
    pub bloom_blend: wgpu::RenderPipeline,
    pub rgb_shift: wgpu::RenderPipeline,
    pub layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub uniform: wgpu::Buffer,
}

impl Hand {
    pub fn new(gpu: &Gpu, present: &Present) -> Hand {
        let module = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("hand-written master chain"),
                source: wgpu::ShaderSource::Wgsl(HAND_WRITTEN.into()),
            });
        // **The chain's own layout**, which is what makes this a comparison
        // rather than two arrangements: the same bindings in the same
        // order, so the only thing that can differ is the body.
        let layout = present.chain_layout().clone();
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("hand-written master chain"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let make = |entry: &str| {
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(entry),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some(entry),
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
                })
        };
        Hand {
            feedback: make("fs_feedback"),
            bloom_bright: make("fs_bloom_bright"),
            bloom_blend: make("fs_bloom_blend"),
            rgb_shift: make("fs_rgb_shift"),
            layout,
            sampler: gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("hand-written master chain"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            uniform: gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("hand-written master chain"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }

    pub fn set(&self, gpu: &Gpu, feedback: f32, bloom: f32, shift: f32) {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&feedback.to_le_bytes());
        bytes[4..8].copy_from_slice(&bloom.to_le_bytes());
        bytes[8..12].copy_from_slice(&shift.to_le_bytes());
        gpu.queue.write_buffer(&self.uniform, 0, &bytes);
    }

    pub fn bind(
        &self,
        gpu: &Gpu,
        src: &wgpu::TextureView,
        aux: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hand-written master chain"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
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
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    pub fn pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::RenderPipeline,
        bind: &wgpu::BindGroup,
        into: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hand-written master chain"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: into,
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
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

pub use crate::engine_common::f16;

pub fn channels(pixels: &[u8]) -> Vec<f32> {
    pixels
        .chunks_exact(2)
        .map(|c| f16(u16::from_le_bytes([c[0], c[1]])))
        .collect()
}

/// The total light in a frame, summed over every colour channel. A blunt
/// instrument on purpose: bloom and feedback both *add* light, and this is
/// the one number that says so whatever else moved.
pub fn light(pixels: &[u8]) -> f64 {
    pixels
        .chunks_exact(8)
        .map(|texel| {
            texel[..6]
                .chunks_exact(2)
                .map(|c| f64::from(f16(u16::from_le_bytes([c[0], c[1]]))))
                .sum::<f64>()
        })
        .sum()
}
