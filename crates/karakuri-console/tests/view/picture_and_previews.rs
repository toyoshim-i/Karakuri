use super::view_common::*;

/// The picture is the canvas's shape at every window, centred in whatever the
/// region has, and a whole number of pixels.
///
/// The rule ADR-0170 took for a deck preview cell, applied where it was first
/// refused. Before it, the picture was the whole region and `Present::draw`
/// letterboxed into it — so at any window above the mock's narrowest a texture
/// was allocated at the region's full size and the bars inside it were rendered
/// and uploaded every frame. At 1920 wide that is 1396 x 262 where 466 x 262 is
/// the picture: two texels in three are black nobody looks at.
///
/// The wide end and the tall end both matter and they fail differently. Wide is
/// the ordinary case and the region is wider than the canvas, so the height is
/// what limits and the leftover is ground either side. Tall only happens when
/// an operator drags the program's height past what the width can carry, and
/// then the width limits and the leftover is above and below — a case a rule
/// written for wide windows alone gets wrong in silence.
///
/// # The window is arranged first, and the region is a different region past
/// the crossover
///
/// Every assertion below is against the `program-view` region, and past a
/// 1588-wide window that region is the whole bay: the cells have gone down the
/// sides and the row is set aside, so the picture's region is the bay itself
/// less nothing. The claims still hold term for term — the picture is still the
/// canvas's shape, still whole pixels, still centred in what the region leaves,
/// still inside it — which is the point worth having: *the picture never leaves
/// the rectangle it is clipped to* is the one property that does not care which
/// arrangement won, and it is the property a rearrangement half-applied would
/// break.
#[test]
fn the_picture_is_the_canvass_shape_at_every_window() {
    for width in [990.0, 1010.0, 1280.0, 1920.0, 3440.0] {
        let panel = arranged(
            karakuri_layout::Rect {
                w: width,
                ..SMALLEST
            },
            CANVAS,
        );
        let layout = panel.layout();
        let rect = picture_rect(layout, CANVAS).expect("the picture is on screen");
        let region = rect_of(layout, "program-view");

        assert!(
            (rect.width() / rect.height() - 16.0 / 9.0).abs() < 0.01,
            "at {width} wide the picture is {}x{} and the canvas is 16:9",
            rect.width(),
            rect.height()
        );
        // **Whole pixels**, which is what makes the texture a blit rather than
        // a resample: `physical` rounds, so a fractional rectangle is a
        // texture of one size drawn into a box of another.
        assert!(
            near(rect.width(), rect.width().round()) && near(rect.height(), rect.height().round()),
            "at {width} wide the picture is {}x{}, which is not a whole number of pixels",
            rect.width(),
            rect.height()
        );
        // **Centred in the box the region leaves**, so the ground either side
        // is equal — the same claim ADR-0170 makes about a cell in its track.
        let before = rect.min.x - (region.x + 9.0);
        let after = (region.x + region.w - 9.0) - rect.max.x;
        assert!(
            near(before, after),
            "at {width} wide the picture has {before} before it and {after} after it"
        );
        assert!(
            before >= -1e-3,
            "at {width} wide the picture starts {before} inside the region's padding"
        );
        // Inside the region, always: the leftover is the console's ground and
        // never a picture hanging over the bay.
        assert!(
            rect.min.y >= region.y + 27.0 + 9.0 - 1e-3 && rect.max.y <= region.y + region.h + 1e-3,
            "at {width} wide the picture runs from {} to {} outside its region",
            rect.min.y,
            rect.max.y
        );
    }

    // **The width stops following the window**, which is the pass the change
    // removes: the region grows and the picture does not. Stated **within an
    // arrangement**, because ADR-0182 put a step between the two — below, the
    // picture stops at the mock's 466 and the leftover is ground; beside, it
    // stops at what the bay's *height* carries and the leftover is ground
    // again. Neither follows the window; there is one step between them and
    // `tests/rearrange.rs` is where it is held.
    let below = |w: f32| {
        let panel = arranged(karakuri_layout::Rect { w, ..SMALLEST }, CANVAS);
        picture_rect(panel.layout(), CANVAS).expect("on screen")
    };
    let narrow = below(1300.0);
    let mid = below(1450.0);
    assert!(
        near(mid.width(), narrow.width()) && near(mid.height(), narrow.height()),
        "a window 150 wider changed the picture below the crossover: {:?} against {:?}",
        mid.size(),
        narrow.size()
    );
    let wide = below(PLAUSIBLE.w);
    let widest = below(3440.0);
    assert!(
        near(widest.width(), wide.width()) && near(widest.height(), wide.height()),
        "a window 1520 wider changed the picture beside the cells: {:?} against {:?}",
        widest.size(),
        wide.size()
    );
    // And the region did grow, in both, so the assertions above are about the
    // rule and not about a window that never widened.
    let panel = arranged(PLAUSIBLE, CANVAS);
    let region = rect_of(panel.layout(), "program-view");
    assert!(region.w - wide.width() > 500.0);

    // Height drag expands the picture until aspect-ratio bounds favor below-layout (ADR-0182).
    let mut layout = solved(PLAUSIBLE);
    let centre = id_of(&layout, "centre");
    layout.set_divider(centre, 0, 4000.0);
    layout.solve();
    let dragged = picture_rect(&layout, CANVAS).expect("on screen");
    // **Against the same window before the drag**, and it is a ratio rather
    // than the 400 pixels this used to add: `wide` is the picture *beside* the
    // cells now, 592 x 333 rather than the mock's 466 x 262, so the margin it
    // was compared against was measured on a rectangle that is no longer the
    // one at this window. Doubling was the claim while the bay was 378; with
    // the preview captions in it the picture beside is 350 rather than 333, so
    // what a full-height drag buys is 642 against 350 — under twice, and the
    // ratio is written as what it measures rather than rounded up to a claim
    // the rectangle no longer supports.
    assert!(
        dragged.height() > wide.height() * 1.8,
        "dragging the program taller left the picture at {} tall against the {} it had \
         before the drag",
        dragged.height(),
        wide.height()
    );
    assert!(
        (dragged.width() / dragged.height() - 16.0 / 9.0).abs() < 0.01,
        "a picture dragged taller is {}x{}",
        dragged.width(),
        dragged.height()
    );

    // **And past a point the width is what answers instead** — a narrow window
    // dragged tall, where the leftover is above and below rather than either
    // side. A rule written for wide windows alone gets this one wrong in
    // silence, because on screen it is still a picture in a bay.
    let mut layout = solved(karakuri_layout::Rect {
        w: SMALLEST.w,
        h: 1400.0,
        ..SMALLEST
    });
    let centre = id_of(&layout, "centre");
    layout.set_divider(centre, 0, 4000.0);
    layout.solve();
    let tall = picture_rect(&layout, CANVAS).expect("on screen");
    let region = rect_of(&layout, "program-view");
    let box_h = region.h - 27.0 - 9.0;
    assert!(
        box_h > tall.height() + 100.0,
        "the region is {box_h} tall inside its head and the picture is {}, so nothing was \
         left over and this is not the width-limited case",
        tall.height()
    );
    assert!(
        (tall.width() / tall.height() - 16.0 / 9.0).abs() < 0.01,
        "a picture taller than its width can carry is {}x{}",
        tall.width(),
        tall.height()
    );
    // The width is the box's, so the picture is exactly the mock's again — and
    // it is centred in what is left, top and bottom.
    assert!(near(tall.width(), 466.0), "{} wide", tall.width());
    let above = tall.min.y - (region.y + 27.0 + 9.0);
    let below = (region.y + region.h) - tall.max.y;
    assert!(
        near(above, below),
        "the picture has {above} above it and {below} below it"
    );
    assert!(
        above > 100.0,
        "there is only {above} of leftover above the picture"
    );
}

