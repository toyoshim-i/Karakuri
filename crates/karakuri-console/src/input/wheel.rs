use karakuri_layout::Point;

use crate::panel::Panel;
use crate::view::{inspector, library, View};

/// Returns which region a mouse wheel at `p` scrolls, or `None` if outside scrollable areas.
///
/// Routed by region (Library bay or Inspector pane, ADR-0312) rather than by control.
/// Tests the full bay boundary using the current scrolled layout.
pub fn wheeled(panel: &mut Panel, view: &View, p: Point) -> Option<Turned> {
    // Rule 1: a gesture in progress is not re-decided, and a wheel is not part
    // of it. `claim` gives the event to the panel either way; what this says
    // is that nothing scrolls.
    if panel.dragging() {
        return None;
    }
    // Rule 2, the same cards and choosers as [`claim`]: a hand
    // mid-choice is not a hand on a pane.
    if view.has_modal_overlay() {
        return None;
    }
    panel.solve();
    let at = egui::Pos2::new(p.x, p.y);
    if library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
    .is_some_and(|bay| bay.bay.contains(at))
    {
        return Some(Turned::Library);
    }
    view.inspector.iter().enumerate().find_map(|(index, pane)| {
        let laid = inspector(panel.layout(), index, pane, view.scroll_in(index))?;
        laid.head
            .union(laid.deck_head)
            .union(laid.body)
            .contains(at)
            .then_some(Turned::Pane(index))
    })
}

/// Identifies which region a mouse wheel event targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turned {
    /// The `index`th Inspector pane — [`crate::view::View::scroll_by`].
    Pane(usize),
    /// The Library bay's listing — [`crate::view::View::scroll_library_by`].
    Library,
}
