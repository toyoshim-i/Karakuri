//! **The Mixer bay: as many strips as the deck has, every one of them a
//! readout, and nothing at all where there is no deck.**
//!
//! Six things, and the first two are why this file exists rather than a few
//! more assertions in `view.rs`:
//!
//! 1. **That the strips are the deck's count and not four.** A page has four
//!    tracks whatever the deck holds, and a track with no strip in it draws
//!    nothing — not the empty strip the mock draws, which would be six
//!    readings nobody took.
//! 2. **That a console with no deck behind it draws nothing in the bay's
//!    body** — asserted by drawing a frame and counting what landed in the
//!    strips' own rectangle, because *nothing is drawn* is a claim about the
//!    paint pass and not about a rectangle.
//! 3. Where everything in a strip is, from the mock's own boxes — and the
//!    boxes themselves against the stylesheet's literals, which is a different
//!    claim from the relations stated in terms of them (ADR-0177).
//! 4. **That the faders and the meter follow their values**, at zero, at one
//!    and in between, and that the peak mark never leaves the well.
//! 5. That the tally, the blend and the mask show the state they were given.
//!    **What the tally does when the residency it was given and the one that
//!    was asked for disagree is `parked.rs`**, not here: the strips in this
//!    file are all settled, so nothing in it moves.
//! 6. **Which of these are controls and which are readouts** — the two fader
//!    knobs and nothing else, both directions stated rather than inferred from
//!    the presence or absence of a hit test. What a drag on one *does* is
//!    `tests/fader.rs`.
//!
//! None of it needs a window or a device. It does need `egui`'s fonts, because
//! the tally's capsule and the blend's mini are as wide as the words in them.

mod common;

use common::{drawn_once, id_of, near, rect_of, solved, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::Panel;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{mixer, Level, Mask, Mixer, Strip, StripBox, Tally, View, DECKS};
use karakuri_layout::{Point, Rect};
use karakuri_operation::BlendMode;

/// **The mock's own first strip, as values**: `drift_night` live, a trim at
/// 0.72, the fader at 1.00, `add` over no mask, and a meter reading.
///
/// The numbers are the mock's percentages read as the values behind them —
/// `.trim`'s `width: 72%`, `.strip-num`'s `1.00`, `.vmeter b`'s `74%` and
/// `.vmeter u`'s `82%`.
fn mock() -> Strip {
    Strip {
        name: "drift_night".to_owned(),
        tally: Tally::Live,
        // Settled: the request and the effective residency agree, so nothing
        // in these strips is pending and nothing rolls. What a strip whose
        // two halves disagree draws is `parked.rs`.
        requested: Tally::Live,
        gain: 0.72,
        opacity: 1.0,
        blend: BlendMode::Add,
        mask: Mask::None,
        level: Some(Level {
            mean: 0.74,
            peak: 0.82,
        }),
    }
}

/// The mock's second and third, which are the other two tallies, the other
/// mask, a different blend and — on the third — no reading at all.
fn mock_strips() -> Vec<Strip> {
    vec![
        mock(),
        Strip {
            name: "lattice_veil".to_owned(),
            tally: Tally::Priming,
            requested: Tally::Priming,
            gain: 0.44,
            opacity: 0.3,
            blend: BlendMode::Over,
            mask: Mask::Linear,
            level: Some(Level {
                mean: 0.12,
                peak: 0.12,
            }),
        },
        Strip {
            name: "glass_shell".to_owned(),
            tally: Tally::Allocated,
            requested: Tally::Allocated,
            gain: 0.0,
            opacity: 0.0,
            blend: BlendMode::Add,
            mask: Mask::Radial,
            level: None,
        },
    ]
}

/// A panel at a viewport, solved, with a context that has drawn once — the
/// pair every test here starts from, and `transport.rs`'s own opening.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// The bay, laid out with `strips`.
fn bay<'a>(panel: &Panel, ctx: &egui::Context, strips: &'a [Strip]) -> Mixer<'a> {
    mixer(ctx, panel.layout(), strips).expect("the mixer bay draws its strips")
}

