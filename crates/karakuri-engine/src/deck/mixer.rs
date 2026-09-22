use crate::transition::Control;

use super::types::{clamp_gain, clamp_opacity, Blend, DeckSlot, Mask, MaskKind};
use super::Deck;

impl Deck {
    pub fn gain(&self, slot: DeckSlot) -> f32 {
        self.slots[slot.index()].gain
    }

    /// Sets the linear colour gain for a slot, cancelling active gain transitions.
    pub fn set_gain(&mut self, slot: DeckSlot, gain: f32) {
        self.cancel(slot, Control::Gain);
        self.slots[slot.index()].gain = clamp_gain(gain);
    }

    pub fn opacity(&self, slot: DeckSlot) -> f32 {
        self.slots[slot.index()].opacity
    }

    /// Sets the blend opacity fader for a slot in `[0.0, 1.0]`, cancelling active opacity transitions.
    pub fn set_opacity(&mut self, slot: DeckSlot, opacity: f32) {
        self.cancel(slot, Control::Opacity);
        self.slots[slot.index()].opacity = clamp_opacity(opacity);
    }

    /// Recomputes online state for each slot based on solo and mute settings.
    fn arbitrate_mix(&mut self) {
        let solo = self.solo;
        for (i, slot) in self.slots.iter_mut().enumerate() {
            slot.online = match solo {
                Some(s) => i == s,
                None => !self.muted.get(i).copied().unwrap_or(false),
            };
        }
        self.revision = self.revision.wrapping_add(1);
    }

    /// Returns whether the specified slot is currently online in the composite mix.
    pub fn is_online(&self, slot: DeckSlot) -> bool {
        self.slots
            .get(slot.index())
            .map(|s| s.online)
            .unwrap_or(false)
    }

    /// Directly sets the online state of the specified slot.
    pub fn set_online(&mut self, slot: DeckSlot, online: bool) {
        if let Some(s) = self.slots.get_mut(slot.index()) {
            s.online = online;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Returns whether the specified slot is currently muted.
    pub fn is_muted(&self, slot: DeckSlot) -> bool {
        self.muted.get(slot.index()).copied().unwrap_or(false)
    }

    /// Sets the mute state of the specified slot and re-arbitrates mix.
    pub fn set_mute(&mut self, slot: DeckSlot, muted: bool) {
        if slot.index() < self.slots.len() {
            self.muted[slot.index()] = muted;
            self.arbitrate_mix();
        }
    }

    /// Toggles the mute state of the specified slot, returning the new state.
    pub fn toggle_mute(&mut self, slot: DeckSlot) -> bool {
        if slot.index() < self.slots.len() {
            let next = !self.is_muted(slot);
            self.set_mute(slot, next);
            next
        } else {
            false
        }
    }

    /// Returns the currently soloed slot index, if any.
    pub fn solo(&self) -> Option<usize> {
        self.solo
    }

    /// Returns whether any slot is currently soloed.
    pub fn any_soloed(&self) -> bool {
        self.solo.is_some()
    }

    /// Returns whether the specified slot is currently soloed.
    pub fn is_soloed(&self, slot: DeckSlot) -> bool {
        self.solo == Some(slot.index())
    }

    /// Sets the exclusive solo state of the specified slot and re-arbitrates mix.
    pub fn set_solo(&mut self, slot: DeckSlot, solo: bool) {
        if slot.index() < self.slots.len() {
            if solo {
                self.solo = Some(slot.index());
            } else if self.solo == Some(slot.index()) {
                self.solo = None;
            }
            self.arbitrate_mix();
        }
    }

    /// Toggles the exclusive solo state of the specified slot, returning the new state.
    pub fn toggle_solo(&mut self, slot: DeckSlot) -> bool {
        if slot.index() < self.slots.len() {
            let is_currently = self.is_soloed(slot);
            self.set_solo(slot, !is_currently);
            !is_currently
        } else {
            false
        }
    }

    /// Clears any active solo and restores each slot according to its mute setting.
    pub fn clear_solo(&mut self) {
        if self.solo.is_some() {
            self.solo = None;
            self.arbitrate_mix();
        }
    }

    /// Current revision counter for mixer state changes (solo/mute/online).
    pub fn mixer_revision(&self) -> u64 {
        self.revision
    }

    /// Evaluates whether a slot contributes to the final composite mix.
    ///
    /// A slot is in the mix if it is online and has positive opacity.
    pub fn is_in_mix(&self, slot: DeckSlot) -> bool {
        if let Some(s) = self.slots.get(slot.index()) {
            s.online && s.opacity > 0.0
        } else {
            false
        }
    }

    pub fn out(&self) -> f32 {
        self.out
    }

    /// Sets the master output scaling level applied to the composited mix.
    pub fn set_out(&mut self, out: f32) {
        self.out = clamp_gain(out);
    }

    pub fn blend(&self, slot: DeckSlot) -> Blend {
        self.slots[slot.index()].blend
    }

    pub fn mask(&self, slot: DeckSlot) -> Mask {
        self.slots[slot.index()].mask
    }

    /// Sets the mask shape geometry and angle without altering its current reveal position.
    pub fn set_mask_shape(&mut self, slot: DeckSlot, kind: MaskKind, angle: f32) {
        let mask = self.slots[slot.index()].mask;
        self.slots[slot.index()].mask = Mask::new(kind, angle, mask.position(), mask.softness());
    }

    /// Sets the mask reveal position, cancelling any active mask position transitions.
    pub fn set_mask_position(&mut self, slot: DeckSlot, position: f32) {
        self.cancel(slot, Control::MaskPosition);
        let mask = self.slots[slot.index()].mask;
        self.slots[slot.index()].mask = mask.at(position);
    }

    /// Sets the full mask specification for a slot, cancelling active mask position transitions.
    pub fn set_mask(&mut self, slot: DeckSlot, mask: Mask) {
        self.set_mask_shape(slot, mask.kind(), mask.angle());
        self.set_mask_position(slot, mask.position());
        let at = self.slots[slot.index()].mask;
        self.slots[slot.index()].mask =
            Mask::new(at.kind(), at.angle(), at.position(), mask.softness());
    }

    /// Sets the blend mode for the specified slot.
    pub fn set_blend(&mut self, slot: DeckSlot, blend: Blend) {
        self.slots[slot.index()].blend = blend;
    }
}
