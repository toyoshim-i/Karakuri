//! The L1 node: `() -> Geometry`.

use karakuri_codegen::generate_l1;
use karakuri_codegen::layout::{
    binding, counts, group, step_args, UniformLayout, VERTICES_PER_ELEMENT, WORKGROUP_SIZE,
};
use karakuri_ir::layout::{ElementLayout, ALIVE_BYTES};
use karakuri_ir::typed::Checked;

use super::{Geometry, Tick};
use crate::compaction::Compaction;
use crate::set::{ElementStorage, MAX_STEPS};
use crate::storage::SimulationStorage;
use crate::uniforms::UniformScratch;

/// Parameter name used to quantize element spawning.
///
/// Spawning reads this value through [`Tick::param`] so that dynamic bindings
/// (such as noise sources) are applied consistently across both the uniform buffer
/// and the host-side spawn accumulator. Parameter values are scoped to this node.
const SPAWN_RATE: &str = "spawn_rate";

/// One direction's pair of storage buffers (element or alive).
struct Pair {
    a: wgpu::Buffer,
    b: wgpu::Buffer,
}

impl Pair {
    /// Returns the physical buffer acting as "previous" for the given parity.
    fn prev(&self, parity: bool) -> &wgpu::Buffer {
        if parity {
            &self.a
        } else {
            &self.b
        }
    }

    /// Returns the physical buffer acting as "next" for the given parity.
    fn next(&self, parity: bool) -> &wgpu::Buffer {
        if parity {
            &self.b
        } else {
            &self.a
        }
    }

    /// Returns the total memory consumed by both buffers in bytes.
    fn bytes(&self) -> u64 {
        self.a.size() + self.b.size()
    }
}

/// L1 simulation node producing geometry: `() -> Geometry`.
///
/// Owns all per-element state for a Set, including ping-pong element and alive
/// buffers, spawn accumulation, and compaction state. Timings are supplied
/// per substep via [`Tick`].
pub(crate) struct Simulation {
    capacity: u32,
    seed_salt: u32,
    has_spawn: bool,
    /// Indicates whether buffer allocation exceeded device limits.
    ///
    /// If true, GPU buffers are error objects and [`Simulation::initialize`] skips
    /// allocating host-side initial state arrays to prevent host OOM before the
    /// validation error scope is reported.
    refused_by_device: bool,
    /// Monotonically increasing seed ordinal assigned to newly spawned elements.
    ///
    /// Advances by the requested spawn count regardless of capacity clamping to
    /// ensure seed uniqueness across frames.
    seed_base: u32,
    /// Fractional spawn accumulator carried forward across substeps.
    spawn_carry: f32,
    staged_spawn_carry: Option<f32>,
    /// Per-substep spawn counts computed during [`Simulation::prepare`].
    step_spawn_counts: [u32; MAX_STEPS as usize],
    parity: bool,
    staged_parity: Option<bool>,

    element_layout: ElementLayout,
    element_buf: Pair,
    alive_buf: Pair,

    uniform_layout: UniformLayout,
    /// Preallocated scratch buffer for packing per-frame uniforms without allocation.
    scratch: UniformScratch,
    uniforms: wgpu::Buffer,
    /// Counts buffer containing live element range and indirect dispatch arguments.
    counts: wgpu::Buffer,
    /// `MAX_STEPS` entries of `StepArgs`, `step_args::STRIDE` apart.
    step_args: wgpu::Buffer,

    /// Prefix-sum compaction pipeline, or `None` if the live set is static.
    compaction: Option<Compaction>,

    element: wgpu::ComputePipeline,
    /// Present only when the procedure declares a `spawn` block.
    spawn: Option<wgpu::ComputePipeline>,

    uniform_bg: wgpu::BindGroup,
    /// One per substep, binding `step_args` at that substep's offset.
    step_bg: Vec<wgpu::BindGroup>,
    /// Indexed by parity: [false, true].
    prev_bg: [wgpu::BindGroup; 2],
    next_bg: [wgpu::BindGroup; 2],

    /// Declared parameter field names matching uniform buffer layout entries.
    param_names: Vec<String>,
    /// Addressable parameter keys, expanding vector parameters into component keys.
    param_keys: Vec<String>,
}

