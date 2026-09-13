//! Type definitions, errors, and data structures for Set compilation and execution.

use std::collections::{HashMap, HashSet};

use karakuri_ir::layout::Synthetic;
use karakuri_ir::typed::{Checked, InputPort};
use karakuri_ir::Kind;

use crate::binding::Binding;
use crate::camera::Orbit;
use crate::mix::Input;
use crate::node::{Deform, Renderer, Simulation};
use crate::storage::{DeformStorage, SimulationStorage};

/// Maximum simulation substeps per frame before falling behind.
pub const MAX_STEPS: u8 = 4;

/// Default name assigned to the built-in orbit camera when unnamed.
pub const BUILTIN_CAMERA: &str = "orbit";

/// Fixed simulation time step in seconds (60 Hz).
pub const DT: f32 = 1.0 / 60.0;

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
pub(crate) enum Clock {
    /// Session master clock, used when on air.
    Session,
    /// Local Set simulation clock, used during pre-warming off air.
    Local,
}

/// Geometry source and associated deformation and rendering pipeline instances.
pub(crate) struct Source {
    /// Unique hash salt for this source instance.
    pub(crate) salt: u32,
    /// Indices into the Set's L1 procedure list.
    pub(crate) procedures: Vec<usize>,
    /// Primary L1 simulation node instance.
    pub(crate) sim: Simulation,
    /// Optional secondary simulation node for paired geometry deformations.
    pub(crate) paired: Option<Simulation>,
    /// Sequential L2 deformation nodes instantiated for this source.
    pub(crate) deforms: Vec<Deform>,
    /// L4 renderer nodes instantiated for this source.
    pub(crate) renderers: Vec<Renderer>,
}

pub struct Set {
    pub(crate) seed_salt: u32,
    /// Simulation steps elapsed. Time is `steps_taken * dt`.
    pub(crate) steps_taken: u64,
    /// Uncommitted simulation steps staged for the current frame.
    pub(crate) staged_delta: u64,
    pub(crate) staged_edges: Option<Vec<Input>>,
    pub(crate) dt: f32,
    /// Most recent beat count written to the L4 uniform block.
    pub(crate) last_beats: f32,
    pub(crate) viewport: [f32; 2],
    /// Indicates whether all procedures in the Set are closed-form functions of seed, t, and parameters.
    pub(crate) closed_form: bool,
    /// Indicates whether any procedure in the Set reads the ambient beat count.
    pub(crate) reads_beats: bool,

    /// Geometry sources and their associated pipeline chains.
    pub(crate) sources: Vec<Source>,
    /// Hash salts assigned to each geometry source, in L1 procedure order.
    pub(crate) source_salts: Vec<u32>,
    /// Declared capacity ranges `[min, max, default]` for each geometry source.
    pub(crate) declared_capacities: Vec<[u32; 3]>,
    /// Canonical names for each node in node order.
    pub(crate) names: Vec<String>,
    /// Authority level for each node in node order.
    pub(crate) authorities: Vec<Authority>,
    /// Number of distinct L1 procedures compiled into the Set.
    pub(crate) l1_count: usize,

    /// Manual parameter values per node.
    pub(crate) params: Vec<HashMap<String, f32>>,
    /// Typed parameter values keyed by canonical declaration name.
    pub(crate) param_values: Vec<HashMap<String, karakuri_store::record::Value>>,
    /// Parameter keys whose values have been explicitly modified from defaults.
    pub(crate) moved: Vec<HashSet<String>>,
    /// Declared `[min, max]` ranges for each parameter per node.
    pub(crate) ranges: Vec<HashMap<String, [f32; 2]>>,
    /// Bound on minimum primitive size for each L4 renderer procedure.
    pub(crate) rate_bounds: Vec<karakuri_ir::rate::RateBound>,
    /// Configured orbit camera parameters for the built-in camera.
    pub(crate) camera: Orbit,
    /// GPU camera nodes, ending with the built-in orbit camera.
    pub(crate) cameras: Vec<crate::node::Camera>,
    /// Optional L5 merge compositor.
    pub(crate) merge: Option<crate::node::Merge>,
    /// Composite input configurations for each renderer in draw order.
    pub(crate) edges: Vec<Input>,
    /// Active parameter signal bindings.
    pub(crate) bindings: Vec<Binding>,
    /// Controls published to the console interface.
    pub(crate) interface: Vec<Published>,
    /// Spliced field parameters under semantic names.
    pub(crate) field_params: Vec<String>,
    /// Spliced field parameter component names per field.
    pub(crate) field_declared: Vec<Vec<String>>,
    /// Bound Field slots: `(node_index, slot_name, field_ordinal)`.
    pub(crate) field_bound: Vec<(usize, String, usize)>,
    /// Bound Source slots: `(node_index, slot_name, geometry_ordinal)`.
    pub(crate) source_bound: Vec<(usize, String, usize)>,
    /// Number of distinct Field procedures in the Set.
    pub(crate) field_count: usize,
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
    pub(crate) fn over(key: &str, landing: &[(Kind, u32, Authority)]) -> Option<CrossesAuthority> {
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
    pub(crate) names: Vec<String>,
    /// Camera procedures, with `None` representing the default built-in camera.
    pub(crate) cameras: Vec<Option<&'a Checked>>,
    /// Range in `names` occupied by camera nodes.
    pub(crate) camera_range: std::ops::Range<usize>,
    /// Bound Field slots: `(node_index, slot_name, field_ordinal)`.
    pub(crate) field_bound: Vec<(usize, String, usize)>,
    /// Bound Camera slots: `(node_index, camera_ordinal)`.
    pub(crate) camera_bound: Vec<(usize, usize)>,
    /// Bound Source slots: `(node_index, slot_name, l1_index)`.
    pub(crate) source_bound: Vec<(usize, String, usize)>,
    /// Index into `l1s` of secondary geometry in a pairing configuration.
    pub(crate) far_at: Option<usize>,
    /// Indices of primary geometry chain heads.
    pub(crate) heads: Vec<usize>,
    /// Hash salts assigned to each geometry.
    pub(crate) source_salts: Vec<u32>,
    /// Synthesized attributes required for each chain head.
    pub(crate) derived: Vec<Vec<karakuri_ir::Attr>>,
    /// L1 procedures and requested element capacities.
    pub(crate) l1s: Vec<(&'a Checked, u32)>,
    /// L2 deformation procedures in chain order.
    pub(crate) l2s: Vec<&'a Checked>,
    /// Field procedures available for splicing.
    pub(crate) fields: Vec<&'a Checked>,
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
            let shader = karakuri_codegen::generate_l1(l1, derived, &bound_at(at));
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
                let shader = karakuri_codegen::generate_l2(
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
