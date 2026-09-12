//! A Set groups nodes that form one video source, representing the unit of compilation and lifecycle.
//!
//! GPU state is owned by individual nodes in [`crate::node`]:
//! - L1 simulation nodes own element and alive buffers, counts, compaction scan, and spawn accumulators.
//! - L4 renderer nodes own render pipelines, uniform buffers, accumulation targets, and bind groups.
//!
//! The Set manages shared parameter values, bindings, viewport state, simulation clock, and execution order.
//! Parameter values are written via uniform buffers, while node compilation and pipeline generation
//! remain decoupled within their respective modules.

use std::collections::{HashMap, HashSet};

use karakuri_codegen::{generate_l1, generate_l2};
use karakuri_ir::layout::{ElementLayout, Synthetic};
use karakuri_ir::typed::{Checked, InputPort};
use karakuri_ir::Kind;

use crate::binding::{Binding, ParamWrite, Signals, CONTROL_PREFIX};
use crate::camera::Orbit;
use crate::mix::Input;
use crate::node::{Deform, Renderer, Simulation};
use crate::storage::{DeformStorage, SimulationStorage};
use crate::video_source::VideoSource;

/// Maximum simulation substeps per frame before falling behind.
pub const MAX_STEPS: u8 = 4;

/// Default name assigned to the built-in orbit camera when unnamed.
pub const BUILTIN_CAMERA: &str = "orbit";

/// Memory allocated by a node for per-element storage and corresponding element capacity.
///
/// Only per-element buffers (element buffers, alive flags, and compaction indices) are counted.
/// Fixed-size overheads such as uniforms, counts blocks, and pyramid scans are excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementStorage {
    /// Sum of sizes in bytes of the node's per-element buffers.
    pub bytes: u64,
    /// Number of elements covered by these allocations.
    pub capacity: u32,
}

impl ElementStorage {
    /// Returns the exact bytes allocated per element. Returns 0 if capacity is 0.
    pub fn per_element(self) -> u64 {
        match self.capacity {
            0 => 0,
            n => self.bytes / u64::from(n),
        }
    }
}

/// Fixed simulation time step in seconds (60 Hz).
pub const DT: f32 = 1.0 / 60.0;

/// Rendering compositing strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layering {
    /// Renderers sequentially draw into a single attachment.
    #[default]
    Overdraw,
    /// Each renderer draws into an isolated target folded by [`crate::node::Merge`].
    Composite,
}

/// Node authority level governing parameter control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Authority {
    /// Node parameters are controlled solely by direct user input.
    #[default]
    Manual,
    /// Automated agents propose parameter adjustments for manual acceptance.
    Suggesting,
    /// Automated agents directly adjust node parameters.
    Automatic,
}

impl Authority {
    /// All authority levels in increasing order of automation.
    pub const ALL: [Authority; 3] = [
        Authority::Manual,
        Authority::Suggesting,
        Authority::Automatic,
    ];

    /// Returns the lowercase string representation of the authority level.
    pub fn name(self) -> &'static str {
        match self {
            Authority::Manual => "manual",
            Authority::Suggesting => "suggesting",
            Authority::Automatic => "automatic",
        }
    }

    /// Parses an authority level from its lowercase name, returning `None` if unrecognized.
    pub fn from_name(name: &str) -> Option<Authority> {
        Authority::ALL.into_iter().find(|a| a.name() == name)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SetError {
    /// Multiple nodes in a Set share the same name.
    #[error("two nodes are both called `{name}` — a name addresses one node in a Set")]
    DuplicateNodeName { name: String },
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
    /// An L4 renderer consumes an attribute that the L1 source does not emit.
    #[error(
        "`{l4}` consumes {missing} which `{l1}` does not emit\n\
         hint: {hint}"
    )]
    Composition {
        l1: String,
        l4: String,
        missing: String,
        /// Contextual resolution hint.
        hint: String,
    },
    /// A GPU resource or pipeline creation failed driver validation checks.
    #[error(
        "`{proc}` produced something this device refused, which is a compiler bug rather \
         than a mistake in the file: {detail}\n\
         hint: it should have been refused with a sentence about the `.kir`. Please report \
         the procedure and this message"
    )]
    Invalid { proc: String, detail: String },
    /// Combined complexity of caller and inlined field exceeds instruction budget.
    #[error(
        "`{caller}` with `{field}` inlined through `{slot}` is over budget: {detail}\n\
         hint: a field costs its caller once per evaluation — cut the field, the \
         evaluations, or the loop around them"
    )]
    FieldTooExpensive {
        caller: String,
        /// Slot through which the field was invoked.
        slot: String,
        field: String,
        detail: String,
    },
    /// A pairing L2 deformer is positioned non-first in the deformation chain.
    #[error(
        "`{l2}` pairs two geometries and sits at position {at} in the chain\n\
         hint: a pairing L2 reads a *source*, so it has to be the first one. Put the \
         deformations after it"
    )]
    PairingNotFirst { l2: String, at: usize },
    /// Pairing L2 requires exactly two geometry sources in the Set.
    #[error(
        "`{l2}` takes a second geometry and this Set has {sources}\n\
         hint: name exactly two L1s — the one the chain runs over and the one the slot is \
         bound to. A third would be a source no node reads"
    )]
    PairingArity { l2: String, sources: usize },
    /// Paired source uses dynamic compaction or spawning, violating slot-index correspondence.
    #[error(
        "`{l2}` pairs by slot index and `{l1}` does not keep its elements at fixed slots\n\
         hint: a paired source must have no `spawn` block and no `kill()` — either one \
         compacts, and after a compaction element 5 of one source is not element 5 of the \
         other"
    )]
    PairingNotStatic { l2: String, l1: String },
    /// Far source of a pairing L2 lacks attributes required by downstream derivations.
    #[error(
        "`{l2}` pairs with `{l1}`, and `{attr}` is derived from something `{l1}` does not \
         emit\n\
         hint: both sides of a pairing carry the same attributes — emit the source attribute \
         in both, or stop consuming `{attr}`"
    )]
    PairingDerivation {
        l2: String,
        l1: String,
        attr: String,
    },
    /// Paired sources have differing capacities.
    #[error(
        "`{l2}` pairs two geometries of {a} and {b} elements\n\
         hint: pairing is by slot index, so both sources have to be the same size — set one \
         `--capacity`, or declare the same default in both"
    )]
    PairingCapacity { l2: String, a: u32, b: u32 },
    /// A declared input slot remains unbound in the Set.
    #[error(
        "`{node}` declares `{slot} : {takes}` and nothing in this Set says what fills it\n\
         hint: bind it — `--edge {node}.{slot}=<node>`. This Set holds: {holds}"
    )]
    SlotUnbound {
        node: String,
        slot: String,
        /// Expected slot type.
        takes: &'static str,
        holds: String,
    },
    /// An edge targets an undeclared input slot on a node.
    #[error(
        "`{node}` declares no slot called `{slot}`\n\
         hint: an edge names a slot the procedure declared with `uses {slot} : \
         <Geometry|Field|Camera|Source>`{declares}"
    )]
    NoSuchSlot {
        node: String,
        slot: String,
        /// Available slot declarations.
        declares: String,
    },
    /// An edge targets a node name not present in the Set.
    #[error(
        "`{node}.{slot}` is bound to `{to}`, which is not a node of this Set\n\
         hint: this Set holds: {holds}"
    )]
    EdgeToUnknown {
        node: String,
        slot: String,
        to: String,
        holds: String,
    },
    /// An edge intended for a Geometry slot targets a non-geometry node.
    #[error(
        "`{node}.{slot}` is bound to `{to}`, which is {layer}\n\
         hint: a slot declared `: Geometry` takes an L1 — the sources in this Set are: {sources}"
    )]
    EdgeToNotGeometry {
        node: String,
        slot: String,
        to: String,
        /// Layer descriptor of the bound node.
        layer: &'static str,
        sources: String,
    },
    /// An edge intended for a Field slot targets a non-field node.
    #[error(
        "`{node}.{slot}` is bound to `{to}`, which is {layer}\n\
         hint: a slot declared `: Field` takes a `kind Field` procedure — this Set's is: {fields}"
    )]
    EdgeToNotField {
        node: String,
        slot: String,
        to: String,
        /// Layer descriptor of the bound node.
        layer: &'static str,
        /// Description of available fields.
        fields: String,
    },
    /// An edge intended for a Camera slot targets a non-camera node.
    #[error(
        "`{node}.{slot}` is bound to `{to}`, which is {layer}\n\
         hint: a slot declared `: Camera` takes an L3 or the built-in camera — \
         this Set's are: {cameras}"
    )]
    EdgeToNotCamera {
        node: String,
        slot: String,
        to: String,
        /// Layer descriptor of the bound node.
        layer: &'static str,
        /// Names of available cameras.
        cameras: String,
    },
    /// An edge intended for a Source slot targets a non-source node.
    #[error(
        "`{node}.{slot}` is bound to `{to}`, which is {layer}\n\
         hint: a slot declared `: Source` takes an L1 — it is the identity `source` is \
         compared against, and only a geometry has one. The sources in this Set are: {sources}"
    )]
    EdgeToNotSource {
        node: String,
        slot: String,
        to: String,
        /// Layer descriptor of the bound node.
        layer: &'static str,
        sources: String,
    },
    /// Multiple edges attempt to bind the same slot.
    #[error(
        "`{node}.{slot}` is bound twice, to `{first}` and to `{second}`\n\
         hint: a slot is one input — remove one of the edges"
    )]
    SlotBoundTwice {
        node: String,
        slot: String,
        first: String,
        second: String,
    },
    /// A Set contains no geometry nodes.
    #[error("a Set needs at least one L1 — there is nothing to draw")]
    NoGeometry,
    /// Requested element count exceeds hardware storage buffer limits.
    #[error(
        "`{l2}` amplifies to {elements} elements ({bytes} bytes), and this device binds \
         at most {limit}\n\
         hint: lower `--capacity`, lower `amplify` in `{l2}`, or narrow the chain's `emit` \
         — the buffer is the Set's capacity times every factor above this node, times the \
         element stride"
    )]
    TooManyElements {
        l2: String,
        elements: u64,
        bytes: u64,
        limit: u64,
    },
    /// Number of composited inputs exceeds maximum capacity of L5 merge pass.
    #[error(
        "`{l1}` composites {count} renderers and an L5 folds at most {max}\n\
         hint: drop `--merge` for this slot and they overdraw instead, which has no limit — \
         or split them across Sets, which is what a deck is"
    )]
    TooManyInputs {
        l1: String,
        count: usize,
        max: usize,
    },
    /// A Set contains no renderer nodes.
    #[error("a Set needs at least one L4 to draw `{l1}` with")]
    NoRenderer { l1: String },
    /// Weighted blending requested on a fullscreen renderer where additive yields equivalent output.
    #[error(
        "`{l4}` draws the whole frame, where `blend weighted` resolves to exactly what \
         `additive` accumulates\n\
         hint: one fragment per texel makes the weighted resolve the identity — it would \
         cost a revealage target and a resolve pass to reproduce the picture `blend \
         additive` gives for nothing. Declare `additive`"
    )]
    WeightedFullscreen { l4: String },
    /// Internal panic occurred during compilation.
    #[error("building `{label}` panicked, which is a bug in this compiler rather than in the `.kir`: {detail}")]
    Panicked { label: String, detail: String },
}

/// Clock source used to sample oscillator signals.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Clock {
    /// Session master clock, used when on air.
    Session,
    /// Local Set simulation clock, used during pre-warming off air.
    Local,
}

/// Geometry source and associated deformation and rendering pipeline instances.
pub(crate) struct Source {
    /// Unique hash salt for this source instance.
    salt: u32,
    /// Indices into the Set's L1 procedure list.
    procedures: Vec<usize>,
    /// Primary L1 simulation node instance.
    sim: Simulation,
    /// Optional secondary simulation node for paired geometry deformations.
    paired: Option<Simulation>,
    /// Sequential L2 deformation nodes instantiated for this source.
    deforms: Vec<Deform>,
    /// L4 renderer nodes instantiated for this source.
    renderers: Vec<Renderer>,
}

pub struct Set {
    seed_salt: u32,
    /// Simulation steps elapsed. Time is `steps_taken * dt`.
    steps_taken: u64,
    /// Uncommitted simulation steps staged for the current frame.
    staged_delta: u64,
    staged_edges: Option<Vec<Input>>,
    dt: f32,
    /// Most recent beat count written to the L4 uniform block.
    last_beats: f32,
    viewport: [f32; 2],
    /// Indicates whether all procedures in the Set are closed-form functions of seed, t, and parameters.
    closed_form: bool,
    /// Indicates whether any procedure in the Set reads the ambient beat count.
    reads_beats: bool,

    /// Geometry sources and their associated pipeline chains.
    sources: Vec<Source>,
    /// Hash salts assigned to each geometry source, in L1 procedure order.
    source_salts: Vec<u32>,
    /// Declared capacity ranges `[min, max, default]` for each geometry source.
    declared_capacities: Vec<[u32; 3]>,
    /// Canonical names for each node in node order.
    names: Vec<String>,
    /// Authority level for each node in node order.
    authorities: Vec<Authority>,
    /// Number of distinct L1 procedures compiled into the Set.
    l1_count: usize,

    /// Manual parameter values per node.
    params: Vec<HashMap<String, f32>>,
    /// Typed parameter values keyed by canonical declaration name.
    param_values: Vec<HashMap<String, karakuri_store::record::Value>>,
    /// Parameter keys whose values have been explicitly modified from defaults.
    moved: Vec<HashSet<String>>,
    /// Declared `[min, max]` ranges for each parameter per node.
    ranges: Vec<HashMap<String, [f32; 2]>>,
    /// Bound on minimum primitive size for each L4 renderer procedure.
    rate_bounds: Vec<karakuri_ir::rate::RateBound>,
    /// Configured orbit camera parameters for the built-in camera.
    camera: Orbit,
    /// GPU camera nodes, ending with the built-in orbit camera.
    cameras: Vec<crate::node::Camera>,
    /// Optional L5 merge compositor.
    merge: Option<crate::node::Merge>,
    /// Composite input configurations for each renderer in draw order.
    edges: Vec<Input>,
    /// Active parameter signal bindings.
    bindings: Vec<Binding>,
    /// Controls published to the console interface.
    interface: Vec<Published>,
    /// Spliced field parameters under semantic names.
    field_params: Vec<String>,
    /// Spliced field parameter component names per field.
    field_declared: Vec<Vec<String>>,
    /// Bound Field slots: `(node_index, slot_name, field_ordinal)`.
    field_bound: Vec<(usize, String, usize)>,
    /// Bound Source slots: `(node_index, slot_name, geometry_ordinal)`.
    source_bound: Vec<(usize, String, usize)>,
    /// Number of distinct Field procedures in the Set.
    field_count: usize,
}

