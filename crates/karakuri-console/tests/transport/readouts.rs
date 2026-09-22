use super::transport_common::*;

// ---------------------------------------------------------------------------
// Where the readouts are
// ---------------------------------------------------------------------------

/// The row's furniture is the row's rectangle and the mock's own boxes, and
/// every number here is read off `style.css` rather than off the panel.
///
/// `.transport { display: flex; align-items: center; gap: 14px; padding: 9px
/// 12px }` around a `.bpm` of `font-size: 20px` — 30 tall at the console's
/// `line-height: 1.5`, which is the 30 in the arrangement's `9 + 30 + 9` — and
/// a `.beat-grid` of 15x6 dots with `gap: 4px` between them. So the number
/// starts one padding in, everything after it starts one gap after the thing
/// before it, and every box is centred in the row whatever its own height is.
#[test]
fn the_readouts_are_the_rows_own_geometry() {
    let (panel, ctx) = console(SMALLEST);
    let strip = rect_of(panel.layout(), "transport");
    let row = row(&panel, &ctx);
    let mid = strip.y + strip.h * 0.5;

    // **The transcription itself, against the stylesheet.** Everything below
    // is a *relation* — this box is one gap after that one — and every one of
    // them holds just as well with a gap transcribed wrong, so the numbers the
    // relations are stated in are asserted here as the literals `style.css`
    // has. This is the half of the row that is checked against the mock rather
    // than against itself.
    assert!(
        near(size::TRANSPORT_PAD_X, 12.0),
        "`.transport` is `padding: 9px 12px`"
    );
    assert!(
        near(size::TRANSPORT_PAD_Y, 9.0),
        "`.transport` is `padding: 9px 12px`"
    );
    assert!(
        near(size::TRANSPORT_GAP, 14.0),
        "`.transport` is `gap: 14px`"
    );
    assert!(near(size::BPM_SIZE, 20.0), "`.bpm` is `font-size: 20px`");
    assert!(near(size::BEAT_W, 15.0), "`.beat-grid i` is `width: 15px`");
    assert!(near(size::BEAT_H, 6.0), "`.beat-grid i` is `height: 6px`");
    assert!(near(size::BEAT_GAP, 4.0), "`.beat-grid` is `gap: 4px`");

    // The number: `.transport`'s left padding, 30 tall, centred.
    assert!(
        near(row.bpm.min.x, strip.x + size::TRANSPORT_PAD_X),
        "the tempo starts at {} and the row's padding leaves {}",
        row.bpm.min.x,
        strip.x + size::TRANSPORT_PAD_X
    );
    assert!(
        near(row.bpm.height(), 30.0),
        "the tempo's box is {} tall and `.bpm` is 20px at line-height 1.5",
        row.bpm.height()
    );
    assert!(near(row.bpm.center().y, mid));

    // **And that 30 is the row's 48**, which is the same derivation the
    // arrangement's transport was written from: 9 + 30 + 9. The centring gives
    // the padding back exactly here, where the Outputs row's 8 + 18.5 + 8 has
    // to spend a quarter pixel.
    assert!(near(strip.h, 48.0), "the transport row is {} tall", strip.h);
    assert!(near(row.bpm.min.y - strip.y, size::TRANSPORT_PAD_Y));
    assert!(near(
        strip.y + strip.h - row.bpm.max.y,
        size::TRANSPORT_PAD_Y
    ));

    // Every pair is one `.transport` gap apart, in writing order.
    for (before, after, what) in [
        (row.bpm, row.label, "the tempo and its label"),
        (row.label, row.grid, "the label and the beat grid"),
        (row.grid, row.bar, "the beat grid and the bar"),
    ] {
        assert!(
            near(after.min.x - before.max.x, size::TRANSPORT_GAP),
            "{what} are {} apart and `.transport`'s gap is {}",
            after.min.x - before.max.x,
            size::TRANSPORT_GAP
        );
    }

    // Every one of them centred in the row, whatever its own height is.
    for (rect, what) in [
        (row.label, "the BPM label"),
        (row.grid, "the beat grid"),
        (row.bar, "the bar"),
        (row.frame, "the frame readout"),
    ] {
        assert!(
            near(rect.center().y, mid),
            "{what} is not centred in the row"
        );
    }

    // The beat grid: `.beat-grid i`'s 15x6, and the gaps are **between** the
    // dots — four dots have three gaps and not four.
    assert_eq!(row.dots, 4);
    assert!(near(row.grid.height(), size::BEAT_H));
    assert!(
        near(row.grid.width(), size::BEAT_W * 4.0 + size::BEAT_GAP * 3.0),
        "the grid is {} wide and four 15px dots with three 4px gaps are {}",
        row.grid.width(),
        size::BEAT_W * 4.0 + size::BEAT_GAP * 3.0
    );

    // **`.sep { flex: 1 }` puts what follows it against the right padding, and
    // the last of those is the health capsule** — the mock draws `landed`
    // after the frame readout, so the capsule takes the padding and the frame
    // readout is one row gap before it.
    let health = row
        .health
        .expect("the mock's row draws its `landed` capsule");
    assert!(
        near(strip.x + strip.w - health.max.x, size::TRANSPORT_PAD_X),
        "the health capsule ends {} from the right edge and the padding is {}",
        strip.x + strip.w - health.max.x,
        size::TRANSPORT_PAD_X
    );
    assert!(
        near(health.min.x - row.frame.max.x, size::TRANSPORT_GAP),
        "the frame readout and the capsule are {} apart and `.transport`'s gap is {}",
        health.min.x - row.frame.max.x,
        size::TRANSPORT_GAP
    );
    // `.pill`'s own box: `padding: 0 8px` round one word, at the row's type.
    assert!(near(health.height(), size::PILL_H));
    assert!(near(health.center().y, mid), "the capsule is not centred");
    assert!(
        health.width() > size::PILL_PAD_X * 2.0,
        "the capsule is {} wide and its padding alone is {}, so there is no word in it",
        health.width(),
        size::PILL_PAD_X * 2.0
    );
    assert!(
        row.frame.min.x > row.bar.max.x,
        "the frame readout is on top of the bar"
    );

    // And the whole of it is inside the row it is drawn in.
    let strip =
        egui::Rect::from_min_size(egui::pos2(strip.x, strip.y), egui::vec2(strip.w, strip.h));
    for (rect, what) in [
        (row.bpm, "the tempo"),
        (row.label, "the label"),
        (row.grid, "the beat grid"),
        (row.bar, "the bar"),
        (row.frame, "the frame readout"),
        (health, "the health capsule"),
    ] {
        assert!(
            strip.contains_rect(rect),
            "{what} at {rect:?} is not inside the row {strip:?}"
        );
    }
}

