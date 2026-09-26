use super::*;
use karakuri_environment::mix::{op_wire_name, TONEMAPS};

// -- key-handling logic, separated from winit and the GPU ---------------

#[test]
fn slot_bounds_reject_the_slot_count_itself_and_beyond() {
    // The neighbour of the fixed off-by-one: digits are indices, so the
    // valid range is `0..slot_count`, and `slot_count` itself is already
    // out of range.
    assert!(slot_in_range(0, 4));
    assert!(slot_in_range(3, 4));
    assert!(!slot_in_range(4, 4));
    assert!(!slot_in_range(9, 4));
    // A deck of one: only slot 0 is valid, matching the default run.
    assert!(slot_in_range(0, 1));
    assert!(!slot_in_range(1, 1));
}

/// Verifies that parked slots display distinct residency tags from unrequested allocated slots.
#[test]
fn a_parked_slot_does_not_read_as_one_nobody_asked_about() {
    assert_ne!(
        residency_tag(Residency::Allocated, true),
        residency_tag(Residency::Allocated, false)
    );
    assert_ne!(
        residency_name(Residency::Allocated, true),
        residency_name(Residency::Allocated, false)
    );
    // Parked is off air, so it must not read as either of the two states
    // that are not: a park shown as `prim` claims warming that is not
    // happening, and shown as `LIVE` claims a slot on air.
    for parked in [true, false] {
        assert_ne!(
            residency_tag(Residency::Allocated, parked),
            residency_tag(Residency::Priming, parked)
        );
        assert_ne!(
            residency_tag(Residency::Allocated, parked),
            residency_tag(Residency::Live, parked)
        );
    }
    // `parked` is only ever true of an Allocated slot — `Deck::is_parked`
    // says so — but the tag is fixed-width regardless of what it is asked,
    // because the columns after it are positional.
    for residency in [Residency::Live, Residency::Priming, Residency::Allocated] {
        for parked in [true, false] {
            assert_eq!(residency_tag(residency, parked).len(), 4);
        }
    }
}

/// Verifies that only a stopped slot displays a stopped tag on the status line,
/// while non-stopped slots emit an empty string (ADR-0316).
#[test]
fn a_stopped_slot_is_the_only_one_the_status_line_says_anything_about() {
    assert_eq!(stopped_tag(false), "");
    assert!(
        stopped_tag(true).starts_with("overloaded"),
        "the status line does not use the word every other surface uses"
    );
    assert!(
        stopped_tag(true).ends_with(' '),
        "the word runs into the column after it"
    );
    // And it is not a residency, which is the one reading it must not take:
    // a stopped slot keeps whichever of the four it had.
    for residency in [Residency::Live, Residency::Priming, Residency::Allocated] {
        for parked in [true, false] {
            assert_ne!(
                residency_tag(residency, parked).trim(),
                stopped_tag(true).trim(),
                "the stopped word is spelled like a residency"
            );
        }
    }
}

#[test]
fn gain_floors_at_zero_but_has_no_ceiling() {
    assert_eq!(clamp_gain(-5.0), 0.0);
    assert_eq!(clamp_gain(0.0), 0.0);
    assert_eq!(clamp_gain(1.0), 1.0);
    assert_eq!(clamp_gain(1000.0), 1000.0, "HDR gain is not capped at 1.0");
}

#[test]
fn exposure_clamps_into_a_finite_positive_range() {
    assert_eq!(clamp_exposure(0.0), EXPOSURE_MIN);
    assert_eq!(clamp_exposure(-5.0), EXPOSURE_MIN);
    assert_eq!(clamp_exposure(1_000_000.0), EXPOSURE_MAX);
    assert_eq!(clamp_exposure(1.0), 1.0);
}

/// Verifies that tone mapping operator cycling traverses all operators and wraps back.
#[test]
fn tonemap_cycles_through_all_four_and_back_to_the_start() {
    let start = TonemapOp::Clamp;
    let mut op = start;
    let mut seen = vec![op];
    for _ in 0..TONEMAPS.len() - 1 {
        op = next_tonemap(op);
        seen.push(op);
    }
    assert_eq!(
        seen,
        vec![
            TonemapOp::Clamp,
            TonemapOp::Reinhard,
            TonemapOp::Aces,
            TonemapOp::AgX,
        ]
    );
    assert_eq!(next_tonemap(op), start, "the cycle must close");
    assert_eq!(
        seen.len(),
        TONEMAPS.len(),
        "the cycle and `TONEMAPS` disagree about how many operators there are"
    );
    for op in TONEMAPS {
        assert!(seen.contains(&op), "{} is not in the cycle", op_name(op));
    }
}

