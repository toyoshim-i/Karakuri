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

use crate::binding::{Binding, Signals};
use karakuri_signal::Oscillator;
use crate::camera::Orbit;
use crate::compaction::Compaction;
use crate::uniforms::UniformScratch;
use crate::video_source::VideoSource;

/// Past this the simulation falls behind rather than catching up — the
/// ir-spec's cap, restated here because it is now load-bearing rather than
/// advisory: each substep needs its own spawn-count entry, and that array is
/// sized once, at build time.
pub const MAX_STEPS: u8 = 4;

/// The fixed simulation step. **Not** the real frame delta — see the
/// determinism invariant in `README.md`. Public because the session clock a
/// binding reads has to advance by exactly this: an oscillator on a different
/// step would drift away from the `t` the Sets are running at, and the drift
/// would be invisible until a beat landed in the wrong place.
pub const DT: f32 = 1.0 / 60.0;

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
    /// One param name declared by both procedures.
    ///
    /// [`Set::params`] is keyed by name alone across both layers, so two
    /// declarations of one name are one value: the second silently takes the
    /// first's place, a `param` record moves both at once, and a `bind` — which
    /// *is* keyed by layer — blends from whichever declaration happened to
    /// land last. Neither procedure can see the other's names, so this is a
    /// coincidence rather than a mistake, and it is caught here because the
    /// pair is the first place both are in hand. Every collision is reported at
    /// once, for the same reason the checker reports every error at once.
    ///
    /// The fix on the other side is to key values by layer, the way the record
    /// format already does; until then, refusing beats picking one.
    #[error(
        "`{l1}` and `{l4}` both declare a param named {keys}\n\
         hint: parameter values are keyed by name across the whole Set, so the two \
         declarations would be one value — rename one side"
    )]
    ParamCollision {
        l1: String,
        l4: String,
        keys: String,
    },
    /// A rendering mode the language admits and this engine cannot run yet.
    ///
    /// **Named rather than left to panic.** `generate_l4` used to `expect` a
    /// vertex block, so a procedure without one — which is how a fullscreen
    /// renderer says what it is — would have killed the build worker silently.
    /// The language accepting something the engine refuses is a state worth
    /// having briefly and worth saying out loud: it is what the eight SDF
    /// builtins have been in since M1, and the whole point of the fullscreen
    /// work is to end it.
    #[error(
        "`{l4}` draws the whole frame, which this engine cannot do yet\n\
         hint: a fullscreen renderer is an L4 with no `vertex` block. The language accepts \
         one; the draw path that would run it is not built. Give it a `vertex` block to \
         draw per element in the meantime"
    )]
    Unrenderable { l4: String },
    /// A build panicked rather than returning. Not reachable through any
    /// `.kir` a checker accepts, which is exactly why it needs a variant:
    /// wgpu's default handler for an uncaptured validation error is a panic,
    /// so generated WGSL that naga refuses kills whatever thread built it.
    /// On the swap worker that is silent — the render thread keeps running
    /// and simply never receives anything again. A rejection says so.
    #[error("building `{label}` panicked, which is a bug in this compiler rather than in the `.kir`: {detail}")]
    Panicked { label: String, detail: String },
}

