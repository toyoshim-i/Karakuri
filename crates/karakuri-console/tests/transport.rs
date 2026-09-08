//! **The transport row: its readouts, and nothing at all where there is no
//! engine.**
//!
//! Five things, and the first two are why this file exists rather than a few
//! more assertions in `view.rs`:
//!
//! 1. **That a console with no engine behind it draws nothing there** — not a
//!    row of zeroes and not a row of dashes, either of which is a reading
//!    invented for a panel that has none. It is asserted by drawing a frame
//!    and counting what landed in the row, because *nothing is drawn* is a
//!    claim about the paint pass and not about a rectangle.
//! 2. **That the beat grid draws the position it is handed**, across a bar
//!    boundary, at beat zero, and at the two edges arithmetic on an `f64` gets
//!    wrong — and that what it draws between two beats is a light on its way
//!    from one dot to the next rather than a dot switching
//!    ([ADR-0212](../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)).
//! 3. Where everything in the row is, derived from the row's own geometry and
//!    the mock's boxes.
//! 4. **That the readouts are not controls**, which is the answer stated
//!    rather than inferred from the absence of a hit test: `claim` gives every
//!    point of this row to `egui` unless a boundary has it — or unless it is
//!    on one of the row's controls, which have files of their own:
//!    `tests/arrangement_pill.rs` for the pill, and `tests/tempo_figure.rs`
//!    for the tempo, which was in the list here until the figure became the
//!    track a press names a tempo along.
//! 5. That the values are the harness's and the console keeps no copy.
//!
//! None of it needs a window or a device. It does need `egui`'s fonts, because
//! where each readout ends is where the next one starts — see
//! `common::drawn_once` — and one of the tests runs a whole frame through
//! `Context::run_ui`, which is what `drawn_once` already does with an empty
//! closure.

mod common;

