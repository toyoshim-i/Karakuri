use super::mixer_common::*;

#[test]
fn a_strip_is_the_mocks_own_boxes() {
    // Asserts literal stylesheet dimensions against `style.css` before relations (ADR-0177).
    assert!(
        near(size::STRIPS_PAD, 6.0),
        "`.mixer-strips` is `padding: 6px`"
    );
    assert!(near(size::STRIP_GAP, 4.0), "`.mixer-strips` is `gap: 4px`");
    assert!(
        near(size::STRIP_PAD_X, 4.0),
        "`.strip` is `padding: 7px 4px`"
    );
    assert!(
        near(size::STRIP_PAD_Y, 7.0),
        "`.strip` is `padding: 7px 4px`"
    );
    assert!(near(size::STRIP_GAP_Y, 5.0), "`.strip` is `gap: 5px`");
    assert!(
        near(size::STRIP_RADIUS, 9.0),
        "`.strip` is `border-radius: 9px`"
    );
    assert!(
        near(size::STRIP_NAME_SIZE, 10.0),
        "`.strip-name` is `font-size: 10px`"
    );
    assert!(near(size::TALLY_SIZE, 9.0), "`.tally` is `font-size: 9px`");
    assert!(near(size::TALLY_PAD_X, 7.0), "`.tally` is `padding: 0 7px`");
    assert!(near(size::TRIM_GAP, 5.0), "`.trim` is `gap: 5px`");
    assert!(near(size::TRIM_PAD_X, 3.0), "`.trim` is `padding: 0 3px`");
    assert!(
        near(size::TRIM_LABEL_SIZE, 9.0),
        "`.trim .lbl` is `font-size: 9px`"
    );
    assert!(near(size::FADER_H, 5.0), "`.fader` is `height: 5px`");
    assert!(near(size::FADER_KNOB_W, 9.0), "`.fader s` is `width: 9px`");
    assert!(
        near(size::FADER_KNOB_H, 11.0),
        "`.fader s` is `height: 11px`"
    );
    assert!(
        near(size::FADER_COL_H, 104.0),
        "`.fader-col` is `height: 104px`"
    );
    assert!(near(size::FADER_COL_GAP, 6.0), "`.fader-col` is `gap: 6px`");
    assert!(near(size::VFADER_W, 17.0), "`.vfader` is `width: 17px`");
    assert!(
        near(size::VFADER_INSET, 3.0),
        "`.vfader b` is `left: 3px; right: 3px; bottom: 3px`"
    );
    assert!(
        near(size::VFADER_KNOB_H, 9.0),
        "`.vfader s` is `height: 9px`"
    );
    assert!(
        near(size::VFADER_KNOB_OUT, 2.0),
        "`.vfader s` is `left: -2px; right: -2px`"
    );
    assert!(near(size::VMETER_W, 6.0), "`.vmeter` is `width: 6px`");
    assert!(
        near(size::VMETER_PEAK_H, 2.0),
        "`.vmeter u` is `height: 2px`"
    );
    assert!(
        near(size::STRIP_NUM_SIZE, 10.0),
        "`.strip-num` is `font-size: 10px`"
    );
    assert!(near(size::MODE_GAP, 3.0), "`.strip-mode` is `gap: 3px`");
    assert!(near(size::MINI_SIZE, 9.0), "`.mini` is `font-size: 9px`");
    assert!(near(size::MINI_PAD_X, 6.0), "`.mini` is `padding: 0 6px`");

    // **And the sum of them is the 215.5 the arrangement was written from.**
    // `lib.rs` derives the mixer's 316 as a bay head, `.mixer-strips`'s 6 + 6
    // around a strip, and `.xfade`'s 61.
    assert!(
        near(size::STRIP_H, 215.5),
        "a strip is {} tall",
        size::STRIP_H
    );

    // Asserts remaining vertical space under strips accounts for transition row (`XFADE_H`),
    // the retired crossfader row (`CROSSFADER_ROW`), and gaps against total bay height (316).
    const CROSSFADER_ROW: f32 = 16.5;
    let leftover = 316.0 - size::HEAD_H - size::STRIPS_PAD * 2.0 - size::STRIP_H;
    assert!(
        near(size::XFADE_H, 37.5),
        "`.xfade` is {} tall here and the mock draws 37.5 of it",
        size::XFADE_H
    );
    assert!(
        near(
            leftover,
            size::XFADE_H + size::XFADE_GAP + CROSSFADER_ROW + 0.5
        ),
        "the {leftover} the mixer has under its strips is not the transition row ({}) plus the \
         crossfader's gap and row ({} + {CROSSFADER_ROW}) plus the half pixel the bay's own \
         27 + 227.5 + 61 was rounded up by",
        size::XFADE_H,
        size::XFADE_GAP
    );
    assert!(
        near(leftover, 61.5),
        "what the mixer has left under its strips is not the 61 `.xfade` was reserved — the \
         bay's own 27 + 227.5 + 61 was rounded up by the half pixel this is over"
    );

    let strips = mock_strips();
    let (panel, ctx) = console(SMALLEST);
    let region = rect_of(panel.layout(), "mixer");
    let bay = bay(&panel, &ctx, &strips);
    let at = bay.strip(0);

    // The row: under the head, inside `.mixer-strips`'s padding, and exactly a
    // strip tall rather than whatever the bay has left.
    let row = strips_row(region);
    assert!(near(at.rect.min.y, row.min.y));
    assert!(near(at.rect.height(), size::STRIP_H));
    assert!(near(at.rect.min.x, row.min.x));

    // Four tracks, whatever the deck holds — three strips here.
    let track = (row.width() - size::STRIP_GAP * 3.0) / 4.0;
    assert!(
        near(at.rect.width(), track),
        "a strip is {} wide and a quarter of the row less three 4px gaps is {track}",
        at.rect.width()
    );

    // The six children, stacked from the strip's own padding with one gap
    // between each pair, in writing order.
    let inner_top = at.rect.min.y + size::STRIP_PAD_Y;
    assert!(near(at.name.min.y, inner_top));
    assert!(near(at.name.height(), size::STRIP_NAME_SIZE * size::LINE));
    assert!(near(at.name.min.x, at.rect.min.x + size::STRIP_PAD_X));
    assert!(
        near(at.name.width(), at.rect.width() - size::STRIP_PAD_X * 2.0),
        "`.strip-name` is `width: 100%` and this one is {} of {}",
        at.name.width(),
        at.rect.width() - size::STRIP_PAD_X * 2.0
    );

    for (before, after, what) in [
        (at.name, at.tally, "the name and the tally"),
        (at.tally, at.trim_label, "the tally and the trim"),
        (at.trim_label, at.fader, "the trim and the fader column"),
        (at.fader, at.num, "the fader column and the number"),
        (at.num, at.blend, "the number and the modes"),
    ] {
        assert!(
            near(after.min.y - before.max.y, size::STRIP_GAP_Y),
            "{what} are {} apart and `.strip`'s gap is {}",
            after.min.y - before.max.y,
            size::STRIP_GAP_Y
        );
    }

    // Each row's own height, which is its own type at the console's
    // line-height — and the fader column's stated 104.
    assert!(near(at.tally.height(), size::TALLY_H));
    assert!(near(at.trim_label.height(), size::TRIM_H));
    assert!(near(at.fader.height(), size::FADER_COL_H));
    assert!(near(at.meter.height(), size::FADER_COL_H));
    assert!(near(at.num.height(), size::STRIP_NUM_SIZE * size::LINE));
    assert!(near(at.blend.height(), size::MINI_H));
    assert!(near(at.mask.height(), size::MINI_H));

    // `.fader-col`: 17 and 6 with a 6px gap, centred across the strip.
    assert!(near(at.fader.width(), size::VFADER_W));
    assert!(near(at.meter.width(), size::VMETER_W));
    assert!(
        near(at.meter.min.x - at.fader.max.x, size::FADER_COL_GAP),
        "the fader and the meter are {} apart and `.fader-col`'s gap is {}",
        at.meter.min.x - at.fader.max.x,
        size::FADER_COL_GAP
    );
    let column = size::VFADER_W + size::FADER_COL_GAP + size::VMETER_W;
    assert!(near(
        (at.fader.min.x + at.meter.max.x) * 0.5,
        at.rect.center().x
    ));
    assert!(near(at.meter.max.x - at.fader.min.x, column));

    // `.trim`: the `g` on the row's own padding, the track after one gap, and
    // the track ends on the padding at the other side.
    assert!(near(
        at.trim_label.min.x,
        at.rect.min.x + size::STRIP_PAD_X + size::TRIM_PAD_X
    ));
    assert!(
        near(at.trim.min.x - at.trim_label.max.x, size::TRIM_GAP),
        "the `g` and the track are {} apart and `.trim`'s gap is {}",
        at.trim.min.x - at.trim_label.max.x,
        size::TRIM_GAP
    );
    assert!(near(
        at.trim.max.x,
        at.rect.max.x - size::STRIP_PAD_X - size::TRIM_PAD_X
    ));
    assert!(near(at.trim.height(), size::FADER_H));
    assert!(
        near(at.trim.center().y, at.trim_label.center().y),
        "`.trim` is `align-items: center` and the 5px track is not on the label's line"
    );

    // `.strip-mode`: two minis, one gap apart, centred.
    assert!(
        near(at.mask.min.x - at.blend.max.x, size::MODE_GAP),
        "the two minis are {} apart and `.strip-mode`'s gap is {}",
        at.mask.min.x - at.blend.max.x,
        size::MODE_GAP
    );
    assert!(near(
        (at.blend.min.x + at.mask.max.x) * 0.5,
        at.rect.center().x
    ));
    // The mask's mini holds a mark rather than a word, so its width is the
    // mark's own size inside `.mini`'s padding and border.
    assert!(near(
        at.mask.width(),
        size::MINI_SIZE + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0
    ));

    // And the whole of it is inside the strip it is drawn in, and the strip is
    // inside the row.
    for (rect, what) in [
        (at.name, "the name"),
        (at.tally, "the tally"),
        (at.trim, "the trim"),
        (at.fader, "the fader"),
        (at.meter, "the meter"),
        (at.num, "the number"),
        (at.blend, "the blend"),
        (at.mask, "the mask"),
    ] {
        assert!(
            at.rect.contains_rect(rect),
            "{what} at {rect:?} is not inside the strip {:?}",
            at.rect
        );
    }
    assert!(row.contains_rect(at.rect));
}

