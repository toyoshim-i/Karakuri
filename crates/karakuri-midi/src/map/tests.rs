use super::*;
use crate::Message;

fn map(text: &str) -> Map {
    let (map, notes) = Map::parse(text);
    assert!(notes.is_empty(), "{notes:?}");
    map
}

fn cc(controller: u8, value: u8) -> Message {
    Message::ControlChange {
        channel: 0,
        controller,
        value,
    }
}

fn note(note: u8) -> Message {
    Message::NoteOn {
        channel: 0,
        note,
        velocity: 100,
    }
}

/// A 14-bit pair, as a helper: the two messages a surface sends for one
/// fader position, in the order it sends them.
fn pair(msb: u8, lsb: u8, value: u16) -> [Message; 2] {
    [cc(msb, (value >> 7) as u8), cc(lsb, (value & 0x7f) as u8)]
}

/// Verifies that `cc14 <msb> <lsb>` is the pair.
#[test]
fn a_cc14_line_is_one_mapping_and_spells_itself_back() {
    let m = map("cc14 1 33 -> gain 0");
    assert_eq!(m.len(), 1, "a pair was counted as two mappings");
    assert_eq!(
        m.bound("gain 0").as_deref(),
        Some("cc14 1 33"),
        "the readout did not say the line that was written"
    );
    assert_eq!(
        m.lines().collect::<Vec<_>>(),
        vec!["cc14 1 33 -> gain 0".to_string()],
        "the line written back was not the line loaded"
    );
    // And the channel survives, in the front panel's 1-16.
    let m = map("cc14 1 33 ch 2 -> gain 0");
    assert_eq!(m.bound("gain 0").as_deref(), Some("cc14 1 33 ch 2"));
}

/// Verifies that a malformed pair is refused at parse, naming both numbers.
#[test]
fn a_malformed_pair_is_refused_at_parse_naming_both_numbers() {
    for (line, wanted) in [
        // The two halves are one controller.
        ("cc14 7 7 -> gain 0", vec!["7 7"]),
        // The LSB half is past what the wire carries.
        ("cc14 7 200 -> gain 0", vec!["7 200"]),
        // No LSB half at all, and the complaint carries the line to write.
        ("cc14 7 -> gain 0", vec!["cc14 7", "cc14 7 39"]),
        ("cc14 7 ch 2 -> gain 0", vec!["cc14 7", "cc14 7 39"]),
        // Not a number.
        ("cc14 7 lsb -> gain 0", vec!["7 lsb"]),
    ] {
        let (m, notes) = Map::parse(line);
        assert!(m.is_empty(), "`{line}` loaded");
        let note = notes
            .first()
            .unwrap_or_else(|| panic!("`{line}` said nothing"));
        for wanted in wanted {
            assert!(note.contains(wanted), "`{line}` said `{note}`");
        }
    }
    // And a pair on a press target is refused as a `cc` is, because it is
    // one: a pad takes a note.
    let (_, notes) = Map::parse("cc14 1 33 -> residency 0 live");
    assert!(
        notes.first().is_some_and(|note| note.contains("note")),
        "{notes:?}"
    );
}

/// Verifies that the pair assembles into 16384 positions and scales onto the range.
#[test]
fn a_pair_assembles_into_sixteen_thousand_positions_and_scales_onto_the_range() {
    let m = map("cc14 1 33 -> gain 0");
    // Both ends are exact, which is the property every fader here has.
    for (value, wanted) in [(0u16, 0.0f32), (16383, 1.0), (8191, 8191.0 / 16383.0)] {
        let [msb, lsb] = pair(1, 33, value);
        assert_eq!(m.wide(msb).map(|w| w.half), Some(Half::Msb));
        assert_eq!(m.wide(lsb).map(|w| w.half), Some(Half::Lsb));
        assert_eq!(
            m.operation_wide(lsb, value),
            Some(Operation::SetGain {
                deck: 0,
                gain: wanted
            }),
            "the pair {value} did not assemble onto the range"
        );
    }
    // **Two adjacent pairs are a 128th of a 7-bit step apart**, which is
    // the number this line exists for: a 7-bit fader moves a gain by
    // 1/127 and this one by 1/16383.
    let step = |value: u16| match m.operation_wide(pair(1, 33, value)[1], value) {
        Some(Operation::SetGain { gain, .. }) => gain,
        other => panic!("{other:?}"),
    };
    let one = step(8192) - step(8191);
    assert!(
        (one - 1.0 / 16383.0).abs() < 1e-6,
        "one step of a pair was {one}"
    );
    // And a line's own range is scaled onto exactly as a 7-bit line's is.
    let m = map("cc14 1 33 -> gain 0 [0, 2]");
    assert_eq!(
        m.operation_wide(pair(1, 33, 16383)[1], 16383),
        Some(Operation::SetGain { deck: 0, gain: 2.0 })
    );
}

