//! What the arrangement solves to, at a window somebody would really use and
//! at the smallest one it is claimed to work at.

mod common;

use common::{
    assert_sane, assert_within_bounds, implied_min, near, rect_of, solved, PLAUSIBLE, SMALLEST,
};
use karakuri_layout::Axis;

#[test]
fn at_a_plausible_window_the_arrangement_is_sane() {
    let layout = solved(PLAUSIBLE);
    assert_sane(&layout);
    assert_within_bounds(&layout);

    // The mock's own tracks, at a width where nothing is squeezed: 218 and 268
    // are `.body-grid`'s outer columns and the centre absorbs the rest.
    assert!(near(rect_of(&layout, "left-pane").w, 218.0));
    assert!(near(rect_of(&layout, "right-pane").w, 268.0));
    assert!(near(
        rect_of(&layout, "centre-pane").w,
        1920.0 - 218.0 - 268.0 - 20.0
    ));

    // The transport and the outputs strip are the height of their contents and
    // the pane row has the rest.
    assert!(near(rect_of(&layout, "transport").h, 48.0));
    assert!(near(rect_of(&layout, "outputs").h, 34.0));
    assert!(near(rect_of(&layout, "transport").y, 0.0));
    assert!(near(
        rect_of(&layout, "outputs").y + rect_of(&layout, "outputs").h,
        1080.0
    ));
}

/// The manual's *Program, sized by height*, as an assertion: *"A 16:9 view
/// filling a wide centre column would be 763 pixels tall and eat the inspector
/// whole. So you drag its height."*
///
/// A window nearly twice as wide as the narrowest one gives the program not
/// one pixel of extra height. Everything the width buys goes to the picture,
/// which letterboxes into it — and the inspector keeps the height it had.
#[test]
fn the_program_does_not_grow_when_the_window_widens() {
    let narrow = solved(SMALLEST);
    let wide = solved(PLAUSIBLE);

    assert!(near(rect_of(&narrow, "program").h, 378.0));
    assert!(near(rect_of(&wide, "program").h, 378.0));

    // Had it followed the width, at 1920 the centre pane is 1414 wide, its
    // body 1396, and a 16:9 picture in it 785 tall — more than twice what the
    // arrangement gives it, and the inspector's whole height and then some.
    let centre = rect_of(&wide, "centre-pane");
    assert!(centre.w > 1400.0);
    assert!(rect_of(&wide, "program").h < (centre.w - 18.0) * 9.0 / 16.0);

    // And the inspector is what absorbed the height instead.
    assert!(near(rect_of(&wide, "inspector").h, centre.h - 378.0 - 10.0));
}

/// The panel is usable on a small screen, and `SMALLEST` says which one and
/// why. This asserts it there and asserts that the number is the tree's rather
/// than a copy of it that could drift.
#[test]
fn the_panel_is_usable_at_the_smallest_window_it_claims() {
    let layout = solved(SMALLEST);
    assert_sane(&layout);
    assert_within_bounds(&layout);

    // Every region is at or above its own minimum — which is what
    // `assert_within_bounds` says — and the two that decide the size are worth
    // naming: the mixer still has room for four strips, and the inspector's
    // panes are wide enough for the parameter grid the mock draws in them.
    assert!(near(rect_of(&layout, "mixer").h, 316.0));
    assert!(rect_of(&layout, "inspector-1").w >= 207.0);
    assert!(rect_of(&layout, "inspector-2").w >= 207.0);

    // The height is the sum of the column-axis minima, recomputed from the
    // tree. The model does not derive a split's minimum from its children's,
    // so the pane row carries the sum by hand and this is what keeps the two
    // honest.
    let root = layout.root();
    assert!(
        near(implied_min(&layout, root, Axis::Column), SMALLEST.h),
        "the tree's implied minimum height is {}, not the {} claimed",
        implied_min(&layout, root, Axis::Column),
        SMALLEST.h
    );

    // The width is not the tree's sum — that is 846, and the panel is unusable
    // there because the inspector's parameter rows stop fitting. 990 is
    // `.console`'s own `min-width: 1010px` less its padding, and it is the
    // narrower claim of the two. See the report on 340 against 1010.
    assert!(implied_min(&layout, root, Axis::Row) <= SMALLEST.w);
}