impl Simulation {
    /// Compiles and allocates an L1 simulation node for the given capacity.
    ///
    /// Assumes capacity validation has already been performed by the caller.
    pub(crate) fn build(
        device: &wgpu::Device,
        l1: &Checked,
        capacity: u32,
        seed_salt: u32,
        // What the Set decided to synthesise — see `Set::build_many`. The
        // slots those rules read are written by this node and by nothing else.
        derived: &[karakuri_ir::Attr],
        fields: karakuri_codegen::Bound<'_>,
    ) -> Simulation {
        let shader = generate_l1(l1, derived, fields);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} (L1)", l1.name)),
            source: wgpu::ShaderSource::Wgsl(shader.source.as_str().into()),
        });

        // -- buffers ------------------------------------------------------
        let element_layout = shader.element_layout.clone();
        let storage = SimulationStorage::of(capacity, element_layout.stride, shader.compacted);
        let make_pair = |label: &str, size: u64| {
            let make = |suffix: &str| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("{label}_{suffix}")),
                    size,
                    // COPY_SRC enables diagnostic and test readbacks.
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                })
            };
            Pair {
                a: make("a"),
                b: make("b"),
            }
        };
        // Check buffer size limits to avoid allocating host initial state on OOM.
        let ceiling = device.limits().max_buffer_size;
        let refused_by_device = storage.element_buffer() > ceiling
            || storage.alive_buffer() > ceiling
            || storage.dest_buffer().is_some_and(|dest| dest > ceiling);

        let element_buf = make_pair("element", storage.element_buffer());
        let alive_buf = make_pair("alive", storage.alive_buffer());

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

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("L1 uniforms"),
            size: u64::from(shader.uniform_layout.total_size),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // -- bind group layouts -------------------------------------------
        let storage_entry = |binding_num: u32, read_only: bool, vis: wgpu::ShaderStages| {
            wgpu::BindGroupLayoutEntry {
                binding: binding_num,
                visibility: vis,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
        };
        // Binds element and alive buffers together at layout::binding indices.
        let element_and_alive_bgl = |label, read_only| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[
                    storage_entry(binding::ELEMENT, read_only, wgpu::ShaderStages::COMPUTE),
                    storage_entry(binding::ALIVE, read_only, wgpu::ShaderStages::COMPUTE),
                ],
            })
        };

        // Uniform layout includes counts and, for compacted procedures, scan destination indices.
        let mut uniform_entries = vec![
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
        if shader.compacted {
            uniform_entries.push(storage_entry(
                binding::DEST,
                true,
                wgpu::ShaderStages::COMPUTE,
            ));
        }
        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L1 uniforms"),
            entries: &uniform_entries,
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
        let prev_bgl = element_and_alive_bgl("prev", true);
        let next_bgl = element_and_alive_bgl("next", false);

        // -- compaction ----------------------------------------------------
        // Compaction is allocated only if a scan destination buffer is required.
        let compaction = storage.dest_buffer().map(|_dest_bytes| {
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
        let mut uniform_bg_entries = vec![
            wgpu::BindGroupEntry {
                binding: binding::UNIFORM,
                resource: uniforms.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: binding::COUNTS,
                resource: counts_buf.as_entire_binding(),
            },
        ];
        if let Some(c) = &compaction {
            uniform_bg_entries.push(wgpu::BindGroupEntry {
                binding: binding::DEST,
                resource: c.dest_buffer().as_entire_binding(),
            });
        }
        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("L1 uniforms"),
            layout: &uniform_bgl,
            entries: &uniform_bg_entries,
        });

        // Pre-binds StepArgs for each substep offset to avoid dynamic offset overhead.
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
                    wgpu::BindGroupEntry {
                        binding: binding::ELEMENT,
                        resource: elem.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: binding::ALIVE,
                        resource: alive.as_entire_binding(),
                    },
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

        // -- pipelines ------------------------------------------------------
        // Shared pipeline layout across compute entry points.
        let compute_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("L1"),
            bind_group_layouts: &[
                Some(&uniform_bgl),
                Some(&prev_bgl),
                Some(&next_bgl),
                Some(&step_bgl),
            ],
            immediate_size: 0,
        });
        let compute = |entry: &str, layout: &wgpu::PipelineLayout| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(layout),
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let element = compute("element", &compute_pl);
        let spawn = shader.has_spawn.then(|| compute("spawn", &compute_pl));

        Simulation {
            capacity,
            seed_salt,
            has_spawn: shader.has_spawn,
            refused_by_device,
            seed_base: 0,
            spawn_carry: 0.0,
            staged_spawn_carry: None,
            step_spawn_counts: [0; MAX_STEPS as usize],
            parity: false,
            staged_parity: None,
            element_layout,
            element_buf,
            alive_buf,
            uniform_layout: shader.uniform_layout.clone(),
            scratch: UniformScratch::new(&shader.uniform_layout),
            uniforms,
            counts: counts_buf,
            step_args: step_args_buf,
            compaction,
            element,
            spawn,
            uniform_bg,
            step_bg,
            prev_bg,
            next_bg,
            param_names: l1.params.iter().map(|p| p.name.clone()).collect(),
            param_keys: crate::set::declared_keys(l1),
        }
    }

    /// Returns the downstream geometry interface referencing this node's ping-pong buffers.
    pub(crate) fn geometry(&self) -> Geometry<'_> {
        Geometry {
            layout: &self.element_layout,
            elements: [self.element_buf.prev(false), self.element_buf.prev(true)],
            alive: [self.alive_buf.prev(false), self.alive_buf.prev(true)],
            counts: &self.counts,
        }
    }

    /// Returns the element storage metrics allocated by this node.
    pub(crate) fn element_storage(&self) -> ElementStorage {
        ElementStorage {
            bytes: self.element_buf.bytes()
                + self.alive_buf.bytes()
                + self
                    .compaction
                    .as_ref()
                    .map_or(0, |c| c.dest_buffer().size()),
            capacity: self.capacity,
        }
    }

    /// Initializes GPU buffers to their starting state.
    ///
    /// For procedures without a `spawn` block, slots are pre-seeded with their
    /// initial indices and marked alive.
    pub(crate) fn initialize(&self, queue: &wgpu::Queue) {
        // Skip initialization if buffer allocation was refused by the device.
        if self.refused_by_device {
            return;
        }
        let (elements, alive) = initial_state(self.capacity, self.has_spawn, &self.element_layout);

        queue.write_buffer(&self.element_buf.a, 0, &elements);
        queue.write_buffer(&self.element_buf.b, 0, &elements);
        queue.write_buffer(&self.alive_buf.a, 0, &alive);
        queue.write_buffer(&self.alive_buf.b, 0, &alive);
        queue.write_buffer(
            &self.counts,
            0,
            &initial_counts(self.capacity, self.has_spawn),
        );
    }

    /// Resets simulation state and reinitializes buffers to frame zero.
    pub(crate) fn rewind(&mut self, queue: &wgpu::Queue) {
        self.discard();
        self.seed_base = 0;
        self.spawn_carry = 0.0;
        self.step_spawn_counts = [0; MAX_STEPS as usize];
        self.parity = false;
        self.initialize(queue);
    }

    /// Writes per-frame uniforms and step arguments into GPU buffers without allocation.
    pub(crate) fn prepare(&mut self, queue: &wgpu::Queue, tick: &Tick<'_>) {
        // Per-substep timing (`t`) lives in StepArgs; uniforms hold per-frame inputs.
        {
            let mut p = self.scratch.pack(&self.uniform_layout);
            p.f32("dt", tick.dt)
                .u32("capacity", self.capacity)
                .u32("seed_salt", self.seed_salt);
            super::write_params(
                &mut p,
                &self.uniform_layout,
                &self.param_names,
                tick.param,
                tick.param_value,
            );
            // Pack field parameters into the uniform buffer.
            super::write_field_params(
                &mut p,
                &self.uniform_layout,
                tick.field_params,
                tick.field_value,
            );
            // Pack source slots into the uniform buffer.
            super::write_source_slots(&mut p, &self.uniform_layout, tick.source_value);
            queue.write_buffer(&self.uniforms, 0, p.finish());
        }
        self.write_step_args(queue, tick);
    }

    /// Computes spawn counts and writes per-substep arguments into GPU buffers.
    ///
    /// Accumulates spawn counts per substep so that multi-substep frames match
    /// single-substep integration behavior.
    fn write_step_args(&mut self, queue: &wgpu::Queue, tick: &Tick<'_>) {
        self.step_spawn_counts = [0; MAX_STEPS as usize];
        // Non-spawning procedures still advance compaction with a zero spawn count.
        let rate = if self.has_spawn {
            (tick.param)(SPAWN_RATE).unwrap_or(0.0)
        } else {
            0.0
        };

        // Clamp steps to MAX_STEPS to guard fixed-size buffer indexing.
        let steps = tick.steps.min(MAX_STEPS);
        let mut bytes = [0u8; MAX_STEPS as usize * step_args::STRIDE as usize];
        let mut carry = self.spawn_carry;
        for step in 0..usize::from(steps) {
            carry += rate * tick.dt;
            let whole = carry.floor();
            carry -= whole;
            let count = whole.max(0.0) as u32;
            self.step_spawn_counts[step] = count;

            // Time instant and musical beat sampled for this substep.
            let (t, beats) = tick.instants[step];

            let at = step * step_args::STRIDE as usize;
            bytes[at..at + 4].copy_from_slice(&count.to_le_bytes());
            bytes[at + 4..at + 8].copy_from_slice(&self.seed_base.to_le_bytes());
            bytes[at + 8..at + 12].copy_from_slice(&self.capacity.to_le_bytes());
            bytes[at + 12..at + 16].copy_from_slice(&t.to_le_bytes());
            bytes[at + 16..at + 20].copy_from_slice(&beats.to_le_bytes());

            // Advance seed base by the requested count to ensure identity uniqueness.
            self.seed_base = self.seed_base.wrapping_add(count);
        }
        self.staged_spawn_carry = Some(carry);
        queue.write_buffer(&self.step_args, 0, &bytes);
    }

    /// Records compute passes for simulation substeps into the command encoder.
    ///
    /// Must be preceded by [`Simulation::prepare`] in the same frame.
    pub(crate) fn record(&mut self, encoder: &mut wgpu::CommandEncoder, steps: u8) {
        // Dispatches up to four passes per substep:
        //   1. Scan: prefix sum over `prev_alive` yielding compaction destinations.
        //   2. Element: updates elements; surviving elements compact into `next`.
        //   3. Spawn: appends newly spawned elements.
        //   4. Advance: updates live range and indirect draw arguments.
        // For static procedures without dynamic lifecycles, only the element pass runs.
        let mut parity = self.staged_parity.unwrap_or(self.parity);
        for step in 0..usize::from(steps.min(MAX_STEPS)) {
            if let Some(compaction) = &self.compaction {
                compaction.record(encoder, parity);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("element"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.element);
                pass.set_bind_group(group::UNIFORMS, &self.uniform_bg, &[]);
                pass.set_bind_group(group::PREV, &self.prev_bg[usize::from(parity)], &[]);
                pass.set_bind_group(group::NEXT, &self.next_bg[usize::from(parity)], &[]);
                // Reads substep-specific timing and arguments.
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
                    pass.set_bind_group(group::UNIFORMS, &self.uniform_bg, &[]);
                    pass.set_bind_group(group::PREV, &self.prev_bg[usize::from(parity)], &[]);
                    pass.set_bind_group(group::NEXT, &self.next_bg[usize::from(parity)], &[]);
                    pass.set_bind_group(group::STEP, &self.step_bg[step], &[]);
                    // Direct dispatch sized by host spawn accumulator count.
                    pass.dispatch_workgroups(count.div_ceil(WORKGROUP_SIZE), 1, 1);
                }
            }
            if let Some(compaction) = &self.compaction {
                compaction.record_advance(encoder, step);
            }
            // Alternate buffer parity for ping-pong execution.
            parity = !parity;
        }
        self.staged_parity = Some(parity);
    }

    /// Returns the index (0 or 1) of the buffer holding the most recent simulation output.
    pub(crate) fn parity(&self) -> usize {
        usize::from(self.staged_parity.unwrap_or(self.parity))
    }

    /// Returns the committed parity index on the host.
    pub(crate) fn committed_parity(&self) -> usize {
        usize::from(self.parity)
    }

    /// Commit staged parity and spawn carry upon command buffer submission.
    pub(crate) fn commit(&mut self) {
        if let Some(p) = self.staged_parity.take() {
            self.parity = p;
        }
        if let Some(c) = self.staged_spawn_carry.take() {
            self.spawn_carry = c;
        }
    }

    /// Discard staged parity and spawn carry if an open frame is abandoned without submission.
    pub(crate) fn discard(&mut self) {
        self.staged_parity = None;
        self.staged_spawn_carry = None;
    }

    /// The indirect draw arguments a reader dispatches from — the only place
    /// the live range is known.
    pub(crate) fn counts(&self) -> &wgpu::Buffer {
        &self.counts
    }

    pub(crate) fn element_layout(&self) -> &ElementLayout {
        &self.element_layout
    }

    pub(crate) fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Returns addressable parameter keys for this node.
    pub(crate) fn param_keys(&self) -> &[String] {
        &self.param_keys
    }

    /// Reads the current live element count from the GPU counts buffer.
    ///
    /// Warning: Blocks until the GPU queue drains. Intended for testing and diagnostics;
    /// do not call on the active render frame path.
    pub(crate) fn live_count(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> u32 {
        let bytes = read_buffer(device, queue, &self.counts, counts::SIZE);
        let at = counts::RANGE as usize;
        u32::from_le_bytes(
            bytes[at..at + 4]
                .try_into()
                .expect("counts buffer is 48 bytes"),
        )
    }

    /// Reads raw element buffer bytes for testing.
    ///
    /// Warning: Blocks until the GPU queue drains.
    pub(crate) fn read_elements(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        let size = u64::from(self.capacity) * u64::from(self.element_layout.stride);
        read_buffer(device, queue, self.element_buf.prev(self.parity), size)
    }
}

