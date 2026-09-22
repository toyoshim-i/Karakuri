//! The central Operation enumeration and titles table for Karakuri commands.

use crate::types::*;

/// Define [`Operation`] and the list of headings it must match, from one
/// source, so the two cannot drift.
///
/// This is the whole reason there is a macro here. The test that gives this
/// crate its point compares a list of titles against the manual, and a
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
        /// Every operation Karakuri can perform, named once.
        ///
        /// One variant per `<h3>` of `docs/manual/operations.html`, in that page's
        /// order, carrying what the operation acts on. See the crate documentation for
        /// what that means and what it deliberately leaves out.
        #[derive(Debug, Clone, PartialEq)]
        pub enum Operation {
            $( $(#[$attr])* $name $( { $( $(#[$field_attr])* $field : $ty ),+ } )? , )+
        }

        impl Operation {
            /// The heading this operation is specified under, verbatim.
            ///
            /// Verbatim matters: the test compares these against the page's own `<h3>`
            /// text, so a title edited here to read better is a failing test rather than a
            /// silent divergence. The manual is the specification; a title that reads badly
            /// is fixed on the page first.
            pub fn title(&self) -> &'static str {
                match self {
                    $( Operation::$name { .. } => $title, )+
                }
            }

            /// Every heading, in the manual's order. What the test checks the page against,
            /// and what a surface enumerating the vocabulary reads.
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
    /// Absolute, though the heading says nudge. `--latency-offset-ms MS` is
    /// absolute, an absolute value can express every nudge and a nudge cannot
    /// express a setting, and a fader has to be able to reach it. `o` and `p` are
    /// two translations that read the current value and add five.
    SetLatencyOffset { ms: f32 } => "Nudge the latency offset",

    /// A mode the deck's material cannot honour is skipped with a reason, which is
    /// a refusal and not a payload.
    SetSync { deck: u8, sync: Sync } => "Set a deck's sync mode",

    /// The one relative operation here, and it is relative because nothing in this
    /// instrument can set a position.
    ///
    /// Scrubbing moves closed-form material by an amount; the manual is explicit
    /// that accumulating material cannot be moved to a position at all. An absolute
    /// `at_beat` would be inventing an operation that does not exist for two thirds
    /// of the material — see the report.
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
    /// A console pointer: it writes no record, and it is the reason every other
    /// variant names its deck instead of meaning *the selected one*.
    SelectDeck { deck: u8 } => "Select a deck",

    /// Sets the operational residency state (Live, Priming, or Allocated) of a deck slot.
    ///
    /// Live slots step and composite; Priming slots step out of sight without contributing
    /// to output; Allocated slots are paused in memory (ADR-0186).
    SetResidency { deck: u8, residency: Residency }
        => "Put a deck on air, prime it, or take it off",

    /// Loads a Set from the store into a running deck.
    ///
    /// Takes the target deck index and store Set identifier.
    LoadSet { deck: u8, set: String } => "Load material into a deck",

    /// Composite a deck's renderers rather than overdrawing them.
    ///
    /// A `bool` although `--merge N` can only turn it on, because the manual's gap
    /// section names un-compositing as one of the things reachable from nothing at
    /// all. Naming it is what makes it a gap rather than an absence.
    SetCompositing { deck: u8, compositing: bool } => "Composite a deck's renderers",

    // ----- Mixing -------------------------------------------------------

    /// The trim: the level material arrives at, colour only. Not clamped at 1.0 —
    /// the mix is HDR.
    SetGain { deck: u8, gain: f32 } => "Gain",

    /// The fader across the blend, and the only one of the two that touches what a
    /// layer covers.
    SetOpacity { deck: u8, opacity: f32 } => "Opacity",

    /// Mutes or unmutes a deck, excluding it from or including it in the composite mix.
    SetMute { deck: u8, mute: bool } => "Mute a deck",

    /// Solos or un-solos a deck, isolating it in the composite mix.
    SetSolo { deck: u8, solo: bool } => "Solo a deck",

    /// Clears any active solo across all decks.
    ClearSolo => "Clear solo",

    /// Sets whether a deck's slot is online in the composite mix.
    SetOnline { deck: u8, online: bool } => "Set a slot's online state",

    /// Names the mode, where `karakuri-midi`'s `Action::CycleBlend` could only
    /// step. A pad that means *over* is a mapping this made writable, and `note 40
    /// -> blend 0 over` is that mapping.
    SetBlendMode { deck: u8, blend: BlendMode } => "Blend mode",

    /// Starts at the current quantum and lasts the current length, both of which
    /// are [`Operation::SetTransition`]'s and not this one's — the manual is
    /// explicit that they are a console setting deciding what the *next* fade
    /// means.
    FadeDeck {
        deck: u8,
        /// Where the opacity ends up. Opacity only: a gain fade is in the record
        /// vocabulary and has no control.
        to: f32,
    } => "Fade a deck out or in",

    /// One gesture, four records: `to` is silenced and put on air at once, and the
    /// two fades land on the grid.
    ///
    /// Both decks named. *The next deck* is the keyboard's translation of this, not
    /// the operation.
    Crossfade { from: u8, to: u8 } => "Crossfade to the next deck",

    /// A mask at position 0 on the incoming deck, put on air under `over`, and one
    /// scheduled move carrying the front to 1. Refused with no shape chosen.
    ///
    /// The put-on-air and the `over` are written only where they change something,
    /// which is the one part of that sentence that is a condition rather than a
    /// record. A deck the operator has moved off the mode a deck starts in keeps
    /// the mode: a wipe under `add` or `max` is a wipe *on* rather than a wipe
    /// *over*, a different picture and one they may have chosen, and a gesture is
    /// not where an operator's choice is taken back
    /// (`docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`).
    /// A deck already live is not told so again. What decides it is a reading of
    /// where the deck already sits in the mix, which is
    /// `karakuri_operation_record::Current::mix`, so the condition is one sentence
    /// in one place rather than each surface's.
    Wipe {
        /// The deck being covered.
        from: u8,
        /// The deck arriving over it.
        to: u8,
    } => "Wipe the next deck in",

    /// Sets a deck's mask shape (linear front angle or radial) (ADR-0192, ADR-0201).
    SetMaskShape {
        deck: u8,
        kind: WipeKind,
        /// Which way a linear front runs, in radians.
        angle: f32,
    } => "Set a deck's mask shape",

    /// Sets how far a mask's front has travelled in [0.0, 1.0] (ADR-0192, P-0094).
    SetMaskPosition {
        deck: u8,
        /// Where the front is, `[0, 1]`.
        position: f32,
    } => "Set a deck's mask position",

    /// These change nothing you can see and write nothing to the stream. They
    /// decide what the *next* fade, crossfade or wipe means.
    SetTransition { setting: TransitionSetting }
        => "Choose the wipe shape, the quantum, the length",

    /// Only where the deck composites and holds two or more. One-way: no position
    /// in the cycle folds them all back in.
    SelectRenderer {
        deck: u8,
        /// Which renderer, in draw order — the numbering `--param L4:1:…` and a
        /// `select` record use.
        renderer: u32,
    } => "Choose which renderer of a deck is live",

    /// Sets master composite output gain before master post-processing (ADR-0224, P-0064).
    ///
    /// Acts on what the mix produced rather than an individual deck slot. Unbounded above 1.0.
    SetMasterOut { out: f32 } => "Master out",

    /// Sets parameter on a slot in the master chain (ADR-0340, P-0090).
    ///
    /// Addressed by slot position index between master out and exposure input.
    SetChainParam {
        /// Where the slot sits, counted from the mix's output: 0 is the slot the
        /// mix's frame is handed to.
        at: u32,
        /// What is set.
        param: ChainParam,
    } => "Set a chain effect's parameter",

    /// One `kind L5` procedure appended to the end of the master chain.
    ///
    /// Appends: the chain has no operation that inserts and none that reorders,
    /// so what a surface can say is *this procedure, at the end*.
    ///
    /// The slot arrives at the values its procedure declares, and it costs what
    /// its procedure costs from the frame it is added on
    /// (`docs/principles/0091-cost-is-known-before-it-is-paid.md`).
    AddChainEffect {
        /// The content address of the procedure's source, spelled the way a record
        /// spells one — `sha256:…`. An address nothing holds is refused where the
        /// chain is built, with the address in the message.
        procedure: String,
        /// Which retained frame the slot reads. `Some` exactly where the procedure
        /// declares `retains`; the two disagreeing is refused where the slot is
        /// built, rather than defaulted.
        cut: Option<Cut>,
    } => "Add an effect to the master chain",

    /// One slot taken out of the master chain, addressed by its position.
    ///
    /// [`Operation::SetChainParam`]'s address and its answer to a position the
    /// chain has not got. The slots after it move up, so a position names a
    /// different slot once one before it has gone.
    RemoveChainEffect {
        /// Where the slot sits, counted from the mix's output.
        at: u32,
    } => "Remove an effect from the master chain",

    /// The transfer from unbounded linear HDR to something displayable.
    ///
    /// Names the operator; it does not carry the level going into it, which is
    /// [`Operation::SetExposure`].
    SetTonemap { tonemap: Tonemap } => "Tone map",

    /// Sets exposure level for tone mapping (ADR-0180, ADR-0192).
    ///
    /// Applied after every layer has rendered before the present pass reads it.
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

    /// Sets a step state (on/off) in a sequencer pattern lane.
    ///
    /// Identifies the bank, lane, and 16-slot step index. Takes effect when
    /// the playhead reaches that step (ADR-0222, ADR-0320).
    SetStep { pattern: u8, lane: u8, step: u8, on: bool } => "Toggle a step",

    /// Mutes or unmutes a pattern lane (ADR-0222, ADR-0322).
    ///
    /// The pattern continues advancing its playhead but emits no writes while muted.
    SetLaneMute { pattern: u8, lane: u8, muted: bool } => "Mute a lane",

    /// The foot's `+ lane`, and the target is the whole of what is being added: a
    /// lane with nothing to drive emits nothing, so there is no moment at which a
    /// lane exists and its target does not.
    ///
    /// What a target may *be* is answered by the rest of this vocabulary — anything
    /// on it a lane can emit — which is why the console draws three deck faders and
    /// a Set parameter side by side and calls all four lanes. What a target is
    /// spelled as is [`LaneTarget`]: an operation of this vocabulary with its value
    /// left out, which reaches all four where a slot number reaches three and a
    /// node address reaches one
    /// (`docs/adr/0321-a-lanes-target-is-an-operation-with-its-value-elided.md`).
    /// It was [`Undecided`] until 2026-09-09 because *"a deck fader is a slot
    /// number and a Set parameter is a [`NodeAddress`] and a [`ParamAt`], and
    /// nothing here spells both"* — the spelling that reaches both is the one this
    /// vocabulary already uses to reach either.
    ///
    /// It appends, so there is no lane index. The panel draws no control for
    /// re-pointing a lane that already exists, so this row is where a target is
    /// chosen; if it turns out to be two operations it will be because a control
    /// was drawn for the second. Taking one away is [`Operation::RemoveLane`],
    /// which names the lane by index because by then there is a row to point at.
    ///
    /// The two levels are not here, and that is a decision taken when the chooser
    /// was drawn
    /// (`docs/adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md`).
    /// A lane carries an `on` and an `off`, filled in *at the press* from the range
    /// the surface was published, because a pattern outlives the Set it was written
    /// against — and the surface that appends the lane is where that reading
    /// already is, so carrying them here would put two floats in a payload only one
    /// caller could supply meaningfully. A map line and a model can name neither,
    /// which is ADR-0192's rule read the other way.
    PointLane { pattern: u8, target: LaneTarget } => "Point a lane at what it drives",

    /// The minus at the end of a lane's row, which takes that lane out of the
    /// pattern. Its steps go with it: there is nothing left to unmute onto, which
    /// is what separates it from [`Operation::SetLaneMute`].
    ///
    /// A bank and a lane, addressed the way [`Operation::SetStep`] and
    /// [`Operation::SetLaneMute`] are addressed and for their reason. **A lane
    /// index is a position and not an identity**: the lanes after the removed one
    /// move up, so every index above it names a different lane once this has been
    /// applied (`karakuri_pattern::Pattern::remove`). A slot of the master chain is
    /// addressed on the same terms
    /// (`docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md`),
    /// and a glyph on the row is where that record puts the control.
    ///
    /// A lane index the pattern does not hold is refused and the refusal says what
    /// there was to name
    /// (`docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md`).
    ///
    /// Removing the lane that drives a control a hand is also on is allowed and
    /// needs no refusal: a lane's writes are its whole record
    /// (`docs/adr/0322-the-sequencer-is-polled-like-a-transition-live-only-and-its-writes-are-its-record.md`),
    /// so they stop at the removal and the control keeps the value the last step
    /// wrote it.
    RemoveLane { pattern: u8, lane: u8 } => "Remove a lane",

    /// Sets the step subdivision grid mode (sixteenth or eighth) for a pattern bank.
    ///
    /// The pattern spans one fixed bar, so step count directly follows the grid mode
    /// (ADR-0222, ADR-0306).
    SetPatternGrid { pattern: u8, grid: StepMode } => "Choose what a step is worth",

    /// Chooses which sequencer pattern bank (0..3) is active (ADR-0222, ADR-0320, ADR-0327).
    SelectPattern { pattern: u8 } => "Choose which pattern the sequencer plays",

    // ----- Inside a Set -------------------------------------------------

    /// The sharpest gap: a model can rewrite a whole procedure and cannot turn one
    /// knob.
    WriteParam { deck: u8, param: ParamAt, value: ParamValue } => "Write a parameter",

    /// Attaches an audio or analysis signal bus to modulate a parameter.
    ///
    /// Maps the named signal via a curve onto the published parameter range
    /// (ADR-0222, ADR-0286, ADR-0319).
    AttachSignal {
        deck: u8,
        /// A [`BindAt`] and not a [`ParamAt`], because an attachment is one layer's —
        /// see [`BindAt`].
        param: BindAt,
        signal: String,
        curve: Curve,
        /// What the signal is mapped onto, low then high.
        range: [f32; 2],
    } => "Attach a signal to a parameter",

    /// Detaches an automated or modulated signal binding from a parameter (ADR-0286, ADR-0319).
    TakeParamBack { deck: u8, param: BindAt } => "Take a parameter back",

    /// The one operation addressed by name at both ends, which is `Record::Edge`'s
    /// decision and its reason: a position moves when the list is reordered, and
    /// reordering silently changing which geometry a morph blends towards is the
    /// exact failure that record exists to end.
    ///
    /// So the workspace really does spell a node address two ways, and the two are
    /// not a drift to be resolved — [`NodeAddress`] is positional because a param
    /// write wants a wildcard and a position is what a `.kir` gives, and this is by
    /// name because an edge must survive a reorder.
    WireInput {
        deck: u8,
        /// The node that declares the slot: `morph` in `--edge morph.far=…`.
        node: String,
        /// What that node's procedure calls it: `far`.
        slot: InputPort,
        /// The node bound to it: `sphere_shell`.
        to: String,
    } => "Wire a procedure's input to a node",

    /// What the console shows, and — since a knob binds to a position in it — what
    /// a MIDI control counts.
    ///
    /// The whole ordered list, not one entry. A MIDI control is bound to a position
    /// in the published interface, so adding one entry at a time would renumber
    /// every binding after it; and an interface that publishes nothing publishes
    /// everything, which is a statement about the list and not about an entry.
    Publish { deck: u8, controls: Vec<Control> } => "Narrow the published interface",

    /// Each comes from a Set file or a declared default.
    SetProperty { deck: u8, property: Property } => "Element capacity, seeds, the camera",

    /// Sets parameter arbitration authority for a node (ADR-0211, ADR-0223, P-0090).
    ///
    /// Addressed per node within a deck. Configures whether manual operator control,
    /// MIDI automation, or external agents may write to the node.
    SetAuthority {
        deck: u8,
        node: NodeAddress,
        authority: Authority,
    } => "Set a node's authority",

    /// Saves a node's procedure to the library under an identifier (ADR-0128, ADR-0261, ADR-0292, P-0096).
    KeepProcedure {
        deck: u8,
        node: NodeAddress,
        /// What to file it under, or a stamp — [`Operation::SaveSet`]'s field and its
        /// reason.
        id: Option<String>,
    } => "Keep a node's procedure",

    /// Configures an Inspector pane to display the given deck.
    ///
    /// Targets the pane identified by name without changing global deck selection
    /// (ADR-0305).
    PointPane { pane: String, deck: u8 } => "Point an Inspector pane at a deck",

    // ----- The library --------------------------------------------------

    /// Writes the material on screen — the versions running, with their parameters,
    /// capacities, salts, layering and fold — not what any file on disk says.
    SaveSet {
        deck: u8,
        /// What to file it under, or a stamp. A key press cannot type a name; a caller
        /// that can is not made to take a timestamp.
        id: Option<String>,
    } => "Keep what a deck is playing",

    /// Most recent first, narrowed by what a node is called or by which layer a Set
    /// uses, and it says how many it did not show.
    ListSets { holds: Option<String>, layer: Option<Layer> } => "List what the store holds",

    /// Filters library entries by layer kind mask (ADR-0262, ADR-0338).
    FilterLibrary { kinds: LibraryKinds } => "Filter the library by kind",

    /// Selects active library search scope (ADR-0228, P-0090).
    SelectScope { scope: Undecided } => "Choose which scope the library shows",

    /// Stars or unstars a Set in the library, toggling favourite status (ADR-0299).
    SetFavourite { id: String, favourite: bool } => "Star a Set, or take the star off",

    /// Every knob with its range and default, the element count, the attributes
    /// emitted — each read off the artifact's own card, so those three fetch no
    /// source and compile nothing.
    ///
    /// What a node's element storage comes to is the figure that is not a card's.
    /// It needs every source in the Set fetched and checked before anything can be
    /// sized, and it pays for that. The panel draws the three that cost nothing; a
    /// model asking over MCP is offered the fourth as well.
    ReadSet { id: String } => "Read what one Set holds and declares",

    /// Send a Set to somebody, and take one in.
    TransferSet { transfer: SetTransfer } => "Send a Set to somebody, and take one in",

    /// Walks the edit history for a Set (ADR-0276, ADR-0304, ADR-0308, ADR-0342).
    WalkHistory { set: Option<String> } => "Walk the edit history",

    /// Loads a procedure into a layer of a deck's Set (ADR-0228, ADR-0304, ADR-0314).
    LoadProcedure { deck: u8, procedure: String } => "Load a procedure over a layer",

    // ----- Procedures ---------------------------------------------------

    /// Addressed by deck, layer and index. Read before writing.
    ReadProcedure { deck: u8, node: NodeAddress } => "Read one node's source",

    /// Checked as you write it; built on a worker and swapped at a frame boundary.
    /// The source's own `kind` line must name the same layer as the address, which
    /// is a refusal and not a payload.
    WriteProcedure { deck: u8, node: NodeAddress, source: String }
        => "Check and write one node's source",

    /// This row names an event, not an operation, and that is the undecided part.
    ///
    /// Somebody editing a file in another program is not something a surface
    /// performs. The only operation nearby is *watch these files, or stop* — and
    /// `--watch` takes no argument, cannot be turned off, and does not say which
    /// files it would take if it could. Whether the row is that operation, or
    /// belongs on the page at all, is a decision about the manual.
    ///
    /// So this payload is the row's marker for one flag, and that is what it is
    /// for: `--watch` is the only thing in this program that turns the watching on,
    /// it takes no argument and nothing turns it off, so there is no state for a
    /// payload to name until somebody decides the row is an operation. It stays
    /// [`Undecided`] rather than becoming a `bool` nothing can set.
    ///
    /// A model has no route here, and nothing is owed for one. A model does not
    /// edit a file in another program: it calls `write_procedure`, which is its
    /// edit — checked, written and swapped at a frame boundary — so there is
    /// nothing a model would say on this row that the write does not already say.
    /// That is
    /// `docs/adr/0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md`'s
    /// kind of answer rather than a missing route, and it is why the page's MCP
    /// badge is `gap` and not `plan`
    /// (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
    WatchFiles { watching: Undecided } => "Edit the file instead",

    /// Landed, overloaded for costing too much, or failed to build. A write
    /// returning cleanly means it compiled, not that it is on screen.
    ///
    /// The middle word is a state and not an outcome, since ADR-0316: the version
    /// is in the slot and the slot has stopped updating, holding the frame it last
    /// drew, and nothing ends that on its own.
    SwapOutcome => "Find out what a write did",

    /// Promotes a staging lane candidate to a persistent library procedure (ADR-0228, ADR-0316, ADR-0326).
    KeepCandidate { deck: u8, node: NodeAddress } => "Keep a candidate",

    /// Restores a procedure to a previous revision from edit history (ADR-0192, ADR-0316, ADR-0326).
    RestoreProcedure { deck: u8, revision: Revision } => "Put a node's previous version back",

    // ----- Arranging the console ----------------------------------------

    /// Adjusts layout pane boundary divider position.
    ///
    /// Payload is currently undecided pending non-pointer coordinate specification
    /// (ADR-0259, ADR-0315, ADR-0342).
    MoveBoundary { boundary: Undecided } => "Move a boundary",

    /// A folded bay takes no space at all and no divider is drawn beside it; what
    /// it had goes to the bay that takes height back in that pane.
    ///
    /// Addressed by name, which is available: `karakuri_layout::Layout::find`
    /// resolves one, and `Layout::new` refuses an arrangement that uses a name
    /// twice, so *"the answer here is the only node that could be meant."*
    FoldBay { bay: String } => "Fold a bay away",

    /// The whole left or right pane, with everything in it, and the centre takes
    /// the width.
    ///
    /// Same payload as [`Operation::FoldBay`] and a separate row, because the
    /// manual describes two different consequences. Whether they are one operation
    /// over two kinds of region is a question for the page — see the report.
    FoldPane { pane: String } => "Fold a pane away",

    /// A folded region has no rectangle, so a pointer cannot reach it — the one
    /// operation on this page a mouse cannot be the only way into.
    ///
    /// `None` brings back everything folded, which is what reaches a fold nobody
    /// has a name for; a name brings back that region and whatever stands between
    /// it and the screen. That is `panel::Op`'s `Unfold` and `UnfoldAll`, two of
    /// its variants under one heading.
    Unfold { region: Option<String> } => "Bring back what is folded",

    /// Everything that is not this region, on the way to it, or inside it folds
    /// away, and it holds the window.
    ///
    /// `None` undoes the solo and restores what was folded before, including
    /// whatever was already folded. Explicit rather than a toggle: the caller says
    /// which way.
    Solo { region: Option<String> } => "Solo a region",

    /// Resets UI window arrangement to default layout (ADR-0175, ADR-0221).
    ResetArrangement => "Reset the arrangement",

    /// Saves current UI arrangement under a user-defined name (ADR-0221, P-0087).
    SaveArrangement { name: String } => "Save the arrangement",

    /// Restores a saved UI arrangement (ADR-0158, ADR-0221).
    RestoreArrangement { name: String } => "Put a saved arrangement back",

    // ----- Output and recording -----------------------------------------

    /// Resizes console application window (ADR-0220, ADR-0246, ADR-0247).
    SizeWindow { width: u32, height: u32 } => "Size the window",

    /// Routes rendered video output frame to a sink or display (ADR-0171, P-0090).
    RouteFrame { output: Output, on: bool } => "Choose where the frame goes",

    /// The timeline as it happens, replayable frame for frame.
    RecordSession { recording: Recording } => "Record the session",

    /// Gracefully exits the application after awaiting pending writes (ADR-0259, ADR-0332, P-0094).
    Quit => "Quit",
}

#[cfg(test)]
mod tests;