/// The `.mixer-strips` row inside the mixer's region, worked out here from the
/// mock's boxes rather than asked of the crate — so that a row derived wrong
/// is a row in the wrong place rather than two functions agreeing.
fn strips_row(region: Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            region.x + size::STRIPS_PAD,
            region.y + size::HEAD_H + size::STRIPS_PAD,
        ),
        egui::vec2(region.w - size::STRIPS_PAD * 2.0, size::STRIP_H),
    )
}

/// A `karakuri_layout` point, from `egui`'s. Named for what it makes rather
/// than for where it is, because `at` is a strip's box everywhere below.
fn point(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

// ---------------------------------------------------------------------------
// Where a strip is
// ---------------------------------------------------------------------------

/// **A strip is the mock's own boxes**, and every number here is read off
/// `style.css` rather than off the panel.
#[test]
fn a_strip_is_the_mocks_own_boxes() {
    // **The transcription itself, against the stylesheet.** Everything after
    // this is a *relation* — this box is one gap under that one — and every
    // one of them holds just as well with a gap transcribed wrong, so the
    // numbers the relations are stated in are asserted here as the literals
    // `style.css` has. ADR-0177 records the pass where that trap was found.
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
    // around a strip, and `.xfade`'s 61 — so a strip is this tall in both
    // places or in neither, and what is left under the strips is the
    // crossfade row this pass does not draw.
    assert!(
        near(size::STRIP_H, 215.5),
        "a strip is {} tall",
        size::STRIP_H
    );
    assert!(
        near(
            316.0 - size::HEAD_H - size::STRIPS_PAD * 2.0 - size::STRIP_H,
            61.5
        ),
        "what the mixer has left under its strips is not `.xfade`'s 61 — the bay's own \
         27 + 227.5 + 61 was rounded up by the half pixel this is over"
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

/// **The strips tile their row and never overlap**, which is the same gap
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

/// **A page is four tracks wide whatever the deck holds**, so a strip does not
/// get wider because there are fewer of them — and there are exactly as many
/// strips as there are values.
///
/// The one-slot deck is this example's, and the number that would change is a
/// strip's *width*: tracks that followed the count would make one strip four
/// times as wide, which is the alternative `view::mixer` writes down and
/// rejects.
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

/// **A strip this bay has not got is a panic and not a rectangle** — the rule
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

/// Every shape the console paints **wholly inside** `rect`, on one frame.
///
/// `transport.rs`'s own helper, and it is written again here for the reason
/// that one gives: a whole frame through `Context::run_ui`, containment rather
/// than intersection so the bay's card and its head are not counted as things
/// drawn in the body, and the texture delta cleared because `epaint` panics on
/// one dropped unapplied.
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

/// How many `.strip` wells were painted inside `rect`: a filled rectangle a
/// strip's own size.
fn wells(shapes: &[egui::Shape], width: f32) -> usize {
    shapes
        .iter()
        .filter(|shape| match shape {
            egui::Shape::Rect(at) => {
                near(at.rect.height(), size::STRIP_H) && near(at.rect.width(), width)
            }
            _ => false,
        })
        .count()
}

/// **With no deck behind the console the bay's body is empty, and that is
/// asserted by drawing it.**
///
/// `View::mixer` is empty in every test in this crate and in the whole of
/// `cargo test -p karakuri-console`, which is a console with nothing running
/// behind it. What it draws then is the card and the head and **nothing in the
/// body** — not four empty strips, which is what the mock's own fourth strip
/// is and which would be six readings nobody took.
///
/// So this counts what lands inside the strips' own rectangle on a frame drawn
/// with nothing, with one strip and with four. **One well per strip and not
/// one per track** is the whole claim, and it fails in both directions: a bay
/// that always drew four, and a bay that drew none.
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

/// **Before anything has been drawn there are no strips**, for `outputs`'s own
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

/// **The strips are drawn where the bay can hold them, and nowhere else** —
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
// The faders and the meter
// ---------------------------------------------------------------------------

/// **A fader's fill is its value**, at zero, at one and in between — and the
/// two run in different directions, which is the mistake worth catching: the
/// trim fills from the left and the fader fills from the **bottom**.
#[test]
fn the_faders_fill_by_their_values_and_the_tall_one_fills_upwards() {
    let (panel, ctx) = console(PLAUSIBLE);
    let strips = vec![mock()];
    let at = bay(&panel, &ctx, &strips).strip(0);

    for value in [0.0, 0.25, 0.5, 1.0] {
        let trim = at.trim_at(value);
        assert!(
            near(trim.fill.width(), at.trim.width() * value),
            "a trim at {value} filled {} of a {} track",
            trim.fill.width(),
            at.trim.width()
        );
        // From the left, and never taller or shorter than its track.
        assert!(near(trim.fill.min.x, at.trim.min.x));
        assert!(near(trim.fill.height(), at.trim.height()));
        // The knob is centred on the fill's moving edge, which is the one
        // number driving both.
        assert!(near(trim.knob.center().x, trim.fill.max.x));
        assert!(near(trim.knob.center().y, at.trim.center().y));
        assert!(near(trim.knob.width(), size::FADER_KNOB_W));
        assert!(near(trim.knob.height(), size::FADER_KNOB_H));

        // The tall one, whose fill sits `.vfader b`'s 3px inside its track.
        let inner = at.fader.height() - size::VFADER_INSET * 2.0;
        let fader = at.fader_at(value);
        assert!(
            near(fader.fill.height(), inner * value),
            "a fader at {value} filled {} of a {inner} track",
            fader.fill.height()
        );
        assert!(
            near(fader.fill.max.y, at.fader.max.y - size::VFADER_INSET),
            "the fader filled from the top: its fill ends at {} and the track's floor is {}",
            fader.fill.max.y,
            at.fader.max.y - size::VFADER_INSET
        );
        assert!(near(
            fader.fill.width(),
            size::VFADER_W - size::VFADER_INSET * 2.0
        ));
        assert!(near(fader.knob.center().y, fader.fill.min.y));
        assert!(near(fader.knob.center().x, at.fader.center().x));
        assert!(near(fader.knob.width(), size::VFADER_KNOB_W));
        assert!(near(fader.knob.height(), size::VFADER_KNOB_H));
    }

    // A value off either end is the end, and a NaN is zero — a fader whose
    // value is not a number is a broken control, and only one of the two
    // readings available is a fader.
    for (value, filled) in [(-1.0, 0.0), (2.0, 1.0), (f32::NAN, 0.0)] {
        assert!(
            near(at.trim_at(value).fill.width(), at.trim.width() * filled),
            "a trim at {value} is not clamped"
        );
        assert!(near(
            at.fader_at(value).fill.height(),
            (at.fader.height() - size::VFADER_INSET * 2.0) * filled
        ));
    }
}

/// **The meter's column is the mean and its mark is the peak**, and the mark
/// never leaves the well — which is the one thing `.vmeter`'s
/// `overflow: hidden` would hide rather than show, at exactly the reading that
/// matters most.
#[test]
fn the_meters_column_is_the_mean_and_its_mark_is_the_peak() {
    let (panel, ctx) = console(PLAUSIBLE);
    let strips = vec![mock()];
    let at = bay(&panel, &ctx, &strips).strip(0);
    let travel = at.meter.height() - size::VMETER_PEAK_H;

    for (mean, peak) in [(0.0, 0.0), (0.25, 0.4), (0.74, 0.82), (1.0, 1.0)] {
        let meter = at.meter_at(Level { mean, peak });
        assert!(
            near(meter.fill.height(), at.meter.height() * mean),
            "a mean of {mean} filled {} of a {} well",
            meter.fill.height(),
            at.meter.height()
        );
        assert!(
            near(meter.fill.max.y, at.meter.max.y),
            "the meter filled from the top"
        );
        assert!(near(meter.fill.width(), at.meter.width()));

        // The mark: a 2px bar whose own travel is the well less its height, so
        // that a peak of 1.0 sits against the ceiling rather than two pixels
        // above it.
        assert!(near(meter.peak.height(), size::VMETER_PEAK_H));
        assert!(near(meter.peak.width(), at.meter.width()));
        assert!(
            near(meter.peak.max.y, at.meter.max.y - travel * peak),
            "a peak of {peak} put its mark at {} and the well's travel is {travel}",
            meter.peak.max.y
        );
        assert!(
            at.meter.contains_rect(meter.peak),
            "a peak of {peak} put its mark at {:?}, outside the well {:?} — `.vmeter` is \
             `overflow: hidden`, so that mark is not drawn at all",
            meter.peak,
            at.meter
        );
    }

    // A peak above the top is the top, and it is still inside the well.
    let meter = at.meter_at(Level {
        mean: 4.0,
        peak: 4.0,
    });
    assert!(near(meter.fill.height(), at.meter.height()));
    assert!(at.meter.contains_rect(meter.peak));
}

// ---------------------------------------------------------------------------
// The state a strip is given
// ---------------------------------------------------------------------------

/// Every shape painted inside one strip, on a frame drawn with `strips`.
fn strip_shapes(strips: Vec<Strip>, index: usize) -> (StripBox, Vec<egui::Shape>) {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let ctx = drawn_once();
    let at = mixer(&ctx, panel.layout(), &strips)
        .expect("the mixer bay draws its strips")
        .strip(index);
    let mut view = View::new(Room::Day);
    view.mixer = strips;
    // The whole strip and half the gap around it, which is what `strip_into`
    // clips to: a knob is meant to stand a little proud of its track.
    let shapes = shapes_inside(
        &mut view,
        &mut panel,
        at.rect.expand(size::STRIP_GAP * 0.5 + 1.0),
    );
    (at, shapes)
}

/// The galleys painted inside `rect`.
fn galleys(shapes: &[egui::Shape], rect: egui::Rect) -> Vec<std::sync::Arc<egui::Galley>> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(text) if rect.contains(text.pos) => Some(text.galley.clone()),
            _ => None,
        })
        .collect()
}