/// The shape is the canvas's and not a 16:9 written into this crate.
///
/// The failure is silent and it lasts until somebody runs a performance at a
/// canvas the mock's designer never drew: a hard-coded 16:9 gives a picture of
/// the wrong shape, `Present::draw` letterboxes the real canvas inside it, and
/// what comes back is bars in a rectangle that was supposed to have none —
/// which is exactly the state this whole rule exists to leave behind, with
/// nothing on screen saying it came back.
///
/// `--canvas` takes any pair of numbers and reaches a replay through
/// `Record::Canvas`, so the shape is a value and not a constant.
#[test]
fn the_shape_is_the_canvass_and_not_a_sixteen_by_nine_in_this_crate() {
    let layout = solved(PLAUSIBLE);
    let wide = picture_rect(&layout, CANVAS).expect("on screen");
    let squarish = picture_rect(&layout, SQUARISH).expect("on screen");

    assert!(
        (squarish.width() / squarish.height() - 4.0 / 3.0).abs() < 0.01,
        "a 4:3 canvas got a {}x{} picture, so the shape came from somewhere other than \
         the canvas",
        squarish.width(),
        squarish.height()
    );
    // Same box, same height, and a narrower picture — the region is wider than
    // either, so the height is what limits both.
    assert!(near(squarish.height(), wide.height()));
    assert!(squarish.width() < wide.width());

    // And a canvas taller than it is wide is not a special case either.
    let portrait = picture_rect(&layout, (720, 1280)).expect("on screen");
    assert!(
        (portrait.width() / portrait.height() - 9.0 / 16.0).abs() < 0.01,
        "a portrait canvas got a {}x{} picture",
        portrait.width(),
        portrait.height()
    );
}

