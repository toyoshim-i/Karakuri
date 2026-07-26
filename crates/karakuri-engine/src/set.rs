//! The Set: filled slots forming one video source, and the unit of both
//! compilation and lifecycle.
//!
//! A Set owns the buffers, bind groups, and pipelines that a pair of generated
//! shaders needs, and it is what a structural change forks. Nothing here
//! mutates a live Set in place; parameter values are the one exception, and
//! they are uniform writes.
//!
//! What runs here is generated, not written. `Set::build` takes two checked
//! procedures, asks `karakuri-codegen` for WGSL, and creates pipelines against
//! the binding layout that crate publishes — the engine never reads the
//! generated text to find out where anything is bound.

use std::collections::HashMap;

use karakuri_codegen::layout::{group, AttrSlot, UniformLayout, WORKGROUP_SIZE};
use karakuri_codegen::{generate_l1, generate_l4};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

use crate::camera::Orbit;
use crate::uniforms::UniformPacker;
use crate::video_source::VideoSource;

#[derive(Debug, thiserror::Error)]
pub enum SetError {
    #[error("slot {slot} needs a {expected:?} procedure, got {actual:?}")]
    WrongKind {
        slot: &'static str,
        expected: Kind,
        actual: Kind,
    },
    #[error("capacity {requested} is outside the range [{min}, {max}] that `{proc}` declares")]
    Capacity {
        proc: String,
        requested: u32,
        min: u32,
        max: u32,
    },
    #[error("`{0}` declares no capacity range")]
    NoCapacity(String),
}

/// Bytes per element in every attribute buffer. Uniform across attributes by
/// construction — see the layout contract — so sizing needs no case analysis.
const ATTR_STRIDE: u64 = 16;

/// One attribute's pair of storage buffers.
struct AttrPair {
    a: wgpu::Buffer,
    b: wgpu::Buffer,
}

impl AttrPair {
    /// `parity` picks which physical buffer is currently "previous". The
    /// generated shader never learns about this: prev and next keep fixed
    /// binding numbers and the engine swaps what backs them.
    fn prev(&self, parity: bool) -> &wgpu::Buffer {
        if parity {
            &self.a
        } else {
            &self.b
        }
    }

    fn next(&self, parity: bool) -> &wgpu::Buffer {
        if parity {
            &self.b
        } else {
            &self.a
        }
    }
}

pub struct Set {
    capacity: u32,
    live_count: u32,
    seed_base: u32,
    seed_salt: u32,
    /// Simulation time. Advanced by `steps * dt`, never read from a clock.
    t: f32,
    dt: f32,
    viewport: [f32; 2],
    parity: bool,
    has_spawn: bool,

    attrs: HashMap<String, AttrPair>,

    l1_uniform_layout: UniformLayout,
    l4_uniform_layout: UniformLayout,
    l1_uniforms: wgpu::Buffer,
    l4_uniforms: wgpu::Buffer,

    element: wgpu::ComputePipeline,
    /// Present only when the procedure declares a `spawn` block. Wiring it up
    /// waits on compaction: until the live set is compacted, there is no
    /// contiguous free range for new elements to be written into.
    #[allow(dead_code)]
    spawn: Option<wgpu::ComputePipeline>,
    render: wgpu::RenderPipeline,

    l1_uniform_bg: wgpu::BindGroup,
    l4_uniform_bg: wgpu::BindGroup,
    /// Indexed by parity: [false, true].
    prev_bg: [wgpu::BindGroup; 2],
    next_bg: [wgpu::BindGroup; 2],
    l4_attr_bg: [wgpu::BindGroup; 2],

    pub params: HashMap<String, f32>,
    pub camera: Orbit,
    l1_param_names: Vec<String>,
    l4_param_names: Vec<String>,
}

