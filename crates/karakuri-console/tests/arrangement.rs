//! What the arrangement solves to, at a window somebody would really use and
//! at the smallest one it is claimed to work at.

mod common;

use common::{
    assert_sane, assert_within_bounds, id_of, implied_min, near, rect_of, solved, PLAUSIBLE,
    SMALLEST,
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
        rect_of(&layout, "centre").w,
        1920.0 - 218.0 - 268.0 - 20.0
    ));

    // The transport and the outputs row are the height of their contents and
    // the body row has the rest.
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

    // Had it followed the width, at 1920 the centre is 1414 wide, its
    // body 1396, and a 16:9 picture in it 785 tall — more than twice what the
    // arrangement gives it, and the inspector's whole height and then some.
    let centre = rect_of(&wide, "centre");
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
    // so the body row carries the sum by hand and this is what keeps the two
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

/// **The Program bay is two regions, and the split is the bay's own 378 read
/// out loud.**
///
/// *"The bay is two regions and they fold apart."* — `console.html`. The
/// number the bay is checked against has not changed and neither has the way
/// it was derived: bay head 27, padding 9 + 9, picture 262, gap 8, previews
/// 63. What changed is that three of those terms are now one region, one is
/// the divider, and two are the other — so this asserts the sum term by term
/// rather than asserting 378 twice.
#[test]
fn the_program_bay_is_a_picture_over_a_preview_row() {
    let layout = solved(SMALLEST);

    let program = id_of(&layout, "program");
    assert_eq!(
        layout.axis(program),
        Some(Axis::Column),
        "the picture is over the previews, so the bay is a column"
    );
    let children: Vec<Option<&str>> = layout
        .children(program)
        .iter()
        .map(|c| layout.name(*c))
        .collect();
    assert_eq!(
        children,
        vec![Some("program-view"), Some("deck-previews")],
        "the Program bay's regions, in the order the mock draws them"
    );

    // 27 + 9 + 262 for the picture, `.program-body`'s 8px gap as the divider,
    // and 63 + 9 for the previews and the padding under them.
    assert!(near(rect_of(&layout, "program-view").h, 298.0));
    assert!(near(layout.divider(program).expect("a split has one"), 8.0));
    assert!(near(rect_of(&layout, "deck-previews").h, 72.0));
    assert!(near(rect_of(&layout, "program").h, 378.0));

    // The picture is what absorbs the bay's height, and the preview row is
    // content-height: drag the program's bottom edge down and every pixel of
    // it goes to the picture.
    let mut wider = solved(PLAUSIBLE);
    let centre = id_of(&wider, "centre");
    let was = rect_of(&wider, "program-view").h;
    wider.set_divider(centre, 0, rect_of(&wider, "program").y + 600.0);
    wider.solve();
    assert_sane(&wider);
    assert!(near(rect_of(&wider, "deck-previews").h, 72.0));
    assert!(near(rect_of(&wider, "program-view").h, was + 222.0));
}

/// **Both parts are addressable, and they fold independently** — which is the
/// whole reason the bay is a split rather than a leaf with two rectangles
/// drawn inside it.
///
/// *"The picture is a sink ... and it is on screen exactly when that sink is
/// on — so there is no state where it is hidden and still costing a pass. The
/// deck previews under it are auditions of their own, so they stay when it
/// goes."* Turning the sink off is a fold by name; the previews staying is
/// that fold not reaching them.
#[test]
fn the_picture_and_the_previews_fold_apart() {
    // The picture off: the previews stay, and they are what is left in the
    // bay — at the 72 they are, not swollen to the height the picture was
    // holding. The bay claims what its visible content can use, which is the
    // preview row and nothing else, so the bay is 72 too. That the 306 it gave
    // up goes to the inspector is the sink's own sentence and is asserted in
    // the test below; what is asserted here is what the fold does — the row
    // survives it, at its own size.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "program-view"));
    layout.solve();
    assert_sane(&layout);
    assert!(!layout.visible(id_of(&layout, "program-view")));
    assert!(
        layout.visible(id_of(&layout, "deck-previews")),
        "the previews went with the picture; they are auditions of their own"
    );
    assert!(near(rect_of(&layout, "deck-previews").h, 72.0));
    assert!(near(rect_of(&layout, "program").h, 72.0));

    // And the other way round, which is what says the first half is about the
    // two folding apart rather than about the picture.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "deck-previews"));
    layout.solve();
    assert_sane(&layout);
    assert!(!layout.visible(id_of(&layout, "deck-previews")));
    assert!(layout.visible(id_of(&layout, "program-view")));
    assert!(near(rect_of(&layout, "program-view").h, 378.0));

    // Neither fold touched the bay around them, and unfolding restores what
    // was stored all along.
    layout.expand(id_of(&layout, "deck-previews"));
    layout.solve();
    assert!(near(rect_of(&layout, "program-view").h, 298.0));
    assert!(near(rect_of(&layout, "deck-previews").h, 72.0));
}

/// **The sink's own sentence, as an assertion.**
///
/// *"The picture is a sink, listed in Outputs as program view ... Turn it off
/// and that picture goes, giving its height to the inspector."* —
/// `console.html`. The bay is `Fixed(378)` and the solve is top-down, so for
/// as long as the bay claimed its stored size whatever was left inside it, the
/// height went to the preview row instead and the manual's sentence was a
/// sentence about nothing. What makes it true is the bay claiming what its
/// visible content can use: with the picture folded that is the preview row's
/// 72, and the flexible child of the same column — the inspector — takes the
/// 306 the bay gave up.
#[test]
fn folding_the_picture_gives_the_bays_height_to_the_inspector() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let mut layout = solved(viewport);
        let program = rect_of(&layout, "program").h;
        let inspector = rect_of(&layout, "inspector").h;
        assert!(near(program, 378.0));

        layout.collapse(id_of(&layout, "program-view"));
        layout.solve();
        assert_sane(&layout);

        // The bay claims the preview row and the preview row alone.
        assert!(
            near(rect_of(&layout, "program").h, 72.0),
            "the bay is {} rather than the 72 its content can use",
            rect_of(&layout, "program").h
        );
        // And the row is still its own size rather than swollen into the
        // space the picture left — it is an audition, not a picture.
        assert!(
            near(rect_of(&layout, "deck-previews").h, 72.0),
            "the preview row swelled to {}",
            rect_of(&layout, "deck-previews").h
        );
        // Every pixel of the difference, to the inspector: 378 - 72 = 306.
        assert!(
            near(rect_of(&layout, "inspector").h, inspector + program - 72.0),
            "the inspector is {} rather than {}",
            rect_of(&layout, "inspector").h,
            inspector + program - 72.0
        );

        // The bay declares a minimum of 200, and it does not hold it here: 200
        // is what the bay needs while the picture is in it. Nothing was
        // written back, so unfolding the sink restores the 378 and the
        // inspector gives the 306 straight back.
        assert_eq!(layout.bounds(id_of(&layout, "program")).0, 200.0);
        layout.expand(id_of(&layout, "program-view"));
        layout.solve();
        assert_sane(&layout);
        assert_within_bounds(&layout);
        assert!(near(rect_of(&layout, "program").h, program));
        assert!(near(rect_of(&layout, "inspector").h, inspector));
    }
}