/// Control published to the external console interface.
#[derive(Debug, Clone, PartialEq)]
pub struct Published {
    /// Display label shown on the console.
    pub name: String,
    /// Target node layer and index, or `None` to target all declaring nodes.
    pub at: Option<(Kind, u32)>,
    /// Internal parameter identifier.
    pub key: String,
    /// Active range for the control, constrained to a subset of declared range.
    pub range: [f32; 2],
}

/// Errors occurring when defining a published interface control.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PublishError {
    #[error("nothing in this Set declares `{key}`{at}")]
    NoSuchControl { key: String, at: String },
    #[error(
        "`{name}` publishes `{key}` over [{low}, {high}], which is outside the [{min}, {max}] \
         the procedure declares\n\
         hint: a published range narrows and never redefines — the declared range is the \
         procedure's statement about where it still looks like itself"
    )]
    RangeNotASubset {
        name: String,
        key: String,
        low: f32,
        high: f32,
        min: f32,
        max: f32,
    },
    #[error("`{0}` is published twice; a console shows one control per name")]
    DuplicateName(String),
}

/// Refusal resulting from a wildcard parameter write targeting nodes under conflicting authorities.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error(
    "`{key}` is declared by nodes that are not under one authority — {landing} — and a bare \
     name writes every node that declares it\n\
     hint: an authority is per node, so this write would cross a grant. Address the node \
     it is meant for, or put the nodes it lands on under one authority"
)]
pub struct CrossesAuthority {
    /// Parameter key that was written.
    pub key: String,
    /// Description of target nodes and their conflicting authority levels.
    pub landing: String,
}

impl CrossesAuthority {
    /// Evaluates whether a wildcard write for `key` across `landing` crosses authority boundaries.
    fn over(key: &str, landing: &[(Kind, u32, Authority)]) -> Option<CrossesAuthority> {
        let first = landing.first()?.2;
        if landing.iter().all(|(_, _, held)| *held == first) {
            return None;
        }
        let named: Vec<String> = landing
            .iter()
            .map(|(layer, index, held)| format!("{layer:?}:{index} {}", held.name()))
            .collect();
        Some(CrossesAuthority {
            key: key.to_string(),
            landing: named.join(", "),
        })
    }
}

/// Connection binding an input slot on a node to another node in the Set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    /// Declaring node name.
    pub node: String,
    /// Slot name on the declaring node.
    pub slot: InputPort,
    /// Name of the node connected to the slot.
    pub to: String,
}

/// Specification of node names and slot wirings for building a Set.
#[derive(Debug, Clone, Copy, Default)]
pub struct Wiring<'a> {
    pub l1s: &'a [Option<String>],
    pub l2s: &'a [Option<String>],
    /// Optional names for L3 cameras, including the built-in camera.
    pub l3s: &'a [Option<String>],
    pub l4s: &'a [Option<String>],
    /// Optional names for Field procedures.
    pub fields: &'a [Option<String>],
    /// Slot bindings applicable to this Set.
    pub edges: &'a [Edge],
}

/// Returns the first duplicate string in the slice, if any.
fn first_duplicate(names: &[String]) -> Option<String> {
    names
        .iter()
        .enumerate()
        .find(|(at, name)| names[..*at].contains(name))
        .map(|(_, name)| name.clone())
}

/// Result of attempting to bind a parameter to a signal source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// Successfully bound.
    Yes,
    /// Parameter key does not exist on target node.
    NoSuchParam,
    /// Referenced control is not published by the interface.
    NoSuchControl,
}

impl Bound {
    /// Returns true if the binding was successfully attached.
    pub fn attached(self) -> bool {
        self == Bound::Yes
    }
}

/// Pre-compilation execution and memory plan computed during Set validation.
pub struct Plan<'a> {
    /// Node names in execution order.
    names: Vec<String>,
    /// Camera procedures, with `None` representing the default built-in camera.
    cameras: Vec<Option<&'a Checked>>,
    /// Range in `names` occupied by camera nodes.
    camera_range: std::ops::Range<usize>,
    /// Bound Field slots: `(node_index, slot_name, field_ordinal)`.
    field_bound: Vec<(usize, String, usize)>,
    /// Bound Camera slots: `(node_index, camera_ordinal)`.
    camera_bound: Vec<(usize, usize)>,
    /// Bound Source slots: `(node_index, slot_name, l1_index)`.
    source_bound: Vec<(usize, String, usize)>,
    /// Index into `l1s` of secondary geometry in a pairing configuration.
    far_at: Option<usize>,
    /// Indices of primary geometry chain heads.
    heads: Vec<usize>,
    /// Hash salts assigned to each geometry.
    source_salts: Vec<u32>,
    /// Synthesized attributes required for each chain head.
    derived: Vec<Vec<karakuri_ir::Attr>>,
    /// L1 procedures and requested element capacities.
    l1s: Vec<(&'a Checked, u32)>,
    /// L2 deformation procedures in chain order.
    l2s: Vec<&'a Checked>,
    /// Field procedures available for splicing.
    fields: Vec<&'a Checked>,
}

/// Projected element storage allocation for a specific node instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedStorage {
    /// Index into [`Plan::node_names`] corresponding to the node.
    pub node: usize,
    /// Allocated element memory and capacity.
    pub storage: ElementStorage,
}

impl<'a> Plan<'a> {
    /// Calculates projected element buffer allocations without compiling GPU pipelines.
    pub fn element_storage(&self) -> Vec<PlannedStorage> {
        let bound_at = |at: usize| -> Vec<(&str, &Checked)> {
            self.field_bound
                .iter()
                .filter(|(node, _, _)| *node == at)
                .map(|(_, slot, ordinal)| (slot.as_str(), self.fields[*ordinal]))
                .collect()
        };
        let simulation = |at: usize, derived: &[karakuri_ir::Attr]| {
            let (l1, capacity) = self.l1s[at];
            let shader = generate_l1(l1, derived, &bound_at(at));
            PlannedStorage {
                node: at,
                storage: SimulationStorage::of(
                    capacity,
                    shader.element_layout.stride,
                    shader.compacted,
                )
                .total(),
            }
        };

        let mut out = Vec::new();
        for (head, &at) in self.heads.iter().enumerate() {
            let (l1, capacity) = self.l1s[at];
            let derived = &self.derived[head];
            out.push(simulation(at, derived));
            if let Some(far_at) = self.far_at {
                out.push(simulation(far_at, derived));
            }

            let mut upstream: Vec<karakuri_ir::Attr> = l1.emit.clone();
            let mut synthetic = Synthetic::NONE;
            let mut chain_capacity = capacity;
            for (k, l2) in self.l2s.iter().enumerate() {
                let far = self
                    .far_at
                    .filter(|_| l2.geometry_slot().is_some())
                    .map(|far_at| self.l1s[far_at].0.emit.as_slice());
                let shader = generate_l2(
                    l2,
                    &upstream,
                    synthetic,
                    derived,
                    far,
                    &bound_at(self.l1s.len() + k),
                );
                chain_capacity = chain_capacity.saturating_mul(shader.amplify.unwrap_or(1));
                out.push(PlannedStorage {
                    node: self.l1s.len() + k,
                    storage: DeformStorage::of(
                        chain_capacity,
                        shader.element_layout.stride,
                        shader.amplify.is_some(),
                    )
                    .total(),
                });
                upstream = shader.emits;
                synthetic = shader.synthetic;
            }
        }
        out
    }

    /// Returns canonical node names in evaluation order.
    pub fn node_names(&self) -> &[String] {
        &self.names
    }
}

/// Validates requested capacity against declared range in the L1 procedure.
fn capacity_in_range(l1: &Checked, capacity: u32) -> Result<(), SetError> {
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
    Ok(())
}

