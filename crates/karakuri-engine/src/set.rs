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

use karakuri_codegen::layout::{
    binding, counts, group, step_args, ElementLayout, UniformLayout, VERTICES_PER_ELEMENT, WORKGROUP_SIZE,
};
use karakuri_codegen::{generate_l1, generate_l4};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

use crate::camera::Orbit;
use crate::compaction::Compaction;
use crate::uniforms::UniformScratch;
use crate::video_source::VideoSource;

/// Past this the simulation falls behind rather than catching up — the
/// ir-spec's cap, restated here because it is now load-bearing rather than
/// advisory: each substep needs its own spawn-count entry, and that array is
/// sized once, at build time.
pub const MAX_STEPS: u8 = 4;

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
    /// `consumes ⊆ emit`, checked here because it is the first point where
    /// both procedures are in hand — a `.kir` declaring `consumes` alone is
    /// the normal shape of an L4 file, not an error, so no single-procedure
    /// pass can decide this. See the IR spec's validation pipeline, stage 6.
    #[error(
        "`{l4}` consumes {missing} which `{l1}` does not emit\n\
         hint: add {missing} to `{l1}`'s `emit`, or pair `{l4}` with an L1 that emits it \
         — there is no derivation step"
    )]
    Composition {
        l1: String,
        l4: String,
        missing: String,
    },
}

/// Bytes per element in the alive buffer: a dense `array<u32>`, one flag per
/// element, no vec4 padding — see the layout contract for why this is the
/// one piece of per-element state that is *not* 16-byte padded: the
/// compaction scan reads it as a plain array with no stride arithmetic.
const ALIVE_STRIDE: u64 = 4;

/// One direction's pair of storage buffers (element or alive).
struct Pair {
    a: wgpu::Buffer,
    b: wgpu::Buffer,
}

impl Pair {
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
    /// The monotone spawn ordinal the next new element gets. Advances by
    /// what the engine *asked* for, not by what the GPU managed to fit: a
    /// gap in the seed sequence is harmless, a repeated seed would break
    /// element identity.
    seed_base: u32,
    seed_salt: u32,
    /// Simulation steps elapsed. Time is `steps_taken * dt`, computed on
    /// demand rather than accumulated: a running `t += dt * steps` sum drifts
    /// by an ULP or two depending on how the steps were grouped, so twenty
    /// steps taken one at a time would land at a different `t` from ten taken
    /// in pairs. Two tick histories reaching the same elapsed time have to be
    /// the same point in the session, and a float sum is not that function.
    steps_taken: u64,
    dt: f32,
    /// The spawn accumulator, in whole elements. `spawn_rate * dt` is rarely
    /// an integer, so the fractional remainder carries into the next substep
    /// and the long-run rate comes out exact — ir-spec, "Spawn timing".
    spawn_carry: f32,
    /// This frame's per-substep spawn counts, as `prepare` computed them.
    /// `render` needs them to size the direct `spawn` dispatch; the shaders
    /// read the same numbers out of `step_args`.
    step_spawn_counts: [u32; MAX_STEPS as usize],
    viewport: [f32; 2],
    parity: bool,
    has_spawn: bool,

    element_layout: ElementLayout,
    element_buf: Pair,
    alive_buf: Pair,

    l1_uniform_layout: UniformLayout,
    l4_uniform_layout: UniformLayout,
    /// Host-side staging for the two uniform writes `prepare` makes every
    /// frame, sized once here against the layouts above. `prepare` runs on
    /// the render thread and the render thread does not allocate — see the
    /// module doc on `crate::uniforms`.
    l1_scratch: UniformScratch,
    l4_scratch: UniformScratch,
    l1_uniforms: wgpu::Buffer,
    l4_uniforms: wgpu::Buffer,
    /// Engine counts plus the indirect arguments derived from them. The only
    /// place the live range is known — see `karakuri_codegen::layout::counts`.
    counts: wgpu::Buffer,
    /// `MAX_STEPS` entries of `StepArgs`, `step_args::STRIDE` apart.
    step_args: wgpu::Buffer,

    /// `None` for a procedure whose live set cannot change. Its scan would
    /// compute the identity permutation at full capacity every frame.
    compaction: Option<Compaction>,

