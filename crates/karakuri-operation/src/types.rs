//! Core domain types, targets, and parameters for the Karakuri operation vocabulary.

use crate::op::Operation;
use std::path::PathBuf;

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

/// Which layer of a Set a node sits on.
///
/// `Field` addresses no node in the rendering sense — it has no pass and no
/// buffers, and lowers into whoever evaluates it — but its params are declared
/// and addressable, which is why it is here at all. That paragraph is
/// `karakuri_store::record::Layer`'s, and this is a third spelling of that list
/// beside `karakuri_ir::ast::Kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    L1,
    L2,
    L3,
    L4,
    Field,
    /// A `kind L5` procedure — a frame effect over the picture handed to it. See
    /// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
    L5,
}

impl Layer {
    /// Every layer variant, in canonical order.
    pub const ALL: [Layer; 6] = [
        Layer::L1,
        Layer::L2,
        Layer::L3,
        Layer::L4,
        Layer::Field,
        Layer::L5,
    ];

    /// Name of the layer as used across configurations and displays.
    pub const fn name(&self) -> &'static str {
        match self {
            Layer::L1 => "L1",
            Layer::L2 => "L2",
            Layer::L3 => "L3",
            Layer::L4 => "L4",
            Layer::Field => "Field",
            Layer::L5 => "L5",
        }
    }
}

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// An error returned when parsing a [`Layer`] from a string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLayerError(String);

impl std::fmt::Display for ParseLayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown layer: `{}`; expected one of L1, L2, L3, L4, Field, L5",
            self.0
        )
    }
}

impl std::error::Error for ParseLayerError {}

impl std::str::FromStr for Layer {
    type Err = ParseLayerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "L1" => Ok(Layer::L1),
            "L2" => Ok(Layer::L2),
            "L3" => Ok(Layer::L3),
            "L4" => Ok(Layer::L4),
            "FIELD" => Ok(Layer::Field),
            "L5" => Ok(Layer::L5),
            _ => Err(ParseLayerError(s.to_string())),
        }
    }
}

/// Which kinds of row a library listing shows: the five procedure kinds and
/// Sets, each on or off, with an OR across the ones that are on and everything
/// where none is.
///
/// [`Operation::FilterLibrary`]'s payload, and the whole of it. Seven named
/// booleans rather than a list of members, because the list is closed by
/// construction — a procedure declares one of [`Layer`]'s six kinds and the
/// seventh row kind is a Set — and a `Vec` would admit a member said twice,
/// which is a state this control cannot be in.
///
/// Every state is said at once. A press names the whole row and never one chip,
/// which is [`Operation::Publish`]'s rule on a different list: *"adding or
/// removing one at a time is a statement about an entry, and an interface that
/// publishes nothing publishes everything is a statement about the list"* — and
/// it is what keeps two hands on one bay from disagreeing about which kinds are
/// showing.
///
/// The affordance is the surface's. Nothing here says *toggle*
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md));
/// a chip that flips one field and sends all six is the console's arithmetic,
/// exactly as the blend chip's cycle is.
///
/// See
/// `docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryKinds {
    pub l1: bool,
    pub l2: bool,
    pub l3: bool,
    pub l4: bool,
    pub field: bool,
    /// The seventh, and it arrived the day `kind L5` did: a frame effect is a
    /// procedure with a `kind` like any other, so it is a row of the Library and
    /// the filter row has a toggle for it. See
    /// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
    pub l5: bool,
    /// Sets, which is the one row kind that is not a procedure's `kind` — a Set
    /// fills several layers and declares none, so *is this its kind* is not a
    /// question it answers.
    pub sets: bool,
}

impl LibraryKinds {
    /// Nothing narrowed, which shows everything and is where a run begins. It is
    /// also the state a press can always get back to, which is what
    /// [`P-0090`](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// asks of a control with more than two positions.
    pub const EVERYTHING: LibraryKinds = LibraryKinds {
        l1: false,
        l2: false,
        l3: false,
        l4: false,
        field: false,
        l5: false,
        sets: false,
    };

    /// Whether any button is on. `false` is [`LibraryKinds::EVERYTHING`], and the
    /// two readings of it — *nothing shows* and *everything shows* — are settled
    /// here rather than at each caller: everything, because a filter row that could
    /// hide the whole listing would have a state an operator cannot see their way
    /// out of.
    pub fn narrowing(&self) -> bool {
        self.l1 || self.l2 || self.l3 || self.l4 || self.field || self.l5 || self.sets
    }

    /// Whether a procedure of `layer` is shown. `true` for every layer while
    /// nothing is on, which is [`LibraryKinds::narrowing`]'s answer applied.
    pub fn shows_layer(&self, layer: Layer) -> bool {
        if !self.narrowing() {
            return true;
        }
        match layer {
            Layer::L1 => self.l1,
            Layer::L2 => self.l2,
            Layer::L3 => self.l3,
            Layer::L4 => self.l4,
            Layer::Field => self.field,
            Layer::L5 => self.l5,
        }
    }

    /// Whether a Set is shown, on [`LibraryKinds::shows_layer`]'s terms.
    pub fn shows_sets(&self) -> bool {
        !self.narrowing() || self.sets
    }
}

/// What a deck's clock is locked to. `karakuri_engine::transport::Sync`'s
/// three, in the order a control cycles them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sync {
    Free,
    Tempo,
    Beat,
}

