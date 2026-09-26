//! Frame repaint eligibility assertions without requiring a window or event loop (ADR-0164).
//!
//! Validates `Repaint::Never` for idle panels and guarantees repaints for model mutations.

mod common;

use std::time::Duration;

use common::{drawn_once, showing, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Outcome, Panel};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::view::rearrange;
use karakuri_layout::Point;

/// The canvas the picture is fitted to — the workspace's reference workload,
/// and what the Program bay arranges its body for.
const CANVAS: (u32, u32) = (1280, 720);

/// A panel at a plausible window size, solved.
fn panel() -> Panel {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    panel
}

/// The middle of the first boundary the arrangement has.
fn on_a_boundary(panel: &mut Panel) -> Point {
    panel.solve();
    let layout = panel.layout();
    let (split, index) = layout.boundaries().next().expect("no boundary");
    let gap = layout.boundary(split, index).expect("no pair");
    Point::new(gap.x + gap.w * 0.5, gap.y + gap.h * 0.5)
}

/// Executes a named operation on the panel and returns its outcome.
fn did(panel: &mut Panel, op: Op) -> Outcome {
    panel.op(op)
}

/// The id of a named region.
fn node(panel: &mut Panel, name: &str) -> karakuri_layout::NodeId {
    panel.solve();
    panel.layout().find(name).expect("no such region")
}

// ---------------------------------------------------------------------------
// The still half
// ---------------------------------------------------------------------------

/// An idle panel or a no-op operation emits `Repaint::Never` and leaves egui duration at max.
#[test]
fn a_still_panel_asks_for_no_repaint() {
    let mut panel = panel();
    let root = panel.layout().root();

    // Verifies no repaint occurs when attempting operations on nodes without an enclosing split.
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::FoldEnclosing(root))).repaint(),
        Repaint::Never,
        "a fold of the split enclosing the root, which has none, moved nothing"
    );
    // Asked speculatively, and both say they did nothing.
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::UnfoldAll)).repaint(),
        Repaint::Never,
        "an unfold with nothing folded moved nothing"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::Unsolo)).repaint(),
        Repaint::Never,
        "an unsolo with nothing soloed moved nothing"
    );
    // A report is a print. It is the case most obviously worth catching: it
    // returns a `Vec` of every region, which reads like a change and is not.
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::Report)).repaint(),
        Repaint::Never,
        "a report printed and moved nothing"
    );

    // The Program bay returns no repaint when rearrangement check yields no geometry change.
    let mut still = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    rearrange(&mut still, CANVAS);
    assert!(
        !rearrange(&mut still, CANVAS),
        "the bay rearranged itself on a panel nobody touched"
    );
    assert_eq!(
        Change::Rearranged { moved: false }.repaint(),
        Repaint::Never,
        "a frame on which the Program bay did not move asked for another one"
    );

    // The pointer, where `egui` is the one answering for it.
    assert_eq!(Change::Pointer(Claim::Egui).repaint(), Repaint::Never);
    assert_eq!(
        Change::Wheeled(Claim::Egui, false).repaint(),
        Repaint::Never
    );
    // A wheel the panel claimed and scrolled nothing with — one withheld
    // mid-drag so that it cannot reach `egui`, or one spun against the top of
    // an Inspector pane's list. A claim withheld is not an action taken, and
    // a pane already at the top of what it holds draws what it drew.
    assert_eq!(
        Change::Wheeled(Claim::Panel, false).repaint(),
        Repaint::Never
    );

    // Navigation inputs hitting boundary limits emit Pointed(false), requiring no repaint.
    assert_eq!(Change::Pointed(false).repaint(), Repaint::Never);

    // And `egui` itself, on a pass that wanted nothing.
    assert_eq!(Repaint::asked(Duration::MAX), Repaint::Never);
    assert!(!Repaint::Never.wanted());

    // Put together, a still panel is still: the soonest of every answer above
    // is still nothing at all.
    let together = [
        Change::Pointer(Claim::Egui).repaint(),
        Change::Wheeled(Claim::Panel, false).repaint(),
        Change::Rearranged { moved: false }.repaint(),
        Change::Pointed(false).repaint(),
        Repaint::asked(Duration::MAX),
    ]
    .into_iter()
    .fold(Repaint::Never, Repaint::soonest);
    assert_eq!(together, Repaint::Never);
}