    element: wgpu::ComputePipeline,
    /// Present only when the procedure declares a `spawn` block.
    spawn: Option<wgpu::ComputePipeline>,
    render: wgpu::RenderPipeline,

    l1_uniform_bg: wgpu::BindGroup,
    l4_uniform_bg: wgpu::BindGroup,
    /// One per substep, binding `step_args` at that substep's offset.
    step_bg: Vec<wgpu::BindGroup>,
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
        // Before anything is generated: an L4 reads the element struct an L1
        // wrote, so a consumed attribute the L1 never emitted has no field to
        // read. Left unchecked it surfaces as a WGSL parse failure inside
        // `create_shader_module` — an internal error where the contract calls
        // for a diagnostic. Every missing attribute is reported at once, for
        // the same reason the IR checker reports every error at once: one
        // regeneration should be able to fix all of them.
        let missing: Vec<&str> = l4
            .consumes
            .iter()
            .filter(|a| !l1.emit.contains(a))
            .map(|a| a.name())
            .collect();
        if !missing.is_empty() {
            return Err(SetError::Composition {
                l1: l1.name.clone(),
                l4: l4.name.clone(),
                missing: missing
                    .iter()
                    .map(|a| format!("`{a}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
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
        let l4_shader = generate_l4(l4, &l1_shader.element_layout);

        let l1_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} (L1)", l1.name)),
            source: wgpu::ShaderSource::Wgsl(l1_shader.source.as_str().into()),
        });
        let l4_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} (L4)", l4.name)),
            source: wgpu::ShaderSource::Wgsl(l4_shader.source.as_str().into()),
        });

        // -- buffers ------------------------------------------------------
        let element_layout = l1_shader.element_layout.clone();
        let element_buffer_size = u64::from(capacity) * u64::from(element_layout.stride);
        let alive_buffer_size = u64::from(capacity) * ALIVE_STRIDE;
        let make_pair = |label: &str, size: u64| {
            let make = |suffix: &str| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("{label}_{suffix}")),
                    size,
                    // COPY_SRC only for the readbacks in `Set::live_count`
                    // and `Set::read_elements`, both of which are stalls and
                    // neither of which is on the frame path.
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                })
            };
            Pair { a: make("a"), b: make("b") }
        };
        let element_buf = make_pair("element", element_buffer_size);
        let alive_buf = make_pair("alive", alive_buffer_size);

        let counts_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("counts"),
            size: counts::SIZE,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let step_args_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spawn args"),
            size: u64::from(MAX_STEPS) * step_args::STRIDE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
        let storage_entry = |binding_num: u32, read_only: bool, vis: wgpu::ShaderStages| wgpu::BindGroupLayoutEntry {
            binding: binding_num,
            visibility: vis,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        // `prev`/`next` bind the element buffer and the alive buffer
        // together, at the fixed numbers `layout::binding` publishes. L4
        // binds both too: its draw range holds elements killed during the
        // step that just ran, scattered among the survivors rather than
        // gathered at either end, and the vertex stage skips them per
        // instance by reading the flag.
        let element_and_alive_bgl = |label, read_only| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[
                    storage_entry(binding::ELEMENT, read_only, wgpu::ShaderStages::COMPUTE),
                    storage_entry(binding::ALIVE, read_only, wgpu::ShaderStages::COMPUTE),
                ],
            })
        };

        // The L1 uniform group carries the engine's per-frame counts
        // alongside the uniform buffer, and — only for a compacted
        // procedure — the scan's destination indices. A static procedure
        // writes in place, so declaring `dest` in its layout would oblige
        // the engine to bind a buffer it has no scan to fill.
        let mut l1_uniform_entries = vec![
            wgpu::BindGroupLayoutEntry {
                binding: binding::UNIFORM,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            storage_entry(binding::COUNTS, true, wgpu::ShaderStages::COMPUTE),
        ];
        if l1_shader.compacted {
            l1_uniform_entries.push(storage_entry(binding::DEST, true, wgpu::ShaderStages::COMPUTE));
        }
        let l1_uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L1 uniforms"),
            entries: &l1_uniform_entries,
        });
        let step_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("spawn args"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: binding::UNIFORM,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let l4_uniform_bgl = uniform_bgl("L4 uniforms");
        let prev_bgl = element_and_alive_bgl("prev", true);
        let next_bgl = element_and_alive_bgl("next", false);
        let l4_attr_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("attrs"),
            entries: &[
                storage_entry(binding::ELEMENT, true, wgpu::ShaderStages::VERTEX_FRAGMENT),
                storage_entry(binding::ALIVE, true, wgpu::ShaderStages::VERTEX_FRAGMENT),
            ],
        });

        // -- compaction ----------------------------------------------------
        // Built before the bind groups because `element`'s uniform group
        // binds the scan's `dest` buffer, and skipped entirely for a static
        // procedure — the one whose scan would be the identity permutation.
        let compaction = l1_shader.compacted.then(|| {
            Compaction::new(
                device,
                capacity,
                [alive_buf.prev(false), alive_buf.prev(true)],
                &counts_buf,
                &step_args_buf,
                u32::from(MAX_STEPS),
            )
        });

        // -- bind groups ---------------------------------------------------
        let bind_uniform = |label, bgl: &wgpu::BindGroupLayout, buf: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: binding::UNIFORM,
                    resource: buf.as_entire_binding(),
                }],
            })
        };
        let mut l1_uniform_entries = vec![
            wgpu::BindGroupEntry {
                binding: binding::UNIFORM,
                resource: l1_uniforms.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: binding::COUNTS,
                resource: counts_buf.as_entire_binding(),
            },
        ];
        if let Some(c) = &compaction {
            l1_uniform_entries.push(wgpu::BindGroupEntry {
                binding: binding::DEST,
                resource: c.dest_buffer().as_entire_binding(),
            });
        }
        let l1_uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("L1 uniforms"),
            layout: &l1_uniform_bgl,
            entries: &l1_uniform_entries,
        });
        let l4_uniform_bg = bind_uniform("L4 uniforms", &l4_uniform_bgl, &l4_uniforms);

        // One bind group per substep rather than one dynamic offset: the
        // offsets are known at build time, they never change, and a
        // dynamic-offset entry would force every `set_bind_group` call on
        // this group — including `element`'s, which does not use it — to
        // carry an offset array.
        let step_bg: Vec<wgpu::BindGroup> = (0..u64::from(MAX_STEPS))
            .map(|step| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("spawn args"),
                    layout: &step_bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: binding::UNIFORM,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &step_args_buf,
                            offset: step * step_args::STRIDE,
                            size: wgpu::BufferSize::new(step_args::SIZE),
                        }),
                    }],
                })
            })
            .collect();

        let bind_pair = |label: &str, bgl: &wgpu::BindGroupLayout, parity: bool, next: bool| {
            let (elem, alive) = if next {
                (element_buf.next(parity), alive_buf.next(parity))
            } else {
                (element_buf.prev(parity), alive_buf.prev(parity))
            };
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: binding::ELEMENT, resource: elem.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: binding::ALIVE, resource: alive.as_entire_binding() },
                ],
            })
        };

        let prev_bg = [
            bind_pair("prev0", &prev_bgl, false, false),
            bind_pair("prev1", &prev_bgl, true, false),
        ];
        let next_bg = [
            bind_pair("next0", &next_bgl, false, true),
            bind_pair("next1", &next_bgl, true, true),
        ];
        // L4 reads what L1 last wrote. The frame's compute pass writes "next",
        // then parity flips, so L4 binds the same physical element buffer
        // that is "prev" under the flipped parity.
        let bind_l4_attrs = |label: &str, parity: bool| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &l4_attr_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: binding::ELEMENT,
                        resource: element_buf.prev(parity).as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: binding::ALIVE,
                        resource: alive_buf.prev(parity).as_entire_binding(),
                    },
                ],
            })
        };
        let l4_attr_bg = [bind_l4_attrs("l4attrs0", false), bind_l4_attrs("l4attrs1", true)];

        // -- pipelines ------------------------------------------------------
        // One layout for both entry points: `element` reaches group STEP too,
        // for this substep's `t`. An earlier revision gave `element` a
        // three-group layout on the grounds that only `spawn` needed the
        // fourth, which stopped being true when `t` moved out of the uniform.
        let compute_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("L1"),
            bind_group_layouts: &[&l1_uniform_bgl, &prev_bgl, &next_bgl, &step_bgl],
            push_constant_ranges: &[],
        });
        let compute = |entry: &str, layout: &wgpu::PipelineLayout| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(layout),
                module: &l1_module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let element = compute("element", &compute_pl);
        let spawn = l1_shader.has_spawn.then(|| compute("spawn", &compute_pl));

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
            seed_base: 0,
            seed_salt,
            steps_taken: 0,
            dt: 1.0 / 60.0,
            spawn_carry: 0.0,
            step_spawn_counts: [0; MAX_STEPS as usize],
            viewport: [1.0, 1.0],
            parity: false,
            has_spawn: l1_shader.has_spawn,
            element_layout,
            element_buf,
            alive_buf,
            l1_scratch: UniformScratch::new(&l1_shader.uniform_layout),
            l4_scratch: UniformScratch::new(&l4_shader.uniform_layout),
            l1_uniform_layout: l1_shader.uniform_layout.clone(),
            l4_uniform_layout: l4_shader.uniform_layout.clone(),
            l1_uniforms,
            l4_uniforms,
            counts: counts_buf,
            step_args: step_args_buf,
            compaction,
            element,
            spawn,
            render,
            l1_uniform_bg,
            l4_uniform_bg,
            step_bg,
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

    /// Zero the element and alive buffers and, for a procedure with no
    /// `spawn` block, seed each slot with its own index — `seed` equals the
    /// initial slot index there, which is what makes lattice generators
    /// work as written. Also puts the counts buffer into the state the first
    /// step's invariant assumes: `prev` holds `range` entries at
    /// `[0, range)`, all of them alive.
    fn initialize(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        let (elements, alive) = initial_state(self.capacity, self.has_spawn, &self.element_layout);

        queue.write_buffer(&self.element_buf.a, 0, &elements);
        queue.write_buffer(&self.element_buf.b, 0, &elements);
        queue.write_buffer(&self.alive_buf.a, 0, &alive);
        queue.write_buffer(&self.alive_buf.b, 0, &alive);
        queue.write_buffer(&self.counts, 0, &initial_counts(self.capacity, self.has_spawn));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport = [width.max(1) as f32, height.max(1) as f32];
    }

    pub fn time(&self) -> f32 {
        self.t_at(self.steps_taken)
    }

    /// Simulation time after `n` steps. The one place `t` is derived, so
    /// there is exactly one function from a step count to an instant.
    fn t_at(&self, n: u64) -> f32 {
        n as f32 * self.dt
    }

    /// How many elements the current buffer holds — the draw's instance
    /// count, and the range the next step will scan. Not quite the alive
    /// count: an element killed during the step that just ran still occupies
    /// its slot until the next step's scan reclaims it.
    ///
    /// **This is a stall.** It copies four bytes off the GPU and blocks
    /// until the queue drains to read them, which is exactly what indirect
    /// dispatch exists to avoid. It is here for tests and for a status line
    /// printed once at the end of a run; **never call it on the frame
    /// path.** The alternative — tracking an estimate host-side — would be
    /// worse: a number that is usually right is harder to distrust than one
    /// that is honestly expensive.
    pub fn live_count(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> u32 {
        let bytes = read_buffer(device, queue, &self.counts, counts::SIZE);
        let at = counts::RANGE as usize;
        u32::from_le_bytes(bytes[at..at + 4].try_into().expect("counts buffer is 48 bytes"))
    }

    /// The raw bytes of the element buffer L4 is currently reading, decoded
    /// against [`Set::element_layout`]. **A stall, on the same terms as
    /// [`Set::live_count`]** — this exists so a test can check that
    /// survivors kept their order, which is a claim about `seed` values in
    /// slots and cannot be made from a rendered image.
    pub fn read_elements(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        let size = u64::from(self.capacity) * u64::from(self.element_layout.stride);
        read_buffer(device, queue, self.element_buf.prev(self.parity), size)
    }

    pub fn element_layout(&self) -> &ElementLayout {
        &self.element_layout
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Uploads uniforms and advances simulation time. A parameter change is a
    /// uniform write, which is why it does not need a fork.
    ///
    /// Also quantizes this frame's spawning. `steps` is clamped to
    /// [`MAX_STEPS`] here and in [`VideoSource::render`] alike: past that the
    /// simulation is allowed to fall behind rather than catch up, and the
    /// two have to agree or `t` would advance further than the element
    /// passes did.
    ///
    /// **Nothing in here allocates.** Both uniform writes go through storage
    /// sized at build time (`crate::uniforms::UniformScratch`) and the step
    /// arguments through a stack array, because this is the render thread and
    /// the first invariant in `README.md` is the one about allocating on it.
    pub fn prepare(&mut self, queue: &wgpu::Queue, steps: u8) {
        let steps = steps.min(MAX_STEPS);
        self.steps_taken += u64::from(steps);

        // No `t` here: it differs between this frame's substeps and lives in
        // `StepArgs`. Everything left is input, sampled once per frame.
        {
            let mut p = self.l1_scratch.pack(&self.l1_uniform_layout);
            p.f32("dt", self.dt)
                .u32("capacity", self.capacity)
                .u32("seed_salt", self.seed_salt);
            for name in &self.l1_param_names {
                p.f32(name, self.params[name]);
            }
            queue.write_buffer(&self.l1_uniforms, 0, p.finish());
        }

        self.write_step_args(queue, steps);

        // Read before the packer borrows the scratch: `time` and `view_proj`
        // take `&self`, and the packer holds a `&mut` to one of its fields.
        let t = self.time();
        let aspect = self.viewport[0] / self.viewport[1];
        let camera = self.camera.view_proj(t, aspect);
        {
            let mut p = self.l4_scratch.pack(&self.l4_uniform_layout);
            p.f32("t", t)
                .u32("seed_salt", self.seed_salt)
                .vec2("viewport", self.viewport)
                .mat4("camera", camera);
            for name in &self.l4_param_names {
                p.f32(name, self.params[name]);
            }
            queue.write_buffer(&self.l4_uniforms, 0, p.finish());
        }
    }

    /// The spawn accumulator, one entry per substep.
    ///
    /// The ir-spec writes the accumulator as `carry += spawn_rate * dt *
    /// float(steps)` — one batch per frame. Advancing it once per *substep*
    /// gives the same total over the frame (the carry is a running real
    /// number, so the count emitted by any point is the floor of what has
    /// accumulated to it), and it is the only version under which
    /// substepping holds: a frame of two steps has to spawn the same two
    /// batches, at the same two points in the integration, that two frames
    /// of one step would. One batch per frame would put both frames' worth
    /// of elements in before the second element pass instead.
    ///
    /// `spawn_rate` is a `param`, so it is sampled once per frame and held
    /// constant across the substeps, like every other parameter.
    fn write_step_args(&mut self, queue: &wgpu::Queue, steps: u8) {
        self.step_spawn_counts = [0; MAX_STEPS as usize];
        // A procedure with no `spawn` block never creates anything, but the
        // `advance` pass still runs for it if it can `kill()` — with a zero
        // count and the capacity it needs to clamp against.
        let rate = if self.has_spawn {
            self.params.get("spawn_rate").copied().unwrap_or(0.0)
        } else {
            0.0
        };

        // Substep `k` of this frame is step number `first + k` of the
        // session, and its `t` is that number's instant. `steps_taken` has
        // already been advanced past this frame, so count back from it —
        // deriving both ends from the same counter is what makes a frame of
        // two steps land on the same two instants two frames of one step do.
        let first = self.steps_taken - u64::from(steps) + 1;

        let mut bytes = [0u8; MAX_STEPS as usize * step_args::STRIDE as usize];
        for step in 0..steps as usize {
            self.spawn_carry += rate * self.dt;
            let whole = self.spawn_carry.floor();
            self.spawn_carry -= whole;
            let count = whole.max(0.0) as u32;
            self.step_spawn_counts[step] = count;

            let at = step * step_args::STRIDE as usize;
            bytes[at..at + 4].copy_from_slice(&count.to_le_bytes());
            bytes[at + 4..at + 8].copy_from_slice(&self.seed_base.to_le_bytes());
            bytes[at + 8..at + 12].copy_from_slice(&self.capacity.to_le_bytes());
            bytes[at + 12..at + 16]
                .copy_from_slice(&self.t_at(first + step as u64).to_le_bytes());

            // By the request, not by what fits: the GPU clamps against
            // capacity and silently drops the overflow, and reusing those
            // seeds on the next frame would give two live elements the same
            // identity. Gaps in the sequence cost nothing.
            self.seed_base = self.seed_base.wrapping_add(count);
        }
        queue.write_buffer(&self.step_args, 0, &bytes);
    }

    fn workgroups(count: u32) -> u32 {
        count.div_ceil(WORKGROUP_SIZE)
    }
}

/// Copies `len` bytes off the GPU and blocks until they arrive. Every caller
/// is a stall by construction — see [`Set::live_count`].
fn read_buffer(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &wgpu::Buffer, len: u64) -> Vec<u8> {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("set readback"),
        size: len,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, len);
    queue.submit([encoder.finish()]);

    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
    let out = data.to_vec();
    drop(data);
    staging.unmap();
    out
}

