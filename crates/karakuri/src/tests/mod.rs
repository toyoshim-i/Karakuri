use super::*;

pub(crate) use self as tests;
pub(crate) use crate::session::Sessions;

use std::time::{Duration, Instant};

use karakuri_console::focus::Step;
use karakuri_console::input::Claim;
use karakuri_console::panel::Panel;
use karakuri_console::view::{
    look as look_row, mixer as mixer_bay, picture_rect, preview_rects, tracker_group, Picture,
    Reading, RowKind, Scope, Tracker, TransitionSettings, View, DECKS,
};
use karakuri_console::{egui, egui_wgpu};
use karakuri_engine::set::Layering;
use karakuri_engine::transport::Sync as EngineSync;
use karakuri_engine::{
    compose, Blend, Committed, Cut, Deck, DeckSlot as EngineSlot, Event, Gpu, Look, Mask, MaskKind,
    Residency, Sink, Skip, TonemapOp,
};
use karakuri_environment::{audio, mix, setfile, watch, Asked, Opening};
use karakuri_layout::{Layout, Point};
use karakuri_operation::{BeatSource, BlendMode, GridScale, Operation, SetTransfer, Undecided};
use karakuri_operation_record::{written, Current, Written};
use karakuri_store::record::{DeckSlot, Record};
use karakuri_store::store::Store;
use winit::event::WindowEvent;

mod focus_keys;
mod gpu;
mod outputs_row;
mod press_handler;

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

/// How far one press of the deck head's scrub goes, and the order its sync chip
/// walks the modes in — read from the control that has them rather than written
/// again here: the arrow and the record it becomes are one number, and
/// `Pane::allows` lines up with `SYNCS` or the cycle skips the wrong mode.
use karakuri_console::view::{SCRUB_BEATS, SYNCS};
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
/// operator cannot see from the panel: this program takes the first input there
/// is (ADR-0220's reason one column along — the instrument has no `--midi-in`),
/// and a run that took the wrong one of two would look exactly like a run whose
/// controller is asleep. 2. The map is named, and by its path as well as its
/// name. `default` under the store and `surface` in the preset library are two
/// files, and an operator who has just learned one wants to know which is
/// loaded. 3. No map is a state, with the sentence that tells them what to do
/// next, and nothing plugged in is not a fault.
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
/// state, and says what goes on answering. 2. A device that was named and is
/// not there is loud (P-0094): the sentence carries the list, so an operator
/// who picked a cable that has gone is holding the right names rather than an
/// invitation to go and look. 3. And a refused pick does not take the room
/// away. Nothing is open in this test, so what is asserted is the half that can
/// be: the answer says so rather than going quiet.
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
/// > 変更ノードごとに 1 行
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

