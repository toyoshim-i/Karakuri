//! Deck configuration, sequencer lane targets, slot policies, and transport modes.

use super::address::{ParamAt, ParamValue};
use super::layer::Layer;
use super::refusal::{RefusalCode, RefusalDetail};
use crate::op::Operation;

/// Filters applied to library listings across procedure kinds and Sets.
///
/// Used as the payload for [`Operation::FilterLibrary`]. Evaluated as an OR
/// across all enabled filter flags; if none are enabled, all kinds match.
/// See ADR-0338.
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
    /// A match rather than a table, exactly as [`BlendMode::name`](super::mix::BlendMode::name) is one and for
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

/// Subdivision value for one step of a sequencer pattern: sixteenth or eighth.
///
/// Pattern length is one fixed bar; step count directly corresponds to mode
/// (16 steps for sixteenth, 8 steps for eighth). Stored pattern width remains 16 slots.
/// See ADR-0306 and ADR-0320.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StepMode {
    /// Sixteen steps to the bar, four to the beat. Default mode.
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
    /// [`BlendMode::name`](super::mix::BlendMode::name) are and for their reason: a mode added to this enum does
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
    /// A list is not a cycle, exactly as [`BlendMode::ALL`](super::mix::BlendMode::ALL) is not: what a surface
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
    /// A match rather than a table, for [`BlendMode::name`](super::mix::BlendMode::name)'s reason.
    pub fn name(self) -> &'static str {
        match self {
            Residency::Live => "live",
            Residency::Priming => "priming",
            Residency::Allocated => "allocated",
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
    /// Returns the lowercase representation of this authority level,
    /// matching `karakuri_store::record::Record::Authority`.
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
                deck: u8::try_from(slot).ok(),
                lane: None,
                class: None,
                policy: Some(self.policy),
                in_mix: Some(self.in_mix),
            }),
            SlotPolicy::Off => Some(RefusalDetail {
                code: RefusalCode::SlotPolicyOff,
                message: format!(
                    "slot {slot} is locked against MCP modifications under `off` policy"
                ),
                slot: Some(slot),
                deck: u8::try_from(slot).ok(),
                lane: None,
                class: None,
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