/// Verifies that an MSB alone never leaves the fader between two values.
#[test]
fn an_msb_alone_does_not_leave_the_fader_between_two_values() {
    let m = map("cc14 1 33 -> gain 0");
    let seven = map("cc 1 -> gain 0");
    for msb in [0u8, 1, 64, 126, 127] {
        assert_eq!(
            m.operation(cc(1, msb)),
            seven.operation(cc(1, msb)),
            "an MSB alone read differently from the 7-bit line it is"
        );
    }
    // The top of the fader is unity and not a step short of it, which is
    // what `(msb << 7) / 16383` would have made it.
    assert_eq!(
        m.operation(cc(1, 127)),
        Some(Operation::SetGain { deck: 0, gain: 1.0 })
    );
    // **And a lone LSB moves nothing**: there is nothing to refine until
    // an MSB has been seen, and a caller holding no MSB has no pair.
    assert_eq!(m.operation(cc(33, 100)), None);
    assert_eq!(m.parameter(cc(33, 100)), None);
}

/// Verifies that a controller cannot be a plain line and half of a pair.
#[test]
fn a_controller_that_is_both_a_line_and_a_pairs_lsb_is_reported() {
    for text in [
        "cc14 1 33 -> gain 0\ncc 33 -> opacity 0",
        "cc 33 -> opacity 0\ncc14 1 33 -> gain 0",
    ] {
        let (m, notes) = Map::parse(text);
        assert!(
            notes.iter().any(|note| note.contains("cc 33")),
            "{notes:?} for `{text}`"
        );
        // The plain line is what controller 33 does.
        assert_eq!(
            m.operation(cc(33, 127)),
            Some(Operation::SetOpacity {
                deck: 0,
                opacity: 1.0
            })
        );
        assert_eq!(m.wide(cc(33, 127)), None, "the pair claimed the plain line");
        // And the pair's MSB half still moves its own control, coarsely.
        assert_eq!(
            m.operation(cc(1, 127)),
            Some(Operation::SetGain { deck: 0, gain: 1.0 })
        );
    }
}

