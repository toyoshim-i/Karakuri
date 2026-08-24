//! **Whether a frame is owed**, asserted without a window.
//!
//! [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)'s
//! first clause has two halves and the second one is the dangerous half. *A
//! still panel costs nothing* is easy to get right and easy to see when it is
//! wrong — a window that spins shows up on any clock. *And everything else
//! still costs what it costs* is the half that fails silently: one path that
//! changes the model and reaches no repaint leaves a control on screen showing
//! a value that is not true any more, with nothing anywhere saying so.
//!
//! Neither half can be asserted in the window loop, because a `winit` handler
//! is not something a test can call and a stale pixel is not an error. So the
//! decision came out into [`karakuri_console::repaint`], and this is what asks
//! it: **the still case, and every case that is not still, one at a time.**

mod common;

use std::time::Duration;

use common::PLAUSIBLE;
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Outcome, Panel};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_layout::Point;

/// A panel at a plausible window size, solved.
fn panel() -> Panel {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    panel
}

/// A point inside a named region, well clear of its edges and so of every
/// boundary's grab.
fn inside(panel: &mut Panel, name: &str) -> Point {
    panel.solve();
    let layout = panel.layout();
    let rect = layout.rect(layout.find(name).expect("no such region"));
    Point::new(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
}

/// The middle of the first boundary the arrangement has.
fn on_a_boundary(panel: &mut Panel) -> Point {
    panel.solve();
    let layout = panel.layout();
    let (split, index) = layout.boundaries().next().expect("no boundary");
    let gap = layout.boundary(split, index).expect("no pair");
    Point::new(gap.x + gap.w * 0.5, gap.y + gap.h * 0.5)
}

/// Do an operation with the pointer somewhere, and hand back what it did.
fn did(panel: &mut Panel, at: Point, op: Op) -> Outcome {
    panel.set_cursor(at);
    panel.op(op)
}

// ---------------------------------------------------------------------------
// The still half
// ---------------------------------------------------------------------------

/// **A still panel asks for no repaint, and neither does anything that reaches
/// the model and moves nothing.**
///
/// The first clause is not *draw less often*; it is *do no per-frame work at
/// all when nothing has changed*, so the assertion is [`Repaint::Never`] and
/// not a smaller number. Every case here is one an operator produces without
/// meaning to — a key pressed over an empty part of the panel, `z` with
/// nothing folded, a wheel while a boundary is in hand, a pointer `egui` is
/// already answering for — and each one is a frame that used to be drawn.
///
/// `egui`'s side of it is here too: `Duration::MAX` is what its context is
/// left holding by a pass that asked for nothing, which is every pass on a
/// panel with nothing on it.
#[test]
fn a_still_panel_asks_for_no_repaint() {
    let mut panel = panel();
    let in_a_bay = inside(&mut panel, "library");
    let on_a_gap = on_a_boundary(&mut panel);

    // A key that reaches the model and finds nothing to do.
    let nowhere = Point::new(-50.0, -50.0);
    assert_eq!(
        Change::Operated(&did(&mut panel, nowhere, Op::Fold)).repaint(),
        Repaint::Never,
        "a fold with nothing under the pointer moved nothing"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, on_a_gap, Op::Fold)).repaint(),
        Repaint::Never,
        "a fold with the pointer on a divider moved nothing"
    );
    // Asked speculatively, and both say they did nothing.
    assert_eq!(
        Change::Operated(&did(&mut panel, in_a_bay, Op::UnfoldAll)).repaint(),
        Repaint::Never,
        "an unfold with nothing folded moved nothing"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, in_a_bay, Op::Unsolo)).repaint(),
        Repaint::Never,
        "an unsolo with nothing soloed moved nothing"
    );
    // A report is a print. It is the case most obviously worth catching: it
    // returns a `Vec` of every region, which reads like a change and is not.
    assert_eq!(
        Change::Operated(&did(&mut panel, in_a_bay, Op::Report)).repaint(),
        Repaint::Never,
        "a report printed and moved nothing"
    );

    // The pointer, where `egui` is the one answering for it.
    assert_eq!(Change::Pointer(Claim::Egui).repaint(), Repaint::Never);
    assert_eq!(Change::Wheeled(Claim::Egui).repaint(), Repaint::Never);
    // A wheel the panel claimed — which happens only mid-drag, so that it
    // cannot reach `egui` — is a claim withheld and not an action taken.
    assert_eq!(Change::Wheeled(Claim::Panel).repaint(), Repaint::Never);

    // And `egui` itself, on a pass that wanted nothing.
    assert_eq!(Repaint::asked(Duration::MAX), Repaint::Never);
    assert!(!Repaint::Never.wanted());

    // Put together, a still panel is still: the soonest of every answer above
    // is still nothing at all.
    let together = [
        Change::Pointer(Claim::Egui).repaint(),
        Change::Wheeled(Claim::Panel).repaint(),
        Repaint::asked(Duration::MAX),
    ]
    .into_iter()
    .fold(Repaint::Never, Repaint::soonest);
    assert_eq!(together, Repaint::Never);
}

