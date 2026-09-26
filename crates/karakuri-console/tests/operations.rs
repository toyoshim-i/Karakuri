//! Folding, soloing and dragging, against what `console.html` promises each of
//! them does.

mod common;

use common::{assert_sane, assert_within_bounds, id_of, near, rect_of, rects, solved, PLAUSIBLE};

/// Folding the left pane gives its width to the centre track while keeping
/// the divider at the window edge so it can be dragged back (ADR-0300).
#[test]
fn folding_the_left_pane_gives_its_width_to_the_centre() {
    let mut layout = solved(PLAUSIBLE);
    let centre = rect_of(&layout, "centre");
    let right = rect_of(&layout, "right-pane");

    layout.collapse(id_of(&layout, "left-pane"));
    layout.solve();
    assert_sane(&layout);
    assert_within_bounds(&layout);

    // The centre took the pane's width, and not the divider — the pane is
    // closed rather than gone, so the gap it keeps is still drawn. Nothing
    // else moved: the right pane is exactly where and what it was.
    assert!(layout.is_closed(id_of(&layout, "left-pane")));
    assert!(near(rect_of(&layout, "centre").w, centre.w + 340.0));
    assert!(near(rect_of(&layout, "right-pane").w, right.w));
    assert!(near(rect_of(&layout, "right-pane").x, right.x));
    assert!(near(rect_of(&layout, "left-pane").w, 0.0));

    // And nothing was destroyed by folding it: unfolding restores the width it
    // was storing all along.
    layout.expand(id_of(&layout, "left-pane"));
    layout.solve();
    assert!(near(rect_of(&layout, "left-pane").w, 340.0));
    assert!(near(rect_of(&layout, "centre").w, centre.w));
}

/// The right pane folds the same way and to the same place, which is what says
/// the previous test is about the arrangement rather than about the left pane.
#[test]
fn folding_the_right_pane_gives_its_width_to_the_centre() {
    let mut layout = solved(PLAUSIBLE);
    let centre = rect_of(&layout, "centre");
    let left = rect_of(&layout, "left-pane");

    layout.collapse(id_of(&layout, "right-pane"));
    layout.solve();
    assert_sane(&layout);

    assert!(layout.is_closed(id_of(&layout, "right-pane")));
    assert!(near(rect_of(&layout, "centre").w, centre.w + 400.0));
    assert!(near(rect_of(&layout, "left-pane").w, left.w));
}

/// Soloing the program view expands it to fill the entire window without margins (ADR-0157).
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

/// *"Every divider drags, because a preview's size is a machine's answer rather
/// than a layout's."* The one that matters most is the program's height, so it
/// is asserted from both ends: it goes down to its minimum and up to most of
/// the window, on the same arrangement.
#[test]
fn the_program_height_drags_from_small_to_large() {
    let mut layout = solved(PLAUSIBLE);
    let centre = id_of(&layout, "centre");
    let top = rect_of(&layout, "program").y;

    // A weak machine: as small as the arrangement lets it be, and it really is
    // small — a picture, not a placeholder.
    layout.set_divider(centre, 0, top);
    layout.solve();
    assert_sane(&layout);
    assert!(near(rect_of(&layout, "program").h, 217.0));

    // A strong one: aim past the bottom of the window and land against the
    // inspector's minimum, with everything still inside the viewport.
    layout.set_divider(centre, 0, 4000.0);
    layout.solve();
    assert_sane(&layout);
    assert_within_bounds(&layout);
    let centre_h = rect_of(&layout, "centre").h;
    assert!(near(rect_of(&layout, "program").h, centre_h - 10.0 - 151.5));
    assert!(rect_of(&layout, "program").h > 800.0);
}

/// Moving dividers out and back restores exact layout dimensions across both
/// fixed and flexible tracks.
#[test]
fn dragging_a_divider_and_dragging_it_back_reproduces_the_arrangement() {
    let mut layout = solved(PLAUSIBLE);
    let before = rects(&layout);
    let body = layout.children(layout.root())[1];
    let centre = id_of(&layout, "centre");

    // The boundary starts at the left pane's far edge, 218.
    let landed = layout.set_divider(body, 0, 300.0);
    layout.solve();
    assert!(
        near(landed, 300.0),
        "the drag was stopped at {landed} rather than reaching 300 — nothing \
         in the arrangement should be pinning this divider"
    );
    assert!(near(rect_of(&layout, "left-pane").w, 300.0));

    let top = before[0].y;
    let program_edge = 48.0 + 10.0 + 395.0;
    let landed = layout.set_divider(centre, 0, top + program_edge + 160.0);
    layout.solve();
    assert!(near(landed, top + program_edge + 160.0));
    assert!(near(rect_of(&layout, "program").h, 555.0));
    assert_sane(&layout);

    layout.set_divider(centre, 0, top + program_edge);
    layout.set_divider(body, 0, 340.0);
    layout.solve();
    assert_eq!(before, rects(&layout));
}
