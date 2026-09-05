//! **What may be asked, and by what.** The audit
//! [ADR-0235](../../../docs/adr/0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md)
//! decided: every operation is connected, and the ones that could stop a
//! performance are **refused until the operator opens the class they are in**.
//!
//! # It is one classification and one sentence, for every route
//!
//! [ADR-0236](../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
//! is why it is not in the MCP server: *"a rule held by one surface binds one
//! surface"*, and a sequencer lane is already decided as a fifth route
//! (ADR-0222) which would arrive with a second copy of this table. The audit
//! belongs to **the map** — the layer between a surface and the vocabulary —
//! and that layer does not exist as a crate: `karakuri-midi` is its only built
//! instance and ADR-0236 deliberately builds none of the rest of it.
//!
//! **So this sits in the leaf every surface already depends on, and that is a
//! co-location rather than a claim that the audit is vocabulary.** ADR-0236 is
//! explicit that *the map is not the vocabulary*, and this module is not an
//! [`Operation`]: it names none, it adds none, and `Operation::TITLES` is
//! untouched by it. What it buys by being here is the thing ADR-0236 asks for —
//! `karakuri-console`, `karakuri-midi`, `karakuri-cli` and
//! `karakuri-environment` all depend on this crate and on nothing in common
//! besides, so the check and the sentence are written once and no future route
//! needs a new dependency to reach them. The day the map layer is a crate this
//! module moves into it whole.
//! (`docs/contributing.md` §4:
//! *every surface reaches the vocabulary through a map* is the shape being
//! named, not the shape that exists.)
//!
//! # The check is a type, not a call at the top of a function
//!
//! *"An audit skipped on one path is the whole mechanism gone."* So
//! [`audit`] is the only constructor of [`Allowed`], [`Allowed`]'s field is
//! private, and a performer takes an [`Allowed`] rather than an [`Operation`].
//! A caller in another crate **cannot** reach the performer without having
//! been through here; forgetting the check is a compile error rather than a
//! review comment.
//! (`docs/contributing.md` §4.)
//!
//! # Closed by default is the type's own default
//!
//! [`Open`]'s fields are private and every one of them is `false` to begin, so
//! there is no literal anywhere that starts a class open: the only way to an
//! open class is [`Open::with`], which names it. A caller that says nothing
//! has closed all four.
//!
//! # The classification is exhaustive over the vocabulary
//!
//! [`standing`] is a `match` with **no wildcard arm**, which is
//! `karakuri_operation_record::written`'s discipline and its reason: *"an
//! operation added to the vocabulary stops the build here until somebody says
//! what it writes, so the classification cannot drift the way a wildcard arm
//! would let it."* A sixty-fifth operation does not compile until somebody
//! says which class it is in.

use crate::Operation;

/// **A class of operations the operator can open**, and the bay whose head
/// opens it. ADR-0235 names four and each has a bay.
///
/// The classes are drawn off P-0094's question — *what does this do at its
/// worst, on the frame it goes wrong, while the operator's attention is on the
/// room?* — and **not off the nouns**. That is why
/// [`Operation::WriteProcedure`] is open although it rewrites what a live deck
/// is drawing: it is priced before it is built, lands at a frame boundary and
/// rolls back on its own, so it fails to be an *unpriced, immediate,
/// irreversible* write on every count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// **What a deck that is live is drawing.** Opened at the head of the
    /// **Program** bay, which is where what is on air lives.
    LiveDeck,
    /// **The mix faders.** Opened at the head of the **Mixer** bay.
    MixFaders,
    /// **The master effects.** Opened at the head of the **Master** bay.
    MasterEffects,
    /// **Inputs and outputs, routed, enabled and disabled.** Opened at the head
    /// of the **Outputs** row.
    InputsAndOutputs,
}

impl Class {
    /// Every class, for a caller drawing one indicator per class.
    pub const ALL: &'static [Class] = &[
        Class::LiveDeck,
        Class::MixFaders,
        Class::MasterEffects,
        Class::InputsAndOutputs,
    ];

    /// The class in the words ADR-0235 named it in, which is the words the
    /// refusal says it in.
    pub fn title(self) -> &'static str {
        match self {
            Class::LiveDeck => "what a deck that is live is drawing",
            Class::MixFaders => "the mix faders",
            Class::MasterEffects => "the master effects",
            Class::InputsAndOutputs => "inputs and outputs, routed, enabled and disabled",
        }
    }

    /// **Where the operator opens it**, which the refusal has to say or the
    /// model reports the instrument as incapable rather than as closed
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    pub fn bay(self) -> &'static str {
        match self {
            Class::LiveDeck => "Program",
            Class::MixFaders => "Mixer",
            Class::MasterEffects => "Master",
            Class::InputsAndOutputs => "Outputs",
        }
    }

    /// **Where the operator finds the pill**, in the words the refusal says it
    /// in — and three of the four are *the head of the … bay* while one is not.
    ///
    /// The Outputs row **has no head to put an indicator in**. `karakuri-console`
    /// says so outright of `Kind::Outputs`: it is a label *"inside a row that has
    /// no head at all (ADR-0159), so it is that typography and none of that
    /// structure: no hairline under it, no pills or grip beside it."* So the
    /// pill sits beside the word that stands in for a head, and
    /// `docs/manual/console.html` specifies it there and says why.
    ///
    /// **A refusal naming a place that does not exist is worse than one naming
    /// none**, because a model repeats it to the person sitting there and sends
    /// them looking for a head. That is the whole reason this is a second
    /// function rather than a format string over [`Class::bay`].
    pub fn opened_at(self) -> &'static str {
        match self {
            Class::LiveDeck => "the head of the Program bay",
            Class::MixFaders => "the head of the Mixer bay",
            Class::MasterEffects => "the head of the Master bay",
            Class::InputsAndOutputs => "the Outputs row, which has no head",
        }
    }
}

