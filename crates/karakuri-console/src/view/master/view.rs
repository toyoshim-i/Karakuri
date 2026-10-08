//! Master bay helper methods on `View`.

use super::layout::master_with_state;
use super::types::MasterRow;
use crate::view::View;

impl View {
    /// Whether the master chain slot at `at` is folded/collapsed.
    pub fn is_master_folded(&self, at: u32) -> bool {
        self.master_folded.contains(&at)
    }

    /// Toggles the folded state of the master chain slot at `at`.
    pub fn toggle_master_fold(&mut self, at: u32) {
        if !self.master_folded.remove(&at) {
            self.master_folded.insert(at);
        }
    }

    /// Whether the master chain slot at `at` is muted.
    pub fn is_master_muted(&self, at: u32) -> bool {
        self.master_muted.contains(&at)
    }

    /// Toggles the muted state of the master chain slot at `at`.
    pub fn toggle_master_mute(&mut self, at: u32) {
        if !self.master_muted.remove(&at) {
            self.master_muted.insert(at);
        }
    }

    /// Whether the master chain slot at `at` is soloed.
    pub fn is_master_soloed(&self, at: u32) -> bool {
        self.master_soloed == Some(at)
    }

    /// Toggles the soloed state of the master chain slot at `at`.
    pub fn toggle_master_solo(&mut self, at: u32) {
        if self.master_soloed == Some(at) {
            self.master_soloed = None;
        } else {
            self.master_soloed = Some(at);
        }
    }

    /// Whether the master chain slot at `at` is online and active.
    pub fn is_master_online(&self, at: u32) -> bool {
        match self.master_soloed {
            Some(s) => s == at,
            None => !self.master_muted.contains(&at),
        }
    }

    /// Vertical scroll offset of the Master bay in logical pixels.
    pub fn master_scroll(&self) -> f32 {
        self.master_scroll
    }

    /// Scrolls the Master bay by `by` logical pixels, returning true if moved.
    pub fn scroll_master_by(&mut self, by: f32) -> bool {
        let prev = self.master_scroll;
        self.master_scroll = (prev + by).max(0.0);
        self.master_scroll != prev
    }

    /// Whether the inspector node at `(pane, node)` is folded/collapsed.
    pub fn is_node_folded(&self, pane: usize, node: usize) -> bool {
        self.inspector_folded.contains(&(pane, node))
    }

    /// Toggles the folded state of the inspector node at `(pane, node)`.
    pub fn toggle_node_fold(&mut self, pane: usize, node: usize) {
        if !self.inspector_folded.remove(&(pane, node)) {
            self.inspector_folded.insert((pane, node));
        }
        self.sync_inspector_folded();
    }

    /// Synchronizes the `folded` field on each `view::Node` with `self.inspector_folded`.
    pub fn sync_inspector_folded(&mut self) {
        for (pane_idx, pane) in self.inspector.iter_mut().enumerate() {
            for (node_idx, node) in pane.nodes.iter_mut().enumerate() {
                node.folded = self.inspector_folded.contains(&(pane_idx, node_idx));
            }
        }
    }

    /// Derives the master row using the active View state.
    pub fn master_row_layout(
        &self,
        ctx: &egui::Context,
        layout: &karakuri_layout::Layout,
    ) -> Option<MasterRow> {
        master_with_state(
            ctx,
            layout,
            self.master_out,
            self.master_chain.as_ref(),
            &self.chain_choices(),
            self.master_scroll,
            &self.master_folded,
            &self.master_muted,
            self.master_soloed,
            self.vr_mode,
            &self.vr_projection,
            self.vr_projection_folded,
        )
    }
}