/// The words painted inside `rect`.
fn words(shapes: &[egui::Shape], rect: egui::Rect) -> Vec<String> {
    galleys(shapes, rect)
        .iter()
        .map(|galley| galley.text().to_owned())
        .collect()
}

/// The filled circles painted inside `rect`, as their radii.
fn discs(shapes: &[egui::Shape], rect: egui::Rect) -> Vec<f32> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Circle(circle) if rect.contains(circle.center) && circle.fill.a() > 0 => {
                Some(circle.radius)
            }
            _ => None,
        })
        .collect()
}

/// **The tally says the residency it was given, in that residency's own
/// colour** — three states, three words and three washes, and the live one is
/// the only one with a halo on it.
#[test]
fn the_tally_shows_the_residency_it_is_given() {
    let pal = Room::Day.palette();
    for (index, tally, word, ink) in [
        (0, Tally::Live, "LIVE", pal.pink),
        (1, Tally::Priming, "PRIM", pal.sun),
        (2, Tally::Allocated, "ALLOC", pal.dim),
    ] {
        let (at, shapes) = strip_shapes(mock_strips(), index);
        let strips = mock_strips();
        assert_eq!(strips[index].tally, tally);

        assert_eq!(
            words(&shapes, at.tally),
            vec![word.to_owned()],
            "a {tally:?} tally does not read {word}"
        );

        // The ink, read off the galley's own fallback colour — every run in
        // these jobs carries its colour, so the one the painter was handed is
        // the one the word is in.
        let painted = shapes.iter().find_map(|shape| match shape {
            egui::Shape::Text(text) if at.tally.contains(text.pos) => Some(text.fallback_color),
            _ => None,
        });
        assert_eq!(
            painted,
            Some(ink),
            "a {tally:?} tally is not in the colour `.tally.{}` gives it",
            tally.word()
        );

        // The halo: `.tally.live` alone carries `box-shadow: 0 0 10px`, and an
        // `epaint` shadow is a rectangle with a blur width on it — which is
        // what tells one from the strip's own well, a rectangle that also
        // contains the tally and is also wider.
        let haloes = shapes
            .iter()
            .filter(|shape| match shape {
                egui::Shape::Rect(rect) => {
                    rect.blur_width > 0.0 && rect.rect.contains_rect(at.tally)
                }
                _ => false,
            })
            .count();
        assert_eq!(
            haloes,
            usize::from(tally == Tally::Live),
            "a {tally:?} tally drew {haloes} haloes, and only a live one carries one"
        );
    }
}