/// **Closed by ADR-0235's rule applied past the four classes, with no bay
/// named.**
///
/// ADR-0235 closes fourteen rows that are in none of the four — *"His list is
/// exemplary — 例えば — and the rule is the question, not the list"* — and says
/// so of [`Unclassed::Quitting`] in as many words: *"the sharpest case in the
/// vocabulary and **in none of the four classes**."* It names no bay for any of
/// them, and its own *What this leaves undone* keeps them open: *"whether the
/// clock, `Quit`, `SelectDeck` and the lane rows are closed as this record
/// classes them … each is one line to move."*
///
/// **So they are refused and nothing opens them**, which is the safe half of an
/// undecided question and is said out loud rather than smoothed over: an
/// [`Open`] has no field for these, so a caller cannot open one by mistake and
/// cannot open one on purpose either. Moving a group into a [`Class`] is one
/// line here the day the maintainer says which bay it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unclassed {
    /// The clock. *"Unpriced, immediate, irreversible — a tap sets the phase
    /// and there is no un-tapping it."*
    Clock,
    /// [`Operation::Quit`]. *"It does not risk stopping the performance, it
    /// stops it."*
    Quitting,
    /// [`Operation::SelectDeck`]. Closed by P-0094's second half: it decides
    /// which deck the operator's *next key press* lands on.
    Selection,
    /// The sequencer's lanes. A lane emits operations, so one pointed at a
    /// fader and unmuted is a mix write on a delay.
    Lanes,
    /// [`Operation::SetAuthority`]. **A permission an actor can grant itself is
    /// not a permission**, and ADR-0235 recommends this one is never openable
    /// while leaving that the maintainer's.
    Authority,
}

impl Unclassed {
    /// The group in the words the refusal says it in.
    pub fn title(self) -> &'static str {
        match self {
            Unclassed::Clock => "the clock",
            Unclassed::Quitting => "quitting",
            Unclassed::Selection => "which deck a key press lands on",
            Unclassed::Lanes => "the sequencer's lanes",
            Unclassed::Authority => "who may move a node",
        }
    }
}

/// Which reading a row's class turns on, so a refusal can say what nobody
/// handed over rather than that something was missing.
///
/// `karakuri_operation_record::Reading`'s shape and its reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// [`Running::live`], which [`Operation::LoadSet`]'s class turns on.
    Live,
}

/// **What the audit reads about the instrument**, for the rows whose class is a
/// predicate over an operation *and its target* rather than over the operation
/// alone.
///
/// ADR-0235 calls this *"the sharpest new cost in the design: the audit is not
/// a pure function of the operation"*, and it is one row —
/// [`Operation::LoadSet`], because the class the maintainer drew is *a deck in
/// live mode*. Loading into an `Allocated` or `Priming` slot touches nothing on
/// air.
#[derive(Debug, Clone, Copy, Default)]
pub struct Running<'a> {
    /// **The decks whose slot is live**, as somebody read them from the engine.
    ///
    /// `None` is *nobody read it*, and a row that turns on it is then refused
    /// rather than guessed — the reading is missing, the safe answer is the
    /// closed one, and [`refusal`] says which reading was missing rather than
    /// saying *closed* and leaving the caller nothing to act on.
    pub live: Option<&'a [u8]>,
}

impl<'a> Running<'a> {
    /// Nothing read. See [`Running::live`].
    pub fn unread() -> Running<'a> {
        Running { live: None }
    }

    /// The decks that are live, read.
    pub fn live(decks: &'a [u8]) -> Running<'a> {
        Running { live: Some(decks) }
    }
}

/// **Where one operation stands with the audit**, before the operator's opening
/// is consulted.
///
/// [`standing`] answers this; [`audit`] is what turns it and an [`Open`] into
/// a yes or a sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// **Open by default** — twenty-three rows. Nothing gates it and nothing
    /// ever did.
    Open,
    /// **Closed by default**, and the operator opens this class at the head of
    /// [`Class::bay`].
    Closed(Class),
    /// **Closed by default**, and no bay opens it. See [`Unclassed`].
    ClosedUnclassed(Unclassed),
    /// **Closed**, because the reading its class turns on was not taken. See
    /// [`Running`].
    Unread(Reading),
}

/// **Which classes the operator has opened.**
///
/// The fields are private and every one is `false`, so **closed by default is
/// the type's own `Default`**: there is no way to write down an `Open` that
/// starts open, and the only route to one is [`Open::with`], which has to name
/// the class.
///
/// [`Unclassed`] groups have no field here on purpose — see that type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Open {
    live_deck: bool,
    mix_faders: bool,
    master_effects: bool,
    inputs_and_outputs: bool,
}

impl Open {
    /// All four closed, which is what a run starts with.
    pub const CLOSED: Open = Open {
        live_deck: false,
        mix_faders: false,
        master_effects: false,
        inputs_and_outputs: false,
    };

    /// This opening with one class set. **Names a state and never a
    /// direction**, which is
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// applied to a setting the vocabulary does not own: a bay-head pill can be
    /// a toggle, and what it writes still says which state it means.
    pub fn with(self, class: Class, open: bool) -> Open {
        let mut next = self;
        match class {
            Class::LiveDeck => next.live_deck = open,
            Class::MixFaders => next.mix_faders = open,
            Class::MasterEffects => next.master_effects = open,
            Class::InputsAndOutputs => next.inputs_and_outputs = open,
        }
        next
    }

