//! **The transport row: four readouts, no controls, and nothing at all where
//! there is no engine.**
//!
//! Five things, and the first two are why this file exists rather than a few
//! more assertions in `view.rs`:
//!
//! 1. **That a console with no engine behind it draws nothing there** — not a
//!    row of zeroes and not a row of dashes, either of which is a reading
//!    invented for a panel that has none. It is asserted by drawing a frame
//!    and counting what landed in the row, because *nothing is drawn* is a
//!    claim about the paint pass and not about a rectangle.
//! 2. **That the beat grid lights the beat the position says**, across a bar
//!    boundary, at beat zero, and at the two edges arithmetic on an `f64` gets
//!    wrong.
//! 3. Where everything in the row is, derived from the row's own geometry and
//!    the mock's boxes.
//! 4. **That nothing in it is a control**, which is the answer stated rather
//!    than inferred from the absence of a hit test: `claim` gives every point
//!    of this row to `egui` unless a boundary has it.
//! 5. That the values are the harness's and the console keeps no copy.
//!
//! None of it needs a window or a device. It does need `egui`'s fonts, because
//! where each readout ends is where the next one starts — see
//! `common::drawn_once` — and one of the tests runs a whole frame through
//! `Context::run_ui`, which is what `drawn_once` already does with an empty
//! closure.

mod common;

use common::{drawn_once, id_of, near, rect_of, solved, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::Panel;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{transport, Transport, TransportRow, View};
use karakuri_layout::{Point, Rect};

/// **The mock's own transport, as numbers**: `128.0 BPM`, the first beat of
/// bar 37 lit, and `58 fps · 12.4/16.6 ms`.
///
/// Bar 37 beat 0 is 36 whole bars of four beats — 144 — which is the one place
/// in this file where a beat count is written rather than derived, and it is
/// written from the mock's own `bar 37`.
fn mock() -> Transport {
    Transport {
        bpm: 128.0,
        beats: 144.0,
        beats_per_bar: 4,
        fps: Some(58.0),
        frame_ms: 12.4,
        budget_ms: Some(16.6),
    }
}

/// A panel at a viewport, solved, with a context that has drawn once — the
/// pair every test here starts from, and `outputs.rs`'s own opening.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// The row, drawn with the mock's values.
fn row(panel: &Panel, ctx: &egui::Context) -> TransportRow {
    transport(ctx, panel.layout(), Some(mock())).expect("the transport row draws its readouts")
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

// ---------------------------------------------------------------------------
// Where the readouts are
// ---------------------------------------------------------------------------

/// **The row's furniture is the row's rectangle and the mock's own boxes**,
/// and every number here is read off `style.css` rather than off the panel.
///
/// `.transport { display: flex; align-items: center; gap: 14px;
/// padding: 9px 12px }` around a `.bpm` of `font-size: 20px` — 30 tall at the
/// console's `line-height: 1.5`, which is the 30 in the arrangement's
/// `9 + 30 + 9` — and a `.beat-grid` of 15x6 dots with `gap: 4px` between
/// them. So the number starts one padding in, everything after it starts one
/// gap after the thing before it, and every box is centred in the row whatever
/// its own height is.
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

    // The frame readout: `.sep { flex: 1 }` puts it against the right padding.
    assert!(
        near(strip.x + strip.w - row.frame.max.x, size::TRANSPORT_PAD_X),
        "the frame readout ends {} from the right edge and the padding is {}",
        strip.x + strip.w - row.frame.max.x,
        size::TRANSPORT_PAD_X
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
    ] {
        assert!(
            strip.contains_rect(rect),
            "{what} at {rect:?} is not inside the row {strip:?}"
        );
    }
}

/// **The dots tile their grid and never overlap**, which is the same gap
/// arithmetic `preview_cells` is checked for and the same wrong version:
/// a gap per dot rather than a gap between two.
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

/// **The readouts move with the row and not with the window**, which is the
/// failure a rectangle taken once looks exactly like until somebody drags
/// something.
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
        wide.frame.min.x > narrow.frame.min.x + 900.0,
        "the frame readout is at {} in a 1920 window and {} in a 990 one, so it is not \
         following the row's right edge",
        wide.frame.min.x,
        narrow.frame.min.x
    );
}

// ---------------------------------------------------------------------------
// No engine behind the console
// ---------------------------------------------------------------------------

