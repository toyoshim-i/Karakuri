//! Set files: the material, through the record stream at last.
//!
//! Everything else an operator moves — audio, tempo, the mix, the transport —
//! reaches the engine as a record. The *material* did not: two `.kir` paths and
//! a handful of flags went straight into `Set::build`, so `README.md`'s
//! invariant had to say "the performance is on the record path and the material
//! is not". This is the other half.
//!
//! Two directions, and both are needed or neither is worth anything:
//!
//! - [`save`] writes a Set file that references every node's source by hash,
//!   with the capacities, parameters, bindings, camera and seed the run was
//!   using. Getting the bytes into the store is the caller's — see [`Node`].
//! - [`load`] reads one back and returns everything `Set::build_many` and the
//!   flags used to supply.
//!
//! ## The whole chain, not an L1 and its renderers
//!
//! Both halves handled a pair and then a stack: an L1, and the L4s drawn over
//! it. Everything else a `--set` can spell — the L2s that deform, an L3 that
//! looks, a `kind Field` that shapes — was refused by [`save`] and skipped with
//! a note by [`load`], so a cube morphing into a sphere was a Set that could be
//! played and could not be kept, and `--record-session` refused it for the same
//! reason, since a session opens with a Set file.
//!
//! **Nothing in the format had to change to close that.** A `slot` record has
//! carried a layer, an index and a name since the address existed; what was
//! missing was a writer that put a node's own `kind` into it and a reader that
//! honoured the index on every layer rather than on one.
//!
//! ## Where the flag went
//!
//! `--bind`'s fields are `Record::Bind`'s fields, and `docs/roadmap.md` records
//! the debt that came with that: **two diagnostics guarding the flag — a `bpm`
//! binding, and `noise.octaves` on a kind that has no octaves — lived only in
//! the flag, and the decoder owed them too.** Paying that by writing them a
//! second time would be two copies of a rule that must not differ.
//!
//! So the flag is now what its documentation always claimed: **a way to write
//! the record**. `parse_bind` turns a `--bind` string into a [`Record::Bind`]
//! and hands it to [`binding_from_record`], which is where every semantic check
//! lives. One rule, one place, and a Set file and a command line cannot disagree
//! about what a binding means.
//!
//! ## One place the format is still finer than the engine
//!
//! A `param` may be a vector, and the engine's map holds `f32`, so a vector
//! write is reported rather than carried.
//!
//! Capacity was a second and `seed` a third, and both are closed the same way:
//! each is keyed by node, the engine now holds one per geometry, and so each is
//! carried rather than reported. What a recorded seed buys is more than the
//! symmetry — a salt derived from a source's position in `--set` moves when the
//! list is reordered, and one read back from a file does not.
//!
//! None of that is resolved here and none of it is silently dropped. Loading
//! reports what it could not carry — see [`Loaded::notes`] — because a Set file
//! that half-applies is the failure mode this repository keeps refusing:
//! checking clean and coming up short later. The gaps themselves are the
//! format's and the engine's to settle, and `Set::build` refusing a colliding
//! param name is the same disagreement seen from the other side.

use std::collections::BTreeMap;

use karakuri_engine::binding::{Curve, NOISE_SIGNAL};
use karakuri_engine::camera::Orbit;
use karakuri_engine::set::Layering;
use karakuri_engine::{Binding, ParamWrite};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;
use karakuri_signal::{NoiseConfig, NoiseKind};
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{BindNoise, Layer, Record, Value};
use karakuri_store::store::{Store, StoreError};

/// The Set file format version this build writes. One number for the whole
/// file, on `Record::Set`.
const VERSION: u32 = 1;

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
    pub names: crate::Names,
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

/// Convert one [`Record::Bind`] into the binding the engine applies.
///
/// **The one place a binding's semantics live.** `--bind` reaches here too, so
/// the flag and a Set file cannot mean different things by the same fields.
///
/// The two diagnostics `docs/roadmap.md` says the decoder owes are here and
/// nowhere else:
///
/// - **`signal=bpm` is refused.** A tempo is not a `[0, 1]` signal, so the
///   curve clamps it and the binding sits pinned at the top of its range for
///   the whole run. From the outside a pinned binding and a working one are the
///   same number on a status line, which is exactly why this cannot be a
///   silent clamp. `beat` and `bar` carry the same tempo in the range a binding
///   is defined over.
/// - **`noise.octaves` needs `kind=fbm`.** The other three generators have no
///   layers, so an octave count on one of them is asking for a generator nobody
///   named. Refused rather than ignored, for the same reason.
///
/// And one more that is the same shape: `noise` on a binding whose signal is
/// not `noise` is refused, because accepting it leaves an operator re-reading
/// the noise fields to find out why the parameter does not move.
pub fn binding_from_record(record: &Record) -> Result<Binding, String> {
    let Record::Bind {
        layer,
        index,
        key,
        signal,
        curve,
        range,
        noise,
    } = record
    else {
        return Err("not a `bind` record".to_string());
    };

    let bad = |what: String| format!("bind {}={key}: {what}", layer_name(*layer));

    let kind = kind_of(*layer);
    let curve = Curve::parse(curve).ok_or_else(|| {
        bad(format!(
            "curve `{curve}` — expected lin, pow2, sqrt or smooth"
        ))
    })?;

    if signal == "bpm" {
        return Err(bad(
            "signal `bpm` — a tempo is not a [0, 1] signal, so the curve clamps it and this \
             binding would sit at the top of its range for the whole run; bind `beat` or \
             `bar` instead"
                .to_string(),
        ));
    }

    let mut binding = Binding::new(kind, key.clone(), signal.clone(), curve, *range);
    // Absent stays absent: a binding with no `index` is the layer's, every node
    // declaring the key — see `Binding::index`.
    if let Some(at) = index {
        binding = binding.at(*at);
    }
    if signal != NOISE_SIGNAL {
        if noise.is_some() {
            return Err(bad(format!(
                "a generator needs `signal={NOISE_SIGNAL}`, and this binds `{signal}`"
            )));
        }
        return Ok(binding);
    }
    // **Absent means the default generator, not the absence of one**, which is
    // what `Record::Bind::noise` says and the only thing the name can mean: a
    // binding to `noise` with nothing else said is a binding to the default
    // generator. Materialised here rather than left as `None` so that what the
    // binding carries is what it will use.
    let noise = noise.clone().unwrap_or_default();
    let noise = &noise;
    // Read before the kind is folded, because `NoiseKind` carries the octave
    // count inside the `fbm` variant: once folded there is nothing left to
    // check against, and an octave count on a `white` would have turned it into
    // an `fbm` on the way past.
    let octaves_named = noise.octaves != DEFAULT_OCTAVES;
    if octaves_named && noise.kind != "fbm" {
        return Err(bad(format!(
            "`octaves` needs kind `fbm`, and this asks for `{}`",
            noise.kind
        )));
    }
    let kind = match noise.kind.as_str() {
        "white" => NoiseKind::White,
        "value" => NoiseKind::Value,
        "perlin" => NoiseKind::Perlin,
        "fbm" => NoiseKind::Fbm {
            octaves: noise.octaves,
        },
        other => {
            return Err(bad(format!(
                "noise kind `{other}` — expected white, value, perlin or fbm"
            )))
        }
    };
    Ok(binding.with_noise(NoiseConfig {
        kind,
        rate: noise.rate,
        stream: noise.stream,
    }))
}

/// A written param's fold key: its address, then its name. `None` sorts first,
/// which puts the Set-wide value above the narrower ones that override it.
type ParamKey<'a> = (Option<(u8, u32)>, &'a str);

/// `Layer` as a number, so an address can sort. In node order, which is the
/// order a Set holds them in.
fn layer_ordinal(layer: Kind) -> u8 {
    match layer {
        Kind::L1 => 0,
        Kind::L2 => 1,
        Kind::L3 => 2,
        Kind::L4 => 3,
        // Last, matching `Set::slot_of`.
        Kind::Field => 4,
    }
}

fn layer_from_ordinal(n: u8) -> Layer {
    match n {
        0 => Layer::L1,
        1 => Layer::L2,
        2 => Layer::L3,
        3 => Layer::L4,
        // **Not the `_` arm.** `Field` used to fall into `L4`'s catch-all, so a
        // `--param Field:0:x` was saved as `L4:0:x` and reloaded onto renderer
        // zero — silently dropped if that renderer had no such name, and
        // silently wrong if it did.
        _ => Layer::Field,
    }
}

/// The engine `Kind` a record `Layer` names.
///
/// **Total, now that every layer a record can name is a node a Set can hold.**
/// It returned an `Option` while L2 and L3 were record-only, and the three
/// callers that unwrapped it each carried a "this engine builds L1 and L4 only"
/// message. Those messages were true when they were written and became wrong
/// silently, which is what a total function here prevents happening again.
fn kind_of(layer: Layer) -> Kind {
    match layer {
        Layer::L1 => Kind::L1,
        Layer::L2 => Kind::L2,
        Layer::L3 => Kind::L3,
        Layer::L4 => Kind::L4,
        Layer::Field => Kind::Field,
    }
}

/// The record `Layer` an engine [`Kind`] names. The inverse of [`kind_of`], and
/// total for the same reason: every layer a Set can hold is a layer a record
/// can address.
pub(crate) fn layer_of(kind: Kind) -> Layer {
    match kind {
        Kind::L1 => Layer::L1,
        Kind::L2 => Layer::L2,
        Kind::L3 => Layer::L3,
        Kind::L4 => Layer::L4,
        Kind::Field => Layer::Field,
    }
}

/// The record a binding is. The inverse of [`binding_from_record`], and what
/// [`save`] writes.
pub fn record_from_binding(binding: &Binding) -> Record {
    Record::Bind {
        layer: layer_of(binding.layer),
        index: binding.index,
        key: binding.key.clone(),
        signal: binding.signal.clone(),
        curve: binding.curve.name().to_string(),
        range: binding.range,
        noise: binding.noise.map(|n| BindNoise {
            kind: match n.kind {
                NoiseKind::White => "white".to_string(),
                NoiseKind::Value => "value".to_string(),
                NoiseKind::Perlin => "perlin".to_string(),
                NoiseKind::Fbm { .. } => "fbm".to_string(),
            },
            rate: n.rate,
            stream: n.stream,
            octaves: match n.kind {
                NoiseKind::Fbm { octaves } => octaves,
                _ => DEFAULT_OCTAVES,
            },
        }),
    }
}

/// A record [`Layer`] spelled the way every surface spells it. `pub(crate)`
/// because `--list-sets` names a node's layer in a line, and a second table in
/// `main.rs` would be a second spelling of an address an operator then types.
pub(crate) fn layer_name(layer: Layer) -> &'static str {
    match layer {
        Layer::L1 => "L1",
        Layer::L2 => "L2",
        Layer::L3 => "L3",
        Layer::L4 => "L4",
        Layer::Field => "Field",
    }
}

/// One node of a Set on its way into a file: **the content address its source
/// is already stored under**, which layer its `kind` declaration puts it on,
/// which node of that layer it is, and what the operator called it.
///
/// **The layer is read where the chain was sorted, not worked out again here.**
/// `main.rs` already sorts a `--set` list by the `kind` each file declares —
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