/// **The blend mini says the mode it was given**, in the vocabulary's own
/// lower-case word.
///
/// **The list is now closed and that is the change**: this used to be handed
/// arbitrary strings — `screen` and `multiply` among them — because the field
/// was the engine's `&'static str` and the console had nothing to do with the
/// list but draw it. The chip is a control now (ADR-0187), so it carries a
/// `BlendMode` and the three this walks are every mode there is. A fourth
/// would fail to compile in `BlendMode::name` before it reached here.
#[test]
fn the_blend_mini_shows_the_mode_it_is_given() {
    for blend in BlendMode::ALL {
        let strips = vec![Strip { blend, ..mock() }];
        let (at, shapes) = strip_shapes(strips, 0);
        assert_eq!(
            words(&shapes, at.blend),
            vec![blend.name().to_owned()],
            "the blend mini does not read {}",
            blend.name()
        );
        // And the mini is as wide as the word in it, inside `.mini`'s padding
        // and border — so a longer word is a wider chip and not a clipped one.
        assert!(at.blend.width() > size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0);
    }
    // A wider word is a wider mini, which is what says the chip was measured
    // rather than fixed. `over` is four glyphs and `add` is three.
    let (narrow, _) = strip_shapes(
        vec![Strip {
            blend: BlendMode::Add,
            ..mock()
        }],
        0,
    );
    let (wide, _) = strip_shapes(
        vec![Strip {
            blend: BlendMode::Over,
            ..mock()
        }],
        0,
    );
    assert!(
        wide.blend.width() > narrow.blend.width(),
        "`add` and `over` measured to the same mini"
    );
}