/// The library is what the store holds, and a store that is not there is listed
/// as nothing rather than created.
///
/// Two claims, and the second is the one worth a test: `Store::open`
/// establishes the layout it is pointed at, so a listing that opened first
/// would leave a `.karakuri` behind in whatever directory this program was run
/// from. [`library`] asks whether the root is there before it opens anything,
/// and this is what says so.
///
/// A CPU test: nothing here takes a device, and the store is a directory.
#[test]
fn a_library_is_the_store_and_a_missing_store_is_not_made() {
    let root = std::env::temp_dir().join(format!(
        "karakuri-console-library-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    // Whatever a previous run left, so the first claim is about a root
    // that is genuinely not there.
    let _ = std::fs::remove_dir_all(&root);

    assert!(
        library(&root).is_empty(),
        "a store that is not there listed something"
    );
    assert!(
        !root.exists(),
        "listing a library that is not there created one at {}",
        root.display()
    );

    // And with two Sets in it, both names come back — **most recent
    // first**, which is what the operation's own row says a listing is and
    // what the MCP tool already answered. `morph01` is written second and
    // is first here either way the clock falls: on a fine one it is the
    // more recent, and on a coarse one the two mtimes tie and the tie-break
    // is the id ascending.
    let store = Store::open(&root).expect("a store to list");
    for id in ["night01", "morph01"] {
        store
            .write_set(id, &[])
            .unwrap_or_else(|e| panic!("writing {id}: {e}"));
    }
    let listed: Vec<String> = library(&root).into_iter().map(|set| set.id).collect();
    assert_eq!(
        listed,
        vec!["morph01".to_owned(), "night01".to_owned()],
        "the bay lists {listed:?}"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A version, written where `history::list` reads them.
///
/// The name is `Snapshots::record`'s own — a `HHMMSS-mmm` stamp, the slot, the
/// layer with its index where it is not the first, the procedure name, and
/// `@<set>` where there was a Set — and it is spelled here rather than recorded
/// through that type because what these tests are about is the *reading*: a
/// version filed under a Set, one filed under none, and the difference between
/// them.
fn version_file(root: &std::path::Path, day: &str, name: &str, body: &str) {
    let dir = root.join("history").join(day);
    std::fs::create_dir_all(&dir).expect("a day directory");
    std::fs::write(dir.join(name), body).expect("a version");
}

/// The `history` scope lists the versions of the Set the load pulldown's deck
/// is running, and a version filed under no Set is not one of them.
///
/// That last clause is the one ADR-0276 wrote down and ADR-0308 had to obey:
/// *"a narrowing must treat a `None` row as matching no Set rather than as a
/// wildcard."* A run launched on a pair somebody typed files every version it
/// writes under none, so a wildcard would put the whole of that run's editing
/// under whatever Set the operator loaded afterwards — silently, in a bay whose
/// rows are names.
///
/// And a deck running nothing lists nothing, with the sentence saying which
/// nothing it is. A CPU test: `listing` reaches a disk and no device.
#[test]
fn a_history_listing_is_one_sets_versions_and_a_none_row_is_nobodys() {
    let root = scratch_dir("history-listing");
    std::fs::create_dir_all(&root).expect("a store root");
    version_file(
        &root,
        "2026/09/08",
        "143052-271_slot0_L4_beat_strokes@x.kir",
        "kind L4\n",
    );
    version_file(
        &root,
        "2026/09/08",
        "142930-004_slot0_L1_drift_shell@x.kir",
        "kind L1\n",
    );
    // **Under no Set at all**, which is the row that must not match.
    version_file(
        &root,
        "2026/09/08",
        "142800-000_slot1_L4_soft_points.kir",
        "kind L4\n",
    );
    // **And under another Set**, so that *narrowed to `x`* is a claim with
    // something to be wrong about.
    version_file(
        &root,
        "2026/09/07",
        "235959-999_slot2_L4_other_thing@y.kir",
        "kind L4\n",
    );

    let mut view = View::new(karakuri_console::room::Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert!(view.select_scope(Scope::History), "the mark did not move");

    let said = listing(&mut view, &root, None, None, Some("x"));
    assert_eq!(
        view.library,
        vec![
            "20260908-143052-271_slot0_L4_beat_strokes".to_owned(),
            "20260908-142930-004_slot0_L1_drift_shell".to_owned(),
        ],
        "the walk listed {:?} — a row of another Set, or a row filed under none, was \
         treated as one of `x`'s",
        view.library
    );
    assert!(
        said.contains("history") && said.contains('2'),
        "the line does not say what the scope listed: {said}"
    );

    // **A deck running the pair the run launched with**, which is every
    // deck of a fresh run: the versions under `None` are exactly the rows
    // this would list if a `None` matched anything, so an empty listing
    // here is the same claim as above read from the other side.
    let said = listing(&mut view, &root, None, None, None);
    assert!(
        view.library.is_empty(),
        "a deck running no Set listed {:?}",
        view.library
    );
    assert!(
        said.contains("typed pair"),
        "the line does not say why the scope is empty: {said}"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A landing writes the version's bytes over the node's working copy, and a
/// file that is gone is refused by name.
///
/// The write is the whole of what a landing does on this side: nothing touches
/// a deck, nothing sends an aim, and the watcher already looking at that file
/// is what builds it — ADR-0228's argument met from the other end. So what this
/// asserts is the bytes and the path: the file the version was of, resolved
/// through the slot's own nodes rather than through the copies the run launched
/// with.
///
/// The refusal names the file, because `rm -rf history/2026/07` is this store's
/// whole retention policy: a row whose file an operator deleted by hand is an
/// ordinary state, and P-0083 says a refusal carries what the next attempt
/// needs.
///
/// A CPU test: [`put_back`] takes a store, a slot and a `Slots`, and no device.
#[test]
fn a_landing_writes_the_versions_bytes_over_the_nodes_working_copy() {
    let root = scratch_dir("history-landing");
    let scratch = root.join(karakuri_environment::scratch::DIR);
    std::fs::create_dir_all(&scratch).expect("a scratch");
    let head = scratch.join("A0-drift_shell.kir");
    let rest = scratch.join("A1-soft_points.kir");
    std::fs::write(&head, "kind L1\n// what is playing\n").expect("the head");
    std::fs::write(&rest, "kind L4\n// what is playing\n").expect("the renderer");
    let slots = karakuri_mcp::Slots::of(vec![(head.clone(), vec![rest.clone()])]);

    version_file(
        &root,
        "2026/09/08",
        "143052-271_slot0_L4_soft_points@x.kir",
        "kind L4\n// the version an operator picked\n",
    );
    let picked = "20260908-143052-271_slot0_L4_soft_points";

    let said = put_back(
        &root,
        0,
        "A",
        "x",
        &karakuri_operation::Revision::Picked(picked.to_owned()),
        &slots,
    );
    assert!(
        said.contains(&rest.display().to_string()),
        "the line does not name the file it wrote: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(&rest).expect("the renderer's working copy"),
        "kind L4\n// the version an operator picked\n",
        "the version's bytes are not what the node is playing from"
    );
    assert_eq!(
        std::fs::read_to_string(&head).expect("the head's working copy"),
        "kind L1\n// what is playing\n",
        "landing on the L4 wrote over the L1 as well"
    );

    // **The file behind the row is gone**, which is the retention policy
    // being used: the name is still in nothing this program keeps, so the
    // walk is re-asked and the row is simply not there.
    std::fs::remove_dir_all(root.join("history")).expect("the operator's own `rm -rf`");
    let said = put_back(
        &root,
        0,
        "A",
        "x",
        &karakuri_operation::Revision::Picked(picked.to_owned()),
        &slots,
    );
    assert!(
        said.contains(picked) && said.contains("nothing moved"),
        "a landing on a version that is gone did not say so: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(&rest).expect("the renderer's working copy"),
        "kind L4\n// the version an operator picked\n",
        "a refused landing wrote something"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A step back lands the version before the one the node is running, and a node
/// with only that one version is refused saying so.
///
/// The Staging lane's arm of `karakuri_operation::Revision`, and the whole of
/// what it adds over the Library bay's: the surface says the node, and which
/// version that is, is worked out here — the history walked for that node of
/// that Set, most recent first, with the entry after the newest taken
/// (ADR-0326). The newest is what the slot is running, because the history is
/// gated on compiling and not on landing, so a version stopped for cost is
/// filed too and is the one an operator most wants to step away from.
///
/// Three versions and not two, so that *the one before the one running* is a
/// different answer from *the oldest*: a resolver that took the last row of the
/// chain would pass a two-version fixture and land the wrong file here.
///
/// And the chain is narrowed by the node as well as by the Set, which is the
/// assertion the L1 version beside them makes: a version of another node of the
/// same Set is a newer row of the same listing, so a walk that narrowed only by
/// the Set would call it *the one running* and land the L4's own newest as the
/// step back.
///
/// A CPU test: [`put_back`] takes a store, a slot and a `Slots`, and no device.
#[test]
fn a_step_back_lands_the_version_before_the_one_running() {
    let root = scratch_dir("history-step-back");
    let scratch = root.join(karakuri_environment::scratch::DIR);
    std::fs::create_dir_all(&scratch).expect("a scratch");
    let head = scratch.join("A0-drift_shell.kir");
    let rest = scratch.join("A1-soft_points.kir");
    std::fs::write(&head, "kind L1\n// what is playing\n").expect("the head");
    std::fs::write(&rest, "kind L4\n// the third\n").expect("the renderer");
    let slots = karakuri_mcp::Slots::of(vec![(head.clone(), vec![rest.clone()])]);

    // Oldest first here so the file reads in the order the operator wrote
    // them; `history::list` answers the other way round, which is the
    // ordering this test is about.
    for (at, body) in [
        ("143050-100", "kind L4\n// the first\n"),
        ("143051-100", "kind L4\n// the second\n"),
        ("143052-100", "kind L4\n// the third\n"),
    ] {
        version_file(
            &root,
            "2026/09/09",
            &format!("{at}_slot0_L4_soft_points@x.kir"),
            body,
        );
    }
    // **A newer version of another node of the same Set**, which is what
    // says the walk narrows by the node.
    version_file(
        &root,
        "2026/09/09",
        "143053-100_slot0_L1_drift_shell@x.kir",
        "kind L1\n// a newer edit of the geometry\n",
    );

    let renderer = karakuri_operation::NodeAddress {
        layer: karakuri_operation::Layer::L4,
        index: 0,
    };
    let said = put_back(
        &root,
        0,
        "A",
        "x",
        &karakuri_operation::Revision::Previous(renderer),
        &slots,
    );
    assert!(
        said.contains(&rest.display().to_string()),
        "the line does not name the file it wrote: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(&rest).expect("the renderer's working copy"),
        "kind L4\n// the second\n",
        "a step back landed something other than the version before the one running"
    );
    assert_eq!(
        std::fs::read_to_string(&head).expect("the head's working copy"),
        "kind L1\n// what is playing\n",
        "stepping the L4 back wrote over the L1 as well"
    );

    // **A node with one version has nothing before what it is playing**,
    // which is an ordinary state rather than a fault — the first edit of a
    // node files one version, and a step back at that point has nowhere to
    // go. P-0083: the sentence names what is in the way.
    let geometry = karakuri_operation::NodeAddress {
        layer: karakuri_operation::Layer::L1,
        index: 0,
    };
    let said = put_back(
        &root,
        0,
        "A",
        "x",
        &karakuri_operation::Revision::Previous(geometry),
        &slots,
    );
    assert!(
        said.contains("L1:0") && said.contains("one version"),
        "a step back with nothing behind it did not say what is in the way: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(&head).expect("the head's working copy"),
        "kind L1\n// what is playing\n",
        "a refused step back wrote something"
    );

    // **And a node the history has never heard of**, which is the same
    // refusal one step further out: nothing is filed, so there is not even
    // a version running.
    let field = karakuri_operation::NodeAddress {
        layer: karakuri_operation::Layer::Field,
        index: 0,
    };
    let said = put_back(
        &root,
        0,
        "A",
        "x",
        &karakuri_operation::Revision::Previous(field),
        &slots,
    );
    assert!(
        said.contains("no version"),
        "a step back on a node with nothing filed did not say so: {said}"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A keep takes its own row off the lane and leaves every other row standing.
///
/// The half of the press that is not the operation: [`kept`] is where a
/// `KeepCandidate` is performed, because `written` answers
/// `Silent(Silent::Surface)` for it and there is no record for `apply` to move
/// a deck with. What it changes is one line in one list.
///
/// Two rows of one slot is the case worth the test, and it is what ADR-0326
/// made possible: a build that changed two nodes draws two rows on one deck, so
/// a keep that retired by *slot* would take a node nobody ruled on off the lane
/// with the one they did.
///
/// And a keep on a node with no row says so rather than reporting a press that
/// did nothing — no control on this panel can ask it, so the line is for the
/// day a map or a model reaches this row.
///
/// A CPU test: a `View` takes no device.
#[test]
fn a_keep_takes_its_own_row_off_the_lane() {
    let node = |layer: karakuri_operation::Layer, index: u32, name: &str| Changed {
        at: karakuri_operation::NodeAddress { layer, index },
        addr: node_addr(ir_layer(layer), index),
        name: name.to_owned(),
    };
    let mut view = View::new(karakuri_console::room::Room::Day);
    let both = [
        node(karakuri_operation::Layer::L1, 0, "drift_shell"),
        node(karakuri_operation::Layer::L4, 0, "soft_points"),
    ];
    settle(
        &mut view.staging,
        0,
        "drift_shell + soft_points",
        view::Stage::Landed,
        &[],
        &both,
    );
    settle(
        &mut view.staging,
        1,
        "drift_shell + soft_points",
        view::Stage::Landed,
        &[],
        &both[..1],
    );
    assert_eq!(view.staging.len(), 3, "three rows over two slots");

    let said = kept(
        &mut view,
        &Operation::KeepCandidate {
            deck: 0,
            node: both[0].at,
        },
    )
    .expect("a keep is performed here");
    assert!(
        said.contains("L1:0") && said.contains("deck A"),
        "the line does not say which row left: {said}"
    );
    assert_eq!(
        view.staging
            .iter()
            .map(|row| (row.deck, row.addr.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "L4:0"), (1, "L1:0")],
        "a keep took a row it was not addressed to"
    );

    // **A second keep on the same node has no row to take**, which is the
    // negative control: a version that retired by slot, or one that
    // ignored the node, would go on answering as though it had done
    // something.
    let said = kept(
        &mut view,
        &Operation::KeepCandidate {
            deck: 0,
            node: both[0].at,
        },
    )
    .expect("a keep is performed here");
    assert!(
        said.contains("no candidate row"),
        "a keep on a node with no row claimed to have kept one: {said}"
    );
    assert_eq!(view.staging.len(), 2, "the second keep took a row anyway");

    // And an operation that is not a keep is not this function's.
    assert_eq!(
        kept(&mut view, &Operation::SelectDeck { deck: 1 }),
        None,
        "`kept` answered for an operation that is not a keep"
    );
}

/// A temporary directory of this test's own, named after the test that wants it
/// — the shape every other CPU test in this file uses.
pub(crate) fn scratch_dir(what: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "karakuri-{what}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    // Whatever a previous run left behind, so the claims are about what
    // this run put there.
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// `examples/`, as a preset library this run was told about.
fn shipped_presets() -> karakuri_environment::places::Presets {
    karakuri_environment::places::Presets {
        dir: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples"),
        found: karakuri_environment::places::Found::Given,
    }
}

/// The `presets` scope lists the Set files and not the parts beside them, which
/// is `console.html`'s *"a directory of `.kir` files is a directory of parts,
/// and a library lists what you can put on a deck"*.
///
/// It is asserted against `examples/`, which is the directory this program
/// actually opens on: thirty-five parts and twenty-three Set files in one place
/// is exactly the mixture the rule is about, and a listing that took the parts
/// would draw fifty-eight rows of which thirty-five name nothing this
/// vocabulary can load.
///
/// A CPU test: a preset library is a directory.
#[test]
fn the_presets_scope_lists_the_kset_files_and_not_the_parts_beside_them() {
    let presets = shipped_presets();
    let listed = presets_listing(Some(&presets));
    assert!(
        listed.len() >= 20,
        "`examples/` holds twenty-three `.kset` files and the listing found {}",
        listed.len()
    );
    for preset in &listed {
        assert!(
            preset.path.extension().and_then(|e| e.to_str()) == Some("kset"),
            "`{}` is listed and is not a Set file",
            preset.path.display()
        );
        assert!(
            !preset.id.ends_with(".kset") && !preset.id.is_empty(),
            "the row reads `{}`, which is a file name rather than a name",
            preset.id
        );
    }
    assert!(
        listed.iter().any(|preset| preset.id == "beat_cloud"),
        "`beat_cloud.kset` is in `examples/` and the listing does not have it"
    );
    assert!(
        !listed
            .iter()
            .any(|preset| preset.id.contains("drift_shell")),
        "`drift_shell.kir` is a part and the listing took it for a row"
    );

    // **Sorted, because a directory read is not.** Two runs that drew the
    // rows in two orders would be a bay nobody can point at.
    let mut sorted = listed.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
    sorted.sort();
    assert_eq!(
        listed.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
        sorted,
        "the listing is not in name order"
    );

    // And no preset library at all is no rows, which is a state rather
    // than a failure.
    assert!(presets_listing(None).is_empty());
}

/// A folder row is taken into the store and then loaded, which is the half of
/// *Send a Set to somebody, and take one in* the folder scope was refused for
/// until it had a directory.
///
/// It is the preset press over somebody else's directory — ADR-0267's *"the bay
/// is already a file browser"* reached from the taking-in side, and ADR-0275 is
/// what gave the scope a directory to be pointed at. The two arms are one arm
/// in the press handler and [`Taking`] is the whole of the difference, so this
/// asserts the difference rather than the shared half: the same `taking_in`,
/// pointed at a folder.
///
/// Both spellings, because a folder holds both and a presets root holds one.
/// `examples/` is a directory of `.kset` files, which is the authored form; the
/// store the first take-in wrote is a directory of `.kbset` files, which is the
/// bundle — so pointing a second store's folder scope at the first store's
/// `sets/` is a take-in of a form the `presets` scope could never have offered.
/// That is the branch this row gained and the one `karakuri-cli`'s `--take-in`
/// has always had.
///
/// A CPU test: a store, a directory, and no window.
#[test]
fn a_folder_row_is_taken_in_by_the_same_press_a_preset_row_is() {
    let root = scratch_dir("folder-take-in");
    Store::open(&root).expect("a store to take into");
    let examples = shipped_presets().dir;

    // **The authored form, out of a folder rather than out of `presets`.**
    // The rows are the same words the bay draws, and the file behind one
    // is found by asking the directory again on the press.
    assert!(
        folder_listing(Some(&examples))
            .iter()
            .any(|id| id == "beat_cloud"),
        "the folder scope does not list `beat_cloud` in `examples/`"
    );
    let taken = taking_in(&root, Taking::Folder(Some(&examples)), "beat_cloud")
        .expect("a folder row is taken in");
    assert_eq!(taken.id, "beat_cloud");
    assert_eq!(
        library(&root)
            .into_iter()
            .map(|set| set.id)
            .collect::<Vec<_>>(),
        vec!["beat_cloud".to_owned()],
        "the folder row was taken in and `all` does not list it"
    );
    // **And the two operations one press performs**, in the order they
    // happen: the transfer names the *file* and the load names the id the
    // file filed itself under.
    let [take, load] = taken_in_press(1, taken);
    assert!(
        matches!(&take, Operation::TransferSet { transfer: SetTransfer::Take { file } }
            if file.starts_with(&examples)),
        "the transfer does not name the file the row came off: {take:?}"
    );
    assert_eq!(
        load,
        Operation::LoadSet {
            deck: 1,
            set: "beat_cloud".to_owned()
        }
    );

    // **The bundle form, which a `presets` root never offers**, and it is
    // what a *send* writes: `setfile::bundle` is `--package`'s own reading
    // — the Set with every source it names inlined — where the `.kbset`
    // sitting in `<store>/sets/` is a projection whose material is the
    // artifacts beside it and is **not** self-contained. So this is the
    // loop the send half closes, driven with the half that exists: a
    // package written into a directory the bay can be pointed at, and
    // taken in from a row of it. It is where a send's dialog opens
    // (ADR-0311), so the pair is the ordinary one rather than a contrived
    // one.
    let second = scratch_dir("folder-take-in-bundle");
    Store::open(&second).expect("a second store");
    let sent = scratch_dir("folder-take-in-sent");
    std::fs::create_dir_all(&sent).expect("a folder to send into");
    let package = karakuri_environment::setfile::bundle(
        &Store::open(&root).expect("the store"),
        "beat_cloud",
    )
    .expect("a package to send");
    karakuri_store::ndjson::write(
        &sent.join(format!("beat_cloud{}", Store::SET_FILE_SUFFIX)),
        &package,
    )
    .expect("the package is written where the bay is pointed");
    assert_eq!(folder_listing(Some(&sent)), vec!["beat_cloud".to_owned()]);
    let taken = taking_in(&second, Taking::Folder(Some(&sent)), "beat_cloud")
        .expect("a `.kbset` row is taken in");
    assert_eq!(taken.id, "beat_cloud");
    karakuri_environment::setfile::load(
        &Store::open(&second).expect("the second store"),
        "beat_cloud",
    )
    .expect("the Set that was just taken in cannot be read back");

    // **A folder nobody has pointed anywhere holds no row**, which is the
    // same refusal a preset root that has gone gives.
    assert!(taking_in(&second, Taking::Folder(None), "beat_cloud").is_err());

    std::fs::remove_dir_all(&root).expect("clean up");
    std::fs::remove_dir_all(&second).expect("clean up");
    std::fs::remove_dir_all(&sent).expect("clean up");
}

/// A word two files in a folder wear is refused, and both names come back.
///
/// `folder_listing` draws one row per *file*, so a directory holding
/// `night.kbset` and `night.kset` draws two rows reading `night` — that is its
/// own decision and it is deliberate, because choosing between the two forms in
/// a listing would be inventing a precedence between them. What it left open
/// was which of them a press means, and the answer is that nothing here answers
/// it: a row names a word, two files wear the word, and taking one would be
/// this program choosing for an operator between two rows they cannot tell
/// apart on screen.
///
/// So the refusal carries both file names
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)),
/// which is what the next attempt needs: rename or move one of them.
///
/// A CPU test: a directory and two empty files.
#[test]
fn a_folder_row_two_files_wear_is_refused_with_both_names() {
    let dir = scratch_dir("folder-two-forms");
    std::fs::create_dir_all(&dir).expect("a folder to point at");
    std::fs::write(dir.join("night.kbset"), b"").expect("a bundle");
    std::fs::write(dir.join("night.kset"), b"").expect("an authoring file");

    assert_eq!(
        folder_listing(Some(&dir)),
        vec!["night".to_owned(), "night".to_owned()],
        "a directory of two forms of one Set draws two rows"
    );
    let refused = Taking::Folder(Some(&dir))
        .file("night")
        .expect_err("a word two files wear was resolved to one of them");
    assert!(
        refused.contains("night.kbset") && refused.contains("night.kset"),
        "the refusal does not name both files: {refused}"
    );

    std::fs::remove_dir_all(&dir).expect("clean up");
}

/// Loading a preset takes it into the store, so `all` gains a row nobody made —
/// `console.html`'s *"which is why opening a preset leaves one of your own
/// behind"*. It gains no row under `my sets`, which is ADR-0299's answer to the
/// roadmap's symptom: a preset packaged on load is a Set of the operator's and
/// is not one they chose to keep.
///
/// The whole of the press is asserted here except the aim, which is
/// [`loading`]'s and has its own test below: what a preset row adds is the
/// packaging in front of it, and the claim is that after it the id is one the
/// store holds and one `my sets` lists — which is what makes the load after it
/// the same route a `my sets` row takes rather than a second one.
///
/// A CPU test: a store is a directory, and resolving a `.kset` is a read, a
/// hash and a store put.
#[test]
fn loading_a_preset_takes_it_in_and_leaves_it_under_my_sets() {
    let root = scratch_dir("preset-take-in");
    let presets = shipped_presets();
    Store::open(&root).expect("a store to take a preset into");

    assert!(
        library(&root).is_empty(),
        "a fresh store lists something under `all`"
    );
    let taken = taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud")
        .unwrap_or_else(|e| panic!("`beat_cloud` was not taken in: {e}"));
    assert_eq!(
        taken.id, "beat_cloud",
        "the id is the file's own `set` record and not the row's word"
    );
    assert!(
        taken.said.contains("took `beat_cloud` in"),
        "the report the operator reads is `{}`",
        taken.said
    );
    assert_eq!(
        library(&root)
            .into_iter()
            .map(|set| set.id)
            .collect::<Vec<_>>(),
        vec!["beat_cloud".to_owned()],
        "the preset was taken in and `all` does not list it"
    );

    // **And the parts are in the store**, which is what makes the load
    // after this a load of material the store holds: `setfile::load` is
    // what the aim is built from and it reads them by address.
    karakuri_environment::setfile::load(&Store::open(&root).expect("the store"), "beat_cloud")
        .expect("the Set that was just taken in cannot be read back");

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A take-in names the file it read, because that is what the operation carries
/// — `SetTransfer::Take`'s own sentence, *"a path because a file is what the
/// only existing route takes"*.
///
/// The id and the file are two different answers and the row needs both: the
/// load after the press names the id, and the transfer names the file. The
/// claim here is that the file is the row's own `.kset` in the preset library
/// and not something re-derived afterwards — asking the listing a second time
/// to name what was already taken in would be two answers to *which file was
/// this* with a directory read between them.
///
/// A CPU test, for the test above's reason.
#[test]
fn a_take_in_names_the_file_it_read_because_that_is_what_the_operation_carries() {
    let root = scratch_dir("preset-take-in-file");
    let presets = shipped_presets();
    Store::open(&root).expect("a store to take a preset into");

    let taken = taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud")
        .unwrap_or_else(|e| panic!("`beat_cloud` was not taken in: {e}"));
    assert_eq!(
        taken.file.file_name().and_then(|n| n.to_str()),
        Some("beat_cloud.kset"),
        "the take-in named `{}`, which is not the row's own authoring file",
        taken.file.display()
    );
    assert!(
        taken.file.starts_with(&presets.dir),
        "the take-in named `{}`, which is outside the preset library at `{}`",
        taken.file.display(),
        presets.dir.display()
    );
    assert!(
        taken.file.is_file(),
        "the take-in named `{}` and there is no file there",
        taken.file.display()
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A press on a `presets` row is *Send a Set to somebody, and take one in*
/// performed at the second of its two moments, and then the load — so it emits
/// both, in that order.
///
/// `docs/manual/operations.html`: *"Taking one in is not a second row — opening
/// a preset is this row"*, and `console.html`: *"That is one press rather than
/// two because taking it in is what gives it the name the load needs."* Two
/// rows of the page and one press, and what this defends is that the press
/// names both of them: emitting only the load would be a press that performs
/// two rows and names one, and the row it dropped is the one nothing else in
/// this workspace constructs.
///
/// The titles are asked of [`Operation::title`] rather than written out here,
/// so a heading that moves on the page moves in one place.
///
/// A CPU test: it builds two values.
#[test]
fn a_press_on_a_preset_row_names_the_take_in_and_then_the_load() {
    let root = scratch_dir("preset-press");
    let presets = shipped_presets();
    Store::open(&root).expect("a store to take a preset into");

    let taken = taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud")
        .unwrap_or_else(|e| panic!("`beat_cloud` was not taken in: {e}"));
    let file = taken.file.clone();
    let [take, load] = taken_in_press(2, taken);

    assert_eq!(
        take,
        Operation::TransferSet {
            transfer: SetTransfer::Take { file }
        },
        "the first of the two is not the take-in, or it does not name the file it read"
    );
    assert_eq!(
        take.title(),
        "Send a Set to somebody, and take one in",
        "the first of the two does not name the row the press performed"
    );
    assert_eq!(
        load,
        Operation::LoadSet {
            deck: 2,
            set: "beat_cloud".to_owned()
        },
        "the second of the two is not the load, or it does not name the id the file filed \
         itself under"
    );
    assert_eq!(
        load.title(),
        "Load material into a deck",
        "the second of the two does not name the row the press performed"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The take-in the press names writes no record, and that is settled — which is
/// why emitting it is a naming rather than a second route into anything.
///
/// `karakuri-operation-record` answers `Silent(NoRecord)` for a transfer:
/// nothing in the session vocabulary carries a Set arriving from somewhere
/// else. So [`App::performed`] performs nothing for it, exactly as it performs
/// nothing for the scope step `space` emits, and [`unwritten`] is what an
/// operator reads. If that ever became `Owed`, the press would be emitting a
/// gap rather than a settled silence and this file would be the place to say
/// so.
///
/// A CPU test: it is a `match` on an operation.
#[test]
fn the_take_in_the_press_names_writes_no_record_and_that_is_settled() {
    let take = Operation::TransferSet {
        transfer: SetTransfer::Take {
            file: std::path::PathBuf::from("night01.kset"),
        },
    };
    assert_eq!(
        written(&take, &Current::default()),
        Written::Silent(Silent::NoRecord),
        "a transfer the press emits no longer writes a settled nothing"
    );
    let said = unwritten(&take, &written(&take, &Current::default()))
        .expect("a press that wrote no record says so");
    assert!(
        said.contains("no record, and that is settled"),
        "what the operator reads is `{said}`"
    );
}

/// An id this store already holds is refused rather than overwritten, and the
/// operator is told which of the two acts failed.
///
/// The refusal is `setfile::unbundle`'s and is not written twice — *"an id
/// already taken is refused rather than overwritten"* — so what is asserted
/// here is that the press goes through it: a second press on the same preset
/// row leaves the store exactly as the first one left it, and the sentence
/// names the id rather than the file.
///
/// A CPU test, for the test above's reason.
#[test]
fn a_preset_whose_id_this_store_holds_is_refused_rather_than_overwritten() {
    let root = scratch_dir("preset-refused");
    let presets = shipped_presets();
    Store::open(&root).expect("a store to take a preset into");

    taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud").expect("the first take-in");
    let held = library(&root);

    let refused = taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud")
        .expect_err("the same preset was taken in twice");
    assert!(
        refused.contains("already in this store") && refused.contains("beat_cloud"),
        "the refusal an operator reads is `{refused}`"
    );
    assert_eq!(
        library(&root),
        held,
        "a refused take-in changed what the store holds"
    );

    // And a row that is not in the preset library at all is refused
    // saying so, which is the other way a press finds nothing: the listing
    // is asked again on the press, so a file that has moved is met here
    // rather than inside the packaging.
    let gone = taking_in(&root, Taking::Presets(Some(&presets)), "no_such_preset")
        .expect_err("a preset that is not there was taken in");
    assert!(
        gone.contains("no_such_preset"),
        "the refusal an operator reads is `{gone}`"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A scope is a listing on this side, and stepping to one answers it — two of
/// the four with rows here, and two with nothing and a sentence saying which
/// kind of nothing it is.
///
/// The two that answer nothing are the whole point of the test: they are empty
/// for two *different* reasons — nothing in this store is starred, and this
/// console has not been pointed at a folder — and a program that said the same
/// thing about both would be hiding one of them.
///
/// `folder` is here because of the console and not because of the scope: it
/// answers with rows the moment one is dropped on the window, which is
/// `a_folder_dropped_on_the_window_points_the_bay_at_it`, and the sentence it
/// answers with here is the third of the three — a scope nobody has pointed
/// anywhere.
///
/// A CPU test: a `View` takes no device.
#[test]
fn every_scope_is_answered_and_the_two_that_answer_nothing_say_which_nothing() {
    let root = scratch_dir("scope-listing");
    let presets = shipped_presets();
    let store = Store::open(&root).expect("a store to list");
    store.write_set("night01", &[]).expect("a Set to list");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    // **`all` is the first chip and is where a row of chips starts**, so
    // this is asserted rather than marked: `select_scope` answers whether
    // the mark *moved*, and a console handed this row is already on it.
    assert_eq!(view.scope(), Some(Scope::AllSets));

    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(view.library, vec!["night01".to_owned()]);
    assert!(said.contains("all") && said.contains('1'), "{said}");

    // **And `my sets` is that listing starred, which is nothing yet**
    // (ADR-0299): the store holds a Set and the operator has not chosen
    // it, so the subset is empty for an answer rather than for an absence.
    assert!(view.select_scope(Scope::MySets));
    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        view.library.is_empty(),
        "`my sets` listed {:?} and nothing has been starred",
        view.library
    );
    assert!(said.contains("my sets"), "{said}");

    // **A star put on it puts the row there**, which is the whole of what
    // the subset is: the same store, the same listing, one file beside it.
    assert!(
        favourite(
            &root,
            Asked::Operator,
            &Operation::SetFavourite {
                id: "night01".to_owned(),
                favourite: true,
            },
        )
        .is_some(),
        "`favourite` answered nothing for a `SetFavourite`"
    );
    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(view.library, vec!["night01".to_owned()], "{said}");
    assert!(
        view.starred.contains("night01"),
        "the bay was not told which rows are starred: {:?}",
        view.starred
    );
    // **And taking it off takes the row away again**, which is the state
    // this test goes on to assert the empty sentence of.
    favourite(
        &root,
        Asked::Operator,
        &Operation::SetFavourite {
            id: "night01".to_owned(),
            favourite: false,
        },
    )
    .expect("`favourite` answered nothing for a `SetFavourite`");

    assert!(view.select_scope(Scope::Presets));
    assert_eq!(view.scope(), Some(Scope::Presets));
    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        view.library.iter().any(|id| id == "beat_cloud"),
        "the `presets` scope lists {:?}",
        view.library
    );
    assert!(said.contains("presets"), "{said}");

    // The two that answer nothing here, and the sentences they answer
    // with are not one sentence. **`folder` is in this list because this
    // console has been pointed nowhere**, and not because the scope cannot
    // be answered: point it at a directory and it lists what is in it,
    // which is `a_folder_dropped_on_the_window_points_the_bay_at_it`.
    for scope in [Scope::MySets, Scope::Folder] {
        assert!(view.select_scope(scope));
        let said = listing(&mut view, &root, Some(&presets), None, None);
        assert!(
            view.library.is_empty(),
            "`{}` listed {:?}, and nothing in this run put a row there",
            scope.name(),
            view.library
        );
        assert!(
            said.contains(scope.name()) && said.contains(why_nothing(scope, false, false)),
            "`{}` lists nothing and says `{said}`",
            scope.name()
        );
    }
    assert_ne!(
        why_nothing(Scope::MySets, false, false),
        why_nothing(Scope::Folder, false, false),
        "the two scopes that answer nothing are empty for two different reasons and this \
         program gives one sentence for both"
    );
    // **It says what to press**, which is the difference between a scope
    // that is empty and a scope that is broken: `my sets` is the starred
    // subset, so the way to fill it is a star and the sentence names one.
    assert!(
        why_nothing(Scope::MySets, false, false).contains("star"),
        "the `my sets` sentence does not say what fills it: {}",
        why_nothing(Scope::MySets, false, false)
    );
    assert_ne!(
        why_nothing(Scope::AllSets, false, false),
        why_nothing(Scope::MySets, false, false),
        "a store nobody has saved into and a store nobody has starred in are given one \
         sentence"
    );
    // **It says how a directory is chosen, and it used to say `operation`.**
    // This asserted that word until ADR-0275, on the reading that the chip
    // waited on one — which `ListSets`' own shape refutes: both its fields
    // narrow what a store already holds, and which store is asked at all
    // never was the operation's. What the sentence owes now is the way in.
    assert!(
        why_nothing(Scope::Folder, false, false).contains("drag")
            && why_nothing(Scope::Folder, false, false).contains("dropped"),
        "the `folder` sentence does not say how a directory is chosen: {}",
        why_nothing(Scope::Folder, false, false)
    );
    // **And a folder that *has* been pointed somewhere is a third kind of
    // nothing**, which is the sentence that arrived with the drop: an
    // empty scope for want of a gesture and one for want of a Set file in
    // the directory are the same drawing and not the same fact.
    assert_ne!(
        why_nothing(Scope::Folder, false, false),
        why_nothing(Scope::Folder, true, false),
        "a folder nobody has pointed anywhere and a folder holding no Set are given one \
         sentence"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A folder let go on this window points the bay at it, marks the chip and
/// lists what is in it — all three on the pass the drop arrives on.
///
/// The three are one act by ADR-0275's third policy: `dropped_files` is visible
/// for one pass and then gone, so a bay that had set the directory and waited
/// for the chip to be pressed would be holding a path nothing will hand it
/// again. It is asserted as three outcomes of one call for that reason.
///
/// And what the listing holds is Sets and not parts, which is the `presets`
/// scope's rule one chip along: both spellings of a Set file are rows, a `.kir`
/// is not, and a subdirectory named like a Set file belongs to whoever made it.
///
/// A CPU test: a directory and a `View`.
#[test]
fn a_folder_dropped_on_the_window_points_the_bay_at_it() {
    let root = scratch_dir("folder-drop");
    let store = root.join("store");
    let handed = root.join("from-somebody");
    std::fs::create_dir_all(&handed).expect("a folder to drop");
    for name in ["night01.kbset", "sketch.kset", "adrift.kbset"] {
        std::fs::write(handed.join(name), "").expect("a Set file");
    }
    // What a folder scope must not list: a part, and a directory wearing a
    // Set file's name.
    std::fs::write(handed.join("blur.kir"), "").expect("a part");
    std::fs::create_dir_all(handed.join("unpacked.kbset")).expect("a directory");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert!(view.select_scope(Scope::MySets));
    let mut folder: Option<std::path::PathBuf> = None;

    let said = folder_dropped(&mut view, &mut folder, &store, None, &[handed.as_path()])
        .expect("a drop of one folder was answered");

    assert_eq!(folder.as_deref(), Some(handed.as_path()));
    assert_eq!(
        view.folder.as_deref(),
        Some(handed.display().to_string().as_str()),
        "the panel was not handed the line to draw in the `.path` row"
    );
    assert_eq!(
        view.scope(),
        Some(Scope::Folder),
        "the drop set the directory and left another chip marked"
    );
    assert_eq!(
        view.library,
        vec![
            "adrift".to_owned(),
            "night01".to_owned(),
            "sketch".to_owned()
        ],
        "the folder scope lists {:?}",
        view.library
    );
    assert!(
        said.contains(&handed.display().to_string())
            && said.contains("the `folder` chip is marked"),
        "the drop said `{said}`"
    );

    // **And the row is not drawn as a hover**: the release is what was
    // read, so nothing is on its way in afterwards.
    assert_eq!(view.pointed().map(|at| at.incoming), Some(false));

    // A second drop re-points it, which is the whole of *re-pointing the
    // bay is another drop*.
    let empty = root.join("empty");
    std::fs::create_dir_all(&empty).expect("a second folder");
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[empty.as_path()])
        .expect("a second drop was answered");
    assert_eq!(folder.as_deref(), Some(empty.as_path()));
    assert!(view.library.is_empty(), "{:?}", view.library);
    assert!(
        said.contains(why_nothing(Scope::Folder, true, false)),
        "a folder holding no Set said `{said}`"
    );
    // **The chip is marked on this one too**, and the sentence says so:
    // the second drop moved the listing without moving the mark, which is
    // the one case a *whether it moved* answer would have got backwards.
    assert!(
        said.contains("the `folder` chip is marked"),
        "a second drop said `{said}`"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The two refusals ADR-0275 writes, and what they leave behind.
///
/// - One path, and it has to be a directory. A file is refused naming
///   what was dropped rather than read as the folder it sits in, which
///   would point this bay at a directory nobody pointed at; a `.kbset` is
///   refused with the press that *does* take a Set in, because a file
///   landing on this window has no row under it.
/// - More than one path is refused, and all of them are, counting what
///   arrived: a multi-item drag is one pass with several entries, so there
///   is no first to act on and nothing says which was aimed at.
///
/// Every one of them says where the library is still pointed, which is
/// the half that makes a refusal readable at a glance (P-0083): a bay that
/// went on listing what it listed and a bay that quietly moved are the
/// same drawing.
///
/// A CPU test: a directory and a `View`.
#[test]
fn a_drop_that_is_not_one_folder_is_refused_and_the_bay_keeps_what_it_had() {
    let root = scratch_dir("folder-refusal");
    let store = root.join("store");
    let handed = root.join("from-somebody");
    std::fs::create_dir_all(&handed).expect("a folder to drop");
    std::fs::write(handed.join("night01.kbset"), "").expect("a Set file");
    let second = root.join("another");
    std::fs::create_dir_all(&second).expect("a second folder");
    let loose = root.join("notes.txt");
    std::fs::write(&loose, "").expect("a file to drop");
    let set = root.join("handover.kbset");
    std::fs::write(&set, "").expect("a Set file to drop");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    let mut folder: Option<std::path::PathBuf> = None;
    folder_dropped(&mut view, &mut folder, &store, None, &[handed.as_path()])
        .expect("the folder this bay is pointed at");
    let listed = view.library.clone();

    // A file, and it is not read as the directory it sits in.
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[loose.as_path()])
        .expect("a file was answered");
    assert!(
        said.contains("notes.txt") && said.contains("folder"),
        "the refusal an operator reads is `{said}`"
    );
    // **And it says where the bay is still pointed**, which is the half
    // that makes the refusal readable rather than the operator having to
    // look at the row to find out whether anything moved.
    assert!(
        said.contains(&format!("still pointed at `{}`", handed.display())),
        "the refusal does not say where the library is still pointed: `{said}`"
    );

    // A Set file, and the refusal carries the press that takes one in.
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[set.as_path()])
        .expect("a Set file was answered");
    assert!(
        said.contains("handover.kbset") && said.contains("row"),
        "a `.kbset` let go on the window said `{said}`"
    );

    // Two folders at once, and neither of them is taken.
    let said = folder_dropped(
        &mut view,
        &mut folder,
        &store,
        None,
        &[handed.as_path(), second.as_path()],
    )
    .expect("two paths were answered");
    assert!(
        said.contains('2') && said.contains("none of them"),
        "two folders at once said `{said}`"
    );

    // A path that is not there at all, which is a third thing and is said
    // as one: whether it is a folder was never learned.
    let gone = root.join("no-such-folder");
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[gone.as_path()])
        .expect("a path that is not there was answered");
    assert!(
        said.contains("no-such-folder") && said.contains("could not be examined"),
        "a path that is not there said `{said}`"
    );

    // **And after all four the bay is where it was**, which every one of
    // them said it would be.
    assert_eq!(folder.as_deref(), Some(handed.as_path()));
    assert_eq!(view.library, listed);
    assert_eq!(view.scope(), Some(Scope::Folder));

    // Nothing dropped is not a refusal and is not an answer.
    assert_eq!(
        folder_dropped(&mut view, &mut folder, &store, None, &[]),
        None
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A folder over the window reads in the `.path` row, and two read as none.
///
/// The hover is the one thing about this bay that a frame does — `egui` clones
/// the hovered files onto every pass while a drag lasts — so this is the whole
/// of what a pass owes it: the path where there is one to draw, nothing where a
/// release would set nothing, and no allocation where neither has changed.
///
/// A CPU test: a `View` and a list of paths.
#[test]
fn a_folder_over_the_window_reads_in_the_path_row_and_two_read_as_none() {
    let one = karakuri_console::egui::HoveredFile {
        path: Some(std::path::PathBuf::from("/Volumes/stick/handover")),
        mime: String::new(),
    };
    let two = karakuri_console::egui::HoveredFile {
        path: Some(std::path::PathBuf::from("/Volumes/stick/another")),
        mime: String::new(),
    };

    let mut view = View::new(Room::Day);
    folder_over(&mut view, std::slice::from_ref(&one));
    assert_eq!(view.incoming.as_deref(), Some("/Volumes/stick/handover"));
    assert_eq!(view.pointed().map(|at| at.incoming), Some(true));

    // **Two at once say nothing**, because there is no path a release
    // would set — and picking the first would be the choice the refusal
    // above exists to refuse.
    folder_over(&mut view, &[one.clone(), two]);
    assert_eq!(view.incoming, None);

    // Out of the window again, and what was chosen is what is drawn.
    view.folder = Some("/Users/somebody/sets".to_owned());
    folder_over(&mut view, std::slice::from_ref(&one));
    assert_eq!(
        view.pointed().map(|at| at.path),
        Some("/Volumes/stick/handover")
    );
    folder_over(&mut view, &[]);
    assert_eq!(
        view.pointed(),
        Some(karakuri_console::view::Pointed {
            path: "/Users/somebody/sets",
            incoming: false,
        })
    );
}

/// A load writes the Set's procedures where the slot's watcher is looking and
/// aims it there, and it touches no deck at all.
///
/// This is the whole of what `Operation::LoadSet` needed, and what it is *not*
/// is the claim: `Deck::install` is the one function that puts a built Set in a
/// slot and is documented as deliberately unreachable from a key or a surface,
/// because *"a live run changes its material by editing a file and letting the
/// worker build it, which is what the budget watchdog is attached to"*. So this
/// asserts files and an aim. A load that built a Set here would be a picture
/// nothing measured, in a slot the watchdog never got to judge.
///
/// Three things beyond *it happened*, and each is a wrong load that looks
/// right. The scratch name carries the deck letter and the node's place,
/// because `scratch::place` overwrites by name and two decks loading Sets whose
/// procedures share one would silently become one file — the second load moving
/// the first deck on its watcher's next poll. The aim restates the layering,
/// the fold, the capacity and the salts the *file* recorded rather than the
/// ones the slot was running at, because that is the failure every `Watch`
/// field is documented against and it does not show on the load: it shows on
/// the first save afterwards. And the node names are the file's, because an
/// `edge` resolves against them.
///
/// A CPU test: a store is a directory, and nothing here takes a device.
#[test]
fn a_load_writes_the_sets_procedures_into_the_scratch_and_aims_the_slot_there() {
    use karakuri_environment::setfile;
    use karakuri_store::Hash;

    let root = std::env::temp_dir().join(format!(
        "karakuri-load-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = Store::open(&root).expect("a store to load from");

    // Two real procedures, so the load goes through the checker the way a
    // Set out of the library does. `lattice_shell` is chosen for its name:
    // it is what the scratch file has to be called after.
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let sources: Vec<(karakuri_store::Layer, String)> = [
        (karakuri_store::Layer::L1, "lattice_shell.kir"),
        (karakuri_store::Layer::L4, "soft_points.kir"),
    ]
    .into_iter()
    .map(|(layer, file)| {
        let src = std::fs::read_to_string(examples.join(file)).expect("an example");
        (layer, src)
    })
    .collect();
    let nodes: Vec<setfile::Node> = sources
        .iter()
        .map(|(layer, src)| setfile::Node {
            hash: {
                let hash = Hash::of(src.as_bytes());
                store.put_artifact(src.as_bytes()).expect("store a source");
                hash
            },
            layer: match layer {
                karakuri_store::Layer::L1 => karakuri_ir::Kind::L1,
                _ => karakuri_ir::Kind::L4,
            },
            index: 0,
            // A name the file wrote, which is what a rebuild has to call
            // the node — not the procedure's own.
            name: Some(match layer {
                karakuri_store::Layer::L1 => "grid".to_owned(),
                _ => "draw".to_owned(),
            }),
        })
        .collect();
    // Values no default here produces, so an aim that kept the slot's own
    // cannot pass by accident.
    let seeds = [0x0bad_cafeu32];
    setfile::save(
        &store,
        Asked::Operator,
        "night01",
        setfile::Saving {
            nodes: &nodes,
            capacities: &[2048],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &karakuri_engine::camera::Orbit::default(),
            layering: Layering::Composite,
            live: Some(0),
            seeds: &seeds,
        },
    )
    .expect("write the Set file");

    // Deck B, so the letter in the scratch name is not the first one and a
    // hard-coded `A` fails here.
    let (tx, rx) = std::sync::mpsc::channel();
    // **An [`Aiming`] and not a bare sender**, because a load keeps where it
    // pointed the watcher — see the assertion at the end of this test.
    let mut aiming = Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named::bare("nowhere.kir"),
            rest: Vec::new(),
            layering: Layering::Overdraw,
            live: None,
            capacity: None,
            seed_salt: 0,
            salts: Vec::new(),
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            // **Running material no Set names**, which is where every slot
            // of this program starts and what the launch seed files under.
            set: None,
        },
        // What this test asserts is the *aim*; where the layout is
        // published is `a_load_moves_what_the_mcp_server_resolves_against`.
        karakuri_mcp::Slots::unpointed(),
        ASKED_TO_PRIME,
    );
    let line = loading(
        &root,
        ASKED_TO_PRIME,
        slot_salt(ASKED_TO_PRIME),
        &mut aiming,
        "night01",
    )
    .unwrap_or_else(|e| panic!("the load failed: {e}"));
    assert!(
        line.contains("deck B"),
        "the line does not say where: {line}"
    );

    let aim = rx.try_recv().expect("the slot was aimed at something");
    assert_eq!(aim.rest.len(), 1, "the renderer did not travel with it");
    assert_eq!(
        aim.head.name.as_deref(),
        Some("grid"),
        "the head is not called what the file called it, so an edge would not resolve"
    );
    for (named, (_, src)) in std::iter::once(&aim.head)
        .chain(aim.rest.iter())
        .zip(&sources)
    {
        let name = named
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a file name");
        assert!(
            name.starts_with("B0-") || name.starts_with("B1-"),
            "`{name}` carries neither the deck nor the node, so two decks would share it"
        );
        assert_eq!(
            std::fs::read_to_string(&named.path).expect("the scratch file"),
            *src,
            "the watcher is pointed at a file that is not the Set's source"
        );
    }

    assert_eq!(aim.layering, Layering::Composite, "the file's layering");
    assert_eq!(aim.live, Some(0), "the file's fold");
    assert_eq!(aim.capacity, Some(2048), "the file's capacity");
    assert_eq!(aim.seed_salt, seeds[0], "the file's seed");
    assert_eq!(aim.salts, vec![seeds[0]], "the file's salts");
    assert!(
        aim.authorities.is_empty(),
        "a Set file carries no grant, so a load must hand none over"
    );
    // **The id the versions after this load are filed under.** The slot was
    // running material no Set names, and it is running `night01` now; an
    // aim that left this at `None` would go on writing this Set's edits
    // into the history under no Set at all, which is a chain that answers
    // *what versions has `night01` had* with nothing (ADR-0276).
    assert_eq!(
        aim.set.as_deref(),
        Some("night01"),
        "the load did not tell the watcher which Set the slot is running"
    );

    // **And the load kept where it pointed the watcher**, which is what a
    // later rewiring restates the other twelve fields from: an `Aiming` that
    // sent an aim and left `at` behind would re-aim this slot at the pair
    // the run launched with. See [`Aiming`].
    assert_eq!(
        aiming.at.head.name.as_deref(),
        Some("grid"),
        "the load sent an aim and did not keep it"
    );
    assert_eq!(aiming.at.live, Some(0), "the kept aim is not the sent one");
    assert_eq!(
        aiming.at.set.as_deref(),
        Some("night01"),
        "the kept aim does not carry the Set, so the next rewiring would restate none"
    );

    // And a Set that is not there is a sentence with nothing sent: the
    // deck goes on playing what it was.
    let e = loading(&root, ON_AIR, slot_salt(ON_AIR), &mut aiming, "nothing01")
        .expect_err("a Set that is not in the store");
    assert!(e.contains("nothing01"), "the refusal does not name it: {e}");
    assert!(
        rx.try_recv().is_err(),
        "a load that failed aimed the slot anyway"
    );
    assert_eq!(
        aiming.at.live,
        Some(0),
        "a load that failed moved where the watcher is pointed"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A load moves what the MCP server resolves an address against, and it moves
/// nothing else's.
///
/// This program built the `mcp::Slots` it handed the server out of the launch
/// working copies and never wrote it again. A library load writes new scratch
/// files and re-points that slot's watcher at them (ADR-0228), so from the
/// first load onwards every address the server resolved was the layout the deck
/// had stopped running: `read_procedure` answered about the wrong material,
/// `write_procedure` wrote a file no watcher was polling and reported that it
/// was being built, and a node the loaded Set does hold was refused for not
/// existing. None of the three fails — they are plausible wrong answers on the
/// surface whose reader is a program in a loop (`docs/principles/0094-…`).
/// ADR-0308 recorded it and worked around it for the landing alone.
///
/// The assertion is `Slots::file`, which is the walk the server writes through:
/// `write_procedure` and `read_procedure` resolve an address with
/// `Slots::path`, and `file` is that same private walk with the layer taken as
/// a word. So the path asserted here is the path the server would write to.
///
/// Three things. The loaded slot resolves to the new scratch file and not to
/// the launch copy; a renderer the launch pair had and the loaded Set has not
/// is refused naming what the slot holds now (P-0083); and the slot nobody
/// loaded onto has not moved, because a publication per slot that overwrote the
/// deck would be a worse defect than the one being fixed.
///
/// A CPU test: a store and a scratch are directories, and nothing here takes a
/// device.
#[test]
fn a_load_moves_what_the_mcp_server_resolves_against() {
    use karakuri_environment::setfile;
    use karakuri_store::Hash;

    let root = scratch_dir("mcp-load");
    let store = Store::open(&root).expect("a store to load from");

    // A Set of two nodes, through the checker exactly as a load out of the
    // library goes. `soft_points` is the renderer's name, which is what the
    // scratch file is called after.
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let nodes: Vec<setfile::Node> = [
        (karakuri_ir::Kind::L1, "lattice_shell.kir"),
        (karakuri_ir::Kind::L4, "soft_points.kir"),
    ]
    .into_iter()
    .map(|(layer, file)| {
        let src = std::fs::read_to_string(examples.join(file)).expect("an example");
        let hash = Hash::of(src.as_bytes());
        store.put_artifact(src.as_bytes()).expect("store a source");
        setfile::Node {
            hash,
            layer,
            index: 0,
            name: None,
        }
    })
    .collect();
    setfile::save(
        &store,
        Asked::Operator,
        "night02",
        setfile::Saving {
            nodes: &nodes,
            capacities: &[2048],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &karakuri_engine::camera::Orbit::default(),
            layering: Layering::Overdraw,
            live: None,
            seeds: &[0x0bad_cafe],
        },
    )
    .expect("write the Set file");

    // The launch layout: two decks, and the one about to be loaded onto
    // holds **two** renderers, so `L4:1` is a real address before the press
    // and the refusal asserted below is a change rather than a constant.
    let launch = |name: &str, kind: &str| {
        let path = root.join(name);
        std::fs::write(&path, format!("proc launched {{\n  kind {kind}\n}}\n"))
            .expect("a launch copy");
        path
    };
    let a_l1 = launch("A0-launch.kir", "L1");
    let a_l4 = launch("A1-launch.kir", "L4");
    let b_l1 = launch("B0-launch.kir", "L1");
    let b_l4 = launch("B1-launch.kir", "L4");
    let b_l4_second = launch("B2-launch.kir", "L4");
    let pointing = karakuri_mcp::Slots::of(vec![
        (a_l1.clone(), vec![a_l4.clone()]),
        (b_l1, vec![b_l4.clone(), b_l4_second.clone()]),
    ]);
    assert_eq!(
        pointing
            .file(ASKED_TO_PRIME, "L4", 0)
            .expect("the launch L4"),
        b_l4,
        "the fixture does not start on the launch copies"
    );

    // The slot's watcher, pointed where the run launched it. `Aiming::new`
    // publishes that, which is what a window remade does too.
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut aiming = Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named::bare(&b_l4),
            rest: Vec::new(),
            layering: Layering::Overdraw,
            live: None,
            capacity: None,
            seed_salt: 0,
            salts: Vec::new(),
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            set: None,
        },
        pointing.clone(),
        ASKED_TO_PRIME,
    );

    loading(
        &root,
        ASKED_TO_PRIME,
        slot_salt(ASKED_TO_PRIME),
        &mut aiming,
        "night02",
    )
    .unwrap_or_else(|e| panic!("the load failed: {e}"));

    // **The new scratch file, and not the launch copy.**
    let landed = pointing
        .file(ASKED_TO_PRIME, "L4", 0)
        .expect("the loaded Set's renderer");
    assert_eq!(
        landed,
        root.join(karakuri_environment::scratch::DIR)
            .join("B1-soft_points.kir"),
        "a write addressed to deck B's renderer would not reach the file its \
         watcher is polling"
    );
    assert_ne!(
        landed, b_l4,
        "the address resolved against the layout the deck stopped running"
    );
    assert_eq!(
        pointing
            .file(ASKED_TO_PRIME, "L1", 0)
            .expect("the loaded Set's geometry"),
        root.join(karakuri_environment::scratch::DIR)
            .join("B0-lattice_shell.kir")
    );

    // **The second renderer is gone, and the refusal says what is there
    // now** rather than reporting a range the deck stopped holding.
    let refused = pointing
        .file(ASKED_TO_PRIME, "L4", 1)
        .expect_err("`night02` holds one renderer");
    assert_eq!(
        refused, "slot 1 holds one L4 and `index` is 1",
        "the refusal does not name what the deck holds now"
    );

    // **And nothing else moved.** A publication that wrote the deck rather
    // than the slot would be a worse defect than the one this fixes.
    assert_eq!(
        pointing.file(ON_AIR, "L4", 0).expect("deck A is untouched"),
        a_l4,
        "the load moved a deck nobody named"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The requirement this program was failing: the same preset loaded into every
/// slot, and each deck watching its own separate copy in its own place.
///
/// Every slot used to be handed [`Sources`] itself — the two paths the operator
/// typed — so four watchers polled two files. One save rebuilt four slots, and
/// since a parked slot's trial never reaches a verdict, three of the four rows
/// it put in the Staging lane stayed there for the rest of the run. That
/// symptom is this defect's, not the lane's.
///
/// Four things, and the third is the one the requirement is about. The copies
/// are under the scratch and not where the operator pointed; the four decks
/// hold eight distinct files rather than two shared ones; an edit made through
/// deck B's L1 moves deck B and no other deck; and the file the operator named
/// is not written to at all.
///
/// The version every deck starts on is in the history before the window opens,
/// and it is filed under no Set.
///
/// Two claims, and the second is the one that is a decision. That there is a
/// seed at all is ADR-0089's — a first edit whose predecessor was never written
/// down cannot be walked back — and this program had no history at all until it
/// was given one, so a run's whole night of edits was kept nowhere. That every
/// row reads `None` is ADR-0276's: the material is a pair somebody typed, and
/// filing it under `Sources::material` would put rows under a Set no listing
/// can ever match.
///
/// Every node of every slot, and the count is derived from the copies rather
/// than written here — a slot is an L1 and a renderer, and each deck runs from
/// its own pair, so a seed that filed one deck or one node would leave the
/// others' first edits with nothing behind them.
///
/// A CPU test: a store and a scratch are directories, and nothing here takes a
/// device.
#[test]
fn the_launch_versions_are_filed_before_a_window() {
    let root = scratch_dir("seeded");
    let named = shipped();
    let (_, running) =
        working_copies(&root, &named, SLOTS).unwrap_or_else(|e| panic!("no copies: {e}"));

    let _shared = seeded(&root, &running);

    let listing = karakuri_environment::history::list(&root, 64).expect("the history lists");
    let nodes: usize = running.len() * 2;
    assert_eq!(
        listing.versions.len(),
        nodes,
        "a deck's starting version is missing, so its first edit has nothing to be \
         walked back to: {:?}",
        listing.versions
    );
    assert!(
        listing.versions.iter().all(|v| v.set.is_none()),
        "a slot launched on a typed pair filed its version under a Set: {:?}",
        listing.versions
    );
    for slot in 0..running.len() {
        let of_slot: Vec<&karakuri_environment::history::Version> =
            listing.versions.iter().filter(|v| v.slot == slot).collect();
        assert_eq!(
            of_slot.len(),
            2,
            "deck {} filed {} of its two nodes",
            deck_letter(slot as u8),
            of_slot.len()
        );
        assert!(
            of_slot.iter().any(|v| v.layer == "L1") && of_slot.iter().any(|v| v.layer == "L4"),
            "deck {}'s two versions are not its geometry and its renderer: {of_slot:?}",
            deck_letter(slot as u8)
        );
    }

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A CPU test: a scratch is a directory and nothing here takes a device.
#[test]
fn every_deck_runs_from_its_own_copy_and_an_edit_moves_one_deck() {
    let root = scratch_dir("own-copy");
    let named = shipped();
    let (dir, running) =
        working_copies(&root, &named, SLOTS).unwrap_or_else(|e| panic!("no copies: {e}"));

    assert_eq!(
        running.len(),
        SLOTS,
        "a deck of {SLOTS} slots got {running:?}"
    );
    assert_eq!(dir, root.join(karakuri_environment::scratch::DIR));

    // 1. Nothing a deck holds points at what the operator typed.
    for (slot, pair) in running.iter().enumerate() {
        for path in [&pair.l1, &pair.l4] {
            assert!(
                path.starts_with(&dir),
                "deck {} still runs from {}",
                deck_letter(slot as u8),
                path.display()
            );
        }
    }

    // 2. Eight files and not two, and each says which deck it belongs to.
    let mut every: Vec<&std::path::PathBuf> = running.iter().flat_map(|p| [&p.l1, &p.l4]).collect();
    let held = every.len();
    every.sort();
    every.dedup();
    assert_eq!(
        every.len(),
        held,
        "{SLOTS} decks on one pair share a file, so an edit cannot reach one of them"
    );
    for (slot, pair) in running.iter().enumerate() {
        let letter = deck_letter(slot as u8);
        for (at, path) in [&pair.l1, &pair.l4].into_iter().enumerate() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a file name");
            assert!(
                name.starts_with(&format!("{letter}{at}-")),
                "`{name}` carries neither the deck nor the node's place"
            );
        }
    }

    // 3. **The claim.** An edit in one place moves one deck.
    let before = std::fs::read_to_string(&running[ON_AIR].l1).expect("deck A's L1");
    std::fs::write(&running[ASKED_TO_PRIME].l1, "deck B only").expect("edit deck B");
    assert_eq!(
        std::fs::read_to_string(&running[ASKED_TO_PRIME].l1).expect("read"),
        "deck B only"
    );
    for (slot, deck) in running.iter().enumerate().take(SLOTS) {
        if slot == ASKED_TO_PRIME {
            continue;
        }
        assert_eq!(
            std::fs::read_to_string(&deck.l1).expect("read"),
            before,
            "editing deck B moved deck {} as well",
            deck_letter(slot as u8)
        );
    }

    // 4. And the preset is what it was, which is the whole reason the
    // scratch exists: three shipped presets were replaced in one session.
    assert_eq!(
        std::fs::read_to_string(&named.l1).expect("the preset"),
        before,
        "the file the operator named was written to"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The program has to say it. Four decks on one preset are four files whose
/// names an operator cannot guess and cannot tell apart by content — at startup
/// they hold the same bytes — so the startup print names the directory and then
/// one file pair per deck.
#[test]
fn the_startup_print_names_one_file_per_deck() {
    let root = scratch_dir("own-copy-said");
    let (dir, running) = working_copies(&root, &shipped(), SLOTS).expect("copies");
    let said = running_from(&dir, &running);

    assert!(
        said.contains(&dir.display().to_string()),
        "the print does not say where: {said}"
    );
    for (slot, pair) in running.iter().enumerate() {
        let letter = deck_letter(slot as u8);
        assert!(
            said.contains(&format!("deck {letter}:")),
            "deck {letter} is not in the print: {said}"
        );
        for path in [&pair.l1, &pair.l4] {
            let name = path.file_name().and_then(|n| n.to_str()).expect("a name");
            assert!(
                said.contains(name),
                "`{name}` is a file the deck runs from and the print does not name it: \
                 {said}"
            );
        }
    }
    // **And the four lines are four different answers.** A print that
    // named the same two files under all four decks would be a print an
    // operator cannot act on — which is exactly what this program said
    // while every deck watched the pair that was typed.
    let lines: Vec<&str> = said
        .lines()
        .filter(|line| line.trim_start().starts_with("deck "))
        .collect();
    assert_eq!(lines.len(), SLOTS, "one line per deck, and got {lines:?}");
    let named: Vec<&str> = lines
        .iter()
        .map(|line| line.split_once(':').expect("`deck A: files`").1)
        .collect();
    let mut distinct = named.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        named.len(),
        "two decks were told to open the same file: {named:?}"
    );

    // And it says the thing an operator will otherwise read as a bug.
    assert!(
        said.contains("not written to"),
        "the print does not say the named paths are left alone: {said}"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A root of this test's own, cleaned of whatever a previous run left.
fn arrangement_root(what: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "karakuri-arrangement-{what}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// A panel with something folded and something soloed — the two things an
/// arrangement is kept for, and the two a reset forgets.
fn arranged(width: f32, height: f32) -> Panel {
    let mut panel = Panel::new(width, height);
    let staging = panel.layout().find("staging").expect("a staging bay");
    panel.op(Op::Fold(staging));
    let mixer = panel.layout().find("mixer").expect("a mixer bay");
    panel.op(Op::Solo(mixer));
    panel.solve();
    panel
}

/// The bytes are at the path `karakuri-store`'s own header claims, read off
/// that path and not through the store that wrote them.
///
/// This is the assertion ADR-0221 §4 says the store's suite had to spell out
/// rather than leave to a round trip: *"a format test is not a location test"*,
/// because a defect that files the arrangement in the wrong directory entirely
/// is invisible to a test that writes and reads through the same wrong path.
/// The same hole is open one layer up — this file chooses the name it hands
/// over — so the same assertion is made here, about
/// `arrangements/<name>.arrangement.json` under the store's root.
#[test]
fn an_arrangement_is_kept_at_the_path_the_stores_header_names() {
    let root = arrangement_root("kept");
    let mut panel = arranged(1280.0, 720.0);

    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::SaveArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a save is one of the two operations this route answers for");

    let at = root.join("arrangements").join("four_deck.arrangement.json");
    let bytes = std::fs::read(&at).unwrap_or_else(|e| {
        panic!(
            "nothing at {} after `{said}` — a saved arrangement is one path component of \
             name, one of what it is, and one of the format it is in, under the store's \
             fourth directory: {e}",
            at.display()
        )
    });

    // And what is at that path is this panel's arrangement rather than
    // some other document that happens to be there.
    let back: Layout =
        serde_json::from_slice(&bytes).expect("the bytes at that path are an arrangement");
    assert!(
        back.is_soloed(),
        "the file at {} did not carry the solo the panel was saved with",
        at.display()
    );

    // Nothing else was created under the store: an arrangement is a fourth
    // thing beside `sets/`, `sessions/` and the artifacts, and not one of
    // them.
    assert!(
        !root.join("sets").join("four_deck.kbset").exists(),
        "the arrangement was filed as a Set"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// What comes back is the arrangement that was kept, in the window it arrives
/// in — which is `Op::Reset` carrying the viewport across, with the arrangement
/// handed in rather than built.
///
/// The two viewports differ in both axes on purpose: an arrangement carries the
/// viewport it was saved at, so a restore that took the file's would open a
/// console arranged on a desktop inside a smaller window with every rectangle
/// past the edge.
#[test]
fn an_arrangement_put_back_arrives_in_the_window_this_one_already_has() {
    let root = arrangement_root("back");
    let mut saved = arranged(1920.0, 1080.0);
    arrangement(
        &root,
        &mut saved,
        &mut view::Arrangement::default(),
        &Operation::SaveArrangement {
            name: "night_b".to_owned(),
        },
    )
    .expect("a save");

    // A window of a different size, with nothing folded and nothing soloed.
    let mut window = Panel::new(1280.0, 720.0);
    let staging = window.layout().find("staging").expect("a staging bay");
    assert!(!window.layout().is_collapsed(staging));

    let said = arrangement(
        &root,
        &mut window,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "night_b".to_owned(),
        },
    )
    .expect("a restore");

    let now = window.layout();
    assert_eq!(
        (now.viewport().w, now.viewport().h),
        (1280.0, 720.0),
        "`{said}` — the arrangement brought the window it was saved at with it. The window \
         is the operator's and never the file's"
    );
    assert!(
        now.is_soloed() && now.is_collapsed(now.find("staging").expect("staging")),
        "`{said}` — the fold and the solo did not come back, so what was put back is not \
         what was kept"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A name nothing is filed under is said back, and the console does not move.
///
/// The refusal that matters most in this family: `read_arrangement` never falls
/// back to the built-in, so an operator who mistyped a name is told the name
/// rather than watching their console reset (ADR-0221 §2). Asked three ways,
/// because the three failures send an operator to three different places — no
/// store at all, no such name, and a file that will not read back.
#[test]
fn a_name_nothing_is_filed_under_is_said_back_and_nothing_resets() {
    let root = arrangement_root("refused");
    let mut panel = arranged(1280.0, 720.0);
    let kept: Vec<bool> = panel
        .nodes()
        .iter()
        .map(|n| panel.layout().is_collapsed(n.id))
        .collect();
    let unchanged = |panel: &Panel, said: &str| {
        let now: Vec<bool> = panel
            .nodes()
            .iter()
            .map(|n| panel.layout().is_collapsed(n.id))
            .collect();
        assert_eq!(
            now, kept,
            "`{said}` and the arrangement moved — a refusal that resets the console is the \
             one thing `read_arrangement` promises never to do"
        );
        assert!(
            panel.layout().is_soloed(),
            "`{said}` and the solo went — see above"
        );
    };

    // 1. No store at all, and asking a question does not make one.
    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a restore");
    assert!(
        said.contains("four_deck"),
        "the refusal did not say the name back: {said}"
    );
    assert!(
        !root.exists(),
        "putting an arrangement back that is not there created a store at {}",
        root.display()
    );
    unchanged(&panel, &said);

    // 2. A store, and no such name in it.
    Store::open(&root).expect("a store");
    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a restore");
    assert!(
        said.contains("four_deck"),
        "the refusal did not say the name back: {said}"
    );
    unchanged(&panel, &said);

    // 3. A file filed under the name that is not an arrangement. Refused
    //    whole rather than repaired (ADR-0158), and told apart from
    //    *there is no such arrangement*, which is the distinction the
    //    sentence carries.
    Store::open(&root)
        .expect("a store")
        .write_arrangement("four_deck", b"{\"nodes\":[]}")
        .expect("bytes the store does not have to understand");
    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a restore");
    assert!(
        said.contains("disagrees with itself"),
        "a file that will not read back was reported as a missing arrangement, which sends \
         an operator looking for a name they typed correctly: {said}"
    );
    unchanged(&panel, &said);

    std::fs::remove_dir_all(&root).expect("clean up");
}

// -- the arrangement pill's half of the family ----------------------

/// A name that is not one path component is refused here, which is the
/// authority the pill deliberately does not hold (P-0090).
///
/// The negative control is the point: a check that refused everything would
/// pass an assertion that only ever looked for a refusal, so the names that
/// must be *accepted* are asserted beside the ones that must not
/// (`docs/contributing.md` §3).
#[test]
fn a_typed_arrangement_name_is_refused_where_the_file_is_written() {
    for good in ["night", "four_deck", "set-2", "A9"] {
        assert!(
            checked_name(good).is_ok(),
            "`{good}` is letters, digits, `-` and `_`, and was refused"
        );
    }
    for (bad, why) in [
        ("", "nothing was typed"),
        ("../../elsewhere", "a path"),
        ("night deck", "a space"),
        ("night.json", "a suffix of its own"),
    ] {
        let refusal = checked_name(bad).expect_err(&format!("`{bad}` is {why} and was kept"));
        assert!(
            refusal.starts_with("arrangement: "),
            "the refusal does not say what it is about: {refusal}"
        );
        assert!(
            bad.is_empty() || refusal.contains(bad),
            "the refusal does not say the name back, so an operator cannot see what \
             they typed: {refusal}"
        );
    }
}

/// The name in use follows the file and never the press.
///
/// A save that landed and a restore that landed each make that arrangement the
/// one in use, so the pill names it; a save that was refused leaves the pill
/// saying what it said, because nothing under that name is on the disk. And the
/// menu's listing gains the new name only where a file appeared.
#[test]
fn the_pill_names_the_arrangement_only_once_the_file_is_there() {
    let root = arrangement_root("in-use");
    let mut panel = arranged(1280.0, 720.0);
    let mut arr = view::Arrangement::NONE;

    // Refused: the name is not one path component, so nothing was filed
    // and nothing is in use.
    let said = arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::SaveArrangement {
            name: "night/one".to_owned(),
        },
    )
    .expect("a save is one of the operations this route answers for");
    assert!(said.contains("holds `/`"), "{said}");
    assert_eq!(arr.name, None, "a refused save put a name on the pill");
    assert!(arr.filed.is_empty());

    // Kept: in use, and listed.
    arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::SaveArrangement {
            name: "night".to_owned(),
        },
    )
    .expect("a save");
    assert_eq!(arr.name.as_deref(), Some("night"));
    assert_eq!(arr.filed, vec!["night".to_owned()]);

    // A restore of a name nothing is filed under is refused where the
    // bytes are, and leaves the pill alone.
    let said = arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::RestoreArrangement {
            name: "rehearsal".to_owned(),
        },
    )
    .expect("a restore");
    assert!(said.contains("rehearsal"), "{said}");
    assert_eq!(
        arr.name.as_deref(),
        Some("night"),
        "a refused restore moved the name the pill is showing"
    );

    // And one that is filed does put it in use.
    arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::RestoreArrangement {
            name: "night".to_owned(),
        },
    )
    .expect("a restore");
    assert_eq!(arr.name.as_deref(), Some("night"));

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A reset takes the name off the pill, whichever surface asked.
///
/// `r` and the menu's *start a new one* are one operation and `Readout::op` is
/// where both arrive, so this is asserted through the method rather than
/// through either control: the default arrangement is what is on screen and the
/// default has no name.
#[test]
fn a_reset_leaves_the_pill_naming_no_file() {
    let mut readout = Readout::new(1280.0, 720.0);
    readout.view.arrangement.name = Some("night".to_owned());

    // An operation that is not a reset leaves it alone, which is what says
    // the clearing is the reset's and not every operation's.
    let staging = readout.panel.layout().find("staging").expect("staging");
    readout.op(Op::Fold(staging));
    assert_eq!(readout.view.arrangement.name.as_deref(), Some("night"));

    assert_eq!(readout.op(Op::Reset), Outcome::Reset);
    assert_eq!(
        readout.view.arrangement.name, None,
        "the console was reset to the default and the pill still names a file"
    );
}

/// What the load control's five asks do to this program, and that a pick moves
/// this bay's mark and nothing else (ADR-0305).
///
/// [`every_ask_the_pill_makes_is_acted_on_and_shuts_the_menu`]'s shape one bay
/// along, and it is here rather than in `karakuri-console` for the reason that
/// test is: `Readout::aimed` is the host's half of the seam, and the console's
/// own tests cannot reach it.
///
/// The claim a reader will doubt is the third one. *Surely picking a deck
/// selects it* — and it must not: `Operation::SelectDeck` moves the keys, and
/// this mark is the one that is allowed to name another deck. So the selection
/// is read before and after.
#[test]
fn every_ask_the_load_control_makes_is_acted_on_and_moves_only_this_bays_mark() {
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.mixer = (0..4)
        .map(|slot| view::Strip {
            name: format!("slot{slot}"),
            tally: view::Tally::Live,
            requested: view::Tally::Live,
            gain: 0.5,
            gain_to: None,
            opacity: 0.5,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
        })
        .collect();
    assert!(readout.view.select(1), "the keys did not go to deck B");

    assert_eq!(readout.aimed(Aim::Open), Acted::Nothing);
    assert!(readout.view.target_open(), "the list did not come down");
    assert_eq!(readout.aimed(Aim::Shut), Acted::Nothing);
    assert!(!readout.view.target_open());

    // **A pick names a deck, asks for nothing, and puts the list away.**
    readout.aimed(Aim::Open);
    assert_eq!(readout.aimed(Aim::Deck(2)), Acted::Nothing);
    assert_eq!(
        readout.view.target_deck(),
        2,
        "the pick did not aim the load"
    );
    assert_eq!(
        readout.view.selection(),
        1,
        "a pick in the pulldown moved the deck selection, which is the one thing this \
         control must not do"
    );
    assert!(
        !readout.view.target_open(),
        "the list stayed down after a pick"
    );

    // **And the load goes down the path the key and the drop take.**
    let want = Operation::LoadSet {
        deck: 2,
        set: "drift_night".to_owned(),
    };
    assert_eq!(
        readout.aimed(Aim::Load(want.clone())),
        Acted::Emitted(Some(want)),
        "the load did not go down the path every other emitted operation takes"
    );
    assert_eq!(
        readout.aimed(Aim::NoSet),
        Acted::Nothing,
        "a press with no Set under the cursor emitted something"
    );
}

/// What the menu's five asks do to this program, and that every one of them
/// that acts shuts the card.
#[test]
fn every_ask_the_pill_makes_is_acted_on_and_shuts_the_menu() {
    let mut readout = Readout::new(1280.0, 720.0);
    readout.view.arrangement.filed = vec!["night".to_owned()];

    assert!(matches!(readout.arranged(Ask::Open), Acted::Nothing));
    assert!(readout.view.arrangement.open());
    assert!(matches!(readout.arranged(Ask::Shut), Acted::Nothing));
    assert!(!readout.view.arrangement.open());

    readout.arranged(Ask::Open);
    assert!(matches!(readout.arranged(Ask::Name), Acted::Nothing));
    assert_eq!(
        readout.view.arrangement.naming(),
        Some(""),
        "the one item that asks for letters left nothing asking for any"
    );

    readout.arranged(Ask::Open);
    let did = readout.arranged(Ask::Panel(Op::Reset));
    assert!(
        matches!(did, Acted::Operated(Outcome::Reset)),
        "the reset was not performed: {did:?}"
    );
    assert!(
        !readout.view.arrangement.open(),
        "the card is still standing"
    );

    readout.arranged(Ask::Open);
    let want = Operation::RestoreArrangement {
        name: "night".to_owned(),
    };
    let did = readout.arranged(Ask::Operation(want.clone()));
    assert_eq!(
        did,
        Acted::Emitted(Some(want)),
        "the operation did not go down the path every other emitted operation takes"
    );
    assert!(!readout.view.arrangement.open());
}

/// The four presses the Sequencer bay performs, and the one thing this window
/// does with a pattern that no test in `karakuri-console` can see: that bay
/// hands back an operation and applies nothing, so this is the other side of
/// that seam.
///
/// A press names a bank, and a bank this session does not have is refused and
/// said out loud — [`pointed`]'s rule, and the reason each arm answers a
/// sentence rather than `None`.
#[test]
fn a_sequencer_press_moves_the_pattern_it_names() {
    let mut banks = demonstration_banks();
    let mut playhead = karakuri_pattern::Playhead::default();
    assert!(
        banks.pattern().lanes()[0].muted(),
        "a run starts with the lane muted, so nothing writes deck A's fader until a hand asks"
    );
    // The mute, taken back.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SetLaneMute {
            pattern: 0,
            lane: 0,
            muted: false,
        }
    )
    .is_some());
    assert!(!banks.pattern().lanes()[0].muted());
    // A step, set rather than flipped.
    for on in [true, true, false] {
        assert!(sequenced(
            &mut banks,
            &mut playhead,
            &View::new(Room::Day),
            &Operation::SetStep {
                pattern: 0,
                lane: 0,
                step: 4,
                on,
            }
        )
        .is_some());
        assert_eq!(
            banks.pattern().lanes()[0].slot_on(4),
            on,
            "a press asks for a state, so asking twice for the same one leaves it there"
        );
    }
    // A slot the mode press must not touch, and it is slot 5 — an odd one,
    // which an eighth does not read at all and which is therefore the slot
    // a store sized to the count would have thrown away.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SetStep {
            pattern: 0,
            lane: 0,
            step: 5,
            on: true,
        }
    )
    .is_some());
    // The mode, which changes a reading and forgets where the playhead
    // was, because the index it remembered is about a reading that has
    // gone.
    playhead.advance(banks.pattern(), 0.0);
    assert!(playhead.at().is_some());
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SetPatternGrid {
            pattern: 0,
            grid: karakuri_operation::StepMode::Eighth,
        }
    )
    .is_some());
    assert_eq!(banks.pattern().mode().count(), 8);
    assert_eq!(
        playhead.at(),
        None,
        "a mode press is the same bar at another width, so the next poll is a boundary"
    );
    assert!(
        banks.pattern().lanes()[0].slot_on(5),
        "and the sixteen slots underneath are untouched — including the odd ones an eighth \
         does not read"
    );
    // A bank, and one this session does not have.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SelectPattern { pattern: 3 }
    )
    .is_some());
    assert_eq!(banks.armed(), 3);
    assert!(
        banks.pattern().is_empty(),
        "bank 3 is one of the three empty ones"
    );
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SelectPattern { pattern: 9 },
    )
    .expect("a press this session cannot perform says so rather than going quiet");
    assert!(refused.contains("there is no bank 9"));
    assert_eq!(banks.armed(), 3, "and a refused press moves nothing");
}

/// A lane arrives with its two levels filled in from the range the console was
/// published, and it arrives muted.
///
/// This is the half of `+ lane` no test in `karakuri-console` can see: that bay
/// hands back `Operation::PointLane { pattern, target }` and appends nothing,
/// and the payload carries no levels — a fader's are 1.0 and 0.0 and a
/// parameter's are the range `View::inspector` holds, which is the same reading
/// the chooser drew its items from (ADR-0320, ADR-0327).
#[test]
fn a_pointed_lane_takes_its_levels_from_the_published_range() {
    let mut banks = karakuri_pattern::Banks::default();
    let mut playhead = karakuri_pattern::Playhead::default();
    let param = karakuri_operation::ParamAt {
        node: Some(karakuri_operation::NodeAddress {
            layer: karakuri_operation::Layer::L2,
            index: 0,
        }),
        key: "twist".to_owned(),
    };
    let mut view = View::new(Room::Day);
    view.inspector = vec![view::Pane {
        deck: 1,
        material: "lattice_veil".to_owned(),
        sync: karakuri_operation::Sync::Free,
        allows: [true; view::SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: false,
        // Not this test's row: the deck head's two build chips are
        // drawn from this and nothing here is about them.
        aimed: None,
        nodes: vec![view::Node {
            addr: "L2:0".to_owned(),
            name: "warp".to_owned(),
            authority: None,
            // Not this test's control either: nothing here presses a
            // node head's `keep`.
            keep: None,
            uses: Vec::new(),
            renderers: Vec::new(),
            params: vec![view::Param {
                ord: Some(1),
                name: "twist".to_owned(),
                value: 1.0,
                // **Not `[0, 1]`**, so a range that was read and a pair
                // that was assumed cannot look alike.
                range: [0.25, 4.0],
                param: param.clone(),
                bound: None,
            }],
        }],
    }];

    // A fader's pair is the gate, and no reading is consulted for it.
    let line = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Fader { deck: 0 },
        },
    )
    .expect("a lane press says what it did");
    assert!(line.contains("on 1 off 0"), "a fader's gate: {line}");
    let lane = &banks.pattern().lanes()[0];
    assert_eq!((lane.on(), lane.off()), (1.0, 0.0));
    assert!(
        lane.muted(),
        "a lane arrives with every slot off, and an off step writes `off` — so an unmuted \
         fader lane would hold its deck at zero from the press"
    );

    // A parameter's pair is the published range, top then bottom.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Param {
                deck: 1,
                param: param.clone(),
            },
        }
    )
    .is_some());
    let lane = &banks.pattern().lanes()[1];
    assert_eq!(
        (lane.on(), lane.off()),
        (4.0, 0.25),
        "an on step writes the top of the published range and an off step the bottom"
    );

    // **A control this console holds no row for is refused and said**,
    // rather than defaulted into two numbers nobody chose.
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Param {
                deck: 3,
                param: param.clone(),
            },
        },
    )
    .expect("a press this window cannot perform says so rather than going quiet");
    assert!(refused.contains("no published range"), "{refused}");
    assert_eq!(
        banks.pattern().lanes().len(),
        2,
        "and a refused press appends nothing"
    );
}

/// A finished name is one operation of the vocabulary, and the card is gone
/// before it is emitted — whether or not the name is any good, since the
/// refusal is said out loud by `checked_name` and a card left standing over it
/// would be the panel asking again without saying the answer.
#[test]
fn a_finished_name_is_the_save_the_menu_would_have_asked_for() {
    let mut readout = Readout::new(1280.0, 720.0);
    assert_eq!(
        readout.named(),
        Acted::Nothing,
        "a console with nothing being typed committed a name"
    );

    readout.arranged(Ask::Name);
    for c in "four_deck".chars() {
        assert!(readout.view.arrangement.typed(c));
    }
    assert_eq!(
        readout.named(),
        Acted::Emitted(Some(Operation::SaveArrangement {
            name: "four_deck".to_owned()
        }))
    );
    assert!(!readout.view.arrangement.open());

    // An empty name is emitted as one and refused where the file is
    // written, rather than being swallowed here.
    readout.arranged(Ask::Name);
    assert_eq!(
        readout.named(),
        Acted::Emitted(Some(Operation::SaveArrangement {
            name: String::new()
        }))
    );
}

/// The menu's list is the store's, and a store that is not there is listed as
/// nothing and is not created — `library`'s two rules over the fourth
/// directory.
#[test]
fn the_menu_lists_the_store_and_makes_none() {
    let root = arrangement_root("listing");
    assert!(
        arrangements(&root).is_empty(),
        "a store that is not there listed something"
    );
    assert!(
        !root.exists(),
        "listing the arrangements created a store at {}",
        root.display()
    );

    let mut panel = arranged(1280.0, 720.0);
    for name in ["rehearsal", "four_deck"] {
        keep_arrangement(&root, &panel, name);
    }
    panel.solve();
    assert_eq!(
        arrangements(&root),
        vec!["four_deck".to_owned(), "rehearsal".to_owned()],
        "the menu lists what the store holds, in the order the store sorts it"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Nothing that comes back from `get_current_texture` is dropped without a
/// decision. The loop waits for events, so an outcome that neither reconfigures
/// nor asks for another frame is a window that never draws again and says
/// nothing about it.
#[test]
fn every_frame_that_could_not_be_acquired_is_acted_on() {
    use wgpu::CurrentSurfaceTexture as Acquired;
    // The two that mean the swapchain is stale: reconfigure, and ask again.
    assert_eq!(missed(&Acquired::Outdated), Some(Missed::Remake));
    assert_eq!(missed(&Acquired::Lost), Some(Missed::Remake));
    // Jitter: ask again, without reconfiguring.
    assert_eq!(missed(&Acquired::Timeout), Some(Missed::Again));
    // A window nobody can see: asking again is a spin, and the OS says
    // when it is back.
    assert_eq!(missed(&Acquired::Occluded), Some(Missed::Idle));
    // Not self-correcting, so it is said rather than retried.
    assert_eq!(missed(&Acquired::Validation), Some(Missed::Fault));
}

/// A figure quoted in prose is held against the run that was just taken, so a
/// panel that grows says so instead of leaving a sentence that was true of a
/// smaller one.
///
/// This is the failure the guard exists for, and it is not hypothetical: the
/// line above the reading cited ADR-0164's 184 allocations and said the `egui`
/// pass "is still that" through the mixer bay landing at 456 and the parked
/// deck at 525 — two commits of a present-tense claim nobody re-checked,
/// because nothing re-checked it.
///
/// What can be asserted here is the verdict, not the reading. A reading needs a
/// device, a window and three seconds of nobody touching it, so it cannot be
/// taken from `cargo test`; what this file can do is make the figure in the
/// sentence and the figure under the verdict one constant, and hold [`drifted`]
/// to catching what actually went wrong.
#[test]
fn a_reading_that_has_moved_says_the_sentence_quoting_it_is_stale() {
    // The band a run has to stay inside to say nothing. The nine runs
    // behind the figure of 2026-08-31 agreed to the allocation; the nine
    // behind 2026-08-26's read between 524 and 538, which is the widest
    // run-to-run spread this file has ever taken, and a guard that fired
    // on 14 allocations is one nobody could keep passing.
    assert_eq!(drifted(WRITTEN_ALLOCS, WRITTEN_ALLOCS), None);
    assert_eq!(
        drifted(WRITTEN_ALLOCS + 14, WRITTEN_ALLOCS),
        None,
        "the run-to-run spread of the reading this quotes must not read as staleness"
    );

    // And what it is for: ADR-0164's 184 against the mixer bay's 456 is
    // 2.5x, so the first run after that bay landed would have said the
    // sentence had stopped being true. It is the same answer whichever of
    // the two is the one written down, because a pass that got cheaper
    // makes the sentence just as untrue.
    assert!(
        drifted(456, 184).is_some(),
        "the mixer bay's landing is the drift this exists to have caught"
    );
    assert!(
        drifted(184, 456).is_some(),
        "drift is not caught one way round only"
    );

    // A band of two is still a band: an order of magnitude is well out of
    // it, from either end.
    assert!(drifted(WRITTEN_ALLOCS * 10, WRITTEN_ALLOCS).is_some());
    assert!(drifted(WRITTEN_ALLOCS / 10, WRITTEN_ALLOCS).is_some());
}

/// A frame nobody asked for is the one the reading is about.
///
/// The measurement carries the claim now, so what it counts has to be asserted
/// rather than eyeballed on stdout. The failure it exists for is the one that
/// made the first run of this read `3 frames` on a window that had behaved
/// perfectly: an event resets the stretch, and the frame that event asked for
/// lands a millisecond into the new one and gets blamed on the panel. The other
/// direction is worse and is asserted too — a `push` that never counts anything
/// reads `0 frames` whatever the window is doing, which is a measurement that
/// cannot fail.
#[test]
fn a_frame_nobody_asked_for_is_the_one_counted_against_a_still_panel() {
    let frame = Cost {
        allocs: 7,
        bytes: 70,
        ..Default::default()
    };
    let mut costs = Costs::new();

    // A frame something asked for is that something's.
    costs.owes();
    costs.push(frame);
    assert_eq!(costs.still, Still::default(), "an owed frame was counted");

    // A frame nobody asked for — `egui`'s own deadline, on an untouched
    // window — is per-frame work on a still panel, which is the number.
    costs.push(frame);
    assert_eq!(
        costs.still,
        Still {
            frames: 1,
            allocs: 7,
            bytes: 70
        },
        "a frame nobody asked for was not counted"
    );

    // Both frames happened, whoever they belonged to.
    assert_eq!(costs.drawn, 2);

    // And touching the window starts the stretch again, so what was drawn
    // during the last one stops being about a still panel.
    costs.touched();
    assert_eq!(costs.still, Still::default());
}

/// The defect this instrument was built for, stated as an assertion.
///
/// A frame that spent 200 ms blocked and 3 ms on the CPU is a 203 ms frame.
/// [`Cost::whole`] answers 3 ms, and that is not an error in it — it is CPU
/// time and says so — but it is what a reader who wants *what did this frame
/// cost* used to be handed, and what a loop at four frames a second was read
/// off as *idle 97.6% of the time*. The number with the wait in it is
/// [`Cost::period`], and the frame's own arithmetic is here so that a later
/// widening of `whole` fails rather than passes.
#[test]
fn a_frames_cost_has_the_wait_in_it_and_the_three_cpu_stretches_do_not() {
    let frame = Cost {
        wait: Duration::from_millis(200),
        engine: Duration::from_millis(1),
        ui: Duration::from_millis(1),
        paint: Duration::from_millis(1),
        period: Some(Duration::from_millis(205)),
        ..Cost::default()
    };

    assert_eq!(frame.whole(), Duration::from_millis(3), "the CPU's share");
    assert_eq!(
        frame.period,
        Some(Duration::from_millis(205)),
        "what the frame cost"
    );
    // The residue: the period, less the wait, less the three stretches.
    // Two milliseconds of this frame are in no field of it, which is the
    // claim `Cost::whole` used to make in prose — that its three *tile the
    // frame exactly* — measured instead of asserted.
    assert_eq!(frame.elsewhere(), Some(Duration::from_millis(2)));

    // **A frame with no predecessor has no period, and no residue
    // either.** A zero here would read as a frame that spent nothing
    // anywhere, which is the shape of answer P-0095 refuses.
    assert_eq!(Cost::default().elsewhere(), None);
}

/// A period is an interval and needs two frames, and an audit happens at most
/// once per [`Costs::AUDIT`] however many frames go by.
///
/// Both are the same rule from two sides: the instrument reads the clock rather
/// than counting frames, so nothing about how fast this window draws changes
/// what either answers.
#[test]
fn the_first_frame_has_no_period_and_an_audit_does_not_repeat() {
    let mut costs = Costs::new();
    let began = Instant::now();

    assert_eq!(
        costs.tick(began),
        None,
        "there was no frame before the first"
    );
    assert_eq!(
        costs.tick(began + Duration::from_millis(17)),
        Some(Duration::from_millis(17)),
        "the interval between two anchors is the frame"
    );

    // Fresh, the stretch has not elapsed: a run does not audit its first
    // frame, which is the one that builds the font atlas.
    assert!(!costs.audit(), "the first frame of a run was audited");
    // Wound back past the stretch, exactly one frame takes the audit and
    // the frame after it does not.
    costs.since_audit = Instant::now() - Costs::AUDIT;
    assert!(costs.audit(), "a stretch elapsed and nothing was audited");
    assert!(!costs.audit(), "two frames in a row were audited");
}

/// A whole drag, through the window loop's own routing.
///
/// `karakuri_console::input`'s tests are about the rule; this is about this
/// file obeying it, which is a different claim and the one that actually
/// reaches an operator. It drives the gesture a hand makes — press on the
/// boundary between the left pane and the centre, run the pointer well past it
/// and across two bays, let go — through `Readout::pointer`, which is the
/// method `window_event` calls, and asserts two things: the boundary moved, so
/// the drag works with a toolkit in the loop, and `egui` was never told about
/// any of it, so the two never both think they are dragging.
///
/// It cannot be a real pointer: synthesising one takes an Accessibility grant
/// this process does not have, and a test that needs a human to click is not a
/// test.
#[test]
fn a_drag_through_the_window_loops_own_routing_never_reaches_egui() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let layout = readout.panel.layout();
    let centre = layout.rect(layout.find("centre").expect("centre"));
    let left = layout.rect(layout.find("left-pane").expect("left-pane"));
    let was = left.w;
    // The gap between the left pane and the centre, at half height.
    let start = Point::new((left.x + left.w + centre.x) * 0.5, left.y + left.h * 0.5);

    // Approaching it is egui's until the pointer is on it.
    assert_eq!(
        readout
            .pointer(&ctx, Pointer::Moved(Point::new(start.x - 60.0, start.y)))
            .0,
        Claim::Egui
    );
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(start)).0, Claim::Panel);
    assert_eq!(readout.pointer(&ctx, Pointer::Down).0, Claim::Panel);

    // A hand does not stay on the boundary: it runs on across the panel,
    // and every one of these is inside a bay.
    for x in [start.x + 40.0, start.x + 120.0, start.x + 200.0] {
        assert_eq!(
            readout
                .pointer(&ctx, Pointer::Moved(Point::new(x, start.y)))
                .0,
            Claim::Panel,
            "the drag lost its claim at x = {x}"
        );
        // A wheel in the middle of a drag is the panel's too, and it
        // scrolls nothing: rule 1 withholds it from `egui` and
        // `input::wheeled` refuses it, so no pane moves under a hand that
        // is holding a boundary.
        let wheeled = readout.pointer(
            &ctx,
            Pointer::Wheel(karakuri_console::room::size::WHEEL_STEP),
        );
        assert_eq!(wheeled.0, Claim::Panel);
        assert_eq!(
            wheeled.1,
            Acted::Nothing,
            "a wheel in the middle of a boundary drag moved something"
        );
    }
    readout.panel.solve();
    let wide = pane_width(&readout);
    assert!(
        wide > was + 100.0,
        "the boundary did not move: the left pane went from {was} to {wide}"
    );

    // Now back the other way, to exactly what the left pane will not go
    // below. The boundary started at 340 and the pointer took hold 5 past
    // it, so 180 back from where it grabbed asks for the pane's own
    // minimum and no further.
    let stop = Point::new(start.x - 180.0, start.y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(stop)).0, Claim::Panel);
    readout.panel.solve();
    assert!(
        (pane_width(&readout) - 160.0).abs() < 0.01,
        "the left pane's stated minimum did not hold the drag: {}",
        pane_width(&readout)
    );

    // And on past it, and let go there. **The pointer ends nowhere near
    // the boundary**, which is the ordinary end of a drag and the case
    // that catches a release routed after `released` rather than before
    // it.
    //
    // **A pull this far past a pane's own minimum closes the pane**
    // (`docs/adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md`),
    // which is what the assertion above used to say instead: the minimum
    // holds a drag that stops at it, and a drag that goes on through it is
    // asking for the fold. The pane keeps its edge, so the boundary is
    // still there at the window's own edge and another drag brings it
    // back — none of which is this test's subject, which is that the
    // window loop's routing never lets go of the gesture.
    let far = Point::new(start.x - 200.0, start.y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(far)).0, Claim::Panel);
    assert_eq!(readout.pointer(&ctx, Pointer::Up).0, Claim::Panel);

    readout.panel.solve();
    let left = readout
        .panel
        .layout()
        .find("left-pane")
        .expect("a left pane");
    assert!(
        readout.panel.layout().is_closed(left),
        "a drag 40 past the left pane's own minimum left it {} wide rather than closing it",
        pane_width(&readout)
    );

    // **And the way back, through the same routing.** A closed pane keeps
    // its boundary at the window's own edge, so the gesture that brings it
    // back is a second drag on that boundary — pressed, pulled inward, let
    // go. This is the sufficient half of *Fold a pane away* for the row
    // that needs it: `karakuri-console` cannot depend on this file, so
    // whether a hand on a real window reaches the fold is a test here.
    let edge = readout
        .panel
        .layout()
        .boundary(readout.panel.layout().parent(left).expect("a body row"), 0)
        .expect("a closed pane keeps its boundary");
    let held = Point::new(edge.x + edge.w * 0.5, start.y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(held)).0, Claim::Panel);
    assert_eq!(readout.pointer(&ctx, Pointer::Down).0, Claim::Panel);
    assert_eq!(
        readout
            .pointer(&ctx, Pointer::Moved(Point::new(held.x + 40.0, held.y)))
            .0,
        Claim::Panel
    );
    assert_eq!(readout.pointer(&ctx, Pointer::Up).0, Claim::Panel);
    readout.panel.solve();
    assert!(
        !readout.panel.layout().is_closed(left),
        "the boundary the closed pane kept was dragged inward and the pane did not come back"
    );
    assert!(
        (pane_width(&readout) - 160.0).abs() < 0.01,
        "the pane came back {} wide rather than at the minimum it declares",
        pane_width(&readout)
    );

    // And afterwards the pointer, where it is standing, is egui's again.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(far)).0, Claim::Egui);
}

/// A wheel over an Inspector pane scrolls that pane, and no other.
///
/// # Why it is here and can be nowhere else
///
/// `karakuri-console` holds the position and the derivation, and
/// `input::wheeled` answers *which pane* — but nothing in that crate joins the
/// two, because joining them is routing a window event and there is no window
/// there. `Readout::pointer` is the join, and it is a method rather than four
/// arms of `window_event` for exactly this reason: `winit` hands out no
/// `ActiveEventLoop` outside its own loop, so the handler is not something a
/// test can call and the part worth testing is this
/// ([ADR-0307](../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
///
/// Four things, and the third is the one a single-pane inspector would have
/// hidden: the wheel is aimed with the pointer, so two panes are two positions
/// and turning one must leave the other where it was.
#[test]
fn a_wheel_over_an_inspector_pane_scrolls_that_pane() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.view.inspector = (0..view::PANES).map(deep_pane).collect();
    readout.panel.solve();

    let step = karakuri_console::room::size::WHEEL_STEP;
    let middle = |readout: &Readout, name: &str| {
        let layout = readout.panel.layout();
        let rect = layout.rect(layout.find(name).expect("a pane"));
        Point::new(rect.x + rect.w * 0.5, rect.y + rect.h * 0.75)
    };
    let first = middle(&readout, view::PANE_NAMES[0]);
    let second = middle(&readout, view::PANE_NAMES[1]);

    // 1. The wheel over the first pane is the panel's, and it moves that
    //    pane's position by exactly one notch.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(first)).0, Claim::Egui);
    let turned = readout.pointer(&ctx, Pointer::Wheel(step));
    assert_eq!(
        turned,
        (Claim::Panel, Acted::Pointed),
        "the wheel over a pane did not reach the panel, or reached it and moved nothing"
    );
    assert_eq!(readout.view.scroll_in(0), step);
    assert_eq!(
        readout.view.scroll_in(1),
        0.0,
        "the wheel over one pane moved the other"
    );

    // 2. And the other pane is its own.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(second)).0, Claim::Egui);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Wheel(step * 2.0)),
        (Claim::Panel, Acted::Pointed)
    );
    assert_eq!(readout.view.scroll_in(1), step * 2.0);
    assert_eq!(
        readout.view.scroll_in(0),
        step,
        "the second pane's wheel moved the first"
    );

    // 3. A wheel anywhere else on the console is `egui`'s and moves
    //    nothing — the Library bay's list is a whole column away and is
    //    the bay whose own note says it does not scroll.
    let elsewhere = middle(&readout, "library");
    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(elsewhere)).0,
        Claim::Egui
    );
    assert_eq!(
        readout.pointer(&ctx, Pointer::Wheel(step)),
        (Claim::Egui, Acted::Nothing),
        "a wheel over a bay that does not scroll was taken by the panel"
    );
    assert_eq!(readout.view.scroll_in(0), step);
    assert_eq!(readout.view.scroll_in(1), step * 2.0);

    // 4. A wheel against the top of a pane's list is the panel's and is
    //    owed no frame — which is what `Acted::Nothing` says here and what
    //    `Change::Wheeled`'s second field carries.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(first)).0, Claim::Egui);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Wheel(-step * 10.0)),
        (Claim::Panel, Acted::Pointed),
        "the first spin back should have moved it to the top"
    );
    assert_eq!(readout.view.scroll_in(0), 0.0);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Wheel(-step)),
        (Claim::Panel, Acted::Nothing),
        "a wheel spun against the top of the list asked for a frame"
    );
}