/// The dots tile their grid and never overlap, which is the same gap arithmetic
/// `preview_cells` is checked for and the same wrong version: a gap per dot
/// rather than a gap between two.
#[test]
fn the_dots_tile_the_grid_and_never_overlap() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        let row = row(&panel, &ctx);

        let mut last: Option<egui::Rect> = None;
        for index in 0..row.dots {
            let dot = row.dot(index);
            assert!(near(dot.width(), size::BEAT_W));
            assert!(near(dot.height(), size::BEAT_H));
            assert!(
                row.grid.contains_rect(dot),
                "dot {index} at {dot:?} is outside the grid {:?}",
                row.grid
            );
            if let Some(last) = last {
                assert!(
                    near(dot.min.x - last.max.x, size::BEAT_GAP),
                    "dot {index} is {} after the one before it and `.beat-grid`'s gap is {}",
                    dot.min.x - last.max.x,
                    size::BEAT_GAP
                );
            }
            last = Some(dot);
        }
        // The last dot ends where the grid does, which is what says the width
        // and the stride were built from one number.
        assert!(near(last.expect("four dots").max.x, row.grid.max.x));
    }
}

/// The readouts move with the row and not with the window, which is the failure
/// a rectangle taken once looks exactly like until somebody drags something.
#[test]
fn the_readouts_follow_the_row() {
    let (panel, ctx) = console(SMALLEST);
    let narrow = row(&panel, &ctx);

    let (panel, ctx) = console(PLAUSIBLE);
    let wide = row(&panel, &ctx);

    // The left of the row is the left of the window either way, so the tempo
    // does not move; the frame readout is against the right edge, so it does.
    assert!(near(narrow.bpm.min.x, wide.bpm.min.x));
    assert!(
        wide.frame.min.x > narrow.frame.min.x + 600.0,
        "the frame readout is at {} in a 1920 window and {} in a {} one, so it is not \
         following the row's right edge",
        wide.frame.min.x,
        narrow.frame.min.x,
        SMALLEST.w
    );
}