    /// Whether the operator has opened this class.
    pub fn holds(self, class: Class) -> bool {
        match class {
            Class::LiveDeck => self.live_deck,
            Class::MixFaders => self.mix_faders,
            Class::MasterEffects => self.master_effects,
            Class::InputsAndOutputs => self.inputs_and_outputs,
        }
    }
}

/// **An operation that has been through the audit**, and the only thing a
/// performer will take.
///
/// The field is private and [`audit`] is the only function that builds one, so
/// a caller in another crate has no way to perform an operation that was not
/// checked. That is the structural half of *"an audit skipped on one path is
/// the whole mechanism gone"*.
#[derive(Debug, Clone, Copy)]
pub struct Allowed<'a>(&'a Operation);

impl<'a> Allowed<'a> {
    /// What was allowed. Borrowed rather than owned: the audit reads and
    /// answers, it never rewrites what was asked for.
    pub fn operation(&self) -> &'a Operation {
        self.0
    }
}

/// **The gate.** One operation, the opening the operator has set, and what has
/// been read of what is running — a yes, or the one sentence it is refused in.
///
/// Called once, over the operation, **after the call is named and before it
/// acts**. In `karakuri_environment::mcp` that is between `asked` and
/// `perform`, which is the only seam every tool crosses.
pub fn audit<'a>(
    operation: &'a Operation,
    open: Open,
    running: Running<'_>,
) -> Result<Allowed<'a>, String> {
    let standing = standing(operation, running);
    match standing {
        Standing::Open => Ok(Allowed(operation)),
        Standing::Closed(class) if open.holds(class) => Ok(Allowed(operation)),
        _ => Err(refusal(operation, standing)
            .expect("a standing that is not `Open` and not an opened class has a sentence")),
    }
}

/// **The one sentence a refused call is answered in**, and `None` where there
/// is nothing to refuse.
///
/// A free function, because
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// says a refusal a person can reach from two surfaces is one sentence — and
/// this one will be reachable from a second automatic route the day a lane
/// exists (ADR-0222). Asserted by equality against this function rather than by
/// a `contains`.
///
/// It says three things because
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)
/// asks for the constraint and not only the fact: **which operation**, **which
/// class it is in**, and **that the operator can open that class, and where**.
/// A model told only *no* reports the instrument as incapable; one told this
/// can hand the person sitting there something to do.
///
/// **[`Operation::LoadSet`] names its deck**, because its class turns on the
/// deck being live: two identical calls are answered differently a minute
/// apart, and a refusal that only said *closed* would be unfixable by the model
/// that got it.
pub fn refusal(operation: &Operation, standing: Standing) -> Option<String> {
    let title = operation.title();
    Some(match standing {
        Standing::Open => return None,
        Standing::Closed(class) => format!(
            "`{title}`{} is in the class {}, which is closed by default — the operator opens \
             it at {}.",
            because(operation, class),
            class.title(),
            class.opened_at()
        ),
        Standing::ClosedUnclassed(group) => format!(
            "`{title}` is closed by default with {}, and no bay opens that yet — which class \
             it belongs to is not settled.",
            group.title()
        ),
        Standing::Unread(Reading::Live) => format!(
            "`{title}` is closed by default where the deck it names is live, and which decks \
             are live was not read."
        ),
    })
}

/// The clause a refusal owes where the class turned on more than the
/// operation's name. Empty for every row but one — see [`refusal`].
fn because(operation: &Operation, class: Class) -> String {
    match (operation, class) {
        (Operation::LoadSet { deck, .. }, Class::LiveDeck) => format!(" (deck {deck} is live)"),
        _ => String::new(),
    }
}