/// **Refused rather than written into a file that cannot be read back.**
///
/// This used to refuse an L2, an L3 or a `kind Field` outright, because a Set
/// file recorded "the L1, and every other path": everything else went out as an
/// `L4` slot, and reloading as `slot L4 needs a L4 procedure, got Field` was the
/// *good* case — the bad one was an L2 whose `deform` the loader handed to a
/// renderer. The format has always had a slot per layer, so a chain is now
/// written as the chain it is and none of that is left to refuse.
///
/// What is left is what a Set *is*, and what the projection can fold. A file
/// with no geometry or nothing to draw describes no Set; two nodes at one
/// address fold to one node — see `key_for` in `project.rs` — and a gap in an
/// index describes a chain with a hole in it, which [`from_lines`] refuses on
/// the way back in rather than closing up.
fn refuse_unwritable(nodes: &[Node], capacities: &[u32], seeds: &[u32]) -> Result<(), String> {
    let count = |layer: Kind| nodes.iter().filter(|n| n.layer == layer).count();
    let geometries = count(Kind::L1);
    if geometries == 0 {
        return Err(
            "none of these files declares `kind L1`, and a Set is a geometry and the nodes \
             over it"
                .to_string(),
        );
    }
    if count(Kind::L4) == 0 {
        return Err(
            "none of these files declares `kind L4`, and a Set with no renderer has no frame \
             to give"
                .to_string(),
        );
    }
    if capacities.len() != geometries {
        return Err(format!(
            "{geometries} geometr{} and {} capacit{} — a capacity sizes one geometry, so \
             there is exactly one per L1 node",
            if geometries == 1 { "y" } else { "ies" },
            capacities.len(),
            if capacities.len() == 1 { "y" } else { "ies" },
        ));
    }
    // The same shape as the capacities, and for the same reason: a salt
    // randomises one geometry, so a file that carried a different number of
    // them would be a file where which grid is which colour depends on how a
    // reader lines two lists up.
    if seeds.len() != geometries {
        return Err(format!(
            "{geometries} geometr{} and {} seed{} — a seed salts one geometry, so \
             there is exactly one per L1 node",
            if geometries == 1 { "y" } else { "ies" },
            seeds.len(),
            if seeds.len() == 1 { "" } else { "s" },
        ));
    }
    for layer in [Kind::L1, Kind::L2, Kind::L3, Kind::L4, Kind::Field] {
        let mut indices: Vec<u32> = nodes
            .iter()
            .filter(|n| n.layer == layer)
            .map(|n| n.index)
            .collect();
        indices.sort_unstable();
        if let Some((at, index)) = indices
            .iter()
            .enumerate()
            .find(|(at, index)| **index != *at as u32)
        {
            return Err(format!(
                "the {} nodes are numbered {indices:?}: index {index} is the {}, and a \
                 layer's nodes run from 0 with no gaps and no repeats",
                kind_name(layer),
                match *index < at as u32 {
                    true =>
                        "second node at that address, which is one node in the file that \
                             comes back",
                    false => "far side of a gap, which is a chain with a hole in it",
                },
            ));
        }
    }
    Ok(())
}

/// The name a `kind` declaration uses.
///
/// **Shared with the watcher**, which spells a layer the same way into the edit
/// history and into a session's `procedure` records. A second table would be a
/// second spelling, and a snapshot filed under one and addressed by the other is
/// a version an operator cannot walk back to.
pub(crate) fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::L1 => "L1",
        Kind::L2 => "L2",
        Kind::L3 => "L3",
        Kind::L4 => "L4",
        Kind::Field => "Field",
    }
}

/// **Write a Set file.**
///
/// The file references its sources by hash rather than carrying them, so a Set
/// file is a few dozen lines a human can read rather than a copy of the
/// material. Content addressing means saving the same procedure twice stores it
/// once.
///
/// **Putting them there is the caller's** — see [`Node`] for why the writer
/// stopped reading files. What this owes is that every hash it writes was
/// already stored, and it cannot check that without reading the store back,
/// which is the caller's promise instead.
pub fn save(store: &Store, id: &str, set: Saving<'_>) -> Result<(), String> {
    let Saving {
        nodes,
        capacities,
        params,
        bindings,
        edges,
        camera,
        layering,
        live,
        seeds,
    } = set;
    refuse_unwritable(nodes, capacities, seeds)?;
    let mut lines = vec![Line::new(Record::Set {
        id: id.to_string(),
        v: VERSION,
    })];
    // **One `slot` record per node, in node order** — the geometries, the
    // deformers, the camera, the renderers, then the field. Several on a layer
    // is how the format says a stack or a chain, and it needed no new record to
    // say it: a file holding one geometry and one renderer is the two lines it
    // always was, in the order it always had them.
    //
    // Sorted here rather than trusted from the caller, so that the file is a
    // function of the Set rather than of the order somebody walked it in.
    let mut ordered: Vec<&Node> = nodes.iter().collect();
    ordered.sort_by_key(|n| (layer_ordinal(n.layer), n.index));
    for node in ordered {
        lines.push(Line::new(Record::Slot {
            layer: layer_of(node.layer),
            index: node.index,
            // **Written only where the operator wrote one.** A name belongs to
            // the use rather than to the procedure, so a bare path has none to
            // record, and an absent name is written as nothing — which is what
            // keeps a file this build saves byte for byte the file it saved
            // before the field existed.
            name: node.name.clone(),
            proc_hash: node.hash,
        }));
    }
    // On L1, because that is the layer whose element buffers a capacity sizes,
    // and one per geometry, because that is what it sizes: each source declares
    // its own range and one number cannot serve two of them.
    for (index, value) in capacities.iter().enumerate() {
        lines.push(Line::new(Record::Capacity {
            layer: Layer::L1,
            index: index as u32,
            value: *value,
        }));
    }
    // Sorted, so saving the same state twice produces the same file. A
    // `HashMap`'s order is not a property anything should depend on, and a Set
    // file that differed run to run would make every diff meaningless.
    // Keyed by the address as well as the name, so a wildcard write and a
    // write addressed at one node are two lines rather than one overwriting the
    // other. `None` sorts first, which puts the Set-wide value above the
    // narrower ones that override it — the order a reader wants.
    let ordered: BTreeMap<ParamKey<'_>, f32> = params
        .iter()
        .map(|w| {
            let at = w.at.map(|(layer, i)| (layer_ordinal(layer), i));
            ((at, w.key.as_str()), w.value)
        })
        .collect();
    for ((at, key), value) in ordered {
        lines.push(Line::new(Record::Param {
            // **`layer` is load-bearing exactly when `index` is beside it.** It
            // was a placeholder before the address existed — written as `L1` on
            // everything and ignored on read — so an unaddressed write still
            // says `L1` and still means every node declaring the name. See
            // `Record::Param`.
            layer: at.map_or(Layer::L1, |(l, _)| layer_from_ordinal(l)),
            index: at.map(|(_, i)| i),
            key: key.to_string(),
            value: Value::Scalar(value),
        }));
    }
    for binding in bindings {
        lines.push(Line::new(record_from_binding(binding)));
    }
    // **After the slots, because an edge is written in terms of what they
    // name.** A reader that has met every `slot` record has every name in hand
    // — including the ones nobody wrote, which are derived from the procedure
    // and so are a function of the artifact the slot references.
    //
    // Written as given rather than resolved to addresses. An edge is between
    // *names* and that is the whole of why it exists: an address moves when the
    // list is reordered, which is the failure `--set` position 1 was.
    for edge in edges {
        lines.push(Line::new(Record::Edge {
            node: edge.node.clone(),
            slot: edge.slot.clone(),
            to: edge.to.clone(),
        }));
    }
    // **Written only where the Set composites, because the record's presence
    // is the whole statement.** A Set that overdraws writes no line here: there
    // is no boolean field for one to say `false` with, and a file that could
    // spell overdrawing two ways — absent, and present-and-false — would leave
    // a reader asking what a writer meant by choosing the other. So an
    // overdrawing Set is the file it always was, byte for byte.
    //
    // **After the edges and before the camera**, which is the order
    // `docs/ir-spec.md` shows the format in. It is a node with no procedure
    // described by a record of its own, exactly as the built-in camera below
    // is, so it is written beside it.
    //
    // **`live` only here.** A selection is carried by the record that says the
    // Set composites, so an overdrawing Set's edges — which exist and which
    // nothing reads — leave nothing behind: that is `Record::Select`'s
    // position, unchanged, and the reason `Set::select_renderer` is silently
    // ineffective there rather than refused.
    if layering == Layering::Composite {
        lines.push(Line::new(Record::Merge { live }));
    }
    // **The built-in camera is the last node of the L3 layer**, and the index
    // is written for the same reason a `seed`'s and a `capacity`'s are: the
    // record describes one producer, and a layer that holds several needs to
    // say which. It is `L3:0` in a Set whose files declare no camera procedure,
    // which is every Set written before they could — and zero is not written,
    // so those files are the line they always were.
    lines.push(Line::new(Record::Camera {
        kind: "orbit".to_string(),
        index: nodes.iter().filter(|n| n.layer == Kind::L3).count() as u32,
        radius: camera.radius,
        speed: camera.speed,
    }));
    // On L1, because that is the layer whose randomness a salt moves, and one
    // per geometry, because that is what it salts. **Recorded rather than left
    // to be derived**, which is what `docs/ir-spec.md` asks for: a value
    // derived from a position in `--set` changes when the list is reordered,
    // and a value read back from here does not. Index 0 is absent from the
    // record, so a Set of one geometry is the line it always was.
    for (index, value) in seeds.iter().enumerate() {
        lines.push(Line::new(Record::Seed {
            stream: Layer::L1,
            index: index as u32,
            value: u64::from(*value),
        }));
    }

    store
        .write_set(id, &lines)
        .map_err(|e| format!("writing set `{id}`: {e}"))
}

/// **Read a Set file back into what the engine takes.**
///
/// A source is resolved from the inlined `src` records when the file carries
/// them — the bundled form — and from the store otherwise. Bundling wins
/// because a file that carries its own source is meant to be readable on a
/// machine whose store has never seen it.
pub fn load(store: &Store, id: &str) -> Result<Loaded, String> {
    let lines = store
        .read_set(id)
        .map_err(|e| format!("reading set `{id}`: {e}"))?;
    from_lines(store, id, &lines)
}