/// **The mask mini shows the mask it was given**, and the three marks are
/// three different marks.
///
/// The outline is drawn for all three — it is the circle the mock's `◯` is —
/// and what tells them apart is what is filled inside it: nothing, a half at
/// the mark's own radius, or a disc at half of it.
#[test]
fn the_mask_mini_shows_the_mask_it_is_given() {
    let r = size::MINI_SIZE * 0.5;
    for (mask, filled) in [
        (Mask::None, vec![]),
        (Mask::Linear, vec![r]),
        (Mask::Radial, vec![r * 0.5]),
    ] {
        let strips = vec![Strip { mask, ..mock() }];
        let (at, shapes) = strip_shapes(strips, 0);
        assert_eq!(
            discs(&shapes, at.mask),
            filled,
            "a {mask:?} mask drew the wrong mark"
        );
        // And every one of them carries the outline, which is a stroked circle
        // of the mark's own radius.
        let outlines = shapes
            .iter()
            .filter(|shape| match shape {
                egui::Shape::Circle(circle) => {
                    at.mask.contains(circle.center)
                        && circle.stroke.width > 0.0
                        && near(circle.radius, r)
                }
                _ => false,
            })
            .count();
        assert_eq!(outlines, 1, "a {mask:?} mask drew {outlines} outlines");
    }
}

/// **The number under the fader is the opacity**, to the two places the mock
/// writes it.
#[test]
fn the_number_is_the_opacity() {
    for (opacity, text) in [(1.0, "1.00"), (0.3, "0.30"), (0.0, "0.00")] {
        let strips = vec![Strip { opacity, ..mock() }];
        let (at, shapes) = strip_shapes(strips, 0);
        assert_eq!(words(&shapes, at.num), vec![text.to_owned()]);
    }
}