impl Set {
    /// Compiles checked procedures into a runnable Set.
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1: &Checked,
        l4: &Checked,
        capacity: u32,
        seed_salt: u32,
    ) -> Result<Set, SetError> {
        Set::build_many(
            device,
            queue,
            &[(l1, capacity)],
            &[],
            &[],
            &[],
            &[l4],
            Layering::Overdraw,
            seed_salt,
            &[],
            Wiring::default(),
        )
    }

    /// Compiles multiple geometry sources, deformers, cameras, fields, and renderers into a runnable Set.
    ///
    /// Execution runs within a wgpu validation error scope to report driver-level errors gracefully.
    #[allow(clippy::too_many_arguments)]
    pub fn build_many(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1s: &[(&Checked, u32)],
        l2s: &[&Checked],
        l3s: &[&Checked],
        fields: &[&Checked],
        l4s: &[&Checked],
        layering: Layering,
        seed_salt: u32,
        salts: &[Option<u32>],
        wiring: Wiring<'_>,
    ) -> Result<Set, SetError> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let built = Set::build_inner(
            device, queue, l1s, l2s, l3s, fields, l4s, layering, seed_salt, salts, wiring,
        );
        let captured = pollster::block_on(scope.pop());

        match (built, captured) {
            (Err(e), _) => Err(e),
            (Ok(_), Some(e)) => Err(SetError::Invalid {
                proc: l1s
                    .first()
                    .map_or_else(String::new, |(p, _)| p.name.clone()),
                detail: e.to_string(),
            }),
            (Ok(set), None) => Ok(set),
        }
    }

    /// Validates procedure compatibility, topology, and slot bindings without requiring a GPU device.
    ///
    /// Returns a [`Plan`] containing resolved node names, bindings, and memory allocations.
    #[allow(clippy::too_many_arguments)]
    pub fn validate<'a>(
        l1s: &[(&'a Checked, u32)],
        l2s: &[&'a Checked],
        l3s: &[&'a Checked],
        fields: &[&'a Checked],
        l4s: &[&'a Checked],
        layering: Layering,
        seed_salt: u32,
        salts: &[Option<u32>],
        wiring: Wiring<'_>,
    ) -> Result<Plan<'a>, SetError> {
        let Some(&(first_l1, _)) = l1s.first() else {
            return Err(SetError::NoGeometry);
        };
        if l4s.is_empty() {
            return Err(SetError::NoRenderer {
                l1: first_l1.name.clone(),
            });
        }
        if layering == Layering::Composite && l4s.len() > crate::deck::MAX_SLOTS {
            return Err(SetError::TooManyInputs {
                l1: first_l1.name.clone(),
                count: l4s.len(),
                max: crate::deck::MAX_SLOTS,
            });
        }
        // Names are resolved first to provide concrete addresses for edge binding.
        let given = |at: usize, from: &[Option<String>]| from.get(at).cloned().flatten();
        // Camera nodes: one per L3 procedure, ending with the built-in orbit camera.
        let cameras: Vec<Option<&Checked>> = l3s
            .iter()
            .copied()
            .map(Some)
            .chain(std::iter::once(None))
            .collect();
        let wanted: Vec<(Option<String>, Option<&Checked>)> = l1s
            .iter()
            .enumerate()
            .map(|(at, (l1, _))| (given(at, wiring.l1s), Some(*l1)))
            .chain(
                l2s.iter()
                    .enumerate()
                    .map(|(at, n)| (given(at, wiring.l2s), Some(*n))),
            )
            .chain(
                cameras
                    .iter()
                    .enumerate()
                    .map(|(at, n)| (given(at, wiring.l3s), *n)),
            )
            .chain(
                l4s.iter()
                    .enumerate()
                    .map(|(at, n)| (given(at, wiring.l4s), Some(*n))),
            )
            .chain(
                fields
                    .iter()
                    .enumerate()
                    .map(|(at, n)| (given(at, wiring.fields), Some(*n))),
            )
            .collect();
        let nodes: Vec<Option<&Checked>> = wanted.iter().map(|(_, n)| *n).collect();
        let mut taken: Vec<String> = wanted.iter().filter_map(|(n, _)| n.clone()).collect();
        if let Some(dup) = first_duplicate(&taken) {
            return Err(SetError::DuplicateNodeName { name: dup });
        }
        let names: Vec<String> = wanted
            .into_iter()
            .map(|(name, node)| match name {
                Some(written) => written,
                None => {
                    let mut candidate =
                        node.map_or(BUILTIN_CAMERA, |n| n.name.as_str()).to_string();
                    let mut at = 1;
                    while taken.contains(&candidate) {
                        at += 1;
                        candidate =
                            format!("{}-{at}", node.map_or(BUILTIN_CAMERA, |n| n.name.as_str()));
                    }
                    taken.push(candidate.clone());
                    candidate
                }
            })
            .collect();

        // Node index resolution helpers.
        let node_at = |name: &str| names.iter().position(|n| n == name);
        let geometry_at = |name: &str| node_at(name).filter(|at| *at < l1s.len());
        let holds = || names.join(", ");
        let sources = || names[..l1s.len()].join(", ");
        let field_range = names.len() - fields.len()..names.len();
        let camera_range = l1s.len() + l2s.len()..l1s.len() + l2s.len() + cameras.len();
        let camera_ordinal =
            |at: usize| camera_range.contains(&at).then(|| at - camera_range.start);
        let holds_cameras = || names[camera_range.clone()].join(", ");
        let field_ordinal = |at: usize| field_range.contains(&at).then(|| at - field_range.start);
        let holds_fields = || match field_range.is_empty() {
            true => "none — this Set holds no `kind Field` procedure".to_string(),
            false => names[field_range.clone()].join(", "),
        };
        let layer_of = |at: usize| -> &'static str {
            if at < l1s.len() {
                "an L1"
            } else if at < l1s.len() + l2s.len() {
                "an L2"
            } else if camera_range.contains(&at) {
                "a camera"
            } else if field_range.contains(&at) {
                "a field"
            } else {
                "an L4"
            }
        };

        // Validate that all edges targeting nodes in this Set connect to declared input slots.
        for edge in wiring.edges {
            let Some(at) = node_at(&edge.node) else {
                continue;
            };
            let declared: &[karakuri_ir::typed::Slot] =
                nodes[at].map_or(&[], |n| n.uses.as_slice());
            if !declared.iter().any(|slot| slot.name == edge.slot) {
                return Err(SetError::NoSuchSlot {
                    node: edge.node.clone(),
                    slot: edge.slot.to_string(),
                    declares: match declared.is_empty() {
                        true => String::new(),
                        false => format!(
                            "; `{}` declares {}",
                            edge.node,
                            declared
                                .iter()
                                .map(|s| format!("`{}`", s.name))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    },
                });
            }
        }

        // Validate pairing geometry slot and far geometry binding.
        let pairing = l2s.iter().position(|n| n.geometry_slot().is_some());
        let far_at: Option<usize> = match pairing {
            None => None,
            Some(at) => {
                let l2 = l2s[at];
                let node = names[l1s.len() + at].clone();
                let slot = l2
                    .geometry_slot()
                    .expect("`pairing` is the position of a node that declares a slot")
                    .to_string();
                if at != 0 {
                    return Err(SetError::PairingNotFirst {
                        l2: l2.name.clone(),
                        at,
                    });
                }
                if l1s.len() != 2 {
                    return Err(SetError::PairingArity {
                        l2: l2.name.clone(),
                        sources: l1s.len(),
                    });
                }
                let mut bound = wiring
                    .edges
                    .iter()
                    .filter(|e| e.node == node && e.slot.as_str() == slot);
                let Some(edge) = bound.next() else {
                    return Err(SetError::SlotUnbound {
                        node,
                        slot,
                        takes: karakuri_ir::SlotTy::Geometry.name(),
                        holds: holds(),
                    });
                };
                if let Some(second) = bound.next() {
                    return Err(SetError::SlotBoundTwice {
                        node,
                        slot,
                        first: edge.to.clone(),
                        second: second.to.clone(),
                    });
                }
                let Some(far_at) = geometry_at(&edge.to) else {
                    return Err(match node_at(&edge.to) {
                        Some(other) => SetError::EdgeToNotGeometry {
                            node,
                            slot,
                            to: edge.to.clone(),
                            layer: match other < l1s.len() + l2s.len() {
                                true => "an L2",
                                false => "a node that holds no elements",
                            },
                            sources: sources(),
                        },
                        None => SetError::EdgeToUnknown {
                            node,
                            slot,
                            to: edge.to.clone(),
                            holds: holds(),
                        },
                    });
                };
                for (l1, _) in l1s {
                    if !l1.is_static() {
                        return Err(SetError::PairingNotStatic {
                            l2: l2.name.clone(),
                            l1: l1.name.clone(),
                        });
                    }
                }
                let near_at = (0..l1s.len())
                    .find(|at| *at != far_at)
                    .expect("two sources, one of them bound");
                if l1s[near_at].1 != l1s[far_at].1 {
                    return Err(SetError::PairingCapacity {
                        l2: l2.name.clone(),
                        a: l1s[near_at].1,
                        b: l1s[far_at].1,
                    });
                }
                Some(far_at)
            }
        };

        // Validate and record bindings for all Field slots across nodes.
        let mut field_bound: Vec<(usize, String, usize)> = Vec::new();
        for (at, node) in nodes.iter().enumerate() {
            let Some(node) = node else { continue };
            for slot in node
                .uses
                .iter()
                .filter(|s| s.ty == karakuri_ir::SlotTy::Field)
            {
                let node_name = names[at].clone();
                let mut bound = wiring
                    .edges
                    .iter()
                    .filter(|e| e.node == node_name && e.slot == slot.name);
                let Some(edge) = bound.next() else {
                    return Err(SetError::SlotUnbound {
                        node: node_name,
                        slot: slot.name.to_string(),
                        takes: slot.ty.name(),
                        holds: holds(),
                    });
                };
                if let Some(second) = bound.next() {
                    return Err(SetError::SlotBoundTwice {
                        node: node_name,
                        slot: slot.name.to_string(),
                        first: edge.to.clone(),
                        second: second.to.clone(),
                    });
                }
                match node_at(&edge.to).map(|to| (to, field_ordinal(to))) {
                    Some((_, Some(ordinal))) => {
                        field_bound.push((at, slot.name.to_string(), ordinal));
                    }
                    Some((other, None)) => {
                        return Err(SetError::EdgeToNotField {
                            node: node_name,
                            slot: slot.name.to_string(),
                            to: edge.to.clone(),
                            layer: layer_of(other),
                            fields: holds_fields(),
                        })
                    }
                    None => {
                        return Err(SetError::EdgeToUnknown {
                            node: node_name,
                            slot: slot.name.to_string(),
                            to: edge.to.clone(),
                            holds: holds(),
                        })
                    }
                }
            }
        }

        // Validate and record bindings for all Camera slots across nodes.
        let mut camera_bound: Vec<(usize, usize)> = Vec::new();
        for (at, node) in nodes.iter().enumerate() {
            let Some(node) = node else { continue };
            for slot in node
                .uses
                .iter()
                .filter(|s| s.ty == karakuri_ir::SlotTy::Camera)
            {
                let node_name = names[at].clone();
                let mut bound = wiring
                    .edges
                    .iter()
                    .filter(|e| e.node == node_name && e.slot == slot.name);
                let Some(edge) = bound.next() else {
                    return Err(SetError::SlotUnbound {
                        node: node_name,
                        slot: slot.name.to_string(),
                        takes: slot.ty.name(),
                        holds: holds(),
                    });
                };
                if let Some(second) = bound.next() {
                    return Err(SetError::SlotBoundTwice {
                        node: node_name,
                        slot: slot.name.to_string(),
                        first: edge.to.clone(),
                        second: second.to.clone(),
                    });
                }
                match node_at(&edge.to).map(|to| (to, camera_ordinal(to))) {
                    Some((_, Some(ordinal))) => camera_bound.push((at, ordinal)),
                    Some((other, None)) => {
                        return Err(SetError::EdgeToNotCamera {
                            node: node_name,
                            slot: slot.name.to_string(),
                            to: edge.to.clone(),
                            layer: layer_of(other),
                            cameras: holds_cameras(),
                        })
                    }
                    None => {
                        return Err(SetError::EdgeToUnknown {
                            node: node_name,
                            slot: slot.name.to_string(),
                            to: edge.to.clone(),
                            holds: holds(),
                        })
                    }
                }
            }
        }

        // Validate and record bindings for all Source slots across nodes.
        let mut source_bound: Vec<(usize, String, usize)> = Vec::new();
        for (at, node) in nodes.iter().enumerate() {
            let Some(node) = node else { continue };
            for slot in node
                .uses
                .iter()
                .filter(|s| s.ty == karakuri_ir::SlotTy::Source)
            {
                let node_name = names[at].clone();
                let mut bound = wiring
                    .edges
                    .iter()
                    .filter(|e| e.node == node_name && e.slot == slot.name);
                let Some(edge) = bound.next() else {
                    return Err(SetError::SlotUnbound {
                        node: node_name,
                        slot: slot.name.to_string(),
                        takes: slot.ty.name(),
                        holds: holds(),
                    });
                };
                if let Some(second) = bound.next() {
                    return Err(SetError::SlotBoundTwice {
                        node: node_name,
                        slot: slot.name.to_string(),
                        first: edge.to.clone(),
                        second: second.to.clone(),
                    });
                }
                match node_at(&edge.to) {
                    Some(to) => match geometry_at(&edge.to) {
                        Some(l1_at) => source_bound.push((at, slot.name.to_string(), l1_at)),
                        None => {
                            return Err(SetError::EdgeToNotSource {
                                node: node_name,
                                slot: slot.name.to_string(),
                                to: edge.to.clone(),
                                layer: layer_of(to),
                                sources: sources(),
                            })
                        }
                    },
                    None => {
                        return Err(SetError::EdgeToUnknown {
                            node: node_name,
                            slot: slot.name.to_string(),
                            to: edge.to.clone(),
                            holds: holds(),
                        })
                    }
                }
            }
        }
        for (l1, _) in l1s {
            if l1.kind != Kind::L1 {
                return Err(SetError::WrongKind {
                    slot: "L1",
                    expected: Kind::L1,
                    actual: l1.kind,
                });
            }
        }
        for f in fields {
            if f.kind != Kind::Field {
                return Err(SetError::WrongKind {
                    slot: "Field",
                    expected: Kind::Field,
                    actual: f.kind,
                });
            }
        }
        if !fields.is_empty() {
            let per_evaluation: Vec<u64> = fields
                .iter()
                .map(|f| {
                    karakuri_ir::cost::estimate(f)
                        .map(|c| c.ops_per_evaluation)
                        .unwrap_or(0)
                })
                .collect();
            for (at, caller) in nodes
                .iter()
                .enumerate()
                .filter(|(at, _)| !field_range.contains(at))
                .filter_map(|(at, n)| n.map(|n| (at, n)))
            {
                let per_slot = |slot: &str| {
                    field_bound
                        .iter()
                        .find(|(node, name, _)| *node == at && name == slot)
                        .map(|(_, _, ordinal)| per_evaluation[*ordinal])
                        .unwrap_or(0)
                };
                if let Err(over) = karakuri_ir::cost::check_with_field(caller, &per_slot) {
                    let blamed = field_bound
                        .iter()
                        .find(|(node, name, _)| *node == at && *name == over.slot)
                        .map(|(_, _, ordinal)| fields[*ordinal].name.clone())
                        .unwrap_or_default();
                    return Err(SetError::FieldTooExpensive {
                        caller: caller.name.clone(),
                        slot: over.slot,
                        field: blamed,
                        detail: over
                            .errors
                            .first()
                            .map(|e| e.message.clone())
                            .unwrap_or_default(),
                    });
                }
            }
        }

        for l3 in l3s {
            if l3.kind != Kind::L3 {
                return Err(SetError::WrongKind {
                    slot: "L3",
                    expected: Kind::L3,
                    actual: l3.kind,
                });
            }
        }
        let heads: Vec<usize> = (0..l1s.len()).filter(|at| Some(*at) != far_at).collect();
        let salt_of = |at: usize| -> u32 {
            salts
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| derived_salt(seed_salt, at))
        };
        let source_salts: Vec<u32> = (0..l1s.len()).map(salt_of).collect();
        let mut derived_per_head: Vec<Vec<karakuri_ir::Attr>> = Vec::with_capacity(heads.len());
        for &at in &heads {
            let (l1, capacity) = l1s[at];
            let emitted: Vec<karakuri_ir::Attr> = l1
                .emit
                .iter()
                .chain(l2s.iter().flat_map(|n| n.emit.iter()))
                .copied()
                .collect();
            let mut derived: Vec<karakuri_ir::Attr> = Vec::new();
            let mut blocked: Vec<(karakuri_ir::Attr, karakuri_ir::Attr)> = Vec::new();
            {
                let mut seen: Vec<karakuri_ir::Attr> = l1.emit.clone();
                for node in std::iter::once(&l1).chain(l2s.iter()).chain(l4s.iter()) {
                    for &attr in &node.consumes {
                        if seen.contains(&attr)
                            || derived.contains(&attr)
                            || emitted.contains(&attr)
                        {
                            continue;
                        }
                        let Some(rule) = attr.derivation() else {
                            continue;
                        };
                        if let Some(from) = rule.source().filter(|from| !l1.emit.contains(from)) {
                            blocked.push((attr, from));
                            continue;
                        }
                        derived.push(attr);
                    }
                    for &attr in &node.emit {
                        if !seen.contains(&attr) {
                            seen.push(attr);
                        }
                    }
                }
            }

            let mut available: Vec<karakuri_ir::Attr> = l1.emit.clone();
            available.extend(derived.iter().copied());
            let check_against = |node: &Checked, available: &[karakuri_ir::Attr]| {
                let missing: Vec<String> = node
                    .consumes
                    .iter()
                    .filter(|a| !available.contains(a))
                    .map(|a| format!("`{}`", a.name()))
                    .collect();
                if missing.is_empty() {
                    return None;
                }
                let hint = node
                    .consumes
                    .iter()
                    .find_map(|a| blocked.iter().find(|(attr, _)| attr == a))
                    .map(|(attr, from)| {
                        format!(
                            "`{}` is synthesised from `{}`, and `{}` emits neither. Add `{}` to \
                             `{}`'s `emit` and `{}` follows",
                            attr.name(),
                            from.name(),
                            l1.name,
                            from.name(),
                            l1.name,
                            attr.name()
                        )
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "add {} to `{}`'s `emit`, or pair `{}` with an L1 that emits it. \
                             `age` and `velocity` are synthesised where nothing emits them; nothing \
                             else is",
                            missing.join(", "),
                            l1.name,
                            node.name
                        )
                    });
                Some(SetError::Composition {
                    l1: l1.name.clone(),
                    l4: node.name.clone(),
                    missing: missing.join(", "),
                    hint,
                })
            };
            for l2 in l2s {
                if l2.kind != Kind::L2 {
                    return Err(SetError::WrongKind {
                        slot: "L2",
                        expected: Kind::L2,
                        actual: l2.kind,
                    });
                }
                if let Some(e) = check_against(l2, &available) {
                    return Err(e);
                }
                for &attr in &l2.emit {
                    if !available.contains(&attr) {
                        available.push(attr);
                    }
                }
            }
            for l4 in l4s {
                if l4.kind != Kind::L4 {
                    return Err(SetError::WrongKind {
                        slot: "L4",
                        expected: Kind::L4,
                        actual: l4.kind,
                    });
                }
                if let Some(e) = check_against(l4, &available) {
                    return Err(e);
                }
            }

            if let [only] = l4s {
                if only.blend == Some(karakuri_ir::Blend::Weighted)
                    && only.topology == Some(karakuri_ir::Topology::Fullscreen)
                {
                    return Err(SetError::WeightedFullscreen {
                        l4: only.name.clone(),
                    });
                }
            }

            capacity_in_range(l1, capacity)?;
            if let Some(far_at) = far_at {
                let (far, far_capacity) = l1s[far_at];
                for attr in &derived {
                    if attr
                        .derivation()
                        .and_then(|d| d.source())
                        .is_some_and(|from| !far.emit.contains(&from))
                    {
                        return Err(SetError::PairingDerivation {
                            l2: l2s
                                [pairing.expect("a bound geometry is a node that declared a slot")]
                            .name
                            .clone(),
                            l1: far.name.clone(),
                            attr: attr.name().to_string(),
                        });
                    }
                }
                capacity_in_range(far, far_capacity)?;
            }
            derived_per_head.push(derived);
        }

        Ok(Plan {
            names,
            cameras,
            camera_range,
            field_bound,
            camera_bound,
            source_bound,
            far_at,
            heads,
            source_salts,
            derived: derived_per_head,
            l1s: l1s.to_vec(),
            l2s: l2s.to_vec(),
            fields: fields.to_vec(),
        })
    }

    /// Compiles and instantiates GPU pipelines and buffers according to the validated plan.
    #[allow(clippy::too_many_arguments)]
    fn build_inner(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1s: &[(&Checked, u32)],
        l2s: &[&Checked],
        l3s: &[&Checked],
        fields: &[&Checked],
        l4s: &[&Checked],
        layering: Layering,
        seed_salt: u32,
        salts: &[Option<u32>],
        wiring: Wiring<'_>,
    ) -> Result<Set, SetError> {
        let Plan {
            names,
            cameras,
            camera_range,
            field_bound,
            camera_bound,
            source_bound,
            far_at,
            heads,
            source_salts,
            derived: derived_per_head,
            l1s: _,
            l2s: _,
            fields: _,
        } = Set::validate(
            l1s, l2s, l3s, fields, l4s, layering, seed_salt, salts, wiring,
        )?;
        let bound_at = |at: usize| -> Vec<(&str, &Checked)> {
            field_bound
                .iter()
                .filter(|(node, _, _)| *node == at)
                .map(|(_, slot, ordinal)| (slot.as_str(), fields[*ordinal]))
                .collect()
        };

        let camera_nodes: Vec<crate::node::Camera> = cameras
            .iter()
            .enumerate()
            .map(|(k, l3)| {
                crate::node::Camera::build(device, *l3, &bound_at(camera_range.start + k))
            })
            .collect();

        let renderer_count = l4s.len();
        let mut sources: Vec<Source> = Vec::with_capacity(heads.len());
        for (head, &at) in heads.iter().enumerate() {
            let (l1, capacity) = l1s[at];
            let salt = source_salts[at];
            let derived = &derived_per_head[head];

            let sim = Simulation::build(device, l1, capacity, salt, derived, &bound_at(at));

            let paired: Option<(Vec<karakuri_ir::Attr>, Simulation)> = match far_at {
                None => None,
                Some(far_at) => {
                    let (far, far_capacity) = l1s[far_at];
                    let far_salt = source_salts[far_at];
                    Some((
                        far.emit.clone(),
                        Simulation::build(
                            device,
                            far,
                            far_capacity,
                            far_salt,
                            derived,
                            &bound_at(far_at),
                        ),
                    ))
                }
            };

            let mut deforms: Vec<Deform> = Vec::new();
            let mut upstream: Vec<karakuri_ir::Attr> = l1.emit.clone();
            let mut synthetic = karakuri_ir::layout::Synthetic::NONE;
            let mut chain_capacity = capacity;
            let mut live: Option<usize> = None;
            for (k, l2) in l2s.iter().enumerate() {
                let node = {
                    let from = sim.geometry();
                    let (alive, counts) = match live {
                        None => (from.alive, from.counts),
                        Some(k) => {
                            let g = deforms[k].geometry(from.alive, from.counts);
                            (g.alive, g.counts)
                        }
                    };
                    let input = match deforms.last() {
                        None => sim.geometry(),
                        Some(prev) => prev.geometry(alive, counts),
                    };
                    let paired = paired.as_ref().filter(|_| l2.geometry_slot().is_some());
                    let far = paired.map(|(emits, sim): &(Vec<karakuri_ir::Attr>, Simulation)| {
                        (emits.as_slice(), sim.geometry())
                    });
                    Deform::build(
                        device,
                        l2,
                        &upstream,
                        synthetic,
                        derived,
                        far.as_ref().map(|(a, g)| (*a, g)),
                        &bound_at(l1s.len() + k),
                        &input,
                        chain_capacity,
                    )?
                };
                upstream = node.emits().to_vec();
                synthetic = node.synthetic();
                chain_capacity = chain_capacity.saturating_mul(node.amplify());
                if node.amplifies() {
                    live = Some(deforms.len());
                }
                deforms.push(node);
            }

            let renderers: Vec<Renderer> = {
                let from = sim.geometry();
                let (alive, counts) = match live {
                    None => (from.alive, from.counts),
                    Some(k) => {
                        let g = deforms[k].geometry(from.alive, from.counts);
                        (g.alive, g.counts)
                    }
                };
                let geometry = match deforms.last() {
                    None => sim.geometry(),
                    Some(last) => last.geometry(alive, counts),
                };
                let first_l4 = camera_range.end;
                l4s.iter()
                    .enumerate()
                    .map(|(k, l4)| {
                        let at = first_l4 + k;
                        let camera = camera_bound
                            .iter()
                            .find(|(node, _)| *node == at)
                            .map_or(0, |(_, ordinal)| *ordinal);
                        Renderer::build(device, l4, &geometry, &camera_nodes[camera], &bound_at(at))
                    })
                    .collect()
            };
            sources.push(Source {
                salt,
                procedures: std::iter::once(at).chain(far_at).collect(),
                sim,
                paired: paired.map(|(_, s)| s),
                deforms,
                renderers,
            });
        }

        let params = l1s
            .iter()
            .map(|(l1, _)| declared_defaults(l1))
            .chain(l2s.iter().map(|n| declared_defaults(n)))
            .chain(cameras.iter().map(|n| match n {
                Some(n) => declared_defaults(n),
                None => Orbit::default().placement_values().into_iter().collect(),
            }))
            .chain(l4s.iter().map(|n| declared_defaults(n)))
            .chain(fields.iter().map(|n| declared_defaults(n)))
            .collect();
        let param_values = l1s
            .iter()
            .map(|(l1, _)| declared_default_values(l1))
            .chain(l2s.iter().map(|n| declared_default_values(n)))
            .chain(cameras.iter().map(|n| {
                match n {
                    Some(n) => declared_default_values(n),
                    None => Orbit::default()
                        .placement_values()
                        .into_iter()
                        .map(|(k, v)| (k, karakuri_store::record::Value::Scalar(v)))
                        .collect(),
                }
            }))
            .chain(l4s.iter().map(|n| declared_default_values(n)))
            .chain(fields.iter().map(|n| declared_default_values(n)))
            .collect();
        let ranges = l1s
            .iter()
            .map(|(l1, _)| declared_ranges(l1))
            .chain(l2s.iter().map(|n| declared_ranges(n)))
            .chain(cameras.iter().map(|n| match n {
                Some(n) => declared_ranges(n),
                None => Orbit::placement_ranges().into_iter().collect(),
            }))
            .chain(l4s.iter().map(|n| declared_ranges(n)))
            .chain(fields.iter().map(|n| declared_ranges(n)))
            .collect();
        let authorities = vec![Authority::default(); names.len()];
        let moved = vec![HashSet::new(); names.len()];
        let declared_capacities = l1s
            .iter()
            .map(|(l1, at)| {
                l1.capacity
                    .map_or([*at, *at, *at], |decl| [decl.min, decl.max, decl.default])
            })
            .collect();
        let set = Set {
            names,
            authorities,
            seed_salt,
            source_salts,
            declared_capacities,
            steps_taken: 0,
            staged_delta: 0,
            staged_edges: None,
            dt: DT,
            last_beats: 0.0,
            viewport: [1.0, 1.0],
            closed_form: l1s.iter().all(|(n, _)| n.closed_form)
                && l2s.iter().all(|n| n.closed_form)
                && l3s.iter().all(|n| n.closed_form)
                && l4s.iter().all(|n| n.closed_form),
            reads_beats: l1s.iter().any(|(n, _)| n.reads_beats)
                || l2s.iter().any(|n| n.reads_beats)
                || l3s.iter().any(|n| n.reads_beats)
                || l4s.iter().any(|n| n.reads_beats),
            sources,
            params,
            param_values,
            moved,
            ranges,
            rate_bounds: l4s
                .iter()
                .map(|n| karakuri_ir::rate::point_rate_bound(n))
                .collect(),
            camera: Orbit::default(),
            cameras: camera_nodes,
            merge: (layering == Layering::Composite)
                .then(|| crate::node::Merge::build(device, renderer_count, 1, 1)),
            edges: vec![Input::default(); renderer_count],
            bindings: Vec::new(),
            interface: Vec::new(),
            l1_count: l1s.len(),
            field_count: fields.len(),
            field_declared: fields.iter().map(|f| declared_keys(f)).collect(),
            field_params: {
                let mut keys: Vec<String> = field_bound
                    .iter()
                    .flat_map(|(_, slot, ordinal)| {
                        fields[*ordinal]
                            .params
                            .iter()
                            .map(move |p| karakuri_codegen::layout::field_param_key(slot, &p.name))
                    })
                    .collect();
                keys.sort_unstable();
                keys.dedup();
                keys
            },
            field_bound,
            source_bound,
        };
        for source in &set.sources {
            source.sim.initialize(queue);
            if let Some(other) = &source.paired {
                other.initialize(queue);
            }
        }
        for camera in &set.cameras {
            camera.write_state(queue, &set.orbit().state(0.0));
            camera.write_canvas(queue, 1.0);
        }
        set.prime(device, queue);
        Ok(set)
    }

    /// Resizes renderer viewports and accumulation targets to match new dimensions.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.viewport = [width.max(1) as f32, height.max(1) as f32];
        for renderer in self.sources.iter_mut().flat_map(|s| &mut s.renderers) {
            renderer.resize(device, width, height);
        }
        if let Some(merge) = &mut self.merge {
            merge.resize(device, width, height);
        }
    }

    /// Returns the currently clamped viewport dimensions `(width, height)`.
    pub fn viewport(&self) -> (u32, u32) {
        (self.viewport[0] as u32, self.viewport[1] as u32)
    }

    /// Returns the committed simulation steps elapsed.
    pub fn steps_taken(&self) -> u64 {
        self.steps_taken
    }

    /// Returns the uncommitted steps staged for the current frame.
    pub fn staged_delta(&self) -> u64 {
        self.staged_delta
    }

    /// Returns the active ping-pong buffer index for the primary geometry.
    pub fn parity(&self) -> usize {
        self.sources.first().map_or(0, |s| s.sim.parity())
    }

    /// Returns the committed ping-pong buffer index for the primary geometry.
    pub fn committed_parity(&self) -> usize {
        self.sources.first().map_or(0, |s| s.sim.committed_parity())
    }

    /// Commits staged clock steps, buffer parities, and composite input edges upon submission.
    pub fn commit(&mut self) {
        self.steps_taken += self.staged_delta;
        self.staged_delta = 0;
        if let Some(edges) = self.staged_edges.take() {
            self.edges = edges;
        }
        for source in &mut self.sources {
            source.sim.commit();
            if let Some(other) = &mut source.paired {
                other.commit();
            }
        }
    }

    /// Discards staged simulation clock advancement and ping-pong parities.
    pub fn discard(&mut self) {
        self.staged_delta = 0;
        self.staged_edges = None;
        for source in &mut self.sources {
            source.sim.discard();
            if let Some(other) = &mut source.paired {
                other.discard();
            }
        }
    }

    /// Returns elapsed simulation time in seconds.
    pub fn time(&self) -> f32 {
        self.t_at(self.steps_taken)
    }

    /// Returns simulation time in seconds after `n` steps.
    fn t_at(&self, n: u64) -> f32 {
        n as f32 * self.dt
    }

    /// Reads back the total active element count across all sources. Blocks GPU queue.
    pub fn live_count(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> u32 {
        self.sources
            .iter()
            .map(|s| s.sim.live_count(device, queue))
            .sum()
    }

    /// Reads back raw element buffer bytes decoded against the layout. Blocks GPU queue.
    pub fn read_elements(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        self.sources[0].sim.read_elements(device, queue)
    }

    /// Returns the primary geometry's element buffer layout.
    pub fn element_layout(&self) -> &ElementLayout {
        self.sources[0].sim.element_layout()
    }

    /// Returns the total element capacity across all geometry sources.
    pub fn capacity(&self) -> u32 {
        self.sources.iter().map(|s| s.sim.capacity()).sum()
    }

    /// Returns true if all Set procedures are closed-form functions of seed, time, and params.
    pub fn is_closed_form(&self) -> bool {
        self.closed_form
    }

    /// Returns true if any procedure in the Set references the ambient beat count.
    pub fn reads_beats(&self) -> bool {
        self.reads_beats
    }

    /// Seeks the simulation clock directly to `steps_taken` without running intermediate steps.
    pub fn seek(&mut self, steps_taken: u64) {
        self.discard();
        self.steps_taken = steps_taken;
    }

    /// Resets simulation buffers and clock state back to initial post-build conditions.
    pub fn rewind(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        self.discard();
        self.steps_taken = 0;
        for source in &mut self.sources {
            source.sim.rewind(queue);
            if let Some(other) = &mut source.paired {
                other.rewind(queue);
            }
        }
    }

    /// Attaches a dynamic signal to a parameter, replacing any existing binding on that target.
    pub fn bind(&mut self, binding: Binding) -> Bound {
        let declares = |names: &[String], map: &HashMap<String, f32>| {
            names.contains(&binding.key) && map.contains_key(&binding.key)
        };
        if let Some(name) = binding.signal.strip_prefix(CONTROL_PREFIX) {
            if !self.published().iter().any(|p| p.name == name) {
                return Bound::NoSuchControl;
            }
        }
        let range = self.nodes_of(binding.layer);
        let names = self.declared_names(binding.layer);
        let found = names.iter().enumerate().any(|(at, n)| {
            binding.covers(at)
                && self
                    .params
                    .get(range.start + at)
                    .is_some_and(|map| declares(n, map))
        });
        if !found {
            return Bound::NoSuchParam;
        }
        self.bindings.retain(|b| {
            b.layer != binding.layer || b.key != binding.key || b.index != binding.index
        });
        self.bindings.push(binding);
        Bound::Yes
    }

    /// Detaches a dynamic signal from a parameter.
    pub fn unbind(&mut self, layer: Kind, index: Option<u32>, key: &str) -> bool {
        let before = self.bindings.len();
        self.bindings
            .retain(|b| b.layer != layer || b.key != key || b.index != index);
        self.bindings.len() != before
    }

    /// Returns the element storage allocation for each node instance in the Set.
    pub fn element_storage(&self) -> Vec<ElementStorage> {
        self.sources
            .iter()
            .flat_map(|source| {
                std::iter::once(source.sim.element_storage())
                    .chain(source.paired.iter().map(Simulation::element_storage))
                    .chain(source.deforms.iter().map(Deform::element_storage))
            })
            .collect()
    }

    /// Returns the total bytes allocated across all element buffers in the Set.
    pub fn element_storage_bytes(&self) -> u64 {
        self.element_storage().iter().map(|e| e.bytes).sum()
    }

    /// Returns canonical names for each node in execution order.
    pub fn node_names(&self) -> &[String] {
        &self.names
    }

    /// Returns hash salts assigned to each geometry source.
    pub fn source_salts(&self) -> &[u32] {
        &self.source_salts
    }

    /// Returns allocated element capacities for each geometry source.
    pub fn source_capacities(&self) -> Vec<u32> {
        let mut out = vec![0; self.l1_count];
        for source in &self.sources {
            for (k, sim) in std::iter::once(&source.sim)
                .chain(source.paired.iter())
                .enumerate()
            {
                out[source.procedures[k]] = sim.capacity();
            }
        }
        out
    }

    /// Returns declared `[min, max, default]` capacity specifications for each geometry.
    pub fn declared_capacities(&self) -> &[[u32; 3]] {
        &self.declared_capacities
    }

    /// Looks up a node by name, returning its `(layer, index)` address if found.
    pub fn node_named(&self, name: &str) -> Option<(Kind, u32)> {
        let at = self.names.iter().position(|n| n == name)?;
        Kind::ALL.into_iter().find_map(|kind| {
            let range = self.nodes_of(kind);
            range
                .contains(&at)
                .then(|| (kind, (at - range.start) as u32))
        })
    }

    /// Returns the number of procedures of the given `layer` in this set.
    ///
    /// A procedure chain is instantiated once per source, but procedures are shared.
    fn procedures(&self, layer: Kind) -> usize {
        let first = &self.sources[0];
        match layer {
            Kind::L1 => self.l1_count,
            Kind::L2 => first.deforms.len(),
            Kind::L3 => self.cameras.len(),
            Kind::L4 => first.renderers.len(),
            Kind::Field => self.field_count,
            // Nested L5 nodes are not yet supported.
            Kind::L5 => 0,
        }
    }

    /// Returns the starting index of a layer's parameter maps in [`Set::params`].
    fn slot_of(&self, layer: Kind) -> usize {
        match layer {
            Kind::L1 => 0,
            Kind::L2 => self.l1_count,
            Kind::L3 => self.l1_count + self.procedures(Kind::L2),
            Kind::L4 => self.l1_count + self.procedures(Kind::L2) + self.procedures(Kind::L3),
            Kind::Field => {
                self.l1_count
                    + self.procedures(Kind::L2)
                    + self.procedures(Kind::L3)
                    + self.procedures(Kind::L4)
            }
            Kind::L5 => self.params.len(),
        }
    }

    /// Returns the range of indices in [`Set::params`] belonging to `layer`, in node order.
    fn nodes_of(&self, layer: Kind) -> std::ops::Range<usize> {
        let start = self.slot_of(layer);
        match layer {
            Kind::L1 => start..start + self.l1_count,
            Kind::L2 => start..start + self.procedures(Kind::L2),
            Kind::L3 => start..start + self.procedures(Kind::L3),
            Kind::L4 => start..self.params.len() - self.field_count,
            Kind::Field => start..start + self.field_count,
            Kind::L5 => start..start,
        }
    }

    /// Returns the declared parameter keys for each node of `layer`, in declaration order.
    ///
    /// Entries align with [`Set::nodes_of`]. Vector parameters are expanded to individual
    /// component keys (`x`, `y`, `z`).
    fn declared_names(&self, layer: Kind) -> Vec<&[String]> {
        match layer {
            Kind::L1 => {
                let mut out: Vec<&[String]> = vec![&[]; self.l1_count];
                for source in &self.sources {
                    for (k, sim) in std::iter::once(&source.sim)
                        .chain(source.paired.iter())
                        .enumerate()
                    {
                        out[source.procedures[k]] = sim.param_keys();
                    }
                }
                out
            }
            Kind::L2 => self.sources[0]
                .deforms
                .iter()
                .map(|d| d.param_keys())
                .collect(),
            Kind::L3 => self.cameras.iter().map(|c| c.param_keys()).collect(),
            Kind::Field => self.field_declared.iter().map(Vec::as_slice).collect(),
            Kind::L4 => self.sources[0]
                .renderers
                .iter()
                .map(|r| r.param_keys())
                .collect(),
            Kind::L5 => Vec::new(),
        }
    }

    /// Sets every declaration of `name` across all applicable nodes, returning the count written.
    ///
    /// The built-in camera is excluded from bare name writes; see [`Set::addressed_only`].
    pub fn set_param(&mut self, name: &str, value: f32) -> usize {
        let mut written = 0;
        let addressed_only = self.addressed_only();
        for (at, (node, moved)) in self
            .params
            .iter_mut()
            .zip(self.moved.iter_mut())
            .enumerate()
        {
            if Some(at) == addressed_only {
                continue;
            }
            if let Some(slot) = node.get_mut(name) {
                *slot = value;
                moved.insert(name.to_string());
                written += 1;
            }
        }
        if written > 0 {
            for (slot, node_values) in self.param_values.iter_mut().enumerate() {
                if Some(slot) == addressed_only {
                    continue;
                }
                if let Some((base, comp_idx)) = parse_component_key(name) {
                    if let Some(vec_val) = node_values.get_mut(base) {
                        update_value_component(vec_val, comp_idx, value);
                    }
                } else if let Some(karakuri_store::record::Value::Scalar(s)) =
                    node_values.get_mut(name)
                {
                    *s = value;
                }
            }
        }
        written
    }

    /// Sets a specific node's declaration of `name`.
    ///
    /// Returns `false` if the node does not exist or does not declare `name`.
    pub fn set_param_at(&mut self, layer: Kind, index: u32, name: &str, value: f32) -> bool {
        let Some(slot) = self.nodes_of(layer).nth(index as usize) else {
            return false;
        };
        match self.params.get_mut(slot).and_then(|n| n.get_mut(name)) {
            Some(held) => {
                *held = value;
                self.moved[slot].insert(name.to_string());
                if let Some((base, comp_idx)) = parse_component_key(name) {
                    if let Some(vec_val) = self.param_values[slot].get_mut(base) {
                        update_value_component(vec_val, comp_idx, value);
                    }
                } else if let Some(karakuri_store::record::Value::Scalar(s)) =
                    self.param_values[slot].get_mut(name)
                {
                    *s = value;
                }
                true
            }
            None => false,
        }
    }

    /// Sets a parameter value atomically, either addressed to a node or across all matching nodes.
    ///
    /// Setting a vector parameter under its bare name updates all components atomically.
    pub fn set_param_value(
        &mut self,
        at: Option<karakuri_store::record::NodeAddress>,
        key: &str,
        value: karakuri_store::record::Value,
    ) -> usize {
        match at {
            Some(addr) => {
                let layer = kind_of_layer(addr.layer);
                usize::from(self.set_param_value_at(layer, addr.index, key, value))
            }
            None => {
                let mut written = 0;
                let addressed_only = self.addressed_only();
                let len = self.param_values.len();
                for slot in 0..len {
                    if Some(slot) == addressed_only {
                        continue;
                    }
                    if self.set_param_value_in_slot(slot, key, value) {
                        written += 1;
                    }
                }
                written
            }
        }
    }

    /// Sets one node's declaration of `name` to `value` atomically.
    pub fn set_param_value_at(
        &mut self,
        layer: Kind,
        index: u32,
        name: &str,
        value: karakuri_store::record::Value,
    ) -> bool {
        let Some(slot) = self.nodes_of(layer).nth(index as usize) else {
            return false;
        };
        self.set_param_value_in_slot(slot, name, value)
    }

    fn set_param_value_in_slot(
        &mut self,
        slot: usize,
        key: &str,
        value: karakuri_store::record::Value,
    ) -> bool {
        // Case 1: `key` matches a declaration in `param_values[slot]`
        if let Some(decl_val) = self
            .param_values
            .get(slot)
            .and_then(|m| m.get(key).copied())
        {
            if decl_val.len() == value.len() {
                self.param_values[slot].insert(key.to_string(), value);
                if value.len() == 1 {
                    if let Some(s) = value.get(0) {
                        self.params[slot].insert(key.to_string(), s);
                        self.moved[slot].insert(key.to_string());
                    }
                } else {
                    for (i, v) in value.components().iter().enumerate() {
                        let comp_key = karakuri_ir::component_key(key, i);
                        self.params[slot].insert(comp_key.clone(), *v);
                        self.moved[slot].insert(comp_key);
                    }
                    self.moved[slot].insert(key.to_string());
                }
                return true;
            }
            return false;
        }

        // Case 2: `key` is a single component key (e.g. "glow.y") and `value` is Scalar
        if value.len() == 1 {
            let scalar = value.get(0).unwrap();
            if let Some(held) = self.params.get_mut(slot).and_then(|m| m.get_mut(key)) {
                *held = scalar;
                self.moved[slot].insert(key.to_string());

                if let Some((base, comp_idx)) = parse_component_key(key) {
                    if let Some(vec_val) = self.param_values[slot].get_mut(base) {
                        update_value_component(vec_val, comp_idx, scalar);
                    }
                }
                return true;
            }
        }

        false
    }

    /// Returns the index of the built-in camera within the L3 layer, if present.
    fn builtin_camera(&self) -> Option<usize> {
        self.cameras.iter().position(|c| c.is_builtin())
    }

    /// Returns the slot index of the built-in camera in [`Set::params`].
    ///
    /// The built-in camera is only addressable explicitly and is excluded from wildcard writes.
    fn addressed_only(&self) -> Option<usize> {
        self.builtin_camera()
            .and_then(|at| self.nodes_of(Kind::L3).nth(at))
    }

    /// Returns the current state of the built-in camera orbit.
    pub fn orbit(&self) -> Orbit {
        let slot = self.addressed_only();
        self.camera
            .with_placement(|key| slot.and_then(|slot| self.params[slot].get(key).copied()))
    }

    /// Updates the built-in camera orbit state and its parameter values.
    pub fn aim_camera(&mut self, orbit: Orbit) {
        self.camera = orbit;
        let Some(slot) = self.addressed_only() else {
            return;
        };
        for (key, value) in orbit.placement_values() {
            if let Some(held) = self.params[slot].get_mut(&key) {
                *held = value;
            }
            if let Some(held_val) = self.param_values[slot].get_mut(&key) {
                *held_val = karakuri_store::record::Value::Scalar(value);
            }
        }
    }

    /// Returns the [`Authority`] of the node at `(layer, index)`, or `None` if it does not exist.
    pub fn authority(&self, layer: Kind, index: u32) -> Option<Authority> {
        let slot = self.nodes_of(layer).nth(index as usize)?;
        self.authorities.get(slot).copied()
    }

    /// Sets the [`Authority`] of the node at `(layer, index)`.
    ///
    /// Returns `false` if the node does not exist.
    pub fn set_authority(&mut self, layer: Kind, index: u32, authority: Authority) -> bool {
        let Some(slot) = self.nodes_of(layer).nth(index as usize) else {
            return false;
        };
        match self.authorities.get_mut(slot) {
            Some(held) => {
                *held = authority;
                true
            }
            None => false,
        }
    }

    /// Returns the `(layer, index)` coordinates of every node declaring `key`.
    pub fn landing_of(&self, key: &str) -> Vec<(Kind, u32)> {
        self.landing(key)
            .into_iter()
            .map(|(layer, index, _)| (layer, index))
            .collect()
    }

    /// Returns every node declaring `key`, along with its effective authority.
    fn landing(&self, key: &str) -> Vec<(Kind, u32, Authority)> {
        Kind::ALL
            .into_iter()
            .flat_map(|layer| {
                self.nodes_of(layer)
                    .enumerate()
                    .map(move |(index, slot)| (layer, index as u32, slot))
            })
            .filter(|(_, _, slot)| Some(*slot) != self.addressed_only())
            .filter(|(_, _, slot)| self.params[*slot].contains_key(key))
            .map(|(layer, index, slot)| {
                (
                    layer,
                    index,
                    self.authorities.get(slot).copied().unwrap_or_default(),
                )
            })
            .collect()
    }

    /// Applies a [`ParamWrite`], addressed or wildcarded across nodes.
    ///
    /// Returns the number of nodes updated, or an error if a wildcard write
    /// crosses conflicting node authorities.
    pub fn write_param(&mut self, write: &ParamWrite) -> Result<usize, CrossesAuthority> {
        match write.at {
            None => {
                if let Some(refused) = CrossesAuthority::over(&write.key, &self.landing(&write.key))
                {
                    return Err(refused);
                }
                Ok(self.set_param(&write.key, write.value))
            }
            Some((layer, index)) => Ok(usize::from(self.set_param_at(
                layer,
                index,
                &write.key,
                write.value,
            ))),
        }
    }

    /// Returns whether `key` at `(layer, index)` was modified after initialization.
    fn moved_at(&self, layer: Kind, index: u32, key: &str) -> bool {
        self.nodes_of(layer)
            .nth(index as usize)
            .is_some_and(|slot| self.moved[slot].contains(key))
    }

    /// Copies modified parameter values from an `outgoing` set to this set.
    ///
    /// Only parameters modified in `outgoing` and still declared in `self` are copied.
    /// Keys already modified in `self` are preserved. Returns the count of carried values.
    pub fn carry_moved_from(&mut self, outgoing: &Set) -> usize {
        let mut carried = 0;
        let moved: Vec<(Kind, u32, &str, f32)> = Kind::ALL
            .into_iter()
            .flat_map(|layer| {
                outgoing
                    .nodes_of(layer)
                    .enumerate()
                    .map(move |(index, slot)| (layer, index as u32, slot))
            })
            .flat_map(|(layer, index, slot)| {
                outgoing.moved[slot]
                    .iter()
                    .filter_map(move |key| {
                        Some((layer, index, key.as_str(), *outgoing.params[slot].get(key)?))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        for (layer, index, key, value) in moved {
            if self.moved_at(layer, index, key) {
                continue;
            }
            if self.set_param_at(layer, index, key, value) {
                carried += 1;
            }
        }
        carried
    }

    /// Copies dynamic signal bindings from an `outgoing` set to this set.
    ///
    /// Preserves bindings already stated on this set. Returns the count of carried bindings.
    pub fn carry_bound_from(&mut self, outgoing: &Set) -> usize {
        let mut carried = 0;
        for binding in &outgoing.bindings {
            let stated = self.bindings.iter().any(|b| {
                b.layer == binding.layer && b.index == binding.index && b.key == binding.key
            });
            if stated {
                continue;
            }
            if self.bind(binding.clone()) == Bound::Yes {
                carried += 1;
            }
        }
        carried
    }

    /// Adds a control to this set's published interface.
    ///
    /// Calling this method switches the set from publishing all declared parameters
    /// by default to publishing only explicitly added controls.
    pub fn publish(&mut self, control: Published) -> Result<(), PublishError> {
        let Some([min, max]) = self.declared_range(control.at, &control.key) else {
            return Err(PublishError::NoSuchControl {
                key: control.key,
                at: match control.at {
                    Some((layer, index)) => format!(" at {layer:?}:{index}"),
                    None => String::new(),
                },
            });
        };
        let [low, high] = control.range;
        if low < min || high > max || low > high {
            return Err(PublishError::RangeNotASubset {
                name: control.name,
                key: control.key,
                low,
                high,
                min,
                max,
            });
        }
        if self.interface.iter().any(|p| p.name == control.name) {
            return Err(PublishError::DuplicateName(control.name));
        }
        self.interface.push(control);
        Ok(())
    }

    /// Returns the declared parameter range, taking the intersection over all matching nodes.
    fn declared_range(&self, at: Option<(Kind, u32)>, key: &str) -> Option<[f32; 2]> {
        let mut found: Option<[f32; 2]> = None;
        for layer in Kind::ALL {
            for (index, slot) in self.nodes_of(layer).enumerate() {
                if at.is_some_and(|(l, i)| l != layer || i != index as u32) {
                    continue;
                }
                // Wildcard ranges do not narrow against the built-in camera.
                if at.is_none() && Some(slot) == self.addressed_only() {
                    continue;
                }
                let Some([min, max]) = self.ranges.get(slot).and_then(|n| n.get(key)).copied()
                else {
                    continue;
                };
                found = Some(match found {
                    None => [min, max],
                    Some([lo, hi]) => [lo.max(min), hi.min(max)],
                });
            }
        }
        found
    }

    /// Returns the published control interface for this set.
    ///
    /// If no controls were explicitly published via [`Set::publish`], returns the
    /// full declared interface in definition order.
    pub fn published(&self) -> Vec<Published> {
        if !self.interface.is_empty() {
            return self.interface.clone();
        }
        self.declared_interface()
    }

    /// Returns the full list of declared parameters as a published interface.
    ///
    /// The returned controls maintain a stable, deterministic order based on declaration order.
    pub fn declared_interface(&self) -> Vec<Published> {
        let mut keys: Vec<&String> = Vec::new();
        let mut out: Vec<Published> = Vec::new();
        for layer in Kind::ALL {
            for (index, names) in self.declared_names(layer).into_iter().enumerate() {
                let addressed = self.nodes_of(layer).nth(index) == self.addressed_only();
                for key in names {
                    if addressed {
                        let at = Some((layer, index as u32));
                        if let Some(range) = self.declared_range(at, key) {
                            out.push(Published {
                                name: key.clone(),
                                at,
                                key: key.clone(),
                                range,
                            });
                        }
                        continue;
                    }
                    if keys.contains(&key) {
                        continue;
                    }
                    keys.push(key);
                    if let Some(range) = self.declared_range(None, key) {
                        out.push(Published {
                            name: key.clone(),
                            at: None,
                            key: key.clone(),
                            range,
                        });
                    }
                }
            }
        }
        out
    }

    /// Sets a published control by name, clamping `value` to its published range.
    ///
    /// Returns `Ok(false)` if no control publishes `name`, or an error if the write
    /// crosses conflicting node authorities.
    pub fn set_published(&mut self, name: &str, value: f32) -> Result<bool, CrossesAuthority> {
        let Some(control) = self.published().into_iter().find(|p| p.name == name) else {
            return Ok(false);
        };
        let clamped = value.clamp(control.range[0], control.range[1]);
        Ok(self.write_param(&ParamWrite {
            at: control.at,
            key: control.key,
            value: clamped,
        })? > 0)
    }

    /// Returns a published control's normalized position in `[0.0, 1.0]`.
    fn control_position(&self, name: &str) -> Option<f32> {
        let (at, key, [low, high]) = match self.interface.iter().find(|p| p.name == name) {
            Some(control) => (control.at, control.key.as_str(), control.range),
            None if self.interface.is_empty() => (None, name, self.declared_range(None, name)?),
            None => return None,
        };
        let value = self.value_at(at, key)?;
        if high <= low {
            return Some(1.0);
        }
        Some(((value - low) / (high - low)).clamp(0.0, 1.0))
    }

    /// Returns the current value of a published control by name.
    pub fn published_value(&self, name: &str) -> Option<f32> {
        let control = self.published().into_iter().find(|p| p.name == name)?;
        self.value_at(control.at, &control.key)
    }

    /// Returns the parameter value for `key` at the specified node, or the first matching node for wildcards.
    pub fn value_at(&self, at: Option<(Kind, u32)>, key: &str) -> Option<f32> {
        for layer in Kind::ALL {
            for (index, slot) in self.nodes_of(layer).enumerate() {
                if at.is_some_and(|(l, i)| l != layer || i != index as u32) {
                    continue;
                }
                if let Some(v) = self.params.get(slot).and_then(|n| n.get(key)) {
                    return Some(*v);
                }
            }
        }
        None
    }

    /// Returns the input edges to the composite pass in draw order.
    pub fn inputs(&self) -> &[Input] {
        &self.edges
    }

    /// Sets the composite input edge at index `at`. Returns `false` if out of bounds.
    pub fn set_input(&mut self, at: usize, input: Input) -> bool {
        match self.edges.get_mut(at) {
            Some(edge) => {
                *edge = input;
                true
            }
            None => false,
        }
    }

    /// Selects the renderer at index `at` as the live composite input.
    ///
    /// Returns `false` if `at` is out of bounds.
    pub fn select_renderer(&mut self, at: usize) -> bool {
        crate::mix::select(&mut self.edges, at)
    }

    /// Stages selection of the live renderer during an uncommitted frame.
    pub fn stage_select_renderer(&mut self, at: usize) -> bool {
        if self.staged_edges.is_none() {
            self.staged_edges = Some(self.edges.clone());
        }
        crate::mix::select(self.staged_edges.as_mut().unwrap(), at)
    }

    /// Returns the active or staged inputs to the merge pass.
    pub fn edges(&self) -> &[Input] {
        self.staged_edges.as_deref().unwrap_or(&self.edges)
    }

    /// Returns the rendered primitive topologies in draw order across all sources.
    pub fn drawn_topologies(&self) -> Vec<karakuri_ir::Topology> {
        self.sources
            .iter()
            .flat_map(|s| &s.renderers)
            .map(|r| r.topology())
            .collect()
    }

    /// Returns the conservative primitive rate bounds for each L4 procedure.
    pub fn rate_bounds(&self) -> &[karakuri_ir::rate::RateBound] {
        &self.rate_bounds
    }

    /// Returns the first parameter whose current value violates its rate bound range.
    pub fn rate_bound_contradicted(&self) -> Option<(String, f32, [f32; 2])> {
        let l4s = self.nodes_of(Kind::L4);
        for (bound, slot) in self.rate_bounds.iter().zip(l4s) {
            let karakuri_ir::rate::Bound::AtLeast { over, .. } = &bound.bound else {
                continue;
            };
            let (Some(values), Some(ranges)) = (self.params.get(slot), self.ranges.get(slot))
            else {
                continue;
            };
            for name in over {
                for (key, declared) in ranges {
                    if key != name && !key.strip_prefix(name).is_some_and(|r| r.starts_with('.')) {
                        continue;
                    }
                    let Some(&value) = values.get(key) else {
                        continue;
                    };
                    if value < declared[0] || value > declared[1] {
                        return Some((key.clone(), value, *declared));
                    }
                }
            }
        }
        None
    }

    /// Returns the layering strategy used by this set.
    pub fn layering(&self) -> Layering {
        if self.merge.is_some() {
            Layering::Composite
        } else {
            Layering::Overdraw
        }
    }

    /// Returns the scalar parameter value of `name` from the first declaring node.
    pub fn param(&self, name: &str) -> Option<f32> {
        self.params.iter().find_map(|node| node.get(name).copied())
    }

    /// Returns the typed parameter [`Value`](karakuri_store::record::Value) of `name` from the first declaring node.
    pub fn param_value(&self, name: &str) -> Option<karakuri_store::record::Value> {
        self.param_values
            .iter()
            .find_map(|node| node.get(name).copied())
    }

    /// Returns the typed parameter [`Value`](karakuri_store::record::Value) at `(layer, index)`.
    pub fn param_value_at(
        &self,
        layer: Kind,
        index: u32,
        name: &str,
    ) -> Option<karakuri_store::record::Value> {
        let slot = self.nodes_of(layer).nth(index as usize)?;
        self.param_values
            .get(slot)
            .and_then(|node| node.get(name).copied())
    }

    /// Returns the typed parameter [`Value`](karakuri_store::record::Value) at `address`.
    pub fn param_value_at_address(
        &self,
        address: karakuri_store::record::NodeAddress,
        name: &str,
    ) -> Option<karakuri_store::record::Value> {
        let layer = kind_of_layer(address.layer);
        self.param_value_at(layer, address.index, name)
    }

    /// Returns an iterator over all parameter values with their node coordinates.
    pub fn params(&self) -> impl Iterator<Item = (Kind, u32, &str, f32)> + '_ {
        let addressed: Vec<(Kind, u32, usize)> = Kind::ALL
            .into_iter()
            .flat_map(|layer| {
                self.nodes_of(layer)
                    .enumerate()
                    .map(move |(index, slot)| (layer, index as u32, slot))
            })
            .collect();
        addressed.into_iter().flat_map(move |(layer, index, slot)| {
            self.params[slot]
                .iter()
                .map(move |(k, v)| (layer, index, k.as_str(), *v))
        })
    }

    /// Returns an iterator over all bound parameters and their most recent evaluated values.
    pub fn bound(&self) -> impl Iterator<Item = (&str, f32)> {
        self.bindings.iter().map(|b| (b.key.as_str(), b.value()))
    }

    /// Returns the active parameter signal bindings.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// Prepares simulation uniforms and advances time on the session clock.
    pub fn prepare(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals) {
        self.prepare_on(queue, steps, signals, Clock::Session);
    }

    /// Prepares simulation uniforms for an off-air set on its local clock.
    pub fn prepare_warming(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals) {
        self.prepare_on(queue, steps, signals, Clock::Local);
    }

    /// Common preparation routine for both on-air and warming sets.
    fn prepare_on(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals, clock: Clock) {
        let steps = steps.min(MAX_STEPS);
        self.staged_delta = u64::from(steps);
        let next_steps_taken = self.steps_taken + self.staged_delta;
        let view = match clock {
            Clock::Session => *signals,
            Clock::Local => {
                let lag = signals
                    .oscillator()
                    .steps_taken()
                    .saturating_sub(next_steps_taken);
                signals.behind(lag as f64 * f64::from(self.dt))
            }
        };
        self.resolve_bindings(&view);

        let first = next_steps_taken - u64::from(steps) + 1;
        let mut instants = [(0.0f32, 0.0f32); MAX_STEPS as usize];
        for (k, slot) in instants.iter_mut().enumerate().take(usize::from(steps)) {
            let t = self.t_at(first + k as u64);
            *slot = (t, view.oscillator().at_time(f64::from(t)).beats() as f32);
        }

        {
            let bindings = &self.bindings;
            let field_range = self.nodes_of(Kind::Field);
            let field_maps = &self.params[field_range.clone()];
            let field_bound = &self.field_bound;
            let field_params = &self.field_params;
            let params = &self.params;
            let param_values = &self.param_values;
            let source_bound = &self.source_bound;
            let source_salts = &self.source_salts;

            for source in &mut self.sources {
                let procedures = source.procedures.clone();
                for (k, sim) in std::iter::once(&mut source.sim)
                    .chain(source.paired.as_mut())
                    .enumerate()
                {
                    let at = procedures[k];
                    let own = &params[at];
                    let own_values = &param_values[at];
                    let param = |name: &str| effective(bindings, own, Kind::L1, at, name);
                    let param_val =
                        |name: &str| effective_vector(bindings, own_values, Kind::L1, at, name);
                    let tick = crate::node::Tick {
                        steps,
                        dt: self.dt,
                        instants,
                        param: &param,
                        param_value: Some(&param_val),
                        field_params,
                        field_value: &|name: &str| field_value(field_bound, field_maps, at, name),
                        source_value: &|key: &str| {
                            source_value(source_bound, source_salts, at, key)
                        },
                    };
                    sim.prepare(queue, &tick);
                }
            }
        }

        let t = self.t_at(next_steps_taken);
        self.last_beats = view.oscillator().at_time(f64::from(t)).beats() as f32;
        self.write_l4_uniforms(queue, t);
    }

    /// Writes uniform buffers for L4 renderers and camera nodes.
    fn write_l4_uniforms(&mut self, queue: &wgpu::Queue, t: f32) {
        self.write_l2_uniforms(queue, t);
        if let Some(merge) = &self.merge {
            merge.write_uniform(queue, self.edges());
        }
        {
            let first = self.slot_of(Kind::L3);
            let aspect = self.viewport[0] / self.viewport[1];
            let field_range = self.nodes_of(Kind::Field);
            let (bindings, dt, field_params, field_bound, field_maps, viewport, beats, salt) = (
                &self.bindings,
                self.dt,
                &self.field_params,
                &self.field_bound,
                &self.params[field_range.clone()],
                self.viewport,
                self.last_beats,
                self.seed_salt,
            );
            let no_sources = |_: &str| None;
            let stated = self.camera;
            let params = &self.params[first..];
            let param_values = &self.param_values[first..];
            for (at, (camera, params)) in self.cameras.iter_mut().zip(params).enumerate() {
                camera.write_canvas(queue, aspect);
                let orbit = match camera.is_builtin() {
                    true => {
                        stated.with_placement(|key| effective(bindings, params, Kind::L3, at, key))
                    }
                    false => stated,
                };
                let fallback = orbit.state(t);
                let own_values = &param_values[at];
                let param_val =
                    |name: &str| effective_vector(bindings, own_values, Kind::L3, at, name);
                let view = crate::node::View {
                    t,
                    beats,
                    seed_salt: salt,
                    viewport,
                    field_params,
                    field_value: &|name: &str| {
                        field_value(field_bound, field_maps, first + at, name)
                    },
                    source_value: &no_sources,
                    param: &|name: &str| effective(bindings, params, Kind::L3, at, name),
                    param_value: Some(&param_val),
                };
                camera.prepare(queue, &view, dt, &fallback);
            }
        }
        let field_range = self.nodes_of(Kind::Field);
        let (bindings, beats, viewport, field_params, field_bound, field_maps) = (
            &self.bindings,
            self.last_beats,
            self.viewport,
            &self.field_params,
            &self.field_bound,
            &self.params[field_range.clone()],
        );
        let (source_bound, source_salts) = (&self.source_bound, &self.source_salts);
        let first = self.slot_of(Kind::L4);
        let params = &self.params[first..];
        let param_values = &self.param_values[first..];
        for source in &mut self.sources {
            let salt = source.salt;
            for (at, (renderer, params)) in source.renderers.iter_mut().zip(params).enumerate() {
                let own_values = &param_values[at];
                let param_val =
                    |name: &str| effective_vector(bindings, own_values, Kind::L4, at, name);
                let view = crate::node::View {
                    t,
                    beats,
                    seed_salt: salt,
                    viewport,
                    field_params,
                    field_value: &|name: &str| {
                        field_value(field_bound, field_maps, first + at, name)
                    },
                    source_value: &|key: &str| {
                        source_value(source_bound, source_salts, first + at, key)
                    },
                    param: &|name: &str| effective(bindings, params, Kind::L4, at, name),
                    param_value: Some(&param_val),
                };
                renderer.write_uniforms(queue, &view);
            }
        }
    }

    /// Writes uniform buffers for L2 deformation nodes.
    fn write_l2_uniforms(&mut self, queue: &wgpu::Queue, t: f32) {
        let field_range = self.nodes_of(Kind::Field);
        let (bindings, beats, viewport, dt, field_params, field_bound, field_maps) = (
            &self.bindings,
            self.last_beats,
            self.viewport,
            self.dt,
            &self.field_params,
            &self.field_bound,
            &self.params[field_range.clone()],
        );
        let (source_bound, source_salts) = (&self.source_bound, &self.source_salts);
        let capacity = self.sources[0].sim.capacity();
        let range = self.nodes_of(Kind::L2);
        let first = range.start;
        let params = &self.params[range.clone()];
        let param_values = &self.param_values[range];
        for source in &mut self.sources {
            let salt = source.salt;
            for (at, (node, params)) in source.deforms.iter_mut().zip(params).enumerate() {
                let own_values = &param_values[at];
                let param_val =
                    |name: &str| effective_vector(bindings, own_values, Kind::L2, at, name);
                let view = crate::node::View {
                    t,
                    beats,
                    seed_salt: salt,
                    viewport,
                    field_params,
                    field_value: &|name: &str| {
                        field_value(field_bound, field_maps, first + at, name)
                    },
                    source_value: &|key: &str| {
                        source_value(source_bound, source_salts, first + at, key)
                    },
                    param: &|name: &str| effective(bindings, params, Kind::L2, at, name),
                    param_value: Some(&param_val),
                };
                node.write_uniforms(queue, &view, dt, capacity);
            }
        }
    }

    /// Evaluates each parameter binding against input signals and manual fallback values.
    fn resolve_bindings(&mut self, signals: &Signals) {
        let ranges: Vec<(usize, std::ops::Range<usize>)> = Kind::ALL
            .into_iter()
            .map(|k| (self.slot_of(k), self.nodes_of(k)))
            .collect();
        let mut bindings = std::mem::take(&mut self.bindings);
        let params = &self.params;
        for binding in &mut bindings {
            let (base, range) = match binding.layer {
                Kind::L1 => ranges[0].clone(),
                Kind::L2 => ranges[1].clone(),
                Kind::L3 => ranges[2].clone(),
                Kind::L4 => ranges[3].clone(),
                Kind::Field => (0, 0..0),
                Kind::L5 => (0, 0..0),
            };
            let manual = range
                .clone()
                .filter(|slot| binding.covers(slot - base))
                .filter_map(|slot| params.get(slot))
                .find_map(|node| node.get(&binding.key).copied())
                .unwrap_or(0.0);
            match binding.signal.strip_prefix(CONTROL_PREFIX) {
                Some(name) => {
                    match self.control_position(name) {
                        Some(at) => binding.drive(at),
                        None => binding.hold(manual),
                    };
                }
                None => {
                    binding.resolve(signals, manual);
                }
            }
        }
        self.bindings = bindings;
    }
}

/// Returns the effective value for `name`, resolving bindings before manual values.
fn effective(
    bindings: &[Binding],
    params: &HashMap<String, f32>,
    layer: Kind,
    index: usize,
    name: &str,
) -> Option<f32> {
    match bindings
        .iter()
        .find(|b| b.layer == layer && b.key == name && b.covers(index))
    {
        Some(binding) => Some(binding.value()),
        None => params.get(name).copied(),
    }
}

/// Returns the effective vector value, falling back to `None` if any component is driven by a binding.
fn effective_vector(
    bindings: &[Binding],
    param_values: &HashMap<String, karakuri_store::record::Value>,
    layer: Kind,
    index: usize,
    name: &str,
) -> Option<karakuri_store::record::Value> {
    let has_binding = bindings.iter().any(|b| {
        b.layer == layer
            && b.covers(index)
            && (b.key == name
                || (b.key.starts_with(name) && b.key.as_bytes().get(name.len()) == Some(&b'.')))
    });
    if has_binding {
        return None;
    }
    param_values.get(name).copied()
}

impl Set {
    /// Advances simulation and deformation passes for the current frame without rasterizing.
    pub fn step(&mut self, encoder: &mut wgpu::CommandEncoder, steps: u8) {
        let steps = if self
            .sources
            .iter()
            .flat_map(|s| &s.renderers)
            .all(|r| r.is_fullscreen())
        {
            0
        } else {
            steps
        };
        for source in &mut self.sources {
            source.sim.record(encoder, steps);
            if let Some(other) = &mut source.paired {
                other.record(encoder, steps);
            }
        }
        self.record_counts(encoder);
        for source in &self.sources {
            let parity = source.sim.parity();
            let mut counts = source.sim.counts();
            for node in &source.deforms {
                node.record(encoder, parity, counts);
                if let Some(own) = node.counts() {
                    counts = own;
                }
            }
        }
    }

    /// Records amplifier compute passes to derive instance counts.
    fn record_counts(&self, encoder: &mut wgpu::CommandEncoder) {
        for node in self.sources.iter().flat_map(|s| &s.deforms) {
            node.record_counts(encoder);
        }
    }

    /// Primes the deformation pipeline with initial simulation state during build.
    fn prime(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.sources.iter().all(|s| s.deforms.is_empty()) {
            return;
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("prime the deformation chain"),
        });
        self.record_counts(&mut encoder);
        for source in &self.sources {
            let parity = source.sim.parity();
            let mut counts = source.sim.counts();
            for node in &source.deforms {
                node.record(&mut encoder, parity, counts);
                if let Some(own) = node.counts() {
                    counts = own;
                }
            }
        }
        queue.submit([encoder.finish()]);
    }

    /// Returns the output count buffer from the last amplifier in the deformation chain.
    fn output_counts<'a>(&self, source: &'a Source) -> &'a wgpu::Buffer {
        source
            .deforms
            .iter()
            .rev()
            .find_map(|node| node.counts())
            .unwrap_or_else(|| source.sim.counts())
    }

    /// Records rasterization passes for all renderers into `target`.
    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        for camera in &self.cameras {
            camera.record(encoder);
        }
        let merge = self.merge.as_ref();
        for (source_at, source) in self.sources.iter().enumerate() {
            let (parity, counts) = (source.sim.parity(), self.output_counts(source));
            for (i, renderer) in source.renderers.iter().enumerate() {
                match merge {
                    None => {
                        renderer.draw(encoder, target, parity, counts, source_at == 0 && i == 0)
                    }
                    Some(merge) => {
                        renderer.draw(encoder, merge.target(i), parity, counts, source_at == 0)
                    }
                }
            }
        }
        if let Some(merge) = merge {
            merge.record(encoder, target);
        }
    }
}