impl Sync {
    /// The lower-case word for this mode, which is the one every surface spells it
    /// with: `Record::Transport`'s wire `sync`, `karakuri-cli`'s status line, and a
    /// map file's value.
    ///
    /// A match rather than a table, exactly as [`BlendMode::name`] is one and for
    /// its reason: a mode added to the enum does not compile until it has a name.
    /// The three words are `karakuri_engine::transport::Sync::name`'s, because a
    /// record carries a name and the engine is what reads it back.
    pub fn name(self) -> &'static str {
        match self {
            Sync::Free => "free",
            Sync::Tempo => "tempo",
            Sync::Beat => "beat",
        }
    }
}

/// What one step of a sequencer pattern is worth: a sixteenth or an eighth, and
/// the list is closed.
///
/// The pattern is one bar, fixed, so the count follows the mode rather than
/// being a second thing a hand sets — sixteen cells at a sixteenth and eight at
/// an eighth, the row keeping its width so the cells halve in the finer one
/// (`docs/adr/0306-the-grid-head-is-one-pill-because-the-bar-is-one-bar-and-the-count-follows-the-mode.md`).
///
/// It is the pattern's and not the session's or a lane's, which is the
/// console's own sentence about the pill that draws it: *"It is armed because
/// it is what the pattern is rather than a preference the head is holding."* So
/// [`Operation::SetPatternGrid`] names the bank it is the mode of.
///
/// A pattern stores sixteen slots in both modes and an eighth reads slot `2k`,
/// so this is a change of *reading* and never of the pattern — which is
/// [`StepMode::slot_of`] and
/// `docs/adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md`.
/// The stored width is `karakuri_pattern`'s, because it belongs to the thing
/// that holds the steps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StepMode {
    /// Sixteen steps to the bar, four to the beat. What the console's ruler and
    /// cells have been drawn in since the mock's first commit, and what a pattern
    /// nobody has pressed the pill on is in.
    #[default]
    Sixteenth,
    /// Eight steps to the bar, two to the beat.
    Eighth,
}

impl StepMode {
    /// Both values, in the order the pill names them.
    pub const ALL: [StepMode; 2] = [StepMode::Sixteenth, StepMode::Eighth];

    /// The word the head's pill reads, which is the mock's own spelling.
    ///
    /// A match rather than a table, exactly as [`Sync::name`] and
    /// [`BlendMode::name`] are and for their reason: a mode added to this enum does
    /// not compile until it has a name.
    pub fn name(self) -> &'static str {
        match self {
            StepMode::Sixteenth => "1/16",
            StepMode::Eighth => "1/8",
        }
    }

    /// How many steps there are in the bar at this mode — sixteen and eight. The
    /// bar is fixed, so this follows the mode and is not a second choice beside it
    /// (ADR-0306).
    pub fn count(self) -> usize {
        match self {
            StepMode::Sixteenth => 16,
            StepMode::Eighth => 8,
        }
    }

    /// The multiplier in the step index, which is `floor(beats × steps_per_beat)
    /// mod count` — ADR-0222's own formula, a pure function of `Oscillator::beats`.
    pub fn steps_per_beat(self) -> f64 {
        match self {
            StepMode::Sixteenth => 4.0,
            StepMode::Eighth => 2.0,
        }
    }

    /// Which of the sixteen stored slots step `step` reads.
    ///
    /// The identity at a sixteenth and `2k` at an eighth, which is what makes a
    /// mode press a change of reading: the finer mode and back returns exactly what
    /// was there, and an eighth-mode step sits at the same musical instant as the
    /// sixteenth it is drawn over. Sizing the store to the count instead would
    /// throw half a bar away on one press with nothing to confirm against, which is
    /// the alternative ADR-0320 refuses.
    pub fn slot_of(self, step: usize) -> usize {
        match self {
            StepMode::Sixteenth => step,
            StepMode::Eighth => step * 2,
        }
    }
}

/// What a sequencer lane drives: an operation of this vocabulary with its value
/// left out.
///
/// A lane is a fifth route into this vocabulary rather than a binding
/// (`docs/adr/0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md`), so
/// what it needs is not a new operation but an *address* — and the address a
/// lane wants is one of these arms plus the level the step is worth, which is
/// [`LaneTarget::operation`].
///
/// This is the bay's sharpest question and this is the answer
/// (`docs/adr/0321-a-lanes-target-is-an-operation-with-its-value-elided.md`).
/// The console draws four lanes — three deck faders and a Set parameter — and
/// the two obvious spellings each reach one kind and not the other: a
/// published-interface position is a control a *Set* declares, and
/// `Record::Opacity` is no Set's; a slot number cannot say which parameter. An
/// operation minus its value reaches all four, because the vocabulary already
/// addresses both.
///
/// Two of this vocabulary's rows are not lane targets, and it is one reason:
/// [`Operation::SetResidency`] and [`Operation::SetBlend`] take a word from a
/// closed list rather than a level, and a step is a level — so a lane pointed
/// at one would have to invent the word an on-step means. It is written here
/// rather than left to be noticed from this enum's silence.
///
/// [`Operation::SetGain`], [`Operation::SetMaskPosition`],
/// [`Operation::SetMasterOut`] and [`Operation::SetExposure`] are each one arm
/// and one line of [`LaneTarget::operation`] — additions to a closed list, so
/// none of them is a decision and their absence is scope rather than a gap.
#[derive(Debug, Clone, PartialEq)]
pub enum LaneTarget {
    /// A deck's channel fader — three of the four lanes the console draws.
    Fader { deck: u8 },
    /// A parameter inside the Set on a deck — the fourth.
    ///
    /// A vector parameter costs nothing extra:
    /// `docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md`
    /// makes [`ParamAt::key`] `glow.x` and never `glow`, so a lane reaches a
    /// component by the road `--param` reaches it by and needs no field of its own.
    Param { deck: u8, param: ParamAt },
}

