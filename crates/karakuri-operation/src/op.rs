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

    /// Three states, and one operation naming one of them. A slot is
    /// [`Residency::Live`] — stepped and composited — or [`Residency::Priming`],
    /// stepped out of sight and contributing nothing to the mix, or
    /// [`Residency::Allocated`], compiled and held with its `t` where it stopped.
    ///
    /// This was two `bool`s, and two `bool`s are four combinations for three states
    /// with both `false` destinations undefined: nothing in the vocabulary said
    /// where a deck taken off air or a prime request withdrawn landed. The answer
    /// existed, in `karakuri-cli`'s key handler and nowhere else, which is exactly
    /// the knowledge a vocabulary exists to take out of the surfaces. The engine's
    /// own setter had the shape all along — `Deck::set_residency(slot, Residency)`,
    /// one call naming one of three.
    ///
    /// Live is honoured and Priming is a request, which is the distinction the two
    /// rows carried between them and this one carries in its prose. The governor
    /// may hold a slot below what was asked for and never above, so Live lands and
    /// nothing demotes it, while a slot asked to prime with no room is *parked*:
    /// the request stands, is reconsidered on every pass, and takes effect the
    /// moment there is room, with nothing withdrawn and nothing remembered. A
    /// surface therefore draws the residency the deck reports rather than the one
    /// it asked for, and `park` is the existing name for the two disagreeing.
    SetResidency { deck: u8, residency: Residency }
        => "Put a deck on air, prime it, or take it off",

    /// The single largest gap in the manual's table: nothing loads a Set into a
    /// running deck.
    ///
    /// The id is a store id — what the library shows, what `save_set` comes back
    /// naming, what `--load-set` takes. `--set`'s file paths are the launch
    /// spelling of the same operation and are not carried: a path is how a Set is
    /// *authored*, and the library is what the panel drags from.
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

    /// Toggles the mute state of a deck.
    ToggleMute { deck: u8 } => "Toggle mute",

    /// Solos or un-solos a deck, isolating it in the composite mix.
    SetSolo { deck: u8, solo: bool } => "Solo a deck",

    /// Toggles the exclusive solo state of a deck.
    ToggleSolo { deck: u8 } => "Toggle solo",

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

    /// What shape of the frame a deck's layer reaches, and which way a linear front
    /// runs, in radians.
    ///
    /// Half of a mask, and the half that is a *choice*: `WipeKind::None` reveals
    /// everything at every position, and an angle is what makes one linear front a
    /// different picture from another. It says nothing about how far the front has
    /// travelled, which is [`Operation::SetMaskPosition`].
    ///
    /// Not the row `z` presses. That key sets the shape the *next* wipe takes,
    /// which is a console setting deciding what a later gesture means
    /// ([`TransitionSetting::WipeShape`]); this is the shape a deck's mask has now,
    /// and it names a deck because it changes one.
    ///
    /// `softness` is not here, on `white_point`'s terms at
    /// [`Operation::SetExposure`]: it is in `Record::Mask`, it has one constant
    /// behind it — `karakuri-cli`'s `MASK_SOFTNESS`, which says of itself that it
    /// is *"not a key"* — and no control on any surface. The conversion fills it in
    /// from the mask that is running.
    SetMaskShape {
        deck: u8,
        kind: WipeKind,
        /// Which way a linear front runs, in radians. The other two shapes ignore it,
        /// exactly as the mask does.
        angle: f32,
    } => "Set a deck's mask shape",

    /// How far a mask's front has travelled, `[0, 1]` — 0 reveals nothing anywhere
    /// and 1 reveals everything, both exactly.
    ///
    /// This is why the mask is two rows and not one. A single row carrying a shape,
    /// a position and a softness together could not satisfy this crate's own
    /// standing rule that *a continuous control is set, not nudged*:
    /// `karakuri-midi`'s grammar refuses the cross product in both directions — *"a
    /// note is a press, and this control takes a position"* and *"a control change
    /// is a position, and this control takes a press"* — so a row that was a sum
    /// would be a press, and no control change could ever reach a mask position.
    /// Splitting it is
    /// [ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)
    /// one layer down: an operation asks for what a surface can say, and
    /// `Record::Mask` stays whole.
    ///
    /// It cancels a move, as the gain and the fader do. It is the number a wipe's
    /// transition is writing, so a hand on it wins and whatever was moving it stops
    /// — which is the rule reaching the one control that had an exemption from it.
    /// `karakuri_engine::deck::Deck` is where that is written and
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// is the rule itself. [`Operation::SetMaskShape`] does not cancel, because it
    /// writes no position.
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

    /// One level on the composited frame, at the entry to the master chain — the
    /// whole fold rather than one deck of it.
    ///
    /// It names no deck, and that is the one thing to get right here. Every other
    /// row in this group carries `deck: u8` because it acts on one slot of the mix;
    /// this acts on what the mix *produced*, after every edge has been applied.
    /// `karakuri_engine::deck::Deck::set_out` says so at the setter — *"Not per
    /// slot"* — and it is also why nothing can schedule a move on it: a `Control`
    /// is per slot, so there is no transition for a hand here to cancel.
    ///
    /// It is not [`Operation::SetExposure`], and the difference is where each one
    /// multiplies rather than what either one means. This is applied where the mix
    /// writes the composited frame; the exposure is applied where the present pass
    /// reads it; feedback, bloom and rgb shift go between them. Until one of those
    /// exists there is nothing between the two multiplications and no frame tells
    /// them apart — that cost was weighed and taken in
    /// `docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md`,
    /// against the alternative of folding them into one number that would have to
    /// be pulled back out of the tone mapper the day the chain is not empty.
    ///
    /// Unbounded above 1.0 and floored at zero, which is [`Operation::SetGain`]'s
    /// range and the same function behind it: the pipeline is linear HDR and this
    /// level is applied to values a tone mapper has not seen yet
    /// (`docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md`).
    /// The clamp is the engine's, not this crate's — a conversion that clamped
    /// would be a second opinion about a range the setter already holds.
    SetMasterOut { out: f32 } => "Master out",

    /// One slot of the master chain set outright, addressed by where the slot
    /// sits in the chain.
    ///
    /// The chain is an ordered list of `kind L5` slots between
    /// [`Operation::SetMasterOut`]'s level at its entry and
    /// [`Operation::SetExposure`]'s at the tone mapper's input. A position and
    /// never a procedure: one procedure may hold more than one slot of a chain,
    /// so the address a surface can say is the position
    /// (`docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`).
    ///
    /// A state and never a step: [`ChainParam`] is where the slot is put, not how
    /// far it moves, so two surfaces holding this operation cannot disagree about
    /// where it is (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    ///
    /// A slot runs at every value it can hold, so an effect nobody wants is
    /// [`Operation::RemoveChainEffect`] and not an amount of zero.
    ///
    /// A position the chain has not got writes no record;
    /// `karakuri_operation_record::written` answers that the position is not in
    /// the chain it read.
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

    /// The level going into that transfer, set outright.
    ///
    /// Two operations where `karakuri_store::record::Record::Look` is one record,
    /// and the record is right to be one: a stream that set the exposure without
    /// saying which operator it applies to would be describing a look nobody can
    /// reconstruct. That reason is a reason about a record. A record is what a
    /// replay reconstructs a session from, so it must be complete on its own; an
    /// operation is what a surface *asks for*, and the place that turns one into
    /// the other already knows the look that is running — `karakuri-cli`'s
    /// `set_exposure` builds the record from `Look { exposure, ..self.look }`,
    /// filling the operator in from the current one, and has since before this
    /// crate existed.
    ///
    /// What forced the split: a control change turns exposure alone.
    /// `karakuri-midi` has no engine, no state and no readback by charter
    /// (`docs/adr/0180-…`), so `cc → exposure` — a route the manual marks as
    /// existing — could not become an operation at all while the only variant
    /// demanded an operator beside it. See `docs/adr/0192-…`.
    ///
    /// `white_point` is in that record, has no control on any surface and no row on
    /// the page, so it is not here — see the report.
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

    /// A step of one lane, on or off, heard the next time the playhead reaches that
    /// step rather than when it was asked for.
    ///
    /// `SetStep` and not `ToggleStep`, because there are no toggles in this
    /// vocabulary and the reason is at the top of this file: a toggle is an
    /// affordance built over two operations by whoever draws it, and a map with a
    /// button per direction has to be able to say *this step is on* and mean it.
    /// The manual's heading is the operator's word for the control and the title is
    /// copied from it verbatim, which is all a title is for.
    ///
    /// The address is a bank, a lane and a slot, which is the sentence this payload
    /// used to be [`Undecided`] for: it *"names a step and cannot yet say what it
    /// is a step of"*, and
    /// `docs/adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md`
    /// is what it is a step of. Every one of the three is named rather than
    /// implied, which is [`Operation::SelectDeck`]'s rule: implying the armed bank
    /// would be the shape that record refuses.
    ///
    /// `step` is one of sixteen stored slots and not a step index. A pattern holds
    /// sixteen either way and an eighth reads slot `2k` ([`StepMode::slot_of`]), so
    /// a surface in the finer reading sends the even ones. That keeps this payload
    /// independent of the mode, so a step press and a mode press cannot race into
    /// an address that means two things.
    ///
    /// What an on step is *worth* is not here, and that is the decision rather than
    /// an omission: a cell is a bit and the two levels are the lane's, because a
    /// level only means anything against what the lane drives.
    /// `karakuri_pattern::Lane` carries them.
    ///
    /// The grid under it is no new clock — a step is the beat clock subdivided and
    /// a pure function of `Oscillator::beats`, so correcting the tempo changes the
    /// rate from now on without moving a beat that has already happened (ADR-0222).
    SetStep { pattern: u8, lane: u8, step: u8, on: bool } => "Toggle a step",

    /// The pattern is kept and drives nothing, and the control is the lane's own
    /// label.
    ///
    /// It is not [`Operation::TakeParamBack`], and ADR-0222's consequence saying
    /// that it already is does not hold. That operation names a `deck` and a
    /// [`ParamAt`] — a parameter *inside that deck's Set* — where three of the four
    /// lanes the console draws are deck faders, which are no Set's. It is the same
    /// argument that record used to kill the binding reading in its own body,
    /// *"there is no binding on a deck fader anywhere in the engine"*, so it
    /// reaches one lane in four and an operation that reaches one lane in four is
    /// not this row. The mute is a lane's, addressed the way a lane is addressed.
    ///
    /// A bank, a lane and the state, addressed the way [`Operation::SetStep`] is
    /// addressed and for its reason. Not a toggle, also for its reason.
    ///
    /// It is where a hand takes a lane back. A hand's write lands at once and the
    /// lane writes again at the next step, so the way to keep what a hand did is to
    /// mute the lane — rule 02's *take back sits next to it*, drawn on the lane
    /// label rather than on the strip
    /// (`docs/adr/0322-the-sequencer-is-polled-like-a-transition-live-only-and-its-writes-are-its-record.md`).
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

    /// A mode with two values — a sixteenth or an eighth — drawn as one pill on the
    /// grid head. The pattern is one bar, fixed, so the step count is not a second
    /// thing a hand sets: it follows the mode, sixteen cells at a sixteenth and
    /// eight at an eighth, the row keeping its width so the cells halve in the
    /// finer one.
    ///
    /// This paragraph rationalised three pills until 2026-09-08 — *"sixteen steps
    /// of an eighth apiece is two bars, so any two of the three fix the third"* —
    /// which is arithmetic taken from the wrong two. The mock's ruler had drawn one
    /// bar of sixteenths since the same first commit, and with the length fixed at
    /// a bar neither a count nor a length has anything left to say. ADR-0306.
    ///
    /// The payload is a bank and a [`StepMode`], and it was [`Undecided`] until
    /// 2026-09-09 on two things that are both gone. The list was *"a list this
    /// crate has to own, on [`Curve`]'s terms, that nothing anywhere holds yet"*,
    /// and a two-valued mode is exactly that list; what was left was ADR-0192's
    /// rule — an operation asks for what a surface can say — and the console now
    /// draws the pill that says it. The bank is named rather than implied
    /// ([`Operation::SelectDeck`]'s rule), because the mode is what a *pattern* is
    /// rather than a preference the head holds.
    ///
    /// An eighth at 128 BPM is 234 ms, which is faster than the band
    /// `docs/adr/0255-three-clocks-run-at-once-and-a-slower-ones-work-never-lands-on-a-faster-one.md`
    /// writes the beat clock's rule for; a sixteenth is 117 ms, so the finer of the
    /// two modes is the worse case and neither that record nor ADR-0222 has it.
    /// ADR-0222 records the caveat rather than waving it away, and this is the row
    /// a hand would first feel it through, because it is the one that chooses the
    /// subdivision.
    SetPatternGrid { pattern: u8, grid: StepMode } => "Choose what a step is worth",

    /// The bay head's `seq 1 · seq 2 · +`: which pattern the lanes are reading. The
    /// `+` is this same choice landing on an empty one rather than a second
    /// operation — the arrangement pill is the same shape, and it is why
    /// [`Operation::ResetArrangement`] is the special case of putting a saved one
    /// back rather than a control of its own.
    ///
    /// A bank index, and it is not a name. A bank is a position in the session —
    /// four of them, fixed — and a name is what a save files a pattern under, as
    /// different as a deck slot and a Set's id
    /// (`docs/adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md`).
    /// That is what lets the `+` be this choice landing on an empty bank: an empty
    /// bank has no name, and asking for one would make the `+` a dialog. It was
    /// [`Undecided`] until 2026-09-09 because *"a pattern has no identity
    /// anywhere"* — a bank is the identity a *session* gives one, and ADR-0227's
    /// name is the one a *store* gives it.
    ///
    /// The console draws four pills and no `+`, which is the paragraph above
    /// carried to its end rather than a departure from it: once the count is fixed
    /// at four every bank has a pill, so *this choice landing on an empty one* is a
    /// press on `seq 3`, and a `+` beside it would be a second door to a press
    /// already on the head
    /// (`docs/adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md`).
    /// A press names a bank and never a direction, so asking for the one already
    /// armed is allowed and moves nothing — the cell's own rule one control down.
    ///
    /// Keeping a pattern and putting a saved one back are not rows on the page, so
    /// they are not variants here either: the console draws no control that saves
    /// one. They will arrive the way [`Operation::SaveArrangement`] and
    /// [`Operation::RestoreArrangement`] did — specified on the page, drawn on the
    /// console, built after that, and they are the rows that introduce the name.
    SelectPattern { pattern: u8 } => "Choose which pattern the sequencer plays",

    // ----- Inside a Set -------------------------------------------------

    /// The sharpest gap: a model can rewrite a whole procedure and cannot turn one
    /// knob.
    WriteParam { deck: u8, param: ParamAt, value: ParamValue } => "Write a parameter",

    /// A source, a curve and a range — which is the whole of what "how hard it
    /// reacts" means.
    ///
    /// `signal` is a name on the bus (`energy`, `beat`, `band3`, `noise`) and is a
    /// `String` rather than a list, because that bus is open by design:
    /// `docs/principles/0090-a-surface-offers-it-never-decides.md`.
    ///
    /// A step sequencer is not one more name on that bus, and this documentation
    /// said it was planned as one. ADR-0222 surveyed the bay before drawing it and
    /// found both halves of that plan false: the bus is stateless by construction —
    /// every value on it is a pure function of the local oscillator's `t` and
    /// `bpm`, and *"nothing seeded lives here"* — where a pattern is authored
    /// state; and it is keyed by name alone with one `Signals` per session, so two
    /// lanes sourced from `seq 1` with different targets would sample the same name
    /// in the same frame and get the same value, which is not a sequencer. A lane
    /// is a fifth route into this vocabulary, emitting operations on the beat the
    /// way the other four surfaces do, which is what the sequencer's five rows
    /// above are and why none of them is a binding
    /// (`docs/adr/0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md`).
    ///
    /// The noise generator's own parameters, which `--bind` also takes, describe
    /// the *source* rather than the attachment and are not carried here.
    ///
    /// There is no confidence here and there is nowhere for one to go. A value
    /// arrives with how well it is known and the blend is `lerp(the param's own
    /// value, the mapped signal, confidence)`, so a confidence an operator could
    /// write would be a caller telling the system how much to trust a measurement
    /// it took —
    /// `docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md`
    /// exactly inverted. It comes off the sample and off nothing else.
    ///
    /// `range` is the range the control was *published* over. A surface sending
    /// this has it in hand — it is what the fader on the same row is drawn against
    /// — and it is not a second thing for an operator to choose: `ParamValue` says
    /// a range *"is the procedure's declaration and not an operator's to write"*,
    /// and a published range narrows it without redefining it. So the field states
    /// which of a parameter's declared span the signal is mapped onto, and the
    /// answer a console gives is *all of what it published* (ADR-0286, ADR-0319).
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

    /// Take a knob back from whatever is driving it. It is the second rule's other
    /// half — *you can always see who is holding a control, and always take it
    /// back*.
    ///
    /// It removes the attachment rather than suspending it, and that is a decision
    /// rather than an economy. This documentation said *"without losing the
    /// binding"* until 2026-09-09 and nothing anywhere could have done that:
    /// `karakuri_engine::binding::Binding` carries no suspended state, and a fourth
    /// thing for an attachment to be — attached, absent, suspended, and blended at
    /// a low confidence — would have to be drawn, recorded and restated on every
    /// rebuild, where *not driving this parameter* is already written and is the
    /// absence. What is given up is *hand it back* in one press; what replaces it
    /// is that the session stream carries the source, the curve and the range on
    /// the record that attached it, so re-attaching is a thing a stream can say.
    /// See
    /// `docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`.
    ///
    /// A hand on the value is not this, and the two are deliberately different
    /// presses. Writing a bound parameter with [`Operation::WriteParam`] moves the
    /// value a binding blends *from* and leaves the attachment where it is —
    /// order-independent by construction, which is what a blend on confidence buys
    /// — so nothing an operator does to a knob can detach a signal by accident.
    /// This operation is the only thing that detaches one.
    ///
    /// And it is not the sequencer's lane mute, which ADR-0222's consequences say
    /// it already is: this names a parameter inside one deck's Set and three of the
    /// four lanes the console draws are deck faders, which are no Set's.
    /// [`Operation::SetLaneMute`] is that row, and carries the argument.
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

    /// Who may move one node, which is rule 06 of the manual and one of the four
    /// properties this system is defined by.
    ///
    /// Two shapes at once, and both are already here. It names one of three, which
    /// is [`Operation::SetResidency`]'s shape and [`Operation::SetBlendMode`]'s —
    /// the vocabulary owns the value list, so a surface asks for a destination
    /// rather than for a step
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). And it
    /// addresses a node of a deck's Set, which is [`Operation::WriteProcedure`]'s
    /// shape: a `deck` beside a [`NodeAddress`], which is *"one address shape for
    /// within a Set and one for which Set"*.
    ///
    /// Per node rather than per deck slot, and the manual rules the slot out in the
    /// row above it: *"There is no switch that hands the whole instrument to an
    /// agent, because the useful arrangement is almost always partial"*. A flag on
    /// a `deck: u8` is that switch at deck granularity. Per *layer* is not
    /// addressable at all — no operation here names a layer of a live Set, and
    /// [`Operation::ListSets`]'s `layer` narrows a search of the store rather than
    /// reaching one. See
    /// `docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`.
    ///
    /// [`Layer::Field`] takes one and the merge cannot. A field addresses no node
    /// in the rendering sense and its params are still declared, addressable and an
    /// operator's to ride, which is the whole reason that arm is in [`Layer`] — so
    /// it is a node an agent can be let at. The L5 that folds a Set's renderers is
    /// a node too — `docs/ir-spec.md` says *"`crate::node::Merge` is the node"* —
    /// and it is in neither spelling of [`Layer`], because a `kind` says what a
    /// procedure lowers to and compositing has none. So this operation cannot
    /// address it, and nothing on it can be moved by anybody today either:
    /// `Record::Merge` carries no `gain`, `opacity`, `blend` or `mask` *"because a
    /// record whose producer does not exist waits for it"*. Reaching it means
    /// [`Layer`] growing an arm, which is a change to what a `.kir` may declare and
    /// not a question about authority.
    ///
    /// # What it does today, said rather than implied
    ///
    /// Nothing writes a parameter on an agent's behalf in this workspace, so no
    /// addressed write is refused *because of* a level. The record is kept so that
    /// one can be when something does, which is M6's, and the level is not
    /// decoration in the meantime: a bare-name write over nodes that are not all
    /// under one authority is refused whole
    /// (`docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`),
    /// so granting one renderer and keeping another narrows what one knob may do
    /// from the next press. A surface drawing this must not word it as more than
    /// that.
    SetAuthority {
        deck: u8,
        node: NodeAddress,
        authority: Authority,
    } => "Set a node's authority",

    /// Writes one node's source into the operator's own library, at
    /// `<store>/procedures/<name>.kir`, so that it can be loaded over a layer of
    /// something else afterwards ([`Operation::LoadProcedure`]).
    ///
    /// It is the act that makes that tier exist
    /// ([P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)):
    /// nothing else in this program writes there, and the procedures that ship
    /// under the presets root are never written by anything. The content-addressed
    /// sources every build leaves in the store are not this — those are the edit
    /// history's, one per compile and named by a hash, and `Operation::WalkHistory`
    /// is the surface over them.
    ///
    /// `id` is [`Operation::SaveSet`]'s field one level down, and it is the same
    /// pair of presses: the capsule on a node group's head types nothing and takes
    /// a stamp, and a name typed into the Inspector's pane head is what a keep from
    /// there files under
    /// (`docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md`,
    /// `docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md`).
    ///
    /// A model asked for this writes `<store>/sandbox/`, stamped and overwriting
    /// nothing, which is why a model is not refused here where its star is: what it
    /// saves is a file, so it has a sandbox form to land in
    /// (`docs/adr/0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md`,
    /// `docs/adr/0301-a-models-star-is-refused-because-a-favourite-has-no-sandbox-to-land-in.md`).
    ///
    /// The node is one node. A head standing over more than one carries no capsule,
    /// which is [`Operation::SetAuthority`]'s own rule on the same head: one
    /// control there would be one of several answers drawn as the answer.
    KeepProcedure {
        deck: u8,
        node: NodeAddress,
        /// What to file it under, or a stamp — [`Operation::SaveSet`]'s field and its
        /// reason.
        id: Option<String>,
    } => "Keep a node's procedure",

    /// Which deck a pane of the Inspector is showing. A pulldown on the pane's own
    /// head over the decks the mixer is drawing strips for, which is
    /// `View::select`'s refusal read again rather than a rule of its own.
    ///
    /// It is not [`Operation::SelectDeck`], and the difference is the same one the
    /// Library bay's load pulldown makes
    /// (`docs/adr/0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md`):
    /// that operation moves where the keys are addressed, and this mark exists so
    /// that a pane can show a deck the keys are not on. A pick moves no selection,
    /// no other pane and no load target.
    ///
    /// A pulldown rather than a flip, which is the maintainer's choice and
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// underneath it: a flip is a step, two panes stepping cannot both be aimed
    /// without knowing where they started, and a key, a map line or a model would
    /// have to count presses to say *deck C*. This names the deck.
    ///
    /// `pane` is a `String`, which is [`Operation::FoldPane`]'s spelling and for
    /// its reason: this crate has no dependencies and cannot hold the arrangement's
    /// handle type, so a pane is named by the name the arrangement gives it.
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

    /// Which kinds of row the library listing shows — the five procedure kinds and
    /// Sets, OR across the ones that are on, everything where none is. See
    /// [`LibraryKinds`], which is the whole of the payload.
    ///
    /// It is not [`Operation::ListSets`]'s `layer`, and the two are two facts. That
    /// field asks *which Sets hold a node on this layer* — a predicate over a Set's
    /// contents, over Sets alone — and a button here asks *is this procedure of
    /// this kind*, which is a predicate over one artifact. Folding them into one
    /// field would be a name meaning two things (`docs/contributing.md` §4), so
    /// `ListSets` keeps its field and this operation carries six states beside it.
    /// That is also why the panel's `layer` field is superseded rather than
    /// extended
    /// (`docs/adr/0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md`,
    /// `docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md`).
    ///
    /// `holds` is untouched and stays on `ListSets`: it narrows by node name and is
    /// a filter over what a listing holds, where this decides which populations the
    /// listing is drawn from at all.
    FilterLibrary { kinds: LibraryKinds } => "Filter the library by kind",

    /// Which library is being read, and the four chips the console draws are four
    /// questions rather than four acts — `docs/manual/operations.html` argues that
    /// at the row, and the console page argues the sharpest part of it:
    /// *favourites* is *"this library filtered rather than a fifth place a Set can
    /// be"*, so choosing it and choosing *my sets* differ in the question asked and
    /// not in what is asked.
    ///
    /// [`Undecided`], and the row itself says why. The scopes are *"the one thing
    /// about the library that is not closed: it grows when a directory is added"* —
    /// so an enum of the four here would assert that the list can be finished,
    /// which is the claim that row exists to refuse, and it would go short the
    /// moment an operator points the bay at a directory. What identifies one member
    /// of a growable list is spelled nowhere: not on that page, not on the console
    /// page, and not in this workspace. A folder scope has a path, *presets* has a
    /// root the program was told, and the other two have neither.
    ///
    /// The key steps and this does not. `e` moves to the next scope and wraps, and
    /// that is the translator's arithmetic rather than this operation's payload —
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md).
    /// Whatever a scope turns out to be named by, this names one.
    SelectScope { scope: Undecided } => "Choose which scope the library shows",

    /// The star on a library row, and the fact it is a control over. `id` is the
    /// Set the store holds; `favourite` is the state it is being put in.
    ///
    /// `my sets` is what this fills. It is not the listing of what `<store>/sets/`
    /// holds — that is [`ListSets`](Operation::ListSets) — but the starred subset
    /// of it, so a preset packaged on load and a recording's head land in the
    /// library without appearing there until somebody presses the star
    /// (`docs/adr/0299-my-sets-is-the-starred-subset-and-the-star-is-kept-beside-the-sets.md`).
    ///
    /// Not a toggle, on this crate's general rule: a map with a button per
    /// direction, a model that says which one it wants and a key all have to be
    /// able to say *star this* and mean it. `favourite` is the state, the way
    /// [`SetResidency`](Operation::SetResidency) names one of three.
    ///
    /// The fact is a favourite and the control is a star, which is
    /// `docs/manual/console.html`'s pair and is why this is spelled two ways. That
    /// page also decides where the value lives — beside the Sets, in the store,
    /// rather than in the Set file or in the session stream — and
    /// `karakuri-operation-record` answers `Silent(Surface)` for the second half of
    /// that sentence.
    ///
    /// MIDI cannot reach it and that is a `gap` rather than a plan. Every target a
    /// map line can name carries a slot, a range or a word from a closed list, and
    /// a Set id is none of the three — the same sentence the `read` and `load`
    /// pills beside this control already carry.
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

    /// The listing, and the Library bay draws it: a fifth scope chip whose rows are
    /// the versions of one Set, most recent first, off
    /// `karakuri_environment::history::list`.
    ///
    /// Landing on a row is not this operation. It is
    /// [`Operation::RestoreProcedure`], which carries a [`Revision`] now that a
    /// surface can name one. This row is the walk, and the page says so: *what
    /// versions has this had* is a list, landing on one is a load, and neither word
    /// is *undo*.
    ///
    /// What a walk carries is *which history*, and it is a Set id since 2026-09-10.
    /// The three shapes this was choosing between were answered by
    /// [ADR-0308](../../../docs/adr/0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md)
    /// and the fourth was left open for want of a surface: a cursor and a direction
    /// is the console's own mark, which has no row on the page at all; a count of
    /// steps is something no surface offers; a revision to land on is the
    /// landing's; and *which history* was *"the one thing no surface spells"*. A
    /// model spells one: `read_set` takes a Set id and `list_sets` hands the ids
    /// out, so the sentence that kept this [`Undecided`] had stopped being true
    /// about the surfaces this vocabulary serves
    /// (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
    ///
    /// A walk is narrowed by a Set and not by a deck. Two decks running one Set
    /// have one history between them, and a version is filed under the Set the slot
    /// was running
    /// (`docs/adr/0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md`),
    /// so a deck is how a console *arrives* at an id and never what the listing is
    /// about — which is why the field is the id and not the deck the panel could
    /// have said.
    ///
    /// `None` is a Set the walk does not name, and it is a state rather than an
    /// absence. The deck a panel aims this at can be running the pair the run
    /// launched with; those versions are filed under no Set, and a narrowing to a
    /// Set matches none of them rather than all of them
    /// (`docs/adr/0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md`),
    /// so a walk that names no Set lists nothing and the surface says why. It is
    /// `karakuri_environment::history::Version::set`'s own `Option` read from the
    /// asking side, and the one derivation of it on the panel is the aim
    /// (`Aiming::at`).
    ///
    /// A model names one and is not offered the `None`: `walk_history` requires
    /// `set`, because a walk of no Set is not a question anybody can be answered.
    WalkHistory { set: Option<String> } => "Walk the edit history",

    /// One layer of what a deck is playing, replaced, and everything else left
    /// where it is. A procedure declares one `kind`, and the press re-points the
    /// slot with that one file swapped for the one that was there — every other
    /// field of the aim restated, so the layering, the fold, the capacities, the
    /// salts, the camera and the wiring come back as the slot's own
    /// (`docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md`).
    /// Nothing is installed
    /// (`docs/adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md`).
    ///
    /// `procedure` names a row of the library, in either tier: one the operator
    /// kept under `<store>/procedures/`, or one that ships under the presets root.
    /// It is a name and never a path — the same rule [`Operation::LoadSet`]'s `set`
    /// is under.
    ///
    /// No node address, and the limit is recorded rather than designed around. It
    /// lands on the first node of that kind, so `L4:0` is the renderer a `kind L4`
    /// replaces and the second renderer of a three-renderer Set is unreachable from
    /// this operation. A library row cannot say an index, and a field only one
    /// surface could ever fill would be a payload for a control nobody has drawn;
    /// the day the Inspector's node head grows a *replace this node* control is the
    /// day this gains a [`NodeAddress`]. Where the deck has no node of that kind
    /// the procedure is added as node 0 of it, which is the case the row is for: a
    /// Set declaring no camera holds the built-in orbit at `L3:0`.
    ///
    /// What the slot runs afterwards is a derived Set with no name, and the
    /// versions it writes stay filed under the Set it started from — `watch::Aim`'s
    /// `set` is not moved by this operation, where [`Operation::LoadSet`] replaces
    /// it
    /// (`docs/adr/0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md`).
    /// So the `history` walk goes on listing that deck's versions and the snapshot
    /// every compile takes stays alive. Nothing is saved on the press;
    /// [`Operation::SaveSet`] is what gives the result a name.
    ///
    /// A separate operation from [`Operation::LoadSet`] rather than a second arm of
    /// it. That one names a Set the library holds and restates every layer; this
    /// names a procedure and restates all but one; and only one of the two leaves
    /// the slot running material with no name. What they share is the class and the
    /// timing.
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

    /// The operator's verdict on a candidate, and it is not the watchdog's.
    ///
    /// A version reaches the screen because it compiled, and it goes on *running*
    /// because one frame of it was measured under the frame budget —
    /// `karakuri_engine::swap::Event::Accepted`, which is a judgement about cost.
    /// Whether it is the one to keep is a judgement about taste and nothing in this
    /// instrument can take it. This row is where a person takes it.
    ///
    /// The engine's three words are deliberately not reused. *Accepted*, *rejected*
    /// and *overloaded* already name the budget's verdict on the same object, and
    /// the staging lane is the one surface where both verdicts are visible at once
    /// — a lane offering *accept* over a candidate the watchdog had already
    /// accepted would spell two different judgements the same way
    /// (`docs/contributing.md` §4).
    ///
    /// And it is not offered on an overloaded row at all. That row is a slot that
    /// has stopped, which is not a candidate a person is choosing between; keeping
    /// it would settle the one row whose whole job is to say the slot is not
    /// running (ADR-0316).
    ///
    /// Addressed by the node rather than by the version, because a node has at most
    /// one unsettled version. A write is not held anywhere: it is checked, written,
    /// built on a worker and swapped at a frame boundary, so the candidate for a
    /// node is what that node is playing. Choosing among several older versions is
    /// a different operation and it is [`Operation::WalkHistory`], which names the
    /// Set whose versions are being chosen among rather than the node this row
    /// addresses — a walk is narrowed by a Set and a keep is settled at a node.
    ///
    /// And the surface that says a node is the staging lane, since 2026-09-09. A
    /// lane row is one node a build changed rather than one slot — a save touching
    /// two files draws two rows, each carrying its own address, with the build's
    /// one verdict written on both
    /// (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
    /// The control is the row itself, and the smaller box inside it is the capsule
    /// that asks for [`Operation::RestoreProcedure`]: the free act takes the large
    /// target and the act that writes a file takes the small one. A row that names
    /// no node offers neither — a build that failed, a source the checker turned
    /// down, and a rebuild that changed nothing are verdicts about a slot, and
    /// there is nothing for a keep to settle.
    ///
    /// Silent, and that is its shape rather than an omission. The material already
    /// changed and `karakuri_store::record::Record::Procedure` was written where
    /// the swap landed. What this changes is the lane: the node stops being one
    /// with a version nobody has ruled on.
    KeepCandidate { deck: u8, node: NodeAddress } => "Keep a candidate",

    /// What *a rejected candidate costs nothing* is made of. The version before it
    /// is a file under `<store>/history/`, kept because it compiled rather than
    /// because it landed — so the one thing a person most wants back, the version
    /// before the one that stopped their slot, is exactly the one that is there. It
    /// is also the way out of a stopped slot: nothing puts a version back on its
    /// own since ADR-0316, so landing an earlier one from here is one of the three
    /// things that ends a freeze.
    ///
    /// Which version is a [`Revision`], because two surfaces can ask and each says
    /// a different half. A staging lane row names the node and means the version
    /// its source replaced — one step, never a cursor. A row of the Library bay's
    /// `history` scope names the version itself, and that name carries the node
    /// with it. Walking the versions is [`Operation::WalkHistory`], which is the
    /// listing this picks out of.
    ///
    /// Both arms are asked by the panel since 2026-09-09, and neither resolves a
    /// file: a performer turns the arm it was given into a version and writes that
    /// version's bytes over the node's working copy. What [`Revision::Previous`]
    /// resolves to is the history walked for that node of that Set, most recent
    /// first, with the entry *after* the newest taken — the newest is the version
    /// the slot is running, whether it is stepping or stopped, because the history
    /// is gated on compiling and not on landing. A node whose only version is the
    /// one it is playing is refused in a sentence that says so
    /// (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
    ///
    /// Not [`Operation::WriteProcedure`] carrying that file's text, and the reason
    /// is the surface. A write takes a `source: String` because whoever asks for
    /// one is holding the text: a model has it in the conversation, an editor has
    /// it in a buffer. A staging lane holds neither. It can say *not this one* and
    /// it cannot say four kilobytes of IR, and making it say them would put the
    /// store inside the console — the same reach the Library bay already declines
    /// for a date format
    /// (`docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md`).
    /// So the surface says the part it can say and whoever performs it reads the
    /// file, which is
    /// `docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md`.
    ///
    /// This does write a record, where [`Operation::KeepCandidate`] does not: it is
    /// a procedure change, it lands at a swap like any other, and a session in
    /// which the operator put a version back and that replayed with the version
    /// they threw away is the hole `Record::Procedure` was added to close.
    RestoreProcedure { deck: u8, revision: Revision } => "Put a node's previous version back",

    // ----- Arranging the console ----------------------------------------

    /// Undecided, and the manual says so: *"How a pane is sized and unfolded
    /// without a mouse is not decided."*
    ///
    /// Two things are missing, not one. A boundary is `(split, index)`, and
    /// `karakuri_layout::Layout::name` answers `None` for *"a split the arrangement
    /// left unnamed"* — so more than half the boundaries in the console's
    /// arrangement have no address any surface but the pointer could say. And
    /// `Layout::set_divider` takes a position in the viewport's own coordinates,
    /// which is a pixel: a number a drag produces and a key press or a model has no
    /// way to mean.
    ///
    /// The payload stays [`Undecided`] for the keyboard's reason, which is the
    /// second of the two above read at a key:
    /// `docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md`
    /// is where a key that can only *step* is refused, and a viewport pixel is not
    /// something a press can mean. Settling the address half alone would not settle
    /// it.
    ///
    /// And a model has no window, so the page's MCP badge is `gap`. A divider's
    /// position is the arrangement's own state, which is what the twelve rows of
    /// `docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md`
    /// are — this was the thirteenth, left `plan` only because its payload is open,
    /// and that record's own consequences say so. The two facts are held apart: the
    /// badge is `gap` because a route into a surface's own state is a route into a
    /// window the model is not looking at, and the payload is open because no
    /// surface but the pointer can say a boundary. Settling one would not settle
    /// the other
    /// (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
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

    /// The default arrangement, at the viewport the window already has — and it
    /// takes nothing, because it acts on the arrangement as a whole. ADR-0175 put
    /// it in that group with `UnfoldAll` and `Unsolo` and it has stayed there.
    ///
    /// What it discards is the rest of this section's promise. *Move a boundary*
    /// says a window dragged too small *"gives everything less and forgets
    /// nothing"*, and *Fold a bay away* says a folded bay's *"size is remembered,
    /// so bringing it back puts it where it was"*. Both of those are kept in the
    /// arrangement this replaces: `karakuri_console::layout()` is built fresh and
    /// only the viewport survives, so every fold, every divider a hand has moved,
    /// the solo and both sets of remembered sizes go at once. It is the one
    /// operation on the page that forgets, and the row says so.
    ///
    /// It is the default member of a family that now exists, which is the framing
    /// this row was written to rather than *start again*: an arrangement is named,
    /// kept and put back, and resetting is putting back the one that came with the
    /// program. [`SaveArrangement`](Operation::SaveArrangement) and
    /// [`RestoreArrangement`](Operation::RestoreArrangement) are the other two
    /// members, and the page now says where all three live on the console.
    ///
    /// No payload, and that is decided rather than [`Undecided`] — and it stayed
    /// decided when the family landed. The default arrangement is not a file and
    /// there is no reserved name for it, so this variant reaches code where
    /// `RestoreArrangement` reaches a file, and the two never meet (ADR-0221 §2). A
    /// name here would make *the default* one entry of a listing an operator can
    /// overwrite.
    ResetArrangement => "Reset the arrangement",

    /// File the running arrangement under a name the operator picked, overwriting
    /// whatever is already kept under it.
    ///
    /// The name is a `String` and not an `Option<String>`, which is where this
    /// parts company with [`Operation::SaveSet`] beside it: a Set is *ordinarily*
    /// filed under a stamp nobody chose and an arrangement is not, because the
    /// whole of what a name is for here is that the operator will look for it again
    /// (`docs/principles/0087-name-the-property-never-the-shape.md`). A surface
    /// with nobody there to type one passes
    /// `karakuri_environment::history::stamped_id`, the same stamp `accepted_save`
    /// reaches for — so the fallback is the *caller's* and this payload never has
    /// to say *no name*
    /// (`docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md`).
    ///
    /// One path component, letters, digits, `-` and `_`, which is a Set id's rule
    /// and is the caller's to keep — `karakuri-environment`'s `mcp::checked_id` is
    /// where a name reached from a protocol is refused rather than sanitised.
    ///
    /// What it writes is a file and not a record, and those are two different
    /// things: `karakuri-operation-record` answers `Silent(Surface)` here, because
    /// a saved arrangement lives in a fourth place under the store rather than in
    /// the session stream, and a replay reconstructs nothing from one. See that
    /// crate's arm for why this is not `Silent::OnLanding`.
    SaveArrangement { name: String } => "Save the arrangement",

    /// The arrangement filed under `name`, at the viewport the window already has —
    /// which is [`Operation::ResetArrangement`]'s sentence with a name in it, and
    /// that is the whole relationship between the two.
    ///
    /// Two refusals, and both were written before this row was. Nothing filed under
    /// the name is `StoreError::NoArrangement`, which says the name back rather
    /// than resetting the console under an operator who mistyped it; and a file
    /// that disagrees with itself is refused by `karakuri-layout`'s own loader
    /// rather than repaired
    /// (`docs/adr/0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md`).
    ///
    /// The viewport in the file is the one it was saved at and is not the one it
    /// comes back at. A window is not part of what an operator kept: the caller
    /// sets the current viewport and solves, exactly as
    /// [`ResetArrangement`](Operation::ResetArrangement) carries the viewport
    /// across today.
    ///
    /// This never reaches the built-in. The default arrangement is not a file and
    /// there is no reserved name, so an operator may keep one of their own called
    /// `default` and it shadows nothing (ADR-0221 §2).
    RestoreArrangement { name: String } => "Put a saved arrangement back",

    // ----- Output and recording -----------------------------------------

    /// A window drag sets an output's size, which is what this row means since
    /// 2026-09-09. The render size belongs to an output and not to the session
    /// ([ADR-0246](../../../docs/adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)),
    /// so dragging the projector window sizes that output and dragging the
    /// console's own window — or a divider inside it — sizes the program view,
    /// whose size is the rectangle the Program bay gives the picture. The frame is
    /// composited once at the largest enabled output's size and scaled into each
    /// ([ADR-0247](../../../docs/adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md)),
    /// so a drag moves what a frame costs. *The window is a preview and has no say
    /// in what is drawn* is what this said while ADR-0077 stood, and its premise
    /// was removed rather than argued with.
    ///
    /// No control on this console sizes a window and none is planned. The window
    /// manager draws the frame a hand drags, on every platform this program runs
    /// on, and `crates/karakuri` answers `WindowEvent::Resized` — which is
    /// [`Quit`](Operation::Quit)'s argument one row along, and why the page's panel
    /// column is a `gap` and the badge that says the route is real is in the fifth
    /// cell.
    ///
    /// `karakuri-cli`'s `a` is a translation that asks for the canvas's own size,
    /// and it is that program's keyboard rather than the instrument's (ADR-0220).
    /// The instrument binds no key here and the row's key column is a `gap`: a size
    /// is a pair of viewport pixels, and a key press cannot mean one — which is
    /// [`MoveBoundary`](Operation::MoveBoundary)'s sentence read at the window
    /// instead of at a divider. `a` naming the canvas rather than a size is the way
    /// round that, and it is a *fit* rather than a size somebody said.
    ///
    /// `--canvas` is on this row's CLI badge and is not this operation — see the
    /// report.
    SizeWindow { width: u32, height: u32 } => "Size the window",

    /// One output, named, and whether it is on — see [`Output`], which is where the
    /// closed list is argued.
    ///
    /// It was [`Undecided`] until 2026-09-09 because no output had an identity
    /// anywhere in this workspace: `karakuri_engine::frame::Sink` is a trait with
    /// no name and no id, the projector window and the plugin sinks did not exist,
    /// and the console's Outputs row reached its one sink by folding a *layout
    /// region*, so the only route was spelled as a fold's target and not as an
    /// output at all.
    ///
    /// The fold is still where the picture's state is kept, and that is deliberate:
    /// `RouteFrame { output: Output::Program, on }` is what a press *asks for*, and
    /// `crates/karakuri` performs it by folding the picture's node — so there is
    /// one stored answer to *is the picture on* and this operation names it rather
    /// than duplicating it.
    ///
    /// `on` and not a toggle. A surface that can only switch has no way to arrive,
    /// and two surfaces switching one sink disagree about where they are —
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md).
    /// The chip's toggle is the console's affordance over the two states.
    RouteFrame { output: Output, on: bool } => "Choose where the frame goes",

    /// The timeline as it happens, replayable frame for frame.
    RecordSession { recording: Recording } => "Record the session",

    /// Waits up to five seconds for a save still being written, then says how many
    /// it left behind rather than letting a hung disk hold the quit.
    ///
    /// The instrument binds no key to this, and stopped binding one on 2026-09-09.
    /// `esc` quit until then and now goes up one level of the focused bay's address
    /// ([ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
    /// [ADR-0332](../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)):
    /// a ladder of `esc` presses ends in something irreversible, in front of an
    /// audience, reached by repeating one key
    /// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    /// The way out is the window's own close, which every platform already has a
    /// gesture for and `crates/karakuri` already answers — so the key column of
    /// this row is a `gap` and the badge that says the route is real is in the
    /// fifth cell, which is [`SizeWindow`](Operation::SizeWindow)'s argument one
    /// row back.
    Quit => "Quit",
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The macro is what keeps [`Operation::TITLES`] and the variants in step, and
    /// this is the one property of it worth asserting on its own: a title reached
    /// through a value is the same string the list holds.
    #[test]
    fn a_title_is_the_one_in_the_list() {
        let op = Operation::SetGain { deck: 0, gain: 1.0 };
        assert_eq!(op.title(), "Gain");
        assert!(Operation::TITLES.contains(&op.title()));
        assert_eq!(Operation::Quit.title(), "Quit");
    }

    /// Titles are the key the manual is matched on, so two variants sharing one
    /// would let a missing operation pass the cross-check: the duplicate would
    /// answer for the row and nothing would say the second variant was never
    /// specified.
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

    /// A floor, not a count: the point is that the list cannot come back empty. The
    /// exact number is the manual's to state and is asserted against the page
    /// itself in `tests/`. It read 46 when this landed, 45 once two residency rows
    /// became one (ADR-0186), 46 again since the look split into a tone map and an
    /// exposure (ADR-0192), and 48 since the mask took a row for its shape and a
    /// row for its position (ADR-0201); it moves with the page and is never lowered
    /// to make a shorter list pass. It was 49 once the arrangement gained a reset
    /// (ADR-0208), 50 since a node gained an authority (ADR-0211), 52 since the
    /// staging lane gained a keep and a put-back, 54 since the arrangement's reset
    /// stopped being the only member of its family (ADR-0221), and is 55 since the
    /// master out became a level something can name (ADR-0224).
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
