//! Stereo rendering and VR eye dispatch for multi-view projections (ADR-0246).

use super::{Deck, DeckSlot};

impl Deck {
    fn active_stereo_set_mut(
        &mut self,
        target_slot: Option<DeckSlot>,
    ) -> Option<&mut crate::set::Set> {
        let active_slot = target_slot.or_else(|| {
            (0..self.slots.len())
                .map(|i| DeckSlot(i as u8))
                .find(|&slot| self.is_in_mix(slot) && !self.overloaded(slot))
        })?;
        let active_idx = active_slot.index();
        let slot = self.slots.get_mut(active_idx)?;
        Some(slot.swap.live_mut())
    }

    /// The viewport size for stereo passes. This matches the active Set's
    /// viewport so that the offscreen intermediate passes (which draw into
    /// `stereo_l5_input`) are allocated at the exact size the downstream
    /// weighted-transparency and merge targets are, and a render pass whose
    /// attachments differ in size is a validation error.
    pub fn stereo_viewport(&mut self, target_slot: Option<DeckSlot>) -> Option<(u32, u32)> {
        self.active_stereo_set_mut(target_slot)
            .map(|set| set.viewport())
            .or_else(|| (self.width > 0 && self.height > 0).then_some((self.width, self.height)))
    }

    /// Returns the deck's configured surface dimensions `(width, height)`.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Records rasterization pass for a single stereo eye into a specified viewport.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_stereo_eye(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        target_slot: Option<DeckSlot>,
        matrices: &crate::camera::StereoMatrices,
        first: bool,
        viewport: (f32, f32, f32, f32),
    ) {
        if let Some(set) = self.active_stereo_set_mut(target_slot) {
            set.draw_stereo_eye(
                queue, encoder, target, depth_view, matrices, first, viewport,
            );
        }
    }

    /// The L5 input of the Set [`Deck::draw_stereo_eye`] draws, or `None`
    /// where it has no L5s; see [`crate::set::Set::l5_input`].
    pub fn stereo_l5_input(&mut self, target_slot: Option<DeckSlot>) -> Option<&wgpu::TextureView> {
        self.active_stereo_set_mut(target_slot)?.l5_input()
    }

    /// Runs that Set's L5s for a stereo eye; see
    /// [`crate::set::Set::record_l5s_unheld`].
    pub fn record_stereo_l5s(
        &mut self,
        target_slot: Option<DeckSlot>,
        encoder: &mut wgpu::CommandEncoder,
        out: &wgpu::TextureView,
    ) {
        if let Some(set) = self.active_stereo_set_mut(target_slot) {
            set.record_l5s_unheld(encoder, out);
        }
    }

    /// Returns true if the active stereo set in the deck mix is purely Fullscreen topology.
    pub fn is_fullscreen_stereo(&self) -> bool {
        let active_slots: Vec<DeckSlot> = (0..self.slots.len())
            .map(|i| DeckSlot(i as u8))
            .filter(|&slot| self.is_in_mix(slot) && !self.overloaded(slot))
            .collect();
        if active_slots.is_empty() {
            let Some(first) = self.slots.first() else {
                return false;
            };
            let topos = first.swap.live().drawn_topologies();
            return !topos.is_empty()
                && topos
                    .iter()
                    .all(|&t| t == karakuri_ir::Topology::Fullscreen);
        }
        let mut has_fullscreen = false;
        for &slot in &active_slots {
            let topos = self.slot(slot).live().drawn_topologies();
            if topos.is_empty() {
                continue;
            }
            for topo in topos {
                if topo == karakuri_ir::Topology::Fullscreen {
                    has_fullscreen = true;
                } else {
                    return false;
                }
            }
        }
        has_fullscreen
    }
}