// ---------------------------------------------------------------------------
// The half that fails silently
// ---------------------------------------------------------------------------

/// **Every path that changes what is on screen reaches a repaint.**
///
/// This is the list from the other direction, and it is the substance of the
/// clause rather than a footnote: a panel that under-repaints is far worse
/// than one that over-repaints, because a stale control looks exactly like a
/// live one.
///
/// Each case is driven through the model and the decision is asked of what the
/// model returned, so a case that stops changing anything stops being asserted
/// here for the right reason.
#[test]
fn everything_that_changes_the_console_asks_for_a_frame() {
    let mut panel = panel();
    let in_a_bay = inside(&mut panel, "library");
    let on_a_gap = on_a_boundary(&mut panel);

    // A fold, and the split enclosing one. Both move every sibling in the
    // split as well as the region named — folding the left pane widens the
    // centre, which is the "operation whose outcome changes another region"
    // that a per-region decision would miss.
    assert_eq!(
        Change::Operated(&did(&mut panel, in_a_bay, Op::Fold)).repaint(),
        Repaint::Now,
        "a fold"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, in_a_bay, Op::UnfoldAll)).repaint(),
        Repaint::Now,
        "an unfold that unfolded something"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, on_a_gap, Op::FoldEnclosing)).repaint(),
        Repaint::Now,
        "folding the split a divider belongs to"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, in_a_bay, Op::UnfoldAll)).repaint(),
        Repaint::Now,
        "an unfold that brought a folded split back"
    );

    // A solo folds everything else away, and undoing it brings it all back.
    assert_eq!(
        Change::Operated(&did(&mut panel, in_a_bay, Op::Solo)).repaint(),
        Repaint::Now,
        "a solo"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, in_a_bay, Op::Unsolo)).repaint(),
        Repaint::Now,
        "an unsolo that undid a solo"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, in_a_bay, Op::Reset)).repaint(),
        Repaint::Now,
        "a reset"
    );

    // The pointer, where the panel is the one answering for it: a boundary is
    // in hand, or the pointer is within `GRAB` of one and `view` is drawing
    // the resize cursor from the hit.
    let mut panel = self::panel();
    let gap = on_a_boundary(&mut panel);
    assert_eq!(claim(&mut panel, gap), Claim::Panel);
    assert_eq!(
        Change::Pointer(Claim::Panel).repaint(),
        Repaint::Now,
        "the pointer on a boundary"
    );
    panel.press(gap);
    let away = Point::new(gap.x + 200.0, gap.y);
    assert_eq!(
        claim(&mut panel, away),
        Claim::Panel,
        "a drag keeps its claim wherever the pointer has gone"
    );
    assert_eq!(
        Change::Pointer(Claim::Panel).repaint(),
        Repaint::Now,
        "the pointer with a boundary in hand"
    );
    panel.moved(away);
    panel.released();
    assert_eq!(
        Change::Pointer(Claim::Panel).repaint(),
        Repaint::Now,
        "the release that let the boundary go"
    );

    // The room, which no `Outcome` reports because it is the view's and not
    // the model's: every colour changes and nothing in the arrangement moves.
    assert_eq!(Change::Room.repaint(), Repaint::Now, "the room toggled");

    // A resize, and a scale change, which re-solve the arrangement into a
    // different viewport.
    assert_eq!(
        Change::Viewport.repaint(),
        Repaint::Now,
        "the window resized, or the display's scale changed"
    );

    // And `egui`, for a frame it wants immediately.
    assert_eq!(Repaint::asked(Duration::ZERO), Repaint::Now);
}