impl LaneTarget {
    /// The operation this lane emits at a step worth `value`.
    ///
    /// [`crate::gate`]'s discipline applied to an address: an exhaustive `match`,
    /// so a target added does not compile until it says what it emits. It is
    /// `karakuri_console::panel::Knob::operation` and
    /// `karakuri_midi::map::Target::operation` a third time — *an address plus a
    /// value becomes an operation* — which is why this is not a new mechanism.
    pub fn operation(&self, value: f32) -> Operation {
        match self {
            LaneTarget::Fader { deck } => Operation::SetOpacity {
                deck: *deck,
                opacity: value,
            },
            LaneTarget::Param { deck, param } => Operation::WriteParam {
                deck: *deck,
                param: param.clone(),
                value: ParamValue::Scalar(value),
            },
        }
    }

    /// Which deck this lane writes into, which is what a caller asking *does a lane
    /// hold this control* starts from
    /// (`docs/adr/0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md`).
    pub fn deck(&self) -> u8 {
        match self {
            LaneTarget::Fader { deck } | LaneTarget::Param { deck, .. } => *deck,
        }
    }
}

/// What a deck slot is *for*. `karakuri_engine::deck::Residency`'s three, in
/// the order they cost.
///
/// This is the request, and the engine keeps two. The operator writes one
/// through `Deck::set_residency`; the governor derives the other from it and
/// the budget, holding a slot below what was asked for and never above. Only
/// the request is an operation, so this enum names three destinations and says
/// nothing about which of them the deck arrived at — a surface reads that back
/// rather than assuming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Residency {
    /// Stepped and composited. Honoured: nothing demotes a live slot.
    Live,
    /// Stepped out of sight, warming its buffers, contributing nothing to the mix.
    /// A request the governor reconsiders every pass, and parks rather than refuses
    /// when there is no room.
    Priming,
    /// Compiled, buffers held, not stepping. Keeps its `t`, so a slot taken here
    /// and brought back resumes where it stopped.
    Allocated,
}

impl Residency {
    /// Every residency there is, in the order they cost — the order this enum
    /// declares them and the order `karakuri_engine::deck::Residency` does.
    ///
    /// A list is not a cycle, exactly as [`BlendMode::ALL`] is not: what a surface
    /// needs from the vocabulary is *which values exist*, and a control that steps
    /// through them is an affordance built over the three operations they name
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    /// `karakuri-midi`'s map reads it to decide what a `residency N <word>` line
    /// may end in, so a value added here is offered to a map file rather than
    /// waiting for a parser's second list to catch up.
    pub const ALL: [Residency; 3] = [Residency::Live, Residency::Priming, Residency::Allocated];

    /// The lower-case word for this level, which is what `Record::Residency`
    /// carries and what `karakuri-cli`'s `mix::residency_wire_name` writes. The
    /// status line's `LIVE`/`prim`/ `park` is a different vocabulary for a
    /// different reader and is deliberately not this one.
    ///
    /// A match rather than a table, for [`BlendMode::name`]'s reason.
    pub fn name(self) -> &'static str {
        match self {
            Residency::Live => "live",
            Residency::Priming => "priming",
            Residency::Allocated => "allocated",
        }
    }
}

/// Where a composited frame goes, by name.
///
/// An output is a destination, a size and an on/off
/// ([ADR-0324](../../../docs/adr/0324-an-output-is-a-named-destination-with-a-size-and-an-on-off.md)).
/// This is the destination half — the only half a *vocabulary* can carry,
/// because the other two are answers rather than names: the size is the window
/// manager's or the Program bay's, and whether it is on is read where it is
/// kept.
///
/// # It is a closed list and not a string
///
/// A label moves and an identity may not. The console's projector chip carries
/// the display it is on — `projector · DELL U2720Q` in the mock — and that
/// string changes when the cable does, when the display is renamed, and when
/// the same window is dragged to the other screen. A name a map line or an MCP
/// call held would then name nothing, and there would be nothing to refuse it
/// with: a string parses whatever it is given, so *there is no such output*
/// would be a sentence somebody had to remember to write at every route in,
/// where a variant is a compile error at the one that mistyped it.
///
/// This crate owns the enumerations a destination is drawn from rather than
/// passing strings, which is
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// and the reason [`Residency`], [`BlendMode`], [`WipeKind`] and the rest are
/// here at all. An output is the same kind of thing they are: a word from a
/// closed list, which is exactly what `karakuri-midi`'s map file can name and
/// what an MCP argument can be validated against.
///
/// And a string would allocate. This crate has no `[dependencies]` and every
/// payload in it is a number or a word; a `String` on the one operation a sink
/// press emits would put an allocation on the path a frame's publishing is
/// switched from.
///
/// # Why the picture is a variant and a preview cell is not
///
///
/// [ADR-0243](../../../docs/adr/0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md)
/// settled it: the picture in the Program bay is in the set that needs naming
/// and the four cells are not. A cell is welded to the deck letter under it,
/// nothing routes one, and a control that names an output can never name one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// The picture in the Program bay, and the first row of the list.
    ///
    /// Its size is the rectangle the bay gives it and its on/off is the picture's
    /// own fold — read where the arrangement keeps it rather than stored a second
    /// time here, which is
    /// [ADR-0161](../../../docs/adr/0161-solo-remembers-which-region-because-it-cannot-be-derived.md)'s
    /// rule read on a sink: a second copy is a copy that drifts.
    Program,
    /// A window this program opens, numbered from zero in the order the list draws
    /// them.
    ///
    /// One is built. The number is here rather than deferred because a second
    /// projector is a list entry and not a new kind of thing, and a variant that
    /// has to be widened later is a compile error at every construction site —
    /// which is the cost this payload was `Undecided` to avoid paying twice.
    Projector(u8),
    /// A sink a plugin brings, by its place in the manifest that loaded it.
    ///
    /// Nothing constructs one, and that is a statement about the manifest rather
    /// than about this variant: `docs/plugins.md` specifies the process and the
    /// handshake, nothing implements them, and the console draws Syphon and NDI as
    /// `no plugin`. The index is the manifest's own order for the same reason
    /// [`Output::Projector`] carries one — a plugin's *name* is a label it chose,
    /// and a label is not an identity.
    Plugin(u8),
}

