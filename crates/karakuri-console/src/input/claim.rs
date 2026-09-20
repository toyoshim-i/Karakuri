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

/// Who gets a pointer event at `p` — see the module documentation for the rule
/// and for why it is written there.
///
/// Takes `&mut Panel` for the solve alone: [`Layout::hit`] refuses to answer
/// from a dirty layout, and on a frame where nothing moved the solve is a flag
/// test.
///
/// Takes the `egui` context to ask where the console's controls are. Rule 4 is
/// about a painted chip whose width is the width of the name in it, so
/// answering it means laying that name out — which is `egui`'s to do and nobody
/// else's, since it is `egui` that will paint the same run. It is a cached
/// galley lookup per event, and before the first frame there are no fonts and
/// no drawn control at all, which [`crate::view::outputs`] answers `None` to.
///
/// What the mixer adds to that is two more galley lookups per strip, for the
/// tally's word and the blend's, because those are what a strip's boxes are
/// laid out around. It is paid on a pointer event and not on a frame, and a
/// console with no deck behind it pays nothing at all — [`mixer`] answers
/// `None` to an empty slice before it asks for any type. The chips cost none of
/// that again: the bay is derived once and all five of the mixer's controls are
/// asked of it — and neither the mask mini nor the strip adds a lookup of its
/// own, because one holds a mark rather than a word and the other is the box
/// the words were laid out into.
///
/// A control in a bay that is not laid out is never reached, and it is
/// [`mixer`] that answers so rather than a check here: a folded mixer — or one
/// inside a folded pane — has no room for its row of strips, `strips_row`
/// answers `None`, and the bay is `None` before any chip is hit-tested.
/// `tests/tally.rs` asserts it both ways round, and the strip itself goes with
/// them: a bay that is not laid out has no rectangle to select a deck by
/// either. [`inspector`] is the same answer one bay along, and [`deck_head`]
/// adds a second: a pane too narrow to hold its own chips draws none, so there
/// is nothing there to press.
///
/// What the deck head adds is three galley lookups per pane, for the mode's
/// word, the anchor's numbers and the fold's — the same arrangement the mixer's
/// are in, and paid on a pointer event rather than on a frame. The scrub's two
/// arrows add none of their own: they are marks rather than words, which is
/// what the mask mini already saves one bay up.
///
/// And it takes the whole [`View`], for the same reason one level further out.
/// A fader's knob sits on the fill's moving edge and the arrangement pill is as
/// wide as the name in it, so *where a control is* depends on what the deck and
/// the store said this frame — and this crate has neither (ADR-0156), so the
/// values arrive the way they arrive everywhere else here: handed in by whoever
/// owns them.
///
/// The view rather than the four fields out of it, which is a change from when
/// this took the strips alone. It is not convenience: rule 4 hit-tests exactly
/// what [`View::draw`] painted, and a caller that passed the strips from one
/// frame and the arrangement from another could put the two out of step with
/// nothing failing to compile. One argument is one frame's answer. A
/// [`View::new`] nobody has written to is a console with no deck and no store
/// behind it — no strips, no knobs, the default arrangement and a shut menu —
/// and it is what every test in this crate that is not about a control passes.
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
        // Rule 4, over all [`CONTROLS`] of the console's controls, one row
        // of [`PROBES`] at a time. Each is asked the same way — the
        // derivation that draws it, asked whether the point is on it — and no
        // answer is stored. The walk short-circuits exactly as the chain of
        // `||` it replaced did, so a press on the sink still costs one galley
        // lookup.
        Hit::View(_) | Hit::Nothing => {
            match PROBES.iter().any(|probe| (probe.ask)(panel, ctx, view, p)) {
                true => Claim::Panel,
                false => Claim::Egui,
            }
        }
    }
}