/// A pane with more in it than any pane on this panel can hold — four node
/// groups of six rows each, which is 4 x (26.5 + 6 x 22.5) = 646 and is taller
/// than the Inspector bay at the window this test opens.
fn deep_pane(deck: usize) -> view::Pane {
    view::Pane {
        deck,
        material: format!("deep_{deck}"),
        sync: karakuri_operation::Sync::Free,
        allows: [true; view::SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: false,
        // Not this test's row: the deck head's two build chips are
        // drawn from this and nothing here is about them.
        aimed: None,
        nodes: (0..4)
            .map(|node| view::Node {
                addr: format!("L1:{node}"),
                name: format!("node_{node}"),
                authority: Some(view::NodeAuthority {
                    at: karakuri_operation::NodeAddress {
                        layer: karakuri_operation::Layer::L1,
                        index: node as u32,
                    },
                    level: karakuri_operation::Authority::Manual,
                }),
                keep: Some(karakuri_operation::NodeAddress {
                    layer: karakuri_operation::Layer::L1,
                    index: node as u32,
                }),
                uses: Vec::new(),
                renderers: Vec::new(),
                params: (0..6)
                    .map(|at| view::Param {
                        ord: Some(at + 1),
                        name: format!("n{node}p{at}"),
                        value: 0.5,
                        range: [0.0, 1.0],
                        param: karakuri_operation::ParamAt {
                            node: Some(karakuri_operation::NodeAddress {
                                layer: karakuri_operation::Layer::L1,
                                index: node as u32,
                            }),
                            key: format!("n{node}p{at}"),
                        },
                        bound: None,
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// The left pane's width, solved. A helper because the test asks three times
/// and the chain is four calls long.
fn pane_width(readout: &Readout) -> f32 {
    let layout = readout.panel.layout();
    layout.rect(layout.find("left-pane").expect("left-pane")).w
}

/// A context that has drawn once, which is what routing a pointer takes: the
/// claim rule asks where the Outputs row's control is, that is the width of the
/// type in it, and `egui`'s fonts are not valid until a pass has run. The
/// window loop has drawn long before a hand arrives; a test has to say so.
///
/// The texture delta is cleared because `epaint` panics if one is dropped
/// unapplied — there is no renderer here to apply it to, which is the whole of
/// what makes this a test and not a window.
///
/// `mod gpu` uses it too: a strip is laid out with the type in it, and a device
/// does not make fonts valid.
pub(super) fn drawn_once() -> egui::Context {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
    out.textures_delta.clear();
    ctx
}

/// The whole of what the four class pills are for: an operation the gate
/// refuses becomes one it allows, because a hand pressed a capsule.
///
/// # Why it is here and can be nowhere else
///
/// It crosses three crates and no two of them can see the third.
/// `karakuri-console` draws the pill and hands back a value; it must not name
/// `karakuri-environment` at all (ADR-0156), so it cannot reach the handle.
/// `karakuri-environment` holds the `Opening` and cannot see a console.
/// `karakuri-operation`'s gate holds the audit and the refusal and depends on
/// neither. This file is the only place all three are in scope, which is the
/// same reason `key_column` is a unit test in this binary: a surface is where
/// the buck stops, nothing may depend on this package, and the checks that need
/// everything at once live in it.
///
/// # What it asserts, in the order an operator's afternoon goes
///
/// 1. `SetGain` is in the mix-fader class, which is the classification ADR-0235
/// drew — asserted against `standing` rather than assumed, so that a row moved
/// out of the class fails here rather than making this test quietly vacuous. 2.
/// On a run nobody has touched it is refused, and the sentence is
/// `gate::refusal`'s own by equality — P-0090, *a refusal a person can reach
/// from two surfaces is one sentence*, asserted against the function rather
/// than with a `contains`. It names the Mixer bay, because a model that is told
/// only *no* reports the instrument as incapable instead of as closed. 3. A
/// press on the Mixer bay's pill — through `Readout::pointer`, which is the
/// same routing a hand goes through, and not by calling `set` here — opens the
/// class. 4. The same call, the same audit, now allowed. Nothing about the
/// operation changed and nothing about the vocabulary changed; the list a model
/// reads never shortened at any point. 5. And exactly that class. The other
/// three are still shut and an operation in one of them is still refused, which
/// is the property the console's own
/// `a_press_opens_exactly_one_class_and_leaves_the_other_three_shut` makes
/// about the value and this one makes about the run. 6. A second press shuts
/// it, and the call is refused again — the other half of the page's *"click
/// again to shut it"*, seen from the gate.
#[test]
fn the_gate_lets_a_refused_operation_through_once_the_class_is_open() {
    use karakuri_operation::gate::{audit, refusal, standing, Running, Standing};

    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    // A write to a mix fader: unpriced, immediate, irreversible, and what
    // the audience is looking at — P-0094's three answers, all missing.
    let write = Operation::SetGain { deck: 0, gain: 0.5 };
    assert_eq!(
        standing(&write, Running::unread()),
        Standing::Closed(Class::MixFaders),
        "`SetGain` is no longer in the class this test is about"
    );
    // And one from another class, to hold the press to one class below.
    let elsewhere = Operation::RecordSession {
        recording: karakuri_operation::Recording::Stop,
    };
    assert_eq!(
        standing(&elsewhere, Running::unread()),
        Standing::Closed(Class::InputsAndOutputs)
    );

    // 2. Refused, in one sentence, and it says where a hand opens it.
    let refused = audit(&write, readout.opening.read(), Running::unread())
        .expect_err("a mix write is allowed on a run nobody has opened anything on");
    assert_eq!(
        refused,
        refusal(&write, Standing::Closed(Class::MixFaders)).expect("a refusal has a sentence")
    );
    assert!(
        refused.contains("the head of the Mixer bay"),
        "the refusal does not say where the pill is: {refused}"
    );

    // 3. The press. Where the capsule is comes from the same derivation
    // that painted it, and the event goes through the window loop's own
    // routing — `Opening::set` is never called from this test.
    let capsule = |readout: &mut Readout| {
        readout.panel.solve();
        let pill = mcp_pill(
            &ctx,
            readout.panel.layout(),
            Class::MixFaders,
            readout.view.opening,
        )
        .expect("the Mixer bay draws its class pill");
        (
            Point::new(pill.pill.center().x, pill.pill.center().y),
            pill.open,
        )
    };
    let (at, open) = capsule(&mut readout);
    assert!(!open, "the pill reads open on a run that has just started");
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Opened,
        "the class pill went down one of the other two paths — an `Operation` \
         or an operation on the arrangement — and ADR-0236 says it is neither"
    );
    readout.pointer(&ctx, Pointer::Up);

    // 4. The same call, the same audit, allowed.
    let allowed = audit(&write, readout.opening.read(), Running::unread())
        .expect("the operator opened the class and the call is still refused");
    assert_eq!(allowed.operation(), &write);

    // 5. And exactly that class.
    for class in Class::ALL {
        assert_eq!(
            readout.opening.read().holds(*class),
            *class == Class::MixFaders,
            "one press opened or shut {class:?} as well"
        );
    }
    assert_eq!(
        audit(&elsewhere, readout.opening.read(), Running::unread())
            .expect_err("opening the mix faders opened the outputs too"),
        refusal(&elsewhere, Standing::Closed(Class::InputsAndOutputs)).expect("a sentence")
    );

    // 6. And a second press shuts it again.
    let (at, open) = capsule(&mut readout);
    assert!(
        open,
        "the pill did not read open after the press that opened it"
    );
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Opened);
    assert_eq!(readout.opening.read(), Open::CLOSED);
    assert_eq!(
        audit(&write, readout.opening.read(), Running::unread())
            .expect_err("the class was shut again and the call still goes through"),
        refused
    );
}

/// All four pills are reachable through the window loop's routing, not the
/// Mixer's alone — three of them are in a bay head and the fourth is in a row
/// that has none, and the one this file could most easily have got wrong is the
/// one with no head to hang it in.
#[test]
fn each_of_the_four_pills_opens_its_own_class_through_a_press() {
    let ctx = drawn_once();
    for class in Class::ALL {
        let mut readout = Readout::new(1440.0, 900.0);
        readout.panel.solve();
        let pill = mcp_pill(&ctx, readout.panel.layout(), *class, readout.view.opening)
            .unwrap_or_else(|| panic!("{class:?} draws no pill"));
        let at = Point::new(pill.pill.center().x, pill.pill.center().y);

        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Opened);
        assert_eq!(
            readout.opening.read(),
            Open::CLOSED.with(*class, true),
            "a press on {class:?}'s pill did not open exactly it"
        );
        // **The view is written in the same breath as the handle**, or the
        // very next press is aimed at the capsule that used to be there:
        // the two words are not the same width.
        assert_eq!(readout.view.opening, readout.opening.read());
    }
}

/// A reading is read off the cards, one row per key, and never off a compile.
///
/// The claim `docs/manual/operations.html` makes for this row — *"each read off
/// the artifact's own card, so those three fetch no source and compile
/// nothing"* — and the four things [`declared`] has to get right, each of which
/// a plainer reading would get wrong:
///
/// 1. One control per key. `exposure` is declared by two nodes here, exactly as
/// it is in the mock's own reading, and it is one row. 2. Over the part of the
/// range both of them accept, which is `Set::published`'s intersection done off
/// the cards: `[0, 1]` and `[0.2, 0.8]` is one control over `[0.2, 0.8]`. 3. A
/// node with no card is counted and not skipped in silence, which is the foot's
/// `n without a card` and the one thing that keeps a knob missing for want of a
/// card from being a knob missing. 4. Nothing was compiled. The artifacts here
/// are not `.kir` at all — they are three bytes each — so a reading that
/// fetched and checked a source could not have answered at all, which is the
/// strongest form this claim can be put in.
///
/// A CPU test: a store is a directory and no adapter is opened.
#[test]
fn a_reading_is_read_off_the_cards_and_never_off_a_compile() {
    use karakuri_store::ndjson::Line;
    let root = scratch_dir("read-set-declares");
    let store = Store::open(&root).expect("a store to read");

    // Three nodes: a geometry, a renderer that declares `exposure` over
    // the whole range, and a second renderer that declares it narrower.
    let card = |records: Vec<Record>| -> Vec<Line> { records.into_iter().map(Line::new).collect() };
    let param = |key: &str, min: f32, max: f32, default: Option<f32>| Record::ParamDecl {
        key: key.to_owned(),
        ty: "float".to_owned(),
        min,
        max,
        default,
    };
    let geometry = store.put_artifact(b"g\n").expect("an artifact");
    store
        .write_meta(
            &geometry,
            &card(vec![
                param("radius", 0.0, 8.0, Some(2.0)),
                Record::CapacityDecl {
                    min: 16384,
                    max: 1048576,
                    default: 262144,
                },
                Record::Emit {
                    attrs: vec!["position".to_owned(), "size".to_owned()],
                },
            ]),
        )
        .expect("a card");
    let wide = store.put_artifact(b"r1\n").expect("an artifact");
    store
        .write_meta(&wide, &card(vec![param("exposure", 0.0, 1.0, Some(0.4))]))
        .expect("a card");
    let narrow = store.put_artifact(b"r2\n").expect("an artifact");
    store
        .write_meta(&narrow, &card(vec![param("exposure", 0.2, 0.8, Some(0.9))]))
        .expect("a card");
    // And a fourth node whose artifact this store has no card for, which
    // is an ordinary state and not a damaged store.
    let bare = store.put_artifact(b"r3\n").expect("an artifact");

    let slot = |layer, index, hash| {
        Line::new(Record::Slot {
            at: karakuri_store::record::NodeAddress { layer, index },
            name: None,
            proc_hash: hash,
        })
    };
    use karakuri_store::record::Layer as Written;
    store
        .write_set(
            "drift_night",
            &[
                slot(Written::L1, 0, geometry),
                slot(Written::L4, 0, wide),
                slot(Written::L4, 1, narrow),
                slot(Written::L4, 2, bare),
            ],
        )
        .expect("a Set to read");

    let reading = declared(&root, "drift_night").expect("the Set reads");
    assert_eq!(reading.id, "drift_night");
    assert_eq!(
        reading
            .knobs
            .iter()
            .map(|knob| (knob.key.as_str(), knob.range.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("radius", "0 – 8 · 2"),
            // **One row and two nodes declare it**, over the part of the
            // range both of them accept, and the default is the first
            // declarer's — a control is one number and two nodes may
            // declare two.
            ("exposure", "0.2 – 0.8 · 0.4"),
        ],
        "the reading publishes {:?}",
        reading.knobs
    );
    assert_eq!(
        reading.capacity.as_deref(),
        Some("16384 – 1048576 · 262144"),
        "the capacity is drawn in a knob's shape"
    );
    assert_eq!(reading.emits.as_deref(), Some("position, size"));
    assert_eq!((reading.nodes, reading.described), (4, 3));
    assert_eq!(reading.cards_word(), "1 without a card");
    assert_eq!(reading.knobs_word(), "2 knobs");
    assert_eq!(reading.nodes_word(), "4 nodes");
    // The head, two knobs, the capacity, what it emits, and the foot.
    assert_eq!(reading.rows(), 6);

    // **A default the card cannot state as a number is a word and not a
    // blank**, because the `.kir` grammar makes the expression mandatory:
    // what a blank would say here is that there is no default, which is
    // false.
    let expr = store.put_artifact(b"g2\n").expect("an artifact");
    store
        .write_meta(&expr, &card(vec![param("hue", 0.0, 1.0, None)]))
        .expect("a card");
    store
        .write_set("expr01", &[slot(Written::L1, 0, expr)])
        .expect("a Set to read");
    assert_eq!(
        declared(&root, "expr01")
            .expect("the Set reads")
            .knobs
            .first()
            .map(|knob| knob.range.clone()),
        Some("0 – 1 · expr".to_owned())
    );

    // **A Set that declares nothing is an answer**: no capacity row, no
    // emits row, and a head that says `0 knobs`.
    let empty = store.put_artifact(b"e\n").expect("an artifact");
    store.write_meta(&empty, &card(vec![])).expect("a card");
    store
        .write_set("empty01", &[slot(Written::L4, 0, empty)])
        .expect("a Set to read");
    let reading = declared(&root, "empty01").expect("the Set reads");
    assert_eq!(
        (reading.capacity.clone(), reading.emits.clone()),
        (None, None)
    );
    assert_eq!(reading.rows(), 2, "a head and a foot are the whole of it");

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The answer is written into the view, under the row the cursor is on, and a
/// Set that cannot be read says so and draws nothing.
///
/// [`read_reading`] is the glue the window loop runs on a press that asked for
/// a reading — [`listing`]'s shape one control along — and the two halves worth
/// a test are the ones a caller cannot see: that what is opened is the row
/// under the cursor rather than the first row, and that a failure closes the
/// block. A reading that failed to read and a Set that declares nothing must
/// not draw the same, which is `library`'s own rule one bay up.
///
/// A CPU test: a store is a directory and a `View` takes no device.
#[test]
fn a_reading_is_written_into_the_view_under_the_row_the_cursor_is_on() {
    let root = scratch_dir("read-set-view");
    let store = Store::open(&root).expect("a store to read");
    store.write_set("night01", &[]).expect("a Set to read");
    store.write_set("morph01", &[]).expect("a Set to read");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = vec!["night01".to_owned(), "morph01".to_owned()];
    assert!(
        view.walk(1, 0..2),
        "the cursor did not move off the first row"
    );

    let said = read_reading(&mut view, &root);
    assert!(
        said.contains("morph01"),
        "the reading names the first row rather than the one under the cursor: `{said}`"
    );
    let open = view.opened().expect("the reading is open under the cursor");
    assert_eq!(open.at, 1);
    assert_eq!(open.reading.id, "morph01");
    // A Set file with no `slot` record in it names no material, which is a
    // reading with nothing in it rather than a failure.
    assert_eq!(open.reading.nodes, 0);
    assert_eq!(open.reading.knobs_word(), "0 knobs");

    // **And a row naming a Set this store does not hold puts the block
    // away and says why**, where leaving the last reading drawn would
    // describe one Set under another's name.
    view.library = vec!["night01".to_owned(), "gone01".to_owned()];
    let said = read_reading(&mut view, &root);
    assert!(
        said.contains("gone01") && said.contains("could not be read"),
        "a Set that is not there was read as `{said}`"
    );
    assert!(
        !view.reading_open(),
        "a reading that failed to read left a block open"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// [`reread_if_open`] re-reads on a move with a reading open, and does nothing
/// on any other press — ADR-0265's rule, as a check on the one function both
/// `App::window_event` call sites share, rather than on the two copies of it
/// the window loop used to carry.
///
/// Until 2026-09-11 the two call sites were two verbatim statements, held equal
/// to each other only by a text scan
/// (`reading_follows_the_cursor::both_surfaces_re_read_the_row_the_cursor_arrived_at`)
/// that read this file and matched each one whole. Now there is one statement
/// and not two to keep in step, and this presses it directly: three presses,
/// only the middle one of which is a move, and only the third of which should
/// read anything.
///
/// A CPU test: a store is a directory and a `View` takes no device.
#[test]
fn reread_if_open_re_reads_only_on_a_move_with_a_reading_open() {
    let root = scratch_dir("reread-if-open");
    let store = Store::open(&root).expect("a store to read");
    store.write_set("night01", &[]).expect("a Set to read");
    store.write_set("morph01", &[]).expect("a Set to read");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = vec!["night01".to_owned(), "morph01".to_owned()];

    // **No reading is open**, so a move re-reads nothing — there is
    // nothing for the rule to keep following.
    assert!(
        view.walk(1, 0..2),
        "the cursor did not move off the first row"
    );
    assert_eq!(
        reread_if_open(true, &mut view, &root),
        None,
        "a move with no reading open re-read something anyway"
    );

    // A reading opens on the row the cursor is on now (`morph01`).
    let _ = read_reading(&mut view, &root);
    assert!(view.reading_open());

    // **A press that did not move the cursor**, with a reading open: the
    // rule is the cursor's, so this is the one call `Readout::took` used
    // to get wrong by discarding the `bool` `View::point_at` handed back.
    assert_eq!(
        reread_if_open(false, &mut view, &root),
        None,
        "a press that did not move the cursor re-read anyway"
    );
    assert_eq!(
        view.opened().expect("still open").reading.id,
        "morph01",
        "a press that did not move the cursor changed which row is open"
    );

    // **A move, with the reading still open**: the row the cursor
    // arrives at is the one that comes back, on whichever surface's
    // `moved` said so.
    assert!(view.walk(-1, 0..2), "the cursor did not move back to row 0");
    let said =
        reread_if_open(true, &mut view, &root).expect("a reading was open and the cursor moved");
    assert!(
        said.contains("night01"),
        "reread_if_open read the row the cursor left rather than the one it arrived at: {said}"
    );
    assert_eq!(view.opened().expect("still open").reading.id, "night01");

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A press on the `params` chip asks for the Set under the cursor, and a second
/// press puts the reading away.
///
/// The seam, driven the way an operator drives it: the pointer arrives, the
/// panel claims it, and what comes back is
/// [`Operation::ReadSet`](karakuri_operation::Operation::ReadSet) naming the
/// row the cursor is on. `press_handler` cannot see this — it reads text and
/// asks whether the control is asked — and `karakuri-console`'s own tests
/// cannot see it either, because the press handler is here.
///
/// The close emits nothing, which is the half worth a test: a second press
/// changes which rows the bay draws and asks no question, so a `ReadSet` here
/// would be this program saying a Set was read at the moment one stopped being.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_the_params_chip_asks_for_the_set_under_the_cursor() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));

    // The capsule, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let chip = |readout: &mut Readout| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .params_chip(&ctx, readout.view.target());
        Point::new(at.min.x + 2.0, at.center().y)
    };

    let at = chip(&mut readout);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::ReadSet {
            id: "drift_night".to_owned()
        })),
        "the chip did not ask for the Set under the cursor"
    );

    // **The reading is the caller's to answer**, and this file's press
    // handler holds no store — so the block is opened here the way the
    // window loop opens it, and the second press is what is under test.
    readout.view.read(Reading {
        id: "drift_night".to_owned(),
        ..Reading::default()
    });
    let at = chip(&mut readout);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Nothing,
        "closing a reading emitted an operation, which says a Set was read"
    );
    assert!(
        !readout.view.reading_open(),
        "the second press did not put the reading away"
    );
}

/// A strip, as far as the Library bay cares: something for a deck to be named
/// on. Every reading in it is beside the point here.
fn bare_strip() -> view::Strip {
    view::Strip {
        name: String::new(),
        tally: view::Tally::Allocated,
        requested: view::Tally::Allocated,
        gain: 0.0,
        gain_to: None,
        opacity: 0.0,
        opacity_to: None,
        blend: BlendMode::Add,
        mask: view::Mask::None,
        mask_angle: 0.0,
        level: None,
    }
}

/// A secondary press on a Library row puts that row's menu down, and a primary
/// press on the same row does not.
///
/// This is the one thing neither crate could assert on its own.
/// `karakuri-console` has never known which button a press was — rules 1 to 4
/// of `input::claim` are all about where the pointer is — so the distinction
/// lives here, in the arm that turns a `winit` button into a [`Pointer`]. Both
/// halves are asserted because a handler that opened the menu on either button
/// would pass a test made only of the first, and would take the row's ordinary
/// press away: a primary press picks a Set up to carry it, which is what the
/// drag onto a strip is.
///
/// And the item is picked with either button, which is the other half of the
/// same seam: once the card is down it is `input::claim`'s rule 2, and that
/// rule is about a card being down rather than about what put it there. So the
/// pick here is a *primary* press on a card a secondary press opened.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_secondary_press_opens_a_rows_menu_and_a_primary_press_does_not() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    readout.view.mixer = std::iter::repeat_with(bare_strip).take(4).collect();
    assert!(readout.view.select_scope(Scope::MySets));

    // The second row, asked of the derivation that draws it.
    let row = |readout: &mut Readout| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .row(1);
        Point::new(at.center().x, at.center().y)
    };

    // **A primary press takes the Set in hand and opens nothing.**
    let row_at = row(&mut readout);
    let at = row_at;
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    readout.pointer(&ctx, Pointer::Down);
    assert!(
        matches!(readout.panel.in_hand(), Some(InHand::Carrying)),
        "a primary press on a row did not take the Set in hand"
    );
    assert!(
        !readout.view.menu_open(),
        "a primary press on a row put that row's menu down, which takes the carry away"
    );
    // Let the carry go again, over nothing, so the gesture does not run on
    // into the presses below.
    readout.pointer(&ctx, Pointer::Up);

    // **A secondary press on the same row puts the menu down.**
    assert_eq!(readout.pointer(&ctx, Pointer::Secondary).1, Acted::Nothing);
    assert!(
        readout.view.menu_open(),
        "a secondary press on a row did not put that row's menu down"
    );
    assert_eq!(
        readout.view.menued().row,
        Some(1),
        "the menu came down on a row the press was not on"
    );

    // **A press on another control's capsule dismisses this card rather
    // than opening that one.** The `load` button is the sharpest case
    // there is: it is in this bay's own foot, it is a control the pointer
    // reaches, and a handler that asked it before the card would have
    // opened the pulldown with a menu still down — two cards down at once,
    // which is the one thing `input::claim`'s rule 2 exists to make
    // impossible. This is the assertion that says which was asked first.
    readout.panel.solve();
    let button = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("the bay draws its foot")
    .load(&ctx, readout.view.target())
    .button;
    let at = Point::new(button.center().x, button.center().y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Nothing);
    assert!(
        !readout.view.menu_open(),
        "a press on the `load` button with a menu down did not dismiss it"
    );
    assert!(
        !readout.view.target_open(),
        "a press on the `load` button with a menu down opened the pulldown as well, so two \
         cards were down at once"
    );

    // Open it again, on the same row, for the pick below.
    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(row_at)).0,
        Claim::Panel
    );
    assert_eq!(readout.pointer(&ctx, Pointer::Secondary).1, Acted::Nothing);
    assert_eq!(readout.view.menued().row, Some(1));

    // **And the send is picked with a primary press on the card**, which
    // is rule 2: the card is down, so the press is the card's whichever
    // button it was.
    readout.panel.solve();
    let bay = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("the bay lists its rows");
    let menu = bay
        .menu(
            &ctx,
            view::to_egui(readout.panel.layout().viewport()),
            readout.view.menued(),
        )
        .expect("the menu is down");
    let save = menu.save.expect("a Set row's menu carries a send").center();
    let at = Point::new(save.x, save.y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::TransferSet {
            transfer: SetTransfer::Send {
                id: "lattice_veil".to_owned()
            }
        })),
        "`Save as a kbset` did not ask to send the row the menu was opened on"
    );
    assert!(
        !readout.view.menu_open(),
        "the card stayed down after an item was picked"
    );
}