// ---------------------------------------------------------------------------
// No engine behind the console
// ---------------------------------------------------------------------------

/// With no engine behind the console the row is empty, and that is asserted by
/// drawing it.
///
/// `View::transport` is `None` in every test in this crate and in the whole of
/// `cargo test -p karakuri-console`, which is a console with nothing running
/// behind it. What it draws then is the card and nothing else — not a row of
/// zeroes, which would be a tempo nothing is running at and a frame time
/// nothing measured, and not a row of dashes, which is the same invention with
/// a different glyph.
///
/// So this counts what lands inside the row on a frame drawn each way. One
/// shape with nothing behind it, which is the card; more than one with the
/// mock's values, which is the five readouts — and the difference is the whole
/// claim.
#[test]
fn a_console_with_no_engine_draws_nothing_in_the_row() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let strip = rect_of(panel.layout(), "transport");
    let strip =
        egui::Rect::from_min_size(egui::pos2(strip.x, strip.y), egui::vec2(strip.w, strip.h));

    let mut view = View::new(Room::Day);
    assert_eq!(
        view.transport, None,
        "a fresh view has an engine behind it before anybody handed it one"
    );
    let empty = shapes_inside(&mut view, &mut panel, strip);
    assert_eq!(
        empty.len(),
        1,
        "the row has no engine behind it and {} shapes were drawn in it: {empty:#?}",
        empty.len()
    );

    // And the one shape is the card, which is what every other empty body in
    // this pass looks like: a filled rectangle the size of the region.
    match &empty[0] {
        egui::Shape::Rect(card) => assert!(
            near(card.rect.width(), strip.width()) && near(card.rect.height(), strip.height()),
            "the one shape in the empty row is a {:?} and not the row's card",
            card.rect
        ),
        other => panic!("the one shape in the empty row is {other:?}"),
    }

    // With the engine's numbers, the same frame draws the readouts. Strictly
    // more, and the assertion is the direction rather than a count of shapes
    // `egui` is free to tessellate differently.
    view.transport = Some(mock());
    let drawn = shapes_inside(&mut view, &mut panel, strip);
    assert!(
        drawn.len() > empty.len(),
        "the row was handed a tempo, a beat and a frame time and drew {} shapes, the same \
         as with nothing behind it",
        drawn.len()
    );
    // The four dots are in there, which is what says the *grid* is drawn and
    // not only the type: four boxes 15 by 6.
    let dots = drawn
        .iter()
        .filter(|shape| {
            let r = shape.visual_bounding_rect();
            near(r.width(), size::BEAT_W) && near(r.height(), size::BEAT_H)
        })
        .count();
    assert_eq!(dots, 4, "the beat grid drew {dots} dots");

    // And taking the engine away again empties it: the row keeps nothing from
    // the frame it was drawn with.
    view.transport = None;
    assert_eq!(
        shapes_inside(&mut view, &mut panel, strip).len(),
        1,
        "the engine went away and the row is still drawing what it last read"
    );
}

/// No values, no row, asked of the derivation rather than of the paint pass —
/// the same rule `View::picture` follows, stated where a caller can reach it.
#[test]
fn no_values_is_no_row() {
    let (panel, ctx) = console(PLAUSIBLE);
    assert_eq!(transport(&ctx, panel.layout(), None), None);
    assert!(transport(&ctx, panel.layout(), Some(mock())).is_some());
}

/// Before anything has been drawn there is no row, for `outputs`'s own reason:
/// the readouts are as wide as the type in them, the type has not been laid
/// out, and `Context::fonts` is not valid until the first pass. A window loop
/// has drawn long before the first frame of the engine's is measured.
#[test]
fn a_row_that_has_not_been_drawn_is_not_there() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let fresh = egui::Context::default();
    assert_eq!(fresh.cumulative_pass_nr(), 0);
    assert_eq!(transport(&fresh, panel.layout(), Some(mock())), None);
}

