//! What the arrangement solves to, at a window somebody would really use and at
//! the smallest one it is claimed to work at.

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

    // The outer tracks: 340 and 400 (ADR-0239) and the centre absorbs the rest.
    assert!(near(rect_of(&layout, "left-pane").w, 340.0));
    assert!(near(rect_of(&layout, "right-pane").w, 400.0));
    assert!(near(
        rect_of(&layout, "centre").w,
        1920.0 - 340.0 - 400.0 - 20.0
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
/// A window nearly twice as wide as the narrowest one gives the program not one
/// pixel of extra height. Everything the width buys goes to the picture, which
/// letterboxes into it — and the inspector keeps the height it had.
#[test]
fn the_program_does_not_grow_when_the_window_widens() {
    let narrow = solved(SMALLEST);
    let wide = solved(PLAUSIBLE);

    assert!(near(rect_of(&narrow, "program").h, 395.0));
    assert!(near(rect_of(&wide, "program").h, 395.0));

    let centre = rect_of(&wide, "centre");
    assert!(rect_of(&wide, "program").h < (centre.w - 18.0) * 9.0 / 16.0);

    // And the inspector is what absorbed the height instead.
    assert!(near(rect_of(&wide, "inspector").h, centre.h - 395.0 - 10.0));
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

    // The width is not the tree's sum. That sum is
    // `karakuri_console::MINIMUM_VIEWPORT.0` — the width below which the solve
    // stops honouring what the three tracks declare — and it is 777 against
    // this window's 1244. The floor under the solve holds every declared
    // minimum and no more; this window holds the *content* claim on top of
    // them, which is the mock's own centre of 484 and the 237.5 panes that come
    // with it. So this window is the wider of the two and the assertion is that
    // they are in that order. **The gap between them used to be a defect as
    // well as a claim** — at a centre of 340 a pane was 165.5 against the 207 a
    // parameter row's fixed tracks want — and ADR-0279 closed that, so what is
    // left between the two numbers is room rather than a hole.
    assert!(implied_min(&layout, root, Axis::Row) <= SMALLEST.w);
}

/// `MINIMUM_VIEWPORT` is the tree's own sum on both axes, recomputed here
/// rather than trusted — the same guard the body row's 556.5 is under, for the
/// same reason: the constant is written by hand because the model does not
/// derive a split's minimum from its children's, so nothing but this stops the
/// two drifting apart the next time a minimum moves.
///
/// It was watched to fail: with the width term written as the 846 this file's
/// comment used to claim, the recomputation answered the 692 the tree said at
/// the time and the assertion named both numbers. The tree says 777 today
/// (ADR-0279); what the test asserts is that the two agree, whatever they are.
#[test]
fn the_minimum_viewport_is_the_sum_of_the_declared_minima() {
    let layout = solved(PLAUSIBLE);
    let root = layout.root();

    // 160 + 425 + 172, and the two 10px dividers between the three tracks.
    let width = implied_min(&layout, root, Axis::Row);
    assert!(
        near(width, karakuri_console::MINIMUM_VIEWPORT.0),
        "the tree's implied minimum width is {width}, not the {} the constant claims",
        karakuri_console::MINIMUM_VIEWPORT.0
    );

    // 48 + 556.5 + 34, and the two 10px dividers between the three rows —
    // which is `SMALLEST.h` as well, and that is not a coincidence: the
    // console's claimed smallest window is held up by the column axis and cut
    // short by the row one.
    let height = implied_min(&layout, root, Axis::Column);
    assert!(
        near(height, karakuri_console::MINIMUM_VIEWPORT.1),
        "the tree's implied minimum height is {height}, not the {} the constant claims",
        karakuri_console::MINIMUM_VIEWPORT.1
    );
    assert!(near(karakuri_console::MINIMUM_VIEWPORT.1, SMALLEST.h));

    // And the width is the one that is *not* the smallest window claimed —
    // see the test above for why the two differ.
    assert!(karakuri_console::MINIMUM_VIEWPORT.0 < SMALLEST.w);
}

/// At the minimum viewport every region is at or above what it declares, and
/// one logical pixel under it none of them is.
///
/// The first half is what the constant means. The second is what it buys: at
/// 691.9 the body row's three tracks are all short — 171.74 for a right pane
/// that declared 172 — because the last step of `solve_split` scales every
/// child together once the frozen minima come to more than the extent
/// (ADR-0250). Nothing is out of place and nothing is negative; the panel is
/// simply no longer the panel its own numbers describe.
#[test]
fn one_pixel_under_the_minimum_no_region_holds_its_minimum() {
    let at = |w: f32, h: f32| {
        solved(karakuri_layout::Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
        })
    };

    let (w, h) = karakuri_console::MINIMUM_VIEWPORT;
    let layout = at(w, h);
    assert_sane(&layout);
    assert_within_bounds(&layout);
    assert!(near(rect_of(&layout, "left-pane").w, 160.0));
    assert!(near(rect_of(&layout, "centre").w, 425.0));
    assert!(near(rect_of(&layout, "right-pane").w, 172.0));

    let layout = at(w - 0.1, h);
    assert_sane(&layout);
    assert!(rect_of(&layout, "left-pane").w < 160.0);
    assert!(rect_of(&layout, "centre").w < 425.0);
    assert!(rect_of(&layout, "right-pane").w < 172.0);

    let layout = at(w, h - 0.1);
    assert_sane(&layout);
    assert!(rect_of(&layout, "transport").h < 48.0);
    assert!(rect_of(&layout, "outputs").h < 34.0);
}

/// At the minimum viewport an inspector pane is wide enough to draw a parameter
/// row's fader, and a pixel narrower it is not — which is what the centre's
/// declared minimum is *for*, and what it was not before ADR-0279.
///
/// The `.param` grid is `15px 88px 1fr 58px` with 8px gaps and 12 + 10 of
/// padding, and a pane is exactly as wide as the rows in it, so 207 of a pane
/// is spoken for before the fader — the `1fr` — has any width at all.
/// `view::param_into` draws the fader only where that leftover is positive, so
/// a pane of exactly 207 draws a parameter row with no fader in it. The minimum
/// is therefore 208 a pane and 2 x 208 + 9 = 425 in the centre, and not the 2 x
/// 207 + 9 = 423 ADR-0272 names: at 423 the leftover is zero and the bay still
/// draws no fader.
///
/// The 207 is recomputed from `room::size` rather than written here, so this is
/// the same reading `lib.rs` makes and not a second copy of it.
///
/// The drag is the other half and it is the reason a window minimum could not
/// have done this job: the body row's first boundary starves the centre to its
/// declared minimum at *any* window width, so the property has to hold at 1920
/// as well as at 777, and what makes it hold is the declared minimum rather
/// than the viewport.
///
/// It was watched to fail with `centre` back at the CSS track's 340 and the
/// panes at 165: *"inspector-1 is 165.5 wide, and a `.param` row's fixed tracks
/// want 207 before the fader has any width"*, at the minimum viewport and again
/// after the drag. With the centre at 423 and the panes at 207 it fails the
/// same way, one pixel out.
#[test]
fn at_the_minimum_an_inspector_pane_draws_a_parameter_fader() {
    use karakuri_console::room::size;

    // The `.param` grid's fixed tracks, term for term: the padding either side,
    // the ordinal, the name, the value, and the three gaps between the four.
    // 12 + 15 + 24 + 88 + 58 + 10 = 207.
    let fixed = size::PARAM_PAD_L
        + size::PARAM_ORD_W
        + size::PARAM_GAP * 3.0
        + size::PARAM_NAME_W
        + size::PARAM_VAL_W
        + size::PARAM_PAD_R;
    assert!(
        near(fixed, 207.0),
        "the `.param` grid reads {fixed}, not 207"
    );

    let panes = |layout: &karakuri_layout::Layout, at: &str| {
        for pane in ["inspector-1", "inspector-2"] {
            let w = rect_of(layout, pane).w;
            assert!(
                w - fixed > 0.0,
                "{at}: {pane} is {w} wide, and a `.param` row's fixed tracks \
                 want {fixed} before the fader has any width"
            );
            assert!(
                w - 1.0 - fixed <= 0.0,
                "{at}: {pane} is {w} wide, which is more than a pixel over the \
                 {fixed} its rows need — the minimum is no longer the threshold"
            );
        }
    };

    // At the minimum viewport, where the centre is at its declared minimum
    // because the whole row is.
    let (w, h) = karakuri_console::MINIMUM_VIEWPORT;
    let layout = solved(karakuri_layout::Rect {
        x: 0.0,
        y: 0.0,
        w,
        h,
    });
    assert_within_bounds(&layout);
    panes(&layout, "at the minimum viewport");

    // And at a window nobody would call small, with the body row's first
    // boundary dragged as far right as it will go — which is the centre at the
    // same declared minimum, reached from the other direction.
    let mut wide = solved(PLAUSIBLE);
    let body = wide.children(wide.root())[1];
    wide.set_divider(body, 0, PLAUSIBLE.w);
    wide.solve();
    assert_sane(&wide);
    assert_within_bounds(&wide);
    assert!(
        near(rect_of(&wide, "centre").w, 425.0),
        "the drag starved the centre to {}, not to the 425 it declares",
        rect_of(&wide, "centre").w
    );
    panes(&wide, "with the centre starved by a drag");
}

/// The Program bay is two regions, and the split is the bay's own 395 read out
/// loud.
///
/// *"The bay is two regions and they fold apart."* — `console.html`. The way
/// the number is derived has not changed: bay head 27, padding 9 + 9, picture
/// 262, gap 8, previews 80. The previews term is the one that moved — a cell is
/// its 63 of image, `.cell`'s 4px gap and `.caption`'s 13px under it — and
/// `lib.rs` is where growing the row rather than shrinking the cells is argued.
/// Three of those terms are one region, one is the divider, and two are the
/// other, so this asserts the sum term by term rather than asserting 395 twice.
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

    // 27 + 9 + 266 for the picture, `.program-body`'s 4px gap as the divider,
    // and 80 + 9 for the previews and the padding under them.
    assert!(near(rect_of(&layout, "program-view").h, 302.0));
    assert!(near(layout.divider(program).expect("a split has one"), 4.0));
    assert!(near(rect_of(&layout, "deck-previews").h, 89.0));
    assert!(near(rect_of(&layout, "program").h, 395.0));

    // The picture is what absorbs the bay's height, and the preview row is
    // content-height: drag the program's bottom edge down and every pixel of
    // it goes to the picture.
    let mut wider = solved(PLAUSIBLE);
    let centre = id_of(&wider, "centre");
    let was = rect_of(&wider, "program-view").h;
    wider.set_divider(centre, 0, rect_of(&wider, "program").y + 600.0);
    wider.solve();
    assert_sane(&wider);
    assert!(near(rect_of(&wider, "deck-previews").h, 89.0));
    assert!(near(rect_of(&wider, "program-view").h, was + 205.0));
}