/// A send that reached the disk says where it went, and a dismissed dialog
/// writes nothing and says so.
///
/// Three outcomes, one sentence each, and the third is the one worth the test:
/// a press that opened a window over the panel and then wrote nothing is
/// exactly the case a reader would otherwise read as a fault, and rule 04 of
/// the manual is that nothing is hidden quietly.
///
/// The dialog is not driven here and does not need to be. What a save dialog
/// answers is a path or nothing, so [`sent`] takes that answer and the platform
/// stays outside the test — the same split [`Save::run`] is on one act along,
/// where the thread is the caller's and the write is a function.
///
/// `None` is asserted to have written nothing at all, by counting the directory
/// rather than by trusting the sentence: a `sent` that bundled first and threw
/// the bytes away would print the same words.
///
/// A CPU test: a store read and a file written.
#[test]
fn a_send_says_where_it_went_and_a_dismissed_dialog_writes_nothing_and_says_so() {
    let root = scratch_dir("send-set");
    let store = Store::open(&root).expect("a store");
    // One Set with one node, so a bundle has a source to inline.
    let hash = store
        .put_artifact(b"proc p { }\n")
        .expect("the source is stored");
    store
        .write_set(
            "night01",
            &[karakuri_store::ndjson::Line::new(Record::Slot {
                at: karakuri_store::record::NodeAddress {
                    layer: karakuri_store::record::Layer::L1,
                    index: 0,
                },
                name: Some("geo".to_owned()),
                proc_hash: hash,
            })],
        )
        .expect("the set is written");

    let out = root.join("outbox");
    std::fs::create_dir_all(&out).expect("an outbox");

    // **Dismissed**: nothing is asked of the disk and the sentence says so.
    let said = sent(&root, "night01".to_owned(), None);
    assert_eq!(said.to, None);
    assert!(
        said.said().contains("dismissed") && said.said().contains("was not written"),
        "a dismissed dialog was reported as `{}`",
        said.said()
    );
    assert_eq!(
        std::fs::read_dir(&out).expect("the outbox").count(),
        0,
        "a dismissed dialog left a file behind"
    );

    // **Written**: the file is where the operator sent it and carries the
    // source inlined, which is what makes it a bundle rather than a copy.
    let to = out.join("night01.kbset");
    let said = sent(&root, "night01".to_owned(), Some(to.clone()));
    assert_eq!(
        said.outcome,
        Ok(()),
        "the send was refused: {}",
        said.said()
    );
    assert_eq!(
        said.said(),
        format!("  send: `night01` written to `{}`", to.display())
    );
    let text = std::fs::read_to_string(&to).expect("the bundle is on the disk");
    assert!(
        text.contains("proc p"),
        "the file names the source rather than carrying it: {text}"
    );

    // **Refused**: a Set this store does not hold, and the words are the
    // bundler's rather than a second copy of them.
    let said = sent(&root, "gone01".to_owned(), Some(out.join("gone01.kbset")));
    assert!(said.outcome.is_err(), "a Set nobody holds was packaged");
    assert!(
        said.said().contains("gone01") && said.said().contains("was not written to"),
        "a refused send was reported as `{}`",
        said.said()
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The marks a reading is spelled with are in the face the panel draws with.
///
/// [`spelled`] writes `0 – 8 · 2` with an en dash and a middle dot, and the
/// Library bay's old `load → A` pill is what this test exists because of: its
/// arrow was typed with a U+2192 `egui`'s default face does not carry, and the
/// panel drew `load □ A` for a release — *a readout of where a press lands,
/// with a tofu where the lands was*. A range with a tofu in it would be the
/// same failure on every row of every reading.
///
/// The minus is here too, because a declared range can start below zero — the
/// mock's own `twist` is `−2 – 2 · 0` — and it is the sign `format!` writes
/// rather than the typographic one, which is asserted so that the two are not
/// quietly swapped.
///
/// A CPU test: fonts are `egui`'s and take no device.
#[test]
fn the_marks_a_reading_is_spelled_with_are_in_the_face() {
    let ctx = drawn_once();
    let spelling = spelled("-2", "2", Some("0".to_owned()));
    assert_eq!(spelling, "-2 – 2 · 0");
    let font = egui::FontId::new(
        karakuri_console::room::size::BASE,
        egui::FontFamily::Proportional,
    );
    for mark in spelling.chars() {
        assert!(
            ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, &mark.to_string())),
            "`{mark}` is not in the default face, and the panel would draw a tofu where a \
             reading's range is"
        );
    }
}

