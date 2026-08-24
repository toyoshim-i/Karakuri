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
//! # The first control is drawn, and it clears the grab
//!
//! The Outputs row's one sink ([`crate::view::outputs`]) is that control, and
//! the paragraph above is why it is measured rather than assumed. The row is
//! 34 tall and the chip is 18.5, centred, so there is **7.75** of row above
//! the chip and 7.75 below, against a [`GRAB`] of **6**: the boundary's band
//! ends 1.75 pixels above the control and nothing overlaps.
//!
//! **That is a fact about two constants and a rectangle, so it is a test and
//! not a sentence** — `tests/outputs.rs`, which fails if the control moves up,
//! if the row gets shorter, or if [`GRAB`] widens. Widening the grab past 7.75
//! makes the sink unclickable in its top edge and the rule below is what would
//! have to change, deliberately, rather than the control being nudged.
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
//! 3. **Otherwise, if the pointer is on a control the console draws, it is the
//!    panel's** — because the console paints and `egui` owns no widget
//!    anywhere in it, so a press routed to `egui` there reaches nothing at
//!    all. The panel's controls are painted shapes, and the only thing that
//!    knows a press landed on one is this rule.
//! 4. Otherwise it goes to
//!    [`egui_winit::State::on_window_event`](https://docs.rs/egui-winit) and
//!    `egui` decides.
//!
//! Rule 2 before rule 3 is *first refusal* meant literally: a control under a
//! boundary's grab would be dead, and it is the test above that keeps the
//! order from ever mattering.
//!
//! # A caller acts on the control, and it asks the same question again
//!
//! [`claim`] says *the panel's*; it does not say *the dot's*. What a press on
//! the control does is [`crate::view::Outputs::op`] — one operation, named —
//! and the caller asks [`crate::view::outputs`] for it. That is the same one
//! derivation this rule hit-tests, asked a second time rather than copied, so
//! the chip that claims a press and the chip that acts on it cannot come
//! apart.
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
use crate::view::outputs;

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
///
/// **Takes the `egui` context to ask where the console's controls are.** Rule
/// 3 is about a painted chip whose width is the width of the name in it, so
/// answering it means laying that name out — which is `egui`'s to do and
/// nobody else's, since it is `egui` that will paint the same run. It is a
/// cached galley lookup per event, and before the first frame there are no
/// fonts and no drawn control at all, which
/// [`crate::view::outputs`] answers `None` to.
pub fn claim(panel: &mut Panel, ctx: &egui::Context, p: Point) -> Claim {
    // Rule 1, and it comes first: a gesture in progress is not re-decided from
    // where the pointer happens to be now.
    if panel.dragging() {
        return Claim::Panel;
    }
    panel.solve();
    // Rule 2 before rule 3: the boundary's first refusal is what the ordering
    // is, and the control clearing every grab is what stops it costing
    // anything. See the module documentation and `tests/outputs.rs`.
    match panel.layout().hit(p, GRAB) {
        Hit::Divider { .. } => Claim::Panel,
        Hit::View(_) | Hit::Nothing => match outputs(ctx, panel.layout()) {
            Some(row) if row.hit(p) => Claim::Panel,
            _ => Claim::Egui,
        },
    }
}
