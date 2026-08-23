//! Folding, soloing and dragging, against what `console.html` promises each of
//! them does.

mod common;

use common::{assert_sane, assert_within_bounds, id_of, near, rect_of, rects, solved, PLAUSIBLE};

/// *"Fold the left pane away"* — the operation `karakuri-layout` names — and
/// the region that grows is the centre, because the centre is the only
/// flexible track in `.body-grid` (`minmax(340px, 1fr)`) and the mock's two
/// side tracks are fixed pixel widths.
#[test]
fn folding_the_left_pane_gives_its_width_to_the_centre() {
    let mut layout = solved(PLAUSIBLE);
    let centre = rect_of(&layout, "centre-pane");
    let right = rect_of(&layout, "right-pane");

    layout.collapse(id_of(&layout, "left-pane"));
    layout.solve();
    assert_sane(&layout);
    assert_within_bounds(&layout);

    // The centre took the pane's width *and* the divider that is no longer
    // drawn beside it. Nothing else moved: the right pane is exactly where and
    // what it was.
    assert!(near(
        rect_of(&layout, "centre-pane").w,
        centre.w + 218.0 + 10.0
    ));
    assert!(near(rect_of(&layout, "right-pane").w, right.w));
    assert!(near(rect_of(&layout, "right-pane").x, right.x));
    assert!(near(rect_of(&layout, "left-pane").w, 0.0));

    // And nothing was destroyed by folding it: unfolding restores the width it
    // was storing all along.
    layout.expand(id_of(&layout, "left-pane"));
    layout.solve();
    assert!(near(rect_of(&layout, "left-pane").w, 218.0));
    assert!(near(rect_of(&layout, "centre-pane").w, centre.w));
}

/// The right pane folds the same way and to the same place, which is what says
/// the previous test is about the arrangement rather than about the left pane.
#[test]
fn folding_the_right_pane_gives_its_width_to_the_centre() {
    let mut layout = solved(PLAUSIBLE);
    let centre = rect_of(&layout, "centre-pane");
    let left = rect_of(&layout, "left-pane");

    layout.collapse(id_of(&layout, "right-pane"));
    layout.solve();
    assert_sane(&layout);

    assert!(near(
        rect_of(&layout, "centre-pane").w,
        centre.w + 268.0 + 10.0
    ));
    assert!(near(rect_of(&layout, "left-pane").w, left.w));
}

/// *"Solo the program view: the panel folds away and only the picture is left,
/// which is also how you capture this window."*
///
/// So the program's rectangle is the window's, exactly. ADR-0157 says what
/// would spoil it: a maximum is honoured and the leftover is trailing space,
/// so a maximum anywhere on the path — on `program`, on `centre-pane`, or on
/// the pane row — would leave a margin the operator cannot get rid of, in a
/// window they are about to record.
#[test]
fn solo_on_the_program_leaves_the_program_holding_the_window() {
    let mut layout = solved(PLAUSIBLE);
    layout.solo(id_of(&layout, "program"));
    layout.solve();
    assert_sane(&layout);

    assert_eq!(rect_of(&layout, "program"), PLAUSIBLE);

    // Nothing else is drawing.
    for name in [
        "transport",
        "outputs",
        "left-pane",
        "right-pane",
        "inspector",
    ] {
        assert!(
            !layout.visible(id_of(&layout, name)),
            "{name} is still visible under a solo on the program"
        );
    }

    // And the panel comes back exactly as it was.
    layout.unsolo();
    layout.solve();
    assert_eq!(rects(&layout), rects(&solved(PLAUSIBLE)));
}

/// *"Every divider drags, because a preview's size is a machine's answer
/// rather than a layout's."* The one that matters most is the program's
/// height, so it is asserted from both ends: it goes down to its minimum and
/// up to most of the window, on the same arrangement.
#[test]
fn the_program_height_drags_from_small_to_large() {
    let mut layout = solved(PLAUSIBLE);
    let centre = id_of(&layout, "centre-pane");
    let top = rect_of(&layout, "program").y;

    // A weak machine: as small as the arrangement lets it be, and it really is
    // small — a picture, not a placeholder.
    layout.set_divider(centre, 0, top);
    layout.solve();
    assert_sane(&layout);
    assert!(near(rect_of(&layout, "program").h, 200.0));

    // A strong one: aim past the bottom of the window and land against the
    // inspector's minimum, with everything still inside the viewport.
    layout.set_divider(centre, 0, 4000.0);
    layout.solve();
    assert_sane(&layout);
    assert_within_bounds(&layout);
    let centre_h = rect_of(&layout, "centre-pane").h;
    assert!(near(rect_of(&layout, "program").h, centre_h - 10.0 - 126.0));
    assert!(rect_of(&layout, "program").h > 800.0);
}

/// A drag out and a drag back reproduce the arrangement — every rectangle of
/// it, not just the pair that moved. Two dividers, because the two sides of
/// each pair store their size differently: `left-pane` is fixed beside a
/// flexible centre, and `program` is fixed beside a flexible inspector, so a
/// drag writes a size on one side and a weight on the other.
#[test]
fn dragging_a_divider_and_dragging_it_back_reproduces_the_arrangement() {
    let mut layout = solved(PLAUSIBLE);
    let before = rects(&layout);
    let panes = layout.children(layout.root())[1];
    let centre = id_of(&layout, "centre-pane");

    // The boundary starts at the left pane's far edge, 218.
    let landed = layout.set_divider(panes, 0, 300.0);
    layout.solve();
    assert!(
        near(landed, 300.0),
        "the drag was stopped at {landed} rather than reaching 300 — nothing \
         in the arrangement should be pinning this divider"
    );
    assert!(near(rect_of(&layout, "left-pane").w, 300.0));

    let top = before[0].y;
    let program_edge = 48.0 + 10.0 + 378.0;
    let landed = layout.set_divider(centre, 0, top + program_edge + 160.0);
    layout.solve();
    assert!(near(landed, top + program_edge + 160.0));
    assert!(near(rect_of(&layout, "program").h, 538.0));
    assert_sane(&layout);

    layout.set_divider(centre, 0, top + program_edge);
    layout.set_divider(panes, 0, 218.0);
    layout.solve();
    assert_eq!(before, rects(&layout));
}