/// A press on the outputs dot, through the window loop's own routing.
///
/// The other half of the test above: that one is a boundary the panel claims
/// and `egui` never sees, and this is the console's one control, which the
/// panel claims for a different reason — `egui` owns no widget anywhere here,
/// so a press routed to it would reach nothing at all.
///
/// What is asserted is the round trip an operator makes: the picture is on
/// screen, a click on the dot folds it away by name, and a click on the same
/// dot brings it back. The dot is where it is drawn and the press is the
/// panel's at every step.
#[test]
fn a_press_on_the_outputs_dot_folds_the_picture_and_unfolds_it() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let picture = readout
        .panel
        .layout()
        .find("program-view")
        .expect("program-view");
    let dot = |readout: &mut Readout| {
        readout.panel.solve();
        let row =
            outputs(&ctx, readout.panel.layout(), Open::CLOSED).expect("the row draws its sink");
        (Point::new(row.sink.center().x, row.sink.center().y), row.on)
    };

    let (at, on) = dot(&mut readout);
    assert!(on, "the picture is on screen, so the sink is on");

    // The pointer arrives, and the control is the panel's.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Operated(Outcome::Folded {
            id: picture,
            folded: true,
            root: false
        }),
        "the press did not reach the sink"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);

    // And the dot is dark, where it still is, and turns the picture back
    // on rather than unfolding whatever else is folded.
    let (at, on) = dot(&mut readout);
    assert!(!on, "the picture is folded and the sink is still lit");
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Operated(Outcome::Folded {
            id: picture,
            folded: false,
            root: false
        }),
        "the dark dot did not turn the picture back on"
    );
    assert!(
        dot(&mut readout).1,
        "the picture is back and the dot is dark"
    );
}

/// A press on a scope chip, through the window loop's own routing — which is
/// what ADR-0213 makes the *panel* badge mean.
///
/// `karakuri-console`'s `tests/library.rs` asserts everything up to the
/// operation with no window anywhere: that the chips answer a press, that a
/// press names the chip it landed on, and that nothing else in the bay takes
/// one. This is the half that badge is actually about — *"the row is claimed
/// the day a person who launched the instrument can perform that operation from
/// the panel in front of them"* — and a control demonstrated in that crate and
/// never wired here would pass there and be a lie this page tells on its own
/// authority.
///
/// What is asserted is the whole press and not the routing alone: the mark
/// moves to the chip that was pressed, the operation that leaves is
/// `SelectScope` with the payload it is specified to carry, and the library
/// cursor goes back to the top — because the listing under a new scope is a
/// listing this cursor has never seen, and a cursor left where it was would sit
/// on a Set nobody chose under a pill saying a press will load it.
///
/// And the chip that is already marked is pressed too, because that is the case
/// a step cannot reach: a step would go somewhere else, and a pointer names —
/// so the press is answered rather than refused, and the mark stays where it
/// is.
#[test]
fn a_press_on_a_scope_chip_names_the_library_the_bay_reads() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    // A console that has been told what libraries there are and handed a
    // listing for the one it opens on, which is what `resumed` does.
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));

    // The capsule, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let chip = |readout: &mut Readout, want: Scope| {
        readout.panel.solve();
        let bay = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows");
        let (_, at) = bay
            .chips(&ctx, &readout.view.scopes)
            .find(|(scope, _)| *scope == want)
            .expect("the scope is on the row");
        // Two pixels in from its own left edge: the last chip in the row
        // is clipped by the pane, so its centre can be off the row.
        Point::new(at.min.x + 2.0, at.center().y)
    };

    // **The cursor is somewhere other than the top**, so that the move
    // back to it is a move and not the state it was already in.
    assert!(readout.view.walk(1, 0..2), "the cursor did not move");
    assert_eq!(readout.view.cursor_row(), 1);

    let at = chip(&mut readout, Scope::Presets);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SelectScope { scope: Undecided })),
        "the press did not reach the chip"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.scope(),
        Some(Scope::Presets),
        "the press was routed and the mark stayed where it was"
    );
    assert_eq!(
        readout.view.cursor_row(),
        0,
        "the scope changed and the cursor is still pointing into the listing it left"
    );

    // **The marked chip, pressed** — answered rather than refused, and the
    // mark does not step off it the way the key would.
    let at = chip(&mut readout, Scope::Presets);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::SelectScope { scope: Undecided })),
        "a press on the chip that is already marked was not answered"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.scope(),
        Some(Scope::Presets),
        "a press on the marked chip stepped somewhere"
    );
}

/// A press on the star at the left of a library row, through the window loop's
/// own routing — the half of the badge ADR-0213 makes a badge mean, beside
/// [`a_press_on_a_filter_field_asks_the_store_for_a_narrower_listing`].
///
/// `karakuri-console`'s `tests/library.rs` says where the mark is and that it
/// answers a press; this says an operator reaches it — a control demonstrated
/// in that crate and never wired here would pass there and be a lie the page
/// tells on its own authority.
///
/// What is asserted is that the press names a state and not a step (ADR-0299):
/// the same mark pressed twice asks for two different things, because the
/// control reads the row's present mark and asks for the other one. That is the
/// failure a toggle hides completely — a press that always emitted `true` would
/// pass every assertion about the first press and never take a star off.
///
/// And that the star has not swallowed the row it sits in: a press on the row's
/// own ground still takes the Set in hand and names no operation, which is rule
/// 4's *a control claims what it acts on and no more* asked of the two boxes
/// that overlap.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_a_star_names_the_state_the_row_is_not_in() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];

    // The mark's box, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let star = |readout: &mut Readout, index: usize| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .star(index);
        Point::new(at.center().x, at.center().y)
    };

    // **Nothing starred, so the press asks for the star to go on.**
    let at = star(&mut readout, 1);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: true,
        })),
        "the press did not reach the star, or it named the wrong row"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);

    // **And with the row starred it asks for the star to come off**, which
    // is the same control reading the state it is drawn from. The marks
    // are the host's answer, so this is what `listing` would have written
    // after the write.
    readout.view.starred.insert("lattice_veil".to_owned());
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: false,
        })),
        "a starred row was asked to be starred again"
    );
    readout.pointer(&ctx, Pointer::Up);

    // **The row's own ground is still the row's.** A press at the far end
    // of the same row takes the Set in hand and names no operation, which
    // is what a carry is (ADR-0265).
    readout.panel.solve();
    let row = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("the bay lists its rows")
    .row(1);
    let ground = Point::new(row.max.x - 4.0, row.center().y);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(ground)).0,
        Claim::Panel
    );
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        !matches!(did, Acted::Emitted(Some(Operation::SetFavourite { .. }))),
        "a press on the row's own ground was answered by the star in it: {did:?}"
    );
    readout.pointer(&ctx, Pointer::Up);
}

