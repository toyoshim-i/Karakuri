use super::*;
use karakuri_console::focus::Step;
use karakuri_console::view::tracker_group;
use karakuri_environment::{audio, mix, Asked};
use karakuri_operation::{BeatSource, GridScale, Operation};
use karakuri_operation_record::{Current, Written};
/// Verifies surface status line formatting: reports active port, map name/path, or unconfigured status without requiring physical hardware (ADR-0220).
#[test]
fn the_legend_names_the_port_and_the_map_and_nothing_plugged_in_is_a_state() {
    let line = surface_line(
        "nanoKONTROL2 SLIDER/KNOB",
        Some("default"),
        Some(std::path::Path::new("/sets/.karakuri/maps/default.map")),
        40,
        &[],
    );
    assert!(
        line.contains("nanoKONTROL2 SLIDER/KNOB"),
        "the legend did not say which surface answered: {line}"
    );
    assert!(
        line.contains("`default`") && line.contains("/sets/.karakuri/maps/default.map"),
        "the legend named neither the map nor which of the two tiers it came from: {line}"
    );
    assert!(line.contains("40 mappings"), "{line}");
    let one = surface_line("x", Some("m"), Some(std::path::Path::new("m.map")), 1, &[]);
    assert!(
        one.contains(" 1 mapping.") && !one.contains("mappings"),
        "one mapping was pluralised: {one}"
    );

    // Map parsing warnings are reported in the surface status line.
    let noted = surface_line(
        "x",
        Some("m"),
        Some(std::path::Path::new("m.map")),
        2,
        &["line 4: `on-air 0` is a step; write `residency 0 live`".to_owned()],
    );
    assert!(noted.contains("line 4"), "{noted}");

    // Surfaces without a map report instructions for learning controls.
    let bare = surface_line("x", None, None, 0, &[]);
    assert!(
        bare.contains("no map") && bare.contains("turn a knob"),
        "a surface with no map was not told how to get one: {bare}"
    );

    // Having no MIDI inputs connected is treated as normal operation.
    let none = unsurfaced("no MIDI input matching `` — there are no MIDI inputs");
    assert!(
        none.contains("not a fault") && none.contains("pointer"),
        "a machine with nothing plugged in was told it had a problem: {none}"
    );
    // A port that is there and would not open is the other kind of
    // nothing, and it is said in the port's own words.
    let taken = unsurfaced("could not open `nanoKONTROL2`: device is in use");
    assert!(
        taken.contains("device is in use") && !taken.contains("not a fault"),
        "a port held by something else was reported as a state: {taken}"
    );
}

/// Verifies audio input status reporting: missing devices report cleanly without fault, and unconfigured inputs list available choices (P-0084, P-0094).
#[test]
fn a_room_with_no_microphone_is_a_state_and_a_named_one_that_is_gone_is_a_refusal() {
    let quiet = unopened(
        "default",
        &audio::AudioError::NoMatch {
            wanted: "default".to_owned(),
            available: Vec::new(),
        },
    );
    assert!(
        quiet.contains("no audio inputs") && quiet.contains("not a fault"),
        "a machine with no inputs was told it had a problem: {quiet}"
    );
    assert!(
        !quiet.contains("VB-Cable"),
        "the empty case named a device: {quiet}"
    );

    let missing = unopened(
        "scarlett",
        &audio::AudioError::NoMatch {
            wanted: "scarlett".to_owned(),
            available: vec!["VB-Cable".to_owned(), "Built-in".to_owned()],
        },
    );
    assert!(
        missing.contains("scarlett")
            && missing.contains("VB-Cable")
            && missing.contains("Built-in"),
        "the refusal did not carry what the operator needs: {missing}"
    );
    assert!(
        !missing.contains("not a fault"),
        "a device somebody named and is not there was reported as a state: {missing}"
    );

    // Picking an unavailable audio input is refused without altering active stream.
    let mut open: Option<audio::Audio> = None;
    let mut told_pill = Some(AudioIn::NONE);
    let line = attached(
        &mut open,
        120.0,
        &mut told_pill,
        &Operation::AttachBeatSource {
            source: BeatSource::AudioInput("\u{fffd}no such audio input\u{fffd}".to_owned()),
        },
    )
    .expect("`attached` answered nothing for an attach");
    assert!(
        line.contains("nothing is open"),
        "a refused pick said nothing about what is open now: {line}"
    );
    assert!(open.is_none(), "a refused pick opened something");

    // And a process is declined in one sentence rather than ignored.
    let process = attached(
        &mut open,
        120.0,
        &mut told_pill,
        &Operation::AttachBeatSource {
            source: BeatSource::Process("beats --stdout".to_owned()),
        },
    )
    .expect("`attached` ignored a beat source it cannot take");
    assert!(
        process.contains("tempo-source"),
        "a process was declined without saying where that half of the row lives: {process}"
    );

    // Every other operation is somebody else's, which is what keeps this
    // one line in `performed` rather than a second route into the device.
    assert_eq!(
        attached(&mut open, 120.0, &mut told_pill, &Operation::TapBeat),
        None
    );
}

