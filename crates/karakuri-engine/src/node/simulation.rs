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

/// The param the engine quantises spawning from. Named once so the uniform
/// path and the accumulator cannot end up reading two different strings.
///
/// **Read through [`Tick::param`] and not from anywhere else**, so a binding
/// attached to it is carried: binding noise to `spawn_rate` is the ir-spec's
/// whole answer to irregular spawning, and a rate that took its manual value
/// here while the uniform took the bound one would be the same param meaning two
/// things in one frame.
///
/// **Resolved against this node's own map**, which it was not always: the
/// grouping used to hold one flat map across every layer, so an L4 that
/// declared a `spawn_rate` would have fed this accumulator. What stopped it was
/// `SetError::ParamCollision` refusing the pair rather than the lookup being
/// right. Values are keyed by node now, that error is gone, and the name means
/// this node's `spawn_rate` and nothing else.
const SPAWN_RATE: &str = "spawn_rate";

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

    /// What the pair costs, asked of the two buffers rather than remembered
    /// from the one `size` they were created with. A remembered number is a
    /// second expression that has to keep agreeing with a `create_buffer` call;
    /// this is the call's own answer.
    fn bytes(&self) -> u64 {
        self.a.size() + self.b.size()
    }
}

/// An L1 node: `() -> Geometry`.
///
/// **All of a Set's per-element state is here.** L4 is stateless and reads
/// whatever this last wrote, which is what makes warming a Set mean running
/// [`Simulation::record`] and skipping the draw — see "Priming" in
/// [`crate::deck`].
///
/// What is *not* here is the clock. `t` is the grouping's, because one clock
/// serves every node in a Set; this node is handed the instants its substeps
/// land on ([`Tick`]) and never derives one.
pub(crate) struct Simulation {
    capacity: u32,
    seed_salt: u32,
    has_spawn: bool,
    /// **This node asked the device for a buffer larger than it allows**, so
    /// the buffers above are error objects and the validation error is already
    /// in `Set::build_many`'s scope, waiting to be returned as a value.
    ///
    /// Recorded here because [`Simulation::initialize`] is the one thing left
    /// that would touch a capacity the device has refused, and it has no device
    /// to ask. The alternative — making [`Simulation::build`] fallible — is
    /// ruled out by that constructor's own doc comment, and for a reason that
    /// still holds: a capacity is refused by `capacity_in_range` before
    /// anything is built, and a second `Err` in the constructor is a second
    /// place for that refusal to live.
    ///
    /// **Windows found this and macOS hid it.** `initial_state` allocates
    /// `capacity * stride` bytes on the host, so a Set built at `u32::MAX`
    /// asked for 128 GiB before the refusal it had already earned could be
    /// read: on macOS that is a lazy reservation and the build limps on to
    /// return the error, and on Windows the allocator aborts the process.
    /// `crates/karakuri-engine/tests/generated.rs`'s
    /// `a_validation_error_at_build_is_returned_rather_than_fatal` is the test
    /// that says this must be a diagnostic, and on Windows it was the thing
    /// killing the test binary.
    refused_by_device: bool,
    /// The monotone spawn ordinal the next new element gets. Advances by
    /// what the engine *asked* for, not by what the GPU managed to fit: a
    /// gap in the seed sequence is harmless, a repeated seed would break
    /// element identity.
    seed_base: u32,
    /// The spawn accumulator, in whole elements. `spawn_rate * dt` is rarely
    /// an integer, so the fractional remainder carries into the next substep
    /// and the long-run rate comes out exact — ir-spec, "Spawn timing".
    spawn_carry: f32,
    /// This frame's per-substep spawn counts, as [`Simulation::prepare`]
    /// computed them. [`Simulation::record`] needs them to size the direct
    /// `spawn` dispatch; the shaders read the same numbers out of `step_args`.
    step_spawn_counts: [u32; MAX_STEPS as usize],
    parity: bool,

    element_layout: ElementLayout,
    element_buf: Pair,
    alive_buf: Pair,

    uniform_layout: UniformLayout,
    /// Host-side staging for the uniform write [`Simulation::prepare`] makes
    /// every frame, sized once at build time against the layout above. That
    /// runs on the render thread and the render thread does not allocate — see
    /// the module doc on `crate::uniforms`. The L4 node keeps its own.
    scratch: UniformScratch,
    uniforms: wgpu::Buffer,
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

    uniform_bg: wgpu::BindGroup,
    /// One per substep, binding `step_args` at that substep's offset.
    step_bg: Vec<wgpu::BindGroup>,
    /// Indexed by parity: [false, true].
    prev_bg: [wgpu::BindGroup; 2],
    next_bg: [wgpu::BindGroup; 2],

