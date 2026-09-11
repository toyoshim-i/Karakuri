//! **One vocabulary, four ways in.** Every operation Karakuri can perform,
//! named once, so that a panel control, a key, a mapped MIDI message and an
//! MCP tool are four routes into the same name rather than four lists that
//! have to agree.
//!
//! This is the manual's first rule written as a type. [`Operation`] is the
//! enumeration; `docs/manual/operations.html` is its specification, one `<h3>`
//! per variant; and `tests/the_manual_and_the_vocabulary_agree.rs` is what
//! makes that a fact rather than an intention — it reads the page and fails in
//! both directions.
//!
//! # Why it is a crate of its own, with no dependencies
//!
//! Every surface must be able to depend on it, and the surfaces have nothing
//! in common: `karakuri-console` brings `egui` and `karakuri-layout`,
//! `karakuri-midi` brings `midir` and nothing else, `karakuri-cli` brings the
//! engine and a GPU, and the MCP server is inside the CLI. A vocabulary that
//! lived in any one of them would make every other surface depend on that
//! one's world — putting it in `karakuri-console` would mean a MIDI map could
//! not be parsed without a toolkit — and a vocabulary that lived in
//! `karakuri-engine` would be the thing it must not be: engine-shaped.
//!
//! So it is a leaf, and it is **pure data**. It performs nothing. It names
//! what was asked for and stops, exactly as `karakuri_midi::map::Action` — the
//! eight gestures this was the first draft of, and which it has now replaced —
//! said of itself:
//!
//! > What the operator asked for, in terms of the deck rather than of the
//! > wire. Engine-neutral on purpose … this crate names the gesture and
//! > `karakuri-cli` decides what it is worth, which is what keeps every
//! > control reachable from a surface reachable from a key by construction
//! > rather than by two lists agreeing.
//!
//! That is this crate's charter, moved out of `karakuri-midi`, which held it
//! only because MIDI needed it first — and which now routes through it rather
//! than through a vocabulary of its own.
//!
//! **Four surfaces name their operations here, and they do not all route the
//! same way.** `karakuri-console`'s mixer faders were the first customer;
//! `karakuri-cli`'s mix controls are the second — gain, opacity, blend,
//! residency, the tone map, the exposure and the scrub each name an
//! operation and hand it to `karakuri-operation-record`; `karakuri-midi`'s map
//! is the third, which took `Action` away with it
//! (`docs/adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md`);
//! and `karakuri-cli`'s MCP server is the fourth, which names its six tools'
//! operations and **performs them itself**
//! (`docs/adr/0199-mcp-names-its-operations-and-performs-them-itself.md`).
//!
//! **What separates them is what `karakuri-operation-record`'s `written`
//! answers.** An operation that writes a record can be handed to a performer;
//! one that is `Silent` has to be performed by whoever holds the state, because
//! a performer built out of records would do nothing with it. That is why
//! twelve of the keyboard's keys keep a path of their own
//! (`docs/adr/0198-…`), why `karakuri_console::panel::Op` keeps its own type
//! (`docs/adr/0197-…`), and why all six MCP tools do their own work. Said here
//! because an invariant that is not yet true says so
//! (`docs/contributing.md` §4): *every surface routes into the named operation*
//! is true of the naming everywhere, and of the performing only where there is
//! a record to perform.
//!
//! # What a variant carries, and what it does not
//!
//! **What the operation acts on, and nothing about how it was reached.** A
//! deck, a node, a parameter, a value. Not a key, not a controller number, not
//! a rectangle — those belong to the translator on each surface. The model
//! this is built to is that **MCP is the only direct route**: a model reads
//! the published information and speaks these terms. Everything else has a
//! translator. A GUI component turns pointer motion into a number and reads
//! the value back to draw it; the MIDI mapper turns a message into one of
//! these; the keyboard has one too. So a control on the panel is **one
//! operation shown N ways**, which is why MIDI learn is started from a
//! control's tooltip — the control is where the other routes are anchored.
//!
//! Three consequences worth stating, because each of them contradicts
//! something that exists today:
//!
//! **A deck is named, never implied.** `karakuri-cli`'s keys act on the
//! focused deck and MIDI names one in the message; the manual's own gap
//! section calls that out — *"The same operation is addressed two different
//! ways on two surfaces."* An operation that meant *the focused one* would put
//! the console's pointer inside the vocabulary, and [`Operation::SelectDeck`]
//! is a row of its own precisely because selection is a surface's state and
//! writes no record. So every variant that acts on a deck carries `deck: u8`,
//! and the keyboard's translator fills it in from the selection.
//!
//! **What *the selection* is has since been named**, and it strengthens this
//! rather than qualifying it: it is the Mixer bay's remembered address, one
//! instance of the pointer `Tab` moves between bays
//! ([ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
//! [ADR-0332](../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)).
//! An operation meaning *the focused one* would put that pointer inside the
//! vocabulary, and there is now a name for exactly which pointer it would be.
//!
//! **There are no toggles and no cycles.** `karakuri_console::panel::Op`
//! argues this at length and the argument is general: *"A toggle is an
//! affordance built **over** two operations by whoever draws it … a MIDI map
//! with a button per direction, an MCP call that says which one it wants, and
//! a keyboard, all have to be able to say* fold this *and mean it."* So
//! [`Operation::SetResidency`] names one of three residencies and
//! [`Operation::SetBlendMode`] takes a mode. `karakuri-midi`'s
//! `Action::ToggleOnAir`, `Action::TogglePriming` and `Action::CycleBlend` were
//! the shape this replaced, and the first two were the sharpest case in the
//! list: two toggles
//! over **three** states, where each one's `false` had no destination the
//! vocabulary could name. One operation naming one of three has no such hole.
//!
//! **A continuous control is set, not nudged.** A control change carries a
//! position and can only set; a rule that says every operation is addressable
//! by a map therefore says every continuous operation takes a value. `[`, `]`
//! and `\` are three translations of one [`Operation::SetGain`], which is what
//! the manual already says of that row — *"A key steps it; a MIDI control sets
//! it outright."* The one exception is [`Operation::ScrubDeck`], and it is an
//! exception for a reason given at the variant.
//!
//! **It is also what decides how many rows a control gets.** The mask is two —
//! [`Operation::SetMaskShape`] and [`Operation::SetMaskPosition`] — because a
//! single row carrying the shape and the position together would be a press,
//! and `karakuri-midi`'s grammar refuses a control change on a press and a
//! note on a position alike, so no control change could ever reach a mask
//! position. A row that is a sum can only be reached the way its coarsest arm
//! can be
//! (`docs/adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md`).
//!
//! # The cost this crate pays, stated plainly
//!
//! Naming a value means owning the list of values. [`BlendMode`], [`Sync`],
//! [`Residency`], [`Tonemap`], [`Curve`], [`Layer`], [`WipeKind`] and
//! [`Authority`] are this crate's copies of lists `karakuri-engine` and
//! `karakuri-store` already hold. That is duplication and it is deliberate: the alternative is
//! `karakuri_store`'s,
//! which carries these as `String` because *"what a name is allowed to be is
//! the engine's to say"* — and a map file whose typo is refused on the render
//! thread instead of at parse time is a surface that fails where nobody is
//! looking. `karakuri_midi::map::Map::parse` already refuses an unknown target
//! at parse time; this is what lets it go on doing that for a value.
//!
//! The conversion is one function per list in `karakuri-cli` — `mix::blend_mode`
//! and its three neighbours — in the one place every control already ends
//! (`docs/principles/0090-a-surface-offers-it-never-decides.md`), and that
//! package's `mix.rs` is where the two copies of each list are checked against
//! each other, because it is the only crate in the workspace that depends on
//! the engine and on this one at once. **Functions rather than the `From` impls
//! ADR-0180 named**, and it is the orphan rule rather than a preference: both
//! types are foreign to `karakuri-cli`, so the impl is not allowed there at all
//! (`docs/adr/0194-…`).
//!
//! # Where a payload is not decided
//!
//! Three rows name an operation whose payload cannot be written down without
//! a decision nobody has made, and one third of a fourth, the camera arm
//! of [`Property`]. They are [`Operation::MoveBoundary`],
//! [`Operation::WatchFiles`] and [`Operation::SelectScope`], and the first two
//! are the oldest members this crate has.
//!
//! **Two left the group on their own days.** [`Operation::RouteFrame`] carries
//! an [`Output`] and a `bool` since 2026-09-09, and
//! [`Operation::WalkHistory`] carries the Set it is a walk of since
//! 2026-09-10 — not because a decision was taken about a step, but because the
//! thing the payload was waiting for turned out to be sayable: a model spells
//! a Set id in `read_set`, so *which history* stopped being the one thing no
//! surface spells
//! (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
//!
//! **The sequencer's five left on 2026-09-09** ([`Operation::SetStep`],
//! [`Operation::SetLaneMute`], [`Operation::PointLane`],
//! [`Operation::SetPatternGrid`] and [`Operation::SelectPattern`]). They waited
//! on a pattern — authored state nothing in this program held — and a pattern
//! is one bar, a [`StepMode`] and a list of lanes now
//! (`docs/adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md`),
//! with a lane's target an operation of this vocabulary with its value elided
//! ([`LaneTarget`],
//! `docs/adr/0321-a-lanes-target-is-an-operation-with-its-value-elided.md`).
//! Each of the five names the bank it acts on rather than implying the armed
//! one.
//!
//! **Three left on 2026-09-09.** The master chain's effects
//! ([`Operation::SetFeedback`], [`Operation::SetBloom`],
//! [`Operation::SetRgbShift`]) carried [`Undecided`] because their parameters
//! could not be named while the chain did not exist; the chain is three fixed
//! passes now and they carry [`Feedback`], [`Bloom`] and [`RgbShift`] — which
//! is what this marker is for, since replacing it was a compile error at every
//! construction site rather than a search.
//!
//! The three carry [`Undecided`], which is a marker and not a
//! placeholder: it says *this operation exists and what it acts on is an open
//! question*, and it is greppable. Nothing here guesses, because nothing in
//! this repository draws or declares something that claims an answer exists
//! when it does not. Each says at its own definition what the decision is.

use std::path::PathBuf;

// **What may be asked, and by what** — the audit ADR-0235 decided, over the
// vocabulary this crate names. Not an `Operation` and not part of the
// vocabulary; the module's own documentation is where that is argued, and it
// carries the `//!` rather than this line carrying a `///`, so that the links
// in it resolve inside the module they name.
pub mod gate;

/// **A payload that has not been decided, on an operation that has.**
///
/// The row is real — it is in `docs/manual/operations.html`, so it is part of
/// the vocabulary and every surface owes it a route — but what it acts on is a
/// question with more than one live answer, and picking one here would be this
/// type asserting a decision nobody made. So the name lands and the payload
/// says so.
///
/// **Not a `TODO` and not an empty payload.** An empty payload reads as *this
/// operation acts on nothing*, which is a claim, and a wrong one for every
/// one of these. This reads as *what this acts on is open*, which is true, and it
/// is a type — so the day the decision is made, replacing it is a compile
/// error at every construction site rather than a search.
///
/// `docs/contributing.md` §4, applied
/// to a payload rather than to a sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Undecided;

/// **The name a procedure's header gives one declared input** — `far` in
/// `uses far : Geometry` — carried by [`Operation::WireInput`]'s `slot` field.
///
/// **Its own type because `slot` names two unrelated things on this
/// operation's neighbours**: a member of the deck, on every operation that
/// takes one, and this — the name a procedure reads a binding through. See
/// `docs/adr/0344-slot-is-disambiguated-into-three-types-and-adr-0049s-wait-is-over.md`.
///
/// **Mirrors `karakuri_ir::typed::InputPort` and
/// `karakuri_store::record::InputPort` rather than depending on either.**
/// This crate is a leaf by charter — see the crate documentation on why it
/// has no dependencies — so a shared type is not an option here the way it is
/// between `karakuri-engine` and `karakuri-ir`; three definitions of the same
/// concept is the cost, on [`NodeAddress`]'s own precedent below.
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
/// **The deck is not here.** It is a field on the operation, as `slot` is a
/// field on every `karakuri_store::record::Record` that names a node — which
/// keeps one address shape for *within a Set* and one for *which Set*.
///
/// **This is the positional address, and it is not the only one in the
/// workspace.** An edge names its two ends by node *name*, deliberately, and
/// `karakuri_store::record::Record::Edge` gives the reason: *"a position moves
/// when the list is reordered, and reordering silently changing which geometry
/// a morph blends towards is the exact failure this record exists to end."*
/// So [`Operation::WireInput`] takes names and everything else takes this, and
/// the two are not interchangeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeAddress {
    pub layer: Layer,
    /// Which node of that layer, in the order the deck's files were named.
    pub index: u32,
}

/// **Which version [`Operation::RestoreProcedure`] puts back**, and the two
/// arms are the two things a surface can say rather than two features.
///
/// **A surface says the half it holds**
/// ([ADR-0192](../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
/// A staging lane row is a node with one unsettled version on it: it names the
/// node and means *the one this replaced*, and it holds no listing to pick a
/// row out of. A row of the Library bay's `history` scope is a version the
/// operator picked out of a listing, and what that row carries is the name the
/// store filed it under.
///
/// **One enum rather than a `version: Option<String>` beside the `node`.**
/// That shape can say a node and a version at once, and the two can then
/// disagree — a version of `L4:0` addressed to `L2:1` is a payload with two
/// answers to *which node*, and the one that would win is whichever the
/// performer read. Here the address is inside the arm that owns it, and a
/// disagreement cannot be spelled.
///
/// **Neither arm is a path.** A path may sit in a payload only where every
/// route that fills it derives it from something the program itself produced,
/// and no surface here spells one: a walk's row is a *name*, matched back
/// against the listing that produced it, exactly as `SetTransfer::Take`'s file
/// is found again by the word that was pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision {
    /// **The version this node's present source replaced.** One step, and
    /// never a cursor — walking further is [`Operation::WalkHistory`].
    Previous(NodeAddress),
    /// **A version an operator picked out of a walk**, by the name the store
    /// filed it under — `20260908-143052-271_slot0_L4_beat_strokes`.
    ///
    /// **The name is the node's address as well as the moment**, because that
    /// is how a version is filed
    /// (`docs/adr/0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md`),
    /// so this arm needs no [`NodeAddress`] beside it and a second spelling of the
    /// address is not invented here.
    Picked(String),
}