impl Output {
    /// Every output this program can name today, which is the program view and one
    /// projector window.
    ///
    /// It is not every variant, and the difference is the point: a
    /// [`Output::Plugin`] exists in the type so that the day a manifest is read the
    /// list grows rather than the vocabulary changing, and a surface drawing this
    /// list would draw a chip for a plugin that is not there. A surface that wants
    /// the absent ones draws them from what it knows is missing, which is what
    /// `karakuri-console` does.
    pub const ALL: [Output; 2] = [Output::Program, Output::Projector(0)];

    /// The lower-case word for this destination, for a caller that prints one. A
    /// projector and a plugin carry their index, because two of either are two
    /// outputs and a reader has to be able to tell them apart.
    ///
    /// A `String` rather than a `&'static str` for exactly that reason, and it is
    /// the one thing in this crate that allocates — off the frame path, in a line
    /// somebody reads.
    pub fn name(self) -> String {
        match self {
            Output::Program => "program view".to_owned(),
            Output::Projector(n) => format!("projector {n}"),
            Output::Plugin(n) => format!("plugin {n}"),
        }
    }
}

/// How a deck meets the ones under it in the fold.
///
/// Named `BlendMode` rather than `Blend` on purpose. `Blend` already means two
/// different things here — `karakuri_engine::deck::Blend` is how a deck meets
/// the mix, `karakuri_ir::ast::Blend` is how an L4's fragments meet each other
/// — and a name means one thing across the system (`docs/contributing.md` §4).
/// A third `Blend` would make it three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Add,
    Over,
    Max,
}

impl BlendMode {
    /// Every mode there is, in the engine's own order —
    /// `karakuri_engine::deck::Blend::ALL`, which states it as *"in cycle order.
    /// `Add` first, because it is the default and a cycle should start where a slot
    /// starts."*
    ///
    /// A list is not a cycle, and this crate does not own the cycle. A control that
    /// steps through these is an affordance built over the three operations they
    /// name, and it belongs to whoever draws the control
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). What this is
    /// for is the two things a surface genuinely needs from the vocabulary: *which
    /// values exist*, and *in what order they are conventionally shown*. The mixer
    /// strip's blend chip does its own arithmetic over this
    /// ([ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)),
    /// and a MIDI map is offered the three values rather than a step.
    pub const ALL: [BlendMode; 3] = [BlendMode::Add, BlendMode::Over, BlendMode::Max];

    /// The lower-case word for this mode, which is the one every surface spells it
    /// with: `Record::Blend`'s wire `mode`, `karakuri-cli`'s status line, a map
    /// file's value, and the word a mixer strip's chip draws.
    ///
    /// A match rather than a table, exactly as `karakuri_engine::deck::Blend::name`
    /// is one, and for its reason: a mode added to the enum does not compile until
    /// it has a name. A table indexed by discriminant would take a new variant
    /// silently and hand out the wrong word or panic.
    ///
    /// The three words are the engine's, because a record carries a name and the
    /// engine is what reads it back. This is a copy of that list, which is the cost
    /// this crate pays on purpose — see the module documentation.
    pub fn name(self) -> &'static str {
        match self {
            BlendMode::Add => "add",
            BlendMode::Over => "over",
            BlendMode::Max => "max",
        }
    }
}

/// The transfer from unbounded linear HDR to something displayable.
/// `karakuri_engine::present::TonemapOp`'s four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tonemap {
    Clamp,
    Reinhard,
    Aces,
    AgX,
}

impl Tonemap {
    /// The lower-case word for this operator, which is what `Record::Look`'s `op`
    /// carries and what `--tonemap` takes.
    ///
    /// The wire spelling and never the reader's: `karakuri-cli`'s `spellings` keeps
    /// two per operator — `("aces", "ACES")` — because one of them is parsed and
    /// the other is only read. This is the parsed one, for [`BlendMode::name`]'s
    /// reason: an operator added to the enum does not compile until it has a name.
    pub fn name(self) -> &'static str {
        match self {
            Tonemap::Clamp => "clamp",
            Tonemap::Reinhard => "reinhard",
            Tonemap::Aces => "aces",
            Tonemap::AgX => "agx",
        }
    }
}

/// Which frame the master chain's feedback pass reads back.
/// `karakuri_engine::master::Cut`'s two, mirrored here rather than imported for
/// this crate's own reason — see the module documentation.
///
/// The maintainer's decision on 2026-09-09 was *both, selectable*: the two are
/// different pictures and a design that picked one would be taking a decision
/// away from a hand. See
/// `docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cut {
    /// The frame as the mix wrote it, before this chain touched it. One echo of the
    /// previous frame and not a trail: nothing read back has been fed back.
    #[default]
    Mix,
    /// The chain's own exit, after rgb shift and before the tone map. A trail,
    /// because what is read back already contains it.
    Exit,
}