/// Verifies Transport latency offset steps by 5ms in expected directions and resets to zero with space (ADR-0259).
#[test]
fn the_offset_steps_down_and_up_by_the_one_step_both_keyboards_use() {
    assert_eq!(
        offset_key(Step::Down, 0.0),
        -audio::LATENCY_OFFSET_STEP_MS,
        "down is the direction that makes the picture wait for the music, so it steps the \
         offset down"
    );
    assert_eq!(
        offset_key(Step::Up, 0.0),
        audio::LATENCY_OFFSET_STEP_MS,
        "up is the direction that makes the picture lead, so it steps the offset up"
    );
    assert_eq!(
        audio::LATENCY_OFFSET_STEP_MS,
        5.0,
        "the page says five milliseconds a press and the constant says otherwise — the page \
         is the specification, so one of the two is wrong and it is not this test"
    );
    assert_eq!(
        offset_key(Step::Default, 37.0),
        0.0,
        "space on a level is the value it was declared at, and an offset is declared at none"
    );
    // Offset step key presses adjust relative to the current offset value.
    assert_eq!(offset_key(Step::Up, 20.0), 25.0);
    assert_eq!(offset_key(Step::Down, 20.0), 15.0);
}

/// Verifies tempo adjustment steps by 1 BPM and clamps at lower oscillator bounds (ADR-0350).
#[test]
fn the_tempo_steps_by_one_beat_a_minute_from_the_grid_it_is_standing_on() {
    assert_eq!(tempo_key(Step::Up, 128.0), 129.0);
    assert_eq!(tempo_key(Step::Down, 128.0), 127.0);
    assert_eq!(
        TEMPO_STEP_BPM, 1.0,
        "the page says one beat a minute a press and the constant says otherwise — the page is \
         the specification"
    );
    assert_eq!(
        tempo_key(Step::Default, 92.5),
        92.5,
        "the tempo figure has no value it was declared at, so a default is the grid unchanged"
    );
    // Floored where `karakuri_signal` floors it: a grid at zero has no beat to
    // run, and this decides what the record says.
    assert_eq!(tempo_key(Step::Down, 1.0), 1.0);
    // And one step is inside the guard a press on the figure is held to, at
    // the slowest tempo the tracker searches.
    const { assert!(TEMPO_STEP_BPM <= 60.0 * karakuri_console::view::TEMPO_BAND) };
}

/// The master out steps by the trim's tenth and is held inside `[0, 1]`, which
/// is the range `Knob::Out` drags over — so a key and a hand can reach the same
/// values and no others.
#[test]
fn the_master_out_steps_by_a_tenth_and_is_held_inside_the_track() {
    assert!((out_key(Step::Up, 0.5) - 0.6).abs() < 1e-5);
    assert!((out_key(Step::Down, 0.5) - 0.4).abs() < 1e-5);
    assert_eq!(
        out_key(Step::Up, 1.0),
        1.0,
        "a press at the top of the track put the whole programme past unity"
    );
    assert_eq!(out_key(Step::Down, 0.0), 0.0);
    assert_eq!(
        out_key(Step::Default, 0.3),
        1.0,
        "space on the master out is not unity, which is where a run starts"
    );
}

/// Verifies exposure adjustments step in quarter-stop increments, four steps equaling one stop.
#[test]
fn the_exposure_steps_a_quarter_stop_and_four_presses_double_it() {
    let mut at = 1.0;
    for _ in 0..4 {
        at = exposure_key(Step::Up, at);
    }
    assert!(
        (at - 2.0).abs() < 1e-4,
        "four presses up from 1.0 landed at {at} rather than at a stop"
    );
    let mut back = at;
    for _ in 0..4 {
        back = exposure_key(Step::Down, back);
    }
    assert!(
        (back - 1.0).abs() < 1e-4,
        "four presses back landed at {back}"
    );
    assert_eq!(
        exposure_key(Step::Default, 8.0),
        1.0,
        "space on the exposure is not 1.0, which is the middle of the track"
    );
}