/// The decode, over lines that are already in hand. Split out so a test can
/// build a file in memory and so a session stream's head can be loaded the same
/// way once anything writes one.
pub fn from_lines(store: &Store, id: &str, lines: &[Line]) -> Result<Loaded, String> {
    let mut notes = Vec::new();
    // Every layer's slots by index, the layers in the order [`layer_ordinal`]
    // gives them. `None` is a gap — an index nothing claimed — which is refused
    // below rather than silently closed up.
    let mut slots: [Vec<Option<Hash>>; 5] = Default::default();
    // What each of those is called, in the same shape and by the same index, so
    // a name and the artifact it belongs to are placed by one statement.
    let mut slot_names: [Vec<Option<String>>; 5] = Default::default();
    // The edges the file recorded, in the order it recorded them.
    let mut edges: Vec<karakuri_engine::set::Edge> = Vec::new();
    let mut inlined: BTreeMap<Hash, BTreeMap<u32, String>> = BTreeMap::new();
    // What each geometry runs at, by index, growing as the file names them.
    let mut capacities: Vec<Option<u32>> = Vec::new();
    let mut params = Vec::new();
    let mut bindings = Vec::new();
    let mut camera = None;
    // **Whether the file said this Set composites, and which renderer it left
    // selected.** `Overdraw` until a `merge` record says otherwise, because
    // that is what the record's absence has always meant — every file written
    // before it existed says overdraw by saying nothing.
    let mut layering = Layering::Overdraw;
    let mut live: Option<u32> = None;
    // Which camera node the `camera` record above was about, checked against
    // the file's L3 slots once every one of them has been met.
    let mut camera_index = 0;
    // What each geometry is salted with, by index, growing as the file names
    // them — the same shape as `capacities`, because a `seed` is addressed the
    // same way and for the same reason.
    let mut salts: Vec<Option<u32>> = Vec::new();
    let mut file_id = id.to_string();

    for line in lines {
        match line.record() {
            Record::Set { id, v } => {
                if *v != VERSION {
                    notes.push(format!(
                        "the file says version {v} and this build writes {VERSION}; \
                         reading it anyway"
                    ));
                }
                file_id = id.clone();
            }
            Record::Slot {
                layer,
                index,
                name,
                proc_hash,
            } => {
                // **A name is carried now, where it used to be reported and
                // dropped.** "Nothing this build points at a node by name" was
                // true until an `edge` did: an edge names the node that
                // declares a slot and the node bound to it, so a Set whose
                // names were dropped on the way in is a Set whose edges resolve
                // against the wrong spellings — or against none.
                //
                // **Placed by layer and index, and every layer reads the same
                // way.** A second `slot` on a layer is a second node — another
                // geometry, another deformer in the chain, another renderer over
                // the same points — rather than a correction of the first, and
                // the index says which, so the records need not arrive in order
                // and the projection can fold them without one.
                //
                // Three of the five layers used to be dropped with a note saying
                // a Set file records an L1 and its renderers, and the L1's own
                // index was dropped beside them: the format could always say a
                // chain and this loader could not read one back, so a cube
                // morphing into a sphere could be played and not kept.
                let at = *index as usize;
                let ordinal = layer_ordinal(kind_of(*layer)) as usize;
                let layer_slots = &mut slots[ordinal];
                if layer_slots.len() <= at {
                    layer_slots.resize(at + 1, None);
                }
                if layer_slots[at].is_some() {
                    notes.push(format!(
                        "two {} slots both claim index {at}; the later one is used",
                        layer_name(*layer)
                    ));
                }
                layer_slots[at] = Some(*proc_hash);
                let layer_names = &mut slot_names[ordinal];
                if layer_names.len() <= at {
                    layer_names.resize(at + 1, None);
                }
                layer_names[at] = name.clone();
            }
            Record::Src { hash, line, s } => {
                inlined.entry(*hash).or_default().insert(*line, s.clone());
            }
            // **Keyed by the geometry it sizes.** A capacity addressed at the
            // second source used to be reported and dropped, because the engine
            // held one number per Set; it holds one per source now, so the
            // number reaches the geometry the file wrote it against rather than
            // resizing the wrong one.
            Record::Capacity {
                layer,
                index,
                value,
            } => match *layer {
                Layer::L1 => {
                    let at = *index as usize;
                    if capacities.len() <= at {
                        capacities.resize(at + 1, None);
                    }
                    if capacities[at].is_some() {
                        notes.push(format!(
                            "two capacities both claim L1 index {at}; the later one is used"
                        ));
                    }
                    capacities[at] = Some(*value);
                }
                other => notes.push(format!(
                    "capacity on {} was skipped: a capacity sizes a geometry, and the \
                     geometries are L1's",
                    layer_name(other)
                )),
            },
            Record::Param {
                layer,
                index,
                key,
                value,
            } => match value {
                // The address is `(layer, index)` present or absent as a unit,
                // so a record with no index is a wildcard whatever its `layer`
                // says — which is what keeps every file written before the
                // address existed meaning what it meant.
                Value::Scalar(v) => params.push(match index {
                    Some(at) => ParamWrite::at(kind_of(*layer), *at, key.clone(), *v),
                    None => ParamWrite::everywhere(key.clone(), *v),
                }),
                _ => notes.push(format!(
                    "param `{key}` was skipped: it is a vector and the engine holds \
                     scalar parameter values only"
                )),
            },
            // **Carried as written, both ends.** Whether the nodes it names
            // are in this Set is not a question this decoder can answer — a
            // name nobody wrote is derived where the Set is built — so it is
            // asked there, once, rather than here and again there.
            Record::Edge { node, slot, to } => edges.push(karakuri_engine::set::Edge {
                node: node.clone(),
                slot: slot.clone(),
                to: to.clone(),
            }),
            record @ Record::Bind { .. } => match binding_from_record(record) {
                Ok(binding) => bindings.push(binding),
                // Reported and skipped rather than failing the load: one
                // unusable binding is not a reason to refuse the material, and
                // the note says exactly which parameter will not move.
                Err(message) => notes.push(format!("{message} — skipped")),
            },
            Record::Camera {
                kind,
                index,
                radius,
                speed,
            } => {
                if kind != "orbit" {
                    notes.push(format!(
                        "camera kind `{kind}` is not one this engine has; using an orbit"
                    ));
                }
                // **The index is checked against where the built-in actually
                // is**, which is after the camera procedures the file names —
                // so it is `0` in a file that names none, which is every file
                // written before this record carried an index. An index naming
                // a camera that *is* a procedure is said rather than applied:
                // such a node writes its own six numbers every frame and would
                // overwrite these before the first draw.
                //
                // Checked after the loop, where the `slot` records have all
                // been met: a record's meaning here depends on how many
                // cameras the file names, and the count is only complete at
                // the end.
                camera_index = *index;
                // The record carries two of the six fields an `Orbit` has, so
                // the rest take their defaults. Said out loud because a saved
                // camera and a loaded one are then not the same camera unless
                // the other four were already default.
                camera = Some(Orbit {
                    radius: *radius,
                    speed: *speed,
                    ..Orbit::default()
                });
            }
            // **This Set composites its renderers**, which is the one thing a
            // Set knew about itself that this file could not say. The record's
            // presence is the whole statement — there is no field to read —
            // and `live` beside it is which renderer the fold is left with.
            //
            // **Absent `live` is every input live and emphatically not node
            // 0**: reading it as 0 would silence every renderer but the first
            // in every composited Set that was saved without a selection, which
            // is a picture nobody asked for. Checked against the renderers the
            // file names after the loop, where the count is complete.
            Record::Merge { live: at } => {
                layering = Layering::Composite;
                live = *at;
            }
            // **One per geometry, and each reaches the source it names.** This
            // took index 0's and reported the rest, because the engine took one
            // salt per Set and derived each source's from it. It takes them per
            // source now, so a Set of two grids comes back with the colours it
            // was saved with however its records are ordered — which is the
            // whole of what recording a salt buys over deriving one.
            Record::Seed {
                stream,
                index,
                value,
            } => match *stream {
                Layer::L1 => {
                    let at = *index as usize;
                    if salts.len() <= at {
                        salts.resize(at + 1, None);
                    }
                    if salts[at].is_some() {
                        notes.push(format!(
                            "two seeds both claim L1 index {at}; the later one is used"
                        ));
                    }
                    salts[at] = Some(*value as u32);
                }
                other => notes.push(format!(
                    "seed on {} was skipped: a seed salts a geometry, and the geometries \
                     are L1's",
                    layer_name(other)
                )),
            },
            // Not a Set file's, and each for its own reason — see
            // `Record::is_set_state`.
            Record::Tick { .. }
            | Record::Audio { .. }
            | Record::Tempo { .. }
            | Record::Gain { .. }
            | Record::Opacity { .. }
            | Record::Blend { .. }
            | Record::Residency { .. }
            | Record::Look { .. }
            | Record::Canvas { .. }
            | Record::Procedure { .. }
            | Record::Transport { .. }
            | Record::Preview { .. }
            | Record::Transition { .. }
            // A `select` names a renderer of a *deck slot* and schedules it at
            // an instant, and nothing in a session says which slot's Set
            // composites or which slot a folded Set was played in — so a
            // stream folded down cannot tell whether a selection it meets is
            // about the Set being written. Which renderer a *Set* is left
            // folded to is `merge`'s `live`, above. See `Record::Select`.
            | Record::Select { .. }
            | Record::Mask { .. }
            // A `save` is here for a second reason as well as that one: it
            // names a Set file, and this *is* the Set file reader. Obeying it
            // would be a load that goes looking for another file.
            | Record::Save { .. } => notes.push(
                "a record that belongs to a session rather than to a Set was skipped".to_string(),
            ),
            // **An artifact's card, in a Set file.** `Store::write_set`
            // refuses to write one, so this is a hand-edited or hand-assembled
            // file — and it is skipped with a sentence of its own rather than
            // under the session one above, because the confusion it comes from
            // is a different confusion: `param_decl` says what a procedure
            // offers and `param` says what this Set turned it to, and an
            // operator who wrote the first meaning the second wants to be told
            // which one they wrote.
            Record::Meta { .. }
            | Record::ParamDecl { .. }
            | Record::CapacityDecl { .. }
            | Record::Emit { .. } => notes.push(
                "a record that belongs to an artifact's metadata rather than to a Set was \
                 skipped — a Set file records what a value is, not what a procedure declares"
                    .to_string(),
            ),
            Record::Unknown => notes.push(
                "a record type this build does not know was skipped, as the format says to"
                    .to_string(),
            ),
        }
    }

    let source_of = |layer: &str, hash: &Hash| -> Result<String, String> {
        if let Some(lines) = inlined.get(hash) {
            // Bundled: the file carries its own source, so it reads on a
            // machine whose store has never seen this artifact.
            return Ok(lines.values().cloned().collect::<Vec<_>>().join("\n"));
        }
        let bytes = store.get_artifact(hash).map_err(|e| {
            format!(
                "set `{file_id}`: {layer} is `{}` and the store does not have it ({e}); \
                 a Set file references its procedures by hash, so the artifact has to be \
                 in the store or inlined in the file",
                hash.short(12)
            )
        })?;
        String::from_utf8(bytes).map_err(|e| format!("set `{file_id}`: {layer} is not UTF-8: {e}"))
    };

    // One layer's sources, in index order. A gap is not a missing artifact:
    // index 2 with no index 1 describes a chain with a hole in it, and closing
    // it up would silently change draw order or what deforms what.
    let sources = |layer: Kind| -> Result<Vec<String>, String> {
        slots[layer_ordinal(layer) as usize]
            .iter()
            .enumerate()
            .map(|(at, hash)| match hash {
                Some(hash) => source_of(kind_name(layer), hash),
                None => Err(format!(
                    "set `{file_id}` names a {} at index {at} but none before it",
                    kind_name(layer)
                )),
            })
            .collect()
    };
    let l1_srcs = sources(Kind::L1)?;
    let l2_srcs = sources(Kind::L2)?;
    let l3_srcs = sources(Kind::L3)?;
    let l4_srcs = sources(Kind::L4)?;
    let field_srcs = sources(Kind::Field)?;
    // **The built-in camera is the last node of the L3 layer**, after the
    // procedures the file names — so a record about any earlier one is about a
    // camera that produces its own six numbers every frame, and applying these
    // to it would be overwritten before the first draw. Reported and dropped,
    // which is what this file does with everything it cannot honour.
    if camera.is_some() && camera_index as usize != l3_srcs.len() {
        notes.push(format!(
            "camera at L3:{camera_index} was skipped: the built-in orbit is this Set's L3:{}, \
             and a camera that is a procedure produces its own state",
            l3_srcs.len()
        ));
        camera = None;
    }
    // **A selection names a renderer this Set draws with**, checked here for
    // the `camera` index's reason one block above: what the number means
    // depends on how many renderers the file names, and that count is only
    // complete once every `slot` record has been met. Reported and dropped
    // rather than refused — the Set is whole, and a fold with every input live
    // is the state it would have come up in anyway.
    if live.is_some_and(|at| at as usize >= l4_srcs.len()) {
        notes.push(format!(
            "the merge selects renderer {} and this file names {} — every renderer \
             is left live",
            live.unwrap_or_default(),
            l4_srcs.len()
        ));
        live = None;
    }
    if l1_srcs.is_empty() {
        return Err(format!("set `{file_id}` has no L1 slot"));
    }
    if l4_srcs.is_empty() {
        return Err(format!("set `{file_id}` has no L4 slot"));
    }
    // **Nothing is capped here any more.** The last two refusals in this
    // function were about the plumbing rather than about the material and said
    // so; a renderer names the slot it draws through and an `edge` names which
    // camera fills it, so several cameras are several nodes with names of their
    // own and there is nothing for this to arbitrate. A file naming two now
    // loads two.
    // A capacity for a geometry the file does not name has nothing to size.
    // Said rather than dropped, because it is the file describing a source that
    // is not there — a `slot` record that went missing, most likely.
    for (at, value) in capacities.iter().enumerate().skip(l1_srcs.len()) {
        if let Some(value) = value {
            notes.push(format!(
                "capacity {value} on L1 index {at} was skipped: this file names {} \
                 geometr{}",
                l1_srcs.len(),
                if l1_srcs.len() == 1 { "y" } else { "ies" }
            ));
        }
    }
    capacities.resize(l1_srcs.len(), None);

    let check = |srcs: &[String]| {
        srcs.iter()
            .map(|src| crate::compile::check(src))
            .collect::<Result<Vec<_>, _>>()
    };
    let l1s = check(&l1_srcs)?;
    let l2s = check(&l2_srcs)?;
    let l3s = check(&l3_srcs)?;
    let l4s = check(&l4_srcs)?;
    let fields = check(&field_srcs)?;

    // Node order, which is what [`Loaded::srcs`] promises: the same order the
    // procedures above are chained in, so the two walk in step.
    let srcs = l1_srcs
        .into_iter()
        .chain(l2_srcs)
        .chain(l3_srcs)
        .chain(l4_srcs)
        .chain(field_srcs)
        .collect();

    // **Trimmed to the nodes that came back**, so the two lists cannot
    // disagree: a name past the end of its layer belongs to a `slot` record the
    // refusals above have already dealt with.
    let mut names = crate::Names {
        l1s: std::mem::take(&mut slot_names[layer_ordinal(Kind::L1) as usize]),
        l2s: std::mem::take(&mut slot_names[layer_ordinal(Kind::L2) as usize]),
        l3s: std::mem::take(&mut slot_names[layer_ordinal(Kind::L3) as usize]),
        l4s: std::mem::take(&mut slot_names[layer_ordinal(Kind::L4) as usize]),
        fields: std::mem::take(&mut slot_names[layer_ordinal(Kind::Field) as usize]),
    };
    names.l1s.truncate(l1s.len());
    names.l2s.truncate(l2s.len());
    names.l3s.truncate(l3s.len());
    names.l4s.truncate(l4s.len());
    names.fields.truncate(fields.len());

    Ok(Loaded {
        id: file_id,
        l1s,
        l2s,
        l3s,
        fields,
        l4s,
        srcs,
        capacities,
        params,
        bindings,
        names,
        edges,
        camera,
        layering,
        live,
        salts,
        notes,
    })
}

