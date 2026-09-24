//! The central Operation enumeration and titles table for Karakuri commands.

use crate::types::*;

/// Defines [`Operation`] and enforces single-source synchronization with specification headings.
macro_rules! operations {
    (
        $(
            $(#[$attr:meta])*
            $name:ident $( { $( $(#[$field_attr:meta])* $field:ident : $ty:ty ),+ $(,)? } )?
                => $title:literal,
        )+
    ) => {
        /// Every operation Karakuri can perform.
        ///
        /// Exactly one variant per `<h3>` in `docs/manual/operations.html`.
        #[derive(Debug, Clone, PartialEq)]
        pub enum Operation {
            $( $(#[$attr])* $name $( { $( $(#[$field_attr])* $field : $ty ),+ } )? , )+
        }

        impl Operation {
            /// The exact specification heading for this operation.
            pub fn title(&self) -> &'static str {
                match self {
                    $( Operation::$name { .. } => $title, )+
                }
            }

            /// All operation specification headings in manual order.
            pub const TITLES: &'static [&'static str] = &[ $( $title, )+ ];
        }
    };
}

operations! {
    // ----- Transport and tempo -----------------------------------------

    /// Three taps or more set the tempo; any tap sets the phase.
    TapBeat => "Tap the beat",

    /// Moves the tracker's octave window with it.
    ScaleGrid { by: GridScale } => "Halve or double the grid",

    /// Sets the signed latency offset in milliseconds between audio and video display.
    SetLatencyOffset { ms: f32 } => "Nudge the latency offset",

    /// Sets a deck's clock sync mode (Free, Tempo, Beat).
    SetSync { deck: u8, sync: Sync } => "Set a deck's sync mode",

    /// Scrubs a deck forward or backward by the given beat offset.
    ScrubDeck {
        deck: u8,
        /// How far, in beats.
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

    /// Enables or disables compositor blending across a deck's renderers.
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

    /// Fades a deck's opacity towards the target value over the current transition duration.
    FadeDeck {
        deck: u8,
        /// Target opacity value in [0.0, 1.0].
        to: f32,
    } => "Fade a deck out or in",

    /// Crossfades between two decks on the musical grid.
    Crossfade { from: u8, to: u8 } => "Crossfade to the next deck",

    /// Initiates a mask wipe transition from one deck to another (ADR-0192, P-0094).
    Wipe {
        /// The deck being covered.
        from: u8,
        /// The incoming deck arriving over it.
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

    /// Configures transition parameters (wipe shape, quantum, length) for upcoming fades.
    SetTransition { setting: TransitionSetting }
        => "Choose the wipe shape, the quantum, the length",

    /// Selects active renderer for a multi-renderer deck slot.
    SelectRenderer {
        deck: u8,
        /// Renderer index in draw order.
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
        /// Slot index counted from mix output (0 is first stage).
        at: u32,
        /// Parameter value to set.
        param: ChainParam,
    } => "Set a chain effect's parameter",

    /// Appends an L5 post-processing effect procedure to the master chain (ADR-0340, P-0091).
    AddChainEffect {
        /// Content address (e.g. `sha256:...`) of the procedure source.
        procedure: String,
        /// Frame retention buffer slot read by the procedure, if any.
        cut: Option<Cut>,
    } => "Add an effect to the master chain",

    /// Removes an effect slot from the master chain by position index (ADR-0352).
    RemoveChainEffect {
        /// Slot position index counted from mix output.
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
    // Sequencer pattern operations act on pattern banks and emit parameter/fader
    // operations on the musical grid (ADR-0222, ADR-0320).

    /// Sets a step state (on/off) in a sequencer pattern lane.
    ///
    /// Identifies the bank, lane, and 16-slot step index. Takes effect when
    /// the playhead reaches that step (ADR-0222, ADR-0320).
    SetStep { pattern: u8, lane: u8, step: u8, on: bool } => "Toggle a step",

    /// Mutes or unmutes a pattern lane (ADR-0222, ADR-0322).
    ///
    /// The pattern continues advancing its playhead but emits no writes while muted.
    SetLaneMute { pattern: u8, lane: u8, muted: bool } => "Mute a lane",

    /// Appends a new sequencer lane pointing to the target address (ADR-0321, ADR-0327).
    PointLane { pattern: u8, target: LaneTarget } => "Point a lane at what it drives",

    /// Removes a sequencer lane by index within the pattern bank (ADR-0322).
    RemoveLane { pattern: u8, lane: u8 } => "Remove a lane",

    /// Sets the step subdivision grid mode (sixteenth or eighth) for a pattern bank.
    ///
    /// The pattern spans one fixed bar, so step count directly follows the grid mode
    /// (ADR-0222, ADR-0306).
    SetPatternGrid { pattern: u8, grid: StepMode } => "Choose what a step is worth",

    /// Chooses which sequencer pattern bank (0..3) is active (ADR-0222, ADR-0320, ADR-0327).
    SelectPattern { pattern: u8 } => "Choose which pattern the sequencer plays",

    // ----- Inside a Set -------------------------------------------------

    /// Writes a scalar or vector parameter value on a deck's node.
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

    /// Wires a source procedure output into a named input slot on a node (ADR-0268).
    WireInput {
        deck: u8,
        /// Target node declaring the input slot.
        node: String,
        /// Input slot identifier on the target node.
        slot: InputPort,
        /// Source node supplying the input geometry or texture.
        to: String,
    } => "Wire a procedure's input to a node",

    /// Replaces the published control list for the deck's exposed interface.
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

    /// Reads parameter definitions, element counts, and emitted attributes for a Set.
    ReadSet { id: String } => "Read what one Set holds and declares",

    /// Send a Set to somebody, and take one in.
    TransferSet { transfer: SetTransfer } => "Send a Set to somebody, and take one in",

    /// Walks the edit history for a Set (ADR-0276, ADR-0304, ADR-0308, ADR-0342).
    WalkHistory { set: Option<String> } => "Walk the edit history",

    /// Loads a procedure into a layer of a deck's Set (ADR-0228, ADR-0304, ADR-0314).
    LoadProcedure { deck: u8, procedure: String } => "Load a procedure over a layer",

    // ----- Procedures ---------------------------------------------------

    /// Reads source code for a procedure node.
    ReadProcedure { deck: u8, node: NodeAddress } => "Read one node's source",

    /// Validates, compiles, and live-swaps source code for a node at a frame boundary.
    WriteProcedure { deck: u8, node: NodeAddress, source: String }
        => "Check and write one node's source",

    /// Observes external file system changes to update running procedures.
    WatchFiles { watching: Undecided } => "Edit the file instead",

    /// Queries the compilation or swap outcome of a recent procedure write (ADR-0316).
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

    /// Folds a named UI bay into collapsed state.
    FoldBay { bay: String } => "Fold a bay away",

    /// Folds an entire left or right UI pane.
    FoldPane { pane: String } => "Fold a pane away",

    /// Restores folded bays and panes to visible state.
    Unfold { region: Option<String> } => "Bring back what is folded",

    /// Solos a layout region by folding away surrounding regions, or restores prior layout if `None`.
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
