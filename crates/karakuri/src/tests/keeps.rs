use super::*;
use karakuri_console::focus::Step;
use karakuri_console::view::tracker_group;
use karakuri_environment::{audio, mix, Asked};
use karakuri_operation::{BeatSource, GridScale, Operation};
use karakuri_operation_record::{Current, Written};

/// A store root of this test's own, cleared of whatever a previous run left —
/// [`a_library_is_the_store_and_a_missing_store_is_not_made`]'s arrangement, so
/// a keep is written where nothing else is writing.
fn keep_root(what: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "karakuri-keep-{what}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// One keep, gathered as [`Keeping::keep_procedure`] gathers it and run on this
/// thread rather than on a spawned one.
fn keep_run(root: &std::path::Path, asked: Asked, name: &str, source: &str) -> Kept {
    Kept {
        asked,
        name: name.to_owned(),
        root: root.to_path_buf(),
        source: Some(std::sync::Arc::from(source)),
        hash: karakuri_store::hash::Hash::of(source.as_bytes()),
        addr: "L3:0".to_owned(),
        outcome: Ok(std::path::PathBuf::new()),
        reply: None,
    }
    .run()
}

/// The writer puts a node's source under the name it was given, and the outcome
/// says where it went — ADR-0338 decision 4, and it is the act that makes the
/// operator's tier of the library exist at all (P-0096).
///
/// A CPU test: the bytes are the run's and the store is a directory, so nothing
/// here takes a device.
#[test]
fn a_keep_writes_the_nodes_source_into_the_operators_library() {
    let root = keep_root("library");
    let source = "proc orbit_wide {\n  kind L3\n}\n";
    let done = keep_run(&root, Asked::Operator, "orbit_wide", source);
    let path = done
        .outcome
        .as_ref()
        .unwrap_or_else(|e| panic!("the keep was refused: {e}"));
    assert_eq!(
        path,
        &root.join("procedures").join("orbit_wide.kir"),
        "an operator's own act landed outside the library"
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
    let said = done.said().expect("an outcome that landed");
    assert!(
        said.contains("orbit_wide") && said.contains("L3:0"),
        "the outcome names neither what was kept nor where it came from: {said}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A name already kept is refused, and nothing claims otherwise.
///
/// The second keep leaves the first file exactly as it was — a keep is not an
/// instruction to replace, which is where it differs from a Set id and an
/// arrangement's name (`StoreError::ProcedureTaken`).
#[test]
fn a_keep_under_a_name_already_there_is_refused() {
    let root = keep_root("taken");
    let first = "proc orbit_wide {\n  kind L3\n}\n";
    keep_run(&root, Asked::Operator, "orbit_wide", first)
        .outcome
        .expect("the first keep");

    let again = keep_run(
        &root,
        Asked::Operator,
        "orbit_wide",
        "proc other {\n  kind L1\n}\n",
    );
    let said = again.said().expect_err("a taken name was accepted");
    assert!(
        said.contains("orbit_wide"),
        "the refusal does not carry the name the next attempt has to change: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("procedures").join("orbit_wide.kir")).unwrap(),
        first,
        "a refused keep wrote over what was there"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A model's keep lands in the sandbox and never in the library, which is a Set
/// save's own division one file kind along (ADR-0261, ADR-0301).
#[test]
fn a_models_keep_lands_in_the_sandbox() {
    let root = keep_root("sandbox");
    let done = keep_run(
        &root,
        Asked::Model,
        "20260910-120000",
        "proc orbit_wide {\n  kind L3\n}\n",
    );
    let path = done.outcome.as_ref().expect("the keep");
    assert_eq!(
        path,
        &root
            .join(karakuri_store::store::Store::SANDBOX)
            .join("20260910-120000.kir")
    );
    assert!(
        !root.join("procedures").join("20260910-120000.kir").exists(),
        "a model's keep turned up in the operator's library"
    );
    let said = done.said().expect("an outcome that landed");
    assert!(
        said.contains("sandbox"),
        "a model is not told where its keep went: {said}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A node a build landed has its bytes in the store rather than in hand, which
/// is `setfile::SavedNode::source` being `None`: the watcher put the source
/// there as it built it, so the keep reads it back by the address it carries
/// and never re-reads the `.kir` on disk.
#[test]
fn a_rebuilt_nodes_keep_reads_its_source_back_out_of_the_store() {
    let root = keep_root("rebuilt");
    let source = "proc orbit_wide {\n  kind L3\n}\n";
    let store = Store::open(&root).expect("a store");
    let hash = store.put_artifact(source.as_bytes()).expect("an artifact");

    let done = Kept {
        asked: Asked::Operator,
        name: "orbit_wide".to_owned(),
        root: root.clone(),
        // **`None`, which is what a rebuilt node carries.**
        source: None,
        hash,
        addr: "L3:0".to_owned(),
        outcome: Ok(std::path::PathBuf::new()),
        reply: None,
    }
    .run();
    let path = done.outcome.as_ref().expect("the keep");
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
    let _ = std::fs::remove_dir_all(&root);
}

/// The two answers that are not a record. `Silent` is named only here, because
/// nothing in the running window reaches that arm; `Owed` is reachable only by
/// a reading this window failed to take, which is the mask's accident and the
/// sync chip's missing tempo and is asserted below. Neither is a gap in the
/// vocabulary any more — the sync chip's was `Owed::NotSettled` until the
/// conversion took a session tempo, and [`unwritten`] carries what that was.
use karakuri_operation_record::{Owed, Silent};

/// The two spellings of feedback's ceiling are one number, and this is the
/// package that can see both.
///
/// `karakuri_operation::Feedback::MAX` is the reach a fader draws;
/// `karakuri_engine::master::Chain::FEEDBACK_MAX` is the wall the engine clamps
/// at, where the record is applied. Two crates state it because neither may
/// depend on the other, and both say at their own definition that it is a
/// convention held here — which is `Current::tempo`'s arrangement one control
/// along, and the shape `docs/contributing.md` §4 asks for when a guarantee
/// cannot be structural.
///
/// Both directions of the cut list too, for the same reason and by the same
/// route: `mix::cut` takes the engine's word to the vocabulary's and
/// `Cut::parse` takes the record's word back, so a cut that survived one leg
/// and not the other would be a picture a replay draws differently. The
/// crossing lives in `karakuri-environment` beside `mix::tonemap` and
/// `mix::sync`, because this window and `karakuri-cli` both make it.
#[test]
fn the_feedback_ceiling_and_the_cut_list_are_one_answer_in_two_crates() {
    assert_eq!(
        karakuri_operation::Feedback::MAX,
        karakuri_engine::Chain::FEEDBACK_MAX,
        "the reach the console draws and the wall the engine clamps at have drifted"
    );
    // **The wall itself moved into the file**, which is the one thing that
    // changed here when the chain became a list: what a slot's `amount` is
    // clamped to is the range `examples/feedback.kir` declares, and this
    // constant is now the *vocabulary's* number rather than the engine's
    // own — kept in the engine because this is the pair of spellings that
    // has to be checked against each other and there has to be somewhere to
    // check it. **The file declares a wider range than the fader draws**,
    // `[0, 1]` against 0.95, and that is a gap named rather than closed:
    // closing it is a `.kir` edit, which changes the procedure's content
    // address and therefore every stream that names it.
    for cut in Cut::ALL {
        assert_eq!(
            Cut::parse(mix::cut(cut).name()),
            Some(cut),
            "a cut did not survive the round trip through a record's word"
        );
    }
}

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

/// "It only means anything with an audio input attached" —
/// `docs/manual/console.html`, and the operations page's row says it too.
///
/// The offset is a term in the lead a beat correction is applied with, so with
/// no room being listened to there is nothing for the picture to be early or
/// late against and nothing to read the current value off. A value dialled
/// against no session would be dropped the moment one opened — [`attached`]
/// starts a new one at the offset the old one held and at the default when
/// there was none — so the press changes nothing and says why, which is
/// [`tapped`]'s and [`scaled`]'s answer to the same state.
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

/// A press on each of the tracker group's three reaches the operation its key
/// already reaches, and each of the three leaves this window by the door its
/// record decides.
///
/// # Why the three are not one answer
///
/// The offset is `Silent(NoRecord)` and falls through [`App::performed`] to
/// [`nudged`], which is where every route into the session's offset ends. The
/// tap and the octave are `Owed(NotSettled)` — a tap's record is the *beat
/// lock's* answer and no `Current` carries it — so `performed` takes them out
/// before `unwritten` can print *"nothing moved, and nothing here decides it"*
/// about a press that moved the grid. That difference is the whole of
/// [`tracked`], and this is what says the three presses land on the right side
/// of it.
///
/// It needs no device: `view::tracker_group` is a derivation over numbers,
/// which is what makes the panel's arithmetic testable at all (ADR-0156).
#[test]
fn a_press_on_the_tracker_group_reaches_the_operation_its_key_reaches() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.transport = Some(view::Transport {
        bpm: 128.0,
        beats: 144.0,
        beats_per_bar: 4,
        fps: Some(58.0),
        frame_ms: 12.4,
        budget_ms: Some(16.6),
        chain_ms: None,
        // The mock's `landed`, so the group is measured against the row
        // the mock draws rather than a shorter one.
        health: Some(view::Stage::Landed),
        // **And the mock's `rec`**, for the same reason: the pill takes
        // the row's right padding, so a row measured without one puts
        // everything after the bar somewhere the running window does not.
        rec: Some(view::Rec::Idle),
    });
    let mut told = AudioIn::NONE;
    told.device = Some("Scarlett 2i2".to_owned());
    readout.view.audio = Some(told);
    readout.view.tracker = Some(Tracker {
        offset_ms: Some(-15.0),
        halve: true,
        double: false,
    });

    let group = tracker_group(
        &ctx,
        readout.panel.layout(),
        readout.view.transport,
        readout.view.audio.as_ref(),
        readout.view.tracker,
    )
    .expect("the transport row draws the tracker group");
    let offset = group.offset.expect("an open session is holding an offset");
    let point = |at: egui::Pos2| Point::new(at.x, at.y);

    // ---- the tap and the octave: owed, and taken out before `written` --
    for (at, expected, what) in [
        (group.tap.center(), Operation::TapBeat, "the tap capsule"),
        (
            group.halve.center(),
            Operation::ScaleGrid {
                by: GridScale::Halve,
            },
            "the octave's `½`",
        ),
    ] {
        let p = point(at);
        let asked = group
            .tapped(p)
            .or_else(|| group.octave(p))
            .unwrap_or_else(|| panic!("a press on {what} asked for nothing"));
        assert_eq!(
            asked, expected,
            "{what} does not name the operation its key names"
        );
        assert_eq!(
            written(&asked, &Current::default()),
            Written::Owed(Owed::NotSettled),
            "{what}'s operation is no longer owed a record — this window takes it out of \
             `performed` before `unwritten` on exactly that grounds, and the arm that does \
             it is now describing something that is not true"
        );
    }

    // ---- the offset: silent, and it ends where every offset ends -------
    let middle = point(offset.grip.center());
    let asked = group.nudge(middle).expect("a press on the offset track");
    assert_eq!(
        asked,
        Operation::SetLatencyOffset { ms: 0.0 },
        "the middle of a symmetric track does not ask for zero"
    );
    assert_eq!(
        written(&asked, &Current::default()),
        Written::Silent(Silent::NoRecord),
        "the offset started owing a record, and this window applies it to the session \
         instead of handing it to `apply`"
    );
    let mut open: Option<audio::Audio> = None;
    let line = nudged(&mut open, &asked).expect("`nudged` answered nothing for the press");
    assert!(
        line.contains(NO_ROOM_FOR_AN_OFFSET),
        "a press on the track with nothing open does not say what the key says: {line}"
    );
}

/// The legend this window prints for one key, or a panic naming the key that is
/// not in it.
///
/// [`KEYS`] is what the loop prints on startup, so it is the one place this
/// program tells an operator in words what a press does. That makes it
/// something the three tests below can check a constant *against*: a step size
/// compared with a second literal is a copy of itself, and a step size compared
/// with the sentence the operator reads is a measurement.
fn legend(key: &str) -> &'static str {
    KEYS.iter()
        .find(|(name, _)| *name == key)
        .map(|(_, line)| *line)
        .unwrap_or_else(|| panic!("`{key}` is not in the legend this window prints at all"))
}

/// Two levels are the same trim, allowing for the arithmetic: a tenth is not a
/// `f32`, so `0.5 - GAIN_STEP` and `0.4` are two different numbers and neither
/// of them is wrong.
fn same(got: f32, want: f32) -> bool {
    (got - want).abs() <= 1e-6
}

/// "A key steps it" — `docs/manual/operations.html`'s Gain row, whose badge
/// reads `&uarr;&darr; space &middot; in the Mixer`. How far a press goes and
/// what `space` does are decided here rather than there, so both are worth
/// measuring rather than reading.
///
/// The pair and the tenth are `karakuri-cli`'s, taken whole — [`gain_key`]'s
/// own sentence, and the reason they are worth an assertion at all: two
/// keyboards that disagree about how far one press goes is the mistake an
/// operator makes in the dark and cannot see. The size is [`GAIN_STEP`], and
/// the legend [`KEYS`] prints for `up` calls it *a tenth*, so the constant is
/// checked against something this program says out loud rather than against a
/// literal written twice.
///
/// Linear and additive, over several levels rather than one. A step written as
/// a proportion of wherever the trim happens to be reads correct at whichever
/// single level a test picked and is wrong at every other one, and the levels
/// above 1.0 are where that shows.
///
/// The two ends are different ends, and that is the decision in here. The floor
/// is real — a negative gain would subtract one slot's light from another's,
/// which is a blend mode rather than a level — and there is no ceiling, because
/// the pipeline is HDR
/// ([P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)),
/// which the legend for `up` says in as many words: *"the trim is not held at
/// 1.0, because the mix is HDR"*. A trim clamped at unity here would look like
/// tidiness and would quietly cap the mix.
///
/// Held at the floor it says nothing, which is where this differs from the
/// offset three tests up: `offset_said` has a sentence for a press that asked
/// past the bound and this route has none — what an operator gets is the
/// absolute value the operation carries, printed twice.
///
/// It took letters and takes a step since 2026-09-10 (ADR-0333). What the
/// arrows and `space` reach is the *addressed* trim, which is the console's
/// answer and `karakuri-console/tests/grammar.rs`'s; this is the arithmetic at
/// the end of it and is unchanged.
#[test]
fn stepping_the_trim_moves_it_a_tenth_each_way_and_space_names_unity() {
    // Every level that says something different: under the default, at it,
    // and above it — above 1.0 is an ordinary place for an HDR trim to be
    // standing, and it is the level a proportional step gets wrong.
    for from in [0.35_f32, 0.5, 1.0, 1.4, 3.0] {
        let down = gain_key(Step::Down, from);
        let up = gain_key(Step::Up, from);
        assert!(
            same(down, from - GAIN_STEP),
            "a down press took a trim standing at {from} to {down}, which is not one step of \
             {GAIN_STEP} down — the size is `karakuri-cli`'s, and two keyboards that disagree \
             about it is the mistake nobody can see"
        );
        assert!(
            same(up, from + GAIN_STEP),
            "an up press took a trim standing at {from} to {up}, which is not one step of \
             {GAIN_STEP} up"
        );
        assert!(
            down < from && from < up,
            "at {from} the two directions did not go opposite ways: down gave {down} and up \
             gave {up}"
        );
        assert_eq!(
            gain_key(Step::Default, from),
            1.0,
            "`space` is a destination and not a step, so from {from} it names 1.0 and nothing \
             else"
        );
    }

    // **The step against the sentence the operator reads**, and the guard
    // that keeps that comparison worth making: without the second
    // assertion the first is a constant checked against a literal.
    assert_eq!(
        GAIN_STEP,
        0.1,
        "the window prints `{}` for `up` and the arm steps by {GAIN_STEP} — an operator \
         reading the legend is told a tenth and gets something else",
        legend("up")
    );
    assert!(
        legend("up").contains("a tenth"),
        "the legend for `up` no longer calls the step a tenth, so the assertion above is \
         comparing the constant against nothing: {}",
        legend("up")
    );

    // **Floored, and it holds there in silence.** A press at the bottom of
    // the travel asks below zero and gets zero, which is the clamp
    // `gain_key` makes rather than one `Deck::set_gain` would make later —
    // it decides what the *record* says, so a session replays the value
    // that took effect.
    assert_eq!(
        gain_key(Step::Down, 0.05),
        0.0,
        "a press that asked below zero came out negative, and a negative gain subtracts \
         one slot's light from another's — a blend mode rather than a level"
    );
    assert_eq!(
        gain_key(Step::Down, 0.0),
        0.0,
        "a press at the floor did not hold at the floor"
    );

    // **And not ceilinged**, which is the half a clamp would have got
    // wrong quietly: an up press at unity is an ordinary press into the
    // HDR mix.
    let over = gain_key(Step::Up, 1.0);
    assert!(
        over > 1.0 && same(over, 1.0 + GAIN_STEP),
        "an up press at 1.0 came out at {over} — the trim was capped at unity, and the mix \
         is HDR"
    );
    assert!(
        legend("up").contains("not held at 1.0"),
        "the legend for `up` no longer says the trim is uncapped, so the assertion above \
         and the window disagree about which of them is the specification: {}",
        legend("up")
    );
    assert!(
        same(gain_key(Step::Up, 4.0), 4.0 + GAIN_STEP),
        "a trim already well above unity stopped stepping up"
    );
    // And `space` comes back down from up there, which is what makes it the
    // way out of a mix somebody has pushed: a destination, not a step.
    assert_eq!(gain_key(Step::Default, 4.0), 1.0);
}

/// "A key steps it" one control along, and the difference between the two is
/// the whole of this test.
///
/// `docs/manual/operations.html`'s Opacity row carries the same badge as the
/// Gain row — `&uarr;&darr; space &middot; in the Mixer` — and is silent about
/// the size for [`gain_key`]'s reason, so the step is [`OPACITY_STEP`] and it
/// is `karakuri-cli`'s.
///
/// The clamp is this surface's and it is the reason for the test. Opacity is a
/// proportion of a blend and there is no such thing as 1.4 of one, where gain
/// is a level into an HDR mix — the legend says as much at the key: *"the fader
/// is held inside 0 and 1"*. It is clamped here rather than left to
/// `Deck::set_opacity`, because this decides what the record says: a session
/// replays the value that took effect rather than one the engine quietly
/// corrected.
///
/// So the two ends are asserted against the trim's, which is the shape a clamp
/// copied from one control to the other would break: at 1.0 the fader holds and
/// the trim does not.
///
/// The default is new and the two steps are not (ADR-0333). `\` had no partner
/// on this control, so `space` on an addressed fader is the first way back to
/// unity it has ever had.
#[test]
fn stepping_the_fader_moves_it_a_tenth_each_way_and_holds_inside_zero_and_one() {
    for from in [0.15_f32, 0.3, 0.5, 0.85] {
        let down = opacity_key(Step::Down, from);
        let up = opacity_key(Step::Up, from);
        assert!(
            same(down, from - OPACITY_STEP),
            "a down press took a fader standing at {from} to {down}, which is not one step \
             of {OPACITY_STEP} down"
        );
        assert!(
            same(up, from + OPACITY_STEP),
            "an up press took a fader standing at {from} to {up}, which is not one step of \
             {OPACITY_STEP} up"
        );
        assert!(
            down < from && from < up,
            "at {from} the pair did not go opposite ways: down gave {down} and up gave {up}"
        );
    }

    assert_eq!(
        OPACITY_STEP,
        0.1,
        "the window prints `{}` for `up` and the fader steps by {OPACITY_STEP}",
        legend("up")
    );

    // **Both ends, and both of them hold.** A press at either end of the
    // travel asks past it and gets the end.
    assert_eq!(
        opacity_key(Step::Down, 0.05),
        0.0,
        "a press that asked below zero came out negative — there is no less than none of \
         a layer"
    );
    assert_eq!(
        opacity_key(Step::Down, 0.0),
        0.0,
        "a press at the bottom of the travel did not hold there"
    );
    assert_eq!(
        opacity_key(Step::Up, 0.95),
        1.0,
        "a press that asked past one came out above it — there is no 1.05 of a blend"
    );
    assert_eq!(
        opacity_key(Step::Up, 1.0),
        1.0,
        "a press at the top of the travel did not hold there"
    );
    assert!(
        legend("up").contains("held inside 0 and 1"),
        "the legend for `up` no longer says the fader is held inside its two ends: {}",
        legend("up")
    );

    // **The fader's default is unity too**, which is the one thing this
    // control gained rather than inherited.
    assert_eq!(
        opacity_key(Step::Default, 0.3),
        1.0,
        "`space` on an addressed fader did not name the value it was declared at"
    );

    // **The two controls part company at 1.0, and that is the assertion a
    // clamp copied across would fail.** The same press, at the same level,
    // on the two controls the page draws two rows for: the fader holds and
    // the trim goes on up.
    assert_ne!(
        gain_key(Step::Up, 1.0),
        1.0,
        "the trim is now held at one as well, so the two controls have been made the same \
         control — and the page draws two rows because they are not: *opacity at zero \
         silences under every blend mode, gain at zero does not silence `over`*"
    );
}

/// Every verdict the engine can report says what it does to the lane, and a row
/// leaves it only when the file and the picture agree.
///
/// The six `swap::Event` variants are three answers: four that put a row on the
/// lane under one of `view::Stage`'s words, one that takes it off, and one that
/// is not about a version at all. The one worth the test is `Accepted`: it is
/// the watchdog's verdict and not the operator's, and it is what clears a row
/// because *keep a candidate* has no control on this panel — see
/// `view::staging`, where that substitution is argued. A run in which
/// `Accepted` did nothing would be a lane that fills up and never empties,
/// which is not the lane the manual describes.
///
/// And the rows stay in slot order, which is the order they are drawn in: a
/// candidate that lands on deck B and then one on deck A must not leave the
/// lane reading B over A, because the letter is the only thing telling two rows
/// of the same material apart.
///
/// A CPU test: an `Event` is a value, and nothing here takes a device.
#[test]
fn every_verdict_says_what_it_does_to_the_lane() {
    let landed = |label: &str| Event::Swapped {
        id: 1,
        label: label.into(),
    };
    let refused = |label: &str| Event::Rejected {
        id: 2,
        label: label.into(),
        error: karakuri_engine::set::SetError::NoCapacity("drift_shell".to_owned()),
    };
    // **The candidate's own cost and not a frame interval**, since
    // ADR-0313: 24 ms is what one frame of that Set was measured at, held
    // against one frame of the display. The lane does not read the number
    // — it reads which verdict it was — but a value that named the wrong
    // quantity here would be a test teaching the wrong sentence.
    let stopped = |label: &str| Event::Overloaded {
        id: 3,
        label: label.into(),
        cost_ms: 24.0,
        basis: karakuri_engine::Basis::Measured,
        budget_ms: 20.0,
    };
    let accepted = Event::Accepted {
        id: 4,
        label: "drift_shell + soft_points".into(),
        cost_ms: Some(9.0),
        basis: karakuri_engine::Basis::Measured,
        budget_ms: 20.0,
    };

    assert_eq!(
        verdict(&landed("a + b")),
        Verdict::Waiting("a + b", view::Stage::Landed)
    );
    assert_eq!(
        verdict(&refused("a + b")),
        Verdict::Waiting("a + b", view::Stage::Refused)
    );
    assert_eq!(
        verdict(&stopped("a + b")),
        Verdict::Waiting("a + b", view::Stage::Overloaded)
    );
    // **The checker's refusal, which is a fourth word and not the
    // build's.** `Rejected` above is a Set that would not assemble;
    // this is a source that never became one, and the two would be
    // indistinguishable on the lane if they shared a `Stage`.
    assert_eq!(
        verdict(&Event::SourceRefused {
            label: "drift_shell.kir".into(),
            said: vec!["3:5: parse: expected `}`".to_owned()],
        }),
        Verdict::Waiting("drift_shell.kir", view::Stage::NotCompiled),
        "a source the checker turned down reached no row, which is the silence the lane              exists to end"
    );
    assert_eq!(
        verdict(&accepted),
        Verdict::Settled,
        "an accepted build leaves the file and the picture agreeing, so its row stays \
         on a lane nothing can clear"
    );
    assert_eq!(
        verdict(&Event::WorkerLost),
        Verdict::Nothing,
        "the worker going is not a verdict on any version, and every row already \
         taken still stands"
    );

    // And the same three through `settle`, which is what a drain does with
    // them: a slot's rows in slot order, replaced whole.
    let mut lane: Vec<view::Candidate> = Vec::new();
    settle(&mut lane, 1, "b + b", view::Stage::Landed, &[], &[]);
    settle(&mut lane, 0, "a + a", view::Stage::Landed, &[], &[]);
    assert_eq!(
        lane.iter().map(|row| row.deck).collect::<Vec<_>>(),
        vec![0, 1],
        "the rows are not in the order the letters are drawn in"
    );
    // **A verdict with no changed node draws one row with no address**,
    // which is what a build that did not happen and a rebuild that changed
    // nothing both come to — see `settle` and ADR-0326.
    assert_eq!(lane[0].at, None, "a verdict with no diff invented a node");
    assert!(lane[0].addr.is_empty(), "and drew an address for it");

    settle(&mut lane, 0, "a + a", view::Stage::Overloaded, &[], &[]);
    assert_eq!(
        lane.len(),
        2,
        "a second verdict on one slot made a second row"
    );
    assert_eq!(lane[0].stage, view::Stage::Overloaded);
    assert_eq!(
        lane[0].name, "a + a",
        "the name was not left as it was found"
    );

    settle(&mut lane, 0, "c + c", view::Stage::Landed, &[], &[]);
    assert_eq!(
        lane[0].name, "c + c",
        "a rebuild of other material kept the old name"
    );

    // **And a refusal's diagnostics reach the row, then leave it with the
    // refusal.** The row is one slot's *newest* verdict, so a build that
    // lands after a refusal must not be drawn under the sentence the
    // refusal put there — which is a stale diagnostic beside material it
    // is not about, and reads as a fault in the build that just worked.
    let said = [
        "3:5: parse: expected `}`".to_owned(),
        "7:1: type: unknown builtin `curl2`".to_owned(),
    ];
    settle(
        &mut lane,
        0,
        "drift_shell.kir",
        view::Stage::NotCompiled,
        &said,
        &[],
    );
    assert_eq!(
        lane[0].said, said,
        "the lane row was told what the checker said and did not keep it"
    );
    settle(&mut lane, 0, "c + c", view::Stage::Landed, &[], &[]);
    assert!(
        lane[0].said.is_empty(),
        "a build landed on a row still carrying the refusal's diagnostics: {:?}",
        lane[0].said
    );

    lane.retain(|row| row.deck != 0);
    assert_eq!(
        lane.iter().map(|row| row.deck).collect::<Vec<_>>(),
        vec![1],
        "clearing one slot's row took another slot's with it"
    );
}

/// One build that changed two nodes draws two rows, and one that changed one
/// draws one — the maintainer's decision on 2026-09-09, and [`settle`] is where
/// it is carried out.
///
/// > one row per changed node
///
/// The verdict is repeated on each row rather than standing over them, which is
/// the half of that decision a nested shape would have spent: a row is a row,
/// and both of these carry the same word. What tells them apart is the address,
/// which is why it is asserted here beside the count — two rows drawn from one
/// build with one address between them would be two lines saying one thing.
///
/// And a slot's rows are replaced whole. A build that changed two and then a
/// build that changed one leaves one row, not the first build's second row
/// standing under a verdict that has been superseded. That is the property
/// `settle` was rewritten for and the one a `find`-and-write would have missed.
///
/// A CPU test: a `Changed` is a value and nothing here takes a device.
#[test]
fn a_build_that_changed_two_nodes_draws_a_row_each() {
    let node = |layer: karakuri_operation::Layer, index: u32, name: &str| Changed {
        at: karakuri_operation::NodeAddress { layer, index },
        addr: node_addr(ir_layer(layer), index),
        name: name.to_owned(),
    };
    let both = [
        node(karakuri_operation::Layer::L1, 0, "drift_shell"),
        node(karakuri_operation::Layer::L4, 0, "soft_points"),
    ];
    let mut lane: Vec<view::Candidate> = Vec::new();
    settle(
        &mut lane,
        0,
        "drift_shell + soft_points",
        view::Stage::Landed,
        &[],
        &both,
    );
    assert_eq!(
        lane.len(),
        2,
        "one build changed two nodes and the lane drew {} row(s)",
        lane.len()
    );
    assert_eq!(
        lane.iter().map(|row| row.addr.as_str()).collect::<Vec<_>>(),
        vec!["L1:0", "L4:0"],
        "the two rows do not name the two nodes"
    );
    assert_eq!(
        lane.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
        vec!["drift_shell", "soft_points"],
        "a row carries the build's label rather than its own node's name"
    );
    assert!(
        lane.iter().all(|row| row.stage == view::Stage::Landed),
        "the verdict is not on both rows"
    );
    assert_eq!(
        lane.iter().map(|row| row.at).collect::<Vec<_>>(),
        both.iter().map(|node| Some(node.at)).collect::<Vec<_>>(),
        "a row's payload and its drawn address do not name the same node"
    );

    // **A second slot's rows go after the first's**, which is the order
    // the letters are drawn in.
    settle(
        &mut lane,
        1,
        "drift_shell + soft_points",
        view::Stage::Overloaded,
        &[],
        &both[1..],
    );
    assert_eq!(
        lane.iter().map(|row| row.deck).collect::<Vec<_>>(),
        vec![0, 0, 1],
        "the rows are not in slot order"
    );

    // **And the next build on slot 0 replaces both of its rows.** A build
    // that changed one node leaves one row on that slot, and the row the
    // build before it drew does not survive its own verdict.
    settle(
        &mut lane,
        0,
        "drift_shell + soft_points",
        view::Stage::Landed,
        &[],
        &both[..1],
    );
    assert_eq!(
        lane.iter()
            .map(|row| (row.deck, row.addr.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "L1:0"), (1, "L4:0")],
        "a build that changed one node left the previous build's second row standing"
    );
}