/// The strips tile their row and never overlap, which is the same gap
/// arithmetic `preview_cells` is checked for and the same wrong version: a gap
/// per track rather than a gap between two.
#[test]
fn the_strips_tile_their_row_and_never_overlap() {
    let strips = std::iter::repeat_with(mock).take(DECKS).collect::<Vec<_>>();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        let bay = bay(&panel, &ctx, &strips);
        assert_eq!(bay.count(), DECKS);

        let row = strips_row(rect_of(panel.layout(), "mixer"));
        let mut last: Option<egui::Rect> = None;
        for index in 0..DECKS {
            let at = bay.strip(index).rect;
            assert!(
                row.contains_rect(at),
                "strip {index} at {at:?} is outside the row {row:?}"
            );
            if let Some(last) = last {
                assert!(near(at.width(), last.width()));
                assert!(
                    near(at.min.x - last.max.x, size::STRIP_GAP),
                    "strip {index} is {} after the one before it and `.mixer-strips`'s gap \
                     is {}",
                    at.min.x - last.max.x,
                    size::STRIP_GAP
                );
            }
            last = Some(at);
        }
        // The last strip ends where the row does, which is what says the width
        // and the stride were built from one number.
        assert!(near(last.expect("four strips").max.x, row.max.x));
    }
}

