use super::mixer_common::*;

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

/// The meter's column is the mean and its mark is the peak, and the mark never
/// leaves the well — which is the one thing `.vmeter`'s `overflow: hidden`
/// would hide rather than show, at exactly the reading that matters most.
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

#[test]
fn the_solo_and_mute_buttons_show_state_in_strip() {
    let pal = Room::Day.palette();
    for (soloed, muted) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut strips = mock_strips();
        strips[0].is_soloed = soloed;
        strips[0].is_muted = muted;
        let (at, shapes) = strip_shapes(strips, 0);

        assert_eq!(words(&shapes, at.solo), vec!["S".to_owned()]);
        assert_eq!(words(&shapes, at.mute), vec!["M".to_owned()]);

        let solo_color = shapes.iter().find_map(|shape| match shape {
            egui::Shape::Text(text) if at.solo.contains(text.pos) => Some(text.fallback_color),
            _ => None,
        });
        let expected_solo = if soloed { pal.sun } else { pal.dim };
        assert_eq!(solo_color, Some(expected_solo));

        let mute_color = shapes.iter().find_map(|shape| match shape {
            egui::Shape::Text(text) if at.mute.contains(text.pos) => Some(text.fallback_color),
            _ => None,
        });
        let expected_mute = if muted { pal.pink } else { pal.faint };
        assert_eq!(mute_color, Some(expected_mute));
    }
}

/// The blend mini says the mode it was given, in the vocabulary's own
/// lower-case word.
///
/// The list is now closed and that is the change: this used to be handed
/// arbitrary strings — `screen` and `multiply` among them — because the field
/// was the engine's `&'static str` and the console had nothing to do with the
/// list but draw it. The chip is a control now (ADR-0187), so it carries a
/// `BlendMode` and the three this walks are every mode there is. A fourth would
/// fail to compile in `BlendMode::name` before it reached here.
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

/// Verifies that the mask icon displays distinct filled glyphs for each mask type.
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

/// The number under the fader is the opacity, to the two places the mock writes
/// it.
#[test]
fn the_number_is_the_opacity() {
    for (opacity, text) in [(1.0, "1.00"), (0.3, "0.30"), (0.0, "0.00")] {
        let strips = vec![Strip { opacity, ..mock() }];
        let (at, shapes) = strip_shapes(strips, 0);
        assert_eq!(words(&shapes, at.num), vec![text.to_owned()]);
    }
}

/// A strip with no reading draws the meter's well and nothing in it, which is
/// the mock's own `alloc` strip — a `.vmeter` with no `b` and no `u`.
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
        "a strip with no reading drew a column or a peak mark, which is a reading nobody took"
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

/// The name is elided rather than wrapped, which is `.strip-name`'s own
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
// The whole strip is claimed, and which control it is the derivation's
// ---------------------------------------------------------------------------