/// Verifies that an echo says what to read and what the wire shows.
#[test]
fn an_echo_says_what_to_read_and_the_wire_shows_it() {
    let m = map(
        "cc 1 -> gain 0\ncc14 2 34 -> opacity 1\nnote 32 -> residency 0 live\ncc 20 -> exposure\n\
             cc 30 -> param 0 3\nnote 61 -> tap",
    );
    let echoes = m.echoes();
    assert_eq!(echoes.len(), 6);
    let of = |control: Control| {
        *echoes
            .iter()
            .find(|echo| echo.control() == control)
            .unwrap_or_else(|| panic!("no echo for {control:?}"))
    };
    let mut wire = Vec::new();

    // A 7-bit fader: one message, and both ends exact.
    let gain = of(Control::Gain { deck: 0 });
    assert_eq!(gain.position(Shown::At(0.0)), 0);
    assert_eq!(gain.position(Shown::At(1.0)), 127);
    gain.wire(gain.position(Shown::At(1.0)), &mut wire);
    assert_eq!(wire, vec![[0xb0, 1, 127]]);

    // A pair: two messages, **MSB first**, and the two halves of the
    // number the position is.
    wire.clear();
    let opacity = of(Control::Opacity { deck: 1 });
    assert_eq!(opacity.position(Shown::At(1.0)), 16383);
    assert_eq!(opacity.position(Shown::At(0.0)), 0);
    let half = opacity.position(Shown::At(0.5));
    opacity.wire(half, &mut wire);
    assert_eq!(
        wire,
        vec![
            [0xb0, 2, (half >> 7) as u8],
            [0xb0, 34, (half & 0x7f) as u8]
        ]
    );
    // And it comes back through the way in, to the value it was shown.
    assert_eq!(
        m.operation_wide(cc(34, 0), half),
        Some(Operation::SetOpacity {
            deck: 1,
            opacity: half as f32 / 16383.0
        })
    );

    // A pad: lit is a note-on at 127 and unlit is one at 0, which is this
    // crate's own reading of a release.
    wire.clear();
    let live = of(Control::Residency {
        deck: 0,
        residency: Residency::Live,
    });
    live.wire(live.position(Shown::On(true)), &mut wire);
    live.wire(live.position(Shown::On(false)), &mut wire);
    assert_eq!(wire, vec![[0x90, 32, 127], [0x90, 32, 0]]);

    // Exposure is a ratio control, so the middle of the fader is unity.
    let exposure = of(Control::Exposure);
    assert_eq!(exposure.position(Shown::At(1.0)), 64);
    assert_eq!(exposure.position(Shown::At(0.25)), 0);
    assert_eq!(exposure.position(Shown::At(4.0)), 127);

    // A published control is shown over the range the Set published it,
    // unless the line wrote one.
    let param = of(Control::Param {
        deck: 0,
        position: 3,
    });
    assert_eq!(
        param.position(Shown::Published {
            value: 4.0,
            declared: [0.0, 8.0]
        }),
        64
    );

    // A value past the end of the line's range shows the end: that is
    // where the fader would have to be.
    assert_eq!(gain.position(Shown::At(1.4)), 127);
    assert_eq!(gain.position(Shown::At(-1.0)), 0);
    assert_eq!(gain.position(Shown::At(f32::NAN)), 0);

    // A beat has no state, and the list carries it so that a target added
    // to the grammar and not to `Control` does not compile.
    assert_eq!(of(Control::Tap).control(), Control::Tap);
}

/// Verifies that a `param` line names a deck and a place in its interface.
#[test]
fn a_param_line_names_a_deck_and_a_position_and_is_not_an_operation_here() {
    let m = map("cc 30 -> param 0 3");
    assert_eq!(
        m.operation(cc(30, 127)),
        None,
        "a position was completed without a deck to resolve it against"
    );
    let asked = m.parameter(cc(30, 127)).expect("a param line answers here");
    assert_eq!(asked.deck, 0);
    assert_eq!(asked.position, 3);
    assert_eq!(asked.range, None, "no range on the line is the Set's own");
    // **The declared range is the caller's**, and both ends are exact on
    // it for the reason every other fader's are.
    assert_eq!(asked.value([0.0, 8.0]), 8.0);
    assert_eq!(
        m.parameter(cc(30, 0)).expect("bottom").value([0.0, 8.0]),
        0.0
    );
    // A range on the line overrides the Set's, as it does on a gain.
    let m = map("cc 30 -> param 1 5 [1, 3]");
    let asked = m.parameter(cc(30, 127)).expect("a param line with a range");
    assert_eq!(asked.range, Some([1.0, 3.0]));
    assert_eq!(
        asked.value([0.0, 8.0]),
        3.0,
        "the line's range has to win over the Set's"
    );
    // It is continuous, so a note on it is refused at parse time.
    let (_, notes) = Map::parse("note 30 -> param 0 3");
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("map a `cc`"), "{notes:?}");
}

/// Verifies that positions count from one.
#[test]
fn a_position_of_zero_is_refused_with_where_the_number_comes_from() {
    let (map, notes) = Map::parse("cc 30 -> param 0 0");
    assert!(map.is_empty(), "a position of zero loaded");
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("count from one"), "{notes:?}");
    assert!(notes[0].contains("Inspector"), "{notes:?}");
    // And the position has to be a number at all.
    let (_, notes) = Map::parse("cc 30 -> param 0 first");
    assert!(notes[0].contains("expected a position"), "{notes:?}");
    // A deck with no position after it is a line half written.
    let (_, notes) = Map::parse("cc 30 -> param 0");
    assert!(notes[0].contains("needs a position"), "{notes:?}");
}