/// A press on one of the Library bay's two filter fields, through the window
/// loop's own routing — the half of the badge ADR-0213 makes a badge mean, one
/// row under [`a_press_on_a_scope_chip_names_the_library_the_bay_reads`].
///
/// `karakuri-console`'s `tests/library.rs` asserts everything up to the
/// operation with no window anywhere: where the two fields are, that each steps
/// its own cycle, that the operation names where it arrived and carries the
/// other field untouched, and that nothing between them takes a press. This is
/// the half that says an operator reaches it — a control demonstrated in that
/// crate and never wired here would pass there and be a lie the page tells on
/// its own authority, which is exactly what `press_handler` was written after.
///
/// What is asserted is the whole press. The operation that leaves is `ListSets`
/// carrying the step; `View::narrow` has been called, so the field the panel
/// draws next frame reads the new value; and the library cursor is back at the
/// top, because the listing under a narrower filter is one this cursor has
/// never seen.
///
/// Both fields, because the operation carries both halves: the press on `layer`
/// has to come back with the `holds` the press before it set, and a route that
/// rebuilt the operation from one field would lose it.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_a_filter_field_asks_the_store_for_a_narrower_listing() {
    use karakuri_console::view::Field;

    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    // A console told what libraries there are, handed the listing for the
    // one it opens on and told what that listing's Sets are made of —
    // which is what `listing` does on this side of the seam.
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    readout.view.holds = vec!["drift_shell".to_owned(), "soft_points".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));
    // **The cursor is somewhere other than the top**, so that the move back
    // to it is a move and not the state it was already in.
    assert!(readout.view.walk(1, 0..2), "the cursor did not move");

    // The box, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let field = |readout: &mut Readout, which: Field| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .field(which)
        .expect("the bay draws its filter field");
        Point::new(at.center().x, at.center().y)
    };
    // **And one kind chip's, asked of the walk that paints them**, which
    // is the same rule one band down: a chip is as wide as the word in it,
    // so where it is is `egui`'s answer and never a remembered number.
    let kind_chip = |readout: &mut Readout, which: usize| {
        readout.panel.solve();
        let bay = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows");
        let at = bay
            .kind_chips(&ctx)
            .nth(which)
            .expect("the bay draws six kind chips")
            .1;
        Point::new(at.center().x, at.center().y)
    };

    let at = field(&mut readout, Field::Holds);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::ListSets {
            holds: Some("drift_shell".to_owned()),
            layer: None,
        })),
        "the press did not reach the `holds` field"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.filters().holds,
        Some("drift_shell"),
        "the press was routed and the field it stepped does not read it"
    );
    assert_eq!(
        readout.view.cursor_row(),
        0,
        "the listing narrowed and the cursor is still pointing into the one it left"
    );

    // **And the kind chips under it, which have to carry the field
    // through**: a press on a chip names all six and says nothing about
    // `holds`, so what the console is narrowed to afterwards is the pair as
    // it now stands (ADR-0338).
    let at = kind_chip(&mut readout, 2);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::FilterLibrary {
            kinds: karakuri_operation::LibraryKinds {
                l3: true,
                ..karakuri_operation::LibraryKinds::EVERYTHING
            },
        })),
        "the press did not reach the `L3` chip, or it named something other than all six"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert!(readout.view.filters().kinds.l3);
    assert_eq!(readout.view.filters().holds, Some("drift_shell"));
}

/// A press on a strip's ground addresses the keys to that deck — the whole
/// route, through the same `Readout::pointer` a hand goes through.
///
/// # Why it is here and can be nowhere else
///
/// `karakuri-console` owns both halves and cannot put them together:
/// `input::claim` there says a press on a strip is the panel's, and
/// `Mixer::select` says which deck it names, and nothing in that crate joins
/// the two. This file's press arm is the join, and the defect this was written
/// against lived exactly in the gap: `on_strip` asked four questions where the
/// bay has five, so every press on a strip's ground was routed to `egui`, the
/// `(Pointer::Down, Claim::Panel)` arm never ran, and the `bay.select(at)` call
/// at the end of it was unreachable — while `operations.html`'s *"click a
/// strip"*, `console.html`'s strip tip, `Mixer::select` and that call all said
/// it worked.
///
/// Deleting `|| bay.select(p).is_some()` from `input::on_strip` is the
/// injection this was watched to fail against: the claim comes back `Egui` and
/// the press asks for nothing.
///
/// The point is the strip's name box, which is the affordance's own words — *"a
/// press anywhere on this strip that no knob under the pointer claimed"* — and
/// the four that could have claimed it are asked here so that the press under
/// test is the leftover rather than a chip.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_a_strips_ground_selects_that_deck() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.mixer = (0..4)
        .map(|slot| view::Strip {
            name: format!("slot{slot}"),
            tally: view::Tally::Live,
            requested: view::Tally::Live,
            gain: 0.5,
            gain_to: None,
            opacity: 0.5,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
        })
        .collect();
    assert_eq!(
        readout.view.selection(),
        0,
        "the selection is not deck A, so a press naming deck C could be naming the \
         selection it started on"
    );

    // The rectangle, off the derivation that draws it — the rule the whole
    // of `input` is written to, and the reason a test presses where the
    // paint painted.
    let bay = mixer_bay(&ctx, readout.panel.layout(), &readout.view.mixer)
        .expect("the mixer bay draws its strips");
    let name = bay.strip(2).name;
    let at = Point::new(name.center().x, name.center().y);
    assert!(
        bay.grab(at).is_none()
            && bay.blend(at).is_none()
            && bay.tally(at).is_none()
            && bay.mask(at).is_none(),
        "the name box is one of the four controls inside the column, so this press is \
         not the leftover the selection is made of"
    );

    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(at)).0,
        Claim::Panel,
        "a press on a strip went to egui, so the panel's own press arm never runs"
    );
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel, "a press on a strip went to egui");
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SelectDeck { deck: 2 })),
        "the press did not address the keys to the deck the strip is"
    );

    // **The selection moves where the operation is performed**, which is
    // `pointed` and not the press arm: `SelectDeck` writes no record, so
    // the surface that emits it is what performs it (ADR-0198).
    assert_eq!(
        readout.view.selection(),
        0,
        "the press moved the pointer itself"
    );
    assert!(pointed(&mut readout.view, &Operation::SelectDeck { deck: 2 }).is_some());
    assert_eq!(readout.view.selection(), 2);
}

/// A Set dragged from a library row onto a mixer strip loads the strip it was
/// let go over — the whole gesture, through the same `Readout::pointer` a hand
/// goes through.
///
/// # Why it is here and can be nowhere else
///
/// `karakuri-console` has both halves of the gesture and cannot put them
/// together: `carry.rs` there presses the model and the view directly, and
/// hands the destination in itself, because that crate has no press handler to
/// ask. The property that matters is which *moment* resolves the deck, and that
/// is this file's: the press is over the Library bay, where there is no strip
/// at all, and the release is over one. So a destination taken at the press
/// names nothing and the drop is cancelled, and a destination taken at the
/// release names the strip under the hand.
///
/// Deleting the `Mixer::dropped` ask from the release arm is the injection this
/// was watched to fail against, and moving it into the press arm is the second
/// — the first answers `Nowhere` for every drop and the second answers it for
/// every drop that began in the library, which is all of them.
///
/// # What it asserts, in the order a hand does it
///
/// 1. A press on the third row is the panel's, and it emits nothing: half a
/// gesture names one operand. 2. The cursor mark follows the hand, and
/// `Acted::Pointed` is the press saying so. It is no longer the whole of what
/// this console draws for a carry — the rectangle under the pointer is ringed
/// and the pointer is a grab, which is `View::draw`'s and is held by
/// `karakuri-console/tests/carry.rs`; the mock still draws no ghost. What is
/// owed on that answer when a reading is open is
/// [`a_carry_that_moves_the_cursor_re_reads_the_row_it_arrived_at`]; here it is
/// the mark alone, and `Acted::Nothing` in its place would be a press that
/// moved the cursor and told nobody. 3. Every move on the way is
/// `Acted::Nothing`, over two strips that are not the one it lands on. 4. The
/// release over strip C asks for `LoadSet` naming deck C and the Set from row 2
/// — not the selection, which is deck A throughout, and not the row the cursor
/// started on. 5. A second carry let go over nothing asks for nothing, which is
/// the outcome no other drag on this panel has.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_drop_on_a_strip_loads_the_strip_it_was_let_go_over() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec![
        "drift_night".to_owned(),
        "lattice_veil".to_owned(),
        "glass_shell".to_owned(),
    ];
    readout.view.mixer = (0..4)
        .map(|slot| view::Strip {
            name: format!("slot{slot}"),
            tally: view::Tally::Live,
            requested: view::Tally::Live,
            gain: 0.5,
            gain_to: None,
            opacity: 0.5,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
        })
        .collect();
    assert_eq!(
        readout.view.selection(),
        0,
        "the selection is not deck A, so a drop naming deck C could be naming the selection"
    );

    // The rectangles, off the derivations that draw them — the rule the
    // whole of `input` is written to, and the reason a test presses where
    // the paint painted.
    let row = |readout: &mut Readout, index: usize| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .row(index);
        Point::new(at.center().x, at.center().y)
    };
    let strip = |readout: &mut Readout, deck: u8| {
        readout.panel.solve();
        let at = mixer_bay(&ctx, readout.panel.layout(), &readout.view.mixer)
            .expect("the mixer bay draws its strips")
            .selected(deck)
            .expect("a strip for the deck");
        Point::new(at.center().x, at.center().y)
    };

    // 1 and 2: the press takes row 2 in hand, asks for nothing, and moves
    // the mark to the row the hand is on.
    let at = row(&mut readout, 2);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel, "a press on a library row went to egui");
    assert_eq!(
        did,
        Acted::Pointed,
        "the press asked for an operation, or moved the mark without answering that it did"
    );
    assert_eq!(
        readout.view.cursor_row(),
        2,
        "the mark did not follow the hand to the row it took"
    );

    // 3: nothing is emitted on the way, including over two strips it does
    // not land on.
    for over in [strip(&mut readout, 0), strip(&mut readout, 1)] {
        let (claim, did) = readout.pointer(&ctx, Pointer::Moved(over));
        assert_eq!(claim, Claim::Panel, "the carry lost its claim");
        assert_eq!(
            did,
            Acted::Nothing,
            "a move with a Set in hand asked for something"
        );
    }

    // 4: and the release names the strip it is over.
    let onto = strip(&mut readout, 2);
    readout.pointer(&ctx, Pointer::Moved(onto));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Emitted(Some(Operation::LoadSet {
            deck: 2,
            set: "glass_shell".to_owned(),
        })),
        "the drop did not name the strip it was let go over"
    );
    assert!(!readout.panel.dragging(), "the carry is still in hand");

    // 5: and one let go over nothing asks for nothing at all.
    let at = row(&mut readout, 0);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Pointed);
    let away = Point::new(-40.0, -40.0);
    readout.pointer(&ctx, Pointer::Moved(away));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Nothing,
        "a drop over nothing asked for a load"
    );
}

/// A Set let go on a deck preview cell loads that cell's deck, through the same
/// `Readout::pointer` a hand goes through — and a cell whose letter names no
/// slot loads nothing.
///
/// # Why it is here and not in `karakuri-console`
///
/// `carry.rs` there asks the two bays itself and hands the destination to
/// `Panel::released`. What this file owns is that the release asks the second
/// bay at all: the press handler resolved the drop against `Mixer::dropped`
/// alone until ADR-0273, so a carry that crossed to the centre column and let
/// go on a cell was answered `Nowhere` — the panel drawing a ring round a
/// rectangle the release then declined to use. Deleting the
/// `ProgramBay::dropped` ask from the release arm is the injection this was
/// watched to fail against.
///
/// # And the slot count is asked with it
///
/// The deck here has three slots and the row is four cells, so cell D is drawn
/// with nothing behind the letter on it. A release there names no deck, which
/// is the refusal `3` already gets from the keyboard — `pointed`, off the same
/// `View::mixer` length. Passing `DECKS` instead of that length is the second
/// injection, and it asks for `LoadSet { deck: 3 }` on a deck that has no slot
/// 3.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_drop_on_a_preview_cell_loads_the_deck_its_letter_names() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    // The Program bay's cells are read from the bit `rearrange` writes, so
    // a frame's own first act is what puts them anywhere at all.
    view::rearrange(&mut readout.panel, readout.view.canvas);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec![
        "drift_night".to_owned(),
        "lattice_veil".to_owned(),
        "glass_shell".to_owned(),
    ];
    readout.view.mixer = (0..3)
        .map(|slot| view::Strip {
            name: format!("slot{slot}"),
            tally: view::Tally::Live,
            requested: view::Tally::Live,
            gain: 0.5,
            gain_to: None,
            opacity: 0.5,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
        })
        .collect();
    assert_eq!(
        readout.view.selection(),
        0,
        "the selection is not deck A, so a drop naming deck B could be naming the selection"
    );

    let row = |readout: &mut Readout, index: usize| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .row(index);
        Point::new(at.center().x, at.center().y)
    };
    let cell = |readout: &mut Readout, deck: usize| {
        readout.panel.solve();
        let cells = preview_rects(readout.panel.layout(), readout.view.canvas)
            .expect("the preview row is on screen");
        assert_eq!(cells.len(), DECKS, "the row is not four cells");
        let at = cells[deck];
        Point::new(at.center().x, at.center().y)
    };

    // Row 1 onto cell B: not the selection, not the row's index as a deck.
    let at = row(&mut readout, 1);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Pointed);
    let onto = cell(&mut readout, 1);
    readout.pointer(&ctx, Pointer::Moved(onto));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Emitted(Some(Operation::LoadSet {
            deck: 1,
            set: "lattice_veil".to_owned(),
        })),
        "the drop did not name the cell it was let go over"
    );
    assert!(!readout.panel.dragging(), "the carry is still in hand");

    // And cell D, which this deck has no slot for, loads nothing.
    let at = row(&mut readout, 2);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Pointed);
    let onto = cell(&mut readout, 3);
    readout.pointer(&ctx, Pointer::Moved(onto));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Nothing,
        "a drop on the fourth cell of a three-slot deck asked for a load"
    );
}