/// **What one node of a Set is called** — the one answer, for every surface
/// that has to name one.
///
/// Three candidates in a fixed order: the name this Set file's own `slot`
/// record carries, then the name the artifact's metadata card declares, then
/// the artifact's short hash.
///
/// **The Set's own name wins over the card's, because it is the more specific
/// fact.** A card says what a procedure calls *itself* and says the same thing
/// in every Set that references the artifact; a `slot` name is what *this* Set
/// decided to call *this* node, is what an `edge` in the same file points at,
/// and is what an operator typed in `--set near=lattice.kir`. A listing that
/// preferred the card would give two nodes of one Set the same name wherever a
/// chain instantiates one procedure twice.
///
/// **A node with neither still has a name.** An artifact with no card is an
/// ordinary state of a working store rather than a damaged one — `mcp`'s
/// `node_block` says so at length — and so is a Set naming an artifact this
/// store never had. Neither is a reason to leave a node out of a listing or to
/// print a blank where its name goes, so the short hash is the last resort: it
/// tells two nodes apart, it recognises the same artifact in two Sets, and it
/// is the same shortening every log line in this program uses.
///
/// **One function, called by both surfaces and by `read_set`.** Two derivations
/// that agree today are two answers that stop agreeing the day one of them is
/// edited, and the name a model reads in a listing has to be the name it then
/// finds when it reads that Set.
pub fn node_called(written: Option<&str>, declared: Option<&str>, hash: &Hash) -> String {
    written
        .or(declared)
        .map_or_else(|| hash.short(12), str::to_string)
}

/// What an artifact's metadata card calls it, or `None` where this store has no
/// card for it.
///
/// `None` covers both absences on purpose — no card written yet, and no
/// artifact here at all — because the answer to *what is this node called* is
/// the same for both: whatever the Set file says, and the short hash otherwise.
/// The two are worth telling apart when a model is choosing material, and
/// `read_set` is where that is done.
pub fn card_name(store: &Store, hash: &Hash) -> Option<String> {
    store
        .read_meta(hash)
        .ok()?
        .iter()
        .find_map(|line| match line.record() {
            Record::Meta { name, .. } => Some(name.clone()),
            _ => None,
        })
}

/// One node of a Set, as a listing needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSummary {
    pub layer: Layer,
    /// Which node of that layer, from 0 — the address `read_set` prints and
    /// the one the other MCP tools take.
    pub index: u32,
    /// What it is called: [`node_called`]'s answer, never a second reading of
    /// the same three candidates.
    pub name: String,
    /// The artifact behind it, whether or not this store holds one.
    pub hash: Hash,
}

/// **What one saved Set holds**, read off the store: the id that names it, when
/// its file was written, and a line per node.
///
/// **One value, rendered twice.** The MCP `list_sets` tool and `--list-sets`
/// answer one question for two readers, and a library summarised once per
/// reader is two answers that agree until somebody edits one of them. Both
/// render this; neither opens a Set file of its own.
///
/// **What it deliberately does not carry**: what any of those nodes declares,
/// what this Set has turned anything to, and whether it builds. Those need a
/// card per artifact, the rest of the file, and a compile pass respectively —
/// `read_set` does all three for *one* Set on purpose, and a listing that did
/// them for a library would compile a thousand procedures to print a thousand
/// lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetSummary {
    pub id: String,
    /// When the file was last written, from the filesystem — a Set file carries
    /// no time of its own. See `Store::list_sets`.
    pub written: std::time::SystemTime,
    pub nodes: Vec<NodeSummary>,
    /// Why this Set's file could not be read, where it could not be.
    ///
    /// **Listed anyway, and said out loud.** A file in `sets/` that will not
    /// parse is something an operator has to be told about; dropping it from
    /// the listing would answer *what have I kept* with a Set missing, and
    /// reporting it with an empty node list would say it holds nothing.
    pub unreadable: Option<String>,
}

/// **Summarise every Set the store holds.**
///
/// One directory read, one read per Set file, and a card read per node the file
/// left unnamed — nothing else. Nothing is compiled, no adapter is opened and
/// `karakuri_engine::Set::validate` is never called: `load` and `read_set` do
/// that for one named Set on purpose, and *what is in my library* is not the
/// question that should pay for it.
///
/// **The card reads are memoised across the whole listing**, keyed by the
/// artifact rather than by the node: a library where fifty Sets reference one
/// unnamed geometry reads that card once. A named node reads no card at all,
/// because [`node_called`] would not have used it.
///
/// The order is `Store::list_sets`'s — ascending by id, which is total and
/// repeatable. A surface that wants recency sorts on [`SetSummary::written`]
/// and says why.
pub fn summarise(store: &Store) -> Result<Vec<SetSummary>, StoreError> {
    let mut cards: BTreeMap<Hash, Option<String>> = BTreeMap::new();
    let mut out = Vec::new();
    for entry in store.list_sets()? {
        let mut nodes = Vec::new();
        let mut unreadable = None;
        match store.read_set(&entry.id) {
            Ok(lines) => {
                for line in &lines {
                    // **The file's own order**, which is the order [`save`]
                    // wrote the nodes in and the order a hand-written file
                    // chose — the same reading `read_set` gives, for the same
                    // reason: sorting by layer here would impose an order
                    // nobody wrote.
                    let Record::Slot {
                        layer,
                        index,
                        name,
                        proc_hash,
                    } = line.record()
                    else {
                        continue;
                    };
                    let declared = match name {
                        Some(_) => None,
                        None => cards
                            .entry(*proc_hash)
                            .or_insert_with(|| card_name(store, proc_hash))
                            .clone(),
                    };
                    nodes.push(NodeSummary {
                        layer: *layer,
                        index: *index,
                        name: node_called(name.as_deref(), declared.as_deref(), proc_hash),
                        hash: *proc_hash,
                    });
                }
            }
            Err(e) => unreadable = Some(e.to_string()),
        }
        out.push(SetSummary {
            id: entry.id,
            written: entry.written,
            nodes,
            unreadable,
        });
    }
    Ok(out)
}

/// When a Set was written, spelled the one way every listing spells it.
///
/// **Local, for the reason [`crate::history::stamped_id`] is local**: the
/// answer has to be the one the person would say out loud, and a UTC clock is
/// the wrong one for half the world and half the day. To the second, because
/// that is as fine as a filesystem mtime is worth reading and finer than
/// anybody scanning a column wants.
pub fn written_at(at: std::time::SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(at)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const L1: &str = r#"
proc ring {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5
  param spin   : float [0.0, 4.0] = 1.0

  emit position, age

  element {
    let a = t * spin + hash1(seed) * 1.2;
    position = vec3(cos(a) * radius, sin(a) * radius, 0.0);
    age      = t;
  }
}
"#;

    const L4: &str = r#"
proc points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0 - length(point_coord * 2.0 - 1.0));
  }
}
"#;

    /// The rest of the chain a `--set` can spell, minimal for the reason the
    /// pair above is: what is under test is the file, not the picture.
    const L2: &str = r#"
proc warp {
  kind L2

  consumes position

  deform {
    position = vec3(position.x, position.y * 1.5, position.z);
  }
}
"#;

    const L3: &str = r#"