/// Every shape the console paints **wholly inside** `rect`, on one frame.
///
/// A whole frame through `Context::run_ui`, which is all it takes: `egui`
/// tessellates on the CPU and the device only ever sees the result, which is
/// the seam this crate is built on read from the test's end. Containment
/// rather than intersection so that the panel's ground and the card's drop
/// shadow — both larger than the row — are not counted as things drawn in it.
///
/// The texture delta is cleared because `epaint` panics on one dropped
/// unapplied; there is no renderer here, which is the whole of what makes this
/// a test and not a window.
fn shapes_inside(view: &mut View, panel: &mut Panel, rect: egui::Rect) -> Vec<egui::Shape> {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter(|clipped| {
            let bounds = clipped.shape.visual_bounding_rect();
            bounds.is_finite() && rect.contains_rect(bounds)
        })
        .map(|clipped| clipped.shape)
        .collect()
}

/// **With no engine behind the console the row is empty, and that is asserted
/// by drawing it.**
///
/// `View::transport` is `None` in every test in this crate and in the whole of
/// `cargo test -p karakuri-console`, which is a console with nothing running
/// behind it. What it draws then is the card and nothing else — **not** a row
/// of zeroes, which would be a tempo nothing is running at and a frame time
/// nothing measured, and not a row of dashes, which is the same invention with
/// a different glyph.
///
/// So this counts what lands inside the row on a frame drawn each way. One
/// shape with nothing behind it, which is the card; more than one with the
/// mock's values, which is the four readouts — and the difference is the whole
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

/// **No values, no row**, asked of the derivation rather than of the paint
/// pass — the same rule `View::picture` follows, stated where a caller can
/// reach it.
#[test]
fn no_values_is_no_row() {
    let (panel, ctx) = console(PLAUSIBLE);
    assert_eq!(transport(&ctx, panel.layout(), None), None);
    assert!(transport(&ctx, panel.layout(), Some(mock())).is_some());
}

/// **Before anything has been drawn there is no row**, for `outputs`'s own
/// reason: the readouts are as wide as the type in them, the type has not been
/// laid out, and `Context::fonts` is not valid until the first pass. A window
/// loop has drawn long before the first frame of the engine's is measured.
#[test]
fn a_row_that_has_not_been_drawn_is_not_there() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let fresh = egui::Context::default();
    assert_eq!(fresh.cumulative_pass_nr(), 0);
    assert_eq!(transport(&fresh, panel.layout(), Some(mock())), None);
}

/// **The row is drawn where the row can hold it, and nowhere else** —
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
// The beat grid
// ---------------------------------------------------------------------------

/// **The lit dot and the bar are one number read two ways**, and this is that
/// number walked across a bar boundary.
///
/// Four beats to the bar, so beats 0 through 3 are bar 1 and beat 4 is bar 2 —
/// the boundary is where a beat index that counted from the session instead of
/// from the bar shows itself, and where a bar that counted from zero does.
/// The half-beats are there because `beats` is a position and not a count: the
/// grid lights the beat that has started, so 1.5 is still beat 1.
#[test]
fn the_beat_grid_lights_the_beat_the_position_says() {
    for (beats, beat, bar) in [
        (0.0, 0, 1),
        (0.5, 0, 1),
        (1.0, 1, 1),
        (1.5, 1, 1),
        (3.0, 3, 1),
        (3.999, 3, 1),
        // The bar boundary, from both sides.
        (4.0, 0, 2),
        (4.25, 0, 2),
        (7.0, 3, 2),
        (8.0, 0, 3),
        // The mock's own reading: the first beat of bar 37.
        (144.0, 0, 37),
        (147.0, 3, 37),
        (148.0, 0, 38),
    ] {
        let t = Transport { beats, ..mock() };
        assert_eq!(t.beat(), beat, "beat {beats} lights dot {}", t.beat());
        assert_eq!(t.bar(), bar, "beat {beats} is in bar {}", t.bar());
    }
}