use common::{drawn_once, id_of, near, rect_of, showing, solved, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::Panel;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{beat_at, transport, Stage, Transport, TransportRow, View};
use karakuri_layout::{Point, Rect};

/// **The mock's own transport, as numbers**, and this file's name for
/// [`common::mock_transport`] — which is where the numbers are, because three
/// test files in this crate wanted the same console's tempo and two of them had
/// their own copy of it.
fn mock() -> Transport {
    common::mock_transport()
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

/// **The position of the light and the bar are one number read two ways**, and
/// this is that number walked across a bar boundary.
///
/// Four beats to the bar, so beats 0 through 3 are bar 1 and beat 4 is bar 2 —
/// the boundary is where a position that counted from the session instead of
/// from the bar shows itself, and where a bar that counted from zero does.
/// **The half-beats are the half of this the grid used to throw away**: the
/// light is between two dots at 1.5, and where it is at every instant is what
/// the grid draws now (ADR-0212).
#[test]
fn the_grid_reads_the_position_the_beats_say() {
    for (beats, at, bar) in [
        (0.0, 0.0, 1),
        (0.5, 0.5, 1),
        (1.0, 1.0, 1),
        (1.5, 1.5, 1),
        (3.0, 3.0, 1),
        // The bar boundary, from both sides.
        (4.0, 0.0, 2),
        (4.25, 0.25, 2),
        (7.0, 3.0, 2),
        (8.0, 0.0, 3),
        // The mock's own reading: the first beat of bar 37.
        (144.0, 0.0, 37),
        (147.0, 3.0, 37),
        (148.0, 0.0, 38),
    ] {
        let t = Transport { beats, ..mock() };
        assert!(
            near(t.position(), at),
            "beat {beats} put the light at {}",
            t.position()
        );
        assert_eq!(t.bar(), bar, "beat {beats} is in bar {}", t.bar());
    }
}

/// **Beat zero is a place on the grid and not the absence of one**, and the
/// two ways an `f64` gets it wrong are here rather than left to be discovered
/// on a panel.
///
/// - **A position a hair before the downbeat.** `(-1e-18).rem_euclid(4.0)` is
///   `4.0` exactly — the true remainder is a hair under the divisor and rounds
///   up to it. As a dot *index* that was one past the end of a four-dot grid
///   and had to be clamped; as a position it is the downbeat, because
///   `beat_at` measures round the cycle and `4.0` is no distance at all from
///   the first dot. **The clamp went with the index** (ADR-0212), so what this
///   asserts now is that the light lands on dot 0 rather than that a number
///   was caught on the way out.
/// - **A position before the session's own zero.** `Oscillator::behind` reads
///   the same grid at an earlier time, which is what a slot warming behind the
///   session is, and a `%` on a negative is negative: a position no grid has.
#[test]
fn beat_zero_and_the_beats_before_it_are_places_on_this_grid() {
    for beats in [0.0, -1e-18, -0.5, -1.0, -4.0, -4.5, -7.9] {
        let t = Transport { beats, ..mock() };
        assert!(
            (0.0..=t.dots() as f32).contains(&t.position()),
            "beat {beats} put the light at {} on a grid of {}",
            t.position(),
            t.dots()
        );
        // Whatever the position is, the light is on the grid: the four dots
        // add up to exactly one dot's worth of light, wherever it is sitting.
        let total: f32 = (0..t.dots())
            .map(|i| beat_at(t.position(), i, t.dots()))
            .sum();
        assert!(
            near(total, 1.0),
            "beat {beats} lit {total} dots' worth of grid"
        );
    }

    // The hair before the downbeat is the downbeat, and it is dot 0 that is
    // lit rather than dot 3 or a rectangle beside the grid.
    let edge = Transport {
        beats: -1e-18,
        ..mock()
    };
    assert!(near(edge.position(), 4.0));
    assert!(near(beat_at(edge.position(), 0, 4), 1.0));
    for index in 1..4 {
        assert_eq!(beat_at(edge.position(), index, 4), 0.0);
    }

    // Named, because each one is a different way of being wrong. Half a beat
    // before the session's zero is half a beat before the downbeat, which is
    // between the last dot and the first — and the bar it is in is the one
    // before the first.
    assert!(near(
        Transport {
            beats: -0.5,
            ..mock()
        }
        .position(),
        3.5
    ));
    assert_eq!(
        Transport {
            beats: -0.5,
            ..mock()
        }
        .bar(),
        0
    );
    assert!(near(
        Transport {
            beats: -4.5,
            ..mock()
        }
        .position(),
        3.5
    ));
    assert_eq!(
        Transport {
            beats: -4.5,
            ..mock()
        }
        .bar(),
        -1
    );

    // And every dot the row hands the painter is one the grid has, at every
    // one of them.
    let (panel, ctx) = console(PLAUSIBLE);
    for beats in [0.0, -1e-18, -0.5, 3.999, 144.0] {
        let values = Transport { beats, ..mock() };
        let row = transport(&ctx, panel.layout(), Some(values)).expect("a row");
        for index in 0..row.dots {
            assert!(row.grid.contains_rect(row.dot(index)));
            assert!(
                (0.0..=1.0).contains(&row.lit(index)),
                "beat {beats} lit dot {index} by {}",
                row.lit(index)
            );
        }
    }
}

/// **The light is a pure function of the position it is handed**, and this is
/// that function on its own — no row, no panel, no `egui` and no clock.
///
/// `beat_at` is [`karakuri_console::view::roll_at`]'s shape one row up
/// (ADR-0212), and the four claims below are the presentation: it peaks under
/// the light, it is exactly dark a dot pitch away, the grid's total light is
/// constant wherever the light is sitting, and it measures **round the
/// cycle** so the last dot and the first are one pitch apart rather than
/// three.
///
/// **Run against its defect**: a falloff measured along the row rather than
/// round it — `(index as f32 - at).abs()` — fails the wrap with *"the light at
/// 3.5 of 4 lit dot 0 by 0"*, which is the light disappearing off the
/// right-hand end of the grid and reappearing at the left a quarter of a beat
/// later.
#[test]
fn the_light_is_a_pure_function_of_the_position() {
    // On a dot: all of the light, and none anywhere else.
    for index in 0..4 {
        assert!(near(beat_at(index as f32, index, 4), 1.0));
        for other in 0..4 {
            if other != index {
                assert_eq!(
                    beat_at(index as f32, other, 4),
                    0.0,
                    "the light on dot {index} reached dot {other}"
                );
            }
        }
    }

    // Between two dots: both of them, and nothing else. Halfway is half each,
    // which is the raised cosine's own midpoint.
    assert!(near(beat_at(1.5, 1, 4), 0.5));
    assert!(near(beat_at(1.5, 2, 4), 0.5));
    assert_eq!(beat_at(1.5, 0, 4), 0.0);
    assert_eq!(beat_at(1.5, 3, 4), 0.0);

    // **Round the cycle.** Between the last dot and the first, which is one
    // pitch and not three: the light leaves the right-hand end and arrives at
    // the left in the same instant.
    assert!(near(beat_at(3.5, 3, 4), 0.5));
    assert!(
        near(beat_at(3.5, 0, 4), 0.5),
        "the light at 3.5 of 4 lit dot 0 by {}",
        beat_at(3.5, 0, 4)
    );
    assert_eq!(beat_at(3.5, 1, 4), 0.0);
    assert_eq!(beat_at(3.5, 2, 4), 0.0);

    // **The grid's total light is constant**, walked round a whole bar: the
    // row does not brighten and dim as the light travels, so a still grid at
    // half brightness is not a picture this can draw and a stop is visible.
    for step in 0..=64 {
        let at = step as f32 / 64.0 * 4.0;
        let total: f32 = (0..4).map(|index| beat_at(at, index, 4)).sum();
        assert!(
            near(total, 1.0),
            "the light at {at} lit {total} dots' worth of grid"
        );
        // And never more than two dots at once, which is what makes it a
        // light with a position rather than a row that pulses.
        assert!((0..4).filter(|&index| beat_at(at, index, 4) > 0.0).count() <= 2);
    }

    // A bar of one beat is a grid of one dot, and it holds all of the light
    // wherever the position is — the nearest thing to a grid there is.
    for at in [0.0, 0.25, 0.5, 0.99] {
        assert!(near(beat_at(at, 0, 1), 1.0));
    }
}

/// **At the instant of a beat the grid is the mock's own picture**, which is
/// the claim above followed all the way to the paint pass.
///
/// `Transport::position` is arithmetic and `TransportRow::at` is that
/// arithmetic carried, and neither of them is a colour: a grid that read the
/// right position and painted the wrong dot would pass every assertion above.
/// So this draws the frame and counts the dots by their fill — `.beat-grid i`
/// is `--c-line`, `.beat-grid i.on` is `--c-pink` — and walks the light from
/// beat to beat, checking each time that **exactly one** dot is fully lit,
/// that it is the one the light is on, and that the other three are exactly
/// the unlit colour.
///
/// **That is the mock's markup**, `<i class="on"></i><i></i><i></i><i></i>`,
/// and it is why the mock did not have to be contradicted to make the beat
/// continuous: it is a frame of the travel rather than a different drawing
/// (ADR-0212). What happens between two of these frames is
/// `between_two_beats_the_light_is_on_two_dots_and_the_halo_follows_it`.
#[test]
fn at_the_instant_of_a_beat_the_grid_is_the_mocks_picture() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let strip = rect_of(panel.layout(), "transport");
    let strip =
        egui::Rect::from_min_size(egui::pos2(strip.x, strip.y), egui::vec2(strip.w, strip.h));
    let pal = Room::Day.palette();
    let mut view = View::new(Room::Day);

    for (beats, beat) in [(0.0, 0), (1.0, 1), (2.0, 2), (3.0, 3), (4.0, 0), (147.0, 3)] {
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

        // The halo: one, in `--c-glowp` at full strength, at the blur the CSS
        // names, and on the dot the light is on. It is scaled by how much of
        // the light is on a dot, so *one at full strength* is the instant of
        // a beat and nothing else.
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

/// **Between two beats the light is on both dots, and the halo goes with
/// it** — which is the half of this presentation the mock's still picture
/// cannot show and the whole of what P-0094 asked for.
///
/// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
/// prefers continuous movement to a discrete flip because a flip proves
/// liveness *across an interval* — an observer knows the panel was alive
/// between two flips — where movement proves it *at every instant*. So the
/// frame drawn halfway between two beats has to be a different picture from
/// the frame drawn at either of them, and a picture nothing else on this panel
/// draws: two dots part lit, two halos, and a total light of exactly one dot.
///
/// **Run against its defect**: painting the fill as
/// `match lit > 0.5 { true => pal.pink, false => pal.line }` — a flip with a
/// threshold, which is what a continuous position drawn discretely comes to —
/// fails with *"halfway between two beats painted 0 dots between the two
/// colours, so the light is switching rather than travelling"*.
#[test]
fn between_two_beats_the_light_is_on_two_dots_and_the_halo_follows_it() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let strip = rect_of(panel.layout(), "transport");
    let strip =
        egui::Rect::from_min_size(egui::pos2(strip.x, strip.y), egui::vec2(strip.w, strip.h));
    let pal = Room::Day.palette();
    let mut view = View::new(Room::Day);

    // Halfway between the second dot and the third, and halfway between the
    // last dot and the first — the second is the one that crosses the bar, and
    // the light arrives at the left-hand end as it leaves the right-hand one.
    for (beats, pair) in [(1.5, (1, 2)), (3.5, (3, 0))] {
        let values = Transport { beats, ..mock() };
        view.transport = Some(values);
        let row = transport(&drawn_once(), panel.layout(), Some(values)).expect("a row");

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

        // **Two dots between the two colours**, and they are the two the
        // light is between.
        let between: Vec<egui::Rect> = dots
            .iter()
            .filter(|(_, fill)| *fill != pal.line && *fill != pal.pink)
            .map(|(rect, _)| *rect)
            .collect();
        assert_eq!(
            between.len(),
            2,
            "halfway between two beats painted {} dots between the two colours, so the \
             light is switching rather than travelling",
            between.len()
        );
        assert!(between.contains(&row.dot(pair.0)) && between.contains(&row.dot(pair.1)));
        assert!(near(row.lit(pair.0), 0.5) && near(row.lit(pair.1), 0.5));

        // **Two halos**, one under each, and each of them fainter than the
        // one a whole beat draws — the light is spread across the two rather
        // than lit twice over.
        let halos: Vec<&egui::epaint::RectShape> = painted
            .iter()
            .filter(|rect| rect.blur_width > 0.0)
            .collect();
        assert_eq!(
            halos.len(),
            2,
            "beat {beats} drew {} halos and the light is on two dots",
            halos.len()
        );
        for halo in &halos {
            assert!(near(halo.blur_width, size::BEAT_GLOW as f32));
            assert!(
                halo.fill.a() < pal.glow_pink.a(),
                "a dot with half the light on it drew a halo at full strength"
            );
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
        assert!(near(values.position(), beat as f32));
        assert_eq!(values.bar(), bar);

        let row = transport(&ctx, panel.layout(), Some(values)).expect("a row");
        assert_eq!(row.dots, beats_per_bar);
        assert!(near(row.at, beat as f32));
        assert!(near(row.lit(beat), 1.0), "the light is not on dot {beat}");
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
    assert_eq!(none.position(), 0.0);
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
// What the last write did
// ---------------------------------------------------------------------------

/// **The capsule draws the verdict it was handed, in the three words the page
/// uses, and draws nothing at all where there is none.**
///
/// The four states are asserted through the paint pass rather than off
/// [`TransportRow::health`], because a rectangle is not a drawing: the
/// rectangle was there before this pill was painted into it, and a version of
/// [`view::transport_into`] that laid the capsule out and never painted it
/// would satisfy every geometric assertion in this file. So this reads the
/// words off the frame, which is `a_missing_rate_or_budget_drops_its_own_words`
/// one item along and the same reason.
///
/// **`None` is the state worth the most here.** A run in which nobody has
/// rewritten a procedure is most of most runs, and the failure it guards
/// against is an `armed` capsule reading `landed` about a build nobody made —
/// the panel asserting on startup that a write it never saw is on screen.
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
        (Stage::RolledBack, "rolled back"),
        (Stage::Refused, "refused"),
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
        for other in ["landed", "rolled back", "refused"] {
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
    for word in ["landed", "rolled back", "refused"] {
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

/// **`.pill.armed` is spent on the one verdict that means the build is on
/// screen**, and the other two take the plain `.pill`.
///
/// The mock draws this capsule `armed` and draws it on `landed`, and armed is
/// this console's *live in the good sense* — the wash the audio-in pill wears
/// with an input open. A rollback wearing it would say the opposite of what it
/// means, and nothing about the *word* would be wrong, which is why this is
/// asserted on the fill and not on the text: `landed` and `rolled back` are
/// both spelled correctly by a version that washes both.
#[test]
fn the_capsule_is_washed_only_where_the_build_is_on_screen() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let ctx = drawn_once();
    let mut view = View::new(Room::Day);

    for (stage, washed) in [
        (Stage::Landed, true),
        (Stage::RolledBack, false),
        (Stage::Refused, false),
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

/// **Every readout in this row is `egui`'s, unless a boundary has it.**
///
/// The console's rule has four claims before `egui`'s: a drag in hand, an open
/// menu, a boundary within `GRAB`, and a control the console draws (ADR-0176).
///
/// **This test said *nothing in this row is a control* until the arrangement
/// pill landed, and the sentence has been narrowed twice rather than
/// deleted.** It was never an argument that a control could not go here — it
/// was the statement that none had, made where a future control added without
/// a decision about the pointer would fail it. That is exactly what happened,
/// and it failed: the pill is a control in this row, it has a decision about
/// the pointer, and `tests/arrangement_pill.rs` is that decision written down
/// — the clearance it keeps, the boundary band it does not sit in, and the
/// rule an open menu changes.
///
/// **The tempo left the list on 2026-09-08 for the same reason**, and the
/// decision behind it is `tests/tempo_figure.rs`: the figure is the track, a
/// press along it names a tempo outright, and the band around what the grid is
/// running at is what a press has to land in. What is left here is a beat, a
/// bar, a frame time and what the last write did — four things a press does
/// not act on — and every *other* control the mock draws in this row is one of
/// the things `view::transport` names and does not draw.
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
            // **The health capsule**, which is drawn as a capsule and is not
            // one: it is what the last write did, and there is nothing to
            // press. It is in this list rather than in a sentence for the
            // reason the four above it are — the answer is stated where a
            // control added here without a decision about the pointer would
            // fail it.
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
        // wider number, a different dot, a different bar, a frame readout
        // with no rate and no budget in it, and no health capsule at all.
        bpm: 92.5,
        beats: 6.0,
        beats_per_bar: 3,
        fps: None,
        frame_ms: 4.0,
        budget_ms: None,
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