/// **A strip with no reading draws the meter's well and nothing in it**, which
/// is the mock's own `alloc` strip — a `.vmeter` with no `b` and no `u`.
#[test]
fn a_strip_with_no_reading_draws_an_empty_meter() {
    let with = strip_shapes(vec![mock()], 0);
    let without = strip_shapes(
        vec![Strip {
            level: None,
            ..mock()
        }],
        0,
    );
    // **The two things a reading puts in the well**, named rather than
    // counted: the mean's column, which is the one `Mesh` in a strip's meter,
    // and the peak's mark, which is the only `--c-pink` thing in it. A meter
    // handed a zero where it was handed nothing draws a mark on the floor,
    // which counts as a shape and is a reading of zero that nobody took.
    let pink = Room::Day.palette().pink;
    let marks = |(at, shapes): &(StripBox, Vec<egui::Shape>)| {
        let inside = |shape: &egui::Shape| at.meter.contains_rect(shape.visual_bounding_rect());
        let column = shapes
            .iter()
            .filter(|shape| inside(shape) && matches!(shape, egui::Shape::Mesh(_)))
            .count();
        let peak = shapes
            .iter()
            .filter(|shape| match shape {
                egui::Shape::Rect(rect) if inside(shape) => rect.fill == pink,
                _ => false,
            })
            .count();
        (column, peak)
    };
    assert_eq!(
        marks(&with),
        (1, 1),
        "a strip with a reading is missing its column or its peak mark"
    );
    assert_eq!(
        marks(&without),
        (0, 0),
        "a strip with no reading drew a column or a peak mark, which is a reading nobody          took"
    );

    // The well itself is still there either way: no reading is not no meter.
    let wells = |(at, shapes): &(StripBox, Vec<egui::Shape>)| {
        shapes
            .iter()
            .filter(|shape| match shape {
                egui::Shape::Rect(rect) => near(rect.rect.width(), size::VMETER_W),
                _ => false,
            })
            .filter(|shape| at.meter.contains_rect(shape.visual_bounding_rect()))
            .count()
    };
    assert!(wells(&without) > 0, "no reading was drawn as no meter");
}

/// **The name is elided rather than wrapped**, which is `.strip-name`'s own
/// `overflow: hidden; text-overflow: ellipsis; white-space: nowrap` — a strip
/// is 53 wide inside its padding and most real names are wider.
#[test]
fn a_name_too_long_for_a_strip_is_elided_on_one_line() {
    let long = "drift_shell + soft_points";
    let strips = vec![Strip {
        name: long.to_owned(),
        ..mock()
    }];
    let (at, shapes) = strip_shapes(strips, 0);
    let painted = galleys(&shapes, at.name);
    assert_eq!(
        painted.len(),
        1,
        "the name was laid out on more than one line"
    );
    assert_eq!(
        painted[0].rows.len(),
        1,
        "`white-space: nowrap` and it wrapped"
    );
    assert!(
        painted[0].elided,
        "`{long}` was laid out whole in a strip {} wide",
        at.name.width()
    );
    assert!(
        painted[0].size().x <= at.name.width(),
        "the name is {} wide in a box of {}",
        painted[0].size().x,
        at.name.width()
    );

    // A name that fits is drawn whole, and an empty one draws nothing at all —
    // a harness with nothing to say rather than a strip with nothing in it.
    let (at, shapes) = strip_shapes(
        vec![Strip {
            name: "a".to_owned(),
            ..mock()
        }],
        0,
    );
    assert_eq!(words(&shapes, at.name), vec!["a".to_owned()]);
    assert!(!galleys(&shapes, at.name)[0].elided);
    let (at, shapes) = strip_shapes(
        vec![Strip {
            name: String::new(),
            ..mock()
        }],
        0,
    );
    assert!(words(&shapes, at.name).is_empty());
}

// ---------------------------------------------------------------------------
// Two controls, and the rest are readouts
// ---------------------------------------------------------------------------