/// The row is drawn where the row can hold it, and nowhere else —
/// `picture_rect`'s rule, stated on a row of readouts.
#[test]
fn a_folded_or_soloed_or_narrow_row_draws_nothing() {
    let ctx = drawn_once();
    let mut layout = solved(PLAUSIBLE);
    assert!(transport(&ctx, &layout, Some(mock())).is_some());

    layout.collapse(id_of(&layout, "transport"));
    layout.solve();
    assert_eq!(
        transport(&ctx, &layout, Some(mock())),
        None,
        "the transport row is folded away and its readouts are still being drawn"
    );

    layout.expand(id_of(&layout, "transport"));
    layout.solve();
    assert!(transport(&ctx, &layout, Some(mock())).is_some());

    // A solo somewhere else takes the row off the panel with it.
    layout.solo(id_of(&layout, "library"));
    layout.solve();
    assert_eq!(transport(&ctx, &layout, Some(mock())), None);
    layout.unsolo();
    layout.solve();
    assert!(transport(&ctx, &layout, Some(mock())).is_some());

    // **And a window too narrow to keep the frame readout clear of the bar.**
    // The mock answers that by wrapping the row onto a second line; this row
    // is 48 tall and has nowhere to put one, so it draws nothing rather than
    // two readouts on top of each other.
    let narrow = solved(Rect {
        w: 240.0,
        ..PLAUSIBLE
    });
    assert_eq!(transport(&ctx, &narrow, Some(mock())), None);
}

// ---------------------------------------------------------------------------
// What the last write did
// ---------------------------------------------------------------------------

/// The capsule draws the verdict it was handed, in the three words the page
/// uses, and draws nothing at all where there is none.
///
/// The four states are asserted through the paint pass rather than off
/// [`TransportRow::health`], because a rectangle is not a drawing: the
/// rectangle was there before this pill was painted into it, and a version of
/// [`view::transport_into`] that laid the capsule out and never painted it
/// would satisfy every geometric assertion in this file. So this reads the
/// words off the frame, which is `a_missing_rate_or_budget_drops_its_own_words`
/// one item along and the same reason.
///
/// `None` is the state worth the most here. A run in which nobody has rewritten
/// a procedure is most of most runs, and the failure it guards against is an
/// `armed` capsule reading `landed` about a build nobody made — the panel
/// asserting on startup that a write it never saw is on screen.
#[test]
fn the_health_capsule_draws_the_verdict_and_nothing_where_there_is_none() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let strip = rect_of(panel.layout(), "transport");
    let strip =
        egui::Rect::from_min_size(egui::pos2(strip.x, strip.y), egui::vec2(strip.w, strip.h));
    let mut view = View::new(Room::Day);

    let words = |view: &mut View, panel: &mut Panel| -> Vec<String> {
        shapes_inside(view, panel, strip)
            .into_iter()
            .filter_map(|shape| match shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    };

    for (stage, word) in [
        (Stage::Landed, "landed"),
        (Stage::Overloaded, "overloaded"),
        (Stage::Refused, "refused"),
        (Stage::NotCompiled, "did not compile"),
    ] {
        view.transport = Some(Transport {
            health: Some(stage),
            ..mock()
        });
        let drawn = words(&mut view, &mut panel);
        assert!(
            drawn.iter().any(|line| line == word),
            "the row was handed {stage:?} and drew {drawn:?}"
        );
        // And exactly that one of the three, which is what says the match is
        // a mapping rather than a constant that happens to be right once.
        for other in ["landed", "overloaded", "refused", "did not compile"] {
            assert_eq!(
                other == word,
                drawn.iter().any(|line| line == other),
                "the row was handed {stage:?} and `{other}` is on it: {drawn:?}"
            );
        }
    }

    // Nothing written yet: no capsule, no word, and no box for one.
    view.transport = Some(Transport {
        health: None,
        ..mock()
    });
    let quiet = words(&mut view, &mut panel);
    for word in ["landed", "overloaded", "refused", "did not compile"] {
        assert!(
            !quiet.iter().any(|line| line == word),
            "nothing has been written and the row says `{word}`: {quiet:?}"
        );
    }
    // The frame readout has the right padding back, which is where `.sep`
    // leaves it when it is the last thing in the row.
    let ctx = drawn_once();
    let row = transport(&ctx, panel.layout(), view.transport).expect("a row");
    assert_eq!(row.health, None);
    assert!(
        near(strip.max.x - row.frame.max.x, size::TRANSPORT_PAD_X),
        "with no capsule the frame readout ends {} from the right edge and the padding is {}",
        strip.max.x - row.frame.max.x,
        size::TRANSPORT_PAD_X
    );
}

