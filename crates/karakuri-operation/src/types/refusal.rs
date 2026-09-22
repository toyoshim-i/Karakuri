//! Structured error and refusal types for agent and gate operations.

use super::deck::SlotPolicy;

/// Machine-readable refusal code for agent operation rejections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    /// Slot is currently active in the live mix under Auto policy.
    SlotInMix,
    /// Slot policy is set to Off (locked against MCP modifications).
    SlotPolicyOff,
    /// Slot is unallocated or does not exist.
    SlotUnallocated,
    /// Bay is closed to MCP operations.
    BayClosed,
    /// Control is currently held by an active sequencer lane.
    LaneHeld,
}

impl RefusalCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            RefusalCode::SlotInMix => "SLOT_IN_MIX",
            RefusalCode::SlotPolicyOff => "SLOT_POLICY_OFF",
            RefusalCode::SlotUnallocated => "SLOT_UNALLOCATED",
            RefusalCode::BayClosed => "BAY_CLOSED",
            RefusalCode::LaneHeld => "LANE_HELD",
        }
    }
}

impl std::fmt::Display for RefusalCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured refusal details describing why an agent operation was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalDetail {
    pub code: RefusalCode,
    pub message: String,
    pub slot: Option<usize>,
    pub deck: Option<u8>,
    pub lane: Option<usize>,
    pub class: Option<crate::gate::Class>,
    pub policy: Option<SlotPolicy>,
    pub in_mix: Option<bool>,
}

impl RefusalDetail {
    /// Constructs a refusal representing a control currently held by a sequencer lane.
    pub fn lane_held(deck: u8, lane: usize) -> Self {
        Self {
            code: RefusalCode::LaneHeld,
            message: format!(
                "deck {deck}'s fader is held by lane {lane} of the armed pattern: mute that lane and ask again"
            ),
            slot: Some(deck as usize),
            deck: Some(deck),
            lane: Some(lane),
            class: None,
            policy: None,
            in_mix: None,
        }
    }

    /// Constructs a refusal representing an unallocated or invalid slot.
    pub fn slot_unallocated(slot: usize, message: String) -> Self {
        Self {
            code: RefusalCode::SlotUnallocated,
            message,
            slot: Some(slot),
            deck: u8::try_from(slot).ok(),
            lane: None,
            class: None,
            policy: None,
            in_mix: None,
        }
    }
}

impl std::fmt::Display for RefusalDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for RefusalDetail {}
