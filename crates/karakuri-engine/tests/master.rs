//! Verification of the master chain: an ordered sequence of L5 post-processing slots.
//!
//! Asserts parity between shipped L5 procedures (`feedback.kir`, `bloom.kir`, `rgb_shift.kir`)
//! and legacy hand-written shader passes on real GPU readbacks (ADR-0340).

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use std::collections::BTreeMap;

    use karakuri_engine::{
        Chain, ChainEvent, ChainSlot, ChainSwap, Clock, Cut, Gpu, Points, Present, RenderPassNode,
        RetentionManager, Slot, SlotSpec, VideoSource,
    };
    use karakuri_ir::typed::Checked;

    const WIDTH: u32 = 128;
    const HEIGHT: u32 = 64;
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    /// The three shipped procedures, compiled in so that a test does not
    /// depend on a working directory.
    const FEEDBACK: &str = include_str!("../../../examples/feedback.kir");
    const BLOOM: &str = include_str!("../../../examples/bloom.kir");
    const RGB_SHIFT: &str = include_str!("../../../examples/rgb_shift.kir");

    /// **The four fragment entry points `master.wgsl` held until 2026-09-10**,
    /// term for term, over the chain's own bind group layout.
    ///
    /// Kept here and nowhere else: the engine no longer contains them, and a
    /// replacement that nothing ever ran against what it replaced is a claim
    /// rather than a fact.
    const HAND_WRITTEN: &str = r#"
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
    const CLOCK: &str = r#"