    param_names: Vec<String>,
}

impl Simulation {
    /// Generate, compile and allocate one L1 node at `capacity`.
    ///
    /// **Infallible, and that is the point.** `capacity` is a Set-level dial
    /// rather than part of the procedure's identity, and checking it against
    /// the range the artifact declares used to be the one check in this
    /// constructor — which made a device the only way to reach the refusal.
    /// The check is `capacity_in_range` in `crate::set` now, called by
    /// `Set::validate` before anything is built, and it is not repeated here:
    /// the return type is what says so, because a second copy would need
    /// somewhere to put its `Err`.
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
        // **Sized where the figure is decided, not here** — see
        // `crate::storage`. What this constructor knows is that it wants a pair
        // of each; how large one is, and whether the scan's destination index
        // is paid for at all, is the same answer `Plan::element_storage` gives
        // a caller with no device.
        let storage = SimulationStorage::of(capacity, element_layout.stride, shader.compacted);
        let make_pair = |label: &str, size: u64| {
            let make = |suffix: &str| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("{label}_{suffix}")),
                    size,
                    // COPY_SRC only for the readbacks in
                    // `Simulation::live_count` and `Simulation::read_elements`,
                    // both of which are stalls and neither of which is on the
                    // frame path.
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
        // **Asked before it is allocated, because the answer decides whether a
        // host image is built at all.** `create_buffer` reports a size past the
        // device's limit into the enclosing error scope and hands back an error
        // object, which is exactly the refusal `Set::build_many` is there to
        // turn into a value — but only if nothing between here and the `pop()`
        // dies first. See `Simulation::refused_by_device`.
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
        // Built before the bind groups because `element`'s uniform group
        // binds the scan's `dest` buffer, and skipped entirely for a static
        // procedure — the one whose scan would be the identity permutation.
        // **Asked of the storage rather than of the shader a second time.**
        // `Some` here is `shader.compacted`, carried through the one place that
        // charges for it, so the node cannot own a scan its figure left out.
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
        // One layout for both entry points: `element` reaches group STEP too,
        // for this substep's `t`. An earlier revision gave `element` a
        // three-group layout on the grounds that only `spawn` needed the
        // fourth, which stopped being true when `t` moved out of the uniform.
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
            step_spawn_counts: [0; MAX_STEPS as usize],
            parity: false,
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
        }
    }

    /// The edge this node offers downstream, resolved against the buffers it
    /// owns.
    ///
    /// **Both parities, not the current one.** A reader binds once, at build
    /// time, and picks a parity per frame — see [`Geometry`].
    pub(crate) fn geometry(&self) -> Geometry<'_> {
        Geometry {
            layout: &self.element_layout,
            elements: [self.element_buf.prev(false), self.element_buf.prev(true)],
            alive: [self.alive_buf.prev(false), self.alive_buf.prev(true)],
            counts: &self.counts,
        }
    }

    /// **What this node allocated to hold elements** — see [`ElementStorage`]
    /// for what counts and why the figure is read off the buffers.
    ///
    /// **An L1 pays for two directions of everything.** It reads what it wrote
    /// last step, so the element buffer and the alive array each exist twice
    /// and swap; nothing further down the chain does, because nothing further
    /// down reads its own previous output.
    ///
    /// **And a third buffer only where the live set can change.** The scan
    /// writes one destination index per element, and a procedure with no
    /// `spawn` and no `kill()` has no scan at all — it writes in place. That is
    /// precisely the kind of term a figure taken from a procedure's text keeps
    /// missing and a figure taken from the buffers cannot.
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

    /// Zero the element and alive buffers and, for a procedure with no
    /// `spawn` block, seed each slot with its own index — `seed` equals the
    /// initial slot index there, which is what makes lattice generators
    /// work as written. Also puts the counts buffer into the state the first
    /// step's invariant assumes: `prev` holds `range` entries at
    /// `[0, range)`, all of them alive.
    pub(crate) fn initialize(&self, queue: &wgpu::Queue) {
        // **Nothing to initialize, and the host image would be the size the
        // device just refused.** The refusal is already in the build's error
        // scope; writing into error buffers would add nothing to it, and
        // building the image first is how the diagnostic became a dead process.
        // See `Simulation::refused_by_device`.
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

    /// Back to exactly what [`Simulation::build`] left. See [`Set::rewind`],
    /// the one caller, for why that has to be exact rather than nearly so.
    ///
    /// [`Set::rewind`]: crate::set::Set::rewind
    pub(crate) fn rewind(&mut self, queue: &wgpu::Queue) {
        self.seed_base = 0;
        self.spawn_carry = 0.0;
        self.step_spawn_counts = [0; MAX_STEPS as usize];
        self.parity = false;
        self.initialize(queue);
    }

    /// This frame's uniform block and its per-substep arguments.
    ///
    /// Two buffer writes and no allocation: the uniform goes through storage
    /// sized at build time and the step arguments through a stack array,
    /// because this is the render thread.
    pub(crate) fn prepare(&mut self, queue: &wgpu::Queue, tick: &Tick<'_>) {
        // No `t` here: it differs between this frame's substeps and lives in
        // `StepArgs`. Everything left is input, sampled once per frame.
        {
            let mut p = self.scratch.pack(&self.uniform_layout);
            p.f32("dt", tick.dt)
                .u32("capacity", self.capacity)
                .u32("seed_salt", self.seed_salt);
            super::write_params(&mut p, &self.uniform_layout, &self.param_names, tick.param);
            // See `View::field_params`: a field has no uniform of its own.
            super::write_field_params(
                &mut p,
                &self.uniform_layout,
                tick.field_params,
                tick.field_value,
            );
            // **The Source slots this procedure declared** — see
            // [`super::View::source_value`]. An L1 may name a source too: a
            // generator that behaves differently in one geometry of a Set is
            // the same question a mask asks, one layer up.
            super::write_source_slots(&mut p, &self.uniform_layout, tick.source_value);
            queue.write_buffer(&self.uniforms, 0, p.finish());
        }
        self.write_step_args(queue, tick);
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
    /// constant across the substeps, like every other parameter — and it comes
    /// through the same [`Tick::param`] the uniform does, so it carries any
    /// binding attached to it. Binding noise to `spawn_rate` is the ir-spec's
    /// whole answer to irregular spawning; a `spawn_rate` that took its manual
    /// value here while its uniform took the bound one would be the same param
    /// meaning two things in one frame.
    fn write_step_args(&mut self, queue: &wgpu::Queue, tick: &Tick<'_>) {
        self.step_spawn_counts = [0; MAX_STEPS as usize];
        // A procedure with no `spawn` block never creates anything, but the
        // `advance` pass still runs for it if it can `kill()` — with a zero
        // count and the capacity it needs to clamp against.
        let rate = if self.has_spawn {
            (tick.param)(SPAWN_RATE).unwrap_or(0.0)
        } else {
            0.0
        };

        // Clamped again, as [`Simulation::record`] clamps again. `Tick` says
        // its `steps` arrives clamped and the Set does clamp it — but both of
        // these index `MAX_STEPS`-long arrays, and now that the caller is a
        // different type from the callee, "the caller promised" is a comment
        // rather than a compiler's business. Two methods on one struct with two
        // policies for one precondition is how the unguarded one gets found.
        let steps = tick.steps.min(MAX_STEPS);
        let mut bytes = [0u8; MAX_STEPS as usize * step_args::STRIDE as usize];
        for step in 0..usize::from(steps) {
            self.spawn_carry += rate * tick.dt;
            let whole = self.spawn_carry.floor();
            self.spawn_carry -= whole;
            let count = whole.max(0.0) as u32;
            self.step_spawn_counts[step] = count;

            // **The instant, and the grid read at exactly that instant.** Both
            // come from the grouping as one pair, for the reason [`Tick`]
            // gives: deriving `beats` from a step number here would be the same
            // number and a different claim.
            let (t, beats) = tick.instants[step];

            let at = step * step_args::STRIDE as usize;
            bytes[at..at + 4].copy_from_slice(&count.to_le_bytes());
            bytes[at + 4..at + 8].copy_from_slice(&self.seed_base.to_le_bytes());
            bytes[at + 8..at + 12].copy_from_slice(&self.capacity.to_le_bytes());
            bytes[at + 12..at + 16].copy_from_slice(&t.to_le_bytes());
            bytes[at + 16..at + 20].copy_from_slice(&beats.to_le_bytes());

            // By the request, not by what fits: the GPU clamps against
            // capacity and silently drops the overflow, and reusing those
            // seeds on the next frame would give two live elements the same
            // identity. Gaps in the sequence cost nothing.
            self.seed_base = self.seed_base.wrapping_add(count);
        }
        queue.write_buffer(&self.step_args, 0, &bytes);
    }

    /// Record this frame's compute passes. **No render pass, no target, no
    /// draw** — this is the whole of what a Priming slot runs.
    ///
    /// Must be paired with a [`Simulation::prepare`] in the same frame: the
    /// uniforms and each substep's `t` come from there.
    pub(crate) fn record(&mut self, encoder: &mut wgpu::CommandEncoder, steps: u8) {
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
        for step in 0..usize::from(steps.min(MAX_STEPS)) {
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
                pass.set_bind_group(group::UNIFORMS, &self.uniform_bg, &[]);
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
                    pass.set_bind_group(group::UNIFORMS, &self.uniform_bg, &[]);
                    pass.set_bind_group(group::PREV, &self.prev_bg[usize::from(parity)], &[]);
                    pass.set_bind_group(group::NEXT, &self.next_bg[usize::from(parity)], &[]);
                    pass.set_bind_group(group::STEP, &self.step_bg[step], &[]);
                    // Direct, because the count is host state: the
                    // accumulator lives on this node so that the sequence is a
                    // pure function of the record stream. The GPU is what
                    // clamps it against the free range.
                    pass.dispatch_workgroups(count.div_ceil(WORKGROUP_SIZE), 1, 1);
                }
            }
            if let Some(compaction) = &self.compaction {
                compaction.record_advance(encoder, step);
            }
            // What this step wrote as "next" is the next step's "prev", and
            // after the last one it is what L4 reads.
            self.parity = !self.parity;
        }
    }

    /// Which half of [`Geometry`]'s arrays holds what was last written.
    pub(crate) fn parity(&self) -> usize {
        usize::from(self.parity)
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

    pub(crate) fn param_names(&self) -> &[String] {
        &self.param_names
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
    pub(crate) fn live_count(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> u32 {
        let bytes = read_buffer(device, queue, &self.counts, counts::SIZE);
        let at = counts::RANGE as usize;
        u32::from_le_bytes(
            bytes[at..at + 4]
                .try_into()
                .expect("counts buffer is 48 bytes"),
        )
    }

    /// The raw bytes of the element buffer a reader is currently reading,
    /// decoded against [`Simulation::element_layout`]. **A stall, on the same
    /// terms as [`Simulation::live_count`]** — this exists so a test can check
    /// that survivors kept their order, which is a claim about `seed` values in
    /// slots and cannot be made from a rendered image.
    pub(crate) fn read_elements(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        let size = u64::from(self.capacity) * u64::from(self.element_layout.stride);
        read_buffer(device, queue, self.element_buf.prev(self.parity), size)
    }
}

/// The host-side byte image [`Simulation::initialize`] uploads: the interleaved
/// `Element` buffer and the dense `alive` buffer, computed against
/// `layout`'s published offsets rather than against any assumption of its
/// own about where a field lands. Pulled out as a pure function — no device,
/// no queue — so the byte-offset arithmetic can be tested directly against
/// `ElementLayout` without a GPU in the loop; see
/// `tests::no_spawn_block_seeds_every_slot_with_its_index_and_marks_it_alive`
/// below.
fn initial_state(capacity: u32, has_spawn: bool, layout: &ElementLayout) -> (Vec<u8>, Vec<u8>) {
    let stride = layout.stride as usize;
    let mut elements = vec![0u8; capacity as usize * stride];
    let mut alive = vec![0u8; capacity as usize * ALIVE_BYTES as usize];

    if !has_spawn {
        // Everything is alive, and nothing needs a birth-fraction
        // correction because nothing was born mid-frame.
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

/// Copies `len` bytes off the GPU and blocks until they arrive. Every caller
/// is a stall by construction — see [`Simulation::live_count`].
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
    //! Byte-level checks for `initial_state`, the pure function behind
    //! `Simulation::initialize`'s interleaved write. The doc on
    //! `karakuri_ir::layout::ElementSlot` warns that getting the host-side
    //! write wrong is silent — "an element reading the middle of the element
    //! before it" — so this checks the exact bytes at the exact offsets
    //! `ElementLayout` publishes, independently of whatever `initialize`
    //! itself does, and needs no GPU to do it.
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

    /// For a spawn-less procedure every slot must come up `seed == i`,
    /// `birth_frac == 1.0`, `alive == 1` — the exact byte offsets
    /// `ElementLayout` publishes, not merely "a shader can read it without
    /// erroring."
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

    /// The other side of the same coin: a procedure *with* a `spawn` block
    /// starts with an empty range, so `initial_state` must leave every slot
    /// zeroed — `seed`, `birth_frac`, and `alive` alike — rather than
    /// reusing the no-spawn seeding path. A stray `alive == 1` here would
    /// make dead slots read as live the moment the range grew past them.
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