/// **Where every operation in the vocabulary stands**, per operation and
/// exhaustively.
///
/// **No wildcard arm.** See the module documentation: a sixty-fifth operation
/// stops the build here until somebody says which class it is in, which is
/// `karakuri_operation_record::written`'s discipline and its reason.
///
/// The counts ADR-0235 states, and which
/// `the_classification_is_the_split_adr_0235_states` holds this to: **40
/// closed, 23 open, 63 total.**
pub fn standing(operation: &Operation, running: Running<'_>) -> Standing {
    match operation {
        // ----- The clock ---------------------------------------------------
        //
        // ADR-0235 applying its own rule past the maintainer's examples:
        // unpriced, immediate, irreversible, and everything moving on the grid
        // moves with them.
        Operation::TapBeat => Standing::ClosedUnclassed(Unclassed::Clock),
        Operation::ScaleGrid { .. } => Standing::ClosedUnclassed(Unclassed::Clock),
        Operation::SetLatencyOffset { .. } => Standing::ClosedUnclassed(Unclassed::Clock),
        Operation::SetSync { .. } => Standing::ClosedUnclassed(Unclassed::Clock),
        Operation::ScrubDeck { .. } => Standing::ClosedUnclassed(Unclassed::Clock),
        Operation::SetFreeRunTempo { .. } => Standing::ClosedUnclassed(Unclassed::Clock),

        // ----- Inputs and outputs, routed, enabled and disabled -------------
        //
        // *"None of them has a bounded worst case and all four are the show's
        // plumbing rather than its picture, which is exactly why they are easy
        // to forget."* ADR-0235 named four; `SetPreview` was retired by
        // ADR-0240 and the remaining three keep the class and the rule.
        Operation::AttachBeatSource { .. } => Standing::Closed(Class::InputsAndOutputs),
        Operation::RouteFrame { .. } => Standing::Closed(Class::InputsAndOutputs),
        Operation::RecordSession { .. } => Standing::Closed(Class::InputsAndOutputs),

        // ----- Which deck the next key press lands on -----------------------
        Operation::SelectDeck { .. } => Standing::ClosedUnclassed(Unclassed::Selection),

        // ----- What a deck that is live is drawing --------------------------
        //
        // *"At its worst each of these replaces what the audience is looking
        // at, in the frame it arrives, with nothing that prices it and nothing
        // that puts it back."*
        //
        // **`SetResidency` is the hinge**: it is how a slot becomes live and
        // how one stops being live, so leaving it open while closing the
        // contents would be a hole big enough to walk the whole class through.
        Operation::SetResidency { .. } => Standing::Closed(Class::LiveDeck),
        // **The one row whose class is a predicate over its target.** Loading
        // into a slot that is not live touches nothing on air; loading into one
        // that is replaces the picture.
        Operation::LoadSet { deck, .. } => match running.live {
            None => Standing::Unread(Reading::Live),
            Some(live) if live.contains(deck) => Standing::Closed(Class::LiveDeck),
            Some(_) => Standing::Open,
        },
        Operation::SetCompositing { .. } => Standing::Closed(Class::LiveDeck),
        Operation::SelectRenderer { .. } => Standing::Closed(Class::LiveDeck),
        Operation::WriteParam { .. } => Standing::Closed(Class::LiveDeck),
        Operation::AttachSignal { .. } => Standing::Closed(Class::LiveDeck),
        Operation::TakeParamBack { .. } => Standing::Closed(Class::LiveDeck),
        Operation::SetProperty { .. } => Standing::Closed(Class::LiveDeck),
        // Here for a reason of its own and the same rule: narrowing a live
        // deck's published interface takes controls out from under the
        // operator's hand and renumbers every MIDI binding after the one it
        // removed.
        Operation::Publish { .. } => Standing::Closed(Class::LiveDeck),

        // ----- The mix faders ----------------------------------------------
        //
        // The class P-0094 already worked: none of its three answers is
        // available, and it is what the audience is looking at.
        Operation::SetGain { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetOpacity { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetBlendMode { .. } => Standing::Closed(Class::MixFaders),
        Operation::FadeDeck { .. } => Standing::Closed(Class::MixFaders),
        Operation::Crossfade { .. } => Standing::Closed(Class::MixFaders),
        Operation::Wipe { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetMaskShape { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetMaskPosition { .. } => Standing::Closed(Class::MixFaders),
        // **A level and not an effect**, which is why it is filed here rather
        // than with the master chain (ADR-0224): one number at the entry to the
        // master chain that blacks out the whole fold.
        Operation::SetMasterOut { .. } => Standing::Closed(Class::MixFaders),

        // ----- The master effects -------------------------------------------
        //
        // Every one acts on the composited frame after the mix has run,
        // unpriced and immediately. The first three carry `Undecided` because
        // the chain does not exist yet — *"so they are closed before they are
        // buildable, which is the correct order."*
        Operation::SetFeedback { .. } => Standing::Closed(Class::MasterEffects),
        Operation::SetBloom { .. } => Standing::Closed(Class::MasterEffects),
        Operation::SetRgbShift { .. } => Standing::Closed(Class::MasterEffects),
        Operation::SetTonemap { .. } => Standing::Closed(Class::MasterEffects),
        Operation::SetExposure { .. } => Standing::Closed(Class::MasterEffects),

        // ----- The sequencer's lanes ----------------------------------------
        //
        // *"The route that would defeat it is a lane, not a tool."* All five
        // carry `Undecided` today; they are classed now so that the pattern
        // arriving is not also the day the audit acquires a hole.
        Operation::SetStep { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),
        Operation::SetLaneMute { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),
        Operation::PointLane { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),
        Operation::SetPatternGrid { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),
        Operation::SelectPattern { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),

        // ----- Who may move a node ------------------------------------------
        Operation::SetAuthority { .. } => Standing::ClosedUnclassed(Unclassed::Authority),

        // ----- Quitting -----------------------------------------------------
        Operation::Quit => Standing::ClosedUnclassed(Unclassed::Quitting),

        // ----- Open by default ----------------------------------------------
        //
        // **Twenty-three rows, and `WriteProcedure` is the case that decides
        // how the classes are drawn.** It rewrites the contents of a deck that
        // is in live mode and it stays open, because the class is not the noun:
        // it is P-0094's question with the noun as its subject, and a procedure
        // rewrite is priced by the check pass, compiled off the frame path,
        // installed at a frame boundary, measured for thirty frames and rolled
        // back to the parked previous version if it costs too much. *"Its worst
        // case is the picture it replaced, coming back."* `WireInput`,
        // `RestoreProcedure`, `KeepCandidate` and `WatchFiles` land through the
        // same rebuild.
        Operation::SetTransition { .. } => Standing::Open,
        Operation::WireInput { .. } => Standing::Open,
        Operation::SaveSet { .. } => Standing::Open,
        Operation::ListSets { .. } => Standing::Open,
        Operation::SelectScope { .. } => Standing::Open,
        Operation::ReadSet { .. } => Standing::Open,
        Operation::TransferSet { .. } => Standing::Open,
        Operation::WalkHistory { .. } => Standing::Open,
        Operation::ReadProcedure { .. } => Standing::Open,
        Operation::WriteProcedure { .. } => Standing::Open,
        Operation::WatchFiles { .. } => Standing::Open,
        Operation::SwapOutcome => Standing::Open,
        Operation::KeepCandidate { .. } => Standing::Open,
        Operation::RestoreProcedure { .. } => Standing::Open,
        // **The nine that arrange the console are the closest call on this
        // side.** Folding away the bay holding the fader an operator is
        // reaching for is a real hazard, and it is P-0094's third answer with
        // the controls intact: it is the largest visible change the panel can
        // make, and it is undone by one key the operator's hand is already
        // near.
        Operation::MoveBoundary { .. } => Standing::Open,
        Operation::FoldBay { .. } => Standing::Open,
        Operation::FoldPane { .. } => Standing::Open,
        Operation::Unfold { .. } => Standing::Open,
        Operation::Solo { .. } => Standing::Open,
        Operation::ResetArrangement => Standing::Open,
        Operation::SaveArrangement { .. } => Standing::Open,
        Operation::RestoreArrangement { .. } => Standing::Open,
        Operation::SizeWindow { .. } => Standing::Open,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Authority, BeatSource, BlendMode, Control, Curve, GridScale, Layer, NodeAt, ParamAt,
        ParamValue, Property, Recording, Residency, SetTransfer, Sync, Tonemap, TransitionSetting,
        Undecided, WipeKind,
    };

    fn node() -> NodeAt {
        NodeAt {
            layer: Layer::L4,
            index: 0,
        }
    }

    fn param() -> ParamAt {
        ParamAt {
            node: Some(node()),
            key: "radius".into(),
        }
    }

    /// **One of every operation in the vocabulary, in the manual's order.**
    ///
    /// It is checked against [`Operation::TITLES`] rather than counted, so an
    /// operation added to the vocabulary and forgotten here is a failing test
    /// rather than a fixture that is quietly one short — which is what would
    /// let the split assertions below go on passing over a row nobody classed.
    fn every_operation() -> Vec<Operation> {
        vec![
            Operation::TapBeat,
            Operation::ScaleGrid {
                by: GridScale::Halve,
            },
            Operation::SetLatencyOffset { ms: 5.0 },
            Operation::SetSync {
                deck: 0,
                sync: Sync::Beat,
            },
            Operation::ScrubDeck {
                deck: 0,
                beats: 0.25,
            },
            Operation::SetFreeRunTempo { bpm: 120.0 },
            Operation::AttachBeatSource {
                source: BeatSource::AudioInput("default".into()),
            },
            Operation::SelectDeck { deck: 0 },
            Operation::SetResidency {
                deck: 0,
                residency: Residency::Live,
            },
            Operation::LoadSet {
                deck: 0,
                set: "a".into(),
            },
            Operation::SetCompositing {
                deck: 0,
                compositing: true,
            },
            Operation::SetGain { deck: 0, gain: 1.0 },
            Operation::SetOpacity {
                deck: 0,
                opacity: 1.0,
            },
            Operation::SetBlendMode {
                deck: 0,
                blend: BlendMode::Add,
            },
            Operation::FadeDeck { deck: 0, to: 0.0 },
            Operation::Crossfade { from: 0, to: 1 },
            Operation::Wipe { from: 0, to: 1 },
            Operation::SetMaskShape {
                deck: 0,
                kind: WipeKind::Linear,
                angle: 0.0,
            },
            Operation::SetMaskPosition {
                deck: 0,
                position: 0.5,
            },
            Operation::SetTransition {
                setting: TransitionSetting::Quantum { beats: 4.0 },
            },
            Operation::SelectRenderer {
                deck: 0,
                renderer: 0,
            },
            Operation::SetMasterOut { out: 1.0 },
            Operation::SetFeedback { params: Undecided },
            Operation::SetBloom { params: Undecided },
            Operation::SetRgbShift { params: Undecided },
            Operation::SetTonemap {
                tonemap: Tonemap::Aces,
            },
            Operation::SetExposure { exposure: 1.0 },
            Operation::SetStep { step: Undecided },
            Operation::SetLaneMute { lane: Undecided },
            Operation::PointLane { target: Undecided },
            Operation::SetPatternGrid { grid: Undecided },
            Operation::SelectPattern { pattern: Undecided },
            Operation::WriteParam {
                deck: 0,
                param: param(),
                value: ParamValue::Scalar(1.0),
            },
            Operation::AttachSignal {
                deck: 0,
                param: param(),
                signal: "rms".into(),
                curve: Curve::Lin,
                range: [0.0, 1.0],
            },
            Operation::TakeParamBack {
                deck: 0,
                param: param(),
            },
            Operation::WireInput {
                deck: 0,
                node: "warp".into(),
                slot: "shape".into(),
                to: "blob".into(),
            },
            Operation::Publish {
                deck: 0,
                controls: vec![Control {
                    name: "level".into(),
                    node: Some(node()),
                    key: "exposure".into(),
                    range: [0.0, 1.0],
                }],
            },
            Operation::SetProperty {
                deck: 0,
                property: Property::Seed {
                    node: node(),
                    salt: 1,
                },
            },
            Operation::SetAuthority {
                deck: 0,
                node: node(),
                authority: Authority::Manual,
            },
            Operation::SaveSet { deck: 0, id: None },
            Operation::ListSets {
                holds: None,
                layer: None,
            },
            Operation::SelectScope { scope: Undecided },
            Operation::ReadSet { id: "a".into() },
            Operation::TransferSet {
                transfer: SetTransfer::Send { id: "a".into() },
            },
            Operation::WalkHistory { step: Undecided },
            Operation::ReadProcedure {
                deck: 0,
                node: node(),
            },
            Operation::WriteProcedure {
                deck: 0,
                node: node(),
                source: "proc p { kind L4 }".into(),
            },
            Operation::WatchFiles {
                watching: Undecided,
            },
            Operation::SwapOutcome,
            Operation::KeepCandidate {
                deck: 0,
                node: node(),
            },
            Operation::RestoreProcedure {
                deck: 0,
                node: node(),
            },
            Operation::MoveBoundary {
                boundary: Undecided,
            },
            Operation::FoldBay {
                bay: "mixer".into(),
            },
            Operation::FoldPane {
                pane: "inspector-1".into(),
            },
            Operation::Unfold { region: None },
            Operation::Solo { region: None },
            Operation::ResetArrangement,
            Operation::SaveArrangement { name: "a".into() },
            Operation::RestoreArrangement { name: "a".into() },
            Operation::SizeWindow {
                width: 1280,
                height: 720,
            },
            Operation::RouteFrame { output: Undecided },
            Operation::RecordSession {
                recording: Recording::Stop,
            },
            Operation::Quit,
        ]
    }

    /// **No deck is live**, which is the reading that puts `LoadSet` on the
    /// open side. The fixture's decks are all 0, so this is a list that does
    /// not hold it rather than an empty one — an empty list would pass against
    /// a `contains` that had been inverted.
    const NOTHING_LIVE: &[u8] = &[7];
    const DECK_0_IS_LIVE: &[u8] = &[0];

    /// The fixture is the vocabulary, and nothing here is asserted about a
    /// shorter list than the manual specifies.
    #[test]
    fn the_fixture_holds_one_of_every_operation() {
        let titles: Vec<&str> = every_operation().iter().map(Operation::title).collect();
        assert_eq!(
            titles,
            Operation::TITLES.to_vec(),
            "the fixture and the vocabulary have come apart — every assertion below is over \
             whichever rows the fixture happens to hold"
        );
    }

    /// **A sixty-fifth operation cannot be added without somebody saying which
    /// class it is in.**
    ///
    /// The property itself is the compiler's: [`standing`] is a `match` over
    /// `Operation` with no wildcard arm, so a new variant is a
    /// `non-exhaustive patterns` error and this crate does not build. **A test
    /// cannot assert that** — a test only runs on a build that succeeded, so a
    /// green suite is evidence of nothing here. What a test can do is hold the
    /// *shape* the compiler needs: the day somebody silences that error with
    /// `_ => Standing::Open` the build goes green again and the mechanism is
    /// gone silently, and this is what makes that loud.
    ///
    /// It reads this file's own text, which is
    /// `karakuri::key_column::the_keys_this_file_lists_are_the_keys_the_window_loop_binds`'s
    /// method and its reason: the thing being checked is the source, so the
    /// source is what is read.
    #[test]
    fn a_wildcard_arm_would_end_the_exhaustiveness() {
        let source = include_str!("gate.rs");
        let from = source
            .find("pub fn standing(")
            .expect("`standing` is in this file");
        let to = source[from..]
            .find("\n#[cfg(test)]")
            .map(|at| from + at)
            .unwrap_or(source.len());
        for line in source[from..to].lines() {
            let line = line.trim_start();
            assert!(
                !(line.starts_with("_ =>") || line.starts_with("_ if")),
                "`standing` has a wildcard arm: `{line}`. The classification is exhaustive \
                 over the vocabulary on purpose — a wildcard is how a sixty-fourth operation \
                 gets a class nobody chose"
            );
        }
    }

    /// **40 closed, 23 open, 63 total** — ADR-0235's count less the one row
    /// ADR-0240 retired. It is still the one number that says the
    /// classification was applied to the whole vocabulary rather than to the
    /// rows somebody remembered: the record read 41, 23, 64, and
    /// *Choose what the output shows* leaving the vocabulary takes one off the
    /// closed side and off the total.
    ///
    /// **Counted with the deck `LoadSet` names live**, because that is how the
    /// record counts it: the row is listed under *what a live deck is drawing*
    /// and the 40 includes it. It is the one row whose standing is not a
    /// function of the operation alone, so the split is a split *given a
    /// reading* — and the reading that makes it 40 is the one the class was
    /// drawn for. With nothing live it is 39 and 24, which is the same
    /// classification and not a second one.
    #[test]
    fn the_classification_is_the_split_adr_0235_states() {
        let running = Running::live(DECK_0_IS_LIVE);
        let mut open = 0;
        let mut closed = 0;
        for operation in every_operation() {
            match standing(&operation, running) {
                Standing::Open => open += 1,
                Standing::Closed(_) | Standing::ClosedUnclassed(_) | Standing::Unread(_) => {
                    closed += 1
                }
            }
        }
        assert_eq!((closed, open, closed + open), (40, 23, 63));
    }

    fn members(class: Class) -> Vec<&'static str> {
        every_operation()
            .iter()
            .filter(|operation| {
                standing(operation, Running::live(DECK_0_IS_LIVE)) == Standing::Closed(class)
            })
            .map(|operation| operation.title())
            .collect()
    }

    /// **What a deck that is live is drawing is closed until the Program bay
    /// opens it**, and `LoadSet` is in it only while the deck it names is live.
    #[test]
    fn what_a_live_deck_is_drawing_is_closed_until_the_program_bay_opens_it() {
        assert_eq!(
            members(Class::LiveDeck),
            vec![
                "Put a deck on air, prime it, or take it off",
                "Load material into a deck",
                "Composite a deck's renderers",
                "Choose which renderer of a deck is live",
                "Write a parameter",
                "Attach a signal to a parameter",
                "Take a parameter back",
                "Narrow the published interface",
                "Element capacity, seeds, the camera",
            ]
        );
        assert_eq!(Class::LiveDeck.bay(), "Program");
    }

    /// **The mix faders are closed until the Mixer bay opens them**, and the
    /// master out is filed with them rather than with the master effects
    /// (ADR-0224: it is a level and not an effect).
    #[test]
    fn the_mix_faders_are_closed_until_the_mixer_bay_opens_them() {
        assert_eq!(
            members(Class::MixFaders),
            vec![
                "Gain",
                "Opacity",
                "Blend mode",
                "Fade a deck out or in",
                "Crossfade to the next deck",
                "Wipe the next deck in",
                "Set a deck's mask shape",
                "Set a deck's mask position",
                "Master out",
            ]
        );
        assert_eq!(Class::MixFaders.bay(), "Mixer");
    }

    /// **The master effects are closed until the Master bay opens them**,
    /// including the three whose payload is `Undecided` — closed before they
    /// are buildable, which is the correct order.
    #[test]
    fn the_master_effects_are_closed_until_the_master_bay_opens_them() {
        assert_eq!(
            members(Class::MasterEffects),
            vec!["Feedback", "Bloom", "RGB shift", "Tone map", "Exposure"]
        );
        assert_eq!(Class::MasterEffects.bay(), "Master");
    }

    /// **Inputs and outputs are closed until the Outputs bay opens them** —
    /// the show's plumbing rather than its picture, which is why they are easy
    /// to forget.
    #[test]
    fn inputs_and_outputs_are_closed_until_the_outputs_bay_opens_them() {
        assert_eq!(
            members(Class::InputsAndOutputs),
            vec![
                "Attach a beat source",
                "Choose where the frame goes",
                "Record the session",
            ]
        );
        assert_eq!(Class::InputsAndOutputs.bay(), "Outputs");
    }

    /// **Closed by default, all four classes**, and it is the type's own
    /// `Default` rather than a value a caller chose: there is no way to write
    /// down an `Open` that starts open.
    #[test]
    fn closed_by_default_is_the_types_own_default() {
        assert_eq!(Open::default(), Open::CLOSED);
        for class in Class::ALL {
            assert!(!Open::CLOSED.holds(*class), "{} starts open", class.title());
        }
    }

    /// **Opening one class opens no other**, which is the whole of what an
    /// opening per class means.
    #[test]
    fn opening_one_class_opens_no_other() {
        for class in Class::ALL {
            let open = Open::CLOSED.with(*class, true);
            for other in Class::ALL {
                assert_eq!(
                    open.holds(*other),
                    other == class,
                    "opening {} changed {}",
                    class.title(),
                    other.title()
                );
            }
            assert_eq!(open.with(*class, false), Open::CLOSED);
        }
    }

    /// **A closed operation is refused in the one sentence**, asserted by
    /// **The one class whose pill is not in a bay head says so**, because the
    /// Outputs row has none — a refusal that sent an operator looking for a
    /// head would be worse than one that named no place at all, since a model
    /// repeats it to the person sitting there.
    #[test]
    fn the_class_with_no_bay_head_does_not_send_anyone_looking_for_one() {
        let operation = Operation::RecordSession {
            recording: crate::Recording::Stop,
        };
        let refused = audit(&operation, Open::CLOSED, Running::live(NOTHING_LIVE))
            .expect_err("the show's plumbing is closed by default");
        assert!(
            refused.ends_with("the Outputs row, which has no head."),
            "the refusal for a class drawn in a headless row still points at a head: {refused}"
        );
        for class in [Class::LiveDeck, Class::MixFaders, Class::MasterEffects] {
            assert!(
                class.opened_at().starts_with("the head of the"),
                "{class:?} is drawn in a bay with a head and its refusal no longer says so"
            );
        }
    }

    /// equality against [`refusal`] rather than by a `contains` — P-0090.
    #[test]
    fn a_closed_operation_is_refused_in_the_one_sentence() {
        let operation = Operation::SetOpacity {
            deck: 2,
            opacity: 0.0,
        };
        let refused = audit(&operation, Open::CLOSED, Running::live(NOTHING_LIVE))
            .expect_err("the mix faders are closed by default");
        assert_eq!(
            refused,
            "`Opacity` is in the class the mix faders, which is closed by default — the \
             operator opens it at the head of the Mixer bay."
        );
        assert_eq!(
            Some(refused),
            refusal(&operation, Standing::Closed(Class::MixFaders))
        );
    }

    /// **A refusal names the operation, its class and where the class is
    /// opened** — P-0083, in the register it applies to a tool surface: a
    /// model told only *no* reports the instrument as incapable rather than as
    /// closed.
    #[test]
    fn a_refusal_names_the_operation_its_class_and_the_bay_that_opens_it() {
        for operation in every_operation() {
            let Standing::Closed(class) = standing(&operation, Running::live(NOTHING_LIVE)) else {
                continue;
            };
            let said = refusal(&operation, Standing::Closed(class)).expect("a closed row refuses");
            assert!(said.contains(operation.title()), "{said}");
            assert!(said.contains(class.title()), "{said}");
            assert!(said.contains(class.bay()), "{said}");
            assert!(said.contains("the operator opens it"), "{said}");
        }
    }

    /// **An open operation is untouched by the gate**, with every class closed
    /// — which is the state a run starts in. `write_procedure` is the case the
    /// classes are drawn to keep open: it rewrites what a live deck is drawing
    /// and its worst case is the picture it replaced, coming back.
    #[test]
    fn an_open_operation_is_untouched_by_the_gate() {
        for operation in every_operation() {
            if standing(&operation, Running::live(NOTHING_LIVE)) != Standing::Open {
                continue;
            }
            let allowed = audit(&operation, Open::CLOSED, Running::live(NOTHING_LIVE))
                .unwrap_or_else(|refused| panic!("{refused}"));
            assert_eq!(allowed.operation(), &operation);
            assert_eq!(refusal(&operation, Standing::Open), None);
        }
    }

    /// **The seven tools that exist stay open**, which is ADR-0235's promise
    /// that nothing closes on the day it is recorded. Held here over the
    /// operations; `karakuri_environment::mcp` holds it over the wire.
    #[test]
    fn the_seven_tools_that_exist_are_all_on_the_open_side() {
        for operation in [
            Operation::ReadProcedure {
                deck: 0,
                node: node(),
            },
            Operation::WriteProcedure {
                deck: 0,
                node: node(),
                source: String::new(),
            },
            Operation::WireInput {
                deck: 0,
                node: "a".into(),
                slot: "b".into(),
                to: "c".into(),
            },
            Operation::SwapOutcome,
            Operation::ReadSet { id: "a".into() },
            Operation::ListSets {
                holds: None,
                layer: None,
            },
            Operation::SaveSet { deck: 0, id: None },
        ] {
            assert_eq!(
                standing(&operation, Running::unread()),
                Standing::Open,
                "`{}` is one of the seven tools that exist and ADR-0235 closes none of them",
                operation.title()
            );
        }
    }

    /// **Opening a class lets its operations through and no others.**
    #[test]
    fn an_opened_class_lets_its_own_operations_through_and_no_others() {
        let open = Open::CLOSED.with(Class::MixFaders, true);
        assert!(audit(
            &Operation::SetGain { deck: 0, gain: 0.0 },
            open,
            Running::live(NOTHING_LIVE)
        )
        .is_ok());
        assert!(
            audit(
                &Operation::SetExposure { exposure: 0.0 },
                open,
                Running::live(NOTHING_LIVE)
            )
            .is_err(),
            "opening the mix faders opened the master effects"
        );
    }

    /// **Loading a set into a deck that is not live touches nothing on air**,
    /// so it is open; into one that is, it replaces the picture, so it is
    /// closed — and the refusal names the deck and says it is live, or the
    /// model that got it cannot act on it.
    #[test]
    fn loading_a_set_is_closed_only_where_the_deck_it_names_is_live() {
        let operation = Operation::LoadSet {
            deck: 0,
            set: "a".into(),
        };
        assert_eq!(
            standing(&operation, Running::live(NOTHING_LIVE)),
            Standing::Open
        );
        assert_eq!(
            standing(&operation, Running::live(DECK_0_IS_LIVE)),
            Standing::Closed(Class::LiveDeck)
        );
        assert_eq!(
            audit(&operation, Open::CLOSED, Running::live(DECK_0_IS_LIVE)).expect_err("live"),
            "`Load material into a deck` (deck 0 is live) is in the class what a deck that is \
             live is drawing, which is closed by default — the operator opens it at the head \
             of the Program bay."
        );
    }

    /// **A reading nobody took closes the row it decides**, and the refusal
    /// says which reading was missing rather than saying *closed* — a
    /// diagnostic says why, not only what.
    #[test]
    fn a_reading_nobody_took_closes_the_row_it_decides() {
        let operation = Operation::LoadSet {
            deck: 0,
            set: "a".into(),
        };
        assert_eq!(
            standing(&operation, Running::unread()),
            Standing::Unread(Reading::Live)
        );
        assert_eq!(
            audit(
                &operation,
                Open::CLOSED.with(Class::LiveDeck, true),
                Running::unread()
            )
            .expect_err("an unread reading is not opened by opening the class"),
            "`Load material into a deck` is closed by default where the deck it names is \
             live, and which decks are live was not read."
        );
    }

    /// **A group ADR-0235 closes past its four classes is refused and nothing
    /// opens it**, and the refusal says that rather than naming a bay that
    /// does not exist.
    #[test]
    fn a_group_with_no_bay_is_refused_and_no_opening_reaches_it() {
        let every = Open::CLOSED
            .with(Class::LiveDeck, true)
            .with(Class::MixFaders, true)
            .with(Class::MasterEffects, true)
            .with(Class::InputsAndOutputs, true);
        for (operation, group) in [
            (Operation::TapBeat, Unclassed::Clock),
            (Operation::Quit, Unclassed::Quitting),
            (Operation::SelectDeck { deck: 0 }, Unclassed::Selection),
            (Operation::PointLane { target: Undecided }, Unclassed::Lanes),
            (
                Operation::SetAuthority {
                    deck: 0,
                    node: node(),
                    authority: Authority::Automatic,
                },
                Unclassed::Authority,
            ),
        ] {
            assert_eq!(
                standing(&operation, Running::live(NOTHING_LIVE)),
                Standing::ClosedUnclassed(group)
            );
            let refused = audit(&operation, every, Running::live(NOTHING_LIVE))
                .expect_err("no opening reaches a group with no bay");
            assert_eq!(
                refused,
                format!(
                    "`{}` is closed by default with {}, and no bay opens that yet — which \
                     class it belongs to is not settled.",
                    operation.title(),
                    group.title()
                )
            );
        }
    }
}
