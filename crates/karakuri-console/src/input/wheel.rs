use karakuri_layout::Point;

use crate::panel::Panel;
use crate::view::{inspector, library, View};

/// What a wheel at `p` turns, or `None` where the wheel belongs to nobody here.
///
/// See the module documentation's *The wheel is routed by region and not by
/// control* for why this is not a row of [`PROBES`]. The caller's whole job
/// with the answer is the matching `scroll` call on [`crate::view::View`], and
/// to treat the event as the panel's — a wheel this answers `None` to goes to
/// `egui` exactly as it did before there was anything to scroll.
///
/// Two regions scroll and this is the one place that says which. It answered an
/// Inspector pane index until 2026-09-09 and answers a [`Turned`] now, because
/// the Library bay scrolls as well
/// ([ADR-0312](../../../docs/adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md)).
/// The Library is asked first, and the order cannot matter: the two bays are
/// two leaves of the arrangement and no point is inside both.
///
/// The whole bay and not its body, for either of them. The wheel is aimed with
/// the pointer and a region's heads are part of the thing being scrolled, so a
/// hand resting over a deck head turns the list under it and a hand over the
/// scope chips turns the listing under them — which is what
/// `docs/manual/console.html` says: *"the wheel over the pane"*, and *"the
/// wheel over the bay"*.
///
/// The derivation that draws the region is the one that answers this, which is
/// rule 4's own arrangement: [`inspector`] and [`library`] are each asked with
/// the position that region is scrolled to, so the rectangle a wheel is
/// measured against is the rectangle the frame drew.
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

/// Which region a wheel turns, where one turns anything at all.
///
/// A sum rather than an index, because the two things that scroll are not two
/// of a kind: an Inspector pane is one of [`crate::view::PANES`] and is named
/// by its index, and the Library bay is the only one of itself. [`Claim`]'s
/// shape one question along — the caller does one thing with each arm and
/// nothing with `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turned {
    /// The `index`th Inspector pane — [`crate::view::View::scroll_by`].
    Pane(usize),
    /// The Library bay's listing — [`crate::view::View::scroll_library_by`].
    Library,
}
