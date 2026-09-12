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

/// What a Set file said, in the terms the engine takes.
///
/// **Per layer, which is the shape `Set::build_many` takes its nodes in.** A
/// Set file records a `slot` per node and the layer is what the node's own
/// `kind` declared, so a chain comes back as a chain rather than as a pile the
/// loader has to classify a second time.
#[cfg_attr(test, derive(Debug))]
pub struct Loaded {
    pub id: String,
    /// The geometry sources, by `slot` index. At least one; several is one Set
    /// simulating several times and drawing all of them.
    pub l1s: Vec<Checked>,
    /// The deformers, by `slot` index — which is chain order, each one reading
    /// what the one before it wrote.
    pub l2s: Vec<Checked>,
    /// **The cameras, by `slot` index.** Empty for a Set that looks from the
    /// built-in orbit — which is a node all the same, so such a Set still holds
    /// one camera and still addresses it at `L3:0`.
    ///
    /// **Several `slot` records on L3 is how a file says so**, which the format
    /// already allowed: this loader used to read the first and report the rest
    /// as skipped, because the engine took one. A file written before this
    /// commit names at most one and reads back unchanged.
    pub l3s: Vec<Checked>,
    /// **The fields, by `slot` index.** Empty for a Set that evaluates none.
    /// **Several `slot` records on Field is how a file says so**, which the
    /// format already allowed — nothing new had to be added, and a file written
    /// before this commit still names exactly one.
    pub fields: Vec<Checked>,
    /// The renderers, in the order their `slot` records indexed them — which is
    /// draw order. **Several `slot` records on L4 is how a file says a stack**;
    /// the format already allowed it and nothing new had to be added.
    pub l4s: Vec<Checked>,
    /// Every procedure as text, in **node order** — the L1s, the L2s, the L3s,
    /// the renderers, then the fields, which is the order `Set::node_names`
    /// reports and the order [`Loaded::nodes`] walks.
    ///
    /// **The built-in camera has no entry**, because it has no source: a Set
    /// that declares no L3 holds a camera node with no procedure behind it, so
    /// this list is one shorter than that Set's node names. See
    /// [`Loaded::node_names`].
    ///
    /// **Carried because a Set file has no `.kir` on disk and an editable run
    /// needs one.** A Set names its procedures by hash; the sources come out of
    /// the store, or out of the file when it was bundled. Before the scratch
    /// existed there was nowhere to put them and `--mcp` with `--load-set` was
    /// refused for exactly that reason. See `scratch::place`.
    pub srcs: Vec<String>,
    /// What each geometry runs at, by `slot` index, one entry per L1.
    ///
    /// `None` where the file gave no `capacity` record for that geometry, which
    /// means the `.kir` default applies — the spec's own wording. **Per
    /// geometry rather than per Set**, because each source declares its own
    /// range and one number cannot serve two of them; the format has keyed it
    /// by node since the address existed.
    pub capacities: Vec<Option<u32>>,
    pub params: Vec<ParamWrite>,
    pub bindings: Vec<Binding>,
    /// **What each node the file named is called**, in the same per-layer shape
    /// the procedures come back in, and `None` for a node the file left
    /// unnamed.
    ///
    /// Read and carried rather than read and reported. Nothing used to point at
    /// a node by name, so a name was noted as unhonoured and dropped; an `edge`
    /// points at two of them, so a loaded Set whose names were dropped is a
    /// loaded Set whose edges cannot resolve.
    pub names: Names,
    /// **Which node fills each declared input slot**, as the file recorded it.
    pub edges: Vec<karakuri_engine::set::Edge>,
    pub camera: Option<Orbit>,
    /// **Whether this Set composites its renderers or overdraws them**, as the
    /// file said.
    ///
    /// [`Layering::Composite`] for a file carrying a `merge` record and
    /// [`Layering::Overdraw`] for one that does not — which is every file
    /// written before the record existed, and is what overdrawing has always
    /// been recorded as: its absence. See [`Record::Merge`].
    pub layering: Layering,
    /// **Which renderer the file left selected**, in draw order, and `None`
    /// where every input is live.
    ///
    /// `None` is not renderer 0 — see [`Record::Merge`]'s `live`, where the
    /// whole of that argument lives: a Set nobody selected in writes no `live`
    /// and comes back with every renderer folded, which is the state it was
    /// saved in.
    ///
    /// **Always `None` under [`Layering::Overdraw`]**, because the only record
    /// that can carry a selection is the one that says the Set composites.
    pub live: Option<u32>,
    /// **What each geometry was salted with**, by `slot` index, one entry per
    /// `seed` record the file carried.
    ///
    /// `None` where the file named no seed for that geometry — an older file
    /// that recorded one salt for the whole Set, or none at all — and the
    /// engine then derives that source's from the Set's seed and its ordinal.
    /// **The first entry is also the Set's own seed**, which is what an L3
    /// reads and what an unsalted source is derived from: one number in one
    /// place rather than a `seed` field beside a `salts` field, disagreeing.
    pub salts: Vec<Option<u32>>,
    /// **What could not be carried across, in the operator's words.**
    ///
    /// Not warnings to be counted and not errors: a Set file that mentions a
    /// second layer's seed is a valid file this engine cannot honour in full,
    /// and the honest response is to load it and say so. Silence here would be
    /// the load succeeding and the material being subtly not what was saved.
    pub notes: Vec<String>,
}