// ---------------------------------------------------------------------------
// As many strips as the deck has
// ---------------------------------------------------------------------------

/// Verifies strip count matches deck values while track width remains fixed at 1/4 row width.
#[test]
fn the_strips_are_the_decks_count_and_a_track_is_a_quarter_either_way() {
    let (panel, ctx) = console(PLAUSIBLE);
    let row = strips_row(rect_of(panel.layout(), "mixer"));
    let track = (row.width() - size::STRIP_GAP * 3.0) / 4.0;

    let all = std::iter::repeat_with(mock).take(DECKS).collect::<Vec<_>>();
    for count in 1..=DECKS {
        let strips = &all[..count];
        let bay = bay(&panel, &ctx, strips);
        assert_eq!(
            bay.count(),
            count,
            "a deck of {count} slots drew {} strips",
            bay.count()
        );
        assert_eq!(bay.placed().count(), count);
        for index in 0..count {
            assert!(
                near(bay.strip(index).rect.width(), track),
                "a bay of {count} strips made strip {index} {} wide, and a quarter of the \
                 row is {track} — the tracks are following the strip count",
                bay.strip(index).rect.width()
            );
        }
        // And the first strip is in the first track at every count, so a bay
        // of one is deck A's strip and not a strip in the middle.
        assert!(near(bay.strip(0).rect.min.x, row.min.x));
    }
}

/// A strip this bay has not got is a panic and not a rectangle — the rule
/// `TransportRow::dot` states about a dot of a grid.
#[test]
#[should_panic(expected = "strip 1 of a mixer of 1")]
fn a_strip_the_deck_has_not_got_is_not_a_rectangle() {
    let (panel, ctx) = console(PLAUSIBLE);
    let strips = vec![mock()];
    bay(&panel, &ctx, &strips).strip(1);
}

// ---------------------------------------------------------------------------
// No deck behind the console
// ---------------------------------------------------------------------------