/// **Beat zero is a dot and not the absence of one**, and the two ways an
/// `f64` gets it wrong are here rather than left to be discovered on a panel.
///
/// - **A position a hair before the downbeat.** `(-1e-18).rem_euclid(4.0)` is
///   `4.0` exactly — the true remainder is a hair under the divisor and rounds
///   up to it — which is one dot past the end of a four-dot grid: a panic on
///   `TransportRow::dot`, or a dot painted beside the grid.
/// - **A position before the session's own zero.** `Oscillator::behind` reads
///   the same grid at an earlier time, which is what a slot warming behind the
///   session is, and a `%` on a negative is negative: a dot index no grid has.
#[test]
fn beat_zero_and_the_beats_before_it_are_dots_this_grid_has() {
    for beats in [0.0, -1e-18, -0.5, -1.0, -4.0, -4.5, -7.9] {
        let t = Transport { beats, ..mock() };
        assert!(
            t.beat() < t.dots(),
            "beat {beats} lights dot {} of a grid of {}",
            t.beat(),
            t.dots()
        );
    }

    // Named, because each one is a different way of being wrong.
    assert_eq!(
        Transport {
            beats: -1e-18,
            ..mock()
        }
        .beat(),
        3
    );
    assert_eq!(
        Transport {
            beats: -0.5,
            ..mock()
        }
        .beat(),
        3
    );
    assert_eq!(
        Transport {
            beats: -0.5,
            ..mock()
        }
        .bar(),
        0
    );
    assert_eq!(
        Transport {
            beats: -4.5,
            ..mock()
        }
        .beat(),
        3
    );
    assert_eq!(
        Transport {
            beats: -4.5,
            ..mock()
        }
        .bar(),
        -1
    );

    // And the lit dot the row hands the painter is one the grid has, at every
    // one of them.
    let (panel, ctx) = console(PLAUSIBLE);
    for beats in [0.0, -1e-18, -0.5, 3.999, 144.0] {
        let values = Transport { beats, ..mock() };
        let row = transport(&ctx, panel.layout(), Some(values)).expect("a row");
        assert!(row.on < row.dots, "beat {beats} lit dot {} of 4", row.on);
        assert!(row.grid.contains_rect(row.dot(row.on)));
    }
}