impl Cut {
    /// Both, in the order a surface shows them. A list is not a cycle —
    /// [`BlendMode::ALL`]'s rule, and the console's own chip does the arithmetic
    /// over these two.
    pub const ALL: [Cut; 2] = [Cut::Mix, Cut::Exit];

    /// The lower-case word for this cut, which is what
    /// `karakuri_store::record::Record::MasterChain` carries and what a map file
    /// spells. [`BlendMode::name`]'s rule: a cut added to the enum does not compile
    /// until it has a name.
    pub fn name(self) -> &'static str {
        match self {
            Cut::Mix => "mix",
            Cut::Exit => "exit",
        }
    }
}

/// What a feedback slot of the master chain is set to: how much of the retained
/// frame comes back, and which frame that is.
///
/// A reading and not an ask. No operation carries it — a surface sets one slot
/// of the chain at a time through [`Operation::SetChainParam`] — and what holds
/// it is `karakuri_operation_record::Chain`, the reading a conversion completes
/// a whole-chain record from.
///
/// The same amount is a one-frame echo under [`Cut::Mix`] and a compounding
/// trail under [`Cut::Exit`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Feedback {
    /// `[0, 0.95]`, and the ceiling is the engine's: at 1.0 the exit cut is an
    /// accumulator with no decay in it. Clamped where the record is applied and not
    /// here — `karakuri_engine::master::Chain::clamped`.
    pub amount: f32,
    /// Which frame the amount is of.
    pub cut: Cut,
}

impl Feedback {
    /// The most a surface may ask for, and the reason it is short of 1.0 is the
    /// engine's: under [`Cut::Exit`] the pass is an accumulator with no decay in
    /// it, so 1.0 runs still material away to infinity. At 0.95 the ceiling is
    /// twenty times the frame.
    ///
    /// This is the reach a control draws, and it is not the wall. The wall is
    /// `karakuri_engine::master::Chain::clamped`, where the record is applied, so a
    /// MIDI map and a model meet it too
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). The number is
    /// here as well so a fader can be laid out without reading the engine, and the
    /// two being one number is asserted where both are visible — `crates/karakuri`,
    /// which depends on this crate and on the engine. That is [`Tonemap`]'s
    /// arrangement for a range instead of a list.
    pub const MAX: f32 = 0.95;
}

/// What [`Operation::SetChainParam`] sets on one slot of the master chain.
///
/// A slot holds one `kind L5` procedure, the values of the parameters that
/// procedure declares, and — where it declares `retains` — the cut it reads.
/// Those are the two kinds of thing a slot is set to: a declared parameter is a
/// number under a name the procedure chose, and a cut is one word of a closed
/// list. One is set at a time
/// (`docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md`).
///
/// See
/// `docs/adr/0348-a-chain-slots-cut-is-set-through-the-parameter-row.md`.
#[derive(Debug, Clone, PartialEq)]
pub enum ChainParam {
    /// One parameter the slot's procedure declares, set outright.
    Declared {
        /// The declaration's own name — `amount` in `param amount : float`. A key
        /// the procedure does not declare is refused where the slot is written.
        key: String,
        /// What it is set to. The declaration's range is where a value is held,
        /// and this crate holds it nowhere.
        value: f32,
    },
    /// Which retained frame the slot reads, on a slot whose procedure declares
    /// `retains`. Refused on a slot whose procedure does not.
    Cut(Cut),
}

/// How a signal is shaped on its way to a parameter.
/// `karakuri_engine::binding::Curve`'s four, and `docs/ir-spec.md`'s names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    Lin,
    Pow2,
    Sqrt,
    Smooth,
}

impl Curve {
    /// Every shape there is, in the order the specification documents them — the
    /// identity, the peak-weighted, its floor-weighted complement, and the one
    /// eased at both ends.
    ///
    /// A list is not a cycle, on [`Authority::ALL`]'s and
    /// `karakuri_engine::deck::Blend::ALL`'s terms: the sensitivity row's curve
    /// chip is an affordance built over these four, the cycle belongs to whoever
    /// draws it, and what crosses this seam is [`Operation::AttachSignal`] naming a
    /// destination (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    pub const ALL: [Curve; 4] = [Curve::Lin, Curve::Pow2, Curve::Sqrt, Curve::Smooth];

    /// The lower-case word a record spells, which is what
    /// `karakuri_store::record::Record::Transition`'s `curve` carries and what a
    /// `bind` record has always carried.
    ///
    /// It arrived later than [`BlendMode::name`] and its neighbours because nothing
    /// needed it: the one operation carrying a curve — [`Operation::AttachSignal`]
    /// — wrote no session record, so this list had no wire to reach. A scheduled
    /// move does: the shape a fade takes is part of what a replay reconstructs it
    /// from, and `karakuri-operation-record` is where a curve becomes a name.
    /// `AttachSignal` writes one now too, and it is `Record::Source`'s `curve` —
    /// the same spelling, so a fade's shape and an attachment's cannot drift apart
    /// on the wire.
    ///
    /// A match rather than a table, for [`BlendMode::name`]'s reason: a curve added
    /// to the enum does not compile until somebody has spelled it.
    pub fn name(self) -> &'static str {
        match self {
            Curve::Lin => "lin",
            Curve::Pow2 => "pow2",
            Curve::Sqrt => "sqrt",
            Curve::Smooth => "smooth",
        }
    }
}