/// Which parameter of a deck's Set.
///
/// **`node` absent is a wildcard, not node 0** — every node of the Set that
/// declares `key`. That is `Record::Param`'s rule and `Published::at`'s rule
/// and `--param exposure=2.0`'s meaning, and it is the useful default: one
/// knob moving every renderer that has an `exposure`.
///
/// **A wildcard is refused where the nodes it lands on are not under one
/// authority**, and the refusal names them. An authority is per node
/// (`docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`)
/// and a bare name reaches every node declaring the key, so one such control
/// spanning a node the operator kept and a node an agent acts on would hand the
/// first over through the second. Landing on the permitted nodes instead was
/// the alternative, and it lost to
/// `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`:
/// see
/// `docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`.
/// **A surface owns none of this**
/// (`docs/principles/0090-a-surface-offers-it-never-decides.md`):
/// the rule lives where the write lands, so every route meets it, and an
/// addressed [`ParamAt`] meets nothing — it says which node it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamAt {
    pub node: Option<NodeAddress>,
    /// The param's own name inside the node.
    pub key: String,
}

/// Which parameter of a deck's Set, as an **attachment** addresses one: a
/// layer, a node of it or all of them, and the key.
///
/// # It is not [`ParamAt`], and the difference is a fact about a binding
///
/// A binding is resolved through the nodes of **one layer** —
/// `karakuri_engine::binding::Binding` carries a required layer and an
/// optional index, and `Set::bind` walks `nodes_of(layer)` — so *every node of
/// every layer that declares this key*, which is exactly what a [`ParamAt`]
/// with no node means, is a set no attachment has ever been able to name. An
/// operation whose address could ask for it would be an operation refused at
/// the far end for a reason the vocabulary already knew, and
/// `karakuri_operation_record::written` would have to invent a layer to write
/// the record with — the placeholder `Record::Param`'s `layer` is and is stuck
/// being (ADR-0280 §1).
///
/// **So the two addresses are two facts and not two spellings.** A value has a
/// wildcard that names no layer, because a value lands wherever the name is
/// declared; an attachment's wildcard is a layer's, because the signal is
/// written into that layer's uniform buffer. `index` absent is *every node of
/// this layer declaring `key`*, which is `Record::Bind`'s rule and `--bind
/// L4:exposure=…`'s meaning.
///
/// **A wildcard here meets no authority refusal**, unlike [`ParamAt`]'s: an
/// authority governs who may **write** a node's params, and attaching a signal
/// is not a write of a value — it says what the value blends towards. What
/// stops one is the take-back, which is a control on the same row
/// (`docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindAt {
    pub layer: Layer,
    /// Which node of that layer, or **every node of it declaring `key`**.
    /// Present or absent as a unit with nothing — the layer is always said.
    pub index: Option<u32>,
    /// The param's own name inside the node, and a **component key** where the
    /// parameter is a vector: `glow.x` and never `glow`, because a binding
    /// resolves to one number and a `vec3` has three places to put it
    /// (ADR-0268).
    pub key: String,
}

/// A parameter's value. The three widths a `.kir` can declare, on
/// `karakuri_store::record::Value`'s terms — a value and never a range, since
/// a range is the procedure's declaration and not an operator's to write.
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
/// `karakuri_store::record::Layer`'s, and this is a third spelling of that
/// list beside `karakuri_ir::ast::Kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    L1,
    L2,
    L3,
    L4,
    Field,
    /// **A `kind L5` procedure** — a frame effect over the picture handed to
    /// it. See
    /// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
    L5,
}

/// **Which kinds of row a library listing shows**: the five procedure kinds and
/// Sets, each on or off, with an **OR** across the ones that are on and
/// **everything** where none is.
///
/// [`Operation::FilterLibrary`]'s payload, and the whole of it. Seven named
/// booleans rather than a list of members, because the list is closed by
/// construction — a procedure declares one of [`Layer`]'s six kinds and the
/// seventh row kind is a Set — and a `Vec` would admit a member said twice,
/// which is a state this control cannot be in.
///
/// **Every state is said at once.** A press names the whole row and never one
/// chip, which is [`Operation::Publish`]'s rule on a different list: *"adding
/// or removing one at a time is a statement about an entry, and an interface
/// that publishes nothing publishes everything is a statement about the list"*
/// — and it is what keeps two hands on one bay from disagreeing about which
/// kinds are showing.
///
/// **The affordance is the surface's.** Nothing here says *toggle*
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
    /// **The seventh**, and it arrived the day `kind L5` did: a frame effect is
    /// a procedure with a `kind` like any other, so it is a row of the Library
    /// and the filter row has a toggle for it. See
    /// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
    pub l5: bool,
    /// Sets, which is the one row kind that is not a procedure's `kind` — a
    /// Set fills several layers and declares none, so *is this its kind* is not
    /// a question it answers.
    pub sets: bool,
}

impl LibraryKinds {
    /// **Nothing narrowed**, which shows everything and is where a run begins.
    /// It is also the state a press can always get back to, which is what
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

    /// Whether any button is on. `false` is [`LibraryKinds::EVERYTHING`], and
    /// the two readings of it — *nothing shows* and *everything shows* — are
    /// settled here rather than at each caller: **everything**, because a
    /// filter row that could hide the whole listing would have a state an
    /// operator cannot see their way out of.
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
    /// **The lower-case word for this mode**, which is the one every surface
    /// spells it with: `Record::Transport`'s wire `sync`, `karakuri-cli`'s
    /// status line, and a map file's value.
    ///
    /// A match rather than a table, exactly as [`BlendMode::name`] is one and
    /// for its reason: a mode added to the enum does not compile until it has
    /// a name. The three words are `karakuri_engine::transport::Sync::name`'s,
    /// because a record carries a name and the engine is what reads it back.
    pub fn name(self) -> &'static str {
        match self {
            Sync::Free => "free",
            Sync::Tempo => "tempo",
            Sync::Beat => "beat",
        }
    }
}

/// **What one step of a sequencer pattern is worth**: a sixteenth or an
/// eighth, and the list is closed.
///
/// The pattern is one bar, fixed, so the count follows the mode rather than
/// being a second thing a hand sets — sixteen cells at a sixteenth and eight
/// at an eighth, the row keeping its width so the cells halve in the finer one
/// (`docs/adr/0306-the-grid-head-is-one-pill-because-the-bar-is-one-bar-and-the-count-follows-the-mode.md`).
///
/// **It is the pattern's and not the session's or a lane's**, which is the
/// console's own sentence about the pill that draws it: *"It is armed because
/// it is what the pattern is rather than a preference the head is holding."*
/// So [`Operation::SetPatternGrid`] names the bank it is the mode of.
///
/// **A pattern stores sixteen slots in both modes and an eighth reads slot
/// `2k`**, so this is a change of *reading* and never of the pattern — which
/// is [`StepMode::slot_of`] and
/// `docs/adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md`.
/// The stored width is `karakuri_pattern`'s, because it belongs to the thing
/// that holds the steps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StepMode {
    /// Sixteen steps to the bar, four to the beat. What the console's ruler
    /// and cells have been drawn in since the mock's first commit, and what a
    /// pattern nobody has pressed the pill on is in.
    #[default]
    Sixteenth,
    /// Eight steps to the bar, two to the beat.
    Eighth,
}

impl StepMode {
    /// Both values, in the order the pill names them.
    pub const ALL: [StepMode; 2] = [StepMode::Sixteenth, StepMode::Eighth];

    /// **The word the head's pill reads**, which is the mock's own spelling.
    ///
    /// A match rather than a table, exactly as [`Sync::name`] and
    /// [`BlendMode::name`] are and for their reason: a mode added to this enum
    /// does not compile until it has a name.
    pub fn name(self) -> &'static str {
        match self {
            StepMode::Sixteenth => "1/16",
            StepMode::Eighth => "1/8",
        }
    }

    /// **How many steps there are in the bar at this mode** — sixteen and
    /// eight. The bar is fixed, so this follows the mode and is not a second
    /// choice beside it (ADR-0306).
    pub fn count(self) -> usize {
        match self {
            StepMode::Sixteenth => 16,
            StepMode::Eighth => 8,
        }
    }

    /// **The multiplier in the step index**, which is
    /// `floor(beats × steps_per_beat) mod count` — ADR-0222's own formula, a
    /// pure function of `Oscillator::beats`.
    pub fn steps_per_beat(self) -> f64 {
        match self {
            StepMode::Sixteenth => 4.0,
            StepMode::Eighth => 2.0,
        }
    }

    /// **Which of the sixteen stored slots step `step` reads.**
    ///
    /// The identity at a sixteenth and `2k` at an eighth, which is what makes
    /// a mode press a change of reading: the finer mode and back returns
    /// exactly what was there, and an eighth-mode step sits at the same
    /// musical instant as the sixteenth it is drawn over. Sizing the store to
    /// the count instead would throw half a bar away on one press with nothing
    /// to confirm against, which is the alternative ADR-0320 refuses.
    pub fn slot_of(self, step: usize) -> usize {
        match self {
            StepMode::Sixteenth => step,
            StepMode::Eighth => step * 2,
        }
    }
}

/// **What a sequencer lane drives**: an operation of this vocabulary with its
/// value left out.
///
/// A lane is a fifth route into this vocabulary rather than a binding
/// (`docs/adr/0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md`), so
/// what it needs is not a new operation but an *address* — and the address a
/// lane wants is one of these arms plus the level the step is worth, which is
/// [`LaneTarget::operation`].
///
/// **This is the bay's sharpest question and this is the answer**
/// (`docs/adr/0321-a-lanes-target-is-an-operation-with-its-value-elided.md`).
/// The console draws four lanes — three deck faders and a Set parameter — and
/// the two obvious spellings each reach one kind and not the other: a
/// published-interface position is a control a *Set* declares, and
/// `Record::Opacity` is no Set's; a slot number cannot say which parameter.
/// **An operation minus its value reaches all four**, because the vocabulary
/// already addresses both.
///
/// **Two of this vocabulary's rows are not lane targets**, and it is one
/// reason: [`Operation::SetResidency`] and [`Operation::SetBlend`] take a word
/// from a closed list rather than a level, and a step is a level — so a lane
/// pointed at one would have to invent the word an on-step means. It is
/// written here rather than left to be noticed from this enum's silence.
///
/// **[`Operation::SetGain`], [`Operation::SetMaskPosition`],
/// [`Operation::SetMasterOut`] and [`Operation::SetExposure`] are each one arm
/// and one line of [`LaneTarget::operation`]** — additions to a closed list, so
/// none of them is a decision and their absence is scope rather than a gap.
#[derive(Debug, Clone, PartialEq)]
pub enum LaneTarget {
    /// **A deck's channel fader** — three of the four lanes the console draws.
    Fader { deck: u8 },
    /// **A parameter inside the Set on a deck** — the fourth.
    ///
    /// A vector parameter costs nothing extra:
    /// `docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md`
    /// makes [`ParamAt::key`] `glow.x` and never `glow`, so a lane reaches a
    /// component by the road `--param` reaches it by and needs no field of its
    /// own.
    Param { deck: u8, param: ParamAt },
}

impl LaneTarget {
    /// **The operation this lane emits at a step worth `value`.**
    ///
    /// [`crate::gate`]'s discipline applied to an address: an exhaustive
    /// `match`, so a target added does not compile until it says what it
    /// emits. It is `karakuri_console::panel::Knob::operation` and
    /// `karakuri_midi::map::Target::operation` a third time — *an address plus
    /// a value becomes an operation* — which is why this is not a new
    /// mechanism.
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

    /// **Which deck this lane writes into**, which is what a caller asking
    /// *does a lane hold this control* starts from
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
/// **This is the request, and the engine keeps two.** The operator writes one
/// through `Deck::set_residency`; the governor derives the other from it and
/// the budget, holding a slot below what was asked for and never above. Only
/// the request is an operation, so this enum names three destinations and says
/// nothing about which of them the deck arrived at — a surface reads that back
/// rather than assuming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Residency {
    /// Stepped and composited. Honoured: nothing demotes a live slot.
    Live,
    /// Stepped out of sight, warming its buffers, contributing nothing to the
    /// mix. A request the governor reconsiders every pass, and parks rather
    /// than refuses when there is no room.
    Priming,
    /// Compiled, buffers held, not stepping. Keeps its `t`, so a slot taken
    /// here and brought back resumes where it stopped.
    Allocated,
}

impl Residency {
    /// Every residency there is, **in the order they cost** — the order this
    /// enum declares them and the order
    /// `karakuri_engine::deck::Residency` does.
    ///
    /// **A list is not a cycle**, exactly as [`BlendMode::ALL`] is not: what a
    /// surface needs from the vocabulary is *which values exist*, and a control
    /// that steps through them is an affordance built over the three operations
    /// they name
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    /// `karakuri-midi`'s map reads it to decide what a `residency N <word>` line
    /// may end in, so a value added here is offered to a map file rather than
    /// waiting for a parser's second list to catch up.
    pub const ALL: [Residency; 3] = [Residency::Live, Residency::Priming, Residency::Allocated];

    /// **The lower-case word for this level**, which is what
    /// `Record::Residency` carries and what `karakuri-cli`'s
    /// `mix::residency_wire_name` writes. The status line's `LIVE`/`prim`/
    /// `park` is a different vocabulary for a different reader and is
    /// deliberately not this one.
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

/// **Where a composited frame goes, by name.**
///
/// An output is a destination, a size and an on/off
/// ([ADR-0324](../../../docs/adr/0324-an-output-is-a-named-destination-with-a-size-and-an-on-off.md)).
/// This is the destination half — the only half a *vocabulary* can carry,
/// because the other two are answers rather than names: the size is the
/// window manager's or the Program bay's, and whether it is on is read where
/// it is kept.
///
/// # It is a closed list and not a string
///
/// **A label moves and an identity may not.** The console's projector chip
/// carries the display it is on — `projector · DELL U2720Q` in the mock — and
/// that string changes when the cable does, when the display is renamed, and
/// when the same window is dragged to the other screen. A name a map line or
/// an MCP call held would then name nothing, and there would be nothing to
/// refuse it with: a string parses whatever it is given, so *there is no such
/// output* would be a sentence somebody had to remember to write at every
/// route in, where a variant is a compile error at the one that mistyped it.
///
/// **This crate owns the enumerations a destination is drawn from rather than
/// passing strings**, which is
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// and the reason [`Residency`], [`BlendMode`], [`WipeKind`] and the rest are
/// here at all. An output is the same kind of thing they are: a word from a
/// closed list, which is exactly what `karakuri-midi`'s map file can name and
/// what an MCP argument can be validated against.
///
/// **And a string would allocate.** This crate has no `[dependencies]` and
/// every payload in it is a number or a word; a `String` on the one operation
/// a sink press emits would put an allocation on the path a frame's
/// publishing is switched from.
///
/// # Why the picture is a variant and a preview cell is not
///
/// [ADR-0243](../../../docs/adr/0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md)
/// settled it: the picture in the Program bay is in the set that needs naming
/// and the four cells are not. A cell is welded to the deck letter under it,
/// nothing routes one, and a control that names an output can never name one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// **The picture in the Program bay**, and the first row of the list.
    ///
    /// Its size is the rectangle the bay gives it and its on/off is the
    /// picture's own fold — read where the arrangement keeps it rather than
    /// stored a second time here, which is
    /// [ADR-0161](../../../docs/adr/0161-solo-remembers-which-region-because-it-cannot-be-derived.md)'s
    /// rule read on a sink: a second copy is a copy that drifts.
    Program,
    /// **A window this program opens**, numbered from zero in the order the
    /// list draws them.
    ///
    /// One is built. The number is here rather than deferred because a second
    /// projector is a list entry and not a new kind of thing, and a variant
    /// that has to be widened later is a compile error at every construction
    /// site — which is the cost this payload was `Undecided` to avoid paying
    /// twice.
    Projector(u8),
    /// **A sink a plugin brings**, by its place in the manifest that loaded
    /// it.
    ///
    /// **Nothing constructs one**, and that is a statement about the manifest
    /// rather than about this variant: `docs/plugins.md` specifies the process
    /// and the handshake, nothing implements them, and the console draws
    /// Syphon and NDI as `no plugin`. The index is the manifest's own order
    /// for the same reason [`Output::Projector`] carries one — a plugin's
    /// *name* is a label it chose, and a label is not an identity.
    Plugin(u8),
}

