//! Node addresses, ports, revisions, and parameter identifiers.

use super::layer::Layer;

/// A payload that has not been decided, on an operation that has.
///
/// The row is real — it is in `docs/manual/operations.html`, so it is part of
/// the vocabulary and every surface owes it a route — but what it acts on is a
/// question with more than one live answer, and picking one here would be this
/// type asserting a decision nobody made. So the name lands and the payload
/// says so.
///
/// Not a `TODO` and not an empty payload. An empty payload reads as *this
/// operation acts on nothing*, which is a claim, and a wrong one for every one
/// of these. This reads as *what this acts on is open*, which is true, and it
/// is a type — so the day the decision is made, replacing it is a compile error
/// at every construction site rather than a search.
///
/// `docs/contributing.md` §4, applied to a payload rather than to a sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Undecided;

/// The name a procedure's header gives one declared input — `far` in `uses far
/// : Geometry` — carried by [`Operation::WireInput`]'s `slot` field.
///
/// Its own type because `slot` names two unrelated things on this operation's
/// neighbours: a member of the deck, on every operation that takes one, and
/// this — the name a procedure reads a binding through. See
/// `docs/adr/0344-slot-is-disambiguated-into-three-types-and-adr-0049s-wait-is-over.md`.
///
/// Mirrors `karakuri_ir::typed::InputPort` and
/// `karakuri_store::record::InputPort` rather than depending on either. This
/// crate is a leaf by charter — see the crate documentation on why it has no
/// dependencies — so a shared type is not an option here the way it is between
/// `karakuri-engine` and `karakuri-ir`; three definitions of the same concept
/// is the cost, on [`NodeAddress`]'s own precedent below.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputPort(pub String);

impl InputPort {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for InputPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for InputPort {
    fn from(name: String) -> InputPort {
        InputPort(name)
    }
}

impl From<&str> for InputPort {
    fn from(name: &str) -> InputPort {
        InputPort(name.to_string())
    }
}

/// Which node of a deck's Set, on the terms `--param L4:1:name=value`,
/// `--publish level=L4:0:exposure`, the console's `L2:0` node heads and
/// `read_procedure`'s `{layer, index}` all already use.
///
/// The deck is not here. It is a field on the operation, as `slot` is a field
/// on every `karakuri_store::record::Record` that names a node — which keeps
/// one address shape for *within a Set* and one for *which Set*.
///
/// This is the positional address, and it is not the only one in the workspace.
/// An edge names its two ends by node *name*, deliberately, and
/// `karakuri_store::record::Record::Edge` gives the reason: *"a position moves
/// when the list is reordered, and reordering silently changing which geometry
/// a morph blends towards is the exact failure this record exists to end."* So
/// [`Operation::WireInput`] takes names and everything else takes this, and the
/// two are not interchangeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeAddress {
    pub layer: Layer,
    /// Which node of that layer, in the order the deck's files were named.
    pub index: u32,
}

/// Which version [`Operation::RestoreProcedure`] puts back, and the two arms
/// are the two things a surface can say rather than two features.
///
/// A surface says the half it holds
/// ([ADR-0192](../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
/// A staging lane row is a node with one unsettled version on it: it names the
/// node and means *the one this replaced*, and it holds no listing to pick a
/// row out of. A row of the Library bay's `history` scope is a version the
/// operator picked out of a listing, and what that row carries is the name the
/// store filed it under.
///
/// One enum rather than a `version: Option<String>` beside the `node`. That
/// shape can say a node and a version at once, and the two can then disagree —
/// a version of `L4:0` addressed to `L2:1` is a payload with two answers to
/// *which node*, and the one that would win is whichever the performer read.
/// Here the address is inside the arm that owns it, and a disagreement cannot
/// be spelled.
///
/// Neither arm is a path. A path may sit in a payload only where every route
/// that fills it derives it from something the program itself produced, and no
/// surface here spells one: a walk's row is a *name*, matched back against the
/// listing that produced it, exactly as `SetTransfer::Take`'s file is found
/// again by the word that was pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision {
    /// The version this node's present source replaced. One step, and never a
    /// cursor — walking further is [`Operation::WalkHistory`].
    Previous(NodeAddress),
    /// A version an operator picked out of a walk, by the name the store filed it
    /// under — `20260908-143052-271_slot0_L4_beat_strokes`.
    ///
    /// The name is the node's address as well as the moment, because that is how a
    /// version is filed
    /// (`docs/adr/0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md`),
    /// so this arm needs no [`NodeAddress`] beside it and a second spelling of the
    /// address is not invented here.
    Picked(String),
}