/// The shape a wipe's front takes. `karakuri_engine::deck::MaskKind`'s three.
///
/// A shape is this and an angle, which is why it is not the six-item list the
/// `z` key cycles: `karakuri-cli`'s `MASK_SHAPES` is a curated six chosen out
/// of an unbounded `(kind, angle)` pair — *"an arbitrary angle is a dial, and a
/// dial with nowhere to show its value is a control an operator cannot read"* —
/// and the curation is a keyboard's compromise, not the operation. A panel dial
/// and an MCP call can both say an angle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeKind {
    /// No mask. `c` is refused under it, which the manual states.
    None,
    /// A straight front crossing the frame at an angle.
    Linear,
    /// A circle opening from the middle.
    Radial,
}

impl WipeKind {
    /// The lower-case word for this shape, which is what `Record::Mask`'s `kind`
    /// carries and what the engine reads back.
    ///
    /// A match rather than a table, for [`BlendMode::name`]'s reason: a shape added
    /// to the enum does not compile until it has a name. The three words are
    /// `karakuri_engine::deck::MaskKind::name`'s, because a record carries a name
    /// and the engine is what reads it back.
    ///
    /// There is no `ALL` beside it, where [`BlendMode`] and [`Residency`] have one.
    /// That constant exists so a map file can be offered the values a target may
    /// end in, and no map target names a shape — see the row at
    /// [`Operation::SetMaskShape`]. It arrives with the first reader.
    pub fn name(self) -> &'static str {
        match self {
            WipeKind::None => "none",
            WipeKind::Linear => "linear",
            WipeKind::Radial => "radial",
        }
    }
}

/// Who may move one node of a Set. The manual's sixth rule, as a list of three:
/// *"Each node of a Set is manual, suggesting, or automatic, and you set that
/// node by node."*
///
/// A permission granted forward, and not a record of who moved something last.
/// The second is rule 02 — *"a parameter driven by something else shows its
/// source instead of a number"* — and it is read off the binding that is
/// driving the param. This is the other question, asked before anything moves:
/// what an agent is *allowed* to do to this node.
///
/// Three destinations and no toggle, which is [`Residency`]'s shape and
/// [`BlendMode`]'s: an operation names one of them outright, and a control that
/// steps through them is an affordance built over the three
/// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). The console
/// draws that affordance as `man / sug / auto` on a node head, and those three
/// words are a surface's abbreviations rather than this list — exactly as the
/// status line's `LIVE`/`prim`/`park` is not [`Residency::name`].
///
/// This is a copy, and the engine holds the list it is a copy of. It was not
/// one when it landed — `karakuri-engine` held no authority at all, which is
/// what
/// `docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`
/// says the engine still owed — and it became one when
/// `karakuri_engine::set::Authority` arrived with the per-node flag a rebuild
/// restates. So it is now [`Residency`]'s and [`Sync`]'s case exactly, and it
/// is the cost the module documentation states: what a node's authority is
/// allowed to be is the engine's to say, this crate names the same three so a
/// surface can refuse a typo without a device, and the two are checked against
/// each other in `karakuri-cli`'s `mix.rs` where every other pair already is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authority {
    /// Yours alone. Nothing else writes this node's params.
    Manual,
    /// An agent proposes and you accept.
    Suggesting,
    /// An agent acts.
    Automatic,
}

impl Authority {
    /// The lower-case word for this level, which is what
    /// `karakuri_store::record::Record::Authority` carries.
    ///
    /// A match rather than a table, for [`BlendMode::name`]'s reason: a level added
    /// to the enum does not compile until it has a name. The three words are rule
    /// 06's own — *manual*, *suggesting*, *automatic* — and not the console's `man
    /// / sug / auto`, which is a node head's abbreviation for a reader rather than
    /// a name a record is read back with.
    ///
    /// There is no `ALL` beside it, on [`WipeKind`]'s terms exactly: that constant
    /// exists so a map file can be offered the values a target may end in, and no
    /// map target names an authority — a map line cannot say a node address at all,
    /// which is what the manual's gap section says of MIDI. It arrives with the
    /// first reader.
    pub fn name(self) -> &'static str {
        match self {
            Authority::Manual => "manual",
            Authority::Suggesting => "suggesting",
            Authority::Automatic => "automatic",
        }
    }
}

/// Slot-level MCP modification policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlotPolicy {
    /// Automatically permit writes when off-air (fader == 0 or muted),
    /// and protect against writes when on-air.
    #[default]
    Auto,
    /// Always allow MCP modifications to this slot.
    On,
    /// Always block MCP modifications to this slot.
    Off,
}

impl SlotPolicy {
    pub const ALL: [SlotPolicy; 3] = [SlotPolicy::Auto, SlotPolicy::On, SlotPolicy::Off];

    pub fn name(self) -> &'static str {
        match self {
            SlotPolicy::Auto => "auto",
            SlotPolicy::On => "on",
            SlotPolicy::Off => "off",
        }
    }

    pub fn pill_word(self) -> &'static str {
        match self {
            SlotPolicy::Auto => "mcp · auto",
            SlotPolicy::On => "mcp · on",
            SlotPolicy::Off => "mcp · off",
        }
    }

    pub fn next(self) -> SlotPolicy {
        match self {
            SlotPolicy::Auto => SlotPolicy::On,
            SlotPolicy::On => SlotPolicy::Off,
            SlotPolicy::Off => SlotPolicy::Auto,
        }
    }
}

/// An error returned when parsing a [`SlotPolicy`] from a string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSlotPolicyError(String);

impl std::fmt::Display for ParseSlotPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown slot policy: `{}`; expected one of auto, on, off",
            self.0
        )
    }
}

impl std::error::Error for ParseSlotPolicyError {}

