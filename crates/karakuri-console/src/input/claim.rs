use karakuri_layout::{Hit, Point};

use crate::panel::{Panel, GRAB};
use crate::view::View;

use super::probes::PROBES;

/// Who a pointer event belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    /// The panel's: a boundary is under the pointer, or one is in hand. `egui` is
    /// not told.
    Panel,
    /// `egui`'s.
    Egui,
}

/// Determines whether a pointer event at `p` belongs to the console panel or falls through to egui.
///
/// Solves layout on demand, checks active gestures and modal overlays, and tests boundaries
/// and registered controls via [`PROBES`] matching rendered frame state (ADR-0156).
pub fn claim(panel: &mut Panel, ctx: &egui::Context, view: &View, p: Point) -> Claim {
    // Rule 1, and it comes first: a gesture in progress is not re-decided from
    // where the pointer happens to be now.
    if panel.dragging() {
        return Claim::Panel;
    }
    panel.solve();
    // Rule 2: a menu, popup card, or modal chooser that is down is a hand mid-choice,
    // and every point of the console is part of that gesture until it is shut.
    // Unified across all 9 modal overlays via [`View::has_modal_overlay`].
    if view.has_modal_overlay() {
        return Claim::Panel;
    }
    // Rule 3 before rule 4: the boundary's first refusal is what the ordering
    // is, and the control clearing every grab is what stops it costing
    // anything. See the module documentation and `tests/outputs.rs`.
    match panel.layout().hit(p, GRAB) {
        Hit::Divider { .. } => Claim::Panel,
        // Rule 4: short-circuiting hit-test across registered [`PROBES`].
        Hit::View(_) | Hit::Nothing => {
            match PROBES.iter().any(|probe| (probe.ask)(panel, ctx, view, p)) {
                true => Claim::Panel,
                false => Claim::Egui,
            }
        }
    }
}