/// Verifies latency offset bounds are clamped within ±200ms with explicit limit reporting (P-0094).
#[test]
fn the_offset_says_which_way_it_points_and_says_when_it_was_held_at_the_bound() {
    let waiting = offset_said(-15.0, -15.0);
    assert!(
        waiting.contains("the picture waits for the music"),
        "a negative offset did not say which of the two is late: {waiting}"
    );
    let leading = offset_said(20.0, 20.0);
    assert!(
        leading.contains("the picture leads the music"),
        "a positive offset did not say which of the two is late: {leading}"
    );
    assert!(
        !leading.contains("as far either way as it goes"),
        "an offset nothing held claimed it was at the end of its travel: {leading}"
    );

    // The far end, asked for by one step and refused by five: the numbers
    // are the range's own, so a range that moved moves this with it.
    let top = *audio::LATENCY_OFFSET_RANGE.end();
    let held = offset_said(top + audio::LATENCY_OFFSET_STEP_MS, top);
    assert!(
        held.contains("as far either way as it goes"),
        "a press that asked past the bound and got the bound said nothing about it: {held}"
    );
    assert!(
        held.contains(&format!("{top:+.0} ms")),
        "the sentence about a clamped press does not carry the value it was held at: {held}"
    );
}

/// Verifies latency offset operations do not modify state when no audio input is attached.
#[test]
fn the_offset_keys_say_so_and_change_nothing_with_no_input_attached() {
    let mut open: Option<audio::Audio> = None;
    let line = nudged(&mut open, &Operation::SetLatencyOffset { ms: 25.0 })
        .expect("`nudged` answered nothing for an offset");
    assert!(
        line.contains("no audio input") && line.contains("audio-in"),
        "a press with nothing open did not say why or where the input is picked: {line}"
    );
    assert!(open.is_none(), "a press with nothing open opened something");
    // The same sentence the key arm prints before it builds an operation
    // at all, so the two paths into this state cannot drift apart.
    assert!(
        line.contains(NO_ROOM_FOR_AN_OFFSET),
        "the two ways into a panel with no room say two different things: {line}"
    );

    // Every other operation is somebody else's, on `attached`'s terms.
    assert_eq!(nudged(&mut open, &Operation::TapBeat), None);
}

/// Verifies console latency offset constants match environment ranges and step intervals (ADR-0156).
#[test]
fn the_consoles_offset_track_spans_the_sessions_own_range() {
    assert_eq!(
        view::LATENCY_OFFSET_MIN_MS,
        *audio::LATENCY_OFFSET_RANGE.start(),
        "the console lays a track out to a floor the session does not clamp to, so a press \
         at the left-hand end asks for a value the offset cannot be moved to"
    );
    assert_eq!(
        view::LATENCY_OFFSET_MAX_MS,
        *audio::LATENCY_OFFSET_RANGE.end(),
        "the console lays a track out to a ceiling the session does not clamp to"
    );
    assert_eq!(
        view::LATENCY_OFFSET_STEP_MS,
        audio::LATENCY_OFFSET_STEP_MS,
        "the track is drawn one pixel per press against a press this program does not make, \
         so pointing at it and stepping it reach two different sets of values"
    );
    assert_eq!(
        view::OFFSET_TRACK_W,
        (audio::LATENCY_OFFSET_RANGE.end() - audio::LATENCY_OFFSET_RANGE.start())
            / audio::LATENCY_OFFSET_STEP_MS,
        "the track is not as many pixels as there are presses between its ends"
    );
}

/// Verifies octave tracker switches activate only within supported BPM bounds (60-200 BPM).
#[test]
fn the_octave_halves_the_panel_draws_live_are_the_ones_the_lock_would_take() {
    let range = audio::BPM_RANGE;
    assert!(
        range.end() / range.start() < 4.0,
        "the trackable range is two octaves or more, so both halves of the octave can be \
         live at once and the control the mock draws is the wrong shape"
    );

    // The mock's tempo: `½` to 64, which is inside; `×2` to 256, which is
    // not.
    let mock = tracking(None, 128.0);
    assert!(
        mock.halve,
        "the panel inerts `½` at the tempo the mock draws it live at"
    );
    assert!(
        !mock.double,
        "the panel offers `×2` at 128.0, where the lock would refuse 256 — a control that \
         undoes itself two seconds later is worse than one that says no"
    );

    // Both ends of the range, where exactly one of the two is reachable.
    let low = tracking(None, *range.start());
    assert!(
        !low.halve && low.double,
        "at the floor only doubling is available"
    );
    let high = tracking(None, *range.end());
    assert!(
        high.halve && !high.double,
        "at the ceiling only halving is available"
    );

    // And the offset is the session's or it is nothing: with none open
    // there is no value to draw a track at, which is what stops the panel
    // showing `DEFAULT_LATENCY_OFFSET_MS` as though something held it.
    assert_eq!(
        tracking(None, 128.0).offset_ms,
        None,
        "the panel was handed an offset on a console with no session, so it would draw a \
         track pointing at a number nobody chose"
    );
}