impl VideoSource for Set {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        steps: u8,
    ) {
        // One step per `steps`, not one per frame. `steps` is what the tick
        // record carries, and the whole point of substepping is that the
        // simulation state at a given `t` does not depend on frame rate — so
        // advancing `t` by `steps * dt` while stepping once would desynchronise
        // the two. At `steps == 0` nothing runs and parity does not flip, so a
        // paused frame renders exactly what the previous one did.
        //
        // `t` is *not* constant across a frame's substeps — parameters and
        // signal bindings are, because they are input, and `t` is the
        // simulation's own clock. Each substep binds its own `StepArgs`
        // entry, carrying that substep's `t` and its spawn count. See
        // `write_step_args` and `layout::step_args`.
        //
        // Four passes per step, in this order:
        //
        //   1. scan     — over `prev_alive[0, range)`, producing `dest[i]`
        //                 and the survivor total `S`.
        //   2. element  — indirect over `range`. Survivors write to
        //                 `next[dest[i]]`, which compacts them; the ones that
        //                 call `kill()` write an alive flag of 0 there and
        //                 are reclaimed by the *next* step's scan.
        //   3. spawn    — direct, over the count this substep's accumulator
        //                 produced, writing at `S + j`.
        //   4. advance  — `range = S + min(spawn_count, capacity - S)`, and
        //                 the dispatch and draw arguments from it.
        //
        // Steps 1, 3 and 4 do not run for a static procedure: its live set
        // cannot change, `range` stays at `capacity` from initialization,
        // and `element` writes in place.
        let steps = steps.min(MAX_STEPS);
        for step in 0..usize::from(steps) {
            let parity = self.parity;
            if let Some(compaction) = &self.compaction {
                compaction.record(encoder, parity);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("element"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.element);
                pass.set_bind_group(group::UNIFORMS, &self.l1_uniform_bg, &[]);
                pass.set_bind_group(group::PREV, &self.prev_bg[usize::from(parity)], &[]);
                pass.set_bind_group(group::NEXT, &self.next_bg[usize::from(parity)], &[]);
                // `element` reads this substep's `t` out of the same buffer
                // `spawn` reads its count from — see `layout::step_args`.
                pass.set_bind_group(group::STEP, &self.step_bg[step], &[]);
                pass.dispatch_workgroups_indirect(&self.counts, counts::ELEM_XYZ);
            }
            if let Some(spawn) = &self.spawn {
                let count = self.step_spawn_counts[step];
                if count > 0 {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("spawn"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(spawn);
                    pass.set_bind_group(group::UNIFORMS, &self.l1_uniform_bg, &[]);
                    pass.set_bind_group(group::PREV, &self.prev_bg[usize::from(parity)], &[]);
                    pass.set_bind_group(group::NEXT, &self.next_bg[usize::from(parity)], &[]);
                    pass.set_bind_group(group::STEP, &self.step_bg[step], &[]);
                    // Direct, because the count is host state: the
                    // accumulator lives on the Set so that the sequence is a
                    // pure function of the record stream. The GPU is what
                    // clamps it against the free range.
                    pass.dispatch_workgroups(Self::workgroups(count), 1, 1);
                }
            }
            if let Some(compaction) = &self.compaction {
                compaction.record_advance(encoder, step);
            }
            // What this step wrote as "next" is the next step's "prev", and
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
            // The instance count is GPU state now, so this is indirect even
            // for a static procedure whose count the host does know — one
            // render path rather than two, at the cost of one buffer read
            // the command processor was going to do anyway.
            pass.draw_indirect(&self.counts, counts::DRAW);
        }
    }
}

