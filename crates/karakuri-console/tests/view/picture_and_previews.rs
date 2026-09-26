use super::view_common::*;

/// The picture matches the canvas aspect ratio and whole-pixel constraints across all window layouts (ADR-0170).
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

    // Picture width stops tracking window width within each layout arrangement (ADR-0182).
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
    // Verify vertical expansion relative to baseline height when dragged taller.
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

/// Asserts that picture dimensions derive from the dynamic canvas shape rather than a fixed 16:9 ratio.
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

/// Asserts that a folded picture produces no render rectangle regardless of whether it was collapsed directly or via its bay.
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

/// Verifies mock preview cell dimensions and padding against the program bay layout.
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
/// Asserts that a folded preview row yields no preview rectangles.
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

    // Folding the picture leaves preview auditions active and visible.
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