/// **The two knobs and the blend chip are the panel's, and everything else in
/// the bay is `egui`'s.**
///
/// The console's rule has three claims before `egui`'s: a drag in hand, a
/// boundary within `GRAB`, and a control the console draws (ADR-0176). This
/// bay now has three of the third kind — the trim's knob, the fader's knob and
/// the blend chip (ADR-0187) — and nothing else in it: the tally, the mask
/// mini, the meter and the number are readouts, and so is a fader's **track**
/// off the knob, because a press there would be a jump nobody asked for.
///
/// **Stated rather than inferred in both directions.** A knob that stopped
/// being claimed would be a control drawn where it cannot be grabbed, and a
/// bay that claimed everything would take presses it does nothing with.
#[test]
fn the_three_controls_are_claimed_and_the_rest_of_the_bay_is_not() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let strips = mock_strips();
    let bay = bay(&panel, &ctx, &strips);
    let at = bay.strip(0);
    let trim = at.trim_at(mock().gain);
    let fader = at.fader_at(mock().opacity);

    // The three that are.
    for (probe, what) in [
        (trim.knob.center(), "the trim's knob"),
        (fader.knob.center(), "the fader's knob"),
        (at.blend.center(), "the blend chip"),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &strips, point(probe)),
            Claim::Panel,
            "{what} is not being claimed, so it is drawn where it cannot be grabbed"
        );
    }

    // And everything else, including both tracks away from their knobs. The
    // mock's trim is at 0.72 and its fader at 1.00, so the far end of the trim
    // and the floor of the fader are both track and neither is knob.
    let track_end = egui::pos2(at.trim.max.x - 1.0, at.trim.center().y);
    let track_floor = egui::pos2(at.fader.center().x, at.fader.max.y - 1.0);
    let probes = [
        (at.rect.center(), "the strip"),
        (at.name.center(), "the name"),
        (at.tally.center(), "the tally"),
        (track_end, "the trim's track, past the knob"),
        (track_floor, "the fader's track, below the knob"),
        (at.meter.center(), "the meter"),
        (at.num.center(), "the number"),
        (at.mask.center(), "the mask"),
    ];
    for (probe, what) in probes {
        assert_eq!(
            claim(&mut panel, &ctx, &strips, point(probe)),
            Claim::Egui,
            "{what} is being claimed as a control the panel acts on"
        );
    }

    // The two points that are track rather than knob have to actually be off
    // the knob, or the paragraph above is asserting nothing.
    assert!(!trim.knob.contains(track_end));
    assert!(!fader.knob.contains(track_floor));
    // And the mask mini has to actually be off the blend chip, or the list
    // above would be asserting that a control is not one.
    assert!(!at.blend.contains(at.mask.center()));

    // The boundary **under** the bay still has its grab, which is what says
    // the answers above are about the controls rather than the rule having
    // gone missing.
    let region = rect_of(panel.layout(), "mixer");
    let below = Point::new(region.x + region.w * 0.5, region.y + region.h);
    assert_eq!(
        claim(&mut panel, &ctx, &strips, below),
        Claim::Panel,
        "the bottom edge of the mixer is not in the grab of the boundary under it, so \
         this test is no longer measuring what it was written for"
    );
}

// ---------------------------------------------------------------------------
// The values are the harness's
// ---------------------------------------------------------------------------

/// **Every value in the bay came in through the argument, and the console
/// keeps none of them.**
///
/// The bay is a function of what it was handed and of the arrangement, and of
/// nothing else — the seam `View::picture` is on, and the reason this crate
/// can be asked about a live console with no device anywhere near it.
#[test]
fn the_values_are_the_harnesss_and_are_stored_nowhere() {
    let (panel, ctx) = console(PLAUSIBLE);
    let one = mock_strips();
    let two = vec![mock()];

    let first = mixer(&ctx, panel.layout(), &one).expect("a bay");
    let other = mixer(&ctx, panel.layout(), &two).expect("a bay");
    assert_ne!(
        first, other,
        "two different decks drew the same bay, so something in it is not coming from the \
         argument"
    );
    assert_eq!(first.count(), 3);
    assert_eq!(other.count(), 1);
    // The bay carries what it was measured from, so whoever measured and
    // whoever paints are one statement.
    assert_eq!(first.strips, one.as_slice());

    // Asked again with the first, and it is the first answer: nothing was
    // written down in between.
    assert_eq!(
        mixer(&ctx, panel.layout(), &one).expect("a bay"),
        first,
        "the bay remembered the deck it was last asked about"
    );

    // And the view holds exactly what a caller put there — a plain field, not
    // a copy the console maintains.
    let mut view = View::new(Room::Day);
    assert!(view.mixer.is_empty());
    view.mixer = one.clone();
    assert_eq!(view.mixer, one);
    view.mixer.clear();
    assert!(view.mixer.is_empty());
}
