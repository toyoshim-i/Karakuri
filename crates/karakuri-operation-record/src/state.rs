//! Engine and surface state snapshots, readings, and outcome classifications.

use karakuri_store::record::{ChainSlot, Record};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub tonemap: karakuri_operation::Tonemap,
    pub exposure: f32,
    /// White point value required by [`Record::Look`].
    pub white_point: f32,
}

/// The master chain that is running, in record order: the reading every chain
/// operation is completed into a whole [`Record::MasterChain`] from.
///
/// One field: `Record::MasterChain` carries the slot list and nothing else.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Chain {
    /// Active chain slots in record order.
    pub slots: Vec<karakuri_store::record::ChainSlot>,
}

impl Chain {
    /// The slots with one of them set, or `None` where `at` is not a slot of this
    /// chain.
    ///
    /// A parameter the slot's procedure does not declare, and a cut on a slot
    /// whose procedure declares no `retains`, are refused where the chain is
    /// built rather than here: this crate resolves no procedure.
    pub(crate) fn set(
        &self,
        at: u32,
        param: &karakuri_operation::ChainParam,
    ) -> Option<Vec<ChainSlot>> {
        let mut slots = self.slots.clone();
        let slot = slots.get_mut(at as usize)?;
        match param {
            karakuri_operation::ChainParam::Declared { key, value } => {
                slot.params.insert(key.clone(), *value);
            }
            karakuri_operation::ChainParam::Cut(cut) => {
                slot.cut = Some(cut.name().to_string());
            }
        }
        Some(slots)
    }

    /// The slots with one more at the end.
    pub(crate) fn added(
        &self,
        procedure: &str,
        cut: Option<karakuri_operation::Cut>,
    ) -> Vec<ChainSlot> {
        let mut slots = self.slots.clone();
        slots.push(ChainSlot {
            procedure: procedure.to_string(),
            cut: cut.map(|cut| cut.name().to_string()),
            params: Default::default(),
        });
        slots
    }

    /// The slots with one taken out, or `None` where `at` is not a slot of this
    /// chain.
    pub(crate) fn removed(&self, at: u32) -> Option<Vec<ChainSlot>> {
        let at = at as usize;
        (at < self.slots.len()).then(|| {
            let mut slots = self.slots.clone();
            slots.remove(at);
            slots
        })
    }
}

/// Deck mask parameters used to complete mask shape and position records.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mask {
    pub kind: karakuri_operation::WipeKind,
    /// Linear front angle in radians.
    pub angle: f32,
    /// Front position in range `[0.0, 1.0]`.
    pub position: f32,
    /// Softness value required by [`Record::Mask`].
    pub softness: f32,
}

/// Deck transport state used to complete scrub records.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    pub sync: karakuri_operation::Sync,
    pub anchor_bpm: f32,
    /// Scrub offset in beats.
    pub scrub_beats: f64,
}

/// Transition parameters governing scheduled fades, crossfades, and wipes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    /// Instant the transition lands on, in absolute session beats.
    pub start: f64,
    /// Duration of the transition in beats (zero represents an immediate cut).
    pub beats: f64,
    /// Easing curve applied to the transition.
    pub curve: karakuri_operation::Curve,
    /// Shape kind for wipe fronts.
    pub wipe_kind: karakuri_operation::WipeKind,
    /// Direction angle of linear wipe fronts in radians.
    pub wipe_angle: f32,
}

/// Which lanes of the armed pattern hold which controls: one entry per lane
/// that holds one, as `(lane index, target)` pairs in lane order.
///
/// A muted lane holds nothing and is not in this list — that is applied where
/// the pattern is read (`karakuri_pattern::Pattern::held`), so a muted lane
/// arrives here as an absent one. The index is the lane's own, because the
/// refusal built from this reading names the lane to mute
/// (`docs/adr/0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lanes {
    /// Every lane holding a control, in lane order.
    pub held: Vec<(usize, karakuri_operation::LaneTarget)>,
}

impl Lanes {
    /// The index of the lane holding `target`, or `None` where none does.
    ///
    /// The one answer to *which lane holds this control* (ADR-0323): a lane's
    /// target is the control's address — deck and what on it — so holding is
    /// equality against what was read, and no surface derives it a second way.
    pub fn holder(&self, target: &karakuri_operation::LaneTarget) -> Option<usize> {
        self.held
            .iter()
            .find(|(_, held)| held == target)
            .map(|(at, _)| *at)
    }
}

/// Deck blend and residency state in the engine mix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mix {
    /// Blend mode of the deck relative to underlying layers.
    pub blend: karakuri_operation::BlendMode,
    /// Deck residency status in the active mix.
    pub residency: karakuri_operation::Residency,
}

/// Snapshot of active engine and surface state when an operation arrives.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Current {
    pub look: Option<Look>,
    /// Active master chain configuration.
    pub master_chain: Option<Chain>,
    pub transport: Option<Transport>,
    pub mask: Option<Mask>,
    /// Session tempo in BPM used to anchor sync mode changes.
    pub tempo: Option<f32>,
    /// Transition timing and shape settings for scheduled moves.
    pub transition: Option<Transition>,
    /// Mix status of the deck arriving in a transition.
    pub mix: Option<Mix>,
    /// Which lanes of the armed pattern hold which controls.
    ///
    /// `None` is a reading that was not taken: nothing is held, nothing is
    /// refused, and every scheduling operation writes the records it writes with
    /// no sequencer in the room. A surface that runs one hands this in
    /// (ADR-0323).
    pub lanes: Option<Lanes>,
}

/// Identifies a specific reading required by an operation when missing from [`Current`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// [`Current::look`].
    Look,
    /// [`Current::master_chain`].
    MasterChain,
    /// [`Current::transport`].
    Transport,
    /// [`Current::mask`].
    Mask,
    /// [`Current::tempo`].
    Tempo,
    /// [`Current::transition`].
    Transition,
    /// [`Current::mix`].
    Mix,
}