/// A carry that moves the library cursor re-reads the row it arrived at, which
/// is the rule the cursor states rather than the keyboard: *"the reading
/// follows the cursor: a move with one open is a read of the row it arrived
/// at"* (`karakuri-console/src/view.rs`, `View::reading_open`).
///
/// # The defect it was written for
///
/// `Readout::took` discarded `View::point_at`'s `moved`. So taking a row in
/// hand while a reading was open on a different row moved the cursor off that
/// row, `View::opened` answered `None` because the row under the cursor was no
/// longer the Set the reading was of, and the block disappeared — for the rest
/// of the run, because nothing on this route ever walks the cursor back. The
/// arrow keys never had it: they re-read on `moved && reading_open()`.
///
/// # Why it is here and can be nowhere else
///
/// It needs all three of a press handler, a store on a disk, and the glue
/// between them, and this file is the only place that has any two. `carry.rs`
/// in `karakuri-console` presses the bay and the view directly and that crate
/// reaches no disk at all (ADR-0156), so the half it can hold is
/// `the_row_a_hand_takes_is_the_row_the_cursor_marks` — that `point_at` answers
/// the move — and not that anything acts on the answer.
///
/// # What it asserts, and what each one fails against
///
/// 1. The press answers `Acted::Pointed`, which is the whole of what
/// `Readout::took` can do about it: the readout holds no store, so the press
/// says *the cursor moved* and the caller reads the file. A `took` that drops
/// the `bool` again answers `Acted::Nothing` here. 2. The block is gone until
/// it is re-read, which is the defect itself, asserted so that step 3 cannot
/// pass by the reading never having moved. 3. `read_reading` — the call the
/// window loop makes on that answer — puts the reading under the row the hand
/// took, naming that row's Set. 4. A press on the row the cursor is already on
/// answers `Acted::Nothing`, so a carry that moved nothing costs no file read.
///
/// What it cannot see is that the window loop makes the call, because `winit`
/// cannot be asked for an `ActiveEventLoop` outside its own loop and an event
/// handler is not something a test can drive — `Readout::pointer`'s own doc.
/// `App::window_event`'s carry arm calls [`reread_if_open`] with
/// `matches!(acted, Acted::Pointed)`, the same function
/// [`reread_if_open_re_reads_only_on_a_move_with_a_reading_open`] presses
/// directly, below — this test is the two halves either side of that call, and
/// neither reaches the call itself.
///
/// A CPU test: a store is a directory and a `Readout` takes no device.
#[test]
fn a_carry_that_moves_the_cursor_re_reads_the_row_it_arrived_at() {
    let ctx = drawn_once();
    let root = scratch_dir("carry-reading");
    let store = Store::open(&root).expect("a store to read");
    store.write_set("drift_night", &[]).expect("a Set to read");
    store.write_set("lattice_veil", &[]).expect("a Set to read");

    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));

    // The rectangles, off the derivation that draws them — and the open
    // reading goes in with them, because the block is drawn among the rows
    // and the row below it is somewhere else while it is down.
    let row = |readout: &mut Readout, index: usize| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .row(index);
        Point::new(at.center().x, at.center().y)
    };

    // The reading is opened the way the window loop opens it, on the row
    // the cursor starts on.
    read_reading(&mut readout.view, &root);
    let open = readout.view.opened().expect("a reading on the first row");
    assert_eq!((open.at, open.reading.id.as_str()), (0, "drift_night"));

    // 1: the press on the other row takes it in hand and says the mark
    // moved.
    let at = row(&mut readout, 1);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel, "a press on a library row went to egui");
    assert_eq!(
        did,
        Acted::Pointed,
        "the carry moved the library cursor and answered nothing, so the reading open on \
         the row it left has nowhere to be drawn and nothing to bring it back"
    );
    assert_eq!(readout.view.cursor_row(), 1);

    // 2: and until the answer is acted on, the block is drawn nowhere.
    assert!(
        readout.view.opened().is_none(),
        "the reading is still drawn on a row the cursor has left"
    );
    assert!(readout.view.reading_open(), "the reading was put away");

    // 3: what the window loop does with that answer.
    let said = read_reading(&mut readout.view, &root);
    assert!(
        said.contains("lattice_veil"),
        "the re-read named a row the hand is not on: `{said}`"
    );
    let open = readout
        .view
        .opened()
        .expect("the reading followed the cursor to the row the hand took");
    assert_eq!((open.at, open.reading.id.as_str()), (1, "lattice_veil"));
    readout.pointer(&ctx, Pointer::Up);

    // 4: and a press on the row the cursor is already on moves nothing,
    // so it owes no read at all.
    let at = row(&mut readout, 1);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Nothing,
        "a press on the row the cursor was already on asked for a re-read of it"
    );
    readout.pointer(&ctx, Pointer::Up);

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The filter row narrows what the bay lists, through the summary.
///
/// The other half of the same press: `a_press_on_a_filter_field_…` says the
/// operation reaches `View::narrow`, and this says the listing that comes back
/// afterwards is a narrower one — which is the whole point, and was impossible
/// while this side asked `Store::list_sets` for names.
///
/// A procedure is a row of `all` and of `presets`, with its kind on it — and of
/// neither `my sets` nor `folder` (ADR-0338, decision 1).
///
/// A CPU test: two tiers on a disk, a `View`, and no window.
#[test]
fn the_two_tiers_list_procedures_beside_sets_and_two_scopes_do_not() {
    use karakuri_operation::LibraryKinds;
    use karakuri_store::hash::Hash;
    use karakuri_store::ndjson::Line;
    use karakuri_store::record::Layer as Written;

    let root = scratch_dir("procedure-listing");
    let store = Store::open(&root).expect("a store to list");
    store
        .write_set(
            "night01",
            &[Line::new(Record::Slot {
                at: karakuri_store::record::NodeAddress {
                    layer: Written::L1,
                    index: 0,
                },
                name: Some("drift_shell".to_owned()),
                proc_hash: Hash::of(b"drift_shell"),
            })],
        )
        .expect("a Set to list");
    std::fs::write(
        root.join(Store::PROCEDURES).join("orbit_wide.kir"),
        "proc orbit_wide {\n  kind L3\n}\n",
    )
    .expect("a kept procedure");
    let shipped = root.join("shipped");
    std::fs::create_dir_all(&shipped).expect("mkdir");
    std::fs::write(shipped.join("beat_glow.kset"), "{}\n").expect("a shipped Set");
    std::fs::write(shipped.join("tunnel_eye.kir"), "  kind L3\n").expect("a shipped procedure");
    let presets = karakuri_environment::places::presets(Some(&shipped))
        .expect("the root resolves")
        .expect("a root");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();

    // `all`: the Set and the kept procedure, and the procedure carries the
    // kind its `kind` line declares while the Set carries its slots'.
    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        view.library.contains(&"orbit_wide".to_owned())
            && view.library.contains(&"night01".to_owned()),
        "`all` lists {:?} — {said}",
        view.library
    );
    let at = view
        .library
        .iter()
        .position(|row| row == "orbit_wide")
        .expect("the procedure is a row");
    assert_eq!(
        view.kinds[at],
        RowKind {
            badges: vec![karakuri_operation::Layer::L3],
            procedure: true
        },
        "the procedure row's badge is not its kind"
    );
    let at = view
        .library
        .iter()
        .position(|row| row == "night01")
        .expect("the Set is a row");
    assert_eq!(
        view.kinds[at],
        RowKind {
            badges: vec![karakuri_operation::Layer::L1],
            procedure: false
        },
        "the Set row's badges are not the layers its slots fill"
    );

    // `presets`: the shipped Set and the shipped procedure, in name order.
    assert!(view.select_scope(Scope::Presets));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["beat_glow".to_owned(), "tunnel_eye".to_owned()]
    );
    assert!(view.kinds[1].procedure, "the shipped `.kir` is not a row");

    // `my sets` lists no procedure, because a star is refused on anything
    // `sets/` does not hold; `folder` lists none, because a folder row is a
    // take and nothing takes a bare `.kir` in.
    assert!(view.select_scope(Scope::MySets));
    listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        !view.library.contains(&"orbit_wide".to_owned()),
        "`my sets` lists a procedure: {:?}",
        view.library
    );
    assert!(view.select_scope(Scope::Folder));
    listing(&mut view, &root, Some(&presets), Some(&shipped), None);
    assert!(
        !view.library.contains(&"tunnel_eye".to_owned()),
        "`folder` lists a procedure: {:?}",
        view.library
    );

    // **The kind chips narrow by OR, and none on is everything.**
    assert!(view.select_scope(Scope::AllSets));
    let cameras = LibraryKinds {
        l3: true,
        ..LibraryKinds::EVERYTHING
    };
    assert!(view.narrow(None, cameras));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["orbit_wide".to_owned()],
        "`L3` on lists {:?}",
        view.library
    );
    let sets_only = LibraryKinds {
        sets: true,
        ..LibraryKinds::EVERYTHING
    };
    assert!(view.narrow(None, sets_only));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["night01".to_owned()],
        "`SET` on lists {:?}",
        view.library
    );
    assert!(view.narrow(
        None,
        LibraryKinds {
            l3: true,
            sets: true,
            ..LibraryKinds::EVERYTHING
        }
    ));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library.len(),
        2,
        "an OR of two lists {:?}",
        view.library
    );
    assert!(view.narrow(None, LibraryKinds::EVERYTHING));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library.len(),
        2,
        "none on is not everything: {:?}",
        view.library
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// What it narrows is the store's own listing, which is `all` and is what *List
/// what the store holds* lists. `my sets` is that listing starred (ADR-0299),
/// so the same retain applies to it and the row is not a control over one chip.
///
/// It is the same retain the MCP tool applies, over the same
/// `setfile::summarise`, which is what keeps one operation from being answered
/// two ways by two surfaces.
///
/// A CPU test: a store, a `View`, and no window.
#[test]
fn the_filter_row_narrows_the_stores_listing_through_the_summary() {
    use karakuri_store::hash::Hash;
    use karakuri_store::ndjson::Line;
    use karakuri_store::record::Layer as Written;

    let root = scratch_dir("filter-listing");
    let store = Store::open(&root).expect("a store to list");
    let slot = |layer: Written, name: &str| {
        Line::new(Record::Slot {
            at: karakuri_store::record::NodeAddress { layer, index: 0 },
            name: Some(name.to_owned()),
            proc_hash: Hash::of(name.as_bytes()),
        })
    };
    store
        .write_set("night01", &[slot(Written::L1, "drift_shell")])
        .expect("a Set to list");
    store
        .write_set("veil02", &[slot(Written::L4, "soft_points")])
        .expect("a second Set to list");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert_eq!(view.scope(), Some(Scope::AllSets));

    // Unnarrowed: both Sets, and the candidates are what their nodes are
    // called — sorted, deduplicated, and read off the *unfiltered* listing.
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library.len(), 2, "the bay lists {:?}", view.library);
    assert_eq!(
        view.holds,
        vec!["drift_shell".to_owned(), "soft_points".to_owned()],
        "the `holds` field can be stepped to {:?}",
        view.holds
    );
    assert!(!said.contains("holding"), "{said}");

    // Narrowed by what a node is called.
    assert!(view.narrow(
        Some("drift_shell"),
        karakuri_operation::LibraryKinds::EVERYTHING
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library, vec!["night01".to_owned()], "{said}");
    assert!(
        said.contains("1 of 2") && said.contains("drift_shell"),
        "{said}"
    );

    // **A filter that matched nothing is a different nothing from an empty
    // store**, and the line says which: the store is not empty, and what to
    // do about it is press a field rather than save a Set.
    assert!(view.narrow(
        Some("drift_shell"),
        karakuri_operation::LibraryKinds {
            l3: true,
            ..karakuri_operation::LibraryKinds::EVERYTHING
        },
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert!(view.library.is_empty(), "the bay lists {:?}", view.library);
    assert!(
        said.contains("none of the 2 Sets here")
            && !said.contains(why_nothing(Scope::AllSets, false, false)),
        "{said}"
    );

    // **And the same retain applies to `my sets`**, which is this listing
    // starred: star one Set, mark the subset, and the filter that named
    // the other one leaves it with nothing — the narrowing is over what
    // the store holds and not over which chip is marked.
    assert!(view.narrow(None, karakuri_operation::LibraryKinds::EVERYTHING));
    favourite(
        &root,
        Asked::Operator,
        &Operation::SetFavourite {
            id: "night01".to_owned(),
            favourite: true,
        },
    )
    .expect("`favourite` answered nothing for a `SetFavourite`");
    assert!(view.select_scope(Scope::MySets));
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library, vec!["night01".to_owned()], "{said}");
    assert!(view.narrow(
        Some("soft_points"),
        karakuri_operation::LibraryKinds::EVERYTHING
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert!(
        view.library.is_empty(),
        "`my sets` lists {:?} under a filter that names the Set that is not starred",
        view.library
    );
    assert!(said.contains("none of the 2 Sets here"), "{said}");

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A press on the Program bay's `solo` pill, through the window loop's own
/// routing.
///
/// `karakuri-console`'s `tests/solo_pill.rs` and `tests/vocabulary.rs` assert
/// everything up to the operation with no window anywhere; this is the half
/// ADR-0213 makes the badge mean — *"the row is claimed the day a person who
/// launched the instrument can perform that operation from the panel in front
/// of them"* — and a control demonstrated in that crate and never wired here
/// would pass there and be a lie the page tells.
///
/// Both directions, because the pill is both. A solo takes every other control
/// off the screen, so the pill is the only thing left to press and the undo has
/// to come from it. What is asserted is the round trip an operator makes: the
/// picture is one region among many, a click on the pill leaves it holding the
/// window, and a click on the same pill — found again where it is now drawn,
/// because the solo moved every rectangle on the console — puts everything
/// back.
#[test]
fn a_press_on_the_solo_pill_solos_the_picture_and_undoes_it() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let picture = readout
        .panel
        .layout()
        .find("program-view")
        .expect("program-view");
    let library = readout.panel.layout().find("library").expect("library");
    // The capsule, asked of the derivation that draws it rather than
    // remembered — which is the rule the whole of `input` is written to,
    // and here it is load-bearing twice over.
    let pill = |readout: &mut Readout| {
        readout.panel.solve();
        let head = program_head(&ctx, readout.panel.layout(), Open::CLOSED)
            .expect("the bay draws its pill");
        (
            Point::new(head.solo.center().x, head.solo.center().y),
            head.soloed,
        )
    };

    let (at, soloed) = pill(&mut readout);
    assert!(!soloed, "something is soloed before anything was pressed");
    assert!(
        readout.panel.layout().visible(library),
        "the library is off the screen already, so soloing would prove nothing"
    );

    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Operated(Outcome::Soloed(picture)),
        "the press did not reach the pill"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    readout.panel.solve();
    assert!(
        !readout.panel.layout().visible(library),
        "the picture is soloed and the library is still on the screen"
    );

    // And the same pill, where it is now, undoes it.
    let (at, soloed) = pill(&mut readout);
    assert!(soloed, "the picture is soloed and the pill does not say so");
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Operated(Outcome::Unsoloed { was: true }),
        "the pill did not undo the solo it made"
    );
    readout.pointer(&ctx, Pointer::Up);
    readout.panel.solve();
    assert!(
        readout.panel.layout().visible(library),
        "undoing the solo left the library folded"
    );
}
/// A control's operation becomes the record every other surface's control ends
/// in, and this is the half of that which needs no device.
///
/// This test is older than the conversion it now checks, and that is the point
/// of it. It was written against the hand-written `record` this file used to
/// carry, asserting term for term what `karakuri-cli`'s `mix::gain_record` and
/// `mix::opacity_record` already wrote. That function is deleted and
/// [`written`] answers instead (ADR-0185's promise, kept where ADR-0194 put the
/// home) — every expectation below is unchanged, so if the crate's conversion
/// disagreed with the one that was deleted, this is what says so.
///
/// `mix::gain_record` is deleted too, by the same record and for the stronger
/// reason: the conversion *is* the derivation now, and two of them is the drift
/// `mix.rs` exists to end. The comments below name it where it stood, because
/// what this test compares against is the record that function wrote rather
/// than the function.
///
/// And the other direction: an operation this program has no control for writes
/// no record here either, and the answer says *which* kind of nothing rather
/// than a bare `None` — which is the whole of what the three answers buy.
#[test]
fn a_controls_operation_becomes_the_record_the_cli_would_have_written() {
    assert_eq!(
        only_record(&Operation::SetGain {
            deck: 2,
            gain: 0.75
        }),
        // What `mix::gain_record(2, 0.75)` wrote, before ADR-0194 deleted
        // it in favour of this conversion.
        Record::Gain {
            slot: DeckSlot(2),
            value: 0.75
        }
    );
    assert_eq!(
        only_record(&Operation::SetOpacity {
            deck: 0,
            opacity: 0.25
        }),
        // `mix::opacity_record(0, 0.25)`.
        Record::Opacity {
            slot: DeckSlot(0),
            value: 0.25
        }
    );
    // **Every mode of the cycle, because a chip that emits three
    // operations has three records to write** — and the mode is a wire
    // name, so a mode that reached `Record::Blend` misspelled would be
    // refused by the engine on the way back rather than here.
    for (deck, blend) in BlendMode::ALL.into_iter().enumerate() {
        let deck = deck as u8;
        assert_eq!(
            only_record(&Operation::SetBlendMode { deck, blend }),
            // `mix::blend_record(deck, blend)`.
            Record::Blend {
                slot: DeckSlot(deck),
                mode: blend.name().to_owned(),
            },
            "`{}` did not become the record `mix::blend_record` writes",
            blend.name()
        );
        // And the engine reads its own name back, which is what says the
        // two lists are the same three words rather than two spellings of
        // them.
        assert_eq!(
            Blend::from_name(blend.name()),
            Some(blend_mode_back(blend)),
            "the engine does not know the vocabulary's `{}`",
            blend.name()
        );
    }

    // **Every residency of the cycle**, for the same reason as the blend:
    // one chip emitting three operations has three records to write. The
    // spelling is the wire's — `mix::residency_wire_name`'s three words,
    // which are deliberately not the status line's `LIVE`/`prim`/`park`
    // and not the chip's `live`/`prim`/`alloc` either, so a record written
    // in the chip's vocabulary would decode as nothing at all.
    for (deck, (residency, level)) in [
        (karakuri_operation::Residency::Live, "live"),
        (karakuri_operation::Residency::Priming, "priming"),
        (karakuri_operation::Residency::Allocated, "allocated"),
    ]
    .into_iter()
    .enumerate()
    {
        let deck = deck as u8;
        assert_eq!(
            only_record(&Operation::SetResidency { deck, residency }),
            // `mix::residency_record(deck, residency)`.
            Record::Residency {
                slot: DeckSlot(deck),
                level: level.to_owned(),
            },
            "{residency:?} did not become the record `mix::residency_record` writes"
        );
        // And it reads back as the level it named, which is what says the
        // two spellings are one list rather than two.
        assert_eq!(
            mix::parse_residency(level),
            Some(residency_back(residency)),
            "the wire spelling `{level}` does not come back as {residency:?}"
        );
    }

    // The vocabulary is larger than what this program reaches: five controls
    // writing five records. A record invented for the other 45 would be
    // somebody deciding what they mean — and the answer is now *which*
    // nothing rather than `None`, because a surface's own state and a
    // record nobody can write yet are not the same silence.
    assert_eq!(
        written(&Operation::Solo { region: None }, &Current::default()),
        Written::Silent(Silent::Surface)
    );
    assert_eq!(
        written(&Operation::SelectDeck { deck: 1 }, &Current::default()),
        Written::Silent(Silent::Surface)
    );
}

/// The one record an operation writes, for the tests that know there is exactly
/// one.
///
/// For the four whose record needs no reading at all, which is where
/// `Current::default()` — *I read nothing* — is the honest answer. A conversion
/// that answered anything but a single record for one of those four is this
/// file's assumption breaking rather than a test needing a helper, which is why
/// the panic says so.
///
/// The mask's operation is not one of them and must not be passed here: its
/// record is written out of the operation *and* a reading of the running mask
/// (ADR-0201), so it would come back `Owed(NotRead)` and this would panic —
/// correctly, and saying which operation. What the mask's tests hand in is a
/// reading, through [`reading`] where there is a deck and by hand where there
/// is not.
pub(super) fn only_record(operation: &Operation) -> Record {
    match written(operation, &Current::default()) {
        Written::Records(records) if records.len() == 1 => records.into_iter().next().unwrap(),
        other => panic!(
            "a control's operation did not write exactly one record: \
             {operation:?} -> {other:?}"
        ),
    }
}

/// A refused wipe says which refusal it was and where the next attempt is made,
/// rather than *refused*.
///
/// `karakuri_console::view::Go` answers which of the two it is because the
/// console is what can see it; the sentence is this window's, and
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)
/// is what it owes: the constraint and the numbers, never a bare no. A press on
/// `go` that printed nothing would read exactly like a press on the card beside
/// it, which is the failure the whole `Go` type exists to prevent.
///
/// The two are asserted to be different sentences, for
/// [`an_operation_whose_record_is_owed_is_said_rather_than_swallowed`]'s reason
/// one test up: a window that printed one line for both would tell an operator
/// with four decks and no shape that they need a second deck.
///
/// Not word for word. What has to hold is that each names what would have to
/// change — the shape pill for one, a second deck for the other — and that the
/// count is in the one whose count is the constraint.
#[test]
fn a_refused_wipe_says_which_refusal_it_was_and_where_to_go_next() {
    let no_shape = refusal(&Go::NoShape, 4);
    assert!(
        no_shape.contains("shape"),
        "the refusal for an unchosen shape does not say what is missing: `{no_shape}`"
    );
    assert!(
        no_shape.contains("pill"),
        "the refusal for an unchosen shape does not say where one is picked, so an \
         operator is told no and not told where to go: `{no_shape}`"
    );

    let alone = refusal(&Go::NoOtherDeck, 1);
    assert!(
        alone.contains('1') && alone.contains("strip"),
        "the refusal for a mixer with nowhere to wipe from does not carry the count \
         that is the constraint: `{alone}`"
    );
    assert!(
        alone.contains("deck"),
        "the refusal for a mixer with nowhere to wipe from does not say what would \
         have to change: `{alone}`"
    );
    // The plural moves with the count, which is this file's rule for every
    // sentence that carries one.
    assert!(refusal(&Go::NoOtherDeck, 0).contains("0 strips"));
    assert!(refusal(&Go::NoOtherDeck, 1).contains("1 strip,"));

    assert_ne!(
        no_shape, alone,
        "a wipe with no shape chosen and a wipe with nowhere to come from came out of \
         this window as the same sentence"
    );
}

/// An operation whose record nobody can write yet does not silently do nothing,
/// and it is not the same event as one that writes no record on purpose.
///
/// This is what the third answer is *for*, and the cheap harness is the one
/// that treats *not `Records`* as a no-op. A press that emitted `TapBeat` would
/// then look exactly like a press that emitted `SelectDeck` — nothing printed
/// and nothing moved — and an operator would read the first as *the tap did not
/// take* when what happened is *nobody has decided what a tap writes* (`Owed`
/// is a question, not an error: ADR-0194).
///
/// The operation this names has had to change twice, which is the test doing
/// what it says on the line below. It was `FadeDeck`, which stopped being owed
/// the day the transition settings became a reading; it was then `Wipe`, which
/// stopped the day the front shape went over with them and the soft edge turned
/// out to be the arriving deck's. It is now `Operation::TapBeat` — and that one
/// is a different shape rather than the next in a queue: what a tap owes is the
/// beat lock's answer and not a value any surface holds, so no reading added to
/// `Current` closes it.
///
/// The second half has been re-pointed once, and the reason is worth reading.
/// It was `FadeDeck`, on the grounds that this panel held no transition
/// settings to hand over — and the day the transition row was wired into this
/// window that stopped being true, without this test going red: it builds a
/// `Current::default()` by hand, so it went on passing while its own sentence
/// had become false. That is the failure mode `docs/contributing.md` §3 is
/// about, met from the wrong side.
///
/// It is `Operation::SetMaskPosition` against a reading nobody took, and the
/// second half stopped being about this window on 2026-09-10. Its record is
/// `Record::Mask` written whole and it needs the shape, the angle and the
/// softness it does not name (ADR-0201). Until that day [`reading`]'s mask arm
/// answered for `SetMaskShape` and for a wipe's arriving deck and for nothing
/// else, so this *was* a gap in this file — which is what ADR-0334 recorded and
/// ADR-0341 closed with one arm.
///
/// What it asserts now is the third answer itself, which is why the operation
/// did not have to change a third time: handed a `Current` with no mask in it —
/// a reading that was not taken, whatever the reason — the conversion says
/// *which* reading is missing rather than sending a front back to wherever a
/// default put it, mid-wipe. That the real reading is now taken is asserted
/// where there *is* a deck,
/// `gpu::the_go_pill_runs_a_wipe_against_the_settings_the_row_is_on`, which is
/// the half a test with no device cannot make.
///
/// Neither sentence is asserted word for word. What has to hold is that the
/// window says something, that it names the operation and the reason, and that
/// the two answers are two different sentences.
#[test]
fn an_operation_whose_record_is_owed_is_said_rather_than_swallowed() {
    // Owed, and `NotSettled` is the reason: a tap's record is the beat
    // lock's answer — a tapped tempo, a phase error, an output lag — and
    // none of it is a value a `Current` carries, so nobody has said what
    // it writes here.
    let tap = Operation::TapBeat;
    let owed = written(&tap, &Current::default());
    assert_eq!(
        owed,
        Written::Owed(Owed::NotSettled),
        "a tap is not owed any more — this test names the operation it does, and \
         the one it names has to still be one nobody can write"
    );
    let said = unwritten(&tap, &owed).expect(
        "a tap owes a record and this window said nothing at all — a press whose \
         record nobody has decided how to write reads, in silence, exactly like a \
         press that did not work",
    );
    assert!(
        said.contains("TapBeat") && said.contains(Owed::NotSettled.why()),
        "the window said `{said}`, which does not name both the operation and the \
         question it is waiting on"
    );

    // **And the other answer, which is a reading nobody took rather than a
    // record nobody has decided.** A mask position converts, and what it
    // needs is the rest of the mask — the shape, the angle and the soft
    // edge `Record::Mask` is written whole out of. Handed a reading with
    // no mask in it, the conversion says *which reading* was not handed
    // over rather than sending a front back to wherever a default put it,
    // and the sentence has to be a different one from the tap's above or
    // the two answers read alike. **This window took no mask for a
    // position until 2026-09-10** and that was the gap this half named;
    // it takes one now (ADR-0341), so what is left here is the third
    // answer itself, asserted against a `Current` built by hand.
    let front = Operation::SetMaskPosition {
        deck: 1,
        position: 0.5,
    };
    let unread = written(&front, &Current::default());
    assert_eq!(
        unread,
        Written::Owed(karakuri_operation_record::Owed::NotRead(
            karakuri_operation_record::Reading::Mask
        )),
        "a mask position with no mask handed in came back with something \
         other than the reading it is missing — a default here is a shape and an \
         angle nobody chose written over the ones a deck is wearing"
    );
    let told = unwritten(&front, &unread).expect(
        "a mask position this window cannot write said nothing at all, so a control \
         that emitted one would read exactly like a control that did not work",
    );
    assert!(
        told.contains("SetMaskPosition")
            && told.contains(Owed::NotRead(karakuri_operation_record::Reading::Mask).why()),
        "the window said `{told}`, which does not name both the operation and the \
         reading it did not get"
    );
    // **And the one it replaced is not owed any more**, which is the half
    // that would have caught this test going quietly stale: a fade is
    // scheduled against settings this console holds now, so `FadeDeck` is
    // no longer a case of *a reading this window does not have*. If this
    // ever fails, the second half above has a candidate again and somebody
    // has to say which of the two this test is about.
    assert_ne!(
        written(
            &Operation::FadeDeck { deck: 1, to: 0.0 },
            &Current {
                transition: Some(karakuri_operation_record::Transition {
                    start: 0.0,
                    beats: 4.0,
                    curve: karakuri_operation::Curve::Smooth,
                    wipe_kind: karakuri_operation::WipeKind::None,
                    wipe_angle: 0.0,
                }),
                ..Current::default()
            }
        ),
        Written::Owed(Owed::NotRead(
            karakuri_operation_record::Reading::Transition
        )),
        "a fade handed the transition settings this window now holds is still owed \
         them, so the reading this panel supplies is not the one the conversion wants"
    );
    assert_ne!(
        told, said,
        "a reading this window forgot and a record nobody has decided how to write \
         read as the same sentence"
    );

    // Silent, and settled: which deck the keys are addressed to is a
    // surface's own state and there is nothing to write.
    let select = Operation::SelectDeck { deck: 1 };
    let silent = written(&select, &Current::default());
    assert_eq!(silent, Written::Silent(Silent::Surface));
    let settled = unwritten(&select, &silent).expect(
        "selecting a deck writes no record and the window said nothing about it \
         either, so a press on such a control would leave no trace at all",
    );
    assert!(
        settled.contains(Silent::Surface.why()),
        "the window said `{settled}`, which does not say why there is no record"
    );

    // **And the two are different sentences.** Collapsing them is the
    // failure this whole test is about at one remove: a harness that
    // printed one line for both would tell an operator that an undecided
    // fade is as settled as a deck selection.
    assert_ne!(
        said, settled,
        "a record nobody can write yet and a record nobody needs to write came out \
         of this window as the same sentence"
    );

    // A record's line is `apply`'s — it says the record *and* what the
    // deck holds afterwards — so this says nothing about that case.
    // Otherwise one press prints twice.
    let gain = Operation::SetGain { deck: 0, gain: 0.5 };
    assert_eq!(
        unwritten(&gain, &written(&gain, &Current::default())),
        None,
        "an operation that wrote a record was also announced as writing none"
    );
}

/// The deck head's two operations, as far as this program can take them without
/// a device — and they go the same distance now, which is the point.
///
/// They used to go different distances: a scrub became a record and a sync mode
/// did not, and the second half of that is what `tests/panel_column.rs`'s one
/// exemption rested on — the chip's badge stayed `plan` because an operator who
/// pressed it reached the emission and not the move. That test said the day it
/// stopped being true it would stop being true here, and this is here.
///
/// The two are still not the same conversion, and that is what the second half
/// asserts. A scrub is relative and reads the transport it moves from; a mode
/// is absolute and reads the session tempo, replacing the anchor and clearing
/// the scrub. A sync mode that came out carrying the position the slot was
/// scrubbed to would be the two conversions having been made one.
#[test]
fn the_deck_heads_two_operations_go_different_distances() {
    // **The scrub is relative, so the record is where the slot is plus
    // what was asked for.** The reading is handed in by hand here for
    // `reading`'s reason at the mask: there is no deck in this test
    // binary, and what is being checked is the arithmetic rather than the
    // read.
    let current = Current {
        transport: Some(karakuri_operation_record::Transport {
            sync: karakuri_operation::Sync::Beat,
            anchor_bpm: 128.0,
            scrub_beats: -1.5,
        }),
        ..Current::default()
    };
    // **The amount is the console's own constant**, not a figure written
    // again here: the arrow that emits it and the record that carries it
    // are one number or the panel and the deck disagree about how far a
    // press goes.
    let scrub = Operation::ScrubDeck {
        deck: 1,
        beats: SCRUB_BEATS,
    };
    assert_eq!(
        written(&scrub, &current),
        Written::Records(vec![Record::Transport {
            slot: DeckSlot(1),
            sync: "beat".to_owned(),
            anchor_bpm: 128.0,
            scrub_beats: -1.25,
        }]),
        "a press of the deck head's forward arrow, from -1.50, did not come out at -1.25 — \
         so the record is not the offset the deck holds plus the amount the arrow asks for"
    );
    // **And the reading is what makes it one**: without it the conversion
    // says so rather than starting the deck's scrub from zero, which is
    // why `reading` has an arm for this operation at all.
    assert_eq!(
        written(&scrub, &Current::default()),
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Transport)),
        "a scrub with no transport read came back with a record, which means it invented \
         the position it moved from"
    );
    // **The mode goes out as a wire name and the engine reads its own name
    // back**, which is what `apply` does with it and is the blend chip's
    // assertion one control along.
    for sync in SYNCS {
        assert_eq!(
            EngineSync::from_name(sync.name()).map(mix::sync),
            Some(sync),
            "the engine does not know the vocabulary's `{}`",
            sync.name()
        );
    }

    // **A sync mode anchors at the session tempo and starts on the
    // grid.** The reading handed in is the same one the scrub used —
    // anchored at 128 and scrubbed to -1.5 — and none of it may survive:
    // `Transport::engaged` clears the scrub because *"a slot brought back
    // to the grid should be on the grid, not on wherever it was scrubbed
    // to a song ago"*, and the anchor is the room's tempo rather than the
    // one the slot was last locked to.
    let set = Operation::SetSync {
        deck: 1,
        sync: karakuri_operation::Sync::Beat,
    };
    let engaged = Current {
        tempo: Some(126.0),
        ..current
    };
    assert_eq!(
        written(&set, &engaged),
        Written::Records(vec![Record::Transport {
            slot: DeckSlot(1),
            sync: "beat".to_owned(),
            anchor_bpm: 126.0,
            scrub_beats: 0.0,
        }]),
        "a press of the deck head's sync chip, in a room at 126 bpm, did not come out \
         anchored at 126 with the scrub cleared — either the slot's old anchor survived \
         being re-engaged, or the position it was scrubbed to did"
    );
    // **And the reading is what makes it one.** Without the tempo the
    // conversion says so rather than anchoring at a guess, which is the
    // scrub's own arrangement two assertions up and the reason `reading`
    // has an arm for this operation at all.
    assert_eq!(
        written(&set, &Current::default()),
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Tempo)),
        "a sync mode with no session tempo read came back with a record, which means the \
         tempo it anchored the deck at was invented"
    );
    let said = unwritten(&set, &written(&set, &Current::default())).expect(
        "a sync chip press with no tempo read said nothing at all — a press that reads, in \
         silence, exactly like a press that did not work",
    );
    assert!(
        said.contains("SetSync")
            && said.contains(Owed::NotRead(karakuri_operation_record::Reading::Tempo).why()),
        "the window said `{said}`, which does not name both the operation and the reading \
         it did not get"
    );
}

/// The two crates walk the sync modes in one order, which is what makes
/// `view::Pane::allows` line up with the field it fills.
///
/// [`inspector`] builds that array by mapping `EngineSync::ALL` and the console
/// reads it by indexing [`SYNCS`], so the two orders are one order or the panel
/// skips the wrong mode — silently, and only on material that refuses
/// something. Two arrays cannot be made one by a comment.
#[test]
fn the_two_crates_walk_the_sync_modes_in_one_order() {
    assert_eq!(EngineSync::ALL.len(), SYNCS.len());
    for (index, mode) in EngineSync::ALL.into_iter().enumerate() {
        assert_eq!(
            mix::sync(mode),
            SYNCS[index],
            "`EngineSync::ALL[{index}]` is `{}` and the console's `SYNCS[{index}]` is \
             `{}` — the deck head's cycle would skip the wrong mode",
            mode.name(),
            SYNCS[index].name()
        );
    }
}

/// [`tally`] the other way round, for the assertion above alone — the
/// vocabulary's residency as the engine's, so that the round trip through the
/// wire name can be compared against something.
fn residency_back(residency: karakuri_operation::Residency) -> Residency {
    match residency {
        karakuri_operation::Residency::Live => Residency::Live,
        karakuri_operation::Residency::Priming => Residency::Priming,
        karakuri_operation::Residency::Allocated => Residency::Allocated,
    }
}

/// [`blend_mode`] the other way round, for the assertion above alone — which is
/// why it is here and not beside it: nothing the program *runs* needs to go
/// this direction, and a conversion in `src` with one test as its only caller
/// would be an abstraction with no second call site.
fn blend_mode_back(blend: BlendMode) -> Blend {
    match blend {
        BlendMode::Add => Blend::Add,
        BlendMode::Over => Blend::Over,
        BlendMode::Max => Blend::Max,
    }
}

/// Anything that makes texels this frame keeps the loop awake, and the list is
/// closed.
///
/// [`live`] decides whether the loop asks for another frame, and it is the one
/// decision in this file that has already been got wrong twice in the same
/// direction. The first time it was set once and never cleared, so folding the
/// picture away left the window drawing at full rate — found by an operator on
/// another machine following this file's own instructions, which said the
/// window goes quiet, and getting 270 frames. The second time it was the
/// picture alone, which is the same failure with a preview under it: fold the
/// picture and deck A goes on auditioning while the loop stops asking for
/// frames, so the panel keeps changing and nothing draws it.
///
/// So the assertion is over every sink, not over the one this program fills: a
/// cell nobody has wired up yet is asserted live all the same, because the
/// failure is a sink left out of the list rather than a sink that is off.
///
/// It needs no device: an `egui::TextureId` is a number, and what is being
/// asserted is a rule about `Option`s.
#[test]
fn anything_that_makes_texels_keeps_the_loop_awake() {
    let some = Picture {
        id: egui::TextureId::User(0),
        rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(16.0, 9.0)),
    };
    let mut view = View::new(Room::Day);

    // Nothing is making texels, so the loop has no reason of its own to
    // draw and `ControlFlow::Wait` gets to block.
    assert!(!live(&view), "an empty panel was called live");

    // The picture, which is what this rule used to be the whole of.
    view.picture = Some(some);
    assert!(live(&view), "a live picture did not keep the loop awake");

    // **The case the picture-alone rule gets wrong**: the picture folded
    // away with deck A still auditioning under it.
    view.picture = None;
    view.previews[0] = Some(some);
    assert!(
        live(&view),
        "the picture is folded away and deck A is still rendering, and the loop was \
         told to sleep — which is the window that kept drawing 270 frames after it \
         was said to have gone quiet"
    );

    // And the list is closed: every cell counts, including the three this
    // program leaves off, because the bug is a sink that is not read here.
    for deck in 0..DECKS {
        let mut view = View::new(Room::Day);
        view.previews[deck] = Some(some);
        assert!(
            live(&view),
            "deck {deck} is rendering and the loop was told to sleep"
        );
    }

    // The other direction, which costs frames rather than pixels: with
    // every sink off the loop stops asking.
    view.previews[0] = None;
    assert!(
        !live(&view),
        "nothing is rendering and the loop stayed awake"
    );
}

/// Two paths or none, and anything else is a refusal rather than a guess.
///
/// [`sources_from`] is the whole of this program's command line and this is
/// what stops it growing a second one. The mistake it will actually be given is
/// *one* path — a Set is two files and reads like one thing — and that is
/// refused by name rather than paired with a default renderer, because a
/// program that silently supplied half the material would draw something nobody
/// asked for and say nothing about it.
///
/// A CPU test: nothing here opens a file, and a path that does not exist is
/// still a path. What is behind one is [`checked`]'s to complain about.
#[test]
fn a_set_is_two_paths_or_none_and_anything_else_is_refused() {
    let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));

    let bare = of(&[]).expect("no arguments is the pair the preset library ships");
    assert_eq!(bare.sources.l1, shipped().l1);
    assert_eq!(bare.sources.l4, shipped().l4);
    assert!(
        bare.sources.l1.is_file() && bare.sources.l4.is_file(),
        "the default pair is not on the disk at {} and {}, so a bare run cannot draw",
        bare.sources.l1.display(),
        bare.sources.l4.display()
    );

    let named = of(&["a/geo.kir", "b/ren.kir"]).expect("two paths are a Set");
    assert_eq!(named.sources.l1, std::path::PathBuf::from("a/geo.kir"));
    assert_eq!(named.sources.l4, std::path::PathBuf::from("b/ren.kir"));
    assert_eq!(
        named.sources.material(),
        "geo + ren",
        "the strip is not named after what was actually loaded"
    );

    let one = of(&["a/geo.kir"]).expect_err(
        "one path was read as a Set, so this program would have invented the other half",
    );
    assert!(
        one.contains("a/geo.kir"),
        "the refusal `{one}` does not name the path it refused"
    );
    assert!(
        of(&["a.kir", "b.kir", "c.kir"]).is_err(),
        "three paths were read as a Set"
    );

    // **An empty message is `--help`**, which is the one arm that is a
    // request rather than a mistake — [`main`] prints [`USAGE`] to stdout
    // and exits 0 on it, and prints it to stderr and exits 2 on every
    // other. A refusal that came back empty would be a silent exit.
    assert_eq!(of(&["--help"]).err(), Some(String::new()));
    assert_eq!(of(&["-h"]).err(), Some(String::new()));
    assert!(
        !one.is_empty(),
        "a refusal came back with no sentence in it, which `main` reads as `--help`"
    );
}

/// `--mcp` takes a port, and it is refused in the three ways a flag with a
/// value is refused.
///
/// The first two are [`value_for`]'s and are the two the other flags already
/// meet — a flag at the end of the line does not fall back to a default, and a
/// flag whose value is the next flag does not eat it. The third is
/// [`number_for`]'s and is new here, because this is the first flag on this
/// command line that takes a number: a port that is not a port is a mistake on
/// the command line, and a run that started serving on some other number would
/// be the wrong kind of helpful.
#[test]
fn the_mcp_flag_takes_a_port_and_is_refused_the_three_ways_a_valued_flag_is() {
    let read = |args: &[&str]| sources_from(args.iter().map(|a| a.to_string()).collect::<Vec<_>>());

    let launch = read(&["--mcp", "8000"]).expect("a port is a port");
    assert_eq!(launch.mcp, Some(8000));
    // **On either side of the pair, like the two flags beside it.** An
    // operator types the flags in whatever order they think of them.
    let pair = shipped();
    let (l1, l4) = (pair.l1.display().to_string(), pair.l4.display().to_string());
    let launch = read(&[&l1, &l4, "--mcp", "0"]).expect("after the pair");
    assert_eq!(launch.mcp, Some(0), "a port after the pair");
    let launch = read(&["--mcp", "0", &l1, &l4]).expect("before the pair");
    assert_eq!(launch.mcp, Some(0), "a port before the pair");

    // And a run that does not ask serves nothing rather than a default port.
    assert_eq!(
        read(&[&l1, &l4]).expect("no flag").mcp,
        None,
        "a run that did not ask for a server was given one"
    );

    // The end of the line: nothing after the flag.
    let why = read(&["--mcp"]).expect_err("a flag with nothing after it");
    assert!(why.contains("--mcp"), "the refusal does not name it: {why}");
    assert!(
        why.contains("needs a value"),
        "the refusal is not the one the other flags give: {why}"
    );

    // The next flag is not a value: `--mcp --store x` must blame `--mcp`
    // rather than reading `--store` as a port and then blaming `x` for
    // being an unknown option.
    let why = read(&["--mcp", "--store", "somewhere"]).expect_err("a flag as a value");
    assert!(
        why.contains("--mcp") && why.contains("--store"),
        "the refusal does not say which flag ate which: {why}"
    );

    // And a value that is not a number.
    let why = read(&["--mcp", "eight-thousand"]).expect_err("a port that is not one");
    assert!(
        why.contains("eight-thousand") && why.contains("a port number"),
        "the refusal does not say what was expected: {why}"
    );
}