impl std::str::FromStr for SlotPolicy {
    type Err = ParseSlotPolicyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(SlotPolicy::Auto),
            "on" => Ok(SlotPolicy::On),
            "off" => Ok(SlotPolicy::Off),
            _ => Err(ParseSlotPolicyError(s.to_string())),
        }
    }
}

/// Machine-readable refusal code for agent operation rejections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    /// Slot is currently active in the live mix under Auto policy.
    SlotInMix,
    /// Slot policy is set to Off (locked against MCP modifications).
    SlotPolicyOff,
    /// Slot is unallocated or does not exist.
    SlotUnallocated,
    /// Bay is closed to MCP operations.
    BayClosed,
}

impl RefusalCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            RefusalCode::SlotInMix => "SLOT_IN_MIX",
            RefusalCode::SlotPolicyOff => "SLOT_POLICY_OFF",
            RefusalCode::SlotUnallocated => "SLOT_UNALLOCATED",
            RefusalCode::BayClosed => "BAY_CLOSED",
        }
    }
}

impl std::fmt::Display for RefusalCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured refusal details describing why an agent operation was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalDetail {
    pub code: RefusalCode,
    pub message: String,
    pub slot: Option<usize>,
    pub policy: Option<SlotPolicy>,
    pub in_mix: Option<bool>,
}

impl std::fmt::Display for RefusalDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for RefusalDetail {}

/// Slot-level MCP access status and write-ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SlotAccess {
    pub policy: SlotPolicy,
    pub in_mix: bool,
}

impl SlotAccess {
    pub fn new(policy: SlotPolicy, in_mix: bool) -> Self {
        Self { policy, in_mix }
    }

    pub fn is_writable(&self) -> bool {
        match self.policy {
            SlotPolicy::Auto => !self.in_mix,
            SlotPolicy::On => true,
            SlotPolicy::Off => false,
        }
    }

    pub fn refusal_detail(&self, slot: usize) -> Option<RefusalDetail> {
        match self.policy {
            SlotPolicy::Auto if self.in_mix => Some(RefusalDetail {
                code: RefusalCode::SlotInMix,
                message: format!(
                    "slot {slot} is currently active in the mix and protected under `auto` policy"
                ),
                slot: Some(slot),
                policy: Some(self.policy),
                in_mix: Some(self.in_mix),
            }),
            SlotPolicy::Off => Some(RefusalDetail {
                code: RefusalCode::SlotPolicyOff,
                message: format!(
                    "slot {slot} is locked against MCP modifications under `off` policy"
                ),
                slot: Some(slot),
                policy: Some(self.policy),
                in_mix: Some(self.in_mix),
            }),
            _ => None,
        }
    }

    pub fn refusal_reason(&self, slot: usize) -> Option<String> {
        self.refusal_detail(slot).map(|d| d.message)
    }
}

/// Which way [`Operation::ScaleGrid`] moves the grid. Two values and not an
/// `f32`: the manual's row is *"Halve or double the grid"*, and a factor of 1.3
/// is not an operation anything in this instrument has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridScale {
    Halve,
    Double,
}

/// What the beat is taken from.
///
/// Two arms because the manual's row is *"An audio input to track, or an
/// external process to follow"* — one operation with two forms of source, not
/// two operations. They are not exclusive at runtime (`--tempo-source` leaves
/// the tracker measuring), so attaching both is two calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeatSource {
    /// `default`, or any part of a device's name. `--audio-in`.
    AudioInput(String),
    /// A child process reporting a beat number. `--tempo-source`.
    Process(String),
}

/// One of the three things [`Operation::SetTransition`] can set.
///
/// This row is three operations and the manual has it as one. Three keys press
/// it — `z`, `n`, `j` — and the panel's transition row would draw three
/// controls. It is carried as a sum rather than split into three variants
/// because the manual is the specification and the vocabulary must enumerate
/// what the manual enumerates; splitting the row is a change to the page, and
/// the page is not this crate's to change. See the crate's report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionSetting {
    /// The shape the next wipe takes, and which way its front runs, in radians.
    WipeShape { kind: WipeKind, angle: f32 },
    /// The musical grid the next scheduled move starts on, in beats: 4 for the next
    /// bar, 1 for the next beat, 0 for now.
    Quantum { beats: f64 },
    /// How long the next scheduled move lasts, in beats. Zero is a cut.
    Length { beats: f64 },
}

/// One control on a deck's published interface, on
/// `karakuri_engine::set::Published`'s terms.
///
/// The range narrows the declared one and never redefines it. `node` absent is
/// the wildcard, as in [`ParamAt`], which is what the *default* interface is
/// made of.
#[derive(Debug, Clone, PartialEq)]
pub struct Control {
    /// What the console shows.
    pub name: String,
    pub node: Option<NodeAddress>,
    pub key: String,
    pub range: [f32; 2],
}

