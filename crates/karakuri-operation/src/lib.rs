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
//! (`docs/principles/0036-…`): *every surface routes into the named operation*
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
//! (`docs/principles/0028-every-control-ends-in-the-same-record.md`), and that
//! package's `mix.rs` is where the two copies of each list are checked against
//! each other, because it is the only crate in the workspace that depends on
//! the engine and on this one at once. **Functions rather than the `From` impls
//! ADR-0180 named**, and it is the orphan rule rather than a preference: both
//! types are foreign to `karakuri-cli`, so the impl is not allowed there at all
//! (`docs/adr/0194-…`).
//!
//! # Where a payload is not decided
//!
//! Twelve rows name an operation whose payload cannot be written down without
//! a decision nobody has made, and one third of a thirteenth, the camera arm
//! of [`Property`]. Four have been here longest —
//! [`Operation::MoveBoundary`], [`Operation::WalkHistory`],
//! [`Operation::WatchFiles`] and [`Operation::RouteFrame`] — and **eight
//! arrived at once, which is two whole bays being specified for the first
//! time**: the master chain's three effects ([`Operation::SetFeedback`],
//! [`Operation::SetBloom`], [`Operation::SetRgbShift`]), whose parameters
//! cannot be named while the chain itself does not exist, and the sequencer's
//! five ([`Operation::SetStep`], [`Operation::SetLaneMute`],
//! [`Operation::PointLane`], [`Operation::SetPatternGrid`] and
//! [`Operation::SelectPattern`]), which wait on a pattern — authored state
//! nothing in this program holds. They carry [`Undecided`], which is a marker
//! and not a
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
/// `docs/principles/0036-an-invariant-that-is-not-yet-true-says-so.md`, applied
/// to a payload rather than to a sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Undecided;

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
pub struct NodeAt {
    pub layer: Layer,
    /// Which node of that layer, in the order the deck's files were named.
    pub index: u32,
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
/// `docs/principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md`:
/// see
/// `docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`.
/// **A surface owns none of this**
/// (`docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md`):
/// the rule lives where the write lands, so every route meets it, and an
/// addressed [`ParamAt`] meets nothing — it says which node it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamAt {
    pub node: Option<NodeAt>,
    /// The param's own name inside the node.
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
    /// (`docs/principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md`).
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

/// How a deck meets the ones under it in the fold.
///
/// **Named `BlendMode` rather than `Blend` on purpose.** `Blend` already means
/// two different things here — `karakuri_engine::deck::Blend` is how a deck
/// meets the mix, `karakuri_ir::ast::Blend` is how an L4's fragments meet each
/// other — and a name means one thing across the system
/// (`docs/principles/0031-…`). A third `Blend` would make it three.
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
    /// (`docs/principles/0074-…`). What this is for is the two things a
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
    /// **The lower-case word a record spells**, which is what
    /// `karakuri_store::record::Record::Transition`'s `curve` carries and what
    /// a `bind` record has always carried.
    ///
    /// It arrived later than [`BlendMode::name`] and its neighbours because
    /// nothing needed it: the one operation carrying a curve —
    /// [`Operation::AttachSignal`] — writes no session record, so this list
    /// had no wire to reach. A scheduled move does: the shape a fade takes is
    /// part of what a replay reconstructs it from, and
    /// `karakuri-operation-record` is where a curve now becomes a name.
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
/// (`docs/principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md`).
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
    pub node: Option<NodeAt>,
    pub key: String,
    pub range: [f32; 2],
}