/// Which parameter of a deck's Set.
///
/// `node` absent is a wildcard, not node 0 — every node of the Set that
/// declares `key`. That is `Record::Param`'s rule and `Published::at`'s rule
/// and `--param exposure=2.0`'s meaning, and it is the useful default: one knob
/// moving every renderer that has an `exposure`.
///
/// A wildcard is refused where the nodes it lands on are not under one
/// authority, and the refusal names them. An authority is per node
/// (`docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`)
/// and a bare name reaches every node declaring the key, so one such control
/// spanning a node the operator kept and a node an agent acts on would hand the
/// first over through the second. Landing on the permitted nodes instead was
/// the alternative, and it lost to
/// `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`:
/// see
/// `docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`.
/// A surface owns none of this
/// (`docs/principles/0090-a-surface-offers-it-never-decides.md`): the rule
/// lives where the write lands, so every route meets it, and an addressed
/// [`ParamAt`] meets nothing — it says which node it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamAt {
    pub node: Option<NodeAddress>,
    /// The param's own name inside the node.
    pub key: String,
}

/// Which parameter of a deck's Set, as an attachment addresses one: a layer, a
/// node of it or all of them, and the key.
///
/// # It is not [`ParamAt`], and the difference is a fact about a binding
///
/// A binding is resolved through the nodes of one layer —
/// `karakuri_engine::binding::Binding` carries a required layer and an optional
/// index, and `Set::bind` walks `nodes_of(layer)` — so *every node of every
/// layer that declares this key*, which is exactly what a [`ParamAt`] with no
/// node means, is a set no attachment has ever been able to name. An operation
/// whose address could ask for it would be an operation refused at the far end
/// for a reason the vocabulary already knew, and
/// `karakuri_operation_record::written` would have to invent a layer to write
/// the record with — the placeholder `Record::Param`'s `layer` is and is stuck
/// being (ADR-0280 §1).
///
/// So the two addresses are two facts and not two spellings. A value has a
/// wildcard that names no layer, because a value lands wherever the name is
/// declared; an attachment's wildcard is a layer's, because the signal is
/// written into that layer's uniform buffer. `index` absent is *every node of
/// this layer declaring `key`*, which is `Record::Bind`'s rule and `--bind
/// L4:exposure=…`'s meaning.
///
/// A wildcard here meets no authority refusal, unlike [`ParamAt`]'s: an
/// authority governs who may write a node's params, and attaching a signal is
/// not a write of a value — it says what the value blends towards. What stops
/// one is the take-back, which is a control on the same row
/// (`docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindAt {
    pub layer: Layer,
    /// Which node of that layer, or every node of it declaring `key`. Present or
    /// absent as a unit with nothing — the layer is always said.
    pub index: Option<u32>,
    /// The param's own name inside the node, and a component key where the
    /// parameter is a vector: `glow.x` and never `glow`, because a binding resolves
    /// to one number and a `vec3` has three places to put it (ADR-0268).
    pub key: String,
}

/// A parameter's value. The three widths a `.kir` can declare, on
/// `karakuri_store::record::Value`'s terms — a value and never a range, since a
/// range is the procedure's declaration and not an operator's to write.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamValue {
    Scalar(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
}
