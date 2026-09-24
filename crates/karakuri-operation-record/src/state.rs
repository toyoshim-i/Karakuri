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
    /// Returns slots with `param` updated at index `at`, or `None` if `at` is out of bounds.
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

/// Mapping of active pattern lanes to the controls they drive (in lane order).
///
/// Muted lanes are excluded.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lanes {
    /// Every lane holding a control, in lane order.
    pub held: Vec<(usize, karakuri_operation::LaneTarget)>,
}

impl Lanes {
    /// Returns the index of the lane holding `target`, or `None` if unheld (ADR-0323).
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
    /// Active pattern lanes holding controls; `None` when sequencer state is unread (ADR-0323).
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

/// Refusal emitted when a scheduled transition targets a control held by an active pattern lane (ADR-0323).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refusal {
    /// The lane holding the control, indexed in the armed pattern's lane order.
    pub lane: usize,
    /// The deck whose channel fader that lane drives.
    pub deck: u8,
}

impl Refusal {
    /// Standardized refusal sentence naming the conflicting deck and lane (P-0083, ADR-0131).
    pub fn why(&self) -> String {
        let Refusal { lane, deck } = *self;
        format!(
            "deck {deck}'s fader is held by lane {lane} of the armed pattern: mute that lane \
             and ask again"
        )
    }

    /// Structured representation of this refusal adhering to Principle P-0083 and ADR-0131.
    pub fn detail(&self) -> karakuri_operation::RefusalDetail {
        karakuri_operation::RefusalDetail::lane_held(self.deck, self.lane)
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
    /// Operation is refused by policy; no record is written (ADR-0323).
    Refused(Refusal),
}

/// Standardized diagnostic sentence returned across surfaces when an operation produces no record (ADR-0131, P-0083).
pub fn not_performed(title: &str, why: &str) -> String {
    format!("`{title}` was not performed and nothing on this run changed: {why}")
}