/// Which clock a binding's oscillator signals are read on. Private: the choice
/// belongs to [`Set::prepare`] and [`Set::prepare_warming`], which name the two
/// situations it distinguishes, and a caller picking a clock directly would be
/// picking one without the situation that justifies it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Clock {
    /// The session's, as handed in. **What a slot on air reads**: it is in the
    /// room, and the room's beat is the session's however far behind the
    /// slot's own clock has fallen.
    Session,
    /// The same grid, read at this Set's own `t`. **What a slot warming off
    /// air reads** — see [`Set::prepare_warming`] for the whole argument.
    Local,
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
    /// The `beats` the last [`Set::prepare`] wrote into the L4 uniform block.
    ///
    /// Kept so that [`Set::refresh_view`] can rewrite that block for a slot
    /// nothing is preparing without moving the grid position it was drawn at.
    /// Derived, never authoritative: the oscillator is the grid, and this is
    /// the answer it gave at this Set's `t`.
    last_beats: f32,
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
    /// Both procedures are a pure function of `seed`, `t`, and their params —
    /// see [`Set::is_closed_form`]. Decided by the check pass and carried here
    /// rather than re-derived; the engine never looks at IR.
    closed_form: bool,
    /// Whether either procedure reads the `beats` ambient — see
    /// [`Set::reads_beats`].
    reads_beats: bool,

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

    /// **Manual** parameter values: the `.kir` defaults, as moved by a `param`
    /// record or a `--param` override. A binding never writes here — it blends
    /// *from* here — so a param that is both bound and set by hand has one
    /// answer rather than a race between two writers. See [`Set::bind`].
    pub params: HashMap<String, f32>,
    pub camera: Orbit,
    l1_param_names: Vec<String>,
    l4_param_names: Vec<String>,
    /// At most one per (layer, param). Resolved once per frame in
    /// [`Set::prepare`] and read back out wherever a param value is written.
    bindings: Vec<Binding>,
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

        // Also before anything is generated, and for the same reason: the two
        // procedures were written without sight of each other, so a shared
        // param name is a coincidence the pair is the first thing able to see.
        // See `SetError::ParamCollision` for why it is refused rather than
        // resolved.
        let clashing: Vec<String> = l1
            .params
            .iter()
            .filter(|p| l4.params.iter().any(|q| q.name == p.name))
            .map(|p| format!("`{}`", p.name))
            .collect();
        if !clashing.is_empty() {
            return Err(SetError::ParamCollision {
                l1: l1.name.clone(),
                l4: l4.name.clone(),
                keys: clashing.join(", "),
            });
        }

        // **There is deliberately no third check, comparing the two
        // topologies.** An L1 declares one and an L4 now carries an inferred
        // one, so the comparison is available and looks principled — and it
        // would refuse the pairing M3 exists to enable: the same cloud drawn
        // as sprites by one L4 and as streaks by another. A segment under
        // `Topology::Lines` gets both of its ends from attributes the L4
        // consumes, so a renderer needs nothing from the geometry beyond what
        // the composition check above already verifies. The declaration on
        // the L1 side says what the geometry is *meant to read as*; it
        // constrains no renderer, and requiring the two to agree would invent
        // a dependency the lowering does not have.
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

        // Before generation, because generation would panic rather than refuse:
        // see `SetError::Unrenderable`.
        if l4.topology == Some(karakuri_ir::Topology::Fullscreen) {
            return Err(SetError::Unrenderable { l4: l4.name.clone() });
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
                    //
                    // **Colour adds; alpha accumulates coverage.** The two
                    // components answer different questions and this is the
                    // only pairing that answers both: colour is emissive and
                    // sums past what any coverage would allow, which is what
                    // `blend additive` is for, while alpha comes out as
                    // `1 - prod(1 - a_i)` — the probability that *something*
                    // drew at this texel, and order-independent because
                    // `a_s + a_d(1 - a_s)` is symmetric in the two.
                    //
                    // **Not bounded at 1, and the mix does not assume it is.**
                    // Nothing clamps what a fragment block assigns to alpha —
                    // the IR calls it straight alpha and says values above 1.0
                    // are expected — so this accumulates whatever the material
                    // wrote. `composite.wgsl` saturates on the way in rather
                    // than L4 clamping on the way out, because clamping here
                    // would change the colour too: additive blending
                    // multiplies colour by this same alpha.
                    //
                    // Nothing in this pass reads it back. It exists for L5:
                    // `Blend::Over` needs to know what a layer covers, and
                    // before this the channel was written by nothing and held
                    // the clear value forever. Colour is premultiplied by
                    // coverage on the way out, which is what makes the mix's
                    // `over` a multiply-add rather than a divide by an alpha
                    // that is allowed to be zero.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
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
            dt: DT,
            last_beats: 0.0,
            spawn_carry: 0.0,
            step_spawn_counts: [0; MAX_STEPS as usize],
            viewport: [1.0, 1.0],
            parity: false,
            has_spawn: l1_shader.has_spawn,
            // Both, because a Set is only seekable if everything in it is. L4
            // is stateless and its flag is vacuously true, so in practice this
            // is the L1's — but writing the conjunction is what keeps it
            // correct when L2 arrives with state of its own.
            closed_form: l1.closed_form && l4.closed_form,
            reads_beats: l1.reads_beats || l4.reads_beats,
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
            bindings: Vec::new(),
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

    /// What [`Set::resize`] last set, as it was clamped. The camera's aspect
    /// ratio comes off this, so a caller that resizes a Set temporarily — the
    /// probe does, to a fixed reference size — has somewhere to read the old
    /// value back from rather than having to remember it.
    pub fn viewport(&self) -> (u32, u32) {
        (self.viewport[0] as u32, self.viewport[1] as u32)
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

    /// **Whether this Set's state at any `t` is reachable by evaluating it
    /// rather than by running forward to it.**
    ///
    /// True when both procedures are a pure function of `seed`, `t`, and their
    /// parameters. Two consequences: it can be taken from Cold to Live with no
    /// warm-up, and it can be scrubbed — forwards at any rate, held, or
    /// backwards. Decided by the check pass (see `Checked::closed_form`) and
    /// deliberately conservative: a `true` here is a promise that skipping the
    /// warm-up shows the same image warming would have, and a `false` may be
    /// pessimistic.
    ///
    /// Two consumers: [`crate::governor`], where a closed-form Set has nothing
    /// to prime and is never worth compute budget; and [`crate::transport`],
    /// where it is what makes beat sync possible at all — a position lock has
    /// to be able to land on a position.
    pub fn is_closed_form(&self) -> bool {
        self.closed_form
    }

    /// **Whether this Set's material is written against the tempo grid** —
    /// either procedure reads the `beats` ambient.
    ///
    /// Such material already follows the room, so a transport that also scaled
    /// its clock by the tempo would make it follow twice and run at roughly the
    /// square of the tempo ratio. [`crate::transport`] refuses that combination
    /// rather than offering it; see `Checked::reads_beats`.
    pub fn reads_beats(&self) -> bool {
        self.reads_beats
    }

    /// **Put the simulation clock at `steps_taken` without running anything.**
    ///
    /// The seek half of a transport. What follows must be exactly one
    /// [`Set::prepare`] of one step and one [`Set::render`] of one step: this
    /// leaves the counter one short of the target, `prepare` bumps it onto the
    /// target and writes the uniforms for that instant, and the single element
    /// pass evaluates the procedure there. One pass is enough because the
    /// caller has promised the procedure is closed form, which is exactly the
    /// promise that its state at `t` does not depend on how it got there.
    ///
    /// **Only for a closed-form Set**, and this does not check, because it
    /// cannot usefully: the Set knows ([`Set::is_closed_form`]) but the
    /// alternative to a caller that checks is a caller that gets a silent
    /// wrong answer either way — an accumulating Set seeked to `t` evaluates
    /// once from wherever it happened to be, which is garbage rather than an
    /// error. [`crate::transport`] is the one caller and refuses beat sync on
    /// accumulating material at the point the operator asks for it, where there
    /// is something to say.
    ///
    /// The element buffers are left alone. They hold the previous instant's
    /// values, which a closed-form `element` block does not read.
    pub fn seek(&mut self, steps_taken: u64) {
        self.steps_taken = steps_taken;
    }

    /// Put this Set back to exactly what [`Set::build`] left: element and alive
    /// buffers at their initial contents, `t` at zero, parity, the spawn
    /// accumulator and the seed counter all reset.
    ///
    /// **Not for the frame path and not a lifecycle operation.** It exists for
    /// one caller: `swap.rs` measures a freshly built Set with the probe before
    /// handing it to the render thread, and measuring means stepping it. A
    /// swapped-in Set is documented as arriving cold, so the measurement has to
    /// leave no trace — this is what makes that true rather than nearly true.
    ///
    /// It re-uploads the whole element and alive buffers, so it is as expensive
    /// as `build`'s own upload and belongs on the worker thread beside it.
    pub fn rewind(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.seed_base = 0;
        self.steps_taken = 0;
        self.spawn_carry = 0.0;
        self.step_spawn_counts = [0; MAX_STEPS as usize];
        self.parity = false;
        self.initialize(device, queue);
    }

    /// Attach a signal to a `param`. Returns `false` if `binding.layer`
    /// declares no scalar `param` of that name, which is the same non-fatal
    /// shape a `--param` for an unknown name has: a Set file naming a param a
    /// regenerated artifact no longer has should not take the show down.
    ///
    /// **At most one binding per (layer, param)**, so a second one replaces
    /// the first rather than stacking behind it. Two bindings on one param
    /// would be resolved in vector order and the winner would be whichever was
    /// attached last — "the last writer wins", which is exactly the answer
    /// this design refuses everywhere else.
    ///
    /// Allocates, so not on the render thread. A binding arrives with a Set
    /// (from a Set file, from `--bind`, or from a rebuild's `Request`), and
    /// all three are off the frame path.
    pub fn bind(&mut self, binding: Binding) -> bool {
        let declared = match binding.layer {
            Kind::L1 => &self.l1_param_names,
            Kind::L4 => &self.l4_param_names,
        };
        // Both checks: `params` holds only the scalar params — a vector one is
        // declared but has no value here — and a binding produces one float.
        if !declared.contains(&binding.key) || !self.params.contains_key(&binding.key) {
            return false;
        }
        self.bindings
            .retain(|b| b.layer != binding.layer || b.key != binding.key);
        self.bindings.push(binding);
        true
    }

    /// Every bound param and what it was last written with. For a status line:
    /// a binding that is doing nothing and a binding that is not there look
    /// identical from outside otherwise.
    pub fn bound(&self) -> impl Iterator<Item = (&str, f32)> {
        self.bindings.iter().map(|b| (b.key.as_str(), b.value()))
    }

    /// The bindings themselves, for a caller that has to carry them across a
    /// rebuild.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
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
    /// `signals` is the **session's** oscillator and seed, one per deck rather
    /// than one per Set, and it has already been advanced by this frame's
    /// `steps` when this is called — so a binding reads the phase at the
    /// instant of the frame's last substep. Every binding is resolved once,
    /// here, and the value is reused wherever that param is written; resolving
    /// twice in one frame would put two different values into one frame.
    ///
    /// **This is the on-air form**, and it reads the session's position on the
    /// grid. A Set warming off air is behind that position and wants
    /// [`Set::prepare_warming`], which is this function with one difference.
    ///
    /// **Nothing in here allocates.** Both uniform writes go through storage
    /// sized at build time (`crate::uniforms::UniformScratch`) and the step
    /// arguments through a stack array, because this is the render thread and
    /// the first invariant in `README.md` is the one about allocating on it.
    /// Binding resolution is the same: a fixed `Vec` written in place, a
    /// stack-sized bus over a borrowed oscillator, and a linear scan to read
    /// values back out.
    pub fn prepare(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals) {
        self.prepare_on(queue, steps, signals, Clock::Session);
    }

    /// [`Set::prepare`] for a Set that is **warming off air**: identical in
    /// every respect but one — oscillator signals are read on this Set's own
    /// clock instead of the session's.
    ///
    /// The difference only exists because a warming slot's clock is behind the
    /// room's. It steps on some frames and not others, so `steps_taken * dt`
    /// falls further behind the session's `t` the slower it is warmed, and
    /// handing it the session's phase would make a binding advance by a whole
    /// frame's worth of beats for every step the slot actually takes. **The
    /// governor picks that rate.** A Set warmed at one step in four would then
    /// warm into different material than the same Set warmed at full rate — a
    /// performance knob, invisible to the operator, silently changing the
    /// picture. Reading the grid at the slot's own `t` removes the rate from
    /// the arithmetic entirely.
    ///
    /// **The lag is a step count, so a slot that is not behind reads exactly
    /// what it would have read on air.** A slot warming at full rate takes a
    /// step whenever the session does, its lag is zero, and
    /// [`Signals::behind`] hands back the session's own oscillator bit for bit
    /// — which is what makes "primed then Live" *identical* to "always Live"
    /// rather than close to it, for bound material as well as unbound. Deriving
    /// a position from this Set's `t` instead would cost an f32 rounding the
    /// session's accumulated `t` never took, and the identity would hold to
    /// about seven digits: it diverges within a second at 120 bpm, and every
    /// step after that reads a different value.
    ///
    /// Two residues, both real and neither fixable here:
    ///
    /// - **Tempo corrections.** Warming slower spans more wall time and
    ///   therefore more corrections, and [`Oscillator::behind`] gives the grid
    ///   as it stands rather than as it was. Invariance holds against a steady
    ///   tempo, not across a change of one.
    /// - **Measured audio.** `energy` and the bands are this frame's
    ///   measurement at every rate, because there is no other measurement to
    ///   give. A Set bound to audio warms into whatever the room was doing while
    ///   it warmed. *Synthesized* `energy` — what the bus invents when nothing
    ///   is measuring — does move with the clock, because it is a function of
    ///   it; [`Signals::behind`] has the split.
    ///
    /// Going on air moves the slot back to the session's grid, and normally
    /// nothing sees the discontinuity that causes: the frame before was not
    /// drawn. That is the whole reason the split is safe — **off air is not in
    /// the room**, and a slot on air must be on the room's beat however far
    /// behind its own clock is.
    ///
    /// **An audition is the case where the frame before *is* drawn**, and it is
    /// the one place that discontinuity is visible: a warming slot is shown at
    /// its own grid position, so material that reads `beats` or carries a
    /// binding moves the moment it goes on air, by however far behind the
    /// governor's rate had it. Named rather than closed, because the close is
    /// to read the session's grid instead — which is the defect this split cost
    /// a repair to fix. See
    /// [`Deck::set_preview`](crate::deck::Deck::set_preview).
    pub fn prepare_warming(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals) {
        self.prepare_on(queue, steps, signals, Clock::Local);
    }

    /// The one body. Both entry points come through here so that "identical in
    /// every respect but one" is structural rather than a claim two functions
    /// have to keep making about each other.
    fn prepare_on(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals, clock: Clock) {
        let steps = steps.min(MAX_STEPS);
        self.steps_taken += u64::from(steps);
        // After the bump, so `Clock::Local` measures the lag as of *this*
        // frame's last substep — the instant `Clock::Session` reads, because
        // the deck advances the session's oscillator before it prepares
        // anything. Reading before it would put every warming binding a frame
        // early.
        //
        // **One grid position for this Set, this frame**, and everything that
        // reads the grid reads it: the bindings below and the `beats` every
        // substep is given. Two lookups could not disagree even in principle,
        // but computing it once is what makes that true by construction rather
        // than by two call sites happening to pass the same argument.
        let view = match clock {
            Clock::Session => *signals,
            // **Subtract step counts, not times.** Both clocks are integer
            // counters of the same `dt`, so their difference is exact and is
            // zero whenever they agree; two `t`s derived from them are not
            // exact and their difference is not zero. `saturating_sub` because
            // a Set may have taken more steps than the session's oscillator —
            // a Set built and stepped before it was ever put on a deck — and
            // that is a slot ahead of the room, which reads the room's phase
            // rather than an extrapolated future one.
            Clock::Local => {
                let lag = signals
                    .oscillator()
                    .steps_taken()
                    .saturating_sub(self.steps_taken);
                signals.behind(lag as f64 * f64::from(self.dt))
            }
        };
        self.resolve_bindings(&view);

        // No `t` here: it differs between this frame's substeps and lives in
        // `StepArgs`. Everything left is input, sampled once per frame.
        {
            let (bindings, params) = (&self.bindings, &self.params);
            let mut p = self.l1_scratch.pack(&self.l1_uniform_layout);
            p.f32("dt", self.dt)
                .u32("capacity", self.capacity)
                .u32("seed_salt", self.seed_salt);
            for name in &self.l1_param_names {
                p.f32(name, effective(bindings, params, Kind::L1, name));
            }
            queue.write_buffer(&self.l1_uniforms, 0, p.finish());
        }

        self.write_step_args(queue, steps, view.oscillator());

        // The grid at exactly this frame's `t`, on the same terms as the
        // per-substep `beats` above: one instant, named twice, derived once.
        // Kept, because [`Set::refresh_view`] has to be able to rewrite this
        // block without moving it.
        self.last_beats = view.oscillator().at_time(f64::from(self.time())).beats() as f32;
        self.write_l4_uniforms(queue);
    }

    /// The L4 uniform block, from state this does not change.
    ///
    /// Split out of [`Set::prepare_on`] because a preview needs it without the
    /// rest: an audition draws a slot nothing prepared, and every field here
    /// but the viewport is already what it should be.
    fn write_l4_uniforms(&mut self, queue: &wgpu::Queue) {
        // Read before the packer borrows the scratch: `time` and `view_proj`
        // take `&self`, and the packer holds a `&mut` to one of its fields.
        let t = self.time();
        let beats = self.last_beats;
        let aspect = self.viewport[0] / self.viewport[1];
        let camera = self.camera.view_proj(t, aspect);
        let (bindings, params) = (&self.bindings, &self.params);
        let mut p = self.l4_scratch.pack(&self.l4_uniform_layout);
        p.f32("t", t)
            .f32("beats", beats)
            .u32("seed_salt", self.seed_salt)
            .vec2("viewport", self.viewport)
            .mat4("camera", camera);
        for name in &self.l4_param_names {
            p.f32(name, effective(bindings, params, Kind::L4, name));
        }
        queue.write_buffer(&self.l4_uniforms, 0, p.finish());
    }

    /// **Rewrite the L4 uniforms against the current viewport**, without
    /// advancing anything.
    ///
    /// For a slot being auditioned that nothing is preparing. `viewport` and
    /// the camera's aspect ratio are written by [`Set::prepare`] and by nothing
    /// else, while [`Set::resize`] moves only the host-side value — so an
    /// `Allocated` slot drawn after a resize would draw at the aspect ratio it
    /// had before it, for as long as it stayed off air. Which is to say
    /// permanently, since going off air is what stops it being prepared.
    ///
    /// **Every other field comes out unchanged**, and that is the whole
    /// contract: `t` is `steps_taken * dt` and nothing here steps, `beats` is
    /// the value the last `prepare` derived, and a bound parameter is whatever
    /// it last resolved to — bindings are not re-resolved, because resolving
    /// them against a moving grid would make a parked slot's parameters drift
    /// while its geometry stood still.
    pub fn refresh_view(&mut self, queue: &wgpu::Queue) {
        self.write_l4_uniforms(queue);
    }

    /// Every binding, once, against the signals it was handed — the session's
    /// on air, the same ones read at this Set's `t` while warming. Which is
    /// [`Set::prepare_on`]'s to decide and not this function's: it resolves
    /// against what it is given.
    ///
    /// Allocates nothing: the `Vec` is written in place, and each binding's
    /// manual value is read out of `params` — which is never written here, so
    /// a `--param` on a bound param survives the frame.
    fn resolve_bindings(&mut self, signals: &Signals) {
        for binding in &mut self.bindings {
            // `unwrap_or` rather than an index: `Set::bind` refuses a param
            // that is not in the map, so this cannot miss, and a panic on the
            // render thread is not the way to find out if it ever does.
            let manual = self.params.get(&binding.key).copied().unwrap_or(0.0);
            binding.resolve(signals, manual);
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
    /// constant across the substeps, like every other parameter — and it is
    /// read through the same binding resolution the uniform is, because
    /// binding noise to `spawn_rate` is the ir-spec's whole answer to
    /// irregular spawning. A `spawn_rate` that took its manual value here
    /// while its uniform took the bound one would be the same param meaning
    /// two things in one frame.
    fn write_step_args(&mut self, queue: &wgpu::Queue, steps: u8, grid: &Oscillator) {
        self.step_spawn_counts = [0; MAX_STEPS as usize];
        // A procedure with no `spawn` block never creates anything, but the
        // `advance` pass still runs for it if it can `kill()` — with a zero
        // count and the capacity it needs to clamp against.
        let rate = if self.has_spawn && self.params.contains_key(SPAWN_RATE) {
            effective(&self.bindings, &self.params, Kind::L1, SPAWN_RATE)
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
            // **The grid read at exactly the `t` written two lines up.**
            // `beats` is defined as the grid at the instant `t` names, so it is
            // derived from that `t` — not from a position counted back from the
            // session's, which would be the same number and a different claim.
            // See `Oscillator::at_time` for why "the same number" is a measured
            // fact here rather than a hopeful one.
            let beats = grid.at_time(f64::from(self.t_at(first + step as u64))).beats() as f32;
            bytes[at + 16..at + 20].copy_from_slice(&beats.to_le_bytes());

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

/// The param the engine quantises spawning from. Named once so the uniform
/// path and the accumulator cannot end up reading two different strings.
const SPAWN_RATE: &str = "spawn_rate";

/// What a param is actually written with: its binding's value if it has one,
/// its manual value otherwise.
///
/// A linear scan, deliberately. This is the render thread: a `HashMap` keyed
/// by `String` would hash a name per param per frame to search a list that is
/// never longer than the params a procedure declares, and the scan touches one
/// cache line for a Set with no bindings at all — which is every Set today.
fn effective(
    bindings: &[Binding],
    params: &HashMap<String, f32>,
    layer: Kind,
    name: &str,
) -> f32 {
    match bindings
        .iter()
        .find(|b| b.layer == layer && b.key == name)
    {
        Some(binding) => binding.value(),
        None => params[name],
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

impl Set {
    /// Record this frame's L1 passes and **nothing else** — no render pass, no
    /// target, no draw.
    ///
    /// This is the whole of what a Priming slot runs. All of a Set's
    /// per-element state is L1's: L4 is stateless and reads whatever L1 last
    /// wrote, so warming a Set means running this and skipping the draw. See
    /// "Priming" in [`crate::deck`] for why that is the right shape and why
    /// `docs/roadmap.md`'s "reduced resolution" is superseded by it.
    ///
    /// [`VideoSource::render`] is this followed by the draw, so the two cannot
    /// disagree about what a step is: there is one copy of the pass sequence
    /// and the parity flip that goes with it.
    ///
    /// Must be paired with a [`Set::prepare`] in the same frame, exactly as
    /// `render` must: the uniforms and this substep's `t` come from there.
    pub fn step(&mut self, encoder: &mut wgpu::CommandEncoder, steps: u8) {
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
    }
}

impl Set {
    /// **The draw, without advancing anything.**
    ///
    /// The L4 pass over whatever L1 last wrote, which for a Set nothing has
    /// stepped this frame is the state it stopped at. Split out of
    /// [`VideoSource::render`] for the same reason [`Set::step`] was split out
    /// of it: a preview draws without stepping and a Priming slot steps without
    /// drawing, and two copies of a render pass is how the two come to disagree
    /// about which parity L4 reads.
    ///
    /// Nothing here touches `t`, `steps_taken` or `parity`. That is what lets
    /// an operator look at an `Allocated` slot without the act of looking
    /// moving it — see [`Deck::set_preview`](crate::deck::Deck::set_preview).
    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let render_parity = usize::from(self.parity);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("L4"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // `TRANSPARENT`, not `BLACK`: alpha in this target is
                        // coverage, accumulated by the blend state below, and
                        // it has to start at "nothing drew here". `BLACK` is
                        // opaque black and would hand the L5 mix a slot that
                        // covers the frame before a single sprite has run.
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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

impl VideoSource for Set {
    /// This frame's L1 passes, then the draw.
    ///
    /// The compute half is [`Set::step`] verbatim and the raster half is
    /// [`Set::draw`] verbatim, because a Priming slot runs the first and a
    /// preview runs the second; keeping one copy of each is what stops "primed
    /// for thirty frames then put on air" from being a different simulation
    /// than "on air for thirty frames", and an auditioned slot from being a
    /// different picture than the same slot on air.
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        steps: u8,
    ) {
        self.step(encoder, steps);
        self.draw(encoder, target);
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