impl VideoSource for Set {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        steps: u8,
    ) {
        self.step(encoder, steps);
        self.draw(encoder, target);
    }

    fn commit(&mut self) {
        self.commit();
    }

    fn discard(&mut self) {
        self.discard();
    }
}

/// Returns the default salt for a source derived from `seed_salt` and `source` index.
pub fn derived_salt(seed_salt: u32, source: usize) -> u32 {
    seed_salt.wrapping_add((source as u32).wrapping_mul(0x9E37_79B9))
}

/// Resolves a spliced field parameter value for `node`.
fn field_value(
    bound: &[(usize, String, usize)],
    maps: &[HashMap<String, f32>],
    node: usize,
    key: &str,
) -> Option<f32> {
    let (slot, declared) = key.strip_prefix("field\u{1}")?.split_once('\u{1}')?;
    let (_, _, ordinal) = bound
        .iter()
        .find(|(at, name, _)| *at == node && name == slot)?;
    maps.get(*ordinal)?.get(declared).copied()
}

/// Returns declared default values as typed [`Value`](karakuri_store::record::Value)s.
fn declared_default_values(node: &Checked) -> HashMap<String, karakuri_store::record::Value> {
    node.params
        .iter()
        .filter_map(|p| {
            let comps = p.default_components()?;
            let val = match comps.len() {
                1 => karakuri_store::record::Value::Scalar(comps[0]),
                2 => karakuri_store::record::Value::Vec2([comps[0], comps[1]]),
                3 => karakuri_store::record::Value::Vec3([comps[0], comps[1], comps[2]]),
                4 => karakuri_store::record::Value::Vec4([comps[0], comps[1], comps[2], comps[3]]),
                _ => return None,
            };
            Some((p.name.clone(), val))
        })
        .collect()
}