/// Verifies a console with no deck behind it paints nothing in the mixer bay.
#[test]
fn a_console_with_no_deck_draws_nothing_in_the_bay() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let region = rect_of(panel.layout(), "mixer");
    let row = strips_row(region);
    let width = (row.width() - size::STRIP_GAP * 3.0) / 4.0;

    let mut view = View::new(Room::Day);
    assert!(
        view.mixer.is_empty(),
        "a fresh view has a deck behind it before anybody handed it one"
    );
    let empty = shapes_inside(&mut view, &mut panel, row);
    assert!(
        empty.is_empty(),
        "the bay has no deck behind it and {} shapes were drawn in its body: {empty:#?}",
        empty.len()
    );

    // And `mixer` itself answers the same, so the rule lives in one place
    // rather than being asked here and again in `View::draw`.
    let ctx = drawn_once();
    assert_eq!(mixer(&ctx, panel.layout(), &[]), None);

    for count in 1..=DECKS {
        view.mixer = std::iter::repeat_with(mock).take(count).collect();
        let drawn = shapes_inside(&mut view, &mut panel, row);
        assert_eq!(
            wells(&drawn, width),
            count,
            "a deck of {count} slots drew {} strip wells",
            wells(&drawn, width)
        );
    }

    // And taking the deck away again empties it: the bay keeps nothing from
    // the frame it was drawn with.
    view.mixer.clear();
    assert!(
        shapes_inside(&mut view, &mut panel, row).is_empty(),
        "the deck went away and the bay is still drawing what it last read"
    );
}

/// Before anything has been drawn there are no strips, for `outputs`'s own
/// reason: the tally and the blend are as wide as the words in them, and
/// `Context::fonts` is not valid until the first pass.
#[test]
fn a_bay_that_has_not_been_drawn_is_not_there() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let fresh = egui::Context::default();
    assert_eq!(fresh.cumulative_pass_nr(), 0);
    let strips = mock_strips();
    assert_eq!(mixer(&fresh, panel.layout(), &strips), None);
}

/// The strips are drawn where the bay can hold them, and nowhere else —
/// `picture_rect`'s rule, stated on a bay of strips.
#[test]
fn a_folded_or_soloed_or_narrow_mixer_draws_nothing() {
    let ctx = drawn_once();
    let strips = mock_strips();
    let mut layout = solved(PLAUSIBLE);
    assert!(mixer(&ctx, &layout, &strips).is_some());

    layout.collapse(id_of(&layout, "mixer"));
    layout.solve();
    assert_eq!(
        mixer(&ctx, &layout, &strips),
        None,
        "the mixer is folded away and its strips are still being drawn"
    );

    layout.expand(id_of(&layout, "mixer"));
    layout.solve();
    assert!(mixer(&ctx, &layout, &strips).is_some());

    // A solo somewhere else takes the bay off the panel with it.
    layout.solo(id_of(&layout, "library"));
    layout.solve();
    assert_eq!(mixer(&ctx, &layout, &strips), None);
    layout.unsolo();
    layout.solve();
    assert!(mixer(&ctx, &layout, &strips).is_some());
}

// ---------------------------------------------------------------------------
/// Verifies mixer draws no strips below minimum viewport width while previews remain (ADR-0272).
#[test]
fn under_the_minimum_viewport_the_bay_draws_no_strips_and_the_previews_remain() {
    let (w, h) = karakuri_console::MINIMUM_VIEWPORT;
    let canvas = (1280, 720);
    let strips = std::iter::repeat_with(mock).take(DECKS).collect::<Vec<_>>();
    let ctx = drawn_once();

    let placed = |w: f32| {
        let mut panel = Panel::new(w, h);
        karakuri_console::view::rearrange(&mut panel, canvas);
        let count = mixer(&ctx, panel.layout(), &strips)
            .map(|bay| bay.placed().count())
            .unwrap_or(0);
        let cells = karakuri_console::view::preview_rects(panel.layout(), canvas).is_some();
        (count, cells)
    };

    assert_eq!(
        placed(w),
        (DECKS, true),
        "at the minimum viewport the bay does not draw four strips"
    );
    assert_eq!(
        placed(w - 0.1),
        (0, true),
        "a tenth of a pixel under it, the strips and the cells did not part company"
    );

    // And the strip is what a press reaches a deck through, so with no strips
    // there is nothing on the bay that selects one.
    let mut panel = Panel::new(w - 0.1, h);
    karakuri_console::view::rearrange(&mut panel, canvas);
    let bay = mixer(&ctx, panel.layout(), &strips).expect("the bay is still laid out");
    let region = rect_of(panel.layout(), "mixer");
    for x in 0..20 {
        for y in 0..20 {
            let p = Point::new(
                region.x + region.w * x as f32 / 19.0,
                region.y + region.h * y as f32 / 19.0,
            );
            assert_eq!(bay.select(p), None, "a press at {p:?} selected a deck");
        }
    }
}