/// No rectangle where the picture is folded away, which is the manual's *"there
/// is no state where it is hidden and still costing a pass"*: a caller that
/// renders into this rectangle records no pass at all when there is none.
///
/// What carries it is the size test and not a visibility test, and that was
/// learnt from this test rather than assumed: written with both, deleting the
/// visibility check left it passing, because a folded region keeps its
/// rectangle and loses its extent. So the check went and this is what holds the
/// remaining line — including for a picture folded by its bay rather than by
/// itself, which a visibility test and a size test answer alike and which is
/// asserted here so that the equivalence is not left as a belief.
#[test]
fn a_folded_picture_has_no_rectangle() {
    let mut layout = solved(PLAUSIBLE);
    assert!(picture_rect(&layout, CANVAS).is_some());

    layout.collapse(id_of(&layout, "program-view"));
    layout.solve();
    assert_eq!(picture_rect(&layout, CANVAS), None);

    // The previews are still there, so this is the picture being folded and
    // not the bay.
    assert!(layout.visible(id_of(&layout, "deck-previews")));

    layout.expand(id_of(&layout, "program-view"));
    layout.solve();
    assert!(picture_rect(&layout, CANVAS).is_some());

    // The bay folded around it, which is `g` over the picture rather than `f`.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "program"));
    layout.solve();
    assert_eq!(picture_rect(&layout, CANVAS), None);

    // And a solo on the previews, which leaves the picture out rather than
    // folding anything above it.
    let mut layout = solved(PLAUSIBLE);
    layout.solo(id_of(&layout, "deck-previews"));
    layout.solve();
    assert_eq!(picture_rect(&layout, CANVAS), None);
}