/// Generates host-side initial byte buffers for elements and alive flags.
fn initial_state(capacity: u32, has_spawn: bool, layout: &ElementLayout) -> (Vec<u8>, Vec<u8>) {
    let stride = layout.stride as usize;
    let mut elements = vec![0u8; capacity as usize * stride];
    let mut alive = vec![0u8; capacity as usize * ALIVE_BYTES as usize];

    if !has_spawn {
        // Mark all slots alive with identity seed values and birth fraction 1.0.
        let seed_offset = layout.offset_of("seed") as usize;
        let birth_frac_offset = layout.offset_of("birth_frac") as usize;
        for i in 0..capacity as usize {
            let base = i * stride;
            elements[base + seed_offset..base + seed_offset + 4]
                .copy_from_slice(&(i as u32).to_le_bytes());
            elements[base + birth_frac_offset..base + birth_frac_offset + 4]
                .copy_from_slice(&1.0f32.to_le_bytes());
            alive[i * ALIVE_BYTES as usize..i * ALIVE_BYTES as usize + 4]
                .copy_from_slice(&1u32.to_le_bytes());
        }
    }

    (elements, alive)
}

/// Generates initial contents for the counts buffer at frame zero.
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

/// Copies `len` bytes from a GPU buffer to host memory, blocking until complete.
pub(super) fn read_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    len: u64,
) -> Vec<u8> {
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
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = slice.get_mapped_range().expect("map");
    let out = data.to_vec();
    drop(data);
    staging.unmap();
    out
}

