use super::*;
use karakuri_console::focus::Step;
use karakuri_console::view::tracker_group;
use karakuri_environment::{audio, mix, Asked};
use karakuri_operation::{BeatSource, GridScale, Operation};
use karakuri_operation_record::{Current, Written};
/// What the legend says about a control surface, and none of it needs one
/// plugged in.
///
/// The same house rule as the room's test below: the half that has no device in
/// it is [`surface_line`] and [`unsurfaced`], and both are pure functions of
/// what was found. What is deliberately not here is that a real port opens —
/// nothing in this file can stand in for a controller on a desk, and
/// `karakuri_environment::midi`'s tests are what hold the route from a message
/// to a record.
///
/// 1. The port is named, because *which surface answered* is the one thing an
///    operator cannot see from the panel: this program takes the first input there
///    is (ADR-0220's reason one column along — the instrument has no `--midi-in`),
///    and a run that took the wrong one of two would look exactly like a run whose
///    controller is asleep.
/// 2. The map is named, and by its path as well as its name. `default` under the
///    store and `surface` in the preset library are two files, and an operator
///    who has just learned one wants to know which is loaded.
/// 3. No map is a state, with the sentence that tells them what to do next,
///    and nothing plugged in is not a fault.
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

    // **A map's complaints are carried rather than swallowed**, which is
    // `Map::parse`'s own requirement of its callers: a caller that dropped
    // them would leave an operator pressing a pad that never loaded.
    let noted = surface_line(
        "x",
        Some("m"),
        Some(std::path::Path::new("m.map")),
        2,
        &["line 4: `on-air 0` is a step; write `residency 0 live`".to_owned()],
    );
    assert!(noted.contains("line 4"), "{noted}");

    // **No map is a state and the sentence is what to do about it.**
    let bare = surface_line("x", None, None, 0, &[]);
    assert!(
        bare.contains("no map") && bare.contains("turn a knob"),
        "a surface with no map was not told how to get one: {bare}"
    );

    // **Nothing plugged in is the ordinary state of this program**, and it
    // is what every machine a test runs on is in.
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

/// The three things this program has decided about a room, and none of them
/// needs a device.
///
/// The house rule for this pass was to say how what could not be opened was
/// tested. This is it: `listening`'s judgement is [`unopened`], which is a pure
/// function of an error, and the case that can only happen during a set — an
/// input picked off a list and gone by the time it is opened — is reachable on
/// any machine at all by picking a name no device can have. What is
/// deliberately not here is that a real input opens; that is `karakuri-audio`'s
/// ignored `the_default_input_opens_and_delivers` and no assertion in this file
/// could stand in for it.
///
/// 1. No device at all is not a fault (P-0084): the sentence says `none` is a
///    state, and says what goes on answering.
/// 2. A device that was named and is not there is loud (P-0094): the sentence
///    carries the list, so an operator who picked a cable that has gone is holding
///    the right names rather than an invitation to go and look.
/// 3. And a refused pick does not take the room away. Nothing is open in this
///    test, so what is asserted is the half that can be: the answer says so rather
///    than going quiet.
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

    // **The case that can only arrive during a set**, on a machine with
    // whatever it happens to have plugged in: a pick nothing can match.
    // Nothing is opened — `pick` refuses before a stream is built — so
    // this runs anywhere and touches no hardware.
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

/// "Five milliseconds a press, down and up" — `docs/manual/operations.html`'s
/// row, and the sign `docs/manual/console.html` says is the half that gets read
/// wrong at two in the morning: *"Negative and the picture waits for the music,
/// positive and it leads."*
///
/// A pair wired the wrong way round reads correct and points backwards. The
/// letters went on 2026-09-10 and the arithmetic did not: `↑↓` on the
/// Transport's offset is what `o` and `p` were, and this asks [`offset_key`]
/// which way each direction goes and that both go by the one constant the
/// command line's own `o` and `p` use.
///
/// And that `space` on it is the value it was declared at, which is ADR-0259's
/// rule for every level and the first way back to no offset at all this control
/// has had.
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
    // **The press is counted from where the session is**, never from zero:
    // a step that ignored what it was standing on would jump the offset to
    // one step whatever a hand had dialled.
    assert_eq!(offset_key(Step::Up, 20.0), 25.0);
    assert_eq!(offset_key(Step::Down, 20.0), 15.0);
}

/// The tempo steps by one beat a minute, counted from the grid it is standing
/// on, and is floored where the oscillator's own clamp is (ADR-0350).
///
/// `space` is not a destination on this control: the figure has no value it was
/// declared at, so `karakuri_console::focus` declines there.
///
/// One step is inside the ±15% a press on the figure is trusted with at every
/// tempo the trackable range holds.
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

/// The exposure steps a quarter stop, taken on the console's own track so that
/// a key and a pointer land on the same forty-eight values —
/// `view::EXPOSURE_TRACK_W`'s whole argument, met from the keyboard's end.
///
/// Four presses are one stop, which is what a quarter stop means and the one
/// claim here worth stating as arithmetic rather than as a constant: a
/// doubling.
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

/// "Negative and the picture waits for the music, positive and it leads", and
/// "held inside 200 milliseconds either way" — the two halves of
/// `docs/manual/console.html`'s offset contract that a panel can be held to
/// without a device.
///
/// The second is the one a control is silent about by default: the value is
/// clamped in `karakuri_environment::audio` and a press at the end of the
/// travel would otherwise print the same number as the press before it with
/// nothing said, which is P-0094's *an instrument says what it did* read from
/// the far end.
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

/// The console's copy of the offset's two ends and its step are this crate's
/// own, held against the originals here because this is the one package that
/// can see both.
///
/// `karakuri-console` restates `LATENCY_OFFSET_RANGE` and
/// `LATENCY_OFFSET_STEP_MS` because it depends on nothing that could reach them
/// (ADR-0156) and a track has to know its own ends to be laid out. That is
/// `EXPOSURE_STOPS`' arrangement one control along — and it is not its
/// position, which is the reason this test exists: the exposure's bounds are
/// private to `karakuri-cli` and nothing in the workspace can compare them,
/// where these two are public and this binary names both crates. A restatement
/// nobody can check is a copy; one that is checked is a transcription with a
/// guard, which is `docs/contributing.md` §4's second way of making a statement
/// hold.
///
/// And the track's width falls out of the two, which is what makes one pixel
/// one press: eighty five-millisecond steps across four hundred milliseconds,
/// and eighty pixels of track.
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

/// Which half of the octave the panel draws live is the half the beat lock
/// would accept, and neither number is written down twice.
///
/// `BeatLock::octave` refuses when `BPM_RANGE` does not contain `bpm * factor`
/// and nothing else, so [`tracking`] asks the range the same question before
/// the press. The tempos here are the mock's own and the two edges of the
/// range: at 128 only `½` is live, which is exactly what
/// `docs/manual/console.html` draws and says — *"128.0 doubled is 256 and the
/// tracker searches 60 to 200 BPM"*.
///
/// The range is under two octaves wide, which is the mock's other claim about
/// this control — *"the two halves are never both available"* — and it is
/// asserted here rather than assumed, because it is the whole reason the chip
/// has two faces instead of one.
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