impl Output {
    /// **Every output this program can name today**, which is the program
    /// view and one projector window.
    ///
    /// **It is not every variant**, and the difference is the point: a
    /// [`Output::Plugin`] exists in the type so that the day a manifest is
    /// read the list grows rather than the vocabulary changing, and a surface
    /// drawing this list would draw a chip for a plugin that is not there.
    /// A surface that wants the absent ones draws them from what it knows is
    /// missing, which is what `karakuri-console` does.
    pub const ALL: [Output; 2] = [Output::Program, Output::Projector(0)];

    /// **The lower-case word for this destination**, for a caller that prints
    /// one. A projector and a plugin carry their index, because two of either
    /// are two outputs and a reader has to be able to tell them apart.
    ///
    /// A `String` rather than a `&'static str` for exactly that reason, and it
    /// is the one thing in this crate that allocates — off the frame path, in
    /// a line somebody reads.
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
/// **Named `BlendMode` rather than `Blend` on purpose.** `Blend` already means
/// two different things here — `karakuri_engine::deck::Blend` is how a deck
/// meets the mix, `karakuri_ir::ast::Blend` is how an L4's fragments meet each
/// other — and a name means one thing across the system
/// (`docs/contributing.md` §4). A third `Blend` would make it three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Add,
    Over,
    Max,
}

impl BlendMode {
    /// Every mode there is, in the engine's own order —
    /// `karakuri_engine::deck::Blend::ALL`, which states it as *"in cycle
    /// order. `Add` first, because it is the default and a cycle should start
    /// where a slot starts."*
    ///
    /// **A list is not a cycle, and this crate does not own the cycle.** A
    /// control that steps through these is an affordance built over the three
    /// operations they name, and it belongs to whoever draws the control
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). What this is for is the two things a
    /// surface genuinely needs from the vocabulary: *which values exist*, and
    /// *in what order they are conventionally shown*. The mixer strip's blend
    /// chip does its own arithmetic over this
    /// ([ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)),
    /// and a MIDI map is offered the three values rather than a step.
    pub const ALL: [BlendMode; 3] = [BlendMode::Add, BlendMode::Over, BlendMode::Max];

    /// **The lower-case word for this mode**, which is the one every surface
    /// spells it with: `Record::Blend`'s wire `mode`, `karakuri-cli`'s status
    /// line, a map file's value, and the word a mixer strip's chip draws.
    ///
    /// A match rather than a table, exactly as `karakuri_engine::deck::Blend::name`
    /// is one, and for its reason: **a mode added to the enum does not compile
    /// until it has a name.** A table indexed by discriminant would take a new
    /// variant silently and hand out the wrong word or panic.
    ///
    /// The three words are the engine's, because a record carries a name and
    /// the engine is what reads it back. This is a copy of that list, which is
    /// the cost this crate pays on purpose — see the module documentation.
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
    /// **The lower-case word for this operator**, which is what
    /// `Record::Look`'s `op` carries and what `--tonemap` takes.
    ///
    /// The wire spelling and never the reader's: `karakuri-cli`'s `spellings`
    /// keeps two per operator — `("aces", "ACES")` — because one of them is
    /// parsed and the other is only read. This is the parsed one, for
    /// [`BlendMode::name`]'s reason: an operator added to the enum does not
    /// compile until it has a name.
    pub fn name(self) -> &'static str {
        match self {
            Tonemap::Clamp => "clamp",
            Tonemap::Reinhard => "reinhard",
            Tonemap::Aces => "aces",
            Tonemap::AgX => "agx",
        }
    }
}

/// **Which frame the master chain's feedback pass reads back.**
/// `karakuri_engine::master::Cut`'s two, mirrored here rather than imported
/// for this crate's own reason — see the module documentation.
///
/// The maintainer's decision on 2026-09-09 was *both, selectable*: the two are
/// different pictures and a design that picked one would be taking a decision
/// away from a hand. See
/// `docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cut {
    /// The frame as the mix wrote it, before this chain touched it. One echo
    /// of the previous frame and not a trail: nothing read back has been fed
    /// back.
    #[default]
    Mix,
    /// The chain's own exit, after rgb shift and before the tone map. A trail,
    /// because what is read back already contains it.
    Exit,
}

impl Cut {
    /// Both, in the order a surface shows them. **A list is not a cycle** —
    /// [`BlendMode::ALL`]'s rule, and the console's own chip does the
    /// arithmetic over these two.
    pub const ALL: [Cut; 2] = [Cut::Mix, Cut::Exit];

    /// **The lower-case word for this cut**, which is what
    /// `karakuri_store::record::Record::MasterChain` carries and what a map
    /// file spells. [`BlendMode::name`]'s rule: a cut added to the enum does
    /// not compile until it has a name.
    pub fn name(self) -> &'static str {
        match self {
            Cut::Mix => "mix",
            Cut::Exit => "exit",
        }
    }
}

/// **The feedback pass's parameters**: how much of the retained frame comes
/// back, and which frame that is.
///
/// **Two fields and not two operations.** The amount without the cut is not a
/// picture anybody can reconstruct — the same 0.5 is a one-frame echo under
/// [`Cut::Mix`] and a compounding trail under [`Cut::Exit`] — so a surface
/// that could move one without saying the other would be asking for a look it
/// had not named. That is `Record::Look`'s argument at the size of one pass.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Feedback {
    /// `[0, 0.95]`, and the ceiling is the engine's: at 1.0 the exit cut is an
    /// accumulator with no decay in it. Clamped where the record is applied
    /// and not here — `karakuri_engine::master::Chain::clamped`.
    pub amount: f32,
    /// Which frame the amount is of.
    pub cut: Cut,
}

impl Feedback {
    /// **The most a surface may ask for**, and the reason it is short of 1.0
    /// is the engine's: under [`Cut::Exit`] the pass is an accumulator with no
    /// decay in it, so 1.0 runs still material away to infinity. At 0.95 the
    /// ceiling is twenty times the frame.
    ///
    /// **This is the reach a control draws, and it is not the wall.** The wall
    /// is `karakuri_engine::master::Chain::clamped`, where the record is
    /// applied, so a MIDI map and a model meet it too
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). The
    /// number is here as well so a fader can be laid out without reading the
    /// engine, and the two being one number is asserted where both are
    /// visible — `crates/karakuri`, which depends on this crate and on the
    /// engine. That is [`Tonemap`]'s arrangement for a range instead of a
    /// list.
    pub const MAX: f32 = 0.95;
}

/// **The bloom pass's parameters**: how much of the blurred bright part is
/// added back.
///
/// **One field, and the two numbers that are not here are stated rather than
/// forgotten.** The *knee* — what counts as bright — is 1.0 and fixed,
/// because in a linear HDR pipeline 1.0 is the top of the range the sRGB
/// encode is honest about rather than an arbitrary level, and what decides how
/// much of a frame is above it is [`Operation::SetMasterOut`] one pass
/// upstream. The *radius* is fixed because the tap count is what a radius
/// costs and a cost is known before it is paid
/// (`docs/principles/0091-cost-is-known-before-it-is-paid.md`). Both are named
/// in `karakuri_engine::master`, with the numbers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bloom {
    /// `[0, 1]`.
    pub amount: f32,
}

/// **The rgb shift pass's parameters**: how far the three channels are pulled
/// apart.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RgbShift {
    /// `[0, 1]`, of the engine's `Chain::SHIFT_MAX` — 2% of the frame's
    /// height. In fractions of the frame and never in texels, so the same
    /// session shifts the same distance on a second display
    /// (ADR-0247).
    pub amount: f32,
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
    /// **Every shape there is, in the order the specification documents
    /// them** — the identity, the peak-weighted, its floor-weighted
    /// complement, and the one eased at both ends.
    ///
    /// **A list is not a cycle**, on [`Authority::ALL`]'s and
    /// `karakuri_engine::deck::Blend::ALL`'s terms: the sensitivity row's
    /// curve chip is an affordance built over these four, the cycle belongs to
    /// whoever draws it, and what crosses this seam is
    /// [`Operation::AttachSignal`] naming a destination
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    pub const ALL: [Curve; 4] = [Curve::Lin, Curve::Pow2, Curve::Sqrt, Curve::Smooth];

    /// **The lower-case word a record spells**, which is what
    /// `karakuri_store::record::Record::Transition`'s `curve` carries and what
    /// a `bind` record has always carried.
    ///
    /// It arrived later than [`BlendMode::name`] and its neighbours because
    /// nothing needed it: the one operation carrying a curve —
    /// [`Operation::AttachSignal`] — wrote no session record, so this list had
    /// no wire to reach. A scheduled move does: the shape a fade takes is part
    /// of what a replay reconstructs it from, and `karakuri-operation-record`
    /// is where a curve becomes a name. **`AttachSignal` writes one now too**,
    /// and it is `Record::Source`'s `curve` — the same spelling, so a fade's
    /// shape and an attachment's cannot drift apart on the wire.
    ///
    /// A match rather than a table, for [`BlendMode::name`]'s reason: a curve
    /// added to the enum does not compile until somebody has spelled it.
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
/// **A shape is this and an angle**, which is why it is not the six-item list
/// the `z` key cycles: `karakuri-cli`'s `MASK_SHAPES` is a curated six chosen
/// out of an unbounded `(kind, angle)` pair — *"an arbitrary angle is a dial,
/// and a dial with nowhere to show its value is a control an operator cannot
/// read"* — and the curation is a keyboard's compromise, not the operation.
/// A panel dial and an MCP call can both say an angle.
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
    /// **The lower-case word for this shape**, which is what `Record::Mask`'s
    /// `kind` carries and what the engine reads back.
    ///
    /// A match rather than a table, for [`BlendMode::name`]'s reason: a shape
    /// added to the enum does not compile until it has a name. The three words
    /// are `karakuri_engine::deck::MaskKind::name`'s, because a record carries
    /// a name and the engine is what reads it back.
    ///
    /// **There is no `ALL` beside it**, where [`BlendMode`] and [`Residency`]
    /// have one. That constant exists so a map file can be offered the values
    /// a target may end in, and no map target names a shape — see the row at
    /// [`Operation::SetMaskShape`]. It arrives with the first reader.
    pub fn name(self) -> &'static str {
        match self {
            WipeKind::None => "none",
            WipeKind::Linear => "linear",
            WipeKind::Radial => "radial",
        }
    }
}

/// **Who may move one node of a Set.** The manual's sixth rule, as a list of
/// three: *"Each node of a Set is manual, suggesting, or automatic, and you set
/// that node by node."*
///
/// **A permission granted forward, and not a record of who moved something
/// last.** The second is rule 02 — *"a parameter driven by something else shows
/// its source instead of a number"* — and it is read off the binding that is
/// driving the param. This is the other question, asked before anything moves:
/// what an agent is *allowed* to do to this node.
///
/// **Three destinations and no toggle**, which is [`Residency`]'s shape and
/// [`BlendMode`]'s: an operation names one of them outright, and a control that
/// steps through them is an affordance built over the three
/// (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
/// The console draws that affordance as `man / sug / auto` on a node head, and
/// those three words are a surface's abbreviations rather than this list —
/// exactly as the status line's `LIVE`/`prim`/`park` is not [`Residency::name`].
///
/// **This is a copy, and the engine holds the list it is a copy of.** It was
/// not one when it landed — `karakuri-engine` held no authority at all, which
/// is what
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
    /// **The lower-case word for this level**, which is what
    /// `karakuri_store::record::Record::Authority` carries.
    ///
    /// A match rather than a table, for [`BlendMode::name`]'s reason: a level
    /// added to the enum does not compile until it has a name. The three words
    /// are rule 06's own — *manual*, *suggesting*, *automatic* — and not the
    /// console's `man / sug / auto`, which is a node head's abbreviation for a
    /// reader rather than a name a record is read back with.
    ///
    /// **There is no `ALL` beside it**, on [`WipeKind`]'s terms exactly: that
    /// constant exists so a map file can be offered the values a target may end
    /// in, and no map target names an authority — a map line cannot say a node
    /// address at all, which is what the manual's gap section says of MIDI. It
    /// arrives with the first reader.
    pub fn name(self) -> &'static str {
        match self {
            Authority::Manual => "manual",
            Authority::Suggesting => "suggesting",
            Authority::Automatic => "automatic",
        }
    }
}