/// Verifies that every target spells itself back into a line this parser accepts.
#[test]
fn a_spelled_target_parses_back_to_itself() {
    let lines = [
        "cc 1 -> gain 0",
        "cc 5 -> opacity 3",
        "cc 20 -> exposure",
        "cc 9 -> mask-position 2",
        "cc 30 -> param 1 7",
        "note 32 -> residency 0 live",
        "note 36 -> residency 1 priming",
        "note 40 -> residency 2 allocated",
        "note 44 -> blend 0 add",
        "note 48 -> blend 1 over",
        "note 52 -> blend 2 max",
        "note 61 -> tap",
    ];
    for line in lines {
        let (from, to) = line.split_once(" -> ").expect("a test line");
        let (_, entry) = parse_line(line).unwrap_or_else(|e| panic!("`{line}`: {e}"));
        assert_eq!(
            entry.target.spelled(),
            to,
            "`{line}` did not spell itself back"
        );
        // And the whole line round-trips through the table, which is what
        // `Map::bound` is asked for.
        let m = map(line);
        assert_eq!(m.bound(to).as_deref(), Some(from), "`{line}`");
        assert_eq!(m.bound("gain 9"), None, "an unmapped control was claimed");
    }
    // A channel survives the round trip, in the front panel's 1-16.
    let m = map("cc 5 ch 2 -> gain 0");
    assert_eq!(m.bound("gain 0").as_deref(), Some("cc 5 ch 2"));
}

/// Verifies that every mapped message answers exactly one of the two accessors.
#[test]
fn every_mapped_message_answers_exactly_one_of_the_two() {
    let text = "cc 1 -> gain 0\ncc 5 -> opacity 0\ncc 20 -> exposure\n\
                    cc 9 -> mask-position 0\ncc 30 -> param 0 3\n\
                    note 32 -> residency 0 live\nnote 44 -> blend 0 add\nnote 61 -> tap";
    let m = map(text);
    let messages = [
        cc(1, 64),
        cc(5, 64),
        cc(20, 64),
        cc(9, 64),
        cc(30, 64),
        note(32),
        note(44),
        note(61),
    ];
    for message in messages {
        let operation = m.operation(message).is_some();
        let parameter = m.parameter(message).is_some();
        assert!(
            operation ^ parameter,
            "{message:?} answered {operation} and {parameter}, which is not exactly one"
        );
    }
    // And an unmapped message answers neither.
    assert!(m.operation(cc(99, 0)).is_none() && m.parameter(cc(99, 0)).is_none());
}

/// Verifies that both ends of a fader are exact.
#[test]
fn a_fader_reaches_both_ends_of_its_range_exactly() {
    let m = map("cc 1 -> gain 0");
    assert_eq!(
        m.operation(cc(1, 0)),
        Some(Operation::SetGain { deck: 0, gain: 0.0 })
    );
    assert_eq!(
        m.operation(cc(1, 127)),
        Some(Operation::SetGain { deck: 0, gain: 1.0 })
    );
    // And an explicit range, both ends, so the default is not the only one
    // that lands.
    let m = map("cc 1 -> gain 0 [0.5, 2.5]");
    assert_eq!(
        m.operation(cc(1, 0)),
        Some(Operation::SetGain { deck: 0, gain: 0.5 })
    );
    assert_eq!(
        m.operation(cc(1, 127)),
        Some(Operation::SetGain { deck: 0, gain: 2.5 })
    );
}