/// Both parts are addressable, and they fold independently — which is the whole
/// reason the bay is a split rather than a leaf with two rectangles drawn
/// inside it.
///
/// *"The picture is a sink ... and it is on screen exactly when that sink is on
/// — so there is no state where it is hidden and still costing a pass. The deck
/// previews under it are auditions of their own, so they stay when it goes."*
/// Turning the sink off is a fold by name; the previews staying is that fold
/// not reaching them.
#[test]
fn the_picture_and_the_previews_fold_apart() {
    // The picture off: the previews stay, and they are what is left in the
    // bay — at the 89 they are, not swollen to the height the picture was
    // holding. The bay claims what its visible content can use, which is the
    // preview row and nothing else, so the bay is 89 too. That the 306 it gave
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
    assert!(near(rect_of(&layout, "deck-previews").h, 89.0));
    assert!(near(rect_of(&layout, "program").h, 89.0));

    // And the other way round, which is what says the first half is about the
    // two folding apart rather than about the picture.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "deck-previews"));
    layout.solve();
    assert_sane(&layout);
    assert!(!layout.visible(id_of(&layout, "deck-previews")));
    assert!(layout.visible(id_of(&layout, "program-view")));
    assert!(near(rect_of(&layout, "program-view").h, 395.0));

    // Neither fold touched the bay around them, and unfolding restores what
    // was stored all along.
    layout.expand(id_of(&layout, "deck-previews"));
    layout.solve();
    assert!(near(rect_of(&layout, "program-view").h, 302.0));
    assert!(near(rect_of(&layout, "deck-previews").h, 89.0));
}