/// The four preview cells are the mock's own, at the width the mock draws them.
///
/// Every figure here is `lib.rs`'s Program bay derivation read from the other
/// end. At `SMALLEST` the centre track is 484, `.program-body`'s 9px padding
/// either side leaves 466, and 466 less three 6px gaps over four tracks is 112
/// — which at 16:9 is 63, the image's height. A cell is that image and the
/// caption band under it, `.cell`'s 4 and `.caption`'s 13, so the row is 80 and
/// the arrangement gave `deck-previews` that plus its 9px of padding
/// underneath. The sum the bay was built from and the rectangles it solves to
/// are the same numbers or the bay is wrong.
///
/// The insets are three of the four on purpose: nothing at the top, because the
/// 9 above the cells in the CSS is the split's 8px divider plus
/// `program-view`'s own bottom and belongs to neither this region nor the
/// picture.
#[test]
fn the_preview_cells_are_the_mocks_at_the_width_the_mock_draws() {
    let layout = solved(SMALLEST);
    let cells = preview_rects(&layout, CANVAS).expect("the previews are on screen");
    let region = rect_of(&layout, "deck-previews");

    for (deck, cell) in cells.iter().enumerate() {
        assert!(
            near(cell.width(), 112.0),
            "cell {deck} is {} wide",
            cell.width()
        );
        assert!(
            near(cell.height(), 63.0),
            "cell {deck} is {} tall",
            cell.height()
        );
        // Nothing above the row, and `.program-body`'s padding under it.
        assert!(
            near(cell.min.y, region.y),
            "cell {deck} starts at {} and the region at {}",
            cell.min.y,
            region.y
        );
        // The **image** ends where the caption band starts, and the caption
        // ends where `.program-body`'s padding does — so the row fills the
        // region less that pad, and the image is one term of the row.
        assert!(
            near(cell.max.y, region.y + region.h - 9.0 - 17.0),
            "cell {deck} ends at {} and the caption band starts at {}",
            cell.max.y,
            region.y + region.h - 9.0 - 17.0
        );
        assert!(
            near(caption_of(*cell).max.y, region.y + region.h - 9.0),
            "cell {deck}'s caption ends at {} and the region's padding leaves {}",
            caption_of(*cell).max.y,
            region.y + region.h - 9.0
        );
    }

    // `.previews`'s `gap: 6px`, between the tracks and nowhere else.
    for deck in 1..DECKS {
        let gap = cells[deck].min.x - cells[deck - 1].max.x;
        assert!(
            near(gap, 6.0),
            "the gap before cell {deck} is {gap} and not 6"
        );
    }

    // The row is the region less a pad either side, so the first cell's left
    // edge and the last cell's right edge are that pad in from the region.
    assert!(
        near(cells[0].min.x, region.x + 9.0),
        "the row starts at {} and the region's padding leaves {}",
        cells[0].min.x,
        region.x + 9.0
    );
    assert!(
        near(cells[DECKS - 1].max.x, region.x + region.w - 9.0),
        "the row ends at {} and the region's padding leaves {}",
        cells[DECKS - 1].max.x,
        region.x + region.w - 9.0
    );
}