/// Verifies that exposure is a ratio, so the middle of the fader is unity.
#[test]
fn exposure_moves_in_stops_rather_than_in_equal_steps() {
    let m = map("cc 20 -> exposure");
    let at = |v: u8| match m.operation(cc(20, v)) {
        Some(Operation::SetExposure { exposure }) => exposure,
        other => panic!("{other:?}"),
    };
    assert_eq!(at(0), 0.25);
    assert!((at(127) - 4.0).abs() < 1e-5, "{}", at(127));
    // Halfway is unity, which is the whole reason for the shape.
    let middle = at(63) + (at(64) - at(63)) / 2.0;
    assert!((middle - 1.0).abs() < 0.01, "{middle}");
    // And it is a ratio scale rather than a line: the step at the bottom
    // is smaller than the step at the top.
    assert!(at(1) - at(0) < at(127) - at(126));
}

/// Verifies that a mask's front reaches both ends of its travel exactly, and moves in
/// equal steps.
#[test]
fn a_mask_front_reaches_both_ends_exactly_and_moves_in_equal_steps() {
    let m = map("cc 9 -> mask-position 0");
    let at = |v: u8| match m.operation(cc(9, v)) {
        Some(Operation::SetMaskPosition { deck: 0, position }) => position,
        other => panic!("{other:?}"),
    };
    assert_eq!(at(0), 0.0, "the front could not be sent back to hidden");
    assert_eq!(at(127), 1.0, "the front could not be carried all the way");
    // Linear scaling: verify middle travel and uniform step size across ends.
    let middle = at(63) + (at(64) - at(63)) / 2.0;
    assert!((middle - 0.5).abs() < 1e-6, "{middle}");
    let (bottom, top) = (at(1) - at(0), at(127) - at(126));
    assert!((bottom - top).abs() < 1e-6, "{bottom} against {top}");
    // Custom written ranges override the default [0.0, 1.0] range.
    let m = map("cc 9 -> mask-position 2 [0.25, 0.75]");
    assert_eq!(
        m.operation(cc(9, 0)),
        Some(Operation::SetMaskPosition {
            deck: 2,
            position: 0.25
        })
    );
    assert_eq!(
        m.operation(cc(9, 127)),
        Some(Operation::SetMaskPosition {
            deck: 2,
            position: 0.75
        })
    );
}

/// Verifies that the front takes a fader and refuses a pad.
#[test]
fn the_mask_front_takes_a_fader_and_refuses_a_pad() {
    let (m, notes) = Map::parse("note 62 -> mask-position 0");
    assert!(m.is_empty(), "a pad on the mask's front loaded");
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("map a `cc`"), "{}", notes[0]);
    // And the message it names does load, so the complaint is a route and
    // not a dead end.
    assert_eq!(map("cc 9 -> mask-position 0").len(), 1);
}

/// Verifies that the mask's other half has no line, and a file that tries to write one
/// is refused rather than half-loaded.
#[test]
fn the_mask_shape_has_no_line_and_every_spelling_of_one_is_refused() {
    for line in [
        "note 62 -> mask-shape 0 linear",
        "note 62 -> mask 0 radial",
        "cc 9 -> mask 0",
        "cc 9 -> mask-angle 0",
    ] {
        let (m, notes) = Map::parse(line);
        assert!(m.is_empty(), "`{line}` loaded");
        assert_eq!(notes.len(), 1, "`{line}`: {notes:?}");
        assert!(
            notes[0].contains("is not a control"),
            "`{line}`: {}",
            notes[0]
        );
        // The complaint lists what there is, and the front is on the list
        // — the one half of the mask a line can reach.
        assert!(notes[0].contains("mask-position"), "`{line}`: {}", notes[0]);
    }
}

/// Verifies that a pad and a fader cannot be mapped to each other's targets.
#[test]
fn a_control_that_takes_a_press_refuses_a_fader_and_the_reverse() {
    let (m, notes) = Map::parse("cc 1 -> residency 0 live\nnote 36 -> gain 0");
    assert!(m.is_empty(), "a mismatched mapping loaded");
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert!(notes[0].contains("map a `note`"), "{}", notes[0]);
    assert!(notes[1].contains("map a `cc`"), "{}", notes[1]);
}