/// One of the two things [`Operation::SetProperty`] can set — and the row is
/// *"Element capacity, seeds, the camera"*, which was three operations wearing
/// one heading and is two. Carried as a sum for [`TransitionSetting`]'s reason.
///
/// # The camera was the third and is not one of these
///
/// It was carried as `Camera(Undecided)` because *the camera* looked like two
/// things that were not one operation: the built-in orbit, whose numbers came
/// in through a `camera` record and which declared nothing, or an L3 procedure
/// whose params are written like any other node's. The answer is that they were
/// never two. The built-in orbit's `radius`, `speed` and `height` are
/// parameters of the camera node — the engine declares them where an artifact
/// would, because the built-in is a node with no procedure behind it — so
/// [`Operation::WriteParam`] moves them, `--param L3:0:radius` reaches them,
/// and a knob learned against their positions in the published interface
/// reaches them too. The arm is deleted rather than filled in: an arm carrying
/// three numbers would have been a second way to write a parameter, and the two
/// would disagree the first time one of them grew a refusal.
///
/// The heading still names the camera and this crate's title still matches it
/// exactly, which `the_manual_and_the_vocabulary_agree` checks. That is a cost
/// taken on purpose: *the camera* is the word an operator comes to that row
/// looking for, and the row's own tip is where the page says which row took it.
/// Renaming it to *Element capacity and seeds* was the alternative and is
/// recorded as the one that lost, so it stays a one-line change if a reader
/// disagrees.
/// `docs/adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md`.
///
/// # Neither arm names a node, and that is where the mismatch was closed
///
/// Both carried a [`NodeAddress`] until 2026-09-09, and the capacity's was the
/// standing reason the row could not be drawn: *"an aim carries one capacity
/// for the whole slot where `Property::Capacity` addresses a node"*, which is
/// [ADR-0228](../../docs/adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)'s
/// recorded limit read as a blocker. It was closed by narrowing the payload
/// rather than by widening the aim. What a slot's watcher is pointed at carries
/// one capacity and one seed for the whole slot, which is exactly what
/// `--capacity` has always meant — *"`--capacity` overrides every source"* — so
/// the operation names the deck it already names and no node.
///
/// A payload nobody reads is a free variable
/// ([P-0087](../../docs/principles/0087-name-the-property-never-the-shape.md)):
/// no route filled the address, nothing could have honoured it, and the one
/// route that exists now — the Inspector deck head's two chips — is per slot.
/// What would revive it is a pairing Set whose two geometries want two
/// different capacities; the day that is a want, `watch::Aim::capacity` grows
/// an entry per geometry and this arm grows its address back.
/// `docs/adr/0328-the-inspectors-deck-head-steps-a-slots-capacity-and-re-salts-it.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Property {
    /// How many elements each of a deck's geometries runs at, overriding what their
    /// own `capacity` declarations name. `--capacity`.
    ///
    /// A number in a range the material declares, and the refusal is the engine's:
    /// `Set::build` rejects a capacity outside the declared range and names the
    /// range in the sentence, so a surface that offers a value is offering rather
    /// than deciding
    /// ([P-0090](../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    Capacity { elements: u32 },
    /// The salt a deck's hash builtins are seeded from, so re-seeding changes
    /// randomness without touching anything structural. There is no flag for this.
    ///
    /// `u32` and not `u64`, which is the width the engine has always used:
    /// `watch::Aim::seed_salt`, `Set::source_salts` and
    /// `karakuri_engine::set::derived_salt` are all `u32`, and a payload twice as
    /// wide as the field it lands in is a number that can be asked for and cannot
    /// arrive.
    Seed { salt: u32 },
}

/// Sending a Set and taking one in — two operations under one heading, and the
/// two are not each other's inverse: one names something the store already
/// holds, the other hands the store something it has never seen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetTransfer {
    /// Write the Set filed under this id with every source it names inlined.
    /// `--package ID` — or `--package FILE.kset`, which resolves an authoring
    /// file's parts into the store first and packages that.
    Send { id: String },
    /// Read a Set file, store its sources, write its Set file. The id comes from
    /// the file, and one already taken is refused. `--take-in FILE`, which takes a
    /// `.kbset` as it stands and resolves a `.kset` first.
    ///
    /// A path because a file is what the only existing route takes; whether a route
    /// that has no filesystem — a model handing over the text — takes bytes instead
    /// is open, and is a smaller question than the row's own.
    Take { file: PathBuf },
}

/// Starting and stopping a session recording.
///
/// A sum rather than `{ recording: bool, id: Option<String> }`, because that
/// shape has a field that means nothing in one of its two states, and a payload
/// nobody reads is a free variable
/// (`docs/principles/0087-name-the-property-never-the-shape.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recording {
    /// Begin one, under this id or under a stamp.
    ///
    /// A press can start one, and this doc said the opposite until 2026-09-08. It
    /// argued that only the instant a run begins can produce a head, so a recording
    /// begun mid-performance would replay the launch deck against a late
    /// performance's records. The premise was wrong: `Recorder::open` takes a Set
    /// file's lines, and a Set file written from the live deck is what a keep
    /// already produces at any frame. `karakuri-cli` builds its head from the
    /// launch arguments because that is all it holds at that instant, not because a
    /// later head cannot be made.
    ///
    /// What survives is narrower, and it is about the replay rather than a refusal.
    /// A Set file says what is playing and at what values and holds no *running*
    /// state, so material that accumulates begins again from the top: a replay from
    /// a mid-performance head is a true session of the material as it stood, and
    /// not the picture that was on screen. `karakuri_store::project`'s `key_for`
    /// still holds — the projection that would fold a session down to the deck
    /// state it ends at does not exist — and it is why a head is gathered from the
    /// deck rather than from the stream.
    ///
    /// Each start is a fresh id, which is what keeps
    /// [P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)
    /// met: `Store::append_session` appends and `session::split` sets `started` at
    /// the first tick and never clears it, so a second head under one id would be
    /// read back as edits.
    Start { id: Option<String> },
    /// End the one running. Nothing refuses it, and `crates/karakuri`'s `rec` pill
    /// is the other end of the same press that starts one (ADR-0289).
    ///
    /// Neither end happens on the frame. `Recorder::finish` blocks on its writer,
    /// and so does dropping one, so a stop hands the recorder to a thread and the
    /// outcome is said when it lands —
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).
    Stop,
}