fn kind_of_layer(layer: karakuri_store::record::Layer) -> Kind {
    match layer {
        karakuri_store::record::Layer::L1 => Kind::L1,
        karakuri_store::record::Layer::L2 => Kind::L2,
        karakuri_store::record::Layer::L3 => Kind::L3,
        karakuri_store::record::Layer::L4 => Kind::L4,
        karakuri_store::record::Layer::Field => Kind::Field,
        karakuri_store::record::Layer::L5 => Kind::L5,
    }
}

fn parse_component_key(key: &str) -> Option<(&str, usize)> {
    let (base, comp) = key.rsplit_once('.')?;
    let idx = match comp {
        "x" | "r" => 0,
        "y" | "g" => 1,
        "z" | "b" => 2,
        "w" | "a" => 3,
        _ => return None,
    };
    Some((base, idx))
}

fn update_value_component(vec_val: &mut karakuri_store::record::Value, comp_idx: usize, val: f32) {
    match vec_val {
        karakuri_store::record::Value::Scalar(s) => {
            if comp_idx == 0 {
                *s = val;
            }
        }
        karakuri_store::record::Value::Vec2(arr) => {
            if comp_idx < 2 {
                arr[comp_idx] = val;
            }
        }
        karakuri_store::record::Value::Vec3(arr) => {
            if comp_idx < 3 {
                arr[comp_idx] = val;
            }
        }
        karakuri_store::record::Value::Vec4(arr) | karakuri_store::record::Value::Color(arr) => {
            if comp_idx < 4 {
                arr[comp_idx] = val;
            }
        }
    }
}