proc clock_probe {
  kind L5

  param amount : float [0.0, 1.0] = 0.0

  frame {
    color = vec4(t, beats, dt, 1.0);
  }
}
"#;

    fn checked(source: &str) -> Checked {
        let proc = karakuri_ir::parse(source).expect("a shipped procedure parses");
        karakuri_ir::check::check(&proc).expect("a shipped procedure checks")
    }

    fn present(gpu: &Gpu) -> Present {
        Present::new(&gpu.device, FORMAT, WIDTH, HEIGHT)
    }

    fn params(amount: f32) -> BTreeMap<String, f32> {
        [("amount".to_string(), amount)].into_iter().collect()
    }

    /// What a slot of the shipped chain is called. A function of the source, so
    /// the address a spec names and the address a built slot reports are one
    /// answer.
    fn address(source: &str) -> String {
        format!("test:{:x}", source.len())
    }

    /// One slot of the shipped chain as a description, for the worker to build.
    fn spec(source: &str, cut: Option<Cut>, amount: f32) -> SlotSpec {
        SlotSpec {
            procedure: address(source),
            cut,
            params: params(amount),
        }
    }

    /// One slot of the shipped chain as `ChainSwap` takes it: the description
    /// and the checked source behind it.
    fn asked(source: &str, cut: Option<Cut>, amount: f32) -> ChainSlot {
        ChainSlot {
            spec: spec(source, cut, amount),
            checked: checked(source),
        }
    }

    /// Spins until `done`, or fails after 30 seconds saying what never
    /// happened. Nothing here sleeps on a fixed interval: the wait is on a
    /// worker thread's progress and the failure is a deadline, not a guess.
    fn until(what: &str, mut done: impl FnMut() -> bool) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !done() {
            assert!(std::time::Instant::now() < deadline, "{what}");
            std::thread::yield_now();
        }
    }

    /// One slot of the shipped chain, built against the `Present` that will run
    /// it — see `Present::chain_layout`.
    fn slot(gpu: &Gpu, present: &Present, source: &str, cut: Option<Cut>, amount: f32) -> Slot {
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
    fn hot_points(gpu: &Gpu) -> Points {
        let mut points = Points::new(&gpu.device, 4096, 19274);
        points.resize(WIDTH, HEIGHT);
        points.params.exposure = 6.0;
        points.params.point_scale = 10.0;
        points.prepare(&gpu.queue, 1);
        points
    }

    fn target(gpu: &Gpu, label: &str) -> wgpu::Texture {
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
    fn readback(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u8> {
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
    fn frame(gpu: &Gpu, present: &Present, points: &mut Points) -> Vec<u8> {
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
    fn mixes(gpu: &Gpu, count: usize) -> Vec<wgpu::Texture> {
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
    struct Hand {
        feedback: wgpu::RenderPipeline,
        bloom_bright: wgpu::RenderPipeline,
        bloom_blend: wgpu::RenderPipeline,
        rgb_shift: wgpu::RenderPipeline,
        layout: wgpu::BindGroupLayout,
        sampler: wgpu::Sampler,
        uniform: wgpu::Buffer,
    }

    impl Hand {
        fn new(gpu: &Gpu, present: &Present) -> Hand {
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
            let pipeline_layout =
                gpu.device
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

        fn set(&self, gpu: &Gpu, feedback: f32, bloom: f32, shift: f32) {
            let mut bytes = [0u8; 16];
            bytes[0..4].copy_from_slice(&feedback.to_le_bytes());
            bytes[4..8].copy_from_slice(&bloom.to_le_bytes());
            bytes[8..12].copy_from_slice(&shift.to_le_bytes());
            gpu.queue.write_buffer(&self.uniform, 0, &bytes);
        }

        fn bind(
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

        fn pass(
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

    /// `tests/deck.rs`'s decoder, and it is a copy for that file's own reason:
    /// an `Rgba16Float` readback is bytes, and the workspace has no `f16` type
    /// in it.
    fn f16(bits: u16) -> f32 {
        let sign = f32::from_bits(u32::from(bits & 0x8000) << 16);
        let exponent = (bits >> 10) & 0x1f;
        let mantissa = bits & 0x03ff;
        let magnitude = match exponent {
            0 => f32::from(mantissa) * 2.0f32.powi(-24),
            0x1f if mantissa == 0 => f32::INFINITY,
            0x1f => f32::NAN,
            e => (1.0 + f32::from(mantissa) / 1024.0) * 2.0f32.powi(i32::from(e) - 15),
        };
        if sign.is_sign_negative() {
            -magnitude
        } else {
            magnitude
        }
    }

    fn channels(pixels: &[u8]) -> Vec<f32> {
        pixels
            .chunks_exact(2)
            .map(|c| f16(u16::from_le_bytes([c[0], c[1]])))
            .collect()
    }

    /// The total light in a frame, summed over every colour channel. A blunt
    /// instrument on purpose: bloom and feedback both *add* light, and this is
    /// the one number that says so whatever else moved.
    fn light(pixels: &[u8]) -> f64 {
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

    /// **The default look is what it was before this chain existed**, bit for
    /// bit, and it is the claim the whole design rests on: an empty chain is no
    /// pass at all rather than a pass that does nothing.
    ///
    /// Three ways of having no chain are compared: one never told about a
    /// chain, one told an empty one, and one that held three slots and had them
    /// taken out again. The third is the one that could fail on its own — it
    /// has allocated, recorded and retained, and the picture still has to be
    /// the same picture.
    #[test]
    fn an_empty_chain_is_the_frame_with_no_chain() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = hot_points(&gpu);

        let untouched = frame(&gpu, &present(&gpu), &mut points);

        let mut emptied = present(&gpu);
        drop(emptied.set_chain(&gpu.device, &gpu.queue, Chain::default()));
        assert_eq!(emptied.chain_targets(), 0, "an empty chain took a target");
        assert_eq!(
            frame(&gpu, &emptied, &mut points),
            untouched,
            "a chain explicitly set to empty drew a different frame from no chain at all"
        );

        let mut returned = present(&gpu);
        let full = Chain::new(vec![
            slot(&gpu, &returned, FEEDBACK, Some(Cut::Exit), 0.5),
            slot(&gpu, &returned, BLOOM, None, 0.5),
            slot(&gpu, &returned, RGB_SHIFT, None, 0.5),
        ]);
        drop(returned.set_chain(&gpu.device, &gpu.queue, full));
        frame(&gpu, &returned, &mut points);
        frame(&gpu, &returned, &mut points);
        drop(returned.set_chain(&gpu.device, &gpu.queue, Chain::default()));
        assert_eq!(
            frame(&gpu, &returned, &mut points),
            untouched,
            "a chain filled and emptied again did not put the frame back"
        );
    }

    /// **`examples/rgb_shift.kir` is `master.wgsl`'s `fs_rgb_shift`**, bit for
    /// bit, on a frame that is not square — so the conversion from a fraction
    /// of the frame's *height* into a step along x is part of what is compared.
    #[test]
    fn the_shipped_rgb_shift_is_the_hand_written_pass_bit_for_bit() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let hand = Hand::new(&gpu, &present);

        for amount in [0.25f32, 1.0] {
            let mix = mixes(&gpu, 1).pop().expect("one frame");
            let out = target(&gpu, "hand rgb shift");
            hand.set(&gpu, 0.0, 0.0, amount);
            let src = mix.create_view(&Default::default());
            let bind = hand.bind(&gpu, &src, &src);
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            hand.pass(
                &mut encoder,
                &hand.rgb_shift,
                &bind,
                &out.create_view(&Default::default()),
            );
            gpu.queue.submit([encoder.finish()]);
            let wanted = readback(&gpu, &out);

            drop(present.set_chain(
                &gpu.device,
                &gpu.queue,
                Chain::new(vec![slot(&gpu, &present, RGB_SHIFT, None, amount)]),
            ));
            let mut points = hot_points(&gpu);
            let got = frame(&gpu, &present, &mut points);
            assert_eq!(
                got, wanted,
                "the shipped rgb shift at {amount} is not the pass it replaced"
            );
        }
    }

    /// **`examples/feedback.kir` is `master.wgsl`'s `fs_feedback`**, bit for
    /// bit, with a history that is a picture rather than a black frame.
    ///
    /// The history is the previous frame's mix, which is what `Cut::Mix` means,
    /// so the comparison is run on the *second* frame: the chain reads what it
    /// retained from the first, and the hand-written pass is handed the same
    /// frame from a copy of it.
    #[test]
    fn the_shipped_feedback_is_the_hand_written_pass_bit_for_bit() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let hand = Hand::new(&gpu, &present);
        const AMOUNT: f32 = 0.7;

        // Two frames of the mix: the first is what the chain retains, the
        // second is what it then adds to.
        let mix = mixes(&gpu, 2);
        let out = target(&gpu, "hand feedback");
        hand.set(&gpu, AMOUNT, 0.0, 0.0);
        let bind = hand.bind(
            &gpu,
            &mix[1].create_view(&Default::default()),
            &mix[0].create_view(&Default::default()),
        );
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        hand.pass(
            &mut encoder,
            &hand.feedback,
            &bind,
            &out.create_view(&Default::default()),
        );
        gpu.queue.submit([encoder.finish()]);
        let wanted = readback(&gpu, &out);

        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), AMOUNT)]),
        ));
        let mut points = hot_points(&gpu);
        frame(&gpu, &present, &mut points);
        let got = frame(&gpu, &present, &mut points);
        assert_eq!(
            got, wanted,
            "the shipped feedback is not the pass it replaced"
        );
    }

    /// **`examples/bloom.kir` is not the pass it replaces, and this is where
    /// the difference is measured.**
    ///
    /// The hand-written bloom is two passes with an intermediate target — a
    /// bright-and-blur-x, then a blur-y-and-add that needs **both** that buffer
    /// and the frame it was taken from. A chain slot's output replaces the
    /// frame, so a second slot could not name the earlier value: letting it is
    /// a graph and the chain is a list. So the shipped procedure is one 9x9
    /// kernel where the pair was two 9-tap passes — 81 fetches against 19 — at
    /// the same radius, the same weights and the same knee (ADR-0340's
    /// *Bloom as two chain slots*).
    ///
    /// **What differs is the filtering and not the arithmetic.** The pair's
    /// second half samples an already-blurred buffer with a bilinear tap; the
    /// single pass taps the frame itself at eighty-one offsets. So this asserts
    /// a *tolerance* and prints the number, rather than asserting an equality
    /// that is not true — and the tolerance is relative to the light the frame
    /// holds, because these are unbounded HDR values.
    #[test]
    fn the_shipped_bloom_is_the_hand_written_pair_within_a_stated_tolerance() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let hand = Hand::new(&gpu, &present);
        const AMOUNT: f32 = 0.6;

        let mix = mixes(&gpu, 1).pop().expect("one frame");
        let blur = target(&gpu, "hand bloom blur");
        let out = target(&gpu, "hand bloom");
        hand.set(&gpu, 0.0, AMOUNT, 0.0);
        let src = mix.create_view(&Default::default());
        let blur_view = blur.create_view(&Default::default());
        let self_bind = hand.bind(&gpu, &src, &src);
        let blend_bind = hand.bind(&gpu, &src, &blur_view);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        hand.pass(&mut encoder, &hand.bloom_bright, &self_bind, &blur_view);
        hand.pass(
            &mut encoder,
            &hand.bloom_blend,
            &blend_bind,
            &out.create_view(&Default::default()),
        );
        gpu.queue.submit([encoder.finish()]);
        let two_pass = channels(&readback(&gpu, &out));

        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![slot(&gpu, &present, BLOOM, None, AMOUNT)]),
        ));
        let mut points = hot_points(&gpu);
        let one_pass = channels(&frame(&gpu, &present, &mut points));

        let plain = channels(&readback(&gpu, &mix));
        let peak = plain.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1.0);
        let (worst, mean) = two_pass
            .iter()
            .zip(&one_pass)
            .fold((0.0f32, 0.0f64), |(worst, sum), (a, b)| {
                ((a - b).abs().max(worst), sum + f64::from((a - b).abs()))
            });
        let mean = mean / two_pass.len() as f64;
        println!(
            "bloom: one 9x9 pass against the separable pair at amount {AMOUNT} — \
             worst channel {worst:.4}, mean {mean:.6}, against a frame peaking at {peak:.3}"
        );
        // **The same picture, and the tolerance says how nearly.** 2% of the
        // frame's peak: enough to admit a bilinear tap's difference and far
        // too tight to admit a different radius, a different knee or a
        // different normalisation, each of which moves the blurred light by
        // tens of percent.
        assert!(
            worst < 0.02 * peak,
            "the shipped bloom differs from the pair it replaces by {worst} \
             against a peak of {peak} — that is not a filter's difference"
        );
        // And it is bloom rather than a copy: the light went up.
        assert!(
            light(&frame(&gpu, &present, &mut hot_points(&gpu)))
                > light(&readback(&gpu, &mix)) * 1.001
        );
    }

    /// **A retention is allocated only where a slot's answer names one, and at
    /// most two ever** — one per cut, however many slots read them.
    ///
    /// The rule ADR-0317 wrote for one fixed pass, at a list's width: *a cut
    /// that is read has to be held, so holding one nothing reads is the cost
    /// nobody would pay* (P-0091).
    #[test]
    fn a_cut_is_held_only_where_a_slot_asked_for_it() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);

        // Nothing retains: two targets, both of them the chain's own.
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &present, BLOOM, None, 0.5),
                slot(&gpu, &present, RGB_SHIFT, None, 0.5),
            ]),
        ));
        assert_eq!(present.chain_retained(), Vec::new());
        assert_eq!(present.chain_targets(), 2, "the entry and one intermediate");

        // One slot retaining: one cut held.
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &present, FEEDBACK, Some(Cut::Exit), 0.5),
                slot(&gpu, &present, RGB_SHIFT, None, 0.5),
            ]),
        ));
        assert_eq!(present.chain_retained(), vec![Cut::Exit]);
        assert_eq!(present.chain_targets(), 3);

        // **Two slots reading the same cut read one frame**, so it is still one
        // target.
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.5),
                slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.25),
            ]),
        ));
        assert_eq!(present.chain_retained(), vec![Cut::Mix]);
        assert_eq!(present.chain_targets(), 3);

        // Two slots naming *different* cuts: both, and that is the ceiling.
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.5),
                slot(&gpu, &present, BLOOM, None, 0.5),
                slot(&gpu, &present, FEEDBACK, Some(Cut::Exit), 0.25),
            ]),
        ));
        assert_eq!(present.chain_retained(), vec![Cut::Mix, Cut::Exit]);
        assert_eq!(
            present.chain_targets(),
            5,
            "the entry, two intermediates and the two cuts"
        );
    }

    /// **The two cuts are two pictures**, which is why the cut is the slot's
    /// answer rather than a decision the design took.
    ///
    /// `Cut::Mix` reads the frame as the mix wrote it, so what comes back has
    /// been through nothing; `Cut::Exit` reads the chain's own output, so what
    /// comes back has already been bloomed and shifted and is bloomed and
    /// shifted again.
    #[test]
    fn the_two_cuts_are_two_pictures() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |cut: Cut| {
            let mut present = present(&gpu);
            let chain = Chain::new(vec![
                slot(&gpu, &present, FEEDBACK, Some(cut), 0.8),
                slot(&gpu, &present, BLOOM, None, 0.8),
                slot(&gpu, &present, RGB_SHIFT, None, 0.5),
            ]);
            drop(present.set_chain(&gpu.device, &gpu.queue, chain));
            let mut points = hot_points(&gpu);
            let mut last = Vec::new();
            for _ in 0..4 {
                last = frame(&gpu, &present, &mut points);
            }
            last
        };

        let from_mix = run(Cut::Mix);
        let from_exit = run(Cut::Exit);
        assert_ne!(
            from_mix, from_exit,
            "the two cuts drew the same frame, so nothing chooses between them"
        );
        // The exit compounds — what is read back already contains the trail —
        // so four frames of it hold more light than four frames of a trail
        // that never feeds itself.
        assert!(
            light(&from_exit) > light(&from_mix),
            "the compounding cut held {} against the mix cut's {}",
            light(&from_exit),
            light(&from_mix)
        );
    }

    /// **A slot reading the mix cut reads the previous frame's mix wherever it
    /// sits in the list**, which is the position-independence the entry target
    /// buys.
    ///
    /// `master.rs` used to copy the mix cut *between* two passes, because the
    /// mix's own target was also the second pass's destination. A list cannot
    /// honour that: the slot that reads the cut may be anywhere. So the entry
    /// is held apart from the ping-pong pair and the copy happens at the end,
    /// and this is the frame that says it worked — feedback second in the list
    /// still trails.
    #[test]
    fn the_mix_cut_is_the_previous_frame_wherever_the_slot_sits() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &present, RGB_SHIFT, None, 0.4),
                slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.8),
            ]),
        ));
        let mut points = hot_points(&gpu);
        let first = frame(&gpu, &present, &mut points);
        let second = frame(&gpu, &present, &mut points);
        assert_ne!(
            first, second,
            "a feedback slot second in the list retained nothing"
        );

        // And the same chain with nothing retained draws a different frame,
        // which is what says the trail is the retention rather than the shift.
        let mut plain = self::present(&gpu);
        drop(plain.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![slot(&gpu, &plain, RGB_SHIFT, None, 0.4)]),
        ));
        let mut points = hot_points(&gpu);
        frame(&gpu, &plain, &mut points);
        assert_ne!(
            frame(&gpu, &plain, &mut points),
            second,
            "the second frame was the same with and without a feedback slot"
        );
    }

    /// **The first frame of a run with a retaining slot reads a black
    /// history.**
    ///
    /// Which is what makes a run and its replay agree on frame one: nothing has
    /// been retained yet, and a freshly allocated target reads as zero. If it
    /// did not, this frame would carry whatever the driver left in that memory
    /// and no two runs would agree.
    #[test]
    fn the_first_frame_of_a_retaining_run_reads_a_black_history() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut points = hot_points(&gpu);

        let plain = frame(&gpu, &present(&gpu), &mut points);

        let mut fed = present(&gpu);
        drop(fed.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![slot(&gpu, &fed, FEEDBACK, Some(Cut::Mix), 0.95)]),
        ));
        assert_eq!(
            frame(&gpu, &fed, &mut hot_points(&gpu)),
            plain,
            "the first frame of a feedback run added something to itself"
        );
    }

    /// **A run with a retention is bit-exact across two runs**, which is the
    /// whole of what makes the retained frame part of the state a replay
    /// reproduces rather than a thing the picture picked up along the way
    /// (`docs/principles/0092-…`).
    ///
    /// Both cuts, because they are held from different targets and only one of
    /// them is downstream of the passes.
    #[test]
    fn a_retaining_run_is_bit_exact_across_two_runs() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |cut: Cut| {
            let mut present = present(&gpu);
            let chain = Chain::new(vec![
                slot(&gpu, &present, FEEDBACK, Some(cut), 0.7),
                slot(&gpu, &present, BLOOM, None, 0.4),
                slot(&gpu, &present, RGB_SHIFT, None, 0.3),
            ]);
            drop(present.set_chain(&gpu.device, &gpu.queue, chain));
            let mut points = hot_points(&gpu);
            let mut frames = Vec::new();
            for _ in 0..5 {
                frames.push(frame(&gpu, &present, &mut points));
            }
            frames
        };

        for cut in Cut::ALL {
            assert_eq!(
                run(cut),
                run(cut),
                "two runs of the same records diverged with the {} cut",
                cut.name()
            );
        }
    }

    /// **A parameter move is a uniform write and not a build**, and this is
    /// what says the two paths draw the same frame.
    ///
    /// `Record::MasterChain` is written whole, so applying one is ordinarily a
    /// list whose shape is the shape already running with one number different
    /// — and `Present::set_chain_params` takes it without allocating. A frame
    /// drawn that way has to be the frame a full rebuild would have drawn, or
    /// the cheap path is a second answer (P-0091).
    #[test]
    fn moving_a_parameter_draws_what_rebuilding_the_list_draws() {
        let gpu = Gpu::headless().expect("no GPU available");
        let shape = |present: &Present| -> Vec<(String, Option<Cut>)> { present.chain_shape() };

        let mut built = present(&gpu);
        drop(built.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &built, BLOOM, None, 0.9),
                slot(&gpu, &built, RGB_SHIFT, None, 0.2),
            ]),
        ));
        let wanted = frame(&gpu, &built, &mut hot_points(&gpu));

        let mut moved = present(&gpu);
        drop(moved.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &moved, BLOOM, None, 0.1),
                slot(&gpu, &moved, RGB_SHIFT, None, 0.7),
            ]),
        ));
        let before = moved.chain_targets();
        assert!(
            moved.set_chain_params(&gpu.queue, &shape(&moved), &[params(0.9), params(0.2)]),
            "the shape that is running was refused as a parameter move"
        );
        assert_eq!(
            moved.chain_targets(),
            before,
            "a parameter move allocated a target"
        );
        assert_eq!(
            frame(&gpu, &moved, &mut hot_points(&gpu)),
            wanted,
            "the cheap path drew a different frame from the built one"
        );

        // **And a different list is refused rather than half-applied**, which
        // is what sends the caller to build one.
        assert!(
            !moved.set_chain_params(&gpu.queue, &[("elsewhere".into(), None)], &[params(0.5)]),
            "a shape that is not running was taken as a parameter move"
        );
    }

    /// **The chain's price is the sum over its slots**, which is not the
    /// addition ADR-0013 forbids: those are three different quantities, these
    /// are rates against one — the frame's texels, covered once by every slot.
    ///
    /// The figures are `karakuri_ir::cost`'s own and are asserted here as a sum
    /// rather than as three numbers, because the three are `cost.rs`'s to hold.
    /// What the sum becomes in milliseconds is
    /// `karakuri_engine::estimate::chain_ms`, and what spends it is
    /// `Deck::govern` — see `the_three_shipped_procedures_price_the_chains_rate`
    /// below.
    #[test]
    fn a_chains_price_is_the_sum_of_its_slots() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        assert_eq!(
            present.chain_ops_per_fragment(),
            0,
            "an empty chain is free"
        );

        let one = Chain::new(vec![slot(&gpu, &present, RGB_SHIFT, None, 0.5)]);
        let alone = one.ops_per_fragment();
        drop(present.set_chain(&gpu.device, &gpu.queue, one));
        assert_eq!(present.chain_ops_per_fragment(), alone);

        let three = Chain::new(vec![
            slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.5),
            slot(&gpu, &present, BLOOM, None, 0.5),
            slot(&gpu, &present, RGB_SHIFT, None, 0.5),
        ]);
        let sum: u32 = three.slots().iter().map(|s| s.ops_per_fragment()).sum();
        assert_eq!(three.ops_per_fragment(), sum);
        println!(
            "chain cost: feedback {}, bloom {}, rgb shift {} — {sum} ops per fragment together",
            three.slots()[0].ops_per_fragment(),
            three.slots()[1].ops_per_fragment(),
            three.slots()[2].ops_per_fragment(),
        );
        drop(present.set_chain(&gpu.device, &gpu.queue, three));
        assert_eq!(present.chain_ops_per_fragment(), sum);
    }

    /// A chain slot's procedure reads the clock the frame hands it, and it
    /// reads all three of `t`, `beats` and `dt`.
    ///
    /// The three shipped procedures read none of them. [`CLOCK`] writes the
    /// clock into the frame's colour channels, so every texel of the readback
    /// is a statement about what the shader was handed.
    ///
    /// Two clocks are run and not one: the second is the negative control, and
    /// the picture has to move with it.
    #[test]
    fn a_chain_slot_reads_the_clock_the_frame_hands_it() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let mut points = hot_points(&gpu);
        drop(present.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![slot(&gpu, &present, CLOCK, None, 0.0)]),
        ));

        let read = |present: &Present, points: &mut Points, clock: Clock| {
            present.set_chain_clock(&gpu.queue, clock);
            assert_eq!(
                present.chain_clock(),
                clock,
                "the chain did not keep the clock it was written"
            );
            let pixels = frame(&gpu, present, points);
            let texel = channels(&pixels[..8]);
            (texel[0], texel[1], texel[2])
        };

        let first = Clock {
            t: 2.5,
            beats: 5.25,
            dt: 0.25,
            seed_salt: 0,
        };
        let (t, beats, dt) = read(&present, &mut points, first);
        assert!(
            (t - first.t).abs() < 1e-3 && (beats - first.beats).abs() < 1e-3,
            "the slot read t {t} and beats {beats} against {first:?}"
        );
        assert!((dt - first.dt).abs() < 1e-4, "the slot read dt {dt}");

        let second = Clock {
            t: 7.0,
            beats: 14.0,
            dt: 0.125,
            seed_salt: 0,
        };
        let (t, beats, dt) = read(&present, &mut points, second);
        assert!(
            (t - second.t).abs() < 1e-3 && (beats - second.beats).abs() < 1e-3,
            "the slot read t {t} and beats {beats} against {second:?}"
        );
        assert!((dt - second.dt).abs() < 1e-4, "the slot read dt {dt}");
    }

    /// A parameter write leaves the running clock where it is.
    /// `set_chain_params` repacks a whole uniform block, and the clock it
    /// repacks is the one that is running.
    #[test]
    fn moving_a_parameter_leaves_the_clock_where_it_is() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let mut points = hot_points(&gpu);
        let built = Chain::new(vec![slot(&gpu, &present, CLOCK, None, 0.0)]);
        let shape: Vec<(String, Option<Cut>)> = built
            .slots()
            .iter()
            .map(|s| (s.proc().to_string(), s.cut()))
            .collect();
        drop(present.set_chain(&gpu.device, &gpu.queue, built));

        let clock = Clock {
            t: 3.0,
            beats: 6.0,
            dt: 0.25,
            seed_salt: 0,
        };
        present.set_chain_clock(&gpu.queue, clock);
        assert!(
            present.set_chain_params(&gpu.queue, &shape, &[params(1.0)]),
            "the shape handed back is the shape that is running"
        );
        let pixels = frame(&gpu, &present, &mut points);
        let texel = channels(&pixels[..8]);
        assert!(
            (texel[0] - clock.t).abs() < 1e-3,
            "a parameter write moved the clock to {}",
            texel[0]
        );
    }

    /// The three shipped procedures are what the chain's rate was calibrated
    /// on, so the price of that chain at the size the measurement was taken at
    /// comes back as the measurement.
    ///
    /// It is the order of magnitude that is held, not the third decimal.
    #[test]
    fn the_three_shipped_procedures_price_the_chains_rate() {
        use karakuri_engine::estimate::{
            chain_ms, CHAIN_REFERENCE_MS, CHAIN_REFERENCE_OPS, CHAIN_REFERENCE_SIZE,
        };

        let gpu = Gpu::headless().expect("no GPU available");
        let present = present(&gpu);
        let three = Chain::new(vec![
            slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.5),
            slot(&gpu, &present, BLOOM, None, 0.5),
            slot(&gpu, &present, RGB_SHIFT, None, 0.5),
        ]);
        assert_eq!(
            three.ops_per_fragment(),
            CHAIN_REFERENCE_OPS,
            "the shipped three no longer cost what the chain's rate was calibrated on"
        );

        let ms = chain_ms(three.ops_per_fragment(), CHAIN_REFERENCE_SIZE);
        assert!(
            (ms - CHAIN_REFERENCE_MS).abs() < 0.01,
            "the shipped three price at {ms:.3} ms against a measured {CHAIN_REFERENCE_MS:.2}"
        );

        assert_eq!(
            chain_ms(0, CHAIN_REFERENCE_SIZE),
            0.0,
            "an empty chain is free"
        );
        let doubled = chain_ms(three.ops_per_fragment(), (2560, 720));
        assert!(
            (doubled - 2.0 * ms).abs() < 1e-3,
            "twice the area priced at {doubled:.3} ms against {:.3}",
            2.0 * ms
        );
    }

    /// **What a slot is refused for**, and each refusal is about the chain
    /// rather than about the language: a `kind` that is not L5, a cut answered
    /// where the file declares no `retains` and the other way round, and a
    /// Texture slot the chain has no `edge` to bind.
    #[test]
    fn a_procedure_that_cannot_be_a_chain_slot_is_refused_with_the_reason() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = present(&gpu);
        let build = |source: &str, cut: Option<Cut>| {
            Slot::build(
                &gpu.device,
                present.chain_layout(),
                "test:refused",
                &checked(source),
                cut,
                params(0.5),
            )
            .err()
            .map(|e| e.to_string())
        };

        assert!(build(FEEDBACK, Some(Cut::Mix)).is_none());
        let bare = build(FEEDBACK, None).expect("`retains` with no cut is refused");
        assert!(bare.contains("mix") && bare.contains("exit"), "{bare}");
        let extra = build(BLOOM, Some(Cut::Exit)).expect("a cut with no `retains` is refused");
        assert!(extra.contains("retains"), "{extra}");
    }

    /// **Unified image pass and retention manager abstractions.**
    #[test]
    fn unified_image_pass_and_retention_abstractions_record() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = present(&gpu);
        let s = slot(&gpu, &present, FEEDBACK, Some(Cut::Mix), 0.5);

        assert_eq!(s.pass().name(), "feedback");
        assert_eq!(s.name(), "feedback");

        let mut retention = RetentionManager::new(&gpu.device);
        assert_eq!(retention.count(), 0);
        let mix_src = target(&gpu, "mix src").create_view(&Default::default());
        let exit_src = target(&gpu, "exit src").create_view(&Default::default());
        retention.allocate(
            &gpu.device,
            WIDTH,
            HEIGHT,
            &[Cut::Mix, Cut::Exit],
            Some(&mix_src),
            Some(&exit_src),
        );
        assert_eq!(retention.count(), 2);
        assert!(retention.is_held(Cut::Mix));
        assert!(retention.is_held(Cut::Exit));
        assert!(retention.held(Cut::Mix).is_some());

        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        retention.record(&mut encoder);
        gpu.queue.submit([encoder.finish()]);

        // Verify bound pass implements RenderPassNode
        let dst = target(&gpu, "dst target").create_view(&Default::default());
        let held = target(&gpu, "held target").create_view(&Default::default());
        let sampler = gpu.device.create_sampler(&Default::default());
        let bg = s.pass().bind(
            &gpu.device,
            present.chain_layout(),
            &mix_src,
            &held,
            &sampler,
            Some("test bg"),
        );
        let bound = s.bound(&bg);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        bound.record(&mut encoder, &dst);
        gpu.queue.submit([encoder.finish()]);
    }

    /// **A chain built on `karakuri-chain` draws what a chain built here
    /// draws**, and it lands at a frame boundary rather than when the build
    /// finished.
    ///
    /// Three slots including a retention, because that is the whole of what a
    /// build allocates: a pipeline and a uniform buffer per slot, an entry
    /// target, the ping-pong between slots, and the history the `mix` cut is
    /// held in. A worker that made any of them differently would draw a
    /// different frame here.
    ///
    /// The frame between the build finishing and the boundary is the one that
    /// says the install waits: the empty chain draws it, and it is not the
    /// frame the built chain draws (P-0094 — the show does not stop).
    #[test]
    fn a_chain_built_on_the_worker_draws_what_building_it_here_draws() {
        let gpu = Gpu::headless().expect("no GPU available");

        let mut here = present(&gpu);
        drop(here.set_chain(
            &gpu.device,
            &gpu.queue,
            Chain::new(vec![
                slot(&gpu, &here, FEEDBACK, Some(Cut::Mix), 0.7),
                slot(&gpu, &here, BLOOM, None, 0.4),
                slot(&gpu, &here, RGB_SHIFT, None, 0.3),
            ]),
        ));
        let wanted = frame(&gpu, &here, &mut hot_points(&gpu));

        let mut there = present(&gpu);
        let mut swap = ChainSwap::new(&gpu.device, &gpu.queue);
        let id = swap.request(
            &there,
            vec![
                asked(FEEDBACK, Some(Cut::Mix), 0.7),
                asked(BLOOM, None, 0.4),
                asked(RGB_SHIFT, None, 0.3),
            ],
        );
        assert_eq!(
            there.chain_len(),
            0,
            "asking for a chain installed one on the calling thread"
        );

        until("the worker never finished the build", || swap.built() == 1);
        assert_eq!(
            there.chain_len(),
            0,
            "a finished build installed itself without a frame boundary"
        );
        let meanwhile = frame(&gpu, &there, &mut hot_points(&gpu));
        assert_ne!(
            meanwhile, wanted,
            "the frame before the boundary was already the built chain's"
        );

        swap.begin_frame(&mut there, &gpu.device, &gpu.queue);
        assert_eq!(swap.installs(), 1, "the frame boundary installed nothing");
        assert_eq!(there.chain_len(), 3);
        assert_eq!(
            there.chain_shape(),
            vec![
                (address(FEEDBACK), Some(Cut::Mix)),
                (address(BLOOM), None),
                (address(RGB_SHIFT), None),
            ]
        );
        let ids: Vec<u64> = swap
            .events()
            .map(|event| match event {
                ChainEvent::Installed { id, slots } => {
                    assert_eq!(slots, 3);
                    id
                }
                other => panic!("{other}"),
            })
            .collect();
        assert_eq!(ids, vec![id]);

        assert_eq!(
            frame(&gpu, &there, &mut hot_points(&gpu)),
            wanted,
            "the chain built on the worker drew a different frame from the one built here"
        );
    }

    /// **Two builds waiting at one frame boundary install the newest, once**,
    /// and the one it superseded is retired without ever being seen.
    ///
    /// Both are waited for before the boundary is taken, so this is about which
    /// one an install chooses rather than about how fast either was.
    #[test]
    fn the_newest_of_two_pending_builds_wins() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let mut swap = ChainSwap::new(&gpu.device, &gpu.queue);

        let stale = swap.request(&present, vec![asked(BLOOM, None, 0.4)]);
        let newest = swap.request(
            &present,
            vec![asked(RGB_SHIFT, None, 0.3), asked(BLOOM, None, 0.9)],
        );
        assert_ne!(stale, newest);
        until("the worker never finished both builds", || {
            swap.built() == 2
        });

        swap.begin_frame(&mut present, &gpu.device, &gpu.queue);
        assert_eq!(
            swap.installs(),
            1,
            "one frame boundary installed more than one chain"
        );
        assert_eq!(
            present.chain_shape(),
            vec![(address(RGB_SHIFT), None), (address(BLOOM), None)],
            "the boundary installed the build the newer one superseded"
        );
        let ids: Vec<u64> = swap
            .events()
            .map(|event| match event {
                ChainEvent::Installed { id, .. } => id,
                other => panic!("{other}"),
            })
            .collect();
        assert_eq!(
            ids,
            vec![newest],
            "the superseded build was announced as well as retired"
        );

        // **And nothing is left waiting**: a second boundary with no request
        // behind it installs nothing.
        swap.begin_frame(&mut present, &gpu.device, &gpu.queue);
        assert_eq!(swap.installs(), 1);
        assert!(swap.pending_events().is_empty());
    }
}
