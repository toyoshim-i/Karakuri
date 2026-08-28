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
//!
//!    **There are five of them now**: the Outputs row's sink
//!    ([`crate::view::outputs`]), a mixer strip's fader knob
//!    ([`crate::view::Mixer::grab`]), its blend chip
//!    ([`crate::view::Mixer::blend`]), its tally chip
//!    ([`crate::view::Mixer::tally`]) and its mask mini
//!    ([`crate::view::Mixer::mask`]). The rule did not change to hold the
//!    second, the third, the fourth or the fifth, which is what it was written
//!    for — and each is asked exactly the way the first is: the derivation
//!    that draws it, asked whether the point is on it, with nothing stored.
//!
//!    **The fourth is the first control with a state that can be pending**,
//!    and it is still only an affordance: it names a destination and refuses
//!    nothing, because what may be asked belongs where the record is applied
//!    ([P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md)).
//!    So this rule holds it unchanged too.
//!
//!    **The fifth is the first control that carries a value it does not
//!    draw**, and this rule does not change for that either: what a press
//!    *asks for* is [`crate::view::Mixer::mask`]'s, and whether a press landed
//!    on the chip is one rectangle either way
//!    ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
//!
//!    **A control claims what it acts on and no more.** A fader's *track* is
//!    drawn by the console and is not claimed, because a press on it does
//!    nothing — see [`crate::view::Mixer::grab`] for why a press off the knob
//!    must not move the value. Claiming a press in order to throw it away
//!    would put the rule and the act out of step, and `egui` owns nothing
//!    there either, so the two answers are the same nothing.
//! 4. Otherwise it goes to
//!    [`egui_winit::State::on_window_event`](https://docs.rs/egui-winit) and
//!    `egui` decides.
//!
//! Rule 2 before rule 3 is *first refusal* meant literally: a control under a
//! boundary's grab would be dead, and it is the test above that keeps the
//! order from ever mattering. **The mixer's knobs are measured the same way**
//! and in the same file's spirit — `tests/fader.rs` asserts that no knob in
//! the bay is within [`GRAB`] of any boundary, and carries the same guard on
//! itself, so the ordering goes on costing nothing there too. **The blend
//! chip is measured a third time** in `tests/blend.rs`, **the tally chip a
//! fourth** in `tests/tally.rs` and **the mask mini a fifth** in
//! `tests/mask.rs`, each with its own guard: whether a control clears every
//! boundary's band is two constants and a rectangle, and it is never inherited
//! from the control above it. No two of the three chips are the same number —
//! the nearest boundary to all of them is the pane divider down the left of
//! the bay, and the **8.97** the blend chip clears it by is the mode row's
//! width, where the tally's **14.23** is its own capsule's. The mask mini
//! shares the blend chip's row and clears the same divider by **41.03**,
//! because it sits at the far end of that row — and it is the one chip whose
//! *nearest boundary is not the same one on every strip*: on the three strips
//! that are not against the pane, the nearest is the boundary under the bay,
//! **74.50** below the mode row. So the number that binds is 41.03 and it was
//! measured rather than inherited from the chip 3px to its left.
//!
//! # A caller acts on the control, and it asks the same question again
//!
//! [`claim`] says *the panel's*; it does not say *the dot's*. What a press on
//! the control does is [`crate::view::Outputs::op`] — one operation, named —
//! and the caller asks [`crate::view::outputs`] for it. That is the same one
//! derivation this rule hit-tests, asked a second time rather than copied, so
//! the chip that claims a press and the chip that acts on it cannot come
//! apart. A fader is the same arrangement: [`crate::view::Mixer::grab`]
//! answers *is this a control* here and *which control, and where along it*
//! to the caller, off one derivation and one set of values. So is the blend
//! chip: [`crate::view::Mixer::blend`] answers *is this a control* and *what
//! does a press on it ask for* — `SetBlendMode` naming the mode after the one
//! the deck reports — off the same laid-out strip this rule hit-tests. So is
//! the tally chip: [`crate::view::Mixer::tally`] answers the same two with
//! `SetResidency` naming the residency after the one that was **requested**,
//! which is the whole of ADR-0195 and is why a press on a parked chip asks for
//! the withdrawal of its own prime request.
//!
//! So is the mask mini: [`crate::view::Mixer::mask`] answers *is this a
//! control* and *what does a press on it ask for* — `SetMaskShape` naming the
//! shape after the one the deck reports, **and the angle the deck reports
//! beside it**, which is the whole of ADR-0203 and is why choosing a shape
//! does not straighten a diagonal front.
//!
//! **Four questions, one derivation.** The mixer bay is laid out once per
//! event and asked for every control it has — a knob, a blend chip, a tally
//! chip and a mask mini are four questions about one laid-out strip, and a
//! second derivation would be a second answer that could disagree with the one
//! the frame drew.
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
use crate::view::{mixer, outputs, Strip};

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
///
/// **What the mixer adds to that is two more galley lookups per strip**, for
/// the tally's word and the blend's, because those are what a strip's boxes
/// are laid out around. It is paid on a pointer event and not on a frame, and
/// a console with no deck behind it pays nothing at all — [`mixer`] answers
/// `None` to an empty slice before it asks for any type. **The chips cost
/// none of that again**: the bay is derived once and all four of the mixer's
/// controls are asked of it — and the mask mini adds no lookup of its own,
/// because it holds a mark rather than a word.
///
/// **A control in a bay that is not laid out is never reached**, and it is
/// [`mixer`] that answers so rather than a check here: a folded mixer — or one
/// inside a folded pane — has no room for its row of strips, `strips_row`
/// answers `None`, and the bay is `None` before any chip is hit-tested.
/// `tests/tally.rs` asserts it both ways round.
///
/// **And it takes the strips, for the same reason one level further out.** A
/// fader's knob sits on the fill's moving edge, so *where the control is*
/// depends on what the deck said this frame — and this crate has no deck
/// (ADR-0156), so the values arrive the way they arrive everywhere else here:
/// handed in by whoever owns it, as [`crate::view::View::mixer`] is. An empty
/// slice is a console with no deck behind it, which has no strips and so no
/// knobs, and it is what every test in this crate that is not about the mixer
/// passes.
pub fn claim(panel: &mut Panel, ctx: &egui::Context, strips: &[Strip], p: Point) -> Claim {
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
        // Rule 3, over all five of the console's controls. Each is asked the
        // same way — the derivation that draws it, asked whether the point is
        // on it — and no answer is stored.
        Hit::View(_) | Hit::Nothing => {
            let on_sink = outputs(ctx, panel.layout()).is_some_and(|row| row.hit(p));
            // **The bay is derived once for all of its controls**, since a
            // knob, a blend chip, a tally chip and a mask mini are four
            // questions about one laid-out strip.
            let on_strip = || {
                mixer(ctx, panel.layout(), strips).is_some_and(|bay| {
                    bay.grab(p).is_some()
                        || bay.blend(p).is_some()
                        || bay.tally(p).is_some()
                        || bay.mask(p).is_some()
                })
            };
            match on_sink || on_strip() {
                true => Claim::Panel,
                false => Claim::Egui,
            }
        }
    }
}