/// The host-side byte image `Set::initialize` uploads: the interleaved
/// `Element` buffer and the dense `alive` buffer, computed against
/// `layout`'s published offsets rather than against any assumption of its
/// own about where a field lands. Pulled out of `initialize` as a pure
/// function — no device, no queue — so the byte-offset arithmetic can be
/// tested directly against `ElementLayout` without a GPU in the loop; see
/// `tests::initial_state_matches_element_layout_offsets` below.
fn initial_state(capacity: u32, has_spawn: bool, layout: &ElementLayout) -> (Vec<u8>, Vec<u8>) {
    let stride = layout.stride as usize;
    let mut elements = vec![0u8; capacity as usize * stride];
    let mut alive = vec![0u8; capacity as usize * ALIVE_STRIDE as usize];

    if !has_spawn {
        // Everything is alive, and nothing needs a birth-fraction
        // correction because nothing was born mid-frame.
        let seed_offset = layout.offset_of("seed") as usize;
        let birth_frac_offset = layout.offset_of("birth_frac") as usize;
        for i in 0..capacity as usize {
            let base = i * stride;
            elements[base + seed_offset..base + seed_offset + 4].copy_from_slice(&(i as u32).to_le_bytes());
            elements[base + birth_frac_offset..base + birth_frac_offset + 4].copy_from_slice(&1.0f32.to_le_bytes());
            alive[i * ALIVE_STRIDE as usize..i * ALIVE_STRIDE as usize + 4].copy_from_slice(&1u32.to_le_bytes());
        }
    }

    (elements, alive)
}

