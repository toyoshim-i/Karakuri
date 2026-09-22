//! Mixing, compositing, tone mapping, and transition parameters.

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
/// and the reason [`Residency`](super::deck::Residency), [`BlendMode`], [`WipeKind`] and the rest are
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

/// Which frame the master chain feedback pass reads back.
///
/// Mirrors `karakuri_engine::master::Cut`. See ADR-0317.
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
/// of the chain at a time through [`Operation::SetChainParam`](crate::op::Operation::SetChainParam) — and what holds
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
    /// Maximum feedback amount allowed (0.95) to prevent unbounded accumulation
    /// under [`Cut::Exit`]. Clamped in `karakuri_engine::master::Chain::clamped`.
    pub const MAX: f32 = 0.95;
}

/// What [`Operation::SetChainParam`](crate::op::Operation::SetChainParam) sets on one slot of the master chain.
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
    /// A list is not a cycle, on `Authority::ALL`'s and
    /// `karakuri_engine::deck::Blend::ALL`'s terms: the sensitivity row's curve
    /// chip is an affordance built over these four, the cycle belongs to whoever
    /// draws it, and what crosses this seam is [`Operation::AttachSignal`](crate::op::Operation::AttachSignal) naming a
    /// destination (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    pub const ALL: [Curve; 4] = [Curve::Lin, Curve::Pow2, Curve::Sqrt, Curve::Smooth];

    /// The lower-case word a record spells, which is what
    /// `karakuri_store::record::Record::Transition`'s `curve` carries and what a
    /// `bind` record has always carried.
    ///
    /// It arrived later than [`BlendMode::name`] and its neighbours because nothing
    /// needed it: the one operation carrying a curve — [`Operation::AttachSignal`](crate::op::Operation::AttachSignal)
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
    /// There is no `ALL` beside it, where [`BlendMode`] and [`super::deck::Residency`] have one.
    /// That constant exists so a map file can be offered the values a target may
    /// end in, and no map target names a shape — see the row at
    /// [`Operation::SetMaskShape`](crate::op::Operation::SetMaskShape). It arrives with the first reader.
    pub fn name(self) -> &'static str {
        match self {
            WipeKind::None => "none",
            WipeKind::Linear => "linear",
            WipeKind::Radial => "radial",
        }
    }
}

/// One of the three things [`Operation::SetTransition`](crate::op::Operation::SetTransition) can set.
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
