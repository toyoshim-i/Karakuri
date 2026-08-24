//! **Who gets a pointer event: the panel's boundaries, or `egui`.**
//!
//! The dividers are ours. [`Panel::press`], [`Panel::moved`] and
//! [`Panel::released`] move them over [`Layout::hit`], and `egui` has no
//! widget in those gaps — they are the ground showing between two bays, and
//! nothing is drawn there to be clicked. So on the face of it there is no
//! conflict, and the two would never both react to the same click.
//!
//! That is exactly why the rule is written down here rather than left to be
//! re-derived. It is true only while the gaps stay empty. [`GRAB`] widens
//! every boundary by six pixels either side, because a nine-pixel gap is not a
//! target a hand finds — and those twelve pixels are *inside the bays*, over
//! whatever the bay draws at its edge. The first control placed near a bay's
//! edge is under a boundary's grab, and then both do think they are dragging.
//!
//! # The rule
//!
//! **The boundary gets first refusal.**
//!
//! 1. **A drag in hand keeps its claim**, wherever the pointer has wandered
//!    to. A drag is a gesture and not a position: a boundary held against a
//!    stop while the pointer runs on across three bays is the ordinary case,
//!    not the odd one, and a claim re-decided from the pointer each event
//!    would hand the middle of that gesture to `egui`.
//! 2. Otherwise, if the pointer is within [`GRAB`] of a boundary, it is the
//!    panel's and `egui` does not see the event.
//! 3. Otherwise it goes to
//!    [`egui_winit::State::on_window_event`](https://docs.rs/egui-winit) and
//!    `egui` decides.
//!
//! # One exception, and it is not a hole in the rule
//!
//! **The panel always learns where the pointer is**, whoever the event is
//! claimed by. That is not the panel *acting* on the event: every keyboard
//! operation is addressed to whatever the pointer is over — fold *this*,
//! solo *this* — so [`Panel::cursor`] is state the panel needs whether or not
//! it is dragging, and a panel that only tracked the pointer over its own
//! boundaries would fold the wrong region the moment the pointer was anywhere
//! useful.
//!
//! So motion updates [`Panel::cursor`] on every event and [`Claim`] decides
//! only whether `egui` is also told. A button or a wheel is exclusive: one of
//! the two, never both.
//!
//! # Deciding a release before it is performed
//!
//! [`Panel::released`] takes the drag out of hand, so [`claim`] on a release
//! has to be asked **first** — asked afterwards it would see no drag, route
//! the release to `egui`, and hand `egui` a button-up it never saw the
//! button-down for. That is rule 1 doing the work it exists for, and it is the
//! ordering a caller gets wrong.

use karakuri_layout::{Hit, Point};

use crate::panel::{Panel, GRAB};

/// Who a pointer event belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    /// The panel's: a boundary is under the pointer, or one is in hand.
    /// `egui` is not told.
    Panel,
    /// `egui`'s.
    Egui,
}

/// Who gets a pointer event at `p` — see the module documentation for the
/// rule and for why it is written there.
///
/// Takes `&mut Panel` for the solve alone: [`Layout::hit`] refuses to answer
/// from a dirty layout, and on a frame where nothing moved the solve is a flag
/// test.
pub fn claim(panel: &mut Panel, p: Point) -> Claim {
    // Rule 1, and it comes first: a gesture in progress is not re-decided from
    // where the pointer happens to be now.
    if panel.dragging() {
        return Claim::Panel;
    }
    panel.solve();
    match panel.layout().hit(p, GRAB) {
        Hit::Divider { .. } => Claim::Panel,
        Hit::View(_) | Hit::Nothing => Claim::Egui,
    }
}