/// The sink's own sentence, as an assertion.
///
/// *"The picture is a sink, listed in Outputs as program view ... Turn it off
/// and that picture goes, giving its height to the inspector."* —
/// `console.html`. The bay is `Fixed(395)` and the solve is top-down, so for as
/// long as the bay claimed its stored size whatever was left inside it, the
/// height went to the preview row instead and the manual's sentence was a
/// sentence about nothing. What makes it true is the bay claiming what its
/// visible content can use: with the picture folded that is the preview row's
/// 89, and the flexible child of the same column — the inspector — takes the
/// 306 the bay gave up.
#[test]
fn folding_the_picture_gives_the_bays_height_to_the_inspector() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let mut layout = solved(viewport);
        let program = rect_of(&layout, "program").h;
        let inspector = rect_of(&layout, "inspector").h;
        assert!(near(program, 395.0));

        layout.collapse(id_of(&layout, "program-view"));
        layout.solve();
        assert_sane(&layout);

        // The bay claims the preview row and the preview row alone.
        assert!(
            near(rect_of(&layout, "program").h, 89.0),
            "the bay is {} rather than the 89 its content can use",
            rect_of(&layout, "program").h
        );
        // And the row is still its own size rather than swollen into the
        // space the picture left — it is an audition, not a picture.
        assert!(
            near(rect_of(&layout, "deck-previews").h, 89.0),
            "the preview row swelled to {}",
            rect_of(&layout, "deck-previews").h
        );
        // Every pixel of the difference, to the inspector: 395 - 89 = 306.
        assert!(
            near(rect_of(&layout, "inspector").h, inspector + program - 89.0),
            "the inspector is {} rather than {}",
            rect_of(&layout, "inspector").h,
            inspector + program - 89.0
        );

        // The bay declares a minimum of 217, and it does not hold it here:
        // 217 is what the bay needs while the picture is in it. Nothing was
        // written back, so unfolding the sink restores the 395 and the
        // inspector gives the 306 straight back.
        assert_eq!(layout.bounds(id_of(&layout, "program")).0, 217.0);
        layout.expand(id_of(&layout, "program-view"));
        layout.solve();
        assert_sane(&layout);
        assert_within_bounds(&layout);
        assert!(near(rect_of(&layout, "program").h, program));
        assert!(near(rect_of(&layout, "inspector").h, inspector));
    }
}
