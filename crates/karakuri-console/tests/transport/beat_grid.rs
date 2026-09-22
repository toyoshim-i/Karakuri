use super::transport_common::*;

// ---------------------------------------------------------------------------
// The beat grid
// ---------------------------------------------------------------------------

/// Verifies that the beat grid calculates continuous positions across bar boundaries (ADR-0212).
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

/// Beat zero is a place on the grid and not the absence of one, and the
/// two ways an `f64` gets it wrong are here rather than left to be discovered
/// on a panel.
///
/// - A position a hair before the downbeat. `(-1e-18).rem_euclid(4.0)` is
///   `4.0` exactly — the true remainder is a hair under the divisor and rounds
///   up to it. As a dot *index* that was one past the end of a four-dot grid
///   and had to be clamped; as a position it is the downbeat, because
///   `beat_at` measures round the cycle and `4.0` is no distance at all from
///   the first dot. The clamp went with the index (ADR-0212), so what this
///   asserts now is that the light lands on dot 0 rather than that a number
///   was caught on the way out.
/// - A position before the session's own zero. `Oscillator::behind` reads
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

/// Verifies that beat indicator brightness smoothly transitions and wraps around bar boundaries (ADR-0212).
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

/// At the instant of a beat the grid is the mock's own picture, which is the
/// claim above followed all the way to the paint pass.
///
/// `Transport::position` is arithmetic and `TransportRow::at` is that
/// arithmetic carried, and neither of them is a colour: a grid that read the
/// right position and painted the wrong dot would pass every assertion above.
/// So this draws the frame and counts the dots by their fill — `.beat-grid i`
/// is `--c-line`, `.beat-grid i.on` is `--c-pink` — and walks the light from
/// beat to beat, checking each time that exactly one dot is fully lit, that it
/// is the one the light is on, and that the other three are exactly the unlit
/// colour.
///
/// That is the mock's markup, `<i class="on"></i><i></i><i></i><i></i>`, and it
/// is why the mock did not have to be contradicted to make the beat continuous:
/// it is a frame of the travel rather than a different drawing (ADR-0212). What
/// happens between two of these frames is
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

/// Verifies continuous interpolation of beat indicators and halos between beats (P-0094).
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

/// How many beats there are in a bar is the harness's answer and not a constant
/// here, so a grid of three is three dots and lights the third.
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

/// A reading nobody has is not drawn as a plausible one, which is the whole
/// value's rule read at the one place it is a word rather than a shape.
///
/// The mock's `58 fps · cpu 12.4/16.6 ms` has two numbers this console may not
/// have: the rate, which needs a stretch of untouched window to measure, and
/// the budget, which is the display's refresh interval and which `winit` will
/// not always name. Neither absence draws a `0`, a `—`, or the mock's own 16.6
/// — each drops its own words and leaves the rest of the line.
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
        all.iter().any(|line| line == "58 fps · cpu 12.4/16.6 ms"),
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
        no_rate.iter().any(|line| line == "cpu 12.4/16.6 ms"),
        "with no rate the frame readout reads {no_rate:?}"
    );
    assert!(
        !no_rate.iter().any(|line| line.contains("fps")),
        "there is no rate and the row is drawing one: {no_rate:?}"
    );

    // No budget: the `/16.6` goes, and the frame time keeps its unit.
    view.transport = Some(Transport {
        budget_ms: None,
        chain_ms: None,
        ..mock()
    });
    let no_budget = words(&mut view, &mut panel);
    assert!(
        no_budget.iter().any(|line| line == "58 fps · cpu 12.4 ms"),
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
        chain_ms: None,
        ..mock()
    });
    assert!(
        words(&mut view, &mut panel)
            .iter()
            .any(|line| line == "cpu 12.4 ms"),
        "with neither, the frame readout is the frame time and its unit"
    );
}