/// Verifies that preview cells maintain 16:9 aspect ratios without overlapping across
/// supported viewport widths below the crossover threshold (ADR-0239).
#[test]
fn the_preview_cells_tile_their_region_and_stay_sixteen_by_nine() {
    for width in [1280.0, 1350.0, 1400.0, 1450.0] {
        let panel = arranged(
            karakuri_layout::Rect {
                w: width,
                ..SMALLEST
            },
            CANVAS,
        );
        let layout = panel.layout();
        let cells = preview_rects(layout, CANVAS).expect("the previews are on screen");
        let region = rect_of(layout, "deck-previews");
        assert!(
            !layout.is_set_aside(id_of(layout, "deck-previews")),
            "at {width} wide the row is set aside, so this is not the arrangement being \
             asserted below"
        );
        let row_left = region.x + 9.0;
        let row_right = region.x + region.w - 9.0;

        for (deck, cell) in cells.iter().enumerate() {
            assert!(
                (cell.width() / cell.height() - 16.0 / 9.0).abs() < 0.01,
                "at {width} wide, cell {deck} is {}x{} and a preview is 16:9",
                cell.width(),
                cell.height()
            );
            assert!(
                cell.min.x >= row_left - 1e-3 && cell.max.x <= row_right + 1e-3,
                "at {width} wide, cell {deck} runs from {} to {} outside the row's {row_left}..{row_right}",
                cell.min.x,
                cell.max.x
            );
            assert!(
                cell.min.y >= region.y - 1e-3 && cell.max.y <= region.y + region.h - 9.0 + 1e-3,
                "at {width} wide, cell {deck} runs from {} to {} outside the region",
                cell.min.y,
                cell.max.y
            );
        }
        for deck in 1..DECKS {
            assert!(
                cells[deck].min.x >= cells[deck - 1].max.x - 1e-3,
                "at {width} wide, cell {deck} starts at {} and cell {} ends at {}",
                cells[deck].min.x,
                deck - 1,
                cells[deck - 1].max.x
            );
        }

        // Centred in its track: the ground either side of a cell is equal, and
        // it is the same for every cell.
        let track = (row_right - row_left - 6.0 * (DECKS - 1) as f32) / DECKS as f32;
        for (deck, cell) in cells.iter().enumerate() {
            let track_x = row_left + (track + 6.0) * deck as f32;
            let before = cell.min.x - track_x;
            let after = track_x + track - cell.max.x;
            assert!(
                near(before, after),
                "at {width} wide, cell {deck} has {before} before it and {after} after it in its track"
            );
        }

        // The row is pinned at 63 tall by the arrangement, so a wider window
        // buys width and no height at all — which is the whole reason a cell
        // cannot both fill its track and stay 16:9.
        assert!(
            near(cells[0].height(), 63.0),
            "at {width} wide the row is {} tall",
            cells[0].height()
        );
    }

    // **1483 is the last window with a row in it and 1484 is the first
    // without**, which is what makes the four widths above the four that are
    // in this test's country rather than four that happen to pass (ADR-0239).
    let row_at = |w: f32| {
        let panel = arranged(karakuri_layout::Rect { w, ..SMALLEST }, CANVAS);
        panel
            .layout()
            .is_set_aside(id_of(panel.layout(), "deck-previews"))
    };
    assert!(
        !row_at(1483.0),
        "the row went beside the picture before 1484"
    );
    assert!(
        row_at(1484.0),
        "the cells are still in the row at 1484 wide"
    );
}
/// rule stated once more on the other half of the bay: a caller that renders
/// four auditions into these rectangles records no pass at all when there are
/// none, and *"a priming deck draws only while something auditions it"*.
///
/// folds the row, and `g` over the bay folds the picture with it.
#[test]
fn a_folded_preview_row_has_no_rectangles() {
    let mut layout = solved(PLAUSIBLE);
    assert!(preview_rects(&layout, CANVAS).is_some());

    layout.collapse(id_of(&layout, "deck-previews"));
    layout.solve();
    assert_eq!(preview_rects(&layout, CANVAS), None);

    // The picture is still there, so this is the row being folded and not the
    // bay — *"The deck previews under it are auditions of their own, so they
    // stay when it goes"*, read the other way round.
    assert!(picture_rect(&layout, CANVAS).is_some());

    layout.expand(id_of(&layout, "deck-previews"));
    layout.solve();
    assert!(preview_rects(&layout, CANVAS).is_some());

    // **And the fold the other way round, which is the sentence the program's
    // readout now prints**: *"fold the picture away (f over it) and deck A
    // keeps the loop awake on its own"*. The manual's own words are the same
    // claim — *"The deck previews under it are auditions of their own, so they
    // stay when it goes"* — and a readout that says a thing the arrangement
    // does not do is how this project has been wrong twice about what folding
    // the picture costs.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "program-view"));
    layout.solve();
    assert_eq!(picture_rect(&layout, CANVAS), None);
    assert!(
        preview_rects(&layout, CANVAS).is_some(),
        "the picture is folded and the previews went with it, so an audition \
         that should still be running has nowhere to go"
    );

    // The bay folded around it.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "program"));
    layout.solve();
    assert_eq!(preview_rects(&layout, CANVAS), None);

    // And a solo on the picture, which leaves the row out rather than folding
    // it.
    let mut layout = solved(PLAUSIBLE);
    layout.solo(id_of(&layout, "program-view"));
    layout.solve();
    assert_eq!(preview_rects(&layout, CANVAS), None);
}