/// A wire request reaches the slot's watcher, and the rest of that watcher's
/// aim is restated with it.
///
/// The three points `mcp::WireRequest` owes, checked without a window: the edge
/// is replaced rather than appended and keyed on the input, the slot is
/// re-aimed with the run's whole wiring, and a slot this deck has not got is
/// refused in the one sentence every surface refuses one in.
///
/// The other fields are the point of the second assertion. An `Aim` is every
/// field of a slot's identity, and a rewiring that restated only the edges
/// would come back with the outgoing slot's camera, fold and salts — a defect
/// that shows on the *next* build rather than on the rewiring, which is why it
/// is asserted here rather than left to be seen. The Set the slot is running is
/// among them, and it is the one whose symptom is not a picture at all:
/// versions filed under the wrong Set, or under none.
#[test]
fn a_wire_request_reaches_the_slots_watcher_with_the_rest_of_its_aim_restated() {
    let edge = |node: &str, slot: &str, to: &str| karakuri_engine::set::Edge {
        node: node.to_string(),
        slot: slot.into(),
        to: to.to_string(),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let mut aims = vec![Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("grid".into()),
                path: std::path::PathBuf::from("A0-grid.kir"),
            },
            rest: vec![karakuri_environment::compile::Named::bare("A1-points.kir")],
            layering: Layering::Composite,
            live: Some(0),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            // **A slot running a Set**, which is what makes the assertion
            // below about `restated` rather than about a default.
            set: Some("night01".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    )];
    let mut edges = Vec::new();

    let said = rewired(
        &[(0, edge("warp", "shape", "field"))],
        &mut edges,
        &mut aims,
        1,
    );
    assert_eq!(said.len(), 1);
    let line = said[0].as_ref().expect("the slot is in range");
    assert!(
        line.contains("warp.shape=field") && line.contains("recompiling"),
        "the answer does not say what was wired or that anything rebuilds: {line}"
    );
    let aim = rx.try_recv().expect("the watcher was not re-aimed at all");
    assert_eq!(aim.edges, vec![edge("warp", "shape", "field")]);
    // **The fields that are not the edges.**
    assert_eq!(aim.head.name.as_deref(), Some("grid"));
    assert_eq!(
        aim.set.as_deref(),
        Some("night01"),
        "the rewiring dropped the Set the slot is running, so every version \
         written after it would be filed under none"
    );
    assert_eq!(aim.live, Some(0), "the fold was silently un-selected");
    assert_eq!(aim.capacity, Some(2048), "the capacity came back as none");
    assert_eq!(aim.salts, vec![9], "the salts would repaint every element");
    assert_eq!(aim.layering, Layering::Composite);

    // **The same input again is a replacement and not a second edge**,
    // because `SetError::SlotBoundTwice` refuses two edges on one input
    // where the Set is built — an append would make a model unable to
    // change its mind.
    let said = rewired(
        &[(0, edge("warp", "shape", "other"))],
        &mut edges,
        &mut aims,
        1,
    );
    assert!(said[0].is_ok(), "{:?}", said[0]);
    assert_eq!(
        edges,
        vec![edge("warp", "shape", "other")],
        "the run is wired with both, and the Set will refuse to build"
    );
    let aim = rx.try_recv().expect("the second request re-aimed nothing");
    assert_eq!(aim.edges, vec![edge("warp", "shape", "other")]);
    // And the aim the watcher is pointed at moved with it, so a third
    // request restates the second rather than the first.
    assert_eq!(aims[0].at.edges, vec![edge("warp", "shape", "other")]);

    // A slot this deck has not got, in the one sentence.
    let said = rewired(
        &[(3, edge("warp", "shape", "field"))],
        &mut edges,
        &mut aims,
        1,
    );
    let why = said[0].as_ref().expect_err("slot 3 of a deck of one");
    assert_eq!(
        why,
        &format!(
            "{}, and nothing was rewired",
            karakuri_environment::no_such_slot(3, 1)
        ),
        "the refusal is not the one every other surface gives"
    );
    assert_eq!(
        edges,
        vec![edge("warp", "shape", "other")],
        "a refused request wrote an edge anyway"
    );
    assert!(
        rx.try_recv().is_err(),
        "a refused request re-aimed a watcher"
    );
}

/// A press on the Inspector deck head's fold re-aims the slot, and the rest of
/// that watcher's aim is restated with it.
///
/// The test above one operation along, and it is the same property for the same
/// reason: a `watch::Aim` is every field of a slot's identity, so an arm that
/// changed the layering and left the rest behind would come back with the
/// outgoing slot's fold, capacity, salts, camera and Set — on the *next* build
/// rather than on the press, which is the hardest version of it to see
/// (ADR-0228, ADR-0314).
///
/// `Aiming::at` is what the second half asserts against. A press that sent an
/// aim and left `at` behind would leave the next re-aim restating the layering
/// the run launched with, so the third assertion here is that a *second* press
/// comes back to where the first one put it rather than to where the run
/// started.
///
/// No window, no device and no `Deck` — `composited` is a free function over
/// the aims for exactly this. A procedure loaded over a layer re-aims the slot
/// with exactly one file replaced, and leaves `Aim::set` where it is —
/// ADR-0338's decision 3, at the seam it crosses.
///
/// Three things it would be wrong about silently: the position it lands on (the
/// first node of that kind), the file it puts there (the procedure's own bytes,
/// in the deck's scratch), and everything else about the aim, which has to come
/// back restated rather than defaulted. The fourth is the one the maintainer
/// answered: the versions this slot writes from here on go on being filed under
/// the Set it started from.
///
/// No window, no device and no `Deck` — `overlaying` takes the aim. The strip
/// reads `<base> + <kir>` once a layer has been written over what a deck is
/// playing, and the base is the Set it is filed under — or the launch pair
/// where it is filed under none (ADR-0338).
#[test]
fn the_strip_reads_the_base_and_the_procedure_written_over_it() {
    assert_eq!(
        derived_material(
            &base_material(Some("drift_night"), "coil_vortex + star_flares"),
            "orbit_wide"
        ),
        "drift_night + orbit_wide"
    );
    // **A slot nobody has loaded a Set onto**: no id names what it is
    // running, so the base is the pair the run opened with.
    assert_eq!(
        derived_material(
            &base_material(None, "coil_vortex + star_flares"),
            "orbit_wide"
        ),
        "coil_vortex + star_flares + orbit_wide"
    );
}

#[test]
fn a_procedure_load_replaces_one_file_and_keeps_the_base_set() {
    let root = scratch_dir("procedure-load");
    Store::open(&root).expect("a store to keep in");
    std::fs::write(
        root.join(Store::PROCEDURES).join("orbit_wide.kir"),
        "proc orbit_wide {\n  kind L3\n}\n",
    )
    .expect("a kept procedure");
    // The slot's own two files, written where a watcher would be looking:
    // an L1 and an L4, which is the pair every run opens on.
    let l1 = karakuri_environment::scratch::place(&root, "A0-drift_shell", "kind L1\n")
        .expect("the geometry");
    let l4 = karakuri_environment::scratch::place(&root, "A1-star_flares", "kind L4\n")
        .expect("the renderer");

    let (tx, rx) = std::sync::mpsc::channel();
    let mut aim = Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("shell".into()),
                path: l1.clone(),
            },
            rest: vec![karakuri_environment::compile::Named {
                name: Some("flares".into()),
                path: l4.clone(),
            }],
            layering: Layering::Composite,
            live: Some(2),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            camera: karakuri_engine::camera::Orbit {
                radius: 3.5,
                ..karakuri_engine::camera::Orbit::default()
            },
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: vec![karakuri_engine::set::Edge {
                node: "flares".to_string(),
                slot: "shape".into(),
                to: "shell".to_string(),
            }],
            authorities: Vec::new(),
            set: Some("drift_night".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    );

    // **The deck holds no camera, so the procedure is added as node 0 of
    // its kind** — the case the row is for.
    let line = overlaying(&root, None, 0, &mut aim, "orbit_wide").expect("the load was refused");
    assert!(
        line.contains("orbit_wide") && line.contains("kept") && line.contains("L3"),
        "{line}"
    );
    let sent = rx.try_recv().expect("no aim was sent");
    assert_eq!(sent.head.path, l1, "the geometry moved");
    assert_eq!(sent.rest.len(), 2, "the slot does not hold three nodes now");
    assert_eq!(sent.rest[0].path, l4, "the renderer moved");
    assert_eq!(
        std::fs::read_to_string(&sent.rest[1].path).expect("the camera's file"),
        "proc orbit_wide {\n  kind L3\n}\n",
        "the file the aim names is not the procedure's own bytes"
    );
    assert_eq!(
        sent.rest[1].name.as_deref(),
        Some("orbit_wide"),
        "a node added by this row is not named after it"
    );

    // **Everything else restated**, which is `Aiming::changed`'s single
    // derivation — the layering, the capacity, the salts, the camera and
    // the wiring come back as the slot's own.
    assert_eq!(sent.layering, Layering::Composite);
    assert_eq!(sent.capacity, Some(2048));
    assert_eq!(sent.salts, vec![9]);
    assert_eq!(sent.camera.radius, 3.5);
    assert_eq!(sent.edges.len(), 1);
    // **And the Set it is filed under does not move**, which is what keeps
    // the snapshot every compile takes alive (ADR-0304, ADR-0308).
    assert_eq!(sent.set.as_deref(), Some("drift_night"));

    // **A second load of the same kind lands on the node the first one
    // added**, which is *the first node of that kind* read a second time:
    // the slot still holds three nodes.
    std::fs::write(
        root.join(Store::PROCEDURES).join("tunnel_eye.kir"),
        "  kind L3\n",
    )
    .expect("a second camera");
    overlaying(&root, None, 0, &mut aim, "tunnel_eye").expect("the second load was refused");
    let sent = rx.try_recv().expect("no second aim was sent");
    assert_eq!(
        sent.rest.len(),
        2,
        "the second camera was added beside the first"
    );
    assert_eq!(
        std::fs::read_to_string(&sent.rest[1].path).expect("the camera's file"),
        "  kind L3\n"
    );
    assert_eq!(
        sent.rest[1].name.as_deref(),
        Some("orbit_wide"),
        "the replaced node did not keep the name the edges resolve against"
    );

    // **A renderer replaces the renderer that is there** — `L4:0`, and the
    // geometry does not move.
    std::fs::write(
        root.join(Store::PROCEDURES).join("hard_dots.kir"),
        "kind L4\n",
    )
    .expect("a renderer");
    overlaying(&root, None, 0, &mut aim, "hard_dots").expect("the renderer load was refused");
    let sent = rx.try_recv().expect("no third aim was sent");
    assert_eq!(sent.head.path, l1, "the geometry moved on a renderer load");
    assert_eq!(sent.rest.len(), 2);
    assert_eq!(
        sent.rest[0].name.as_deref(),
        Some("flares"),
        "the renderer did not keep its node name"
    );
    assert_ne!(
        sent.rest[0].path, l4,
        "the renderer's file was not replaced"
    );

    // **A name neither tier holds is refused with the name back**, and
    // nothing is sent.
    let why = overlaying(&root, None, 0, &mut aim, "no_such_thing")
        .expect_err("a name nothing holds was loaded");
    assert!(
        why.contains("no_such_thing") && why.contains("procedures"),
        "{why}"
    );
    assert!(rx.try_recv().is_err(), "a refused load sent an aim");

    // **A `.kir` that declares no kind is refused too**, because there is
    // no layer to write it over.
    std::fs::write(
        root.join(Store::PROCEDURES).join("mute.kir"),
        "// nothing\n",
    )
    .expect("a procedure with no kind");
    let why = overlaying(&root, None, 0, &mut aim, "mute")
        .expect_err("a procedure with no kind was loaded");
    assert!(why.contains("declares no `kind`"), "{why}");

    std::fs::remove_dir_all(&root).expect("clean up");
}

#[test]
fn a_composite_press_re_aims_the_slot_and_restates_the_rest_of_its_aim() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut aims = vec![Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("grid".into()),
                path: std::path::PathBuf::from("A0-grid.kir"),
            },
            rest: vec![karakuri_environment::compile::Named::bare("A1-points.kir")],
            // **Overdrawing**, so the press below asks for the other one
            // and the assertion is about a field that moved.
            layering: Layering::Overdraw,
            live: Some(2),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            // **A camera nobody's default produces**, so the assertion
            // below is about a value that was carried rather than one that
            // happens to coincide with `Orbit::default()`.
            camera: karakuri_engine::camera::Orbit {
                radius: 3.5,
                ..karakuri_engine::camera::Orbit::default()
            },
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: vec![karakuri_engine::set::Edge {
                node: "warp".to_string(),
                slot: "shape".into(),
                to: "field".to_string(),
            }],
            authorities: Vec::new(),
            set: Some("night01".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    )];

    let line = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
    )
    .expect("the arm answered nothing for the operation it is for");
    assert!(
        line.contains("composite") && line.contains("recompiling"),
        "the answer does not say what was asked for or that the slot rebuilds: {line}"
    );

    let aim = rx.try_recv().expect("the watcher was not re-aimed at all");
    assert_eq!(
        aim.layering,
        Layering::Composite,
        "the press did not move the one field it is about"
    );
    // **The thirteen that did not move.** Each of these is a symptom
    // somebody would meet on the next save rather than on this press.
    assert_eq!(aim.head.name.as_deref(), Some("grid"));
    assert_eq!(aim.rest.len(), 1);
    assert_eq!(aim.live, Some(2), "the fold was silently un-selected");
    assert_eq!(aim.capacity, Some(2048), "the capacity came back as none");
    assert_eq!(aim.seed_salt, 9);
    assert_eq!(aim.salts, vec![9], "the salts would repaint every element");
    // `Orbit` is not `PartialEq`, so the field the camera's own loss shows
    // in is what this reads — `Watch::camera`'s symptom is a slot back at
    // `Orbit::default()`, and a radius nobody could have written is what
    // tells the two apart.
    assert_eq!(
        aim.camera.radius, 3.5,
        "the camera came back at its default"
    );
    assert_eq!(aim.edges.len(), 1, "the run's wiring was dropped");
    assert_eq!(
        aim.set.as_deref(),
        Some("night01"),
        "the press dropped the Set the slot is running, so every version written after it \
         would be filed under none"
    );
    assert_eq!(aims[0].at.layering, Layering::Composite);

    // **Asking for the layering the slot is now in sends nothing**, because
    // a re-aim rebuilds the whole slot and this one would land on the same
    // picture (P-0091).
    let line = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
    )
    .expect("the arm answered nothing");
    assert!(
        line.contains("already"),
        "the answer does not say the slot is already set that way: {line}"
    );
    assert!(
        rx.try_recv().is_err(),
        "a press asking for the state the slot is in recompiled it"
    );

    // **And the second press restates what the first one left**, which is
    // what keeping `Aiming::at` buys: back to overdraw, with the layering
    // read off the aim this program is holding rather than off the launch
    // pair.
    composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: false,
        },
    )
    .expect("the arm answered nothing");
    let aim = rx.try_recv().expect("the second press re-aimed nothing");
    assert_eq!(aim.layering, Layering::Overdraw);
    assert_eq!(aim.set.as_deref(), Some("night01"));

    // A slot this deck has not got, in the one sentence every surface
    // refuses one in.
    let why = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 3,
            compositing: true,
        },
    )
    .expect("a slot the deck has not got answered nothing");
    assert!(
        why.contains(&karakuri_environment::no_such_slot(3, 1)),
        "the refusal is not the one every other surface gives: {why}"
    );
    assert!(rx.try_recv().is_err(), "a refused press re-aimed a watcher");

    // And it answers `None` for everything that is not its operation, so
    // the dispatch above can call it on every press.
    assert!(composited(&mut aims, &Operation::Quit).is_none());
}

/// The two flags say where this program's data is, and either may sit on either
/// side of the pair.
///
/// The order half is the one an operator meets: they type the flags in whatever
/// order they think of them, and `karakuri-cli` accepts `--store` before or
/// after its own command for exactly this reason
/// (`list_sets_prints_and_is_never_a_run`). A parser that matched on the
/// argument slice — which is what this one was — can only ever accept one of
/// the two spellings.
///
/// And the pair still wins, which is the claim [`Sources`]'s doc makes about
/// these flags not being a second material vocabulary: `--presets` moves what a
/// run with *no* paths opens on and reaches nothing else, so a line with both a
/// library and a pair plays the pair.
///
/// Not quite a CPU test, and this is what changed: resolving a presets root is
/// existence checks on real directories. The library it names is this
/// workspace's own `examples/`, which is on the disk whenever these tests run
/// at all.
#[test]
fn the_two_flags_say_where_the_data_is_and_may_sit_on_either_side_of_the_pair() {
    let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));
    let library = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let library = library
        .to_str()
        .expect("this workspace's path is not utf-8");

    // **The default store is the shared constant**, which is the whole of
    // what deleting `const STORE` was for: this asserts the two programs
    // read one directory rather than two that look alike.
    assert_eq!(
        of(&[]).expect("a bare run").store,
        std::path::PathBuf::from(karakuri_environment::places::STORE),
        "a run that said nothing about a store did not get the shared default"
    );

    for spelling in [
        vec!["--store", "/tmp/library", "a/geo.kir", "b/ren.kir"],
        vec!["a/geo.kir", "b/ren.kir", "--store", "/tmp/library"],
        vec!["a/geo.kir", "--store", "/tmp/library", "b/ren.kir"],
    ] {
        let launch = of(&spelling).unwrap_or_else(|why| panic!("{spelling:?}: {why}"));
        assert_eq!(
            launch.store,
            std::path::PathBuf::from("/tmp/library"),
            "{spelling:?} read a store nobody asked for"
        );
        assert_eq!(
            launch.sources.l1,
            std::path::PathBuf::from("a/geo.kir"),
            "{spelling:?} lost the pair to the flag"
        );
        assert_eq!(launch.sources.l4, std::path::PathBuf::from("b/ren.kir"));
    }

    // `--presets` with no pair: it is what the pair defaults to, and the
    // resolution reports it as typed rather than as something found.
    let told = of(&["--presets", library]).expect("a library that is there");
    assert_eq!(
        told.sources.l1,
        std::path::Path::new(library).join("coil_vortex.kir")
    );
    assert_eq!(
        told.sources.l4,
        std::path::Path::new(library).join("star_flares.kir")
    );
    assert_eq!(
        told.presets.as_ref().map(|presets| presets.found),
        Some(karakuri_environment::places::Found::Given),
        "a `--presets` an operator typed was reported as a place this program went \
         looking in"
    );

    // And with a pair, on either side: the pair wins and the library is
    // still the one that was named.
    for spelling in [
        vec!["--presets", library, "a/geo.kir", "b/ren.kir"],
        vec!["a/geo.kir", "b/ren.kir", "--presets", library],
    ] {
        let launch = of(&spelling).unwrap_or_else(|why| panic!("{spelling:?}: {why}"));
        assert_eq!(
            launch.sources.l1,
            std::path::PathBuf::from("a/geo.kir"),
            "{spelling:?}: `--presets` overrode the paths the operator named, which \
             would make it a second way of saying what plays"
        );
        assert_eq!(launch.sources.l4, std::path::PathBuf::from("b/ren.kir"));
        assert_eq!(
            launch.presets.map(|presets| presets.dir),
            Some(std::path::PathBuf::from(library)),
            "{spelling:?} lost the library it was given"
        );
    }

    // A `--presets` that is not there is refused rather than searched
    // past, and the sentence is `places`' own — one refusal, whichever
    // program the operator reached it from.
    let missing = std::path::Path::new(library).join("no-such-library");
    let why = of(&["--presets", missing.to_str().expect("utf-8")])
        .expect_err("a `--presets` that is not there was accepted");
    assert_eq!(why, karakuri_environment::places::no_presets_at(&missing));

    // A flag with nothing after it, and a flag whose value is the next
    // flag. Neither falls back and neither swallows.
    for (spelling, wanted) in [
        (vec!["--presets"], "`--presets` needs a value"),
        (vec!["--store"], "`--store` needs a value"),
        (
            vec!["--presets", "--store", "/tmp/library"],
            "`--presets` was given no value — `--store` is an option, not one",
        ),
    ] {
        assert_eq!(
            of(&spelling).as_ref().err().map(String::as_str),
            Some(wanted),
            "{spelling:?}"
        );
    }

    // **An unknown option is not a path**, which is the mistake a typo
    // actually makes: without this, `--prests DIR` becomes a two-path Set
    // and is reported as a file that will not open.
    let typo = of(&["--prests", library]).expect_err("an unknown option was read as half of a Set");
    assert_eq!(typo, "unknown option `--prests`");
    assert!(
        !typo.is_empty(),
        "a refusal came back with no sentence in it, which `main` reads as `--help`"
    );
}

/// The capacity is the L1's own declaration, read off the `Checked`.
///
/// It was `const CAPACITY: u32 = 262144` here — `drift_shell.kir`'s declared
/// default, transcribed — for as long as this file could only ever load that
/// one file. It takes a path now, so a transcription would be right about one
/// `.kir` and silently wrong about every other: a procedure written for 131072
/// elements would run at 262144 and nothing would say so.
///
/// It is not `karakuri_ir::DEFAULT_CAPACITY` either, which is the language
/// default for a file that declared nothing and is what `check_header` makes
/// unreachable for an L1 that passed checking. The number below is asserted
/// rather than derived on purpose, and it is the reference workload's rather
/// than this program's: `docs/contributing.md` §1 names
/// `examples/drift_cloud.kset` at 1280x720, and 262144 is what that Set's L1
/// declares. It used to be asserted of whatever a bare `cargo run -p karakuri`
/// opened on, which coupled the workload to the demo and is ADR-0270. What is
/// still asserted of the shipped pair is that its capacity is read from its own
/// file, which is a different property and the one this test is named for.
#[test]
fn the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none() {
    let sources = shipped();

    // **The reference workload, pinned by name.** `drift_cloud.kset` is the
    // Set `docs/contributing.md` §1 names, and this is its L1. That the
    // `.kset` names these two parts is checked where every shipped Set is
    // composed, in `karakuri-cli`'s `examples` suite, so it is not
    // transcribed twice here.
    let reference = checked(&sources.l1.with_file_name("drift_shell.kir"));
    let pinned = reference
        .capacity
        .expect("an L1 that passed contract checking always carries a capacity");
    assert_eq!(
        pinned.default, 262_144,
        "`examples/drift_cloud.kset`'s L1 no longer declares the capacity every \
         host-clock figure in this repository was taken at, and \
         `docs/contributing.md` §1 names it as the one reference workload \
         (ADR-0270)"
    );

    // **And the pair this program opens on, checked for a per-file read and
    // not for a number.** ADR-0270 split these: which pair is the default is
    // a demo decision, and what it may not do is run at something other than
    // what its own file declares.
    let l1 = checked(&sources.l1);
    let declared = l1
        .capacity
        .expect("an L1 that passed contract checking always carries a capacity");
    assert!(
        declared.contains(declared.default),
        "the file's own default is outside the range the same file declares"
    );

    assert_eq!(capacity_of(&l1), declared.default);

    assert!(
        checked(&sources.l4).capacity.is_none(),
        "the renderer declares a capacity — `Set::build` is handed the L1's, and \
         two declarations would be two answers to how many elements there are"
    );

    // **A second L1, and it is the one that tells the two mistakes apart.**
    // The pin above is `drift_shell.kir` at 262144, which is also
    // `karakuri_ir::DEFAULT_CAPACITY` — so that assertion passes just as
    // well against a [`capacity_of`] that ignored the file and returned the
    // language default. `strand_shell.kir` declares 131072 and says why in
    // the file (512 strands x 256 samples), and it is what that defect
    // fails on. It is kept although the shipped pair no longer declares the
    // language default either (ADR-0271 moved it to `coil_vortex.kir` at
    // 10240): which pair is the default is a demo decision, and a test that
    // can only tell a per-file read from a constant while the demo happens
    // to be off the constant is a test that goes quiet the next time the
    // demo moves.
    let other = checked(&sources.l1.with_file_name("strand_shell.kir"));
    assert_eq!(
        capacity_of(&other),
        131_072,
        "a second procedure did not run at what it declares — the capacity is being \
         read from somewhere other than the file"
    );
    assert_ne!(
        capacity_of(&other),
        karakuri_ir::DEFAULT_CAPACITY,
        "the second procedure declares the language default, so this test can no \
         longer tell a per-file read from a constant — pick another `.kir`"
    );
}

/// The pair a bare run plays, for the tests that need one on the disk.
///
/// [`Sources::under`] takes a preset library and does not go looking for one;
/// this is the going-looking, and in a test binary the answer is always the
/// last candidate — the workspace this file was compiled in, which is also the
/// tree the test is run from. That is the development entry doing exactly what
/// it is for, and it is why these tests can assert the pair is on the disk
/// without an install anywhere.
///
/// A function rather than an `impl Default` on [`Sources`], because a `Default`
/// is what baked the build machine's own tree into a shipped binary: a type
/// whose default value is a search of the filesystem invites exactly that call
/// from production, and a production caller now has to say which library it
/// means.
///
/// One `.kir`, parsed and checked, for the tests that need a `Checked` and no
/// window.
///
/// Reachable from test modules because it is at the file's own scope.
///
/// `karakuri-environment`'s own five stages and not a sixth spelling. This used
/// to be a hand-rolled parse-then-check, which is what the run itself used to
/// build a slot from; the run compiles through
/// [`karakuri_environment::compile::sort_slot`] now, because that is the one
/// place that keeps the bytes a node's address is derived from
/// ([`karakuri_environment::compile::Placed::source`]). What is left here is a
/// test helper, and a test helper with its own compiler would be a second
/// answer to *does this file check* the day either moved.
#[cfg(test)]
pub(crate) fn checked(path: &std::path::Path) -> karakuri_ir::typed::Checked {
    match karakuri_environment::compile::load(path) {
        Ok((checked, _)) => checked,
        Err(report) => panic!("{report}"),
    }
}

#[cfg(test)]
pub(crate) fn shipped() -> Sources {
    let presets = karakuri_environment::places::presets(None)
        .expect("nothing was typed, so there is no typed path to refuse")
        .expect(
            "no preset library was found from the test binary, so the workspace tree this \
             test compiled in has no `examples/` in it",
        );
    Sources::under(&presets.dir)
}

/// The shipped pair in every slot, for the tests that build an [`Engine`].
///
/// A *run* may not do this — [`working_copies`] is what a run calls, and its
/// whole point is that no two slots watch one file — and this helper is not a
/// way back to that. It is legal here for the reason the copies exist: nothing
/// in these tests edits a `.kir`, no watcher of theirs ever sees a change, and
/// a test that materialised into a temporary store would be asserting the
/// copies rather than the thing it is about. The one test that *is* about the
/// copies calls `working_copies` and is named after the claim.
#[cfg(test)]
pub(crate) fn shipped_slots() -> Vec<Sources> {
    std::iter::repeat_n(shipped(), SLOTS).collect()
}

/// The reference workload's pair, for the tests whose claim is about a cost
/// rather than about what this program opens on.
///
/// `docs/contributing.md` §1 names `examples/drift_cloud.kset` —
/// `drift_shell.kir` at the 262144 elements it declares, with `soft_points.kir`
/// — and this resolves those two out of the same preset library [`shipped`]
/// answers from. It is deliberately not [`shipped_slots`], and the two were one
/// value until 2026-09-07.
///
/// What separated them is a test going quiet rather than red.
/// [`ADR-0271`](../../../docs/adr/0271-the-panel-opens-on-the-demo-rather-than-on-the-reference-workloads-pair.md)
/// moved the default pair to `examples/star_vortex.kset`'s two parts, which are
/// closed-form and 10240 elements.
/// `gpu::the_budget_parks_a_deck_and_the_strip_carries_both_residencies` then
/// measured 1.8 ms a slot against a 2.7 ms headroom and the governor answered
/// `NoPrimingNeeded` — a closed-form Set with nothing to warm — so the park the
/// test is named for was still a park and no longer the budget's. Which pair a
/// bare run opens on is a demo decision (ADR-0270); whether the budget refuses
/// a second Live slot is not, and it needs material chosen for its cost.
#[cfg(test)]
pub(crate) fn reference() -> Sources {
    let shipped = shipped();
    Sources {
        l1: shipped.l1.with_file_name("drift_shell.kir"),
        l4: shipped.l4.with_file_name("soft_points.kir"),
    }
}

#[cfg(test)]
pub(crate) fn empty_keeping() -> Keeping {
    let (_, built) = std::sync::mpsc::channel();
    let (save_tx, saves) = std::sync::mpsc::channel();
    let (send_tx, sends) = std::sync::mpsc::channel();
    let (keep_tx, keeps) = std::sync::mpsc::channel();
    Keeping {
        mcp: None,
        playing: Playing {
            playing: Vec::new(),
        },
        built,
        pending: Vec::new(),
        saves,
        save_tx,
        sends,
        send_tx,
        keeps,
        keep_tx,
        in_flight: 0,
    }
}