/// A channel narrows a mapping and its absence widens it. Both directions,
/// because "any channel" as a default is only safe if a stated channel
/// still excludes the others.
#[test]
fn a_stated_channel_matches_only_that_channel_and_no_channel_matches_all() {
    let m = map("cc 1 ch 3 -> gain 0");
    let on = |channel| {
        m.operation(Message::ControlChange {
            channel,
            controller: 1,
            value: 127,
        })
    };
    assert!(on(2).is_some(), "channel 3 on the panel is 2 on the wire");
    assert_eq!(on(3), None, "a mapping on one channel answered another");

    let m = map("cc 1 -> gain 0");
    assert!(on_any(&m, 0).is_some() && on_any(&m, 9).is_some());
}

fn on_any(m: &Map, channel: u8) -> Option<Operation> {
    m.operation(Message::ControlChange {
        channel,
        controller: 1,
        value: 64,
    })
}

/// Verifies that a specific channel wins over `any`.
#[test]
fn a_mapping_with_a_channel_wins_over_one_without() {
    let m = map("cc 1 -> gain 0\ncc 1 ch 1 -> gain 3");
    assert_eq!(
        m.operation(Message::ControlChange {
            channel: 0,
            controller: 1,
            value: 127
        }),
        Some(Operation::SetGain { deck: 3, gain: 1.0 })
    );
    assert_eq!(
        m.operation(Message::ControlChange {
            channel: 5,
            controller: 1,
            value: 127
        }),
        Some(Operation::SetGain { deck: 0, gain: 1.0 })
    );
}

/// Verifies that a release does nothing.
#[test]
fn a_release_is_not_a_second_press() {
    let m = map("note 36 -> residency 0 live");
    assert_eq!(
        m.operation(note(36)),
        Some(Operation::SetResidency {
            deck: 0,
            residency: Residency::Live
        })
    );
    assert_eq!(
        m.operation(Message::NoteOff {
            channel: 0,
            note: 36
        }),
        None
    );
    // Including the spelling that arrives as a note-on at velocity 0.
    assert_eq!(
        m.operation(Message::parse(&[0x90, 36, 0]).expect("a note-on")),
        None
    );
}

/// Every control the vocabulary names parses and answers, so a control
/// added to `Target` and not to the parser is a line that does not load
/// rather than one that loads as something else.
#[test]
fn every_control_in_the_vocabulary_parses_and_answers() {
    let m = map("cc 1 -> gain 2\n\
             cc 2 -> opacity 3\n\
             cc 3 -> exposure\n\
             cc 4 -> mask-position 2\n\
             note 36 -> residency 1 live\n\
             note 37 -> blend 3 over\n\
             note 41 -> tap");
    assert_eq!(m.len(), 7);
    assert!(matches!(
        m.operation(cc(1, 127)),
        Some(Operation::SetGain { deck: 2, .. })
    ));
    assert!(matches!(
        m.operation(cc(2, 127)),
        Some(Operation::SetOpacity { deck: 3, .. })
    ));
    assert!(matches!(
        m.operation(cc(3, 64)),
        Some(Operation::SetExposure { .. })
    ));
    assert!(matches!(
        m.operation(cc(4, 127)),
        Some(Operation::SetMaskPosition {
            deck: 2,
            position: 1.0
        })
    ));
    assert_eq!(
        m.operation(note(36)),
        Some(Operation::SetResidency {
            deck: 1,
            residency: Residency::Live
        })
    );
    assert_eq!(
        m.operation(note(37)),
        Some(Operation::SetBlendMode {
            deck: 3,
            blend: BlendMode::Over
        })
    );
    assert_eq!(m.operation(note(41)), Some(Operation::TapBeat));
}

/// Verifies that a pad names a state, and every state the vocabulary holds is
/// writable.
#[test]
fn a_pad_names_one_of_the_values_the_vocabulary_holds() {
    for (i, residency) in Residency::ALL.iter().enumerate() {
        let line = format!("note {} -> residency 2 {}", 36 + i, residency.name());
        assert_eq!(
            map(&line).operation(note(36 + i as u8)),
            Some(Operation::SetResidency {
                deck: 2,
                residency: *residency
            }),
            "`{line}` did not name {}",
            residency.name()
        );
    }
    for (i, blend) in BlendMode::ALL.iter().enumerate() {
        let line = format!("note {} -> blend 1 {}", 48 + i, blend.name());
        assert_eq!(
            map(&line).operation(note(48 + i as u8)),
            Some(Operation::SetBlendMode {
                deck: 1,
                blend: *blend
            }),
            "`{line}` did not name {}",
            blend.name()
        );
    }
}