/// Which way [`Operation::ScaleGrid`] moves the grid. Two values and not an
/// `f32`: the manual's row is *"Halve or double the grid"*, and a factor of
/// 1.3 is not an operation anything in this instrument has.
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
/// **This row is three operations and the manual has it as one.** Three keys
/// press it — `z`, `n`, `j` — and the panel's transition row would draw three
/// controls. It is carried as a sum rather than split into three variants
/// because the manual is the specification and the vocabulary must enumerate
/// what the manual enumerates; splitting the row is a change to the page, and
/// the page is not this crate's to change. See the crate's report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionSetting {
    /// The shape the next wipe takes, and which way its front runs, in radians.
    WipeShape { kind: WipeKind, angle: f32 },
    /// The musical grid the next scheduled move starts on, in beats: 4 for the
    /// next bar, 1 for the next beat, 0 for now.
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
/// whose params are written like any other node's. **The answer is that they
/// were never two.** The built-in orbit's `radius`, `speed` and `height` are
/// parameters of the camera node — the engine declares them where an artifact
/// would, because the built-in is a node with no procedure behind it — so
/// [`Operation::WriteParam`] moves them, `--param L3:0:radius` reaches them,
/// and a knob learned against their positions in the published interface
/// reaches them too. **The arm is deleted rather than filled in**: an arm
/// carrying three numbers would have been a second way to write a parameter,
/// and the two would disagree the first time one of them grew a refusal.
///
/// **The heading still names the camera** and this crate's title still matches
/// it exactly, which `the_manual_and_the_vocabulary_agree` checks. That is a
/// cost taken on purpose: *the camera* is the word an operator comes to that
/// row looking for, and the row's own tip is where the page says which row took
/// it. Renaming it to *Element capacity and seeds* was the alternative and is
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
/// **one** capacity and **one** seed for the whole slot, which is exactly what
/// `--capacity` has always meant — *"`--capacity` overrides every source"* —
/// so the operation names the deck it already names and no node.
///
/// **A payload nobody reads is a free variable**
/// ([P-0087](../../docs/principles/0087-name-the-property-never-the-shape.md)):
/// no route filled the address, nothing could have honoured it, and the one
/// route that exists now — the Inspector deck head's two chips — is per slot.
/// **What would revive it** is a pairing Set whose two geometries want two
/// different capacities; the day that is a want, `watch::Aim::capacity` grows
/// an entry per geometry and this arm grows its address back.
/// `docs/adr/0328-the-inspectors-deck-head-steps-a-slots-capacity-and-re-salts-it.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Property {
    /// How many elements each of a deck's geometries runs at, overriding what
    /// their own `capacity` declarations name. `--capacity`.
    ///
    /// **A number in a range the material declares, and the refusal is the
    /// engine's**: `Set::build` rejects a capacity outside the declared range
    /// and names the range in the sentence, so a surface that offers a value
    /// is offering rather than deciding
    /// ([P-0090](../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    Capacity { elements: u32 },
    /// The salt a deck's hash builtins are seeded from, so re-seeding changes
    /// randomness without touching anything structural. There is no flag for
    /// this.
    ///
    /// **`u32` and not `u64`**, which is the width the engine has always used:
    /// `watch::Aim::seed_salt`, `Set::source_salts` and
    /// `karakuri_engine::set::derived_salt` are all `u32`, and a payload twice
    /// as wide as the field it lands in is a number that can be asked for and
    /// cannot arrive.
    Seed { salt: u32 },
}

/// Sending a Set and taking one in — **two operations under one heading**, and
/// the two are not each other's inverse: one names something the store already
/// holds, the other hands the store something it has never seen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetTransfer {
    /// Write the Set filed under this id with every source it names inlined.
    /// `--package ID` — or `--package FILE.kset`, which resolves an authoring
    /// file's parts into the store first and packages that.
    Send { id: String },
    /// Read a Set file, store its sources, write its Set file. The id comes
    /// from the file, and one already taken is refused. `--take-in FILE`,
    /// which takes a `.kbset` as it stands and resolves a `.kset` first.
    ///
    /// A path because a file is what the only existing route takes; whether a
    /// route that has no filesystem — a model handing over the text — takes
    /// bytes instead is open, and is a smaller question than the row's own.
    Take { file: PathBuf },
}

/// Starting and stopping a session recording.
///
/// A sum rather than `{ recording: bool, id: Option<String> }`, because that
/// shape has a field that means nothing in one of its two states, and a
/// payload nobody reads is a free variable
/// (`docs/principles/0087-name-the-property-never-the-shape.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recording {
    /// Begin one, under this id or under a stamp.
    ///
    /// **A press can start one, and this doc said the opposite until
    /// 2026-09-08.** It argued that only the instant a run begins can produce
    /// a head, so a recording begun mid-performance would replay the launch
    /// deck against a late performance's records. The premise was wrong:
    /// `Recorder::open` takes a Set file's lines, and a Set file written from
    /// the live deck is what a keep already produces at any frame.
    /// `karakuri-cli` builds its head from the launch arguments because that
    /// is all it holds at that instant, not because a later head cannot be
    /// made.
    ///
    /// **What survives is narrower, and it is about the replay rather than a
    /// refusal.** A Set file says what is playing and at what values and holds
    /// no *running* state, so material that accumulates begins again from the
    /// top: a replay from a mid-performance head is a true session of the
    /// material as it stood, and not the picture that was on screen.
    /// `karakuri_store::project`'s `key_for` still holds — the projection that
    /// would fold a session down to the deck state it ends at does not exist —
    /// and it is why a head is gathered from the deck rather than from the
    /// stream.
    ///
    /// **Each start is a fresh id**, which is what keeps
    /// [P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)
    /// met: `Store::append_session` appends and `session::split` sets
    /// `started` at the first tick and never clears it, so a second head under
    /// one id would be read back as edits.
    Start { id: Option<String> },
    /// End the one running. Nothing refuses it, and `crates/karakuri`'s `rec`
    /// pill is the other end of the same press that starts one (ADR-0289).
    ///
    /// **Neither end happens on the frame.** `Recorder::finish` blocks on its
    /// writer, and so does dropping one, so a stop hands the recorder to a
    /// thread and the outcome is said when it lands —
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).
    Stop,
}