/// Both spellings of every operator are distinct from every other's, so a
/// `look` record cannot name two operators and `--tonemap` cannot resolve to
/// the wrong one. Two arms of `spellings` sharing a wire name would compile and
/// would make `parse_op` return whichever came first.
#[test]
fn no_two_tonemap_operators_share_a_spelling() {
    for op in TONEMAPS {
        assert_eq!(
            parse_op(op_wire_name(op)),
            Some(op),
            "`{}` does not parse back to itself",
            op_wire_name(op)
        );
    }
    let mut wire: Vec<&str> = TONEMAPS.iter().map(|op| op_wire_name(*op)).collect();
    wire.sort_unstable();
    let before = wire.len();
    wire.dedup();
    assert_eq!(before, wire.len(), "two operators share a wire spelling");
}

/// Verifies that save records occurring after the final tick are noted individually rather than just counted.
#[test]
fn a_trailing_save_is_named_and_not_only_counted() {
    let notes = trailing_notes(&[
        Record::Gain {
            slot: DeckSlot(1),
            value: 0.5,
        },
        Record::Save {
            slot: DeckSlot(2),
            id: "20260816-143052-271".to_string(),
        },
    ]);
    assert_eq!(notes.len(), 2, "the skipped save was not named: {notes:?}");
    assert!(
        notes[0].starts_with("2 records after the last tick"),
        "{notes:?}"
    );
    assert!(
        notes[1].contains("slot 2") && notes[1].contains("20260816-143052-271"),
        "a skipped save has to name the slot and the set an operator would \
         go and load: {notes:?}"
    );

    // **The control.** Nothing to name and the count stands alone.
    let counted = trailing_notes(&[Record::Gain {
        slot: DeckSlot(1),
        value: 0.5,
    }]);
    assert_eq!(
        counted.len(),
        1,
        "a trailing record that reaches nothing outside the stream is counted \
         and no more: {counted:?}"
    );
    assert!(counted[0].starts_with("1 record after"), "{counted:?}");
}

#[cfg(test)]
mod value_tests {
    use super::*;

    /// The error, or `None` if the arguments were accepted. `ParseOutcome` holds
    /// GPU-adjacent config and is not `Debug`, and giving it one just so a test can
    /// print it would be the tail wagging the dog.
    fn parse(args: &[&str]) -> Option<String> {
        parse_args_from(args.iter().map(|s| s.to_string())).err()
    }