// ---------------------------------------------------------------------------
// The half that fails silently
// ---------------------------------------------------------------------------

/// Every operation modifying layout, room, or viewport state triggers a repaint request.
#[test]
fn everything_that_changes_the_console_asks_for_a_frame() {
    let mut panel = panel();
    let library = node(&mut panel, "library");
    let picture = node(&mut panel, "program-view");

    // A fold, and the split enclosing one. Both move every sibling in the
    // split as well as the region named — folding the left pane widens the
    // centre, which is the "operation whose outcome changes another region"
    // that a per-region decision would miss.
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::Fold(library))).repaint(),
        Repaint::Now,
        "a fold"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::UnfoldAll)).repaint(),
        Repaint::Now,
        "an unfold that unfolded something"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::FoldEnclosing(library))).repaint(),
        Repaint::Now,
        "folding the split a region belongs to"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::UnfoldAll)).repaint(),
        Repaint::Now,
        "an unfold that brought a folded split back"
    );

    // Outputs dot operations toggle picture fold/unfold, triggering immediate repaints.
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::Fold(picture))).repaint(),
        Repaint::Now,
        "the sink turned off"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::Unfold(picture))).repaint(),
        Repaint::Now,
        "the sink turned back on"
    );

    // A solo folds everything else away, and undoing it brings it all back.
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::Solo(library))).repaint(),
        Repaint::Now,
        "a solo"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::Unsolo)).repaint(),
        Repaint::Now,
        "an unsolo that undid a solo"
    );
    assert_eq!(
        Change::Operated(&did(&mut panel, Op::Reset)).repaint(),
        Repaint::Now,
        "a reset"
    );

    // The pointer, where the panel is the one answering for it: a boundary is
    // in hand, or the pointer is within `GRAB` of one and `view` is drawing
    // the resize cursor from the hit.
    let mut panel = self::panel();
    let gap = on_a_boundary(&mut panel);
    assert_eq!(
        claim(&mut panel, &drawn_once(), &showing(&[]), gap),
        Claim::Panel
    );
    assert_eq!(
        Change::Pointer(Claim::Panel).repaint(),
        Repaint::Now,
        "the pointer on a boundary"
    );
    panel.press(gap);
    let away = Point::new(gap.x + 200.0, gap.y);
    assert_eq!(
        claim(&mut panel, &drawn_once(), &showing(&[]), away),
        Claim::Panel,
        "a drag keeps its claim wherever the pointer has gone"
    );
    assert_eq!(
        Change::Pointer(Claim::Panel).repaint(),
        Repaint::Now,
        "the pointer with a boundary in hand"
    );
    panel.moved(away);
    panel.released(None);
    assert_eq!(
        Change::Pointer(Claim::Panel).repaint(),
        Repaint::Now,
        "the release that let the boundary go"
    );

    // Program bay rearrangement changes cell rectangles and triggers a repaint.
    let mut wide = Panel::new(1588.0, PLAUSIBLE.h);
    assert!(
        rearrange(&mut wide, CANVAS),
        "a window past the crossover did not rearrange the Program bay"
    );
    assert_eq!(
        Change::Rearranged { moved: true }.repaint(),
        Repaint::Now,
        "the deck previews moved across the bay and no frame was owed"
    );

    assert_eq!(Change::Room.repaint(), Repaint::Now, "the room toggled");

    // Inspector pane wheel scrolling triggers repaint (ADR-0307).
    assert_eq!(
        Change::Wheeled(Claim::Panel, true).repaint(),
        Repaint::Now,
        "a pane scrolled under the pointer and no frame was owed"
    );

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

/// Sub-pixel boundary drags still trigger repaints even if below the readout reporting threshold.
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
        claim(&mut panel, &drawn_once(), &showing(&[]), creep),
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

/// Preserves `egui` delayed repaint durations without prematurely collapsing them into immediate frames.
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