fn declared_defaults(node: &Checked) -> HashMap<String, f32> {
    node.params
        .iter()
        .filter_map(|p| Some((p.keys(), p.default_components()?)))
        .flat_map(|(keys, values)| keys.into_iter().zip(values))
        .collect()
}

/// Returns all addressable parameter keys for `node`, expanding vector parameters.
pub(crate) fn declared_keys(node: &Checked) -> Vec<String> {
    node.params.iter().flat_map(|p| p.keys()).collect()
}

/// Returns the declared valid range for each addressable parameter key.
fn declared_ranges(node: &Checked) -> HashMap<String, [f32; 2]> {
    node.params
        .iter()
        .flat_map(|p| {
            p.keys()
                .into_iter()
                .map(move |key| (key, [p.min, p.max]))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Resolves a declared source slot salt for `node`.
fn source_value(
    bound: &[(usize, String, usize)],
    salts: &[u32],
    node: usize,
    key: &str,
) -> Option<u32> {
    let slot = key.strip_prefix("source\u{1}")?;
    let (_, _, at) = bound
        .iter()
        .find(|(node_at, name, _)| *node_at == node && name == slot)?;
    salts.get(*at).copied()
}

#[cfg(test)]
mod tests {
    //! The two things `Set::build` does that no node does: put the two halves
    //! together, and read the `.kir`'s declared defaults. The byte-level checks
    //! for the L1 node's initial upload moved with it — see
    //! `crate::node::simulation`'s tests.
    use super::*;
    use crate::gpu::Gpu;

    /// Tests that a negative default parameter value is read correctly through the IR fold.
    #[test]
    fn a_negative_param_default_is_read_as_its_declared_value() {
        let src = r#"
proc signed_defaults {
  kind  L4
  blend additive

  param drift  : float [-1.0, 1.0] = -0.35
  param plain  : float [ 0.0, 1.0] =  0.25

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, drift + plain);
  }
}
"#;
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
        let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
        let of = |name: &str| {
            checked
                .params
                .iter()
                .find(|p| p.name == name)
                .expect("the param is declared")
                .default_scalar()
        };
        assert_eq!(
            of("drift"),
            Some(-0.35),
            "a negative default was read as an absence"
        );
        assert_eq!(
            of("plain"),
            Some(0.25),
            "a positive default stopped being read"
        );
    }

    /// Verifies that the engine's parameter map agrees with `karakuri_ir::Param::default_scalar`.
    #[test]
    fn the_engines_param_map_reads_a_negative_default_through_the_ir_fold() {
        let src = r#"
proc signed_defaults {
  kind  L4
  blend additive

  param drift  : float [-1.0, 1.0] = -0.35
  param plain  : float [ 0.0, 1.0] =  0.25

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, drift + plain);
  }
}
"#;
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
        let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
        let map = declared_defaults(&checked);
        assert_eq!(
            map.get("drift").copied(),
            Some(-0.35),
            "the map a uniform is packed from has stopped agreeing with \
             `karakuri_ir::Param::default_scalar`: a fold of this file's own read `-0.35` \
             as an absence, so the run loads a default the `param_decl` beside it does not \
             state"
        );
        assert_eq!(
            map.get("plain").copied(),
            Some(0.25),
            "the map a uniform is packed from has stopped agreeing with \
             `karakuri_ir::Param::default_scalar` about an ordinary positive default"
        );
    }

    /// Verifies that vector parameters are expanded into per-component keys in value and range maps.
    #[test]
    fn a_vector_param_enters_the_value_map_one_component_at_a_time() {
        let src = r#"
proc glowing {
  kind  L4
  blend additive

  param glow   : vec3  [0.0, 4.0] = vec3(0.4, 0.7, 1.0)
  param plain  : float [ 0.0, 1.0] =  0.25

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(glow * plain, 1.0);
  }
}
"#;
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
        let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));

        let values = declared_defaults(&checked);
        assert_eq!(
            values.get("glow.x").copied(),
            Some(0.4),
            "a `vec3` param is still not entering the map the uniform is packed from, so it \
             reaches the shader as zeroes"
        );
        assert_eq!(values.get("glow.y").copied(), Some(0.7));
        assert_eq!(values.get("glow.z").copied(), Some(1.0));
        assert_eq!(
            values.get("glow"),
            None,
            "the bare name addresses no number and must hold none"
        );
        assert_eq!(
            values.get("plain").copied(),
            Some(0.25),
            "a scalar param keeps its own name"
        );

        let ranges = declared_ranges(&checked);
        for key in ["glow.x", "glow.y", "glow.z"] {
            assert_eq!(
                ranges.get(key).copied(),
                Some([0.0, 4.0]),
                "{key} has no declared range, so nothing can publish, bind or clamp it"
            );
        }
        assert_eq!(ranges.get("glow"), None);
        assert_eq!(ranges.get("plain").copied(), Some([0.0, 1.0]));
        assert_eq!(
            values.keys().collect::<std::collections::BTreeSet<_>>(),
            ranges.keys().collect::<std::collections::BTreeSet<_>>(),
            "the value map and the range map are keyed by one walk and have stopped agreeing"
        );
    }

    /// Verifies that a vector default that cannot be evaluated leaves all component keys out.
    #[test]
    fn a_vector_default_that_cannot_be_stated_leaves_no_component_behind() {
        let src = r#"
proc partial {
  kind  L4
  blend additive

  param glow : vec3 [0.0, 4.0] = vec3(0.4, 0.7, 0.5 + 0.5)

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(glow, 1.0);
  }
}
"#;
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
        let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
        assert!(
            declared_defaults(&checked).is_empty(),
            "a default the fold cannot state must leave the whole declaration out"
        );
        assert_eq!(
            declared_ranges(&checked).len(),
            3,
            "the range is declared whatever the default says"
        );
    }

    /// Tests that unassigned nodes default to `Authority::Manual`.
    #[test]
    fn a_node_nobody_has_spoken_for_is_manual() {
        assert_eq!(Authority::default(), Authority::Manual);
        assert_eq!(
            Authority::ALL[0],
            Authority::default(),
            "the list should start where a node starts"
        );
    }

    /// Tests string representation round-trips for each authority level.
    #[test]
    fn every_authority_has_a_name_and_answers_to_it() {
        assert_eq!(Authority::Manual.name(), "manual");
        assert_eq!(Authority::Suggesting.name(), "suggesting");
        assert_eq!(Authority::Automatic.name(), "automatic");
        for level in Authority::ALL {
            assert_eq!(
                Authority::from_name(level.name()),
                Some(level),
                "`{}` does not read back as the level that wrote it",
                level.name()
            );
        }
        assert_eq!(
            Authority::from_name("man"),
            None,
            "the console's abbreviation is a surface's vocabulary, not a record's"
        );
    }

    /// Tests that wildcard parameter writes are refused when matched nodes have differing authorities.
    #[test]
    fn a_bare_name_is_refused_only_where_the_nodes_it_lands_on_disagree() {
        let uniform = [
            (Kind::L1, 0, Authority::Manual),
            (Kind::L4, 0, Authority::Manual),
            (Kind::L4, 1, Authority::Manual),
        ];
        assert_eq!(
            CrossesAuthority::over("radius", &uniform),
            None,
            "three nodes the operator kept are one arrangement, not a disagreement"
        );
        assert_eq!(
            CrossesAuthority::over("radius", &[]),
            None,
            "a key this Set declares nowhere lands on nothing, which is `no parameter \
             named` and not a refusal"
        );
        assert_eq!(
            CrossesAuthority::over("radius", &[(Kind::L4, 0, Authority::Automatic)]),
            None,
            "one node cannot disagree with itself, whatever it was granted to"
        );

        let refused = CrossesAuthority::over(
            "radius",
            &[
                (Kind::L1, 0, Authority::Manual),
                (Kind::L4, 0, Authority::Automatic),
            ],
        )
        .expect("a node the operator kept and a node an agent acts on are two arrangements");
        assert_eq!(refused.key, "radius");
        assert_eq!(
            refused.landing, "L1:0 manual, L4:0 automatic",
            "the refusal names every node the write lands on, with what each is under"
        );
        let sentence = refused.to_string();
        assert!(
            sentence.contains("L1:0 manual, L4:0 automatic") && sentence.contains("`radius`"),
            "the one sentence has to carry the control and the nodes that disagreed: \
             {sentence}"
        );
    }

    // The three that build a Set for real. The ones above check the layout arithmetic
    // `Set::build` would go on to use, and take no device — which is most of what
    // `cargo test -p karakuri-engine --lib -- --skip gpu::` is for.
    // See `../tests/gpu_tests_are_under_mod_gpu.rs`.
    mod gpu {
        use super::*;

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
    point_rate = 0.004;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
            let compile = |src: &str| -> Checked {
                let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
                let checked =
                    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
                karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
                checked
            };
            let l1 = compile(l1_src);
            let l4 = compile(l4_src);
            let set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 8, 1).expect("compatible pair");

            assert_eq!(
                set.live_count(&gpu.device, &gpu.queue),
                0,
                "a spawn-block procedure starts empty"
            );
        }
        /// Verifies that wildcard parameter writes across nodes with conflicting authorities
        /// are refused without mutating parameter values, while addressed writes succeed.
        #[test]
        fn a_wildcard_write_is_refused_where_the_nodes_it_lands_on_disagree() {
            let gpu = Gpu::headless().expect("no GPU available");
            let l1_src = r#"
proc probe_shared_l1 {
  kind     L1
  topology points
  capacity [4, 64] = 8

  param radius : float [0.0, 8.0] = 1.0

  emit position

  element {
    position = vec3(radius, 0.0, 0.0);
  }
}
"#;
            let l4_src = r#"
proc probe_shared_l4 {
  kind  L4
  blend additive

  param radius : float [0.0, 8.0] = 1.0

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = radius;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
            let compile = |src: &str| -> Checked {
                let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
                let checked =
                    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
                karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
                checked
            };
            let l1 = compile(l1_src);
            let l4 = compile(l4_src);
            let mut set =
                Set::build(&gpu.device, &gpu.queue, &l1, &l4, 8, 1).expect("compatible pair");
            let radius = |set: &Set| -> Vec<(Kind, u32, f32)> {
                let mut found: Vec<(Kind, u32, f32)> = set
                    .params()
                    .filter(|(_, _, key, _)| *key == "radius")
                    .map(|(layer, index, _, value)| (layer, index, value))
                    .collect();
                found.sort_by_key(|(layer, index, _)| (format!("{layer:?}"), *index));
                found
            };

            // Nobody has spoken for either node, so the bare name is one
            // control over one arrangement and reaches both.
            assert_eq!(
                set.write_param(&ParamWrite::everywhere("radius", 2.0)),
                Ok(2),
                "a Set nobody has spoken for is uniform, and a bare name reaches every \
                 declaration of the key"
            );
            assert_eq!(
                radius(&set),
                vec![
                    (Kind::L1, 0, 2.0),
                    // The built-in camera declares `radius` but is addressed-only
                    // ([`Set::addressed_only`]), so wildcard writes do not reach it.
                    (Kind::L3, 0, 8.0),
                    (Kind::L4, 0, 2.0),
                ],
                "both declarations should hold what the wildcard wrote, and the camera's own \
                 `radius` should be untouched by a bare name"
            );

            // One node handed to an agent, and the same control now spans two
            // arrangements.
            assert!(
                set.set_authority(Kind::L4, 0, Authority::Automatic),
                "the renderer is a node of this Set"
            );
            let refused = set
                .write_param(&ParamWrite::everywhere("radius", 7.0))
                .expect_err("a bare name over a kept node and a granted one is refused");
            assert_eq!(refused.key, "radius");
            assert_eq!(
                refused.landing, "L1:0 manual, L4:0 automatic",
                "the refusal names the nodes it would have landed on and what each is under"
            );
            assert_eq!(
                radius(&set),
                vec![(Kind::L1, 0, 2.0), (Kind::L3, 0, 8.0), (Kind::L4, 0, 2.0)],
                "a refused write moves nothing — landing on the permitted node is the \
                 silently partial control P-0094 rules out"
            );

            // And the reach is not what was taken away.
            assert_eq!(
                set.write_param(&ParamWrite::at(Kind::L4, 0, "radius", 7.0)),
                Ok(1),
                "an addressed write says which node it means, so it crosses nothing"
            );
            assert_eq!(
                radius(&set),
                vec![(Kind::L1, 0, 2.0), (Kind::L3, 0, 8.0), (Kind::L4, 0, 7.0)],
                "the addressed write lands on the node it names and on no other"
            );
        }

        /// Verifies that derived `Counts` in amplifier stages correctly scales all element counts
        /// including `survivors`, preserving count field invariants across pipeline stages.
        #[test]
        fn an_amplifiers_derived_counts_multiply_every_element_count_and_no_other_field() {
            // Was a silent `return` — the one test in the workspace that
            // reported success for having done nothing at all. Now the same
            // panic as the rest; a machine without an adapter uses
            // `--skip gpu::` rather than a test that lies to it.
            let gpu = Gpu::headless().expect("no GPU available");
            const FACTOR: u32 = 4;
            const CAPACITY: u32 = 64;

            let compile = |src: &str| -> Checked {
                let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
                let checked =
                    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
                karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
                checked
            };
            let l1 = compile(
                r#"
proc still {
  kind     L1
  topology points
  capacity [64, 64] = 64

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#,
            );
            let l2 = compile(
                r#"
proc mirror {
  kind    L2
  amplify 4

  consumes position

  deform {
    position = position + vec3(0.0, float(copy), 0.0);
  }
}
"#,
            );
            let l4 = compile(
                r#"
proc dots {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#,
            );
            let set = Set::build_many(
                &gpu.device,
                &gpu.queue,
                &[(&l1, CAPACITY)],
                &[&l2],
                &[],
                &[],
                &[&l4],
                Layering::Overdraw,
                1,
                &[],
                Wiring::default(),
            )
            .expect("a chain of one L1, one amplifying L2 and one L4");

            // `build_many` primes the chain, so the derived counts are current
            // without a step — which is the other thing this asserts.
            let read = |buf: &wgpu::Buffer| -> [u32; 12] {
                let size = karakuri_codegen::layout::counts::SIZE;
                let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("counts readback"),
                    size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                let mut encoder = gpu.device.create_command_encoder(&Default::default());
                encoder.copy_buffer_to_buffer(buf, 0, &readback, 0, size);
                gpu.queue.submit([encoder.finish()]);
                let slice = readback.slice(..);
                slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
                gpu.device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("poll");
                let data = slice.get_mapped_range().expect("map");
                let mut out = [0u32; 12];
                for (i, w) in data.chunks_exact(4).take(12).enumerate() {
                    out[i] = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
                }
                drop(data);
                readback.unmap();
                out
            };

            let source = &set.sources[0];
            let from = read(source.sim.counts());
            let derived = read(source.deforms[0].counts().expect("the node amplifies"));

            // Field order is `counts::WGSL`'s: elem_xyz, range, vertex_count,
            // instance_count, first_vertex, first_instance, survivors.
            assert_eq!(derived[3], from[3] * FACTOR, "range");
            assert_eq!(derived[5], from[5] * FACTOR, "instance_count");
            assert_eq!(derived[8], from[8] * FACTOR, "survivors");
            assert_eq!(
                derived[0],
                from[3] * FACTOR / karakuri_codegen::layout::WORKGROUP_SIZE,
                "workgroups, over the amplified range"
            );
            assert_eq!((derived[1], derived[2]), (1, 1), "the other two dimensions");

            // Verify non-element count fields (`vertex_count`, `first_vertex`, `first_instance`)
            // are unaffected by the amplifier factor.
            assert_eq!(derived[4], from[4], "vertex_count");
            assert_eq!(derived[6], from[6], "first_vertex");
            assert_eq!(derived[7], from[7], "first_instance");
        }
    }
}