    /// Every flag that takes a value refuses a bad one rather than keeping its
    /// default. The silence is the bug: a run that quietly used 240 frames is
    /// indistinguishable from one that honoured the `--frames` that was typed.
    #[test]
    fn a_flag_given_a_value_it_cannot_use_says_so() {
        for (args, expect) in [
            (vec!["--frames", "24O"], "--frames"),
            (vec!["--capacity", "lots"], "--capacity"),
            (vec!["--budget-ms", "soon"], "--budget-ms"),
            (vec!["--size", "1280"], "--size"),
            (vec!["--size", "1280x"], "--size"),
            (vec!["--canvas", "1920"], "--canvas"),
            (vec!["--canvas", "x1080"], "--canvas"),
            (vec!["--param", "turbulence"], "--param"),
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains(expect), "{args:?} -> {err}");
        }
    }

    /// Zero is a typo, not a size. Every texture descriptor downstream takes
    /// `max(1)` to stay legal, so `--canvas 1920x0` would have rendered a frame one
    /// texel tall and reported the size it was asked for — the same silence as a
    /// `--frames 24O` that renders 240.
    #[test]
    fn an_extent_with_a_zero_side_is_refused_rather_than_clamped() {
        for args in [
            vec!["--canvas", "1920x0"],
            vec!["--canvas", "0x1080"],
            vec!["--canvas", "0x0"],
            vec!["--size", "0x720"],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("zero"), "{args:?} -> {err}");
        }
    }

    /// Verifies that `--size` is refused when running in headless or offscreen modes without a window.
    #[test]
    fn the_preview_windows_size_is_refused_where_there_is_no_window() {
        for args in [
            vec!["--render", "out.png", "--size", "1920x1080"],
            vec!["--seq", "frames", "--size", "1920x1080"],
            vec![
                "--replay",
                "s",
                "--render",
                "out.png",
                "--size",
                "1920x1080",
            ],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("no window"), "{args:?} -> {err}");
        }
        // And it is accepted wherever a window exists, including beside
        // `--canvas`: the two describe different things and were split so they
        // could be given together.
        assert!(parse(&["--size", "800x600"]).is_none());
        assert!(parse(&["--size", "800x600", "--canvas", "1920x1080"]).is_none());
        assert!(parse(&["--render", "out.png", "--canvas", "1920x1080"]).is_none());
    }

    /// Verifies that `--mcp` is refused when no interactive or headless run is being driven.
    #[test]
    fn an_mcp_port_is_refused_where_there_is_no_run_to_drive() {
        for args in [
            vec!["--mcp", "8000", "--render", "out.png"],
            vec!["--mcp", "8000", "--seq", "frames/"],
            vec!["--mcp", "8000", "--replay", "a", "--render", "out.png"],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("--mcp"), "{args:?} -> {err}");
        }
        assert!(parse(&["--mcp", "8000"]).is_none());
        assert!(parse(&["--mcp", "8000", "--watch"]).is_none());
        // A port is a number, and a flag given a value it cannot use says so
        // rather than keeping a default.
        assert!(parse(&["--mcp", "eight-thousand"]).is_some());
    }

    /// Verifies that `--tempo-source` is refused when running offline without a live performance.
    #[test]
    fn a_tempo_source_is_refused_where_there_is_no_performance_to_follow() {
        for args in [
            vec!["--tempo-source", "helper", "--render", "out.png"],
            vec!["--tempo-source", "helper", "--seq", "frames/"],
            vec![
                "--tempo-source",
                "helper",
                "--replay",
                "a",
                "--render",
                "out.png",
            ],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("--tempo-source"), "{args:?} -> {err}");
        }
        assert!(parse(&["--tempo-source", "helper"]).is_none());
        assert!(parse(&["--tempo-source", "helper", "--audio-in", "default"]).is_none());
    }

    /// Verifies that `--record-session` is refused for non-interactive rendering.
    #[test]
    fn recording_is_refused_where_there_is_no_performance() {
        for args in [
            vec!["--render", "out.png", "--record-session", "s"],
            vec!["--seq", "frames", "--record-session", "s"],
            vec![
                "--replay",
                "a",
                "--render",
                "out.png",
                "--record-session",
                "s",
            ],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("--record-session"), "{args:?} -> {err}");
        }
        // A live run is where it belongs, and it does **not** need `--load-set`:
        // the material is saved under `ID-material` and put at the head. The
        // help text said otherwise long after that stopped being true.
        assert!(parse(&["--record-session", "s"]).is_none());
        assert!(parse(&["--record-session", "s", "--load-set", "base"]).is_none());
    }

    /// Verifies that `--canvas` is refused during replay only when the stream records canvas dimensions.
    #[test]
    fn a_replay_refuses_the_canvas_flag_only_when_the_stream_has_one() {
        let performed = Some((1280, 720));
        let flag = (640, 480);

        assert_eq!(
            replay_canvas(performed, flag, false),
            Ok(((1280, 720), None)),
            "the stream's size, and nothing to say about it"
        );

        let refusal = replay_canvas(performed, flag, true).expect_err("was accepted");
        assert!(refusal.contains("--canvas"), "{refusal}");

        // No record: the flag is the way out, and either way it is named,
        // because a replay at a size nobody can vouch for must not look like a
        // faithful one.
        let (size, note) = replay_canvas(None, flag, true).expect("the flag is allowed");
        assert_eq!(size, flag);
        assert!(note.expect("says so").contains("640x480"));

        let (size, note) = replay_canvas(None, flag, false).expect("falls back");
        assert_eq!(size, flag);
        assert!(note.expect("says so").contains("guess"));

        // And the flag reaches parsing at all, which the old refusal blocked.
        assert!(parse(&["--replay", "s", "--render", "out.png", "--canvas", "640x480"]).is_none());
    }

    /// A flag at the end of the line has no value, and the message says which flag
    /// rather than blaming whatever came before it.
    #[test]
    fn a_flag_with_nothing_after_it_says_which_flag() {
        for flag in [
            "--render",
            "--seq",
            "--frames",
            "--size",
            "--set",
            "--exposure",
        ] {
            let err = parse(&[flag]).unwrap_or_else(|| panic!("`{flag}` alone was accepted"));
            assert!(
                err.contains(flag) && err.contains("needs a value"),
                "{flag} -> {err}"
            );
        }
    }

    /// `--render` with the path forgotten used to open a window: `None` fell
    /// through to the interactive branch, so a batch script with a typo hung
    /// instead of failing.
    #[test]
    fn a_flag_does_not_swallow_the_next_flag() {
        let err = parse(&["--render", "--frames", "10"]).expect("`--render --frames` was accepted");
        assert!(err.contains("--render") && err.contains("not one"), "{err}");
    }

    /// A negative number is a value, not an option — the check is "looks like a
    /// flag", and `-1.5` does not.
    #[test]
    fn a_negative_number_still_reads_as_a_value() {
        let err = parse(&["--exposure", "-1.5"]).expect("a negative exposure was accepted");
        assert!(
            err.contains("positive"),
            "rejected for the wrong reason: {err}"
        );
    }
}