/// **The dot the position lights is the one painted pink**, which is the
/// claim above followed all the way to the paint pass.
///
/// `Transport::beat` is arithmetic and `TransportRow::on` is that arithmetic
/// carried, and neither of them is a colour: a grid that lit the right index
/// and painted the wrong dot would pass every assertion above. So this draws
/// the frame and counts the dots by their fill — `.beat-grid i` is
/// `--c-line`, `.beat-grid i.on` is `--c-pink` — and walks the beat across a
/// bar, checking each time that exactly one is lit and that it is the one at
/// the position.
#[test]
fn the_lit_dot_is_the_pink_one_and_there_is_exactly_one() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let strip = rect_of(panel.layout(), "transport");
    let strip =
        egui::Rect::from_min_size(egui::pos2(strip.x, strip.y), egui::vec2(strip.w, strip.h));
    let pal = Room::Day.palette();
    let mut view = View::new(Room::Day);

    for (beats, beat) in [(0.0, 0), (1.0, 1), (2.5, 2), (3.0, 3), (4.0, 0), (147.0, 3)] {
        let values = Transport { beats, ..mock() };
        view.transport = Some(values);
        let row = transport(&drawn_once(), panel.layout(), Some(values)).expect("a row");

        // **Six shapes the size of a dot and not five**: the halo under the
        // lit one is a blurred rectangle of the same size, which is how
        // `.beat-grid i.on`'s `box-shadow: 0 0 9px var(--c-glowp)` is drawn —
        // the same mechanism the Outputs row's dot and every bay's card use.
        // So the two are told apart by their blur, and both are checked.
        let painted: Vec<egui::epaint::RectShape> = shapes_inside(&mut view, &mut panel, strip)
            .into_iter()
            .filter_map(|shape| match shape {
                egui::Shape::Rect(rect)
                    if near(rect.rect.width(), size::BEAT_W)
                        && near(rect.rect.height(), size::BEAT_H) =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .collect();
        let dots: Vec<(egui::Rect, egui::Color32)> = painted
            .iter()
            .filter(|rect| rect.blur_width == 0.0)
            .map(|rect| (rect.rect, rect.fill))
            .collect();
        assert_eq!(dots.len(), 4, "beat {beats} drew {} dots", dots.len());

        // The halo: one, in `--c-glowp`, at the blur the CSS names, and on the
        // dot the beat lit.
        let halos: Vec<&egui::epaint::RectShape> = painted
            .iter()
            .filter(|rect| rect.blur_width > 0.0)
            .collect();
        assert_eq!(
            halos.len(),
            1,
            "beat {beats} drew {} halos and `.beat-grid i.on` is one box-shadow",
            halos.len()
        );
        assert_eq!(halos[0].fill, pal.glow_pink);
        assert!(near(halos[0].blur_width, size::BEAT_GLOW as f32));
        assert_eq!(
            halos[0].rect,
            row.dot(beat),
            "the halo is not on the lit dot"
        );

        let lit: Vec<egui::Rect> = dots
            .iter()
            .filter(|(_, fill)| *fill == pal.pink)
            .map(|(rect, _)| *rect)
            .collect();
        assert_eq!(
            lit.len(),
            1,
            "beat {beats} painted {} dots in `--c-pink`",
            lit.len()
        );
        assert_eq!(
            lit[0],
            row.dot(beat),
            "beat {beats} lit the dot at {:?} and the beat is dot {beat} at {:?}",
            lit[0],
            row.dot(beat)
        );
        // And every other dot is the unlit `--c-line`, rather than some third
        // colour that happens not to be pink.
        for (rect, fill) in &dots {
            if *rect != lit[0] {
                assert_eq!(*fill, pal.line, "an unlit dot is painted {fill:?}");
            }
        }
    }
}

/// **How many beats there are in a bar is the harness's answer and not a
/// constant here**, so a grid of three is three dots and lights the third.
///
/// `karakuri_signal`'s own `BEATS_PER_BAR` says of itself that it is
/// provisional — v0.2 of the IR spec has no time signature anywhere — so a 4
/// written into this crate would be a copy that goes on saying four the day
/// that changes, with nothing on the panel saying so.
#[test]
fn the_grid_is_as_many_dots_as_the_bar_has_beats() {
    let (panel, ctx) = console(PLAUSIBLE);
    for (beats_per_bar, beats, beat, bar) in [(3, 5.0, 2, 2), (4, 5.0, 1, 2), (7, 15.0, 1, 3)] {
        let values = Transport {
            beats_per_bar,
            beats,
            ..mock()
        };
        assert_eq!(values.beat(), beat);
        assert_eq!(values.bar(), bar);

        let row = transport(&ctx, panel.layout(), Some(values)).expect("a row");
        assert_eq!(row.dots, beats_per_bar);
        assert_eq!(row.on, beat);
        assert!(
            near(
                row.grid.width(),
                size::BEAT_W * beats_per_bar as f32 + size::BEAT_GAP * (beats_per_bar - 1) as f32
            ),
            "a grid of {beats_per_bar} is {} wide",
            row.grid.width()
        );
    }

    // A bar of no beats is not a grid, and it is drawn as the nearest thing
    // that is one rather than as a division by zero: `Transport::dots` is
    // where that is decided, once, and every reader goes through it.
    let none = Transport {
        beats_per_bar: 0,
        ..mock()
    };
    assert_eq!(none.dots(), 1);
    assert_eq!(none.beat(), 0);
    let row = transport(&ctx, panel.layout(), Some(none)).expect("a row");
    assert_eq!(row.dots, 1);
    assert!(near(row.grid.width(), size::BEAT_W));
}

/// **A reading nobody has is not drawn as a plausible one**, which is the
/// whole value's rule read at the one place it is a word rather than a shape.
///
/// The mock's `58 fps · 12.4/16.6 ms` has two numbers this console may not
/// have: the rate, which needs a stretch of untouched window to measure, and
/// the budget, which is the display's refresh interval and which `winit` will
/// not always name. Neither absence draws a `0`, a `—`, or the mock's own
/// 16.6 — each drops its own words and leaves the rest of the line.
#[test]
fn a_missing_rate_or_budget_drops_its_own_words_and_nothing_else() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let strip = rect_of(panel.layout(), "transport");
    let strip =
        egui::Rect::from_min_size(egui::pos2(strip.x, strip.y), egui::vec2(strip.w, strip.h));
    let mut view = View::new(Room::Day);

    // Every line of type drawn in the row, as text.
    let words = |view: &mut View, panel: &mut Panel| -> Vec<String> {
        shapes_inside(view, panel, strip)
            .into_iter()
            .filter_map(|shape| match shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    };

    view.transport = Some(mock());
    let all = words(&mut view, &mut panel);
    assert!(
        all.iter().any(|line| line == "58 fps · 12.4/16.6 ms"),
        "the mock's own frame readout is not on the row: {all:?}"
    );
    assert!(all.iter().any(|line| line == "128.0"), "{all:?}");
    assert!(all.iter().any(|line| line == "BPM"), "{all:?}");
    assert!(all.iter().any(|line| line == "bar 37"), "{all:?}");

    // No rate: the `58 fps · ` goes and nothing stands in for it.
    view.transport = Some(Transport {
        fps: None,
        ..mock()
    });
    let no_rate = words(&mut view, &mut panel);
    assert!(
        no_rate.iter().any(|line| line == "12.4/16.6 ms"),
        "with no rate the frame readout reads {no_rate:?}"
    );
    assert!(
        !no_rate.iter().any(|line| line.contains("fps")),
        "there is no rate and the row is drawing one: {no_rate:?}"
    );

    // No budget: the `/16.6` goes, and the frame time keeps its unit.
    view.transport = Some(Transport {
        budget_ms: None,
        ..mock()
    });
    let no_budget = words(&mut view, &mut panel);
    assert!(
        no_budget.iter().any(|line| line == "58 fps · 12.4 ms"),
        "with no budget the frame readout reads {no_budget:?}"
    );
    assert!(
        !no_budget.iter().any(|line| line.contains('/')),
        "nothing knows this display's refresh interval and the row is drawing one: \
         {no_budget:?}"
    );

    // Neither, which is the honest reading on a window that has just been
    // touched and whose display will not say what it refreshes at.
    view.transport = Some(Transport {
        fps: None,
        budget_ms: None,
        ..mock()
    });
    assert!(
        words(&mut view, &mut panel)
            .iter()
            .any(|line| line == "12.4 ms"),
        "with neither, the frame readout is the frame time and its unit"
    );
}

// ---------------------------------------------------------------------------
// Nothing here is a control
// ---------------------------------------------------------------------------

/// **Every point in this row is `egui`'s, unless a boundary has it.**
///
/// The console's rule has three claims before `egui`'s: a drag in hand, a
/// boundary within `GRAB`, and a control the console draws (ADR-0176). This
/// row has no control in it — a tempo, a beat, a bar and a frame time are
/// readouts, and every control the mock draws here is one of the six things
/// `view::transport` names and does not draw — so the third claim never
/// applies and `claim` is unchanged.
///
/// **Stated rather than inferred**, because the absence of a hit test is not
/// an answer anybody can read: a future control added to this row without a
/// decision about the pointer would pass no test at all otherwise, and this
/// one fails.
#[test]
fn nothing_in_the_transport_row_is_a_control() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let strip = rect_of(panel.layout(), "transport");
    let row = row(&panel, &ctx);

    let probes = [
        (row.bpm.center(), "the tempo"),
        (row.label.center(), "the BPM label"),
        (row.grid.center(), "the beat grid"),
        (row.dot(row.on).center(), "the lit beat"),
        (row.dot(row.dots - 1).center(), "the last beat"),
        (row.bar.center(), "the bar"),
        (row.frame.center(), "the frame readout"),
        (
            egui::pos2(row.bar.max.x + 40.0, row.bar.center().y),
            "the empty middle of the row",
        ),
    ];
    for (probe, what) in probes {
        assert_eq!(
            claim(&mut panel, &ctx, at(probe)),
            Claim::Egui,
            "{what} is being claimed as a control the panel acts on"
        );
    }

    // The boundary **below** the row still has its grab, which is what says
    // the answer above is *not a control* rather than the rule having gone
    // missing: this row's own bottom edge is inside it.
    let below = Point::new(strip.x + strip.w * 0.5, strip.y + strip.h);
    assert_eq!(
        claim(&mut panel, &ctx, below),
        Claim::Panel,
        "the bottom edge of the transport row is not in the grab of the boundary under \
         it, so this test is no longer measuring what it was written for"
    );
}

// ---------------------------------------------------------------------------
// The values are the harness's
// ---------------------------------------------------------------------------

/// **Every number on the row came in through the argument, and the console
/// keeps none of them.**
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
        // wider number, a different dot, a different bar, and a frame readout
        // with no rate and no budget in it.
        bpm: 92.5,
        beats: 6.0,
        beats_per_bar: 3,
        fps: None,
        frame_ms: 4.0,
        budget_ms: None,
    };

    let first = transport(&ctx, panel.layout(), Some(one)).expect("a row");
    let other = transport(&ctx, panel.layout(), Some(two)).expect("a row");
    assert_ne!(
        first, other,
        "two different transports drew the same row, so something in it is not coming \
         from the argument"
    );
    assert_eq!(first.on, 0);
    assert_eq!(other.on, 0);
    assert_eq!(other.dots, 3, "the grid did not follow the bar's beats");
    assert_ne!(
        first.bpm.width(),
        other.bpm.width(),
        "128.0 and 92.5 laid out to the same width"
    );
    assert_ne!(
        first.frame.width(),
        other.frame.width(),
        "`58 fps · 12.4/16.6 ms` and `4.0 ms` laid out to the same width"
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