/// Verifies that a file written against the old grammar is refused, line by line, with
/// the line to write instead.
#[test]
fn a_line_in_the_old_grammar_is_refused_with_the_line_to_write_instead() {
    let (m, notes) = Map::parse(
        "note 32 -> on-air 0\n\
             note 36 -> prime 1\n\
             note 40 -> blend 2",
    );
    assert!(m.is_empty(), "a line in the old grammar loaded");
    assert_eq!(notes.len(), 3, "{notes:?}");
    for (i, wanted) in ["residency 0 live", "residency 1 priming", "blend 2 over"]
        .iter()
        .enumerate()
    {
        assert!(
            notes[i].starts_with(&format!("line {}:", i + 1)),
            "{}",
            notes[i]
        );
        assert!(
            notes[i].contains(wanted),
            "the complaint did not name `{wanted}`: {}",
            notes[i]
        );
    }
    // And the other half of each: taking a deck off air, and withdrawing a
    // prime request, are the destinations the old `false` had no name for.
    assert!(notes[0].contains("residency 0 allocated"), "{}", notes[0]);
    assert!(notes[1].contains("residency 1 allocated"), "{}", notes[1]);
}

/// A value word that is not one of the list is refused naming all of them,
/// which is the same complaint a missing one gets — one bad line, on its
/// own line, with somewhere to go.
#[test]
fn a_value_word_that_is_not_one_of_the_list_is_refused_naming_them() {
    let (m, notes) = Map::parse("note 36 -> residency 0 warming");
    assert!(m.is_empty());
    assert!(notes[0].contains("`residency 0 live`"), "{}", notes[0]);
    assert!(notes[0].contains("`residency 0 priming`"), "{}", notes[0]);
    assert!(notes[0].contains("`residency 0 allocated`"), "{}", notes[0]);
    let (m, notes) = Map::parse("note 36 -> blend 0 screen");
    assert!(m.is_empty());
    assert!(notes[0].contains("`blend 0 max`"), "{}", notes[0]);
}

/// Verifies that one bad line is a line, not a file.
#[test]
fn a_bad_line_is_reported_and_the_rest_of_the_file_loads() {
    let (m, notes) = Map::parse(
        "cc 1 -> gain 0\n\
             cc 2 -> wobble 1\n\
             cc 3 -> opacity 0",
    );
    assert_eq!(m.len(), 2, "a bad line took a good one with it");
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].starts_with("line 2:"), "{}", notes[0]);
    assert!(notes[0].contains("wobble"), "{}", notes[0]);
}

/// Comments and blank lines are not errors, and a comment after a mapping
/// does not become part of it.
#[test]
fn comments_and_blank_lines_are_skipped() {
    let m = map("# the slot faders\n\
             \n\
             cc 1 -> gain 0   # channel strip 1\n");
    assert_eq!(m.len(), 1);
    assert!(m.operation(cc(1, 127)).is_some());
}

/// A ratio control cannot have a range that reaches zero — the map is
/// undefined there — and saying so at parse time is the only place an
/// operator can act on it.
#[test]
fn a_ratio_range_that_reaches_zero_is_refused_where_it_is_written() {
    let (m, notes) = Map::parse("cc 3 -> exposure [0, 4]");
    assert!(m.is_empty());
    assert!(notes[0].contains("cannot reach zero"), "{}", notes[0]);
}

/// Verifies that the example map in the repository loads clean.
#[test]
fn the_example_map_in_the_repository_parses_with_no_complaints() {
    let text = include_str!("../../../../examples/surface.map");
    let (map, notes) = Map::parse(text);
    assert!(notes.is_empty(), "{notes:#?}");
    // Every line of it, so a mapping quietly dropped by a duplicate key
    // shows up as a count rather than as a missing knob mid-set.
    let lines = text
        .lines()
        .filter(|l| {
            let l = l.split('#').next().unwrap_or("").trim();
            !l.is_empty()
        })
        .count();
    assert_eq!(map.len(), lines, "a mapping was overwritten by another");
}