impl Set {
    /// Compile two checked procedures into a runnable Set.
    ///
    /// `capacity` is a Set-level dial, not part of either procedure's identity,
    /// so it is passed in here and validated against the range the L1 artifact
    /// declares rather than read out of it.
    ///
    /// There is no target-format parameter: every `VideoSource` renders
    /// `Rgba16Float`, and the conversion to whatever the display wants happens
    /// once, in the present pass.
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1: &Checked,
        l4: &Checked,
        capacity: u32,
        seed_salt: u32,
    ) -> Result<Set, SetError> {
        if l1.kind != Kind::L1 {
            return Err(SetError::WrongKind {
                slot: "L1",
                expected: Kind::L1,
                actual: l1.kind,
            });
        }
        if l4.kind != Kind::L4 {
            return Err(SetError::WrongKind {
                slot: "L4",
                expected: Kind::L4,
                actual: l4.kind,
            });
        }
        let range = l1
            .capacity
            .ok_or_else(|| SetError::NoCapacity(l1.name.clone()))?;
        if !range.contains(capacity) {
            return Err(SetError::Capacity {
                proc: l1.name.clone(),
                requested: capacity,
                min: range.min,
                max: range.max,
            });
        }

        let l1_shader = generate_l1(l1);
        let l4_shader = generate_l4(l4);

        let l1_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} (L1)", l1.name)),
            source: wgpu::ShaderSource::Wgsl(l1_shader.source.as_str().into()),
        });
        let l4_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} (L4)", l4.name)),
            source: wgpu::ShaderSource::Wgsl(l4_shader.source.as_str().into()),
        });

        // -- buffers ------------------------------------------------------
        let buffer_size = u64::from(capacity) * ATTR_STRIDE;
        let mut attrs = HashMap::new();
        for slot in &l1_shader.attr_slots {
            let make = |suffix: &str| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("{}_{suffix}", slot.name)),
                    size: buffer_size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            };
            attrs.insert(
                slot.name.to_string(),
                AttrPair {
                    a: make("a"),
                    b: make("b"),
                },
            );
        }

        let l1_uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("L1 uniforms"),
            size: u64::from(l1_shader.uniform_layout.total_size),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let l4_uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("L4 uniforms"),
            size: u64::from(l4_shader.uniform_layout.total_size),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // -- bind group layouts -------------------------------------------
        let uniform_bgl = |label| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
        };
        let storage_bgl = |label, slots: &[AttrSlot], read_only, vis| {
            let entries: Vec<_> = slots
                .iter()
                .map(|s| wgpu::BindGroupLayoutEntry {
                    binding: s.binding,
                    visibility: vis,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                })
                .collect();
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &entries,
            })
        };

        let l1_uniform_bgl = uniform_bgl("L1 uniforms");
        let l4_uniform_bgl = uniform_bgl("L4 uniforms");
        let prev_bgl = storage_bgl(
            "prev",
            &l1_shader.attr_slots,
            true,
            wgpu::ShaderStages::COMPUTE,
        );
        let next_bgl = storage_bgl(
            "next",
            &l1_shader.attr_slots,
            false,
            wgpu::ShaderStages::COMPUTE,
        );
        let l4_attr_bgl = storage_bgl(
            "attrs",
            &l4_shader.attr_slots,
            true,
            wgpu::ShaderStages::VERTEX_FRAGMENT,
        );

        // -- bind groups ---------------------------------------------------
        let bind_uniform = |label, bgl: &wgpu::BindGroupLayout, buf: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                }],
            })
        };
        let l1_uniform_bg = bind_uniform("L1 uniforms", &l1_uniform_bgl, &l1_uniforms);
        let l4_uniform_bg = bind_uniform("L4 uniforms", &l4_uniform_bgl, &l4_uniforms);

        let bind_attrs = |label: &str,
                          bgl: &wgpu::BindGroupLayout,
                          slots: &[AttrSlot],
                          parity: bool,
                          next: bool| {
            let entries: Vec<_> = slots
                .iter()
                .map(|s| {
                    let pair = &attrs[s.name];
                    let buf = if next {
                        pair.next(parity)
                    } else {
                        pair.prev(parity)
                    };
                    wgpu::BindGroupEntry {
                        binding: s.binding,
                        resource: buf.as_entire_binding(),
                    }
                })
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: bgl,
                entries: &entries,
            })
        };

        let prev_bg = [
            bind_attrs("prev0", &prev_bgl, &l1_shader.attr_slots, false, false),
            bind_attrs("prev1", &prev_bgl, &l1_shader.attr_slots, true, false),
        ];
        let next_bg = [
            bind_attrs("next0", &next_bgl, &l1_shader.attr_slots, false, true),
            bind_attrs("next1", &next_bgl, &l1_shader.attr_slots, true, true),
        ];
        // L4 reads what L1 last wrote. The frame's compute pass writes "next",
        // then parity flips, so L4 binds the same physical buffers that are
        // "prev" under the flipped parity.
        let l4_attr_bg = [
            bind_attrs("l4attrs0", &l4_attr_bgl, &l4_shader.attr_slots, false, false),
            bind_attrs("l4attrs1", &l4_attr_bgl, &l4_shader.attr_slots, true, false),
        ];

        // -- pipelines ------------------------------------------------------
        let compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("L1"),
            bind_group_layouts: &[&l1_uniform_bgl, &prev_bgl, &next_bgl],
            push_constant_ranges: &[],
        });
        let compute = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&compute_layout),
                module: &l1_module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let element = compute("element");
        let spawn = l1_shader.has_spawn.then(|| compute("spawn"));

        let render_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("L4"),
            bind_group_layouts: &[&l4_uniform_bgl, &l4_attr_bgl],
            push_constant_ranges: &[],
        });
        let render = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&l4.name),
            layout: Some(&render_layout),
            vertex: wgpu::VertexState {
                module: &l4_module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &l4_module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    // Always the linear HDR format, never the surface's. A
                    // `VideoSource` renders into the HDR target and the present
                    // pass is the one place that encodes to sRGB; taking this
                    // as a parameter would let a caller quietly break "the
                    // pipeline is linear and HDR end to end".
                    format: crate::present::Present::HDR_FORMAT,
                    // `blend additive`, no depth write.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let params = l1
            .params
            .iter()
            .chain(l4.params.iter())
            .filter_map(|p| default_scalar(p).map(|v| (p.name.clone(), v)))
            .collect();

        let mut set = Set {
            capacity,
            // A procedure with no `spawn` block has every element live from
            // frame zero; one with a spawn block starts empty and fills.
            live_count: if l1_shader.has_spawn { 0 } else { capacity },
            seed_base: 0,
            seed_salt,
            t: 0.0,
            dt: 1.0 / 60.0,
            viewport: [1.0, 1.0],
            parity: false,
            has_spawn: l1_shader.has_spawn,
            attrs,
            l1_uniform_layout: l1_shader.uniform_layout.clone(),
            l4_uniform_layout: l4_shader.uniform_layout.clone(),
            l1_uniforms,
            l4_uniforms,
            element,
            spawn,
            render,
            l1_uniform_bg,
            l4_uniform_bg,
            prev_bg,
            next_bg,
            l4_attr_bg,
            params,
            camera: Orbit::default(),
            l1_param_names: l1.params.iter().map(|p| p.name.clone()).collect(),
            l4_param_names: l4.params.iter().map(|p| p.name.clone()).collect(),
        };
        set.initialize(device, queue);
        Ok(set)
    }

    /// Zero the attribute buffers and, for a procedure with no `spawn` block,
    /// seed each slot with its own index — `seed` equals the initial slot index
    /// there, which is what makes lattice generators work as written.
    fn initialize(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        let zeros = vec![0u8; (u64::from(self.capacity) * ATTR_STRIDE) as usize];
        for pair in self.attrs.values() {
            queue.write_buffer(&pair.a, 0, &zeros);
            queue.write_buffer(&pair.b, 0, &zeros);
        }

        if !self.has_spawn {
            let mut seeds = vec![0u8; zeros.len()];
            for i in 0..self.capacity {
                let at = (i as usize) * ATTR_STRIDE as usize;
                seeds[at..at + 4].copy_from_slice(&i.to_le_bytes());
            }
            let pair = &self.attrs["seed"];
            queue.write_buffer(&pair.a, 0, &seeds);
            queue.write_buffer(&pair.b, 0, &seeds);

            // Everything is alive, and nothing needs a birth-fraction
            // correction because nothing was born mid-frame.
            let ones_u32 = filled(self.capacity, &1u32.to_le_bytes());
            let ones_f32 = filled(self.capacity, &1.0f32.to_le_bytes());
            for (name, data) in [("alive", &ones_u32), ("birth_frac", &ones_f32)] {
                let pair = &self.attrs[name];
                queue.write_buffer(&pair.a, 0, data);
                queue.write_buffer(&pair.b, 0, data);
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport = [width.max(1) as f32, height.max(1) as f32];
    }

    pub fn time(&self) -> f32 {
        self.t
    }

    pub fn live_count(&self) -> u32 {
        self.live_count
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Uploads uniforms and advances simulation time. A parameter change is a
    /// uniform write, which is why it does not need a fork.
    pub fn prepare(&mut self, queue: &wgpu::Queue, steps: u8) {
        self.t += self.dt * f32::from(steps);

        let mut p = UniformPacker::new(&self.l1_uniform_layout);
        p.f32("t", self.t)
            .f32("dt", self.dt)
            .u32("capacity", self.capacity)
            .u32("seed_salt", self.seed_salt)
            .u32("live_count", self.live_count)
            .u32("spawn_count", 0)
            .u32("seed_base", self.seed_base);
        for name in &self.l1_param_names {
            p.f32(name, self.params[name]);
        }
        queue.write_buffer(&self.l1_uniforms, 0, &p.finish());

        let aspect = self.viewport[0] / self.viewport[1];
        let mut p = UniformPacker::new(&self.l4_uniform_layout);
        p.f32("t", self.t)
            .u32("seed_salt", self.seed_salt)
            .vec2("viewport", self.viewport)
            .mat4("camera", self.camera.view_proj(self.t, aspect));
        for name in &self.l4_param_names {
            p.f32(name, self.params[name]);
        }
        queue.write_buffer(&self.l4_uniforms, 0, &p.finish());
    }

    fn workgroups(count: u32) -> u32 {
        count.div_ceil(WORKGROUP_SIZE)
    }
}

impl VideoSource for Set {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        steps: u8,
    ) {
        // One element pass per step, not one per frame. `steps` is what the
        // tick record carries, and the whole point of substepping is that the
        // simulation state at a given `t` does not depend on frame rate — so
        // advancing `t` by `steps * dt` while stepping once would desynchronise
        // the two. At `steps == 0` nothing runs and parity does not flip, so a
        // paused frame renders exactly what the previous one did.
        //
        // `t` is constant across a frame's substeps, the same way parameters
        // and signal bindings are. Substepping exists to keep integration
        // stable under load, not to give external forcing a finer clock.
        for _ in 0..steps {
            if self.live_count == 0 {
                break;
            }
            let parity = usize::from(self.parity);
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("element"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.element);
                pass.set_bind_group(group::UNIFORMS, &self.l1_uniform_bg, &[]);
                pass.set_bind_group(group::PREV, &self.prev_bg[parity], &[]);
                pass.set_bind_group(group::NEXT, &self.next_bg[parity], &[]);
                pass.dispatch_workgroups(Self::workgroups(self.live_count), 1, 1);
            }
            // What this pass wrote as "next" is the next pass's "prev", and
            // after the last one it is what L4 reads.
            self.parity = !self.parity;
        }

        let render_parity = usize::from(self.parity);

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("L4"),
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
            pass.set_pipeline(&self.render);
            pass.set_bind_group(group::UNIFORMS, &self.l4_uniform_bg, &[]);
            pass.set_bind_group(group::ATTRS, &self.l4_attr_bg[render_parity], &[]);
            pass.draw(
                0..karakuri_codegen::layout::VERTICES_PER_ELEMENT,
                0..self.live_count,
            );
        }
    }
}

fn filled(capacity: u32, value: &[u8; 4]) -> Vec<u8> {
    let mut out = vec![0u8; (u64::from(capacity) * ATTR_STRIDE) as usize];
    for i in 0..capacity as usize {
        let at = i * ATTR_STRIDE as usize;
        out[at..at + 4].copy_from_slice(value);
    }
    out
}

/// The scalar default of a param, for the uniform. Vector params are not yet
/// driven from here — every param the examples declare is a float.
fn default_scalar(p: &karakuri_ir::Param) -> Option<f32> {
    use karakuri_ir::{Expr, Lit};
    match &p.default {
        Expr::Lit {
            value: Lit::Float(v),
            ..
        } => Some(*v),
        _ => None,
    }
}