/// Rationale for why an operation intentionally produces no journal record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Silent {
    /// Modifies surface-local UI state only (e.g. selection, folding).
    Surface,
    /// Read-only query changing no session state.
    Question,
    /// Record is emitted asynchronously when work lands (e.g. disk saves, hotswap).
    OnLanding,
    /// Expressed in project or set files rather than session journal streams.
    NoRecord,
    /// Configures external display sinks without affecting internal render state.
    Published,
}

impl Silent {
    /// Returns a human-readable explanation for why the operation writes no record.
    pub fn why(self) -> &'static str {
        match self {
            Silent::Surface => "it is a surface's own state and writes no record",
            Silent::Question => "it asks rather than changes, and a question writes no record",
            Silent::OnLanding => {
                "its record is written where the work lands, not where it was asked for"
            }
            Silent::NoRecord => "nothing in the session record vocabulary carries it",
            Silent::Published => {
                "it says where a frame goes, and where a frame goes is not part of the frame"
            }
        }
    }
}

/// Explains why an operation cannot currently be converted into a journal record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owed {
    /// Missing required context snapshot in [`Current`].
    NotRead(Reading),
    /// The chain that is running has no slot at the position the operation named.
    NotInChain,
    /// Operation semantics are undecided in the vocabulary definition.
    Undecided,
    /// Operation requires engine-level arithmetic or beat tracking not provided in [`Current`].
    NotSettled,
}

impl Owed {
    /// Returns an explanatory diagnostic describing the missing requirement.
    pub fn why(self) -> &'static str {
        match self {
            Owed::NotRead(Reading::Look) => "the look that is running was not read",
            Owed::NotRead(Reading::MasterChain) => {
                "the master chain that is running was not read"
            }
            Owed::NotRead(Reading::Transport) => {
                "the transport of the deck it names was not read"
            }
            Owed::NotRead(Reading::Mask) => "the mask of the deck it names was not read",
            Owed::NotRead(Reading::Tempo) => "the session tempo its anchor comes from was not read",
            Owed::NotRead(Reading::Transition) => {
                "the transition settings its move is scheduled by were not handed over"
            }
            Owed::NotRead(Reading::Mix) => {
                "the blend mode and residency of the deck it names were not read"
            }
            Owed::NotInChain => "the master chain that is running has no slot at that position",
            Owed::Undecided => "what it acts on is an open question in the vocabulary itself",
            Owed::NotSettled => {
                "the record it writes is not a function of values alone, and who supplies the rest is undecided"
            }
        }
    }
}

/// A scheduled move refused because a lane of the armed pattern holds the
/// control it would move.
///
/// The refusal is taken here, before any record is written, and that is the
/// whole of it: a record written live would be replayed by a run with no
/// sequencer in it, where nothing holds the control and the move runs
/// (`docs/adr/0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md`,
/// ADR-0322, P-0092).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refusal {
    /// The lane holding the control, indexed in the armed pattern's lane order.
    pub lane: usize,
    /// The deck whose channel fader that lane drives.
    pub deck: u8,
}

impl Refusal {
    /// The one sentence every surface refuses in, naming the lane to mute.
    ///
    /// One wording, in one place, so the pointer, the keys, a mapped control
    /// and a model meet the same sentence (ADR-0131). It carries what the next
    /// attempt needs — mute that lane and ask again — which is one press on the
    /// lane's label (P-0083). Decks and lanes are counted from zero, as the
    /// vocabulary counts them.
    pub fn why(&self) -> String {
        let Refusal { lane, deck } = *self;
        format!(
            "deck {deck}'s fader is held by lane {lane} of the armed pattern: mute that lane \
             and ask again"
        )
    }
}

/// Translation outcome of converting an [`Operation`] into journal records.
#[derive(Debug, Clone, PartialEq)]
pub enum Written {
    /// Emits one or more journal records to append to the session history in sequence.
    Records(Vec<Record>),
    /// Operation intentionally produces no record.
    Silent(Silent),
    /// Operation cannot produce a record given the current context.
    Owed(Owed),
    /// Operation is refused by decision, and no record is written.
    ///
    /// Not an [`Written::Owed`]: a gap nobody has closed and a decision taken
    /// are two different answers, and a surface says them in two different
    /// sentences (ADR-0323).
    Refused(Refusal),
}

/// The one sentence a surface answers an operation that wrote no record in.
///
/// `title` is [`Operation::title`] and `why` is the reason in the words the
/// type that decided it says them in — [`Refusal::why`], [`Owed::why`] or
/// [`Silent::why`] — so nothing here is written down twice and a caller cannot
/// invent a reason of its own.
///
/// `karakuri_environment::no_such_slot`'s arrangement, one question along,
/// and for its reason (ADR-0131): the instrument and
/// `karakuri-cli` both hand a model an answer through `--mcp`, and two
/// spellings of *nothing happened, and here is why* is one mistake explained
/// twice. It is stated once, here, where the three reasons already live, and
/// each surface pins it with an `assert_eq!` rather than trusting a comment.
///
/// What the next attempt needs is in `why` and not in this wrapper
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)):
/// a lane to mute, a reading nobody handed over, a question the vocabulary has
/// not settled. What this adds is the half a model cannot see — that the ask
/// reached a performer and the run is where it was (ADR-0315).
///
/// A surface may follow it with what is true of that surface alone;
/// `karakuri-cli` does, for a [`Written::Silent`] it has no control for. None
/// of them may respell it.
pub fn not_performed(title: &str, why: &str) -> String {
    format!("`{title}` was not performed and nothing on this run changed: {why}")
}
