use karakuri_engine::camera::Orbit;
use karakuri_engine::set::Layering;
use karakuri_engine::{Binding, ParamWrite};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;
use karakuri_store::hash::Hash;

use crate::compile::Names;

/// The Set file format version this build writes. One number for the whole
/// file, on `Record::Set`.
pub const VERSION: u32 = 1;

/// The octave count an `fbm` binding gets when it does not say. Matches
/// `karakuri-store`'s `BindNoise` default, which is the record this stands for.
pub const DEFAULT_OCTAVES: u32 = 4;

/// Set definition decoded into engine layer collections.
#[cfg_attr(test, derive(Debug))]
pub struct Loaded {
    pub id: String,
    /// The geometry sources, by `slot` index. At least one; several is one Set
    /// simulating several times and drawing all of them.
    pub l1s: Vec<Checked>,
    /// The deformers, by `slot` index — which is chain order, each one reading what
    /// the one before it wrote.
    pub l2s: Vec<Checked>,
    /// Cameras by slot index, or empty if using the built-in orbit.
    pub l3s: Vec<Checked>,
    /// The fields, by `slot` index. Empty for a Set that evaluates none.
    pub fields: Vec<Checked>,
    /// The renderers, in the order their `slot` records indexed them.
    pub l4s: Vec<Checked>,
    /// Source procedure code in node order, excluding built-in cameras.
    pub srcs: Vec<String>,
    /// Vertex/point capacities per geometry slot, or `None` if engine default.
    pub capacities: Vec<Option<u32>>,
    pub params: Vec<ParamWrite>,
    pub bindings: Vec<Binding>,
    /// Explicit node names by layer and slot index, preserved for wiring resolution.
    pub names: Names,
    /// Which node fills each declared input slot, as the file recorded it.
    pub edges: Vec<karakuri_engine::set::Edge>,
    pub camera: Option<Orbit>,
    /// Layering mode: [`Layering::Composite`] if `merge` record present, else [`Layering::Overdraw`].
    pub layering: Layering,
    /// Active solo renderer index under compositing, or `None` if all live or overdrawing.
    pub live: Option<u32>,
    /// Per-geometry salt seeds, or `None` if derived from Set seed and ordinal.
    pub salts: Vec<Option<u32>>,
    /// Non-fatal decoding diagnostics and unsupported feature notices.
    pub notes: Vec<String>,
}

impl Loaded {
    /// Returns node names in canonical execution order, padded with `None` where unnamed.
    pub fn node_names(&self) -> impl Iterator<Item = Option<String>> + '_ {
        let at = |v: &[Option<String>], n: usize| -> Vec<Option<String>> {
            let mut out = v.to_vec();
            out.resize(n, None);
            out
        };
        at(&self.names.l1s, self.l1s.len())
            .into_iter()
            .chain(at(&self.names.l2s, self.l2s.len()))
            .chain(at(&self.names.l3s, self.l3s.len()))
            .chain(at(&self.names.l4s, self.l4s.len()))
            .chain(at(&self.names.fields, self.fields.len()))
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&Checked, &str)> {
        self.l1s
            .iter()
            .chain(&self.l2s)
            .chain(&self.l3s)
            .chain(&self.l4s)
            .chain(&self.fields)
            .zip(self.srcs.iter().map(String::as_str))
    }
}

/// Persisted node descriptor specifying artifact hash, layer kind, slot index, and optional name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// The store's address for this node's source. Already `put`, because a hash
    /// nothing has stored is a Set file that does not load.
    pub hash: Hash,
    pub layer: Kind,
    /// Which node of that layer, numbered from 0 with no gaps. Index is position:
    /// the L2s deform in it and the L4s draw in it.
    pub index: u32,
    /// Custom name assigned to the node, or `None`.
    pub name: Option<String>,
}

impl crate::compile::Placed {
    /// This node as [`Node`]. No store and no disk — the address comes off the
    /// bytes the compile read.
    pub fn node(&self) -> Node {
        Node {
            hash: self.hash(),
            layer: self.layer,
            index: self.index,
            name: self.named.name.clone(),
        }
    }
}

/// Everything a Set file records, gathered so [`save`] takes one argument for
/// the Set rather than nine for its parts. The fields are the records, in the
/// order they are written.
pub struct Saving<'a> {
    /// Every node of the Set, in any order — [`save`] writes them by layer and
    /// index, so the file is the same bytes however the caller gathered them.
    pub nodes: &'a [Node],
    /// Capacity overrides per geometry slot in index order.
    pub capacities: &'a [u32],
    pub params: &'a [ParamWrite],
    pub bindings: &'a [Binding],
    /// Which node fills each declared input slot, exactly as the run was wired.
    /// Empty for a Set no node of which takes a second geometry, which is most of
    /// them.
    pub edges: &'a [karakuri_engine::set::Edge],
    /// Active orbit camera state at save time.
    pub camera: &'a Orbit,
    /// Whether this Set composites its renderers or overdraws them.
    ///
    /// [`Layering::Composite`] serializes as a `merge` record; [`Layering::Overdraw`]
    /// is the default and omits the record.
    pub layering: Layering,
    /// Solo renderer index under composite layering, or `None`.
    pub live: Option<u32>,
    /// Salt seeds per geometry slot in index order.
    pub seeds: &'a [u32],
}

/// Thread-safe, fully owned representation of [`Saving`] for cross-thread persistence.
pub struct Owned {
    pub nodes: Vec<Node>,
    pub capacities: Vec<u32>,
    pub params: Vec<ParamWrite>,
    pub bindings: Vec<Binding>,
    pub edges: Vec<karakuri_engine::set::Edge>,
    pub camera: Orbit,
    /// See [`Saving::layering`].
    pub layering: Layering,
    /// See [`Saving::live`].
    pub live: Option<u32>,
    pub seeds: Vec<u32>,
}

impl Owned {
    /// What [`save`] takes, borrowed out of this.
    pub fn saving(&self) -> Saving<'_> {
        Saving {
            nodes: &self.nodes,
            capacities: &self.capacities,
            params: &self.params,
            bindings: &self.bindings,
            edges: &self.edges,
            camera: &self.camera,
            layering: self.layering,
            live: self.live,
            seeds: &self.seeds,
        }
    }
}