#[cfg(test)]
mod tests {
    //! Tests verifying initial state byte layout without GPU dependency.
    use super::*;

    fn seed_at(elements: &[u8], stride: usize, layout: &ElementLayout, i: usize) -> u32 {
        let off = i * stride + layout.offset_of("seed") as usize;
        u32::from_le_bytes(elements[off..off + 4].try_into().unwrap())
    }

    fn birth_frac_at(elements: &[u8], stride: usize, layout: &ElementLayout, i: usize) -> f32 {
        let off = i * stride + layout.offset_of("birth_frac") as usize;
        f32::from_le_bytes(elements[off..off + 4].try_into().unwrap())
    }

    fn alive_at(alive: &[u8], i: usize) -> u32 {
        let off = i * ALIVE_BYTES as usize;
        u32::from_le_bytes(alive[off..off + 4].try_into().unwrap())
    }

    /// Verifies that spawn-less procedures initialize slots as alive with identity seeds.
    #[test]
    fn no_spawn_block_seeds_every_slot_with_its_index_and_marks_it_alive() {
        let layout = karakuri_ir::layout::generate_element_layout(
            &[karakuri_ir::Attr::Position, karakuri_ir::Attr::Age],
            karakuri_ir::layout::Synthetic::NONE,
            &[],
        );
        let stride = layout.stride as usize;
        let capacity = 8u32;
        let (elements, alive) = initial_state(capacity, false, &layout);

        assert_eq!(elements.len(), capacity as usize * stride);
        assert_eq!(alive.len(), capacity as usize * ALIVE_BYTES as usize);

        for i in 0..capacity as usize {
            assert_eq!(
                seed_at(&elements, stride, &layout, i),
                i as u32,
                "seed at slot {i}"
            );
            assert_eq!(
                birth_frac_at(&elements, stride, &layout, i),
                1.0,
                "birth_frac at slot {i}"
            );
            assert_eq!(alive_at(&alive, i), 1, "alive flag at slot {i}");
        }
    }

    /// Verifies that procedures with a spawn block initialize all slots to zero.
    #[test]
    fn spawn_block_leaves_every_slot_zeroed() {
        let layout = karakuri_ir::layout::generate_element_layout(
            &[karakuri_ir::Attr::Position, karakuri_ir::Attr::Age],
            karakuri_ir::layout::Synthetic::NONE,
            &[],
        );
        let stride = layout.stride as usize;
        let capacity = 8u32;
        let (elements, alive) = initial_state(capacity, true, &layout);

        assert_eq!(
            elements,
            vec![0u8; capacity as usize * stride],
            "spawn-block elements must start zeroed"
        );
        assert_eq!(
            alive,
            vec![0u8; capacity as usize * ALIVE_BYTES as usize],
            "spawn-block alive flags must start zeroed"
        );
    }
}
