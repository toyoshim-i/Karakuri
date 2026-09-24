//! Deck configuration, sequencer lane targets, slot policies, and transport modes.

use super::address::{ParamAt, ParamValue};
use super::layer::Layer;
use super::refusal::{RefusalCode, RefusalDetail};
use crate::op::Operation;

/// Filters applied to library listings across procedure kinds and Sets.
///
/// Used as the payload for [`Operation::FilterLibrary`]. Evaluated as an OR
/// across all enabled filter flags; if none are enabled, all kinds match (ADR-0338).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryKinds {
    pub l1: bool,
    pub l2: bool,
    pub l3: bool,
    pub l4: bool,
    pub field: bool,
    /// L5 frame effect procedure filter flag (ADR-0340).
    pub l5: bool,
    /// Sets filter flag.
    pub sets: bool,
}

impl LibraryKinds {
    /// Nothing narrowed; matches all kinds.
    pub const EVERYTHING: LibraryKinds = LibraryKinds {
        l1: false,
        l2: false,
        l3: false,
        l4: false,
        field: false,
        l5: false,
        sets: false,
    };

    /// Returns `true` if any filter flag is enabled.
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
    /// The lowercase wire name for this mode (`free`, `tempo`, `beat`).
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
/// (16 steps for sixteenth, 8 steps for eighth). Stored pattern width remains 16 slots
/// (ADR-0306, ADR-0320).
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

    /// The display label on the sequencer pill (`1/16` or `1/8`).
    pub fn name(self) -> &'static str {
        match self {
            StepMode::Sixteenth => "1/16",
            StepMode::Eighth => "1/8",
        }
    }

    /// How many steps there are in the bar at this mode (16 or 8, ADR-0306).
    pub fn count(self) -> usize {
        match self {
            StepMode::Sixteenth => 16,
            StepMode::Eighth => 8,
        }
    }

    /// The multiplier in the step index (`floor(beats * steps_per_beat) % count`, ADR-0222).
    pub fn steps_per_beat(self) -> f64 {
        match self {
            StepMode::Sixteenth => 4.0,
            StepMode::Eighth => 2.0,
        }
    }

    /// Maps step index to the corresponding 16-slot storage index (ADR-0320).
    pub fn slot_of(self, step: usize) -> usize {
        match self {
            StepMode::Sixteenth => step,
            StepMode::Eighth => step * 2,
        }
    }
}

/// The target of a sequencer lane: an operation address eliding its value (ADR-0222, ADR-0321).
#[derive(Debug, Clone, PartialEq)]
pub enum LaneTarget {
    /// A deck's channel fader (opacity).
    Fader { deck: u8 },
    /// A scalar parameter inside the Set on a deck (ADR-0268).
    Param { deck: u8, param: ParamAt },
}

impl LaneTarget {
    /// Converts this target and a step value into the corresponding [`Operation`].
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

    /// The deck index written to by this lane (ADR-0323).
    pub fn deck(&self) -> u8 {
        match self {
            LaneTarget::Fader { deck } | LaneTarget::Param { deck, .. } => *deck,
        }
    }
}

/// Target residency state requested for a deck slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Residency {
    /// Actively stepped and composited into the mix.
    Live,
    /// Stepped off-air with buffers warm, ready to transition immediately.
    Priming,
    /// Compiled with buffers allocated, but clock paused.
    Allocated,
}

impl Residency {
    /// All available residency levels ordered by resource cost.
    pub const ALL: [Residency; 3] = [Residency::Live, Residency::Priming, Residency::Allocated];

    /// Lowercase wire name matching `karakuri_engine::deck::Residency`.
    pub fn name(self) -> &'static str {
        match self {
            Residency::Live => "live",
            Residency::Priming => "priming",
            Residency::Allocated => "allocated",
        }
    }
}

/// Access authority for modifying a Set node (ADR-0211).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authority {
    /// Only explicit operator actions may modify this node.
    Manual,
    /// Agents may propose values requiring operator confirmation.
    Suggesting,
    /// Autonomous agents may directly modify values.
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

/// Audio input or external process providing clock tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeatSource {
    /// Audio input device name or `default` (`--audio-in`).
    AudioInput(String),
    /// Child process reporting beat events (`--tempo-source`).
    Process(String),
}