/// One of the three things [`Operation::SetProperty`] can set — and the row is
/// *"Element capacity, seeds, the camera"*, which is three operations wearing
/// one heading. Carried as a sum for [`TransitionSetting`]'s reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Property {
    /// How many elements a geometry runs at, overriding what its own
    /// `capacity` declaration names. `--capacity`.
    Capacity { node: NodeAt, elements: u32 },
    /// The salt for one node's hash builtins, so re-seeding changes randomness
    /// without touching anything structural. There is no flag for this.
    Seed { node: NodeAt, salt: u64 },
    /// **The camera, and it is the one arm with no payload that can be
    /// written down.**
    ///
    /// There are two live answers and they are not the same operation.
    /// `karakuri_store::record::Record::Camera` describes a built-in orbit —
    /// `{ kind, index, radius, speed }` — while L3 is a procedure layer whose
    /// params are written by [`Operation::WriteParam`] like any other node's.
    /// Which of those *is* "the camera" decides whether this arm carries three
    /// numbers or should not exist at all, and there is no flag and no key on
    /// either side to read the answer off.
    Camera(Undecided),
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
    Start { id: Option<String> },
    /// End the one running. **Nothing does this today** — `--record-session`
    /// is a launch flag and the run stops when the program does.
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
    /// (`docs/principles/0079-nothing-takes-the-show-down-and-nothing-takes-it-away-from-the-operator.md`).
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
    /// [P-0078](../../../docs/principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md)
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
    /// **[`Undecided`] twice over, which is what makes this the sharpest of
    /// the three.** No parameter of it can be written down while the chain
    /// does not exist —
    /// `docs/adr/0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md`
    /// declines to name a chain's contents for exactly that reason, *"a record
    /// kept for a thing that does not exist would be inventing its
    /// contents"* — and feedback has a second question the other two do not:
    /// **which cut of the previous frame it reads**. A Set's output, the raw
    /// frame the mix wrote, or the frame as it stands after some effect in
    /// this chain are three different things and nothing here can say which,
    /// and it is not a control that is missing: a cut that is read has to be
    /// held, so naming one recomposes the pipeline rather than setting a
    /// value on it.
    SetFeedback { params: Undecided } => "Feedback",

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
    /// [`Undecided`] because the chain does not exist and no parameter of it
    /// is named anywhere — the row above carries the argument.
    SetBloom { params: Undecided } => "Bloom",

    /// The three channels sampled apart, so an edge fringes. The last of the
    /// three and on [`Operation::SetFeedback`]'s terms, so it is the one the
    /// other two are seen through.
    ///
    /// **The console drawing a dash here where the other two carry a number
    /// is an effect nobody has given a value, not a fourth state**: the chain
    /// is every preset, always, so there is no per-effect switch for a dash to
    /// be the off position of. Written down because an empty payload here
    /// would be that reading — *this acts on nothing* — and it is the one the
    /// manual refuses at this row.
    ///
    /// [`Undecided`] for the row above's reason.
    SetRgbShift { params: Undecided } => "RGB shift",

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
    // hand does to the pattern — and every one of them carries [`Undecided`]
    // because **nothing in this program holds a pattern**. Where one is kept
    // is settled (ADR-0227: library data in two tiers, on the arrangement's
    // shape) and what one *is* is not, so a step, a lane and a target have no
    // spelling for an operation to carry.

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
    /// [`Undecided`], because the payload is which step of which lane of which
    /// pattern, and this row *"names a step and cannot yet say what it is a
    /// step of"*. **The grid under it is no new clock** — a step is the beat
    /// clock subdivided and a pure function of `Oscillator::beats`, so
    /// correcting the tempo changes the rate from now on without moving a beat
    /// that has already happened (ADR-0222) — so what is open is the address
    /// and the state it is set to, and nothing about time.
    SetStep { step: Undecided } => "Toggle a step",

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
    /// [`Undecided`], because what that address is spelled as waits on the
    /// same pattern the row above waits on. **Not a toggle**, for
    /// [`Operation::SetStep`]'s reason.
    SetLaneMute { lane: Undecided } => "Mute a lane",

    /// The foot's `+ lane`, and **the target is the whole of what is being
    /// added**: a lane with nothing to drive emits nothing, so there is no
    /// moment at which a lane exists and its target does not.
    ///
    /// What a target may *be* is answered by the rest of this vocabulary —
    /// anything on it a lane can emit — which is why the console draws three
    /// deck faders and a Set parameter side by side and calls all four lanes.
    /// **What a target is spelled as is not answered**: a deck fader is a slot
    /// number and a Set parameter is a [`NodeAt`] and a [`ParamAt`], and
    /// nothing here spells both. So [`Undecided`], and picking one of the two
    /// would be an operation that reaches one lane in four — this section's
    /// own failure, one row up.
    ///
    /// The panel draws no control for re-pointing a lane that already exists,
    /// so this row is where a target is chosen; if it turns out to be two
    /// operations it will be because a control was drawn for the second.
    PointLane { target: Undecided } => "Point a lane at what it drives",

    /// The head's three pills — `16`, `1/8`, `2 bars` — and **they are not
    /// three settings**: sixteen steps of an eighth apiece *is* two bars, so
    /// any two of the three fix the third, and the panel shows all three
    /// because an operator reads a pattern in whichever of them they are
    /// counting in. A length and a subdivision is what this carries either
    /// way, and which two a hand sets is a decision about the control rather
    /// than about the operation.
    ///
    /// **[`Undecided`] although the manual says what it will carry**, and the
    /// two do not disagree: a length and a subdivision are a *pattern's*, and
    /// no pattern has a name for this to be of. This vocabulary names what an
    /// operation acts on and never implies it — [`Operation::SelectDeck`]'s
    /// rule, applied where there is not even a selection to imply — and a
    /// subdivision would additionally be a list this crate has to own, on
    /// [`Curve`]'s terms, that nothing anywhere holds yet.
    ///
    /// **An eighth at 128 BPM is 234 ms**, which is faster than the band
    /// `docs/principles/0069-the-three-clocks-never-collapse-into-each-other.md`
    /// writes the beat clock's rule for. ADR-0222 records that caveat rather
    /// than waving it away, and this is the row a hand would first feel it
    /// through, because it is the one that chooses the subdivision.
    SetPatternGrid { grid: Undecided } => "Choose a pattern's steps and what a step is worth",

    /// The bay head's `seq 1 · seq 2 · +`: which pattern the lanes are
    /// reading. **The `+` is this same choice landing on an empty one** rather
    /// than a second operation — the arrangement pill is the same shape, and
    /// it is why [`Operation::ResetArrangement`] is the special case of
    /// putting a saved one back rather than a control of its own.
    ///
    /// [`Undecided`], because a pattern has no identity anywhere: ADR-0227
    /// settles that one is kept under the store under a name the operator
    /// typed, and leaves what the file is called and what it holds to the
    /// record that has something to serialise.
    ///
    /// **Keeping a pattern and putting a saved one back are not rows on the
    /// page**, so they are not variants here either: the console draws no
    /// control that saves one. They will arrive the way
    /// [`Operation::SaveArrangement`] and [`Operation::RestoreArrangement`]
    /// did — specified on the page, drawn on the console, built after that.
    SelectPattern { pattern: Undecided } => "Choose which pattern the sequencer plays",

    // ----- Inside a Set -------------------------------------------------

    /// **The sharpest gap**: a model can rewrite a whole procedure and cannot
    /// turn one knob.
    WriteParam { deck: u8, param: ParamAt, value: ParamValue } => "Write a parameter",

    /// A source, a curve and a range — which is the whole of what "how hard it
    /// reacts" means.
    ///
    /// `signal` is a name on the bus (`energy`, `beat`, `band3`, `noise`) and
    /// is a `String` rather than a list, because that bus is open by design:
    /// `docs/principles/0009-what-a-binding-can-express-is-not-baked-into-the-engine.md`.
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
    AttachSignal {
        deck: u8,
        param: ParamAt,
        signal: String,
        curve: Curve,
        /// What the signal is mapped onto, low then high.
        range: [f32; 2],
    } => "Attach a signal to a parameter",

    /// Stop a signal driving a knob without losing the binding. **Nothing does
    /// this today**; it is the second rule's other half.
    ///
    /// **And it is not the sequencer's lane mute**, which ADR-0222's
    /// consequences say it already is: this names a parameter inside one
    /// deck's Set and three of the four lanes the console draws are deck
    /// faders, which are no Set's. [`Operation::SetLaneMute`] is that row, and
    /// carries the argument.
    TakeParamBack { deck: u8, param: ParamAt } => "Take a parameter back",

    /// **The one operation addressed by name at both ends**, which is
    /// `Record::Edge`'s decision and its reason: a position moves when the
    /// list is reordered, and reordering silently changing which geometry a
    /// morph blends towards is the exact failure that record exists to end.
    ///
    /// So the workspace really does spell a node address two ways, and the two
    /// are not a drift to be resolved — [`NodeAt`] is positional because a
    /// param write wants a wildcard and a position is what a `.kir` gives, and
    /// this is by name because an edge must survive a reorder.
    WireInput {
        deck: u8,
        /// The node that declares the slot: `morph` in `--edge morph.far=…`.
        node: String,
        /// What that node's procedure calls it: `far`.
        slot: String,
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
    /// (`docs/principles/0074-…`). And it addresses a node of a deck's Set,
    /// which is [`Operation::WriteProcedure`]'s shape: a `deck` beside a
    /// [`NodeAt`], which is *"one address shape for within a Set and one for
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
    SetAuthority {
        deck: u8,
        node: NodeAt,
        authority: Authority,
    } => "Set a node's authority",

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
    /// [P-0074](../../../docs/principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md).
    /// Whatever a scope turns out to be named by, this names one.
    SelectScope { scope: Undecided } => "Choose which scope the library shows",

    /// Every knob with its range and default, the element count, the
    /// attributes emitted — without fetching a source or compiling it.
    ReadSet { id: String } => "Read what one Set holds and declares",

    /// Send a Set to somebody, and take one in.
    TransferSet { transfer: SetTransfer } => "Send a Set to somebody, and take one in",

    /// **Every compile writes into the store's history and nothing reads it.**
    ///
    /// Undecided because there is nothing anywhere to read a payload off. A
    /// walk needs a cursor and a direction, or a revision to land on, or a
    /// count of steps — and the store has no reader, no surface has a control,
    /// and the manual's own row marks all four routes empty and calls the
    /// surface a later milestone. Naming the operation is what this row is
    /// for; giving it fields would be designing undo here.
    WalkHistory { step: Undecided } => "Walk the edit history",

    // ----- Procedures ---------------------------------------------------

    /// Addressed by deck, layer and index. Read before writing.
    ReadProcedure { deck: u8, node: NodeAt } => "Read one node's source",

    /// Checked as you write it; built on a worker and swapped at a frame
    /// boundary. The source's own `kind` line must name the same layer as the
    /// address, which is a refusal and not a payload.
    WriteProcedure { deck: u8, node: NodeAt, source: String }
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
    WatchFiles { watching: Undecided } => "Edit the file instead",

    /// Landed, rolled back for costing too much, or failed to build. A write
    /// returning cleanly means it compiled, not that it is on screen.
    SwapOutcome => "Find out what a write did",

    /// **The operator's verdict on a candidate, and it is not the watchdog's.**
    ///
    /// A version reaches the screen because it compiled and then held the frame
    /// budget over thirty measured frames, which is
    /// `karakuri_engine::swap::Event::Accepted` and is a judgement about
    /// **cost**. Whether it is the one to keep is a judgement about **taste**
    /// and nothing in this instrument can take it. This row is where a person
    /// takes it.
    ///
    /// **The engine's three words are deliberately not reused.** *Accepted*,
    /// *rejected* and *rolled back* already name the budget's verdict on the
    /// same object, and the staging lane is the one surface where both
    /// verdicts are visible at once — a lane offering *accept* over a
    /// candidate the watchdog had already accepted would spell two different
    /// judgements the same way
    /// (`docs/principles/0031-a-name-means-one-thing-across-the-system.md`).
    ///
    /// **Addressed by the node rather than by the version, because a node has
    /// at most one unsettled version.** A write is not held anywhere: it is
    /// checked, written, built on a worker and swapped at a frame boundary, so
    /// the candidate for a node **is** what that node is playing. Choosing
    /// among several older versions is a different operation and it is
    /// [`Operation::WalkHistory`], which is still undecided for want of the
    /// address this row is able to do without.
    ///
    /// **Silent, and that is its shape rather than an omission.** The material
    /// already changed and `karakuri_store::record::Record::Procedure` was
    /// written where the swap landed. What this changes is the lane: the node
    /// stops being one with a version nobody has ruled on.
    KeepCandidate { deck: u8, node: NodeAt } => "Keep a candidate",

    /// **What *a rejected candidate costs nothing* is made of.** The version
    /// before it is a file under `<store>/history/`, kept because it
    /// **compiled** rather than because it landed — so the one thing a person
    /// most wants back, a real attempt that was rolled back for cost, is
    /// exactly the one that is there.
    ///
    /// **One step, and never a cursor.** Back to the version this one
    /// replaced. Walking further is [`Operation::WalkHistory`] and needs the
    /// address that row says nobody has settled.
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
    RestoreProcedure { deck: u8, node: NodeAt } => "Put a node's previous version back",

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

    /// The window is a preview and has no say in what is drawn: the canvas is
    /// fixed for the run, and the window fits it with the leftover black.
    ///
    /// The `a` key is a translation that asks for the canvas's own size.
    /// **`--canvas` is on this row's CLI badge and is not this operation** —
    /// see the report.
    SizeWindow { width: u32, height: u32 } => "Size the window",

    /// **Undecided because no output has an identity anywhere in this
    /// workspace.**
    ///
    /// The row describes one switchable list — the picture in the Program bay,
    /// a projector window, the plugin sinks — with the program view a row in
    /// it like any other. But `karakuri_engine::frame::Sink` is a trait with
    /// no name and no id; the projector window and the plugin sinks do not
    /// exist; and the console's Outputs row reaches its one sink by folding a
    /// *layout region*, so the only route that exists today is spelled as
    /// [`Operation::FoldBay`]'s target and not as an output at all. Giving
    /// this a payload means deciding what names an output, which is the row's
    /// whole content.
    RouteFrame { output: Undecided } => "Choose where the frame goes",

    /// The timeline as it happens, replayable frame for frame.
    RecordSession { recording: Recording } => "Record the session",

    /// Waits up to five seconds for a save still being written, then says how
    /// many it left behind rather than letting a hung disk hold the quit.
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