/// Define [`Operation`] and the list of headings it must match, from one
/// source, so the two cannot drift.
///
/// **This is the whole reason there is a macro here.** The test that gives
/// this crate its point compares a list of titles against the manual, and a
/// hand-written list beside a hand-written enum is exactly the "two lists
/// agreeing" this type exists to abolish — a variant added without a title
/// would leave the test passing over an operation nobody specified. Through
/// this, a variant cannot be added except with its title, and the title cannot
/// be added except with a variant.
macro_rules! operations {
    (
        $(
            $(#[$attr:meta])*
            $name:ident $( { $( $(#[$field_attr:meta])* $field:ident : $ty:ty ),+ $(,)? } )?
                => $title:literal,
        )+
    ) => {
        /// **Every operation Karakuri can perform, named once.**
        ///
        /// One variant per `<h3>` of `docs/manual/operations.html`, in that
        /// page's order, carrying what the operation acts on. See the crate
        /// documentation for what that means and what it deliberately leaves
        /// out.
        #[derive(Debug, Clone, PartialEq)]
        pub enum Operation {
            $( $(#[$attr])* $name $( { $( $(#[$field_attr])* $field : $ty ),+ } )? , )+
        }

        impl Operation {
            /// The heading this operation is specified under, verbatim.
            ///
            /// **Verbatim matters**: the test compares these against the
            /// page's own `<h3>` text, so a title edited here to read better
            /// is a failing test rather than a silent divergence. The manual
            /// is the specification; a title that reads badly is fixed on the
            /// page first.
            pub fn title(&self) -> &'static str {
                match self {
                    $( Operation::$name { .. } => $title, )+
                }
            }

            /// Every heading, in the manual's order. What the test checks the
            /// page against, and what a surface enumerating the vocabulary
            /// reads.
            pub const TITLES: &'static [&'static str] = &[ $( $title, )+ ];
        }
    };
}

operations! {
    // ----- Transport and tempo -----------------------------------------

    /// Three taps or more set the tempo; any tap sets the phase.
    ///
    /// No payload: a tap is an instant, and the instant is when the operation
    /// arrives.
    TapBeat => "Tap the beat",

    /// Moves the tracker's octave window with it.
    ScaleGrid { by: GridScale } => "Halve or double the grid",

    /// The delay between what a room hears and what it sees, signed.
    ///
    /// **Absolute, though the heading says nudge.** `--latency-offset-ms MS`
    /// is absolute, an absolute value can express every nudge and a nudge
    /// cannot express a setting, and a fader has to be able to reach it.
    /// `o` and `p` are two translations that read the current value and add
    /// five.
    SetLatencyOffset { ms: f32 } => "Nudge the latency offset",

    /// A mode the deck's material cannot honour is skipped with a reason,
    /// which is a refusal and not a payload.
    SetSync { deck: u8, sync: Sync } => "Set a deck's sync mode",

    /// **The one relative operation here, and it is relative because nothing
    /// in this instrument can set a position.**
    ///
    /// Scrubbing moves closed-form material by an amount; the manual is
    /// explicit that accumulating material cannot be moved to a position at
    /// all. An absolute `at_beat` would be inventing an operation that does
    /// not exist for two thirds of the material — see the report.
    ScrubDeck {
        deck: u8,
        /// How far, in beats. A quarter beat is what a key press asks for.
        beats: f64,
    } => "Scrub a deck a quarter beat",

    /// What the grid runs at with nothing driving it. `--bpm`.
    SetFreeRunTempo { bpm: f32 } => "Set the free-run tempo",

    /// An audio input to track, or an external process to follow.
    AttachBeatSource { source: BeatSource } => "Attach a beat source",

    // ----- Decks --------------------------------------------------------

    /// Which deck the keys are addressed to.
    ///
    /// **A console pointer**: it writes no record, and it is the reason every
    /// other variant names its deck instead of meaning *the selected one*.
    SelectDeck { deck: u8 } => "Select a deck",

    /// **Three states, and one operation naming one of them.** A slot is
    /// [`Residency::Live`] — stepped and composited — or
    /// [`Residency::Priming`], stepped out of sight and contributing nothing
    /// to the mix, or [`Residency::Allocated`], compiled and held with its `t`
    /// where it stopped.
    ///
    /// This was two `bool`s, and two `bool`s are four combinations for three
    /// states with **both `false` destinations undefined**: nothing in the
    /// vocabulary said where a deck taken off air or a prime request withdrawn
    /// landed. The answer existed, in `karakuri-cli`'s key handler and nowhere
    /// else, which is exactly the knowledge a vocabulary exists to take out of
    /// the surfaces. The engine's own setter had the shape all along —
    /// `Deck::set_residency(slot, Residency)`, one call naming one of three.
    ///
    /// **Live is honoured and Priming is a request**, which is the distinction
    /// the two rows carried between them and this one carries in its prose.
    /// The governor may hold a slot below what was asked for and never above,
    /// so Live lands and nothing demotes it, while a slot asked to prime with
    /// no room is *parked*: the request stands, is reconsidered on every pass,
    /// and takes effect the moment there is room, with nothing withdrawn and
    /// nothing remembered. A surface therefore draws the residency the deck
    /// reports rather than the one it asked for, and `park` is the existing
    /// name for the two disagreeing.
    SetResidency { deck: u8, residency: Residency }
        => "Put a deck on air, prime it, or take it off",

    /// **The single largest gap in the manual's table**: nothing loads a Set
    /// into a running deck.
    ///
    /// The id is a store id — what the library shows, what `save_set` comes
    /// back naming, what `--load-set` takes. `--set`'s file paths are the
    /// launch spelling of the same operation and are not carried: a path is
    /// how a Set is *authored*, and the library is what the panel drags from.
    LoadSet { deck: u8, set: String } => "Load material into a deck",

    /// Composite a deck's renderers rather than overdrawing them.
    ///
    /// A `bool` although `--merge N` can only turn it on, because the manual's
    /// gap section names un-compositing as one of the things reachable from
    /// nothing at all. Naming it is what makes it a gap rather than an
    /// absence.
    SetCompositing { deck: u8, compositing: bool } => "Composite a deck's renderers",

    // ----- Mixing -------------------------------------------------------

    /// The trim: the level material arrives at, colour only. Not clamped at
    /// 1.0 — the mix is HDR.
    SetGain { deck: u8, gain: f32 } => "Gain",

    /// The fader across the blend, and the only one of the two that touches
    /// what a layer covers.
    SetOpacity { deck: u8, opacity: f32 } => "Opacity",

    /// **Names the mode, where `karakuri-midi`'s `Action::CycleBlend` could
    /// only step.** A pad that means *over* is a mapping this made writable,
    /// and `note 40 -> blend 0 over` is that mapping.
    SetBlendMode { deck: u8, blend: BlendMode } => "Blend mode",

    /// Starts at the current quantum and lasts the current length, both of
    /// which are [`Operation::SetTransition`]'s and not this one's — the
    /// manual is explicit that they are a console setting deciding what the
    /// *next* fade means.
    FadeDeck {
        deck: u8,
        /// Where the opacity ends up. Opacity only: a gain fade is in the
        /// record vocabulary and has no control.
        to: f32,
    } => "Fade a deck out or in",

    /// One gesture, four records: `to` is silenced and put on air at once, and
    /// the two fades land on the grid.
    ///
    /// **Both decks named.** *The next deck* is the keyboard's translation of
    /// this, not the operation.
    Crossfade { from: u8, to: u8 } => "Crossfade to the next deck",

    /// A mask at position 0 on the incoming deck, put on air under `over`, and
    /// one scheduled move carrying the front to 1. Refused with no shape
    /// chosen.
    ///
    /// **The put-on-air and the `over` are written only where they change
    /// something**, which is the one part of that sentence that is a
    /// condition rather than a record. A deck the operator has moved off the
    /// mode a deck starts in keeps the mode: a wipe under `add` or `max` is a
    /// wipe *on* rather than a wipe *over*, a different picture and one they
    /// may have chosen, and a gesture is not where an operator's choice is
    /// taken back
    /// (`docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`).
    /// A deck already live is not told so again. What decides it is a reading
    /// of where the deck already sits in the mix, which is
    /// `karakuri_operation_record::Current::mix`, so the condition is one
    /// sentence in one place rather than each surface's.
    Wipe {
        /// The deck being covered.
        from: u8,
        /// The deck arriving over it.
        to: u8,
    } => "Wipe the next deck in",

    /// **What shape of the frame a deck's layer reaches**, and which way a
    /// linear front runs, in radians.
    ///
    /// Half of a mask, and the half that is a *choice*: `WipeKind::None`
    /// reveals everything at every position, and an angle is what makes one
    /// linear front a different picture from another. It says nothing about
    /// how far the front has travelled, which is
    /// [`Operation::SetMaskPosition`].
    ///
    /// **Not the row `z` presses.** That key sets the shape the *next* wipe
    /// takes, which is a console setting deciding what a later gesture means
    /// ([`TransitionSetting::WipeShape`]); this is the shape a deck's mask has
    /// now, and it names a deck because it changes one.
    ///
    /// **`softness` is not here**, on `white_point`'s terms at
    /// [`Operation::SetExposure`]: it is in `Record::Mask`, it has one
    /// constant behind it — `karakuri-cli`'s `MASK_SOFTNESS`, which says of
    /// itself that it is *"not a key"* — and no control on any surface. The
    /// conversion fills it in from the mask that is running.
    SetMaskShape {
        deck: u8,
        kind: WipeKind,
        /// Which way a linear front runs, in radians. The other two shapes
        /// ignore it, exactly as the mask does.
        angle: f32,
    } => "Set a deck's mask shape",

    /// **How far a mask's front has travelled**, `[0, 1]` — 0 reveals nothing
    /// anywhere and 1 reveals everything, both exactly.
    ///
    /// **This is why the mask is two rows and not one.** A single row carrying
    /// a shape, a position and a softness together could not satisfy this
    /// crate's own standing rule that *a continuous control is set, not
    /// nudged*: `karakuri-midi`'s grammar refuses the cross product in both
    /// directions — *"a note is a press, and this control takes a position"*
    /// and *"a control change is a position, and this control takes a press"*
    /// — so a row that was a sum would be a press, and no control change could
    /// ever reach a mask position. Splitting it is
    /// [ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)
    /// one layer down: an operation asks for what a surface can say, and
    /// `Record::Mask` stays whole.
    ///
    /// **It cancels a move, as the gain and the fader do.** It is the number a
    /// wipe's transition is writing, so a hand on it wins and whatever was
    /// moving it stops — which is the rule reaching the one control that had
    /// an exemption from it. `karakuri_engine::deck::Deck` is where that is
    /// written and
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// is the rule itself. [`Operation::SetMaskShape`] does not cancel,
    /// because it writes no position.
    SetMaskPosition {
        deck: u8,
        /// Where the front is, `[0, 1]`.
        position: f32,
    } => "Set a deck's mask position",

    /// These change nothing you can see and write nothing to the stream. They
    /// decide what the *next* fade, crossfade or wipe means.
    SetTransition { setting: TransitionSetting }
        => "Choose the wipe shape, the quantum, the length",

    /// Only where the deck composites and holds two or more. One-way: no
    /// position in the cycle folds them all back in.
    SelectRenderer {
        deck: u8,
        /// Which renderer, in draw order — the numbering `--param L4:1:…` and
        /// a `select` record use.
        renderer: u32,
    } => "Choose which renderer of a deck is live",

    /// **One level on the composited frame, at the entry to the master
    /// chain** — the whole fold rather than one deck of it.
    ///
    /// **It names no deck, and that is the one thing to get right here.**
    /// Every other row in this group carries `deck: u8` because it acts on one
    /// slot of the mix; this acts on what the mix *produced*, after every
    /// edge has been applied. `karakuri_engine::deck::Deck::set_out` says so
    /// at the setter — *"Not per slot"* — and it is also why nothing can
    /// schedule a move on it: a `Control` is per slot, so there is no
    /// transition for a hand here to cancel.
    ///
    /// **It is not [`Operation::SetExposure`], and the difference is where
    /// each one multiplies rather than what either one means.** This is
    /// applied where the mix writes the composited frame; the exposure is
    /// applied where the present pass reads it; feedback, bloom and rgb shift
    /// go between them. Until one of those exists there is nothing between the
    /// two multiplications and no frame tells them apart — that cost was
    /// weighed and taken in
    /// `docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md`,
    /// against the alternative of folding them into one number that would have
    /// to be pulled back out of the tone mapper the day the chain is not
    /// empty.
    ///
    /// **Unbounded above 1.0 and floored at zero**, which is
    /// [`Operation::SetGain`]'s range and the same function behind it: the
    /// pipeline is linear HDR and this level is applied to values a tone
    /// mapper has not seen yet
    /// (`docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md`).
    /// The clamp is the engine's, not this crate's — a conversion that
    /// clamped would be a second opinion about a range the setter already
    /// holds.
    SetMasterOut { out: f32 } => "Master out",

    /// **The operator moving the feedback effect's parameters.** The chain is
    /// fixed and that is what decides this row and the two below it: it is
    /// presets, all of them loaded, in the order the console draws them —
    /// feedback, then bloom, then rgb shift — between [`Operation::SetMasterOut`]
    /// above and [`Operation::SetExposure`] below. Nothing here edits the
    /// chain, switches one effect off or adds an effect somebody wrote; those
    /// are controls the panel does not draw and decisions nobody has taken.
    ///
    /// **A state and never a step**: [`Feedback`] is where the pass is put,
    /// not how far it moves, so two surfaces holding this operation cannot
    /// disagree about where the pass is
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    ///
    /// **The cut was the open question and it is answered.** Until 2026-09-09
    /// this payload was [`Undecided`] because *which cut of the previous frame
    /// it reads* had three live answers and a cut that is read has to be held,
    /// so naming one recomposed the pipeline rather than setting a value on
    /// it. The maintainer's answer was **both, selectable** — so the cut is a
    /// parameter of this pass, only the chosen one is retained, and the
    /// pipeline it recomposes is one `copy_texture_to_texture` at a different
    /// point in the frame. See
    /// `docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md`.
    ///
    /// **The amount is bounded and this crate does not bound it.** The engine
    /// clamps to `[0, 0.95]` where the record is applied, so every route in
    /// meets the same wall — a conversion that clamped here would be a second
    /// opinion about a range the setter already holds, which is
    /// [`Operation::SetMasterOut`]'s rule.
    SetFeedback { params: Feedback } => "Feedback",

    /// The frame's bright parts spreading into what is beside them, on
    /// [`Operation::SetFeedback`]'s terms: a preset, always present, and what
    /// this names is the operator moving its parameters.
    ///
    /// **It runs in linear HDR, before the one tone map**, which is why it is
    /// on this side of [`Operation::SetTonemap`] rather than the other — what
    /// it blooms from is unbounded light, where the same effect after the
    /// transfer would bloom from a displayable approximation of it
    /// (`docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md`).
    /// It reads what it is handed, so the level it blooms from is
    /// [`Operation::SetMasterOut`]'s and not [`Operation::SetExposure`]'s,
    /// which is ADR-0224's two multiplications seen from between them.
    ///
    /// **One parameter, and the two that are fixed are named at [`Bloom`]
    /// rather than left out of the story**: the knee is 1.0 because that is
    /// the top of the range the sRGB encode is honest about, and the radius is
    /// fixed because the tap count is what a radius costs.
    SetBloom { params: Bloom } => "Bloom",

    /// The three channels sampled apart, so an edge fringes. The last of the
    /// three and on [`Operation::SetFeedback`]'s terms, so it is the one the
    /// other two are seen through.
    ///
    /// **The console drew a dash here where the other two carried a number,
    /// and it draws a figure now.** The dash meant *an effect nobody has given
    /// a value*, which stopped being true the moment the chain existed: every
    /// pass has an amount, zero is a value, and an amount of zero is the pass
    /// not being recorded at all. It is still not a per-effect switch — the
    /// chain is every preset, always — and an empty payload here would still
    /// be the reading the manual refuses at this row.
    SetRgbShift { params: RgbShift } => "RGB shift",

    /// The transfer from unbounded linear HDR to something displayable.
    ///
    /// Names the operator; it does not carry the level going into it, which is
    /// [`Operation::SetExposure`].
    SetTonemap { tonemap: Tonemap } => "Tone map",

    /// The level going into that transfer, set outright.
    ///
    /// **Two operations where `karakuri_store::record::Record::Look` is one
    /// record**, and the record is right to be one: a stream that set the
    /// exposure without saying which operator it applies to would be
    /// describing a look nobody can reconstruct. That reason is a reason about
    /// **a record**. A record is what a replay reconstructs a session from, so
    /// it must be complete on its own; an operation is what a surface *asks
    /// for*, and the place that turns one into the other already knows the
    /// look that is running — `karakuri-cli`'s `set_exposure` builds the
    /// record from `Look { exposure, ..self.look }`, filling the operator in
    /// from the current one, and has since before this crate existed.
    ///
    /// **What forced the split**: a control change turns exposure alone.
    /// `karakuri-midi` has no engine, no state and no readback by charter
    /// (`docs/adr/0180-…`), so `cc → exposure` — a route the manual marks as
    /// existing — could not become an operation at all while the only variant
    /// demanded an operator beside it. See `docs/adr/0192-…`.
    ///
    /// **`white_point` is in that record, has no control on any surface and no
    /// row on the page**, so it is not here — see the report.
    SetExposure { exposure: f32 } => "Exposure",

    // ----- The sequencer ------------------------------------------------
    //
    // **A lane is a fifth route into this vocabulary and not a binding**
    // (`docs/adr/0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md`),
    // which is why this section is five rows and not more: a lane emits
    // operations on the beat the way the pointer, the keys, a map and a model
    // emit them, so the three lanes the console draws first are deck faders
    // emitting [`Operation::SetOpacity`] and the fourth writes a Set
    // parameter, which is [`Operation::WriteParam`]. **A lane needs no new
    // operation to drive anything.** These five are the other half — what a
    // hand does to the pattern — and every one of them carried [`Undecided`]
    // until 2026-09-09 because **nothing in this program held a pattern**.
    // Where one is kept was already settled (ADR-0227: library data in two
    // tiers, on the arrangement's shape); what one *is* is ADR-0320, and
    // `karakuri_pattern` is what holds it. **Each names the bank it acts on**,
    // which is [`Operation::SelectDeck`]'s rule: implying the armed one is the
    // shape that record refuses.

    /// A step of one lane, on or off, heard the next time the playhead reaches
    /// that step rather than when it was asked for.
    ///
    /// **`SetStep` and not `ToggleStep`**, because there are no toggles in
    /// this vocabulary and the reason is at the top of this file: a toggle is
    /// an affordance built over two operations by whoever draws it, and a map
    /// with a button per direction has to be able to say *this step is on* and
    /// mean it. The manual's heading is the operator's word for the control
    /// and the title is copied from it verbatim, which is all a title is for.
    ///
    /// **The address is a bank, a lane and a slot**, which is the sentence
    /// this payload used to be [`Undecided`] for: it *"names a step and cannot
    /// yet say what it is a step of"*, and
    /// `docs/adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md`
    /// is what it is a step of. Every one of the three is named rather than
    /// implied, which is [`Operation::SelectDeck`]'s rule: implying the armed
    /// bank would be the shape that record refuses.
    ///
    /// **`step` is one of sixteen stored slots and not a step index.** A
    /// pattern holds sixteen either way and an eighth reads slot `2k`
    /// ([`StepMode::slot_of`]), so a surface in the finer reading sends the
    /// even ones. That keeps this payload independent of the mode, so a step
    /// press and a mode press cannot race into an address that means two
    /// things.
    ///
    /// **What an on step is *worth* is not here**, and that is the decision
    /// rather than an omission: a cell is a bit and the two levels are the
    /// lane's, because a level only means anything against what the lane
    /// drives. `karakuri_pattern::Lane` carries them.
    ///
    /// **The grid under it is no new clock** — a step is the beat clock
    /// subdivided and a pure function of `Oscillator::beats`, so correcting
    /// the tempo changes the rate from now on without moving a beat that has
    /// already happened (ADR-0222).
    SetStep { pattern: u8, lane: u8, step: u8, on: bool } => "Toggle a step",

    /// The pattern is kept and drives nothing, and the control is the lane's
    /// own label.
    ///
    /// **It is not [`Operation::TakeParamBack`], and ADR-0222's consequence
    /// saying that it already is does not hold.** That operation names a
    /// `deck` and a [`ParamAt`] — a parameter *inside that deck's Set* —
    /// where three of the four lanes the console draws are deck faders, which
    /// are no Set's. It is the same argument that record used to kill the
    /// binding reading in its own body, *"there is no binding on a deck fader
    /// anywhere in the engine"*, so it reaches one lane in four and an
    /// operation that reaches one lane in four is not this row. The mute is a
    /// lane's, addressed the way a lane is addressed.
    ///
    /// **A bank, a lane and the state**, addressed the way [`Operation::SetStep`]
    /// is addressed and for its reason. **Not a toggle**, also for its reason.
    ///
    /// **It is where a hand takes a lane back.** A hand's write lands at once
    /// and the lane writes again at the next step, so the way to keep what a
    /// hand did is to mute the lane — rule 02's *take back sits next to it*,
    /// drawn on the lane label rather than on the strip
    /// (`docs/adr/0322-the-sequencer-is-polled-like-a-transition-live-only-and-its-writes-are-its-record.md`).
    SetLaneMute { pattern: u8, lane: u8, muted: bool } => "Mute a lane",

    /// The foot's `+ lane`, and **the target is the whole of what is being
    /// added**: a lane with nothing to drive emits nothing, so there is no
    /// moment at which a lane exists and its target does not.
    ///
    /// What a target may *be* is answered by the rest of this vocabulary —
    /// anything on it a lane can emit — which is why the console draws three
    /// deck faders and a Set parameter side by side and calls all four lanes.
    /// **What a target is spelled as is [`LaneTarget`]**: an operation of this
    /// vocabulary with its value left out, which reaches all four where a slot
    /// number reaches three and a node address reaches one
    /// (`docs/adr/0321-a-lanes-target-is-an-operation-with-its-value-elided.md`).
    /// It was [`Undecided`] until 2026-09-09 because *"a deck fader is a slot
    /// number and a Set parameter is a [`NodeAddress`] and a [`ParamAt`], and
    /// nothing here spells both"* — the spelling that reaches both is the one
    /// this vocabulary already uses to reach either.
    ///
    /// **It appends, so there is no lane index.** The panel draws no control
    /// for re-pointing a lane that already exists, so this row is where a
    /// target is chosen; if it turns out to be two operations it will be
    /// because a control was drawn for the second. **Removing a lane has no
    /// control, no row and no operation**, and that is a gap the console page
    /// carries a note for rather than an invention here.
    ///
    /// **The two levels are not here, and that is a decision** taken when the
    /// chooser was drawn
    /// (`docs/adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md`).
    /// A lane carries an `on` and an `off`, filled in *at the press* from the
    /// range the surface was published, because a pattern outlives the Set it
    /// was written against — and the surface that appends the lane is where
    /// that reading already is, so carrying them here would put two floats in
    /// a payload only one caller could supply meaningfully. A map line and a
    /// model can name neither, which is ADR-0192's rule read the other way.
    PointLane { pattern: u8, target: LaneTarget } => "Point a lane at what it drives",

    /// **A mode with two values** — a sixteenth or an eighth — drawn as one
    /// pill on the grid head. The pattern is one bar, fixed, so the step count
    /// is not a second thing a hand sets: it follows the mode, sixteen cells
    /// at a sixteenth and eight at an eighth, the row keeping its width so the
    /// cells halve in the finer one.
    ///
    /// **This paragraph rationalised three pills until 2026-09-08** —
    /// *"sixteen steps of an eighth apiece is two bars, so any two of the
    /// three fix the third"* — which is arithmetic taken from the wrong two.
    /// The mock's ruler had drawn one bar of sixteenths since the same first
    /// commit, and with the length fixed at a bar neither a count nor a length
    /// has anything left to say. ADR-0306.
    ///
    /// **The payload is a bank and a [`StepMode`]**, and it was [`Undecided`]
    /// until 2026-09-09 on two things that are both gone. The list was *"a
    /// list this crate has to own, on [`Curve`]'s terms, that nothing anywhere
    /// holds yet"*, and a two-valued mode is exactly that list; what was left
    /// was ADR-0192's rule — an operation asks for what a surface can say —
    /// and the console now draws the pill that says it. **The bank is named
    /// rather than implied** ([`Operation::SelectDeck`]'s rule), because the
    /// mode is what a *pattern* is rather than a preference the head holds.
    ///
    /// **An eighth at 128 BPM is 234 ms**, which is faster than the band
    /// `docs/adr/0255-three-clocks-run-at-once-and-a-slower-ones-work-never-lands-on-a-faster-one.md`
    /// writes the beat clock's rule for; a sixteenth is 117 ms, so the finer
    /// of the two modes is the worse case and neither that record nor ADR-0222
    /// has it. ADR-0222 records the caveat rather than waving it away, and
    /// this is the row a hand would first feel it through, because it is the
    /// one that chooses the subdivision.
    SetPatternGrid { pattern: u8, grid: StepMode } => "Choose what a step is worth",

    /// The bay head's `seq 1 · seq 2 · +`: which pattern the lanes are
    /// reading. **The `+` is this same choice landing on an empty one** rather
    /// than a second operation — the arrangement pill is the same shape, and
    /// it is why [`Operation::ResetArrangement`] is the special case of
    /// putting a saved one back rather than a control of its own.
    ///
    /// **A bank index, and it is not a name.** A bank is a position in the
    /// session — four of them, fixed — and a name is what a save files a
    /// pattern under, as different as a deck slot and a Set's id
    /// (`docs/adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md`).
    /// That is what lets the `+` be this choice landing on an empty bank: an
    /// empty bank has no name, and asking for one would make the `+` a dialog.
    /// It was [`Undecided`] until 2026-09-09 because *"a pattern has no
    /// identity anywhere"* — a bank is the identity a *session* gives one, and
    /// ADR-0227's name is the one a *store* gives it.
    ///
    /// **The console draws four pills and no `+`**, which is the paragraph
    /// above carried to its end rather than a departure from it: once the
    /// count is fixed at four every bank has a pill, so *this choice landing
    /// on an empty one* is a press on `seq 3`, and a `+` beside it would be a
    /// second door to a press already on the head
    /// (`docs/adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md`).
    /// **A press names a bank and never a direction**, so asking for the one
    /// already armed is allowed and moves nothing — the cell's own rule one
    /// control down.
    ///
    /// **Keeping a pattern and putting a saved one back are not rows on the
    /// page**, so they are not variants here either: the console draws no
    /// control that saves one. They will arrive the way
    /// [`Operation::SaveArrangement`] and [`Operation::RestoreArrangement`]
    /// did — specified on the page, drawn on the console, built after that,
    /// and they are the rows that introduce the name.
    SelectPattern { pattern: u8 } => "Choose which pattern the sequencer plays",

    // ----- Inside a Set -------------------------------------------------

    /// **The sharpest gap**: a model can rewrite a whole procedure and cannot
    /// turn one knob.
    WriteParam { deck: u8, param: ParamAt, value: ParamValue } => "Write a parameter",

    /// A source, a curve and a range — which is the whole of what "how hard it
    /// reacts" means.
    ///
    /// `signal` is a name on the bus (`energy`, `beat`, `band3`, `noise`) and
    /// is a `String` rather than a list, because that bus is open by design:
    /// `docs/principles/0090-a-surface-offers-it-never-decides.md`.
    ///
    /// **A step sequencer is not one more name on that bus, and this
    /// documentation said it was planned as one.** ADR-0222 surveyed the bay
    /// before drawing it and found both halves of that plan false: the bus is
    /// stateless by construction — every value on it is a pure function of the
    /// local oscillator's `t` and `bpm`, and *"nothing seeded lives here"* —
    /// where a pattern is authored state; and it is keyed by name alone with
    /// one `Signals` per session, so two lanes sourced from `seq 1` with
    /// different targets would sample the same name in the same frame and get
    /// the same value, which is not a sequencer. **A lane is a fifth route
    /// into this vocabulary**, emitting operations on the beat the way the
    /// other four surfaces do, which is what the sequencer's five rows above
    /// are and why none of them is a binding
    /// (`docs/adr/0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md`).
    ///
    /// The noise generator's own parameters, which `--bind` also takes,
    /// describe the *source* rather than the attachment and are not carried
    /// here.
    ///
    /// **There is no confidence here and there is nowhere for one to go.** A
    /// value arrives with how well it is known and the blend is
    /// `lerp(the param's own value, the mapped signal, confidence)`, so a
    /// confidence an operator could write would be a caller telling the system
    /// how much to trust a measurement it took —
    /// `docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md`
    /// exactly inverted. It comes off the sample and off nothing else.
    ///
    /// **`range` is the range the control was *published* over.** A surface
    /// sending this has it in hand — it is what the fader on the same row is
    /// drawn against — and it is not a second thing for an operator to choose:
    /// `ParamValue` says a range *"is the procedure's declaration and not an
    /// operator's to write"*, and a published range narrows it without
    /// redefining it. So the field states which of a parameter's declared span
    /// the signal is mapped onto, and the answer a console gives is *all of
    /// what it published* (ADR-0286, ADR-0319).
    AttachSignal {
        deck: u8,
        /// **A [`BindAt`] and not a [`ParamAt`]**, because an attachment is one
        /// layer's — see [`BindAt`].
        param: BindAt,
        signal: String,
        curve: Curve,
        /// What the signal is mapped onto, low then high.
        range: [f32; 2],
    } => "Attach a signal to a parameter",

    /// Take a knob back from whatever is driving it. It is the second rule's
    /// other half — *you can always see who is holding a control, and always
    /// take it back*.
    ///
    /// **It removes the attachment rather than suspending it**, and that is a
    /// decision rather than an economy. This documentation said *"without
    /// losing the binding"* until 2026-09-09 and nothing anywhere could have
    /// done that: `karakuri_engine::binding::Binding` carries no suspended
    /// state, and a fourth thing for an attachment to be — attached, absent,
    /// suspended, and blended at a low confidence — would have to be drawn,
    /// recorded and restated on every rebuild, where *not driving this
    /// parameter* is already written and is the absence. What is given up is
    /// *hand it back* in one press; what replaces it is that the session
    /// stream carries the source, the curve and the range on the record that
    /// attached it, so re-attaching is a thing a stream can say. See
    /// `docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`.
    ///
    /// **A hand on the value is not this**, and the two are deliberately
    /// different presses. Writing a bound parameter with
    /// [`Operation::WriteParam`] moves the value a binding blends *from* and
    /// leaves the attachment where it is — order-independent by construction,
    /// which is what a blend on confidence buys — so nothing an operator does
    /// to a knob can detach a signal by accident. This operation is the only
    /// thing that detaches one.
    ///
    /// **And it is not the sequencer's lane mute**, which ADR-0222's
    /// consequences say it already is: this names a parameter inside one
    /// deck's Set and three of the four lanes the console draws are deck
    /// faders, which are no Set's. [`Operation::SetLaneMute`] is that row, and
    /// carries the argument.
    TakeParamBack { deck: u8, param: BindAt } => "Take a parameter back",

    /// **The one operation addressed by name at both ends**, which is
    /// `Record::Edge`'s decision and its reason: a position moves when the
    /// list is reordered, and reordering silently changing which geometry a
    /// morph blends towards is the exact failure that record exists to end.
    ///
    /// So the workspace really does spell a node address two ways, and the two
    /// are not a drift to be resolved — [`NodeAddress`] is positional because a
    /// param write wants a wildcard and a position is what a `.kir` gives, and
    /// this is by name because an edge must survive a reorder.
    WireInput {
        deck: u8,
        /// The node that declares the slot: `morph` in `--edge morph.far=…`.
        node: String,
        /// What that node's procedure calls it: `far`.
        slot: InputPort,
        /// The node bound to it: `sphere_shell`.
        to: String,
    } => "Wire a procedure's input to a node",

    /// What the console shows, and — since a knob binds to a position in it —
    /// what a MIDI control counts.
    ///
    /// **The whole ordered list, not one entry.** A MIDI control is bound to a
    /// position in the published interface, so adding one entry at a time
    /// would renumber every binding after it; and an interface that publishes
    /// nothing publishes everything, which is a statement about the list and
    /// not about an entry.
    Publish { deck: u8, controls: Vec<Control> } => "Narrow the published interface",

    /// Each comes from a Set file or a declared default.
    SetProperty { deck: u8, property: Property } => "Element capacity, seeds, the camera",

    /// **Who may move one node**, which is rule 06 of the manual and one of
    /// the four properties this system is defined by.
    ///
    /// **Two shapes at once, and both are already here.** It names one of
    /// three, which is [`Operation::SetResidency`]'s shape and
    /// [`Operation::SetBlendMode`]'s — the vocabulary owns the value list, so a
    /// surface asks for a destination rather than for a step
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). And it addresses a node of a deck's Set,
    /// which is [`Operation::WriteProcedure`]'s shape: a `deck` beside a
    /// [`NodeAddress`], which is *"one address shape for within a Set and one for
    /// which Set"*.
    ///
    /// **Per node rather than per deck slot**, and the manual rules the slot
    /// out in the row above it: *"There is no switch that hands the whole
    /// instrument to an agent, because the useful arrangement is almost always
    /// partial"*. A flag on a `deck: u8` is that switch at deck granularity.
    /// Per *layer* is not addressable at all — no operation here names a layer
    /// of a live Set, and [`Operation::ListSets`]'s `layer` narrows a search of
    /// the store rather than reaching one.
    /// See
    /// `docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`.
    ///
    /// **[`Layer::Field`] takes one and the merge cannot.** A field addresses
    /// no node in the rendering sense and its params are still declared,
    /// addressable and an operator's to ride, which is the whole reason that
    /// arm is in [`Layer`] — so it is a node an agent can be let at. The L5
    /// that folds a Set's renderers is a node too — `docs/ir-spec.md` says
    /// *"`crate::node::Merge` is the node"* — and it is in neither spelling of
    /// [`Layer`], because a `kind` says what a procedure lowers to and
    /// compositing has none. So this operation cannot address it, and nothing
    /// on it can be moved by anybody today either: `Record::Merge` carries no
    /// `gain`, `opacity`, `blend` or `mask` *"because a record whose producer
    /// does not exist waits for it"*. Reaching it means [`Layer`] growing an
    /// arm, which is a change to what a `.kir` may declare and not a question
    /// about authority.
    ///
    /// # What it does today, said rather than implied
    ///
    /// **Nothing writes a parameter on an agent's behalf in this workspace**,
    /// so no addressed write is refused *because of* a level. The record is
    /// kept so that one can be when something does, which is M6's, and the
    /// level is not decoration in the meantime: a **bare-name** write over
    /// nodes that are not all under one authority is refused whole
    /// (`docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`),
    /// so granting one renderer and keeping another narrows what one knob may
    /// do from the next press. A surface drawing this must not word it as more
    /// than that.
    SetAuthority {
        deck: u8,
        node: NodeAddress,
        authority: Authority,
    } => "Set a node's authority",

    /// **Writes one node's source into the operator's own library**, at
    /// `<store>/procedures/<name>.kir`, so that it can be loaded over a layer
    /// of something else afterwards
    /// ([`Operation::LoadProcedure`]).
    ///
    /// **It is the act that makes that tier exist**
    /// ([P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)):
    /// nothing else in this program writes there, and the procedures that ship
    /// under the presets root are never written by anything. The
    /// content-addressed sources every build leaves in the store are **not**
    /// this — those are the edit history's, one per compile and named by a
    /// hash, and `Operation::WalkHistory` is the surface over them.
    ///
    /// **`id` is [`Operation::SaveSet`]'s field one level down**, and it is the
    /// same pair of presses: the capsule on a node group's head types nothing
    /// and takes a stamp, and a name typed into the Inspector's pane head is
    /// what a keep from there files under
    /// (`docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md`,
    /// `docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md`).
    ///
    /// **A model asked for this writes `<store>/sandbox/`**, stamped and
    /// overwriting nothing, which is why a model is not refused here where its
    /// star is: what it saves is a file, so it has a sandbox form to land in
    /// (`docs/adr/0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md`,
    /// `docs/adr/0301-a-models-star-is-refused-because-a-favourite-has-no-sandbox-to-land-in.md`).
    ///
    /// **The node is one node.** A head standing over more than one carries no
    /// capsule, which is [`Operation::SetAuthority`]'s own rule on the same
    /// head: one control there would be one of several answers drawn as the
    /// answer.
    KeepProcedure {
        deck: u8,
        node: NodeAddress,
        /// What to file it under, or a stamp — [`Operation::SaveSet`]'s field
        /// and its reason.
        id: Option<String>,
    } => "Keep a node's procedure",

    /// **Which deck a pane of the Inspector is showing.** A pulldown on the
    /// pane's own head over the decks the mixer is drawing strips for, which
    /// is `View::select`'s refusal read again rather than a rule of its own.
    ///
    /// **It is not [`Operation::SelectDeck`]**, and the difference is the same
    /// one the Library bay's load pulldown makes
    /// (`docs/adr/0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md`):
    /// that operation moves where the keys are addressed, and this mark exists
    /// so that a pane can show a deck the keys are **not** on. A pick moves no
    /// selection, no other pane and no load target.
    ///
    /// **A pulldown rather than a flip**, which is the maintainer's choice and
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// underneath it: a flip is a step, two panes stepping cannot both be
    /// aimed without knowing where they started, and a key, a map line or a
    /// model would have to count presses to say *deck C*. This names the deck.
    ///
    /// **`pane` is a `String`**, which is [`Operation::FoldPane`]'s spelling
    /// and for its reason: this crate has no dependencies and cannot hold the
    /// arrangement's handle type, so a pane is named by the name the
    /// arrangement gives it.
    PointPane { pane: String, deck: u8 } => "Point an Inspector pane at a deck",

    // ----- The library --------------------------------------------------

    /// Writes the material **on screen** — the versions running, with their
    /// parameters, capacities, salts, layering and fold — not what any file on
    /// disk says.
    SaveSet {
        deck: u8,
        /// What to file it under, or a stamp. A key press cannot type a name;
        /// a caller that can is not made to take a timestamp.
        id: Option<String>,
    } => "Keep what a deck is playing",

    /// Most recent first, narrowed by what a node is called or by which layer
    /// a Set uses, and it says how many it did not show.
    ListSets { holds: Option<String>, layer: Option<Layer> } => "List what the store holds",

    /// **Which kinds of row the library listing shows** — the five procedure
    /// kinds and Sets, OR across the ones that are on, everything where none
    /// is. See [`LibraryKinds`], which is the whole of the payload.
    ///
    /// **It is not [`Operation::ListSets`]'s `layer`, and the two are two
    /// facts.** That field asks *which Sets hold a node on this layer* — a
    /// predicate over a Set's contents, over Sets alone — and a button here
    /// asks *is this procedure of this kind*, which is a predicate over one
    /// artifact. Folding them into one field would be a name meaning two
    /// things (`docs/contributing.md` §4), so `ListSets` keeps its field and
    /// this operation carries six states beside it. That is also why the
    /// panel's `layer` field is superseded rather than extended
    /// (`docs/adr/0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md`,
    /// `docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md`).
    ///
    /// **`holds` is untouched** and stays on `ListSets`: it narrows by node
    /// name and is a filter over what a listing holds, where this decides
    /// which populations the listing is drawn from at all.
    FilterLibrary { kinds: LibraryKinds } => "Filter the library by kind",

    /// **Which library is being read**, and the four chips the console draws
    /// are four questions rather than four acts — `docs/manual/operations.html`
    /// argues that at the row, and the console page argues the sharpest part of
    /// it: *favourites* is *"this library filtered rather than a fifth place a
    /// Set can be"*, so choosing it and choosing *my sets* differ in the
    /// question asked and not in what is asked.
    ///
    /// **[`Undecided`], and the row itself says why.** The scopes are *"the one
    /// thing about the library that is not closed: it grows when a directory is
    /// added"* — so an enum of the four here would assert that the list can be
    /// finished, which is the claim that row exists to refuse, and it would go
    /// short the moment an operator points the bay at a directory. What
    /// identifies one member of a growable list is spelled nowhere: not on that
    /// page, not on the console page, and not in this workspace. A folder scope
    /// has a path, *presets* has a root the program was told, and the other two
    /// have neither.
    ///
    /// **The key steps and this does not.** `e` moves to the next scope and
    /// wraps, and that is the translator's arithmetic rather than this
    /// operation's payload —
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md).
    /// Whatever a scope turns out to be named by, this names one.
    SelectScope { scope: Undecided } => "Choose which scope the library shows",

    /// **The star on a library row, and the fact it is a control over.** `id`
    /// is the Set the store holds; `favourite` is the state it is being put
    /// in.
    ///
    /// **`my sets` is what this fills.** It is not the listing of what
    /// `<store>/sets/` holds — that is
    /// [`ListSets`](Operation::ListSets) — but the starred subset of it, so a
    /// preset packaged on load and a recording's head land in the library
    /// without appearing there until somebody presses the star
    /// (`docs/adr/0299-my-sets-is-the-starred-subset-and-the-star-is-kept-beside-the-sets.md`).
    ///
    /// **Not a toggle**, on this crate's general rule: a map with a button per
    /// direction, a model that says which one it wants and a key all have to
    /// be able to say *star this* and mean it. `favourite` is the state, the
    /// way [`SetResidency`](Operation::SetResidency) names one of three.
    ///
    /// **The fact is a favourite and the control is a star**, which is
    /// `docs/manual/console.html`'s pair and is why this is spelled two ways.
    /// That page also decides where the value lives — beside the Sets, in the
    /// store, rather than in the Set file or in the session stream — and
    /// `karakuri-operation-record` answers `Silent(Surface)` for the second
    /// half of that sentence.
    ///
    /// **MIDI cannot reach it and that is a `gap` rather than a plan.** Every
    /// target a map line can name carries a slot, a range or a word from a
    /// closed list, and a Set id is none of the three — the same sentence the
    /// `read` and `load` pills beside this control already carry.
    SetFavourite { id: String, favourite: bool } => "Star a Set, or take the star off",

    /// Every knob with its range and default, the element count, the
    /// attributes emitted — each read off the artifact's own card, so those
    /// three fetch no source and compile nothing.
    ///
    /// **What a node's element storage comes to is the figure that is not a
    /// card's.** It needs every source in the Set fetched and checked before
    /// anything can be sized, and it pays for that. The panel draws the three
    /// that cost nothing; a model asking over MCP is offered the fourth as
    /// well.
    ReadSet { id: String } => "Read what one Set holds and declares",

    /// Send a Set to somebody, and take one in.
    TransferSet { transfer: SetTransfer } => "Send a Set to somebody, and take one in",

    /// **The listing, and the Library bay draws it**: a fifth scope chip whose
    /// rows are the versions of one Set, most recent first, off
    /// `karakuri_environment::history::list`.
    ///
    /// **Landing on a row is not this operation.** It is
    /// [`Operation::RestoreProcedure`], which carries a [`Revision`] now that a
    /// surface can name one. This row is the walk, and the page says so: *what
    /// versions has this had* is a list, landing on one is a load, and neither
    /// word is *undo*.
    ///
    /// **What a walk carries is *which history*, and it is a Set id since
    /// 2026-09-10.** The three shapes this was choosing between were answered
    /// by [ADR-0308](../../../docs/adr/0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md)
    /// and the fourth was left open for want of a surface: a cursor and a
    /// direction is the console's own mark, which has no row on the page at
    /// all; a count of steps is something no surface offers; a revision to land
    /// on is the landing's; and *which history* was *"the one thing no surface
    /// spells"*. **A model spells one**: `read_set` takes a Set id and
    /// `list_sets` hands the ids out, so the sentence that kept this
    /// [`Undecided`] had stopped being true about the surfaces this vocabulary
    /// serves
    /// (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
    ///
    /// **A walk is narrowed by a Set and not by a deck.** Two decks running one
    /// Set have one history between them, and a version is filed under the Set
    /// the slot was running
    /// (`docs/adr/0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md`),
    /// so a deck is how a console *arrives* at an id and never what the listing
    /// is about — which is why the field is the id and not the deck the panel
    /// could have said.
    ///
    /// **`None` is a Set the walk does not name, and it is a state rather than
    /// an absence.** The deck a panel aims this at can be running the pair the
    /// run launched with; those versions are filed under **no** Set, and a
    /// narrowing to a Set matches none of them rather than all of them
    /// (`docs/adr/0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md`),
    /// so a walk that names no Set lists nothing and the surface says why. It
    /// is `karakuri_environment::history::Version::set`'s own `Option` read
    /// from the asking side, and the one derivation of it on the panel is the
    /// aim (`Aiming::at`).
    ///
    /// **A model names one and is not offered the `None`**: `walk_history`
    /// requires `set`, because a walk of no Set is not a question anybody can
    /// be answered.
    WalkHistory { set: Option<String> } => "Walk the edit history",

    /// **One layer of what a deck is playing, replaced, and everything else
    /// left where it is.** A procedure declares one `kind`, and the press
    /// re-points the slot with that one file swapped for the one that was
    /// there — every other field of the aim restated, so the layering, the
    /// fold, the capacities, the salts, the camera and the wiring come back as
    /// the slot's own
    /// (`docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md`).
    /// Nothing is installed
    /// (`docs/adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md`).
    ///
    /// **`procedure` names a row of the library**, in either tier: one the
    /// operator kept under `<store>/procedures/`, or one that ships under the
    /// presets root. It is a name and never a path — the same rule
    /// [`Operation::LoadSet`]'s `set` is under.
    ///
    /// **No node address, and the limit is recorded rather than designed
    /// around.** It lands on the **first** node of that kind, so `L4:0` is the
    /// renderer a `kind L4` replaces and the second renderer of a
    /// three-renderer Set is unreachable from this operation. A library row
    /// cannot say an index, and a field only one surface could ever fill would
    /// be a payload for a control nobody has drawn; the day the Inspector's
    /// node head grows a *replace this node* control is the day this gains a
    /// [`NodeAddress`]. Where the deck has no node of that kind the procedure is
    /// added as node 0 of it, which is the case the row is for: a Set
    /// declaring no camera holds the built-in orbit at `L3:0`.
    ///
    /// **What the slot runs afterwards is a derived Set with no name**, and
    /// the versions it writes stay filed under the Set it started from —
    /// `watch::Aim`'s `set` is not moved by this operation, where
    /// [`Operation::LoadSet`] replaces it
    /// (`docs/adr/0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md`).
    /// So the `history` walk goes on listing that deck's versions and the
    /// snapshot every compile takes stays alive. Nothing is saved on the
    /// press; [`Operation::SaveSet`] is what gives the result a name.
    ///
    /// **A separate operation from [`Operation::LoadSet`] rather than a second
    /// arm of it.** That one names a Set the library holds and restates every
    /// layer; this names a procedure and restates all but one; and only one of
    /// the two leaves the slot running material with no name. What they share
    /// is the class and the timing.
    LoadProcedure { deck: u8, procedure: String } => "Load a procedure over a layer",

    // ----- Procedures ---------------------------------------------------

    /// Addressed by deck, layer and index. Read before writing.
    ReadProcedure { deck: u8, node: NodeAddress } => "Read one node's source",

    /// Checked as you write it; built on a worker and swapped at a frame
    /// boundary. The source's own `kind` line must name the same layer as the
    /// address, which is a refusal and not a payload.
    WriteProcedure { deck: u8, node: NodeAddress, source: String }
        => "Check and write one node's source",

    /// **This row names an event, not an operation, and that is the
    /// undecided part.**
    ///
    /// Somebody editing a file in another program is not something a surface
    /// performs. The only operation nearby is *watch these files, or stop* —
    /// and `--watch` takes no argument, cannot be turned off, and does not say
    /// which files it would take if it could. Whether the row is that
    /// operation, or belongs on the page at all, is a decision about the
    /// manual.
    ///
    /// **So this payload is the row's marker for one flag**, and that is what
    /// it is for: `--watch` is the only thing in this program that turns the
    /// watching on, it takes no argument and nothing turns it off, so there is
    /// no state for a payload to name until somebody decides the row is an
    /// operation. It stays [`Undecided`] rather than becoming a `bool` nothing
    /// can set.
    ///
    /// **A model has no route here, and nothing is owed for one.** A model does
    /// not edit a file in another program: it calls `write_procedure`, which
    /// **is** its edit — checked, written and swapped at a frame boundary — so
    /// there is nothing a model would say on this row that the write does not
    /// already say. That is
    /// `docs/adr/0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md`'s
    /// kind of answer rather than a missing route, and it is why the page's MCP
    /// badge is `gap` and not `plan`
    /// (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
    WatchFiles { watching: Undecided } => "Edit the file instead",

    /// Landed, overloaded for costing too much, or failed to build. A write
    /// returning cleanly means it compiled, not that it is on screen.
    ///
    /// **The middle word is a state and not an outcome**, since ADR-0316: the
    /// version is in the slot and the slot has stopped updating, holding the
    /// frame it last drew, and nothing ends that on its own.
    SwapOutcome => "Find out what a write did",

    /// **The operator's verdict on a candidate, and it is not the watchdog's.**
    ///
    /// A version reaches the screen because it compiled, and it goes on
    /// *running* because one frame of it was measured under the frame budget —
    /// `karakuri_engine::swap::Event::Accepted`, which is a judgement about
    /// **cost**. Whether it is the one to keep is a judgement about **taste**
    /// and nothing in this instrument can take it. This row is where a person
    /// takes it.
    ///
    /// **The engine's three words are deliberately not reused.** *Accepted*,
    /// *rejected* and *overloaded* already name the budget's verdict on the
    /// same object, and the staging lane is the one surface where both
    /// verdicts are visible at once — a lane offering *accept* over a
    /// candidate the watchdog had already accepted would spell two different
    /// judgements the same way
    /// (`docs/contributing.md` §4).
    ///
    /// **And it is not offered on an overloaded row at all.** That row is a
    /// slot that has stopped, which is not a candidate a person is choosing
    /// between; keeping it would settle the one row whose whole job is to say
    /// the slot is not running (ADR-0316).
    ///
    /// **Addressed by the node rather than by the version, because a node has
    /// at most one unsettled version.** A write is not held anywhere: it is
    /// checked, written, built on a worker and swapped at a frame boundary, so
    /// the candidate for a node **is** what that node is playing. Choosing
    /// among several older versions is a different operation and it is
    /// [`Operation::WalkHistory`], which names the **Set** whose versions are
    /// being chosen among rather than the node this row addresses — a walk is
    /// narrowed by a Set and a keep is settled at a node.
    ///
    /// **And the surface that says a node is the staging lane, since
    /// 2026-09-09.** A lane row is one node a build changed rather than one
    /// slot — a save touching two files draws two rows, each carrying its own
    /// address, with the build's one verdict written on both
    /// (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
    /// The control is the row itself, and the smaller box inside it is the
    /// capsule that asks for [`Operation::RestoreProcedure`]: the free act
    /// takes the large target and the act that writes a file takes the small
    /// one. **A row that names no node offers neither** — a build that failed,
    /// a source the checker turned down, and a rebuild that changed nothing
    /// are verdicts about a slot, and there is nothing for a keep to settle.
    ///
    /// **Silent, and that is its shape rather than an omission.** The material
    /// already changed and `karakuri_store::record::Record::Procedure` was
    /// written where the swap landed. What this changes is the lane: the node
    /// stops being one with a version nobody has ruled on.
    KeepCandidate { deck: u8, node: NodeAddress } => "Keep a candidate",

    /// **What *a rejected candidate costs nothing* is made of.** The version
    /// before it is a file under `<store>/history/`, kept because it
    /// **compiled** rather than because it landed — so the one thing a person
    /// most wants back, the version before the one that stopped their slot, is
    /// exactly the one that is there. **It is also the way out of a stopped
    /// slot**: nothing puts a version back on its own since ADR-0316, so
    /// landing an earlier one from here is one of the three things that ends a
    /// freeze.
    ///
    /// **Which version is a [`Revision`], because two surfaces can ask and
    /// each says a different half.** A staging lane row names the node and
    /// means the version its source replaced — one step, never a cursor. A row
    /// of the Library bay's `history` scope names the version itself, and that
    /// name carries the node with it. Walking the versions is
    /// [`Operation::WalkHistory`], which is the listing this picks out of.
    ///
    /// **Both arms are asked by the panel since 2026-09-09**, and neither
    /// resolves a file: a performer turns the arm it was given into a version
    /// and writes that version's bytes over the node's working copy. What
    /// [`Revision::Previous`] resolves to is the history walked for **that
    /// node of that Set**, most recent first, with the entry *after* the
    /// newest taken — the newest is the version the slot is running, whether
    /// it is stepping or stopped, because the history is gated on compiling
    /// and not on landing. A node whose only version is the one it is playing
    /// is refused in a sentence that says so
    /// (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
    ///
    /// **Not [`Operation::WriteProcedure`] carrying that file's text, and the
    /// reason is the surface.** A write takes a `source: String` because
    /// whoever asks for one is holding the text: a model has it in the
    /// conversation, an editor has it in a buffer. A staging lane holds
    /// neither. It can say *not this one* and it cannot say four kilobytes of
    /// IR, and making it say them would put the store inside the console —
    /// the same reach the Library bay already declines for a date format
    /// (`docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md`).
    /// So the surface says the part it can say and whoever performs it reads
    /// the file, which is
    /// `docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md`.
    ///
    /// **This does write a record**, where [`Operation::KeepCandidate`] does
    /// not: it is a procedure change, it lands at a swap like any other, and a
    /// session in which the operator put a version back and that replayed with
    /// the version they threw away is the hole `Record::Procedure` was added
    /// to close.
    RestoreProcedure { deck: u8, revision: Revision } => "Put a node's previous version back",

    // ----- Arranging the console ----------------------------------------

    /// **Undecided, and the manual says so**: *"How a pane is sized and
    /// unfolded without a mouse is not decided."*
    ///
    /// Two things are missing, not one. A boundary is `(split, index)`, and
    /// `karakuri_layout::Layout::name` answers `None` for *"a split the
    /// arrangement left unnamed"* — so more than half the boundaries in the
    /// console's arrangement have no address any surface but the pointer could
    /// say. And `Layout::set_divider` takes a position in the viewport's own
    /// coordinates, which is a pixel: a number a drag produces and a key press
    /// or a model has no way to mean.
    ///
    /// **The payload stays [`Undecided`] for the keyboard's reason**, which is
    /// the second of the two above read at a key:
    /// `docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md`
    /// is where a key that can only *step* is refused, and a viewport pixel is
    /// not something a press can mean. Settling the address half alone would
    /// not settle it.
    ///
    /// **And a model has no window, so the page's MCP badge is `gap`.** A
    /// divider's position is the arrangement's own state, which is what the
    /// twelve rows of
    /// `docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md`
    /// are — this was the thirteenth, left `plan` only because its payload is
    /// open, and that record's own consequences say so. **The two facts are
    /// held apart**: the badge is `gap` because a route into a surface's own
    /// state is a route into a window the model is not looking at, and the
    /// payload is open because no surface but the pointer can say a boundary.
    /// Settling one would not settle the other
    /// (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
    MoveBoundary { boundary: Undecided } => "Move a boundary",

    /// A folded bay takes no space at all and no divider is drawn beside it;
    /// what it had goes to the bay that takes height back in that pane.
    ///
    /// Addressed by name, which is available: `karakuri_layout::Layout::find`
    /// resolves one, and `Layout::new` refuses an arrangement that uses a name
    /// twice, so *"the answer here is the only node that could be meant."*
    FoldBay { bay: String } => "Fold a bay away",

    /// The whole left or right pane, with everything in it, and the centre
    /// takes the width.
    ///
    /// **Same payload as [`Operation::FoldBay`] and a separate row**, because
    /// the manual describes two different consequences. Whether they are one
    /// operation over two kinds of region is a question for the page — see the
    /// report.
    FoldPane { pane: String } => "Fold a pane away",

    /// **A folded region has no rectangle, so a pointer cannot reach it** —
    /// the one operation on this page a mouse cannot be the only way into.
    ///
    /// `None` brings back everything folded, which is what reaches a fold
    /// nobody has a name for; a name brings back that region and whatever
    /// stands between it and the screen. That is `panel::Op`'s `Unfold` and
    /// `UnfoldAll`, two of its variants under one heading.
    Unfold { region: Option<String> } => "Bring back what is folded",

    /// Everything that is not this region, on the way to it, or inside it
    /// folds away, and it holds the window.
    ///
    /// `None` undoes the solo and restores what was folded before, including
    /// whatever was already folded. Explicit rather than a toggle: the caller
    /// says which way.
    Solo { region: Option<String> } => "Solo a region",

    /// **The default arrangement, at the viewport the window already has** —
    /// and it takes nothing, because it acts on the arrangement as a whole.
    /// ADR-0175 put it in that group with `UnfoldAll` and `Unsolo` and it has
    /// stayed there.
    ///
    /// **What it discards is the rest of this section's promise.** *Move a
    /// boundary* says a window dragged too small *"gives everything less and
    /// forgets nothing"*, and *Fold a bay away* says a folded bay's *"size is
    /// remembered, so bringing it back puts it where it was"*. Both of those
    /// are kept in the arrangement this replaces: `karakuri_console::layout()`
    /// is built fresh and only the viewport survives, so every fold, every
    /// divider a hand has moved, the solo and both sets of remembered sizes go
    /// at once. It is the one operation on the page that forgets, and the row
    /// says so.
    ///
    /// **It is the default member of a family that now exists**, which is the
    /// framing this row was written to rather than *start again*: an
    /// arrangement is named, kept and put back, and resetting is putting back
    /// the one that came with the program.
    /// [`SaveArrangement`](Operation::SaveArrangement) and
    /// [`RestoreArrangement`](Operation::RestoreArrangement) are the other two
    /// members, and the page now says where all three live on the console.
    ///
    /// **No payload, and that is decided rather than [`Undecided`]** — and it
    /// stayed decided when the family landed. The default arrangement is not a
    /// file and there is no reserved name for it, so this variant reaches code
    /// where `RestoreArrangement` reaches a file, and the two never meet
    /// (ADR-0221 §2). A name here would make *the default* one entry of a
    /// listing an operator can overwrite.
    ResetArrangement => "Reset the arrangement",

    /// **File the running arrangement under a name the operator picked**,
    /// overwriting whatever is already kept under it.
    ///
    /// The name is a `String` and not an `Option<String>`, which is where this
    /// parts company with [`Operation::SaveSet`] beside it: a Set is
    /// *ordinarily* filed under a stamp nobody chose and an arrangement is
    /// not, because the whole of what a name is for here is that the operator
    /// will look for it again
    /// (`docs/principles/0087-name-the-property-never-the-shape.md`).
    /// A surface with nobody there to type one passes
    /// `karakuri_environment::history::stamped_id`, the same stamp
    /// `accepted_save` reaches for — so the fallback is the *caller's* and
    /// this payload never has to say *no name*
    /// (`docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md`).
    ///
    /// **One path component**, letters, digits, `-` and `_`, which is a Set
    /// id's rule and is the caller's to keep — `karakuri-environment`'s
    /// `mcp::checked_id` is where a name reached from a protocol is refused
    /// rather than sanitised.
    ///
    /// **What it writes is a file and not a record**, and those are two
    /// different things: `karakuri-operation-record` answers
    /// `Silent(Surface)` here, because a saved arrangement lives in a fourth
    /// place under the store rather than in the session stream, and a replay
    /// reconstructs nothing from one. See that crate's arm for why this is not
    /// `Silent::OnLanding`.
    SaveArrangement { name: String } => "Save the arrangement",

    /// **The arrangement filed under `name`, at the viewport the window
    /// already has** — which is [`Operation::ResetArrangement`]'s sentence
    /// with a name in it, and that is the whole relationship between the two.
    ///
    /// **Two refusals, and both were written before this row was.** Nothing
    /// filed under the name is `StoreError::NoArrangement`, which says the
    /// name back rather than resetting the console under an operator who
    /// mistyped it; and a file that disagrees with itself is refused by
    /// `karakuri-layout`'s own loader rather than repaired
    /// (`docs/adr/0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md`).
    ///
    /// **The viewport in the file is the one it was saved at and is not the
    /// one it comes back at.** A window is not part of what an operator kept:
    /// the caller sets the current viewport and solves, exactly as
    /// [`ResetArrangement`](Operation::ResetArrangement) carries the viewport
    /// across today.
    ///
    /// **This never reaches the built-in.** The default arrangement is not a
    /// file and there is no reserved name, so an operator may keep one of
    /// their own called `default` and it shadows nothing (ADR-0221 §2).
    RestoreArrangement { name: String } => "Put a saved arrangement back",

    // ----- Output and recording -----------------------------------------

    /// **A window drag sets an output's size**, which is what this row means
    /// since 2026-09-09. The render size belongs to an output and not to the
    /// session
    /// ([ADR-0246](../../../docs/adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)),
    /// so dragging the projector window sizes that output and dragging the
    /// console's own window — or a divider inside it — sizes the program
    /// view, whose size is the rectangle the Program bay gives the picture.
    /// The frame is composited once at the largest enabled output's size and
    /// scaled into each
    /// ([ADR-0247](../../../docs/adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md)),
    /// so a drag moves what a frame costs. *The window is a preview and has no
    /// say in what is drawn* is what this said while ADR-0077 stood, and its
    /// premise was removed rather than argued with.
    ///
    /// **No control on this console sizes a window and none is planned.** The
    /// window manager draws the frame a hand drags, on every platform this
    /// program runs on, and `crates/karakuri` answers `WindowEvent::Resized` —
    /// which is [`Quit`](Operation::Quit)'s argument one row along, and why
    /// the page's panel column is a `gap` and the badge that says the route is
    /// real is in the fifth cell.
    ///
    /// **`karakuri-cli`'s `a` is a translation that asks for the canvas's own
    /// size, and it is that program's keyboard rather than the instrument's**
    /// (ADR-0220). The instrument binds no key here and the row's key column
    /// is a `gap`: a size is a pair of viewport pixels, and a key press cannot
    /// mean one — which is [`MoveBoundary`](Operation::MoveBoundary)'s
    /// sentence read at the window instead of at a divider. `a` naming the
    /// canvas rather than a size is the way round that, and it is a *fit*
    /// rather than a size somebody said.
    ///
    /// **`--canvas` is on this row's CLI badge and is not this operation** —
    /// see the report.
    SizeWindow { width: u32, height: u32 } => "Size the window",

    /// **One output, named, and whether it is on** — see [`Output`], which is
    /// where the closed list is argued.
    ///
    /// **It was [`Undecided`] until 2026-09-09** because no output had an
    /// identity anywhere in this workspace: `karakuri_engine::frame::Sink` is
    /// a trait with no name and no id, the projector window and the plugin
    /// sinks did not exist, and the console's Outputs row reached its one sink
    /// by folding a *layout region*, so the only route was spelled as a fold's
    /// target and not as an output at all.
    ///
    /// **The fold is still where the picture's state is kept**, and that is
    /// deliberate: `RouteFrame { output: Output::Program, on }` is what a
    /// press *asks for*, and `crates/karakuri` performs it by folding the
    /// picture's node — so there is one stored answer to *is the picture on*
    /// and this operation names it rather than duplicating it.
    ///
    /// **`on` and not a toggle.** A surface that can only switch has no way to
    /// arrive, and two surfaces switching one sink disagree about where they
    /// are —
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md).
    /// The chip's toggle is the console's affordance over the two states.
    RouteFrame { output: Output, on: bool } => "Choose where the frame goes",

    /// The timeline as it happens, replayable frame for frame.
    RecordSession { recording: Recording } => "Record the session",

    /// Waits up to five seconds for a save still being written, then says how
    /// many it left behind rather than letting a hung disk hold the quit.
    ///
    /// **The instrument binds no key to this, and stopped binding one on
    /// 2026-09-09.** `esc` quit until then and now goes up one level of the
    /// focused bay's address
    /// ([ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
    /// [ADR-0332](../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)):
    /// a ladder of `esc` presses ends in something irreversible, in front of an
    /// audience, reached by repeating one key
    /// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    /// **The way out is the window's own close**, which every platform already
    /// has a gesture for and `crates/karakuri` already answers — so the key
    /// column of this row is a `gap` and the badge that says the route is real
    /// is in the fifth cell, which is
    /// [`SizeWindow`](Operation::SizeWindow)'s argument one row back.
    Quit => "Quit",
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The macro is what keeps [`Operation::TITLES`] and the variants in step,
    /// and this is the one property of it worth asserting on its own: a title
    /// reached through a value is the same string the list holds.
    #[test]
    fn a_title_is_the_one_in_the_list() {
        let op = Operation::SetGain { deck: 0, gain: 1.0 };
        assert_eq!(op.title(), "Gain");
        assert!(Operation::TITLES.contains(&op.title()));
        assert_eq!(Operation::Quit.title(), "Quit");
    }

    /// **Titles are the key the manual is matched on**, so two variants
    /// sharing one would let a missing operation pass the cross-check: the
    /// duplicate would answer for the row and nothing would say the second
    /// variant was never specified.
    #[test]
    fn no_two_operations_share_a_title() {
        let mut seen: Vec<&str> = Vec::new();
        for title in Operation::TITLES {
            assert!(
                !seen.contains(title),
                "`{title}` is the title of two variants of `Operation` — the manual is \
                 matched by title, so the second one would never be checked against a row"
            );
            seen.push(title);
        }
    }

    /// A floor, not a count: the point is that the list cannot come back
    /// empty. The exact number is the manual's to state and is asserted
    /// against the page itself in `tests/`. It read 46 when this landed, 45
    /// once two residency rows became one (ADR-0186), 46 again since the look
    /// split into a tone map and an exposure (ADR-0192), and 48 since the mask
    /// took a row for its shape and a row for its position (ADR-0201); it
    /// moves with the page and is never lowered to make a shorter list pass.
    /// It was 49 once the arrangement gained a reset (ADR-0208), 50
    /// since a node gained an authority (ADR-0211), 52 since the
    /// staging lane gained a keep and a put-back, 54 since the
    /// arrangement's reset stopped being the only member of its family
    /// (ADR-0221), and is 55 since the master out became a level something
    /// can name (ADR-0224).
    #[test]
    fn the_vocabulary_is_not_empty() {
        assert!(
            Operation::TITLES.len() >= 55,
            "only {} operations named — the vocabulary has shrunk below what the manual \
             specifies",
            Operation::TITLES.len()
        );
    }
}