/// `.pill.armed` is spent on the one verdict that means the build is on screen,
/// and the other two take the plain `.pill`.
///
/// The mock draws this capsule `armed` and draws it on `landed`, and armed is
/// this console's *live in the good sense* — the wash the audio-in pill wears
/// with an input open. A rollback wearing it would say the opposite of what it
/// means, and nothing about the *word* would be wrong, which is why this is
/// asserted on the fill and not on the text: `landed` and `overloaded` are both
/// spelled correctly by a version that washes both.
#[test]
fn the_capsule_is_washed_only_where_the_build_is_on_screen() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let ctx = drawn_once();
    let mut view = View::new(Room::Day);

    for (stage, washed) in [
        (Stage::Landed, true),
        (Stage::Overloaded, false),
        (Stage::Refused, false),
        // The checker turned the source down, so nothing was built and the
        // picture did not move — which is as far from *this is working* as a
        // verdict gets.
        (Stage::NotCompiled, false),
    ] {
        let values = Transport {
            health: Some(stage),
            ..mock()
        };
        view.transport = Some(values);
        let capsule = transport(&ctx, panel.layout(), Some(values))
            .expect("a row")
            .health
            .expect("a capsule");
        // `rect_filled` against `rect_stroke`: the armed treatment fills the
        // capsule with a wash of the mint and the plain one draws a hairline
        // round nothing.
        let filled = shapes_inside(&mut view, &mut panel, capsule)
            .into_iter()
            .any(|shape| match shape {
                egui::Shape::Rect(rect) => rect.fill.a() > 0,
                _ => false,
            });
        assert_eq!(
            filled, washed,
            "a {stage:?} capsule is {} and the mock washes only the verdict that is on              screen",
            match filled {
                true => "washed",
                false => "not washed",
            }
        );
    }
}

// ---------------------------------------------------------------------------
// Nothing here is a control
// ---------------------------------------------------------------------------

/// Every readout in this row is `egui`'s, unless a boundary has it.
///
/// The console's rule has four claims before `egui`'s: a drag in hand, an open
/// menu, a boundary within `GRAB`, and a control the console draws (ADR-0176).
///
/// This test said *nothing in this row is a control* until the arrangement pill
/// landed, and the sentence has been narrowed twice rather than deleted. It was
/// never an argument that a control could not go here — it was the statement
/// that none had, made where a future control added without a decision about
/// the pointer would fail it. That is exactly what happened, and it failed: the
/// pill is a control in this row, it has a decision about the pointer, and
/// `tests/arrangement_pill.rs` is that decision written down — the clearance it
/// keeps, the boundary band it does not sit in, and the rule an open menu
/// changes.
///
/// The tempo left the list on 2026-09-08 for the same reason, and the decision
/// behind it is `tests/tempo_figure.rs`: the figure is the track, a press along
/// it names a tempo outright, and the band around what the grid is running at
/// is what a press has to land in. What is left here is a beat, a bar, a frame
/// time and what the last write did — four things a press does not act on — and
/// every *other* control the mock draws in this row is one of the things
/// `view::transport` names and does not draw.
///
/// The pill is asked for from the same view, so this is not the old assertion
/// passing because the pill has gone missing: a console with an engine and an
/// arrangement behind it draws the pill, and every probe below is still
/// `egui`'s.
#[test]
fn the_readouts_in_the_transport_row_are_not_controls() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let strip = rect_of(panel.layout(), "transport");
    let row = row(&panel, &ctx);
    // The view this row is drawn from: an engine behind it, so the pill is
    // there to be claimed and this is not the assertion passing on an absence.
    let mut view = showing(&[]);
    view.transport = Some(mock());
    let pill = karakuri_console::view::arrangement(
        &ctx,
        panel.layout(),
        view.transport,
        None,
        None,
        None,
        &view.arrangement,
    )
    .expect("the row draws its one control");
    assert_eq!(
        claim(&mut panel, &ctx, &view, at(pill.pill.center())),
        Claim::Panel,
        "the row's one control is not being claimed, so the probes below prove nothing"
    );

    let probes = [
        // **The tempo is not in this list and left it on 2026-09-08**, which
        // is the same narrowing the pill made and the reason the sentence
        // above is written the way it is: the figure is a control now, a press
        // on it names a tempo, and what it claims is
        // `tests/tempo_figure.rs`'s whole subject.
        (row.label.center(), "the BPM label"),
        (row.grid.center(), "the beat grid"),
        (row.dot(0).center(), "the dot the light is on"),
        (row.dot(row.dots - 1).center(), "the last beat"),
        (row.bar.center(), "the bar"),
        (row.frame.center(), "the frame readout"),
        (
            // Health indicator capsule (readout only, non-interactive).
            row.health
                .expect("the mock's row draws its capsule")
                .center(),
            "the health capsule",
        ),
        (
            // **Past the pill**, which is where the empty middle of this row
            // now starts: the mock's `learn`, `tap` and the rest are still
            // not drawn, and the gap between the one control and the frame
            // readout is what is left of them.
            egui::pos2(pill.pill.max.x + 20.0, row.bar.center().y),
            "the empty middle of the row",
        ),
    ];
    for (probe, what) in probes {
        assert_eq!(
            claim(&mut panel, &ctx, &view, at(probe)),
            Claim::Egui,
            "{what} is being claimed as a control the panel acts on"
        );
    }

    // The boundary **below** the row still has its grab, which is what says
    // the answer above is *not a control* rather than the rule having gone
    // missing: this row's own bottom edge is inside it.
    let below = Point::new(strip.x + strip.w * 0.5, strip.y + strip.h);
    assert_eq!(
        claim(&mut panel, &ctx, &view, below),
        Claim::Panel,
        "the bottom edge of the transport row is not in the grab of the boundary under \
         it, so this test is no longer measuring what it was written for"
    );
}