impl Loaded {
    /// Every node, compiled and as text, in node order.
    ///
    /// **The one place the two halves are walked together.** `srcs` is a flat
    /// list and the procedures are per layer, so pairing them anywhere else
    /// would be a second copy of what node order is — and a caller that got it
    /// wrong would write one node's source into another node's file. See
    /// `scratch::place`, which is what wants the pairing.
    /// What each node is called, in the same node order [`Loaded::nodes`]
    /// walks, and `None` for one the file left unnamed.
    ///
    /// **For the one caller that has node order and not layers**: a Set file
    /// materialised into the scratch becomes a flat `--set` list, and the name
    /// has to travel with the path it is written beside. Everything else takes
    /// its names per layer, which is the shape `Set::build_many` wants.
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

/// One node of a Set on its way into a file: **the content address its source
/// is already stored under**, which layer its `kind` declaration puts it on,
/// which node of that layer it is, and what the operator called it.
///
/// **The layer is read where the chain was sorted, not worked out again here.**
/// [`crate::compile::sort_compiled`] already sorts a `--set` list by the `kind`
/// each file declares —
/// that is how the engine gets its nodes — so asking the same question a second
/// time is how a Set file comes to disagree with the run it was saved from.
/// A `slot` record is exactly this, which is why the fields are these four.
///
/// **A hash and not a path, and moving `put_artifact` out to the callers is the
/// point of the change.** The writer's job is to write records; where the bytes
/// came from is the caller's, and the two callers have genuinely different
/// answers. A one-shot `--save-set` holds paths that are still true, so it
/// reads them and puts them. A live save holds the hashes the *watcher* stored
/// when the build it is playing landed — and re-reading those paths would
/// record whatever is on disk now, which after a rolled-back build is a version
/// that is not on screen. A writer that read files could only ever have served
/// the first of those.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// The store's address for this node's source. Already `put`, because a
    /// hash nothing has stored is a Set file that does not load.
    pub hash: Hash,
    pub layer: Kind,
    /// Which node of that layer, numbered from 0 with no gaps. Index is
    /// position: the L2s deform in it and the L4s draw in it.
    pub index: u32,
    /// `None` for a path written bare, which is the ordinary case — a name is a
    /// cost paid when something wants to point at the node. See `Named`.
    ///
    /// **Owned, unlike everything else [`Saving`] borrows.** A live save
    /// gathers on the render thread and writes on another one, so this value
    /// has to be able to outlive the frame that read it; the writer copies it
    /// into the record either way, so owning it costs a save one allocation
    /// per node and buys a whole borrow-free [`Owned`].
    pub name: Option<String>,
}

impl crate::compile::Placed {
    /// This node as [`Node`]. **No store and no disk** — the address
    /// comes off the bytes the compile read.
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
    /// What each geometry runs at, one per L1 node in index order.
    ///
    /// **Per geometry, because the record is.** A Set holds several sources,
    /// each with its own declared range, and a single number written against
    /// node 0 was the only thing this could say before the address existed.
    pub capacities: &'a [u32],
    pub params: &'a [ParamWrite],
    pub bindings: &'a [Binding],
    /// **Which node fills each declared input slot**, exactly as the run was
    /// wired. Empty for a Set no node of which takes a second geometry, which
    /// is most of them.
    pub edges: &'a [karakuri_engine::set::Edge],
    /// **The built-in camera as it is now, which is `Set::orbit`** and not the
    /// `Set::camera` field beside it: three of the six are that node's
    /// parameters, so the field holds what was last stated and the map holds
    /// what a hand moved. A caller that passes the field saves a camera nobody
    /// is looking through (ADR-0318).
    pub camera: &'a Orbit,
    /// **Whether this Set composites its renderers or overdraws them.**
    ///
    /// [`Layering::Composite`] writes a `merge` record and
    /// [`Layering::Overdraw`] writes nothing at all: the record's presence is
    /// the whole statement, so a Set that overdraws is a file with no line to
    /// say so — which is what keeps every file written before the record
    /// existed byte for byte the file it was.
    pub layering: Layering,
    /// **Which renderer is the only live one**, in draw order, and `None` where
    /// every one of them is — the state a Set nobody has selected in is in.
    ///
    /// **Read only under [`Layering::Composite`]**, because it rides the record
    /// that says so. An overdrawing Set has edges like any other and nothing
    /// reads them — `Set::select_renderer` is silently ineffective there — so a
    /// selection made on one is a property of the run with nothing in the file
    /// for it to be about, exactly as `Record::Select` has always been.
    pub live: Option<u32>,
    /// **What each geometry is salted with**, one per L1 node in index order.
    ///
    /// **Per geometry, because the record is.** `seed` carries a stream and an
    /// index, so a Set holding two grids records the salt each one is running
    /// at — and comes back with the colours it had whichever order the paths
    /// were spelled in. A Set of one geometry writes the one line it always
    /// wrote: index 0 is absent from the record, so the bytes do not move.
    pub seeds: &'a [u32],
}

/// [`Saving`] with every part owned: the same nine facts, gathered where they
/// live and able to leave the thread that gathered them.
///
/// **It exists because a live save is two threads.** The values are read off
/// the running deck, which only the render thread may touch, and the store
/// write must not happen on a frame — so what crosses between them cannot be a
/// bundle of borrows into a `Set`. Everything here is a `Vec` or a `Copy` of
/// what [`Saving`] points at, which is also why it is this type and not a
/// second writer: [`saving`](Owned::saving) hands the borrows back and the one
/// function that knows the file format stays the one function.
///
/// **Not what `--save-set` uses**, and deliberately not made to be: that path
/// has every value in hand on one thread with nothing to outlive, and copying
/// them to write them would be a cost paid for nothing.
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