/// Verifies that a range with one value in it, or a value that is not one.
#[test]
fn a_range_that_is_not_two_different_numbers_is_refused() {
    for line in [
        "cc 1 -> gain 0 [1, 1]",
        "cc 1 -> gain 0 [nan, 1]",
        "cc 1 -> gain 0 [0, inf]",
    ] {
        let (m, notes) = Map::parse(line);
        assert!(m.is_empty(), "`{line}` loaded");
        assert!(
            notes[0].contains("two different finite ends"),
            "`{line}`: {}",
            notes[0]
        );
    }
    // And the shape it is guarding: two different finite ends load.
    assert_eq!(map("cc 1 -> gain 0 [0, 2]").len(), 1);
}

/// Verifies that the later line wins, and says so.
#[test]
fn a_second_mapping_for_one_message_replaces_the_first_and_is_reported() {
    let (m, notes) = Map::parse("cc 1 -> gain 0\ncc 1 -> gain 3");
    assert_eq!(m.len(), 1);
    assert_eq!(
        m.operation(cc(1, 127)),
        Some(Operation::SetGain { deck: 3, gain: 1.0 }),
        "the earlier line won"
    );
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].starts_with("line 2:"), "{}", notes[0]);
    // Two keys that only look alike are not a collision: a `cc 1` and a
    // `note 1` are two controls.
    let (_, notes) = Map::parse("cc 1 -> gain 0\nnote 1 -> tap");
    assert!(notes.is_empty(), "{notes:?}");
}

/// Verifies that every number a line carries is checked against the wire's range.
#[test]
fn a_number_the_wire_cannot_carry_is_refused_on_the_line() {
    for (line, wanted) in [
        ("cc 128 -> gain 0", "128"),
        ("note 200 -> tap", "200"),
        ("cc 1 ch 0 -> gain 0", "1-16"),
        ("cc 1 ch 17 -> gain 0", "1-16"),
        ("cc 1 zz 2 -> gain 0", "ch"),
        ("cc 1 ch 1 extra -> gain 0", "too many words"),
    ] {
        let (m, notes) = Map::parse(line);
        assert!(m.is_empty(), "`{line}` loaded");
        assert!(notes[0].contains(wanted), "`{line}`: {}", notes[0]);
    }
    // The boundaries either side, which is where an off-by-one lives.
    assert_eq!(map("cc 127 ch 1 -> gain 0").len(), 1);
    assert_eq!(map("cc 0 ch 16 -> gain 0").len(), 1);
}

/// A range on a press is a line whose author expected something else to
/// happen, so it is refused rather than ignored.
#[test]
fn a_range_on_a_press_is_refused_rather_than_dropped() {
    let (m, notes) = Map::parse("note 36 -> tap [0, 1]");
    assert!(m.is_empty());
    assert!(notes[0].contains("no range"), "{}", notes[0]);
}

/// Verifies that every fader's target is continuous and no pad's is.
#[test]
fn a_fader_moves_a_continuous_control_and_a_pad_does_not() {
    let m = map(
        "cc 1 -> gain 0\ncc 2 -> opacity 0\ncc 3 -> exposure\ncc 4 -> mask-position 0\n\
             note 36 -> residency 0 live\nnote 37 -> blend 0 over\n\
             note 38 -> tap",
    );
    for controller in 1..=4 {
        assert!(m.is_continuous(cc(controller, 0)), "cc {controller}");
    }
    for n in 36..=38 {
        assert!(!m.is_continuous(note(n)), "note {n}");
    }
    // Unmapped is not continuous: there is nothing to coalesce, and the
    // router reports it rather than routing it.
    assert!(!m.is_continuous(cc(9, 0)));
    // Nor is a release, which names no target at all.
    assert!(!m.is_continuous(Message::NoteOff {
        channel: 0,
        note: 36
    }));
}