/// **A boundary that moved less than a pixel still asks for a frame**, and
/// this is the trap the decision is written to avoid rather than a curiosity.
///
/// `Panel::moved` returns `None` for a move too small to be *worth saying* —
/// `WORTH_SAYING`, half a pixel, a threshold about how much a readout should
/// print at sixty asks a second. Reading that `None` as *nothing changed* is
/// the shortest route to a silent under-repaint: half a logical pixel is a
/// whole physical one on a 2x display, so the boundary is drawn where it no
/// longer is, in the middle of the one gesture an operator is watching
/// closely.
///
/// So the assertion is a pair. The model reports nothing, **and** the frame is
/// owed anyway, because the decision is taken from who claimed the event and
/// never from what the drag returned.
#[test]
fn a_drag_too_small_to_report_still_asks_for_a_frame() {
    let mut panel = panel();
    let gap = on_a_boundary(&mut panel);
    let axis = {
        let (split, _) = panel.layout().boundaries().next().expect("no boundary");
        panel.layout().axis(split).expect("a divider is on a split")
    };

    panel.press(gap);
    // The first move always reports: nothing has been said yet.
    let first = Point::new(gap.x + 20.0, gap.y + 20.0);
    assert!(panel.moved(first).is_some());

    // A tenth of a pixel further along the boundary's own axis. The layout
    // takes it; the readout does not print it.
    let creep = match axis {
        karakuri_layout::Axis::Row => Point::new(first.x + 0.1, first.y),
        karakuri_layout::Axis::Column => Point::new(first.x, first.y + 0.1),
    };
    assert!(
        panel.moved(creep).is_none(),
        "the move was large enough to report, so this test is no longer about \
         the threshold it was written for"
    );

    // And the frame is owed all the same.
    assert_eq!(
        claim(&mut panel, creep),
        Claim::Panel,
        "a drag in hand keeps its claim"
    );
    assert_eq!(
        Change::Pointer(Claim::Panel).repaint(),
        Repaint::Now,
        "a boundary that moved less than half a pixel is still drawn somewhere new"
    );
}

// ---------------------------------------------------------------------------
// egui's own requests
// ---------------------------------------------------------------------------

/// **`egui`'s repaint request is honoured with the delay it named**, rather
/// than collapsed into "draw now".
///
/// `egui` animates, blinks a text cursor and fades a tooltip in, and it says
/// so by asking to be repainted *after* a duration. Answering a 250 ms request
/// with an immediate frame does not make the animation smoother — the loop
/// then draws, `egui` asks again for what is left of the delay, and the
/// animation becomes a spin at whatever rate the machine can manage. That is
/// the exact cost this whole change exists to stop paying, reached from the
/// one direction that looks like obeying the rule.
///
/// [`Repaint::soonest`] is asserted for the same reason: it may only bring a
/// frame forward, so combining the panel's answer with `egui`'s can lose
/// neither — but it must not turn a deadline into an immediate frame on its
/// own.
#[test]
fn eguis_repaint_delay_is_honoured_rather_than_collapsed_to_now() {
    let quarter = Duration::from_millis(250);
    let tick = Duration::from_millis(16);

    // The three answers, and the two sentinels are `egui`'s own.
    assert_eq!(Repaint::asked(Duration::MAX), Repaint::Never);
    assert_eq!(Repaint::asked(Duration::ZERO), Repaint::Now);
    assert_eq!(Repaint::asked(quarter), Repaint::After(quarter));
    assert!(Repaint::After(quarter).wanted());

    // A deadline survives being combined with a panel that wants nothing, and
    // the shorter of two deadlines wins.
    assert_eq!(
        Repaint::After(quarter).soonest(Repaint::Never),
        Repaint::After(quarter)
    );
    assert_eq!(
        Repaint::Never.soonest(Repaint::After(quarter)),
        Repaint::After(quarter)
    );
    assert_eq!(
        Repaint::After(quarter).soonest(Repaint::After(tick)),
        Repaint::After(tick)
    );
    assert_eq!(
        Repaint::After(tick).soonest(Repaint::After(quarter)),
        Repaint::After(tick)
    );

    // A frame the panel owes now is sooner than any deadline, which is the
    // only direction the combination is allowed to move in.
    assert_eq!(Repaint::After(tick).soonest(Repaint::Now), Repaint::Now);
    assert_eq!(Repaint::Now.soonest(Repaint::After(tick)), Repaint::Now);
    assert_eq!(Repaint::Never.soonest(Repaint::Never), Repaint::Never);
}