proc look {
  kind L3

  camera {
    eye    = vec3(0.0, 2.0, 9.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;

    const FIELD: &str = r#"
proc blob {
  kind Field

  field {
    distance = sd_sphere(point, 1.0);
  }
}
"#;

    /// A store with the two procedures written out beside it, and the paths.
    fn fixture() -> (
        tempfile::TempDir,
        Store,
        std::path::PathBuf,
        std::path::PathBuf,
    ) {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        let l4 = dir.path().join("l4.kir");
        std::fs::write(&l1, L1).expect("write l1");
        std::fs::write(&l4, L4).expect("write l4");
        let store = Store::open(dir.path().join("store")).expect("store");
        (dir, store, l1, l4)
    }

    /// A `.kir` on disk beside the fixture's, so a test can name a node of any
    /// layer it likes.
    fn beside(dir: &tempfile::TempDir, file: &str, src: &str) -> std::path::PathBuf {
        let path = dir.path().join(file);
        std::fs::write(&path, src).expect("write");
        path
    }

    /// **A fixture's source, in the store, as [`save`] now wants it.** The
    /// tests here are written against files on disk, because a file is what a
    /// fixture is; the writer takes hashes. This is the one line that bridges
    /// them, rather than every test growing its own `put`.
    fn stored(store: &Store, path: &std::path::Path) -> Hash {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    }

    /// The nodes of an ordinary Set — one geometry, and the renderers over it
    /// in draw order.
    fn ordinary(store: &Store, l1: &std::path::Path, l4s: &[std::path::PathBuf]) -> Vec<Node> {
        std::iter::once(Node {
            hash: stored(store, l1),
            layer: Kind::L1,
            index: 0,
            name: None,
        })
        .chain(l4s.iter().enumerate().map(|(at, path)| Node {
            hash: stored(store, path),
            layer: Kind::L4,
            index: at as u32,
            name: None,
        }))
        .collect()
    }

    /// The whole set file as bytes, which is what a compatibility claim is
    /// about. `read` keeps each line's text verbatim, so this is what is on
    /// disk rather than a re-serialisation of it.
    fn written(store: &Store, id: &str) -> String {
        store
            .read_set(id)
            .expect("read")
            .iter()
            .map(|line| format!("{}\n", line.as_str()))
            .collect()
    }

    /// A Set file's text, through the file reader — so the bytes a test writes
    /// out are genuinely parsed, rather than hand-built into records that could
    /// not have been written. The file is gone by the time this returns; the
    /// lines are in memory.
    fn parsed(text: &str) -> Vec<Line> {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("hand_written.set.ndjson");
        std::fs::write(&path, text).expect("write");
        karakuri_store::ndjson::read(&path).expect("a hand-written Set file parses")
    }

    /// A Set with nothing but its material and whatever bindings are given.
    /// `Orbit::default()` is not `const`, so this is the one place a test names
    /// its fields; `LazyLock` keeps that to one place rather than one per call.
    static DEFAULT_CAMERA: std::sync::LazyLock<Orbit> = std::sync::LazyLock::new(Orbit::default);

    fn plain<'a>(nodes: &'a [Node], bindings: &'a [Binding]) -> Saving<'a> {
        Saving {
            nodes,
            capacities: &[4096],
            params: &[],
            bindings,
            edges: &[],
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1],
        }
    }

    /// **The edge survives the file, and so do the names it points with.**
    ///
    /// The two halves are one fact: an edge is between *names*, and a file that
    /// carried the edge and dropped the names would come back naming nodes that
    /// are no longer called that. What makes it round-trip at all is that the
    /// names it points with are either written down beside the node — as `far`
    /// is here — or derived from the procedure, which is a function of the
    /// artifact the `slot` record already references.
    #[test]
    fn an_edge_and_the_names_it_points_with_survive_the_file() {
        let (dir, store, l1, l4) = fixture();
        let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
        let l2 = beside(&dir, "l2.kir", L2);
        let nodes = vec![
            Node {
                hash: stored(&store, &l1),
                layer: Kind::L1,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &l1b),
                layer: Kind::L1,
                index: 1,
                // Written, because it is what the edge points with.
                name: Some("far".to_string()),
            },
            Node {
                hash: stored(&store, &l2),
                layer: Kind::L2,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &l4),
                layer: Kind::L4,
                index: 0,
                name: None,
            },
        ];
        let edges = vec![karakuri_engine::set::Edge {
            node: "warp".to_string(),
            slot: "far".to_string(),
            to: "far".to_string(),
        }];
        save(
            &store,
            "wired",
            Saving {
                nodes: &nodes,
                capacities: &[4096, 4096],
                params: &[],
                bindings: &[],
                edges: &edges,
                camera: &DEFAULT_CAMERA,
                layering: Layering::Overdraw,
                live: None,
                seeds: &[1, 2],
            },
        )
        .expect("save");

        let text = written(&store, "wired");
        assert!(
            text.contains(r#"{"t":"edge","node":"warp","slot":"far","to":"far"}"#),
            "the edge is one line naming both ends: {text}"
        );
        // **After the slots**, so a reader has every name in hand by the time
        // it meets the record that uses them.
        assert!(
            text.find(r#""t":"edge""#) > text.rfind(r#""t":"slot""#),
            "an edge is written after the slots it names: {text}"
        );

        let loaded = load(&store, "wired").expect("load");
        assert_eq!(loaded.edges, edges, "the edge came back as it went in");
        assert_eq!(loaded.names.l1s, [None, Some("far".to_string())]);
        assert!(
            loaded.notes.is_empty(),
            "nothing here is unhonourable: {:?}",
            loaded.notes
        );
    }

    /// A deformation that names one of the Set's sources through a slot.
    const DISSOLVE: &str = r#"
proc dissolve {
  kind L2

  uses only : Source

  consumes position

  mask {
    strength = 0.0;
    if source == only { strength = 1.0; }
  }

  deform {
    position = position * 0.2;
  }
}
"#;

    /// **A Source-slot edge survives the file**, with the name it points with.
    ///
    /// The record is the same `edge` a geometry slot writes — node, slot, and
    /// the node it is bound to — because what an edge says is one fact whatever
    /// type the slot was declared with. That is the claim: the fourth slot type
    /// cost this file nothing, and a Set whose mask names a source can be saved
    /// and loaded like any other.
    #[test]
    fn a_source_slot_edge_survives_the_file() {
        let (dir, store, l1, l4) = fixture();
        let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
        let l2 = beside(&dir, "l2.kir", DISSOLVE);
        let nodes = vec![
            Node {
                hash: stored(&store, &l1),
                layer: Kind::L1,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &l1b),
                layer: Kind::L1,
                index: 1,
                // Written, because it is what the edge points with — and here
                // it is what the mask *means*, rather than a second buffer.
                name: Some("victim".to_string()),
            },
            Node {
                hash: stored(&store, &l2),
                layer: Kind::L2,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &l4),
                layer: Kind::L4,
                index: 0,
                name: None,
            },
        ];
        let edges = vec![karakuri_engine::set::Edge {
            node: "dissolve".to_string(),
            slot: "only".to_string(),
            to: "victim".to_string(),
        }];
        save(
            &store,
            "masked",
            Saving {
                nodes: &nodes,
                capacities: &[4096, 4096],
                params: &[],
                bindings: &[],
                edges: &edges,
                camera: &DEFAULT_CAMERA,
                // **The salts are the identities the mask compares**, so a
                // saved Set that gave them back differently would be a mask
                // pointing at a different geometry after a reload.
                layering: Layering::Overdraw,
                live: None,
                seeds: &[11, 22],
            },
        )
        .expect("save");

        let text = written(&store, "masked");
        assert!(
            text.contains(r#"{"t":"edge","node":"dissolve","slot":"only","to":"victim"}"#),
            "one line, both ends, and nothing about the type: {text}"
        );

        let loaded = load(&store, "masked").expect("load");
        assert_eq!(loaded.edges, edges, "the edge came back as it went in");
        assert_eq!(loaded.names.l1s, [None, Some("victim".to_string())]);
        assert_eq!(
            loaded.salts,
            vec![Some(11), Some(22)],
            "and so did the salts the mask compares against"
        );
        assert!(
            loaded.notes.is_empty(),
            "nothing here is unhonourable: {:?}",
            loaded.notes
        );
    }

    fn a_binding() -> Binding {
        Binding::new(Kind::L1, "spin", "beat", Curve::Pow2, [0.5, 3.0])
    }

    /// **Everything a Set file is for, in one assertion**: what went in comes
    /// back out. A format that carried the material and lost the parameters
    /// would still load, still render, and still be the wrong Set.
    #[test]
    fn a_saved_set_loads_back_as_what_was_saved() {
        let (_dir, store, l1, l4) = fixture();
        let params = vec![ParamWrite::everywhere("radius", 3.25)];
        let camera = Orbit {
            radius: 11.5,
            speed: 0.42,
            ..Orbit::default()
        };
        save(
            &store,
            "s1",
            Saving {
                nodes: &ordinary(&store, &l1, std::slice::from_ref(&l4)),
                capacities: &[65_536],
                params: &params,
                bindings: &[a_binding()],
                edges: &[],
                camera: &camera,
                layering: Layering::Overdraw,
                live: None,
                seeds: &[4242],
            },
        )
        .expect("save");

        let loaded = load(&store, "s1").expect("load");
        assert_eq!(loaded.id, "s1");
        assert_eq!(loaded.capacities, vec![Some(65_536)]);
        assert_eq!(loaded.params, params);
        assert_eq!(loaded.salts, vec![Some(4242)]);
        assert_eq!(
            loaded.camera.map(|c| (c.radius, c.speed)),
            Some((11.5, 0.42))
        );
        assert_eq!(loaded.bindings.len(), 1);
        let back = &loaded.bindings[0];
        assert_eq!(back.layer, Kind::L1);
        assert_eq!(back.key, "spin");
        assert_eq!(back.signal, "beat");
        assert_eq!(back.curve, Curve::Pow2);
        assert_eq!(back.range, [0.5, 3.0]);
        // And the procedures themselves came back through the store, compiled.
        assert!(
            loaded.notes.is_empty(),
            "unexpected notes: {:?}",
            loaded.notes
        );
    }

    /// **The material is resolved by hash out of the store**, which is what
    /// makes a Set file a few dozen lines rather than a copy of the source. A
    /// file whose artifacts are missing says so instead of loading something
    /// else.
    #[test]
    fn a_set_whose_artifacts_are_missing_says_which_and_why() {
        let (_dir, store, l1, l4) = fixture();
        save(
            &store,
            "s1",
            plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
        )
        .expect("save");
        let lines = store.read_set("s1").expect("read");

        // A second store that has the file but not the artifacts — a Set file
        // carried to a machine that has never seen the procedures.
        let elsewhere = tempfile::tempdir().expect("tempdir");
        let bare = Store::open(elsewhere.path()).expect("store");
        let err = from_lines(&bare, "s1", &lines).expect_err("the artifacts are not there");
        assert!(err.contains("the store does not have it"), "{err}");
        assert!(err.contains("inlined"), "{err}");
    }

    /// The bundled form: a file carrying its own source reads on a machine
    /// whose store has never seen the artifact. **Inlined source wins over the
    /// store**, so a bundle is self-contained rather than half-resolved.
    #[test]
    fn inlined_source_loads_without_a_store_that_knows_the_artifact() {
        let (_dir, store, l1, l4) = fixture();
        save(
            &store,
            "s1",
            plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
        )
        .expect("save");
        let mut lines = store.read_set("s1").expect("read");
        // Bundle it: every slot's source inlined, line by line, as `src`.
        let mut bundled = Vec::new();
        for line in &lines {
            if let Record::Slot {
                proc_hash, layer, ..
            } = line.record()
            {
                let src = match layer {
                    Layer::L1 => L1,
                    _ => L4,
                };
                for (n, text) in src.lines().enumerate() {
                    bundled.push(Line::new(Record::Src {
                        hash: *proc_hash,
                        line: n as u32,
                        s: text.to_string(),
                    }));
                }
            }
        }
        lines.append(&mut bundled);

        let elsewhere = tempfile::tempdir().expect("tempdir");
        let bare = Store::open(elsewhere.path()).expect("store");
        let loaded = from_lines(&bare, "s1", &lines).expect("the file carries its own source");
        assert_eq!(loaded.l1s[0].name, "ring");
        assert_eq!(loaded.l4s[0].name, "points");
    }

    /// **A file written before the address existed still means what it meant.**
    ///
    /// `layer` on a `param` record was a placeholder: the writer put `L1` on
    /// everything and said so in a comment, and the loader ignored it. So
    /// honouring `layer` now would silently retarget every Set file ever
    /// written — an `exposure` that reached the renderer would start reaching
    /// the L1 and doing nothing.
    ///
    /// What stops that is the address being `(layer, index)` present or absent
    /// **as a unit**: no `index`, no address, whatever `layer` says. This reads
    /// a hand-written old-style file to prove it, rather than one this build
    /// produced — a round trip through the new writer would agree with itself
    /// however wrong both halves were.
    #[test]
    fn a_param_record_without_an_index_is_a_wildcard_whatever_its_layer_says() {
        let (_dir, store, l1, l4) = fixture();
        let hashes: Vec<String> = [&l1, &l4]
            .iter()
            .map(|p| {
                let bytes = std::fs::read(p).expect("read");
                store.put_artifact(&bytes).expect("put").to_string()
            })
            .collect();
        let text = format!(
            r#"{{"t":"set","id":"old","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{{"t":"param","layer":"L1","key":"exposure","value":0.4}}
{{"t":"param","layer":"L4","index":1,"key":"exposure","value":0.9}}
"#,
            hashes[0], hashes[1]
        );
        let lines = parsed(&text);

        let loaded = from_lines(&store, "old", &lines).expect("an old-style file still loads");
        assert_eq!(
            loaded.params,
            vec![
                // No index: a wildcard, even though the record says `L1`.
                ParamWrite::everywhere("exposure", 0.4),
                // An index: an address, and `layer` is load-bearing beside it.
                ParamWrite::at(Kind::L4, 1, "exposure", 0.9),
            ]
        );
    }

    /// **What could not be carried is said, not dropped.** Three shapes, and
    /// each is a real disagreement between what the format can address and what
    /// the engine has: a salt and a capacity belong to a *geometry*, so one
    /// written against a renderer names something that does not exist, and a
    /// vector param has no `f32` to become.
    #[test]
    fn what_the_engine_cannot_carry_is_reported_rather_than_dropped() {
        let (_dir, store, l1, l4) = fixture();
        save(
            &store,
            "s1",
            plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
        )
        .expect("save");
        let mut lines = store.read_set("s1").expect("read");
        lines.push(Line::new(Record::Seed {
            stream: Layer::L4,
            index: 0,
            value: 7,
        }));
        lines.push(Line::new(Record::Capacity {
            layer: Layer::L4,
            index: 0,
            value: 128,
        }));
        lines.push(Line::new(Record::Param {
            layer: Layer::L1,
            index: None,
            key: "tint".to_string(),
            value: Value::Vec3([1.0, 0.0, 0.0]),
        }));

        let loaded = from_lines(&store, "s1", &lines).expect("load");
        let notes = loaded.notes.join("\n");
        assert!(notes.contains("seed on L4"), "{notes}");
        assert!(notes.contains("capacity on L4"), "{notes}");
        assert!(notes.contains("`tint`"), "{notes}");
        // The L1 values are still the ones applied: a note is not a refusal.
        assert_eq!(loaded.salts, vec![Some(1)]);
        assert_eq!(loaded.capacities, vec![Some(4096)]);
    }

    /// **A binding the engine cannot honour is reported and skipped**, and the
    /// load still succeeds. One unusable binding is not a reason to refuse the
    /// material, and the note names the parameter that will not move.
    #[test]
    fn an_unusable_binding_is_named_and_the_rest_of_the_set_still_loads() {
        let (_dir, store, l1, l4) = fixture();
        save(
            &store,
            "s1",
            plain(
                &ordinary(&store, &l1, std::slice::from_ref(&l4)),
                &[a_binding()],
            ),
        )
        .expect("save");
        let mut lines = store.read_set("s1").expect("read");
        lines.push(Line::new(Record::Bind {
            layer: Layer::L1,
            index: None,
            key: "radius".to_string(),
            signal: "bpm".to_string(),
            curve: "lin".to_string(),
            range: [0.0, 1.0],
            noise: None,
        }));

        let loaded = from_lines(&store, "s1", &lines).expect("load");
        assert_eq!(loaded.bindings.len(), 1, "the good binding survived");
        let notes = loaded.notes.join("\n");
        assert!(notes.contains("bpm"), "{notes}");
        assert!(notes.contains("skipped"), "{notes}");
    }

    /// The two diagnostics `docs/roadmap.md` says the decoder owes, asserted
    /// against the decoder rather than against the flag that used to hold them.
    #[test]
    fn the_decoder_carries_the_diagnostics_the_flag_used_to_hold_alone() {
        let bpm = Record::Bind {
            layer: Layer::L1,
            index: None,
            key: "radius".to_string(),
            signal: "bpm".to_string(),
            curve: "lin".to_string(),
            range: [0.0, 1.0],
            noise: None,
        };
        let err = binding_from_record(&bpm).expect_err("a tempo is not a [0,1] signal");
        assert!(err.contains("bar"), "{err}");

        let octaves = Record::Bind {
            layer: Layer::L1,
            index: None,
            key: "radius".to_string(),
            signal: NOISE_SIGNAL.to_string(),
            curve: "lin".to_string(),
            range: [0.0, 1.0],
            noise: Some(BindNoise {
                kind: "white".to_string(),
                octaves: 6,
                ..BindNoise::default()
            }),
        };
        let err = binding_from_record(&octaves).expect_err("white has no octaves");
        assert!(err.contains("fbm"), "{err}");
    }

    /// **Two `slot L1` lines are two geometries, and one address is one node.**
    ///
    /// The index used to be destructured and thrown away on this arm: every
    /// `slot L1` landed in the same entry, so a file describing two sources
    /// loaded as one. Index 1 is now the second geometry it always described,
    /// and what is left to report is the collision — two lines claiming index
    /// 0, which the projection folds to one whatever this loader does.
    #[test]
    fn a_second_geometry_is_carried_and_two_slots_at_one_address_are_reported() {
        let (_dir, store, l1, l4) = fixture();
        let put = |bytes: &[u8]| store.put_artifact(bytes).expect("put");
        let first = put(&std::fs::read(&l1).expect("read"));
        // A second geometry, distinguishable from the first by name so that
        // "the later one is used" is checked rather than asserted.
        let second = put(L1.replace("proc ring", "proc ring_two").as_bytes());
        let renderer = put(&std::fs::read(&l4).expect("read"));
        let lines = parsed(&format!(
            r#"{{"t":"set","id":"two","v":1}}
{{"t":"slot","layer":"L1","proc":"{first}"}}
{{"t":"slot","layer":"L1","proc":"{second}"}}
{{"t":"slot","layer":"L1","index":1,"proc":"{second}"}}
{{"t":"slot","layer":"L4","proc":"{renderer}"}}
"#
        ));

        let loaded = from_lines(&store, "two", &lines).expect("load");
        let notes = loaded.notes.join("\n");
        assert!(
            notes.contains("two L1 slots both claim index 0"),
            "a second geometry replaced the first in silence; notes were {notes:?}"
        );
        // Both geometries, in index order — the file named two sources and two
        // is what a Set holds.
        assert_eq!(
            loaded
                .l1s
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["ring_two", "ring_two"],
            "a geometry at index 1 was dropped; notes were {notes:?}"
        );
        // The later of the colliding pair is what was built, which is what the
        // note promises and the only reading under which the note is true.
        assert_eq!(loaded.l1s[0].name, "ring_two");
    }

    /// **A name the file recorded comes back, where it used to be reported and
    /// dropped.**
    ///
    /// "Nothing this build points at a node by name" was true until an `edge`
    /// did: an edge names the node that declares a slot and the node bound to
    /// it, so a load that dropped the names is a load whose edges resolve
    /// against the wrong spellings — or, for a name nobody wrote, against the
    /// procedure's own and by luck.
    #[test]
    fn a_name_the_file_recorded_comes_back_with_the_node() {
        let (_dir, store, l1, l4) = fixture();
        let put = |path: &std::path::Path| {
            store
                .put_artifact(&std::fs::read(path).expect("read"))
                .expect("put")
        };
        let (geometry, renderer) = (put(&l1), put(&l4));
        let lines = parsed(&format!(
            r#"{{"t":"set","id":"named","v":1}}
{{"t":"slot","layer":"L1","name":"veil","proc":"{geometry}"}}
{{"t":"slot","layer":"L4","proc":"{renderer}"}}
"#
        ));

        let loaded = from_lines(&store, "named", &lines).expect("a named slot still loads");
        assert_eq!(
            loaded.l1s[0].name, "ring",
            "the material is unaffected by the name"
        );
        assert_eq!(
            loaded.names.l1s,
            [Some("veil".to_string())],
            "the name the file recorded is what the node is called"
        );
        // **And an unnamed node stays unnamed** rather than being filled in
        // here: a name nobody wrote is derived where the Set is built, and
        // deriving it here as well would be the second place one fact lives.
        assert_eq!(loaded.names.l4s, [None]);
        let notes = loaded.notes.join("\n");
        assert!(
            !notes.contains("veil"),
            "a name that was honoured is not a note; notes were {notes:?}"
        );
    }

    /// **The whole chain, through the file and back.**
    ///
    /// A Set file recorded an L1 and its renderers: [`save`] refused an L2, an
    /// L3 or a `kind Field` outright, so a cube morphing into a sphere was a
    /// Set that could be played and not kept — and `--record-session` refused
    /// it with the same message, because a session opens with a Set file. Every
    /// layer is asserted separately, because writing them all as `L4` slots is
    /// exactly what used to happen and a count would not have noticed.
    #[test]
    fn the_whole_chain_survives_the_file_it_is_written_to() {
        let (dir, store, l1, l4) = fixture();
        let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
        let l2 = beside(&dir, "l2.kir", L2);
        let l3 = beside(&dir, "l3.kir", L3);
        let field = beside(&dir, "field.kir", FIELD);
        let l4b = beside(&dir, "l4b.kir", &L4.replace("proc points", "proc streaks"));

        let nodes = vec![
            Node {
                hash: stored(&store, &l1),
                layer: Kind::L1,
                index: 0,
                // A name the operator wrote, which is what `--set veil=l1.kir`
                // spells and what nothing here could record before.
                name: Some("veil".to_string()),
            },
            Node {
                hash: stored(&store, &l1b),
                layer: Kind::L1,
                index: 1,
                name: None,
            },
            Node {
                hash: stored(&store, &l2),
                layer: Kind::L2,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &l3),
                layer: Kind::L3,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &field),
                layer: Kind::Field,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &l4),
                layer: Kind::L4,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &l4b),
                layer: Kind::L4,
                index: 1,
                name: None,
            },
        ];
        save(
            &store,
            "chain",
            Saving {
                nodes: &nodes,
                capacities: &[4096, 8192],
                params: &[],
                bindings: &[],
                edges: &[],
                camera: &DEFAULT_CAMERA,
                layering: Layering::Overdraw,
                live: None,
                seeds: &[1, 2],
            },
        )
        .expect("save");
        assert!(
            written(&store, "chain").contains(r#""name":"veil""#),
            "a name the operator wrote went nowhere: {}",
            written(&store, "chain")
        );

        let loaded = load(&store, "chain").expect("load");
        let named = |procs: &[Checked]| procs.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
        assert_eq!(named(&loaded.l1s), ["ring", "ring_two"]);
        assert_eq!(named(&loaded.l2s), ["warp"]);
        assert_eq!(named(&loaded.l3s), ["look"]);
        assert_eq!(named(&loaded.fields), ["blob"]);
        assert_eq!(named(&loaded.l4s), ["points", "streaks"]);
        // Node order, which is what the scratch places them in — see
        // [`Loaded::nodes`].
        assert_eq!(
            loaded
                .nodes()
                .map(|(checked, _)| checked.name.clone())
                .collect::<Vec<_>>(),
            ["ring", "ring_two", "warp", "look", "points", "streaks", "blob"]
        );
        assert!(
            loaded.srcs[2].contains("proc warp"),
            "a node was paired with another node's source: {}",
            loaded.srcs[2]
        );
        // And the name it was written with, placed on the node it belongs to
        // rather than on whichever node the walk happened to reach.
        assert_eq!(loaded.names.l1s, [Some("veil".to_string()), None]);
        assert_eq!(loaded.names.l2s, [None]);
    }

    /// **Two fields, each at its own index, through the file and back.**
    ///
    /// The format could always say it — a `slot` record carries a layer and an
    /// index, and Field is a layer like any other — and the loader would not:
    /// it took `field_srcs.first()` and filed the rest under a note. So a Set
    /// whose marcher took a shape and a cutter saved as a Set that came back
    /// with one of them, and the edge naming the missing one no longer
    /// resolved.
    ///
    /// **Index as well as count**, because a pair that came back in the other
    /// order is a Set whose `--param Field:1:…` moves the wrong shape, and a
    /// count would not have noticed.
    #[test]
    fn two_fields_come_back_at_their_own_indices() {
        let (dir, store, l1, l4) = fixture();
        let shape = beside(&dir, "shape.kir", FIELD);
        let cutter = beside(&dir, "cutter.kir", &FIELD.replace("proc blob", "proc bite"));
        let nodes = vec![
            Node {
                hash: stored(&store, &l1),
                layer: Kind::L1,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &l4),
                layer: Kind::L4,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &shape),
                layer: Kind::Field,
                index: 0,
                name: None,
            },
            // Named, because an edge points with names and a Set holding two
            // fields is the first one that has to tell them apart.
            Node {
                hash: stored(&store, &cutter),
                layer: Kind::Field,
                index: 1,
                name: Some("knife".to_string()),
            },
        ];
        save(
            &store,
            "two_fields",
            Saving {
                nodes: &nodes,
                capacities: &[4096],
                params: &[],
                bindings: &[],
                edges: &[karakuri_engine::set::Edge {
                    node: "points".to_string(),
                    slot: "cutter".to_string(),
                    to: "knife".to_string(),
                }],
                camera: &DEFAULT_CAMERA,
                layering: Layering::Overdraw,
                live: None,
                seeds: &[1],
            },
        )
        .expect("save");

        let loaded = load(&store, "two_fields").expect("load");
        let named = |procs: &[Checked]| procs.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
        assert_eq!(named(&loaded.fields), ["blob", "bite"]);
        assert!(
            loaded.notes.is_empty(),
            "nothing was skipped: {:?}",
            loaded.notes
        );
        // The names, on the nodes they belong to — `None` for the one written
        // bare, which is what an edge pointing at `knife` needs to resolve.
        assert_eq!(loaded.names.fields, [None, Some("knife".to_string())]);
        // Node order, which is what the scratch places sources in: the fields
        // are last and they are in index order.
        assert_eq!(
            loaded
                .nodes()
                .map(|(checked, _)| checked.name.clone())
                .collect::<Vec<_>>(),
            ["ring", "points", "blob", "bite"]
        );
        assert_eq!(
            loaded.node_names().collect::<Vec<_>>(),
            [None, None, None, Some("knife".to_string())]
        );
        assert_eq!(loaded.edges.len(), 1, "the edge that binds the second");
    }

    /// **Each geometry runs at the number written against it.**
    ///
    /// The capacity was keyed by node in the format and by Set in this loader:
    /// a `capacity` on L1 index 1 was reported and dropped, so a Set whose two
    /// sources were sized differently came back with the second at whatever its
    /// `.kir` declared. The engine takes one per source and now so does this.
    #[test]
    fn each_geometry_keeps_the_capacity_it_was_saved_with() {
        let (dir, store, l1, l4) = fixture();
        let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
        let nodes = vec![
            Node {
                hash: stored(&store, &l1),
                layer: Kind::L1,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &l1b),
                layer: Kind::L1,
                index: 1,
                name: None,
            },
            Node {
                hash: stored(&store, &l4),
                layer: Kind::L4,
                index: 0,
                name: None,
            },
        ];
        save(
            &store,
            "two",
            Saving {
                nodes: &nodes,
                capacities: &[4096, 65_536],
                params: &[],
                bindings: &[],
                edges: &[],
                camera: &DEFAULT_CAMERA,
                layering: Layering::Overdraw,
                live: None,
                seeds: &[1, 2],
            },
        )
        .expect("save");

        let loaded = load(&store, "two").expect("load");
        assert_eq!(loaded.capacities, vec![Some(4096), Some(65_536)]);
        assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
    }

    /// **A salt is recorded per geometry and comes back per geometry**, which
    /// is what makes a saved Set reproduce its colours whatever order its
    /// records are in — `docs/ir-spec.md`, "A `source` value is assigned and
    /// recorded, never derived". This wrote one `seed` for the whole Set and
    /// read node 0's, so the second geometry's randomness was a function of
    /// where its path sat on the command line and of nothing in the file.
    ///
    /// **The bytes are asserted, not just the round trip.** Index 0 is absent
    /// and index 1 is written, which is the whole of what keeps the file a Set
    /// of one geometry has always written unchanged.
    #[test]
    fn each_geometry_keeps_the_salt_it_was_saved_with() {
        let (dir, store, l1, l4) = fixture();
        let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
        let nodes = vec![
            Node {
                hash: stored(&store, &l1),
                layer: Kind::L1,
                index: 0,
                name: None,
            },
            Node {
                hash: stored(&store, &l1b),
                layer: Kind::L1,
                index: 1,
                name: None,
            },
            Node {
                hash: stored(&store, &l4),
                layer: Kind::L4,
                index: 0,
                name: None,
            },
        ];
        save(
            &store,
            "two",
            Saving {
                nodes: &nodes,
                capacities: &[4096, 4096],
                params: &[],
                bindings: &[],
                edges: &[],
                camera: &DEFAULT_CAMERA,
                layering: Layering::Overdraw,
                live: None,
                seeds: &[7, 9],
            },
        )
        .expect("save");

        let text = written(&store, "two");
        assert!(
            text.contains("{\"t\":\"seed\",\"stream\":\"L1\",\"value\":7}\n")
                && text.contains("{\"t\":\"seed\",\"stream\":\"L1\",\"index\":1,\"value\":9}\n"),
            "{text}"
        );

        let loaded = load(&store, "two").expect("load");
        assert_eq!(loaded.salts, vec![Some(7), Some(9)]);
        assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
    }

    /// **A Set file written when a seed salted the whole Set still loads**, and
    /// says so by carrying one salt for the geometry it was written against.
    ///
    /// That is the older file's shape: one `seed` record, no index on it, and
    /// however many geometries. The geometry it names keeps the colours it was
    /// saved with; the ones it does not are salted the way an unsaved run is,
    /// which is what `None` in [`Loaded::salts`] asks the engine for. Refusing
    /// or defaulting either half would be a file that loads and draws something
    /// nobody saved.
    #[test]
    fn a_file_that_salted_the_whole_set_still_loads() {
        let (dir, store, l1, l4) = fixture();
        let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
        let put = |path: &std::path::Path| {
            store
                .put_artifact(&std::fs::read(path).expect("read"))
                .expect("put")
        };
        let lines = parsed(&format!(
            r#"{{"t":"set","id":"old","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L1","index":1,"proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{{"t":"seed","stream":"L1","value":4242}}
"#,
            put(&l1),
            put(&l1b),
            put(&l4)
        ));

        let loaded = from_lines(&store, "old", &lines).expect("load");
        assert_eq!(loaded.l1s.len(), 2);
        assert_eq!(
            loaded.salts,
            vec![Some(4242)],
            "the one seed the file carries salts the geometry it names, and the other \
             geometry is left to be derived"
        );
        assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
    }

    /// **A one-geometry Set is byte for byte the file it has always been.**
    ///
    /// Every Set file ever written is one L1 and its renderers, and the fields
    /// that carry a chain — `index` on every layer, `name` on a slot — are
    /// absent rather than defaulted for exactly this reason. A literal, not a
    /// re-save compared against itself: a round trip through one writer agrees
    /// with itself however far both halves have drifted.
    #[test]
    fn a_one_geometry_set_is_byte_for_byte_the_file_it_always_was() {
        let (_dir, store, l1, l4) = fixture();
        save(
            &store,
            "s1",
            plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
        )
        .expect("save");
        let hash = |path: &std::path::Path| {
            store
                .put_artifact(&std::fs::read(path).expect("read"))
                .expect("put")
        };
        assert_eq!(
            written(&store, "s1"),
            format!(
                r#"{{"t":"set","id":"s1","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{{"t":"capacity","layer":"L1","value":4096}}
{{"t":"camera","kind":"orbit","radius":8.0,"speed":0.15}}
{{"t":"seed","stream":"L1","value":1}}
"#,
                hash(&l1),
                hash(&l4)
            )
        );
    }

    /// **A composited Set is saved as one and comes back as one**, folded to
    /// the renderer it was folded to.
    ///
    /// This is the round trip the `merge` record exists for. A Set file could
    /// not say that a slot composites, so a variant pool written out came back
    /// overdrawing: no L5, no edges into one, and `Set::select_renderer` with
    /// nothing to select between. Both halves are asserted here because either
    /// one alone is useless — a layering that survived without its selection
    /// comes up folding every alternative at once, which is a different picture
    /// from the one that was saved.
    #[test]
    fn a_composited_set_comes_back_composited_and_still_folded_where_it_was() {
        let (dir, store, l1, l4) = fixture();
        let l4b = beside(
            &dir,
            "l4b.kir",
            &L4.replace("proc points", "proc points_two"),
        );
        let nodes = ordinary(&store, &l1, &[l4, l4b]);
        save(
            &store,
            "pool",
            Saving {
                layering: Layering::Composite,
                live: Some(1),
                ..plain(&nodes, &[])
            },
        )
        .expect("save");

        // The line itself, because the record's *presence* is the statement
        // and a test that only read the decoder back could pass on a writer
        // that wrote nothing and a reader that assumed everything.
        assert!(
            written(&store, "pool").contains(r#"{"t":"merge","live":1}"#),
            "the file does not carry the merge the Set was saved with:\n{}",
            written(&store, "pool")
        );

        let loaded = load(&store, "pool").expect("load");
        assert_eq!(
            loaded.layering,
            Layering::Composite,
            "a composited Set loaded back overdrawing"
        );
        assert_eq!(
            loaded.live,
            Some(1),
            "the fold came back with a different renderer live"
        );
        assert!(
            loaded.notes.is_empty(),
            "unexpected notes: {:?}",
            loaded.notes
        );
    }

    /// **A Set that overdraws writes no `merge` line at all**, and loads back
    /// overdrawing.
    ///
    /// The record's absence is how overdraw has always been spelled — there is
    /// no boolean field, because a record that could say `false` would be a
    /// second spelling of not writing one. So this asserts the *bytes*: a Set
    /// that overdraws is byte for byte the file it was before the record
    /// existed, and every file written by an older build reads as what it was.
    #[test]
    fn an_overdrawing_set_writes_no_merge_line_and_loads_back_overdrawing() {
        let (dir, store, l1, l4) = fixture();
        let l4b = beside(
            &dir,
            "l4b.kir",
            &L4.replace("proc points", "proc points_two"),
        );
        let nodes = ordinary(&store, &l1, &[l4, l4b]);
        save(&store, "stack", plain(&nodes, &[])).expect("save");

        let text = written(&store, "stack");
        assert!(
            !text.contains("merge"),
            "an overdrawing Set wrote a merge record:\n{text}"
        );

        let loaded = load(&store, "stack").expect("load");
        assert_eq!(
            loaded.layering,
            Layering::Overdraw,
            "a Set that recorded no merge loaded back compositing"
        );
        assert_eq!(loaded.live, None, "a Set with no merge recorded a fold");
    }

    /// **A composited Set nobody selected in writes no `live`**, and comes back
    /// with every renderer live.
    ///
    /// Absent is *not* renderer 0. Read that way it would silence every
    /// renderer but the first in every composited Set ever saved without a
    /// selection — a picture nobody asked for, from a file that said nothing
    /// had changed.
    #[test]
    fn a_composited_set_nobody_selected_in_writes_no_live() {
        let (dir, store, l1, l4) = fixture();
        let l4b = beside(
            &dir,
            "l4b.kir",
            &L4.replace("proc points", "proc points_two"),
        );
        let nodes = ordinary(&store, &l1, &[l4, l4b]);
        save(
            &store,
            "unselected",
            Saving {
                layering: Layering::Composite,
                live: None,
                ..plain(&nodes, &[])
            },
        )
        .expect("save");

        let text = written(&store, "unselected");
        assert!(
            text.contains(r#"{"t":"merge"}"#),
            "the bare merge record is not in the file:\n{text}"
        );

        let loaded = load(&store, "unselected").expect("load");
        assert_eq!(loaded.layering, Layering::Composite);
        assert_eq!(
            loaded.live, None,
            "a merge with no `live` came back naming a renderer"
        );
    }

    /// **A fold naming a renderer the file does not have is said and dropped.**
    ///
    /// A hand-written or hand-edited file can name one; the Set is whole either
    /// way, so the honest answer is to load it with every renderer live — the
    /// state it would have come up in — and say which line was not honoured.
    /// Checked where the `camera` index is checked and for its reason: how many
    /// renderers a file names is only known once every `slot` record has been
    /// met.
    #[test]
    fn a_fold_naming_a_renderer_that_is_not_there_is_reported_and_dropped() {
        let (_dir, store, l1, l4) = fixture();
        let nodes = ordinary(&store, &l1, std::slice::from_ref(&l4));
        save(
            &store,
            "wrong",
            Saving {
                layering: Layering::Composite,
                live: Some(3),
                ..plain(&nodes, &[])
            },
        )
        .expect("save");

        let loaded = load(&store, "wrong").expect("load");
        assert_eq!(loaded.layering, Layering::Composite, "the Set still folds");
        assert_eq!(loaded.live, None, "a fold nobody can honour was applied");
        assert!(
            loaded.notes.iter().any(|n| n.contains("renderer 3")),
            "nothing said which renderer was not there: {:?}",
            loaded.notes
        );
    }

    /// **Refused rather than written into a file that cannot be read back.**
    ///
    /// What [`save`] refuses is no longer a layer — it has a slot for every one
    /// — but the two shapes that are not a Set, and the two a projection keyed
    /// by address cannot fold: a repeat, and a gap.
    #[test]
    fn what_the_file_cannot_hold_is_refused_rather_than_written() {
        let (_dir, store, l1, l4) = fixture();
        // **Stored once and addressed many times**, which is exactly what a
        // content address buys: the two fixtures go in here, and every case
        // below is a different arrangement of the same two hashes. This used to
        // leak both paths to `'static` so that a borrowing `Node` could outlive
        // them; a `Hash` is `Copy` and there is nothing left to outlive.
        let (l1, l4) = (stored(&store, &l1), stored(&store, &l4));
        let node = |hash: Hash, layer, index| Node {
            hash,
            layer,
            index,
            name: None,
        };

        let cases: Vec<(Vec<Node>, &[u32], &str)> = vec![
            (
                vec![node(l4, Kind::L4, 0)],
                &[],
                "none of these files declares `kind L1`",
            ),
            (
                vec![node(l1, Kind::L1, 0)],
                &[4096],
                "none of these files declares `kind L4`",
            ),
            (
                vec![
                    node(l1, Kind::L1, 0),
                    node(l4, Kind::L4, 0),
                    node(l4, Kind::L4, 0),
                ],
                &[4096],
                "second node at that address",
            ),
            (
                vec![
                    node(l1, Kind::L1, 0),
                    node(l4, Kind::L4, 0),
                    node(l4, Kind::L4, 2),
                ],
                &[4096],
                "far side of a gap",
            ),
            (
                vec![node(l1, Kind::L1, 0), node(l4, Kind::L4, 0)],
                &[4096, 4096],
                "1 geometry and 2 capacities",
            ),
        ];
        for (nodes, capacities, expected) in cases {
            let err = save(
                &store,
                "bad",
                Saving {
                    nodes: &nodes,
                    capacities,
                    params: &[],
                    bindings: &[],
                    edges: &[],
                    camera: &DEFAULT_CAMERA,
                    layering: Layering::Overdraw,
                    live: None,
                    seeds: &[1],
                },
            )
            .expect_err("a file nothing could read back was written");
            assert!(err.contains(expected), "{err}");
        }
    }

    /// **A gap in a layer is refused on the way in too**, and on every layer:
    /// index 2 with no index 1 says a chain with a hole in it, and closing it up
    /// would silently change what deforms what — or, on L4, draw order.
    #[test]
    fn a_gap_in_a_chain_is_refused_rather_than_closed_up() {
        let (dir, store, l1, l4) = fixture();
        let l2 = beside(&dir, "l2.kir", L2);
        let put = |path: &std::path::Path| {
            store
                .put_artifact(&std::fs::read(path).expect("read"))
                .expect("put")
        };
        let lines = parsed(&format!(
            r#"{{"t":"set","id":"holed","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L2","index":1,"proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
"#,
            put(&l1),
            put(&l2),
            put(&l4)
        ));

        let err = from_lines(&store, "holed", &lines).expect_err("index 1 with no index 0");
        assert!(
            err.contains("names a L2 at index 0 but none before it"),
            "{err}"
        );
    }

    /// **A second camera is a second camera.** The format could address one
    /// all along and this loader used to read the first and report the rest as
    /// skipped — a refusal about the plumbing, which took a `Vec` here and in
    /// the engine to lift. Which renderer draws from which is an `edge`, so
    /// there is nothing here to arbitrate.
    #[test]
    fn a_second_camera_loads_beside_the_first() {
        let (dir, store, l1, l4) = fixture();
        let l3 = beside(&dir, "l3.kir", L3);
        let l3b = beside(&dir, "l3b.kir", &L3.replace("proc look", "proc look_two"));
        let put = |path: &std::path::Path| {
            store
                .put_artifact(&std::fs::read(path).expect("read"))
                .expect("put")
        };
        let lines = parsed(&format!(
            r#"{{"t":"set","id":"two_eyes","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L3","proc":"{}"}}
{{"t":"slot","layer":"L3","index":1,"proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
"#,
            put(&l1),
            put(&l3),
            put(&l3b),
            put(&l4)
        ));

        let loaded = from_lines(&store, "two_eyes", &lines).expect("two cameras load");
        assert_eq!(
            loaded
                .l3s
                .iter()
                .map(|c| c.name.clone())
                .collect::<Vec<_>>(),
            ["look", "look_two"],
            "the second camera was dropped"
        );
        assert!(
            loaded.notes.is_empty(),
            "two cameras are ordinary material now; notes were {:?}",
            loaded.notes
        );
    }

    /// **A `camera` record with no index is node 0's**, which is what every
    /// file ever written means by it: a Set held one camera, so there was one
    /// node for the record to describe, and a file that names no camera
    /// procedure still has the built-in at `L3:0`.
    ///
    /// The index exists because the L3 layer holds several now. The built-in
    /// orbit is the node after the procedures, and the only one a `camera`
    /// record can be about — a camera that is a procedure writes its own six
    /// numbers every frame — so an index naming one of those is said rather
    /// than applied to it.
    #[test]
    fn a_camera_record_with_no_index_is_node_zeros() {
        let (_dir, store, l1, l4) = fixture();
        let put = |path: &std::path::Path| {
            store
                .put_artifact(&std::fs::read(path).expect("read"))
                .expect("put")
        };
        let file = |camera: &str| {
            parsed(&format!(
                r#"{{"t":"set","id":"seen","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{camera}
"#,
                put(&l1),
                put(&l4)
            ))
        };

        // As a file written before the index existed spells it.
        let loaded = from_lines(
            &store,
            "seen",
            &file(r#"{"t":"camera","kind":"orbit","radius":7.5,"speed":0.5}"#),
        )
        .expect("load");
        assert_eq!(
            loaded.camera.map(|c| (c.radius, c.speed)),
            Some((7.5, 0.5)),
            "a `camera` record with no index has to reach the built-in camera"
        );
        assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);

        // Written out, the index is absent again: 0 is not serialised, so a
        // file this build saves is the line every earlier build wrote.
        save(
            &store,
            "written_back",
            Saving {
                nodes: &ordinary(&store, &l1, std::slice::from_ref(&l4)),
                capacities: &[65_536],
                params: &[],
                bindings: &[],
                edges: &[],
                camera: &DEFAULT_CAMERA,
                layering: Layering::Overdraw,
                live: None,
                seeds: &[1],
            },
        )
        .expect("save");
        let text = written(&store, "written_back");
        let line = text
            .lines()
            .find(|l| l.contains(r#""t":"camera""#))
            .expect("a camera record is written");
        assert!(!line.contains("index"), "index 0 is not written: {line}");

        // An index that is not the built-in's names a camera that is a
        // procedure, which produces its own state — said rather than applied.
        let loaded = from_lines(
            &store,
            "seen",
            &file(r#"{"t":"camera","kind":"orbit","index":1,"radius":7.5,"speed":0.5}"#),
        )
        .expect("load");
        assert!(
            loaded.camera.is_none(),
            "an orbit was applied to a camera node that produces its own state"
        );
        assert!(
            loaded.notes.iter().any(|n| n.contains("camera at L3:1")),
            "and it has to be said: {:?}",
            loaded.notes
        );
    }

    /// A `bind` record and a `Binding` are the same thing in two shapes, and
    /// `save` writes one from the other. **Every generator kind survives**, so
    /// a saved `fbm` does not come back as the perlin the default would give.
    #[test]
    fn every_noise_generator_survives_the_record_it_is_written_as() {
        for kind in [
            NoiseKind::White,
            NoiseKind::Value,
            NoiseKind::Perlin,
            NoiseKind::Fbm { octaves: 6 },
        ] {
            let binding = Binding::new(Kind::L1, "radius", NOISE_SIGNAL, Curve::Lin, [0.0, 1.0])
                .with_noise(NoiseConfig {
                    kind,
                    rate: 2.5,
                    stream: 3,
                });
            let back = binding_from_record(&record_from_binding(&binding))
                .unwrap_or_else(|e| panic!("{kind:?}: {e}"));
            assert_eq!(back.noise.map(|n| n.kind), Some(kind), "{kind:?}");
            assert_eq!(back.noise.map(|n| (n.rate, n.stream)), Some((2.5, 3)));
        }
    }
}
