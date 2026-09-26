use karakuri_layout::{Hit, Point};

use crate::panel::{Panel, GRAB};
use crate::view::View;

use super::probes::PROBES;

/// Target system for routing pointer events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    /// The console panel claims the event; egui is not notified.
    Panel,
    /// Routed to egui.
    Egui,
}

/// Determines whether a pointer event at `p` belongs to the console panel or falls through to egui.
///
/// Solves layout on demand, checks active gestures and modal overlays, and tests boundaries
/// and registered controls via [`PROBES`] matching rendered frame state (ADR-0156).
pub fn claim(panel: &mut Panel, ctx: &egui::Context, view: &View, p: Point) -> Claim {
    // Active drag gestures maintain claim regardless of cursor position.
    if panel.dragging() {
        return Claim::Panel;
    }
    panel.solve();
    // Modal overlays and popups claim all input until dismissed (ADR-0156).
    if view.has_modal_overlay() {
        return Claim::Panel;
    }
    // Layout boundaries take precedence over view controls.
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