/// The counts buffer's frame-zero contents, matching what `initial_state`
/// put in the element and alive buffers: a spawn-less procedure comes up
/// with all `capacity` slots occupied and alive, one with a `spawn` block
/// comes up empty and fills.
///
/// `survivors` starts equal to `range` for the same reason `range` does —
/// the scan overwrites it before anything reads it, but a compacted
/// procedure that somehow rendered before its first step should draw a
/// consistent range rather than an arbitrary one.
fn initial_counts(capacity: u32, has_spawn: bool) -> Vec<u8> {
    let range = if has_spawn { 0 } else { capacity };
    let mut bytes = vec![0u8; counts::SIZE as usize];
    let mut put = |at: u64, v: u32| {
        let at = at as usize;
        bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
    };
    put(counts::ELEM_XYZ, range.div_ceil(WORKGROUP_SIZE));
    put(counts::ELEM_XYZ + 4, 1);
    put(counts::ELEM_XYZ + 8, 1);
    put(counts::RANGE, range);
    put(counts::DRAW, VERTICES_PER_ELEMENT);
    put(counts::DRAW + 4, range);
    put(counts::SURVIVORS, range);
    bytes
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

#[cfg(test)]
mod tests {
    //! Byte-level checks for `initial_state`, the pure function behind
    //! `Set::initialize`'s interleaved write. The module doc on
    //! `karakuri_codegen::layout` warns that getting the host-side write
    //! wrong is silent — "a plausible-looking image with the wrong values
    //! in it" — so this checks the exact bytes at the exact offsets
    //! `ElementLayout` publishes, independently of whatever `initialize`
    //! itself does, and needs no GPU to do it.
    use super::*;
    use crate::gpu::Gpu;

    fn seed_at(elements: &[u8], stride: usize, layout: &ElementLayout, i: usize) -> u32 {
        let off = i * stride + layout.offset_of("seed") as usize;
        u32::from_le_bytes(elements[off..off + 4].try_into().unwrap())
    }

    fn birth_frac_at(elements: &[u8], stride: usize, layout: &ElementLayout, i: usize) -> f32 {
        let off = i * stride + layout.offset_of("birth_frac") as usize;
        f32::from_le_bytes(elements[off..off + 4].try_into().unwrap())
    }

    fn alive_at(alive: &[u8], i: usize) -> u32 {
        let off = i * ALIVE_STRIDE as usize;
        u32::from_le_bytes(alive[off..off + 4].try_into().unwrap())
    }

    /// For a spawn-less procedure every slot must come up `seed == i`,
    /// `birth_frac == 1.0`, `alive == 1` — the exact byte offsets
    /// `ElementLayout` publishes, not merely "a shader can read it without
    /// erroring."
    #[test]
    fn no_spawn_block_seeds_every_slot_with_its_index_and_marks_it_alive() {
        let layout = karakuri_codegen::layout::generate_element_layout(&[karakuri_ir::Attr::Position, karakuri_ir::Attr::Age]);
        let stride = layout.stride as usize;
        let capacity = 8u32;
        let (elements, alive) = initial_state(capacity, false, &layout);

        assert_eq!(elements.len(), capacity as usize * stride);
        assert_eq!(alive.len(), capacity as usize * ALIVE_STRIDE as usize);

        for i in 0..capacity as usize {
            assert_eq!(seed_at(&elements, stride, &layout, i), i as u32, "seed at slot {i}");
            assert_eq!(birth_frac_at(&elements, stride, &layout, i), 1.0, "birth_frac at slot {i}");
            assert_eq!(alive_at(&alive, i), 1, "alive flag at slot {i}");
        }
    }

    /// The other side of the same coin: a procedure *with* a `spawn` block
    /// starts with an empty range, so `initial_state` must leave every slot
    /// zeroed — `seed`, `birth_frac`, and `alive` alike — rather than
    /// reusing the no-spawn seeding path. A stray `alive == 1` here would
    /// make dead slots read as live the moment the range grew past them.
    #[test]
    fn spawn_block_leaves_every_slot_zeroed() {
        let layout = karakuri_codegen::layout::generate_element_layout(&[karakuri_ir::Attr::Position, karakuri_ir::Attr::Age]);
        let stride = layout.stride as usize;
        let capacity = 8u32;
        let (elements, alive) = initial_state(capacity, true, &layout);

        assert_eq!(elements, vec![0u8; capacity as usize * stride], "spawn-block elements must start zeroed");
        assert_eq!(alive, vec![0u8; capacity as usize * ALIVE_STRIDE as usize], "spawn-block alive flags must start zeroed");
    }

    /// End-to-end smoke test that a procedure *with* a `spawn` block builds
    /// a `Set` successfully and starts with a zero live count —
    /// exercising the real `generate_l1`/`generate_l4` path (not the
    /// hand-built `ElementLayout` the two tests above use) for the one
    /// shape `crates/karakuri-engine/tests/generated.rs` never covers.
    #[test]
    fn a_procedure_with_a_spawn_block_builds_and_starts_empty() {
        let gpu = Gpu::headless().expect("no GPU available");
        let l1_src = r#"
proc probe_spawn_l1 {
  kind     L1
  topology points
  capacity [4, 64] = 8

  param spawn_rate : float [0.0, 40000.0] = 1000.0

  emit position, age

  spawn {
    position = vec3(0.0, 0.0, 0.0);
    age      = 0.0;
  }

  element {
    position = position;
    age      = age;
  }
}
"#;
        let l4_src = r#"
proc probe_l4 {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 1.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
        let compile = |src: &str| -> Checked {
            let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
            let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
            karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
            checked
        };
        let l1 = compile(l1_src);
        let l4 = compile(l4_src);
        let set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 8, 1).expect("compatible pair");

        assert_eq!(set.live_count(&gpu.device, &gpu.queue), 0, "a spawn-block procedure starts empty");
    }
}