// ---------------------------------------------------------------------------
// The values are the harness's
// ---------------------------------------------------------------------------

/// Every number on the row came in through the argument, and the console keeps
/// none of them.
///
/// The row is a function of what it was handed and of the arrangement, and of
/// nothing else — which is the seam `View::picture` is on, and the reason this
/// crate can be asked about a live console with no device anywhere near it. So
/// this asks the same panel twice with two different transports and gets two
/// different rows, and asks again with the first and gets the first answer
/// back: a console that had kept anything would answer the third call with the
/// second call's tempo.
#[test]
fn the_values_are_the_harnesss_and_are_stored_nowhere() {
    let (panel, ctx) = console(PLAUSIBLE);

    let one = mock();
    let two = Transport {
        // Every field different, and each one visible somewhere in the row: a
        // wider number, a different dot, a different bar, a frame readout
        // with no rate and no budget in it, and no health capsule at all.
        bpm: 92.5,
        beats: 6.0,
        beats_per_bar: 3,
        fps: None,
        frame_ms: 4.0,
        budget_ms: None,
        chain_ms: None,
        health: None,
        rec: None,
    };

    let first = transport(&ctx, panel.layout(), Some(one)).expect("a row");
    let other = transport(&ctx, panel.layout(), Some(two)).expect("a row");
    assert_ne!(
        first, other,
        "two different transports drew the same row, so something in it is not coming \
         from the argument"
    );
    assert_eq!(first.at, 0.0);
    assert_eq!(other.at, 0.0);
    assert_eq!(other.dots, 3, "the grid did not follow the bar's beats");
    assert_ne!(
        first.bpm.width(),
        other.bpm.width(),
        "128.0 and 92.5 laid out to the same width"
    );
    assert_ne!(
        first.frame.width(),
        other.frame.width(),
        "`58 fps · cpu 12.4/16.6 ms` and `cpu 4.0 ms` laid out to the same width"
    );

    // Asked again with the first, and it is the first answer: nothing was
    // written down in between.
    assert_eq!(
        transport(&ctx, panel.layout(), Some(one)).expect("a row"),
        first,
        "the row remembered the transport it was last asked about"
    );

    // And the view holds exactly what a caller put there — a plain field, not
    // a copy the console maintains.
    let mut view = View::new(Room::Day);
    assert_eq!(view.transport, None);
    view.transport = Some(one);
    assert_eq!(view.transport, Some(one));
    view.transport = None;
    assert_eq!(view.transport, None);
}
