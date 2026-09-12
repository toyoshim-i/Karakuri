use super::*;

fn router(text: &str) -> Router {
    let (map, notes) = Map::parse(text);
    assert!(notes.is_empty(), "{notes:?}");
    Router::new(map)
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

/// A deck's published interface, without a deck. Positions count from one, and
/// each answers with the key the Inspector would draw beside it and the range
/// the Set published it over — which is what [`Decks`] reads off a real one. A
/// `Deck` takes a device and every decision on this route is about what
/// arrived, so the trait is what keeps these tests CPU-only.
struct Fake(Vec<Vec<(&'static str, [f32; 2])>>);

impl Interface for Fake {
    fn control_at(&self, slot: u8, position: u16) -> Option<(ParamAt, [f32; 2])> {
        let deck = self.0.get(usize::from(slot))?;
        let (key, range) = deck.get(usize::from(position).checked_sub(1)?)?;
        Some((
            ParamAt {
                node: None,
                key: (*key).to_owned(),
            },
            *range,
        ))
    }
}

/// A deck read back, without a deck. What [`Lit`] answers off a real one, as a
/// table a test writes: a value per continuous control and a state per pad.
/// `None` is a control this stands in for nothing of, which is what `tap` and a
/// slot past the end of a deck are.
#[derive(Default)]
struct Held(std::collections::HashMap<String, Shown>);

impl Held {
    fn at(mut self, control: &str, value: f32) -> Held {
        self.0.insert(control.to_string(), Shown::At(value));
        self
    }
    fn on(mut self, control: &str, on: bool) -> Held {
        self.0.insert(control.to_string(), Shown::On(on));
        self
    }
    fn set(&mut self, control: &str, value: f32) {
        self.0.insert(control.to_string(), Shown::At(value));
    }
}

/// The spelling of a control, which is the map's own key for it
/// (`Target::spelled`) and the one both directions agree on.
fn named(control: Control) -> String {
    match control {
        Control::Gain { deck } => format!("gain {deck}"),
        Control::Opacity { deck } => format!("opacity {deck}"),
        Control::Exposure => "exposure".to_string(),
        Control::MaskPosition { deck } => format!("mask-position {deck}"),
        Control::Residency { deck, residency } => {
            format!("residency {deck} {}", residency.name())
        }
        Control::Blend { deck, blend } => format!("blend {deck} {}", blend.name()),
        Control::Param { deck, position } => format!("param {deck} {position}"),
        Control::Tap => "tap".to_string(),
    }
}

impl Feedback for Held {
    fn shown(&self, control: Control) -> Option<Shown> {
        self.0.get(&named(control)).copied()
    }
}

/// A deck with nothing published, which is what every test that is not about
/// parameters wants: it answers `None` to everything.
struct Nothing;

impl Interface for Nothing {
    fn control_at(&self, _slot: u8, _position: u16) -> Option<(ParamAt, [f32; 2])> {
        None
    }
}

fn routed(r: &mut Router, messages: &[Message], slots: usize) -> Vec<Operation> {
    let mut out = Vec::new();
    r.route(messages, slots, &Nothing, &mut out);
    out
}

fn over(
    r: &mut Router,
    messages: &[Message],
    slots: usize,
    interface: &dyn Interface,
) -> Vec<Operation> {
    let mut out = Vec::new();
    r.route(messages, slots, interface, &mut out);
    out
}

/// A message naming a slot the deck does not have is dropped here, so the
/// refusal is said once per slot rather than once per message — a fader sweep
/// into `gain 4` on a deck of four is several hundred of them, and `cc 1 ->
/// gain 4` is the likeliest typo a map has because the `ch` on the same line
/// *is* one-based.
#[test]
fn an_operation_past_the_end_of_the_deck_is_dropped_rather_than_routed() {
    let mut r = router("cc 1 -> gain 3\ncc 2 -> gain 0");
    let out = routed(&mut r, &[cc(1, 127), cc(2, 64)], 2);
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(matches!(out[0], Operation::SetGain { deck: 0, .. }));
    // The boundary either side of it, which is where an off-by-one lives.
    assert_eq!(routed(&mut r, &[cc(1, 127)], 4).len(), 1);
    assert_eq!(routed(&mut r, &[cc(1, 127)], 3).len(), 0);
}

/// The mask's front is checked here too, and for a sharper reason than the
/// other five: a `gain 4` that got past this is refused once by `mix::change`,
/// where a `mask-position 4` is stopped at `Live::operate` — which cannot read
/// a mask the deck does not hold, answers `Owed::NotRead` and prints it every
/// time. Left out of `deck_of`, a fader sweep into a mistyped slot would be a
/// blocking write per message inside `Live::frame`.
#[test]
fn a_mask_front_past_the_end_of_the_deck_is_dropped_here_rather_than_printed() {
    let mut r = router("cc 9 -> mask-position 4\ncc 10 -> mask-position 1");
    let out = routed(&mut r, &[cc(9, 127), cc(10, 64)], 2);
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(matches!(out[0], Operation::SetMaskPosition { deck: 1, .. }));
}

/// Previous frame's operations do not arrive again. `out` is a buffer the
/// caller keeps, and one left unclear would apply every press on it a second
/// time on a frame nobody touched the surface.
#[test]
fn a_frame_with_no_messages_produces_no_operations() {
    let mut r = router("note 36 -> residency 0 live");
    let mut out = Vec::new();
    r.route(&[note(36)], 4, &Nothing, &mut out);
    assert_eq!(out.len(), 1);
    r.route(&[], 4, &Nothing, &mut out);
    assert!(out.is_empty(), "{out:?}");
}

/// Every message routes to the operation its line names, and only mapped ones
/// route at all. The seam this file exists for, end to end.
#[test]
fn a_mapped_message_becomes_its_operation_and_an_unmapped_one_becomes_nothing() {
    let mut r = router("cc 1 -> gain 2\nnote 36 -> residency 1 live\nnote 37 -> tap");
    let out = routed(
        &mut r,
        &[cc(1, 127), cc(9, 64), note(36), note(99), note(37)],
        4,
    );
    assert_eq!(
        out,
        vec![
            Operation::SetGain { deck: 2, gain: 1.0 },
            Operation::SetResidency {
                deck: 1,
                residency: karakuri_operation::Residency::Live
            },
            Operation::TapBeat,
        ]
    );
}

/// Said once per control, not once per message. A fader sweep is several
/// hundred messages and this runs inside a frame, so a line each would be a
/// blocking write per message on the render thread. Counted through the sets
/// rather than by capturing stderr, which is the only handle a test has on it.
#[test]
fn an_unmapped_control_and_a_missing_slot_are_each_reported_once() {
    let mut r = router("cc 1 -> gain 9");
    let sweep: Vec<Message> = (0..128).map(|v| cc(1, v)).collect();
    let unmapped: Vec<Message> = (0..128).map(|v| cc(2, v)).collect();
    routed(&mut r, &sweep, 4);
    routed(&mut r, &unmapped, 4);
    routed(&mut r, &sweep, 4);
    routed(&mut r, &unmapped, 4);
    // Nothing left to say by the fourth pass, which is the claim: the
    // first sweep said it and no message since has said it again.
    assert!(r.notices().is_empty(), "{:?}", r.notices());

    // And each of them was said exactly once, from a fresh router.
    let mut r = router("cc 1 -> gain 9");
    let mut said = Vec::new();
    for messages in [&sweep, &unmapped, &sweep, &unmapped] {
        routed(&mut r, messages, 4);
        said.extend(r.notices().iter().cloned());
    }
    assert_eq!(said.len(), 2, "{said:?}");
    // **The keys' own sentence, word for word.** This asked only for
    // `contains("no slot 9")`, which passed while this module said `no slot
    // 9 — this deck holds slots 0-3` and every other surface said `no slot
    // 9: …`. See [`crate::no_such_slot`].
    assert!(
        said.iter().any(|s| *s == crate::no_such_slot(9, 4)),
        "{said:?}"
    );
    assert!(said.iter().any(|s| s.contains("cc 2")), "{said:?}");
}

/// The dedup key distinguishes everything that is a different control. A `cc 1`
/// and a `note 1` are two knobs, and the same number on two channels is two
/// knobs on two devices — collapsing either would leave an operator turning
/// something that never prints.
#[test]
fn two_different_controls_are_two_discoveries() {
    let mut r = router("");
    routed(
        &mut r,
        &[
            cc(1, 0),
            note(1),
            Message::ControlChange {
                channel: 5,
                controller: 1,
                value: 0,
            },
        ],
        4,
    );
    assert_eq!(r.notices().len(), 3, "{:?}", r.notices());
}

/// A sweep in one frame is one operation, carrying the value the fader ended
/// the frame at. Several hundred messages arrive between two frames; each one
/// built a record, and on `exposure` and `mask-position` that record carries a
/// name, which is a heap allocation per message inside `Live::frame` — the
/// first rule this repository has. The values before the last were never on
/// screen: the frame draws what the deck holds once, after all of them have
/// been applied.
#[test]
fn a_sweep_in_one_frame_is_one_operation_carrying_the_last_value() {
    let mut r = router("cc 20 -> exposure");
    let sweep: Vec<Message> = (0..128).map(|v| cc(20, v)).collect();
    let out = routed(&mut r, &sweep, 4);
    assert_eq!(out.len(), 1, "{out:?}");
    // The top of the default range, which is what `cc 20 127` asks for.
    assert_eq!(out[0], Operation::SetExposure { exposure: 4.0 });
}

/// Two presses in one frame are two operations, and that is the half of this
/// that must not coalesce: `residency 0 live` then `residency 0 allocated` is
/// not the second one alone in intent, and a note is a press rather than a
/// position. The same control, so a coalescer that keyed on the pad would keep
/// one of them.
#[test]
fn two_presses_of_one_pad_in_one_frame_are_two_operations() {
    let mut r = router("note 36 -> residency 0 live\nnote 37 -> residency 0 allocated");
    let out = routed(&mut r, &[note(36), note(37)], 4);
    assert_eq!(
        out,
        vec![
            Operation::SetResidency {
                deck: 0,
                residency: karakuri_operation::Residency::Live
            },
            Operation::SetResidency {
                deck: 0,
                residency: karakuri_operation::Residency::Allocated
            },
        ]
    );
    // And the same pad twice, which is a press repeated rather than a
    // value repeated: both are hits.
    let mut r = router("note 36 -> residency 1 live");
    assert_eq!(routed(&mut r, &[note(36), note(36)], 4).len(), 2);
}

/// Two faders are two controls. Coalescing is per control, so a frame in which
/// four of them moved says four things — collapsing to one operation a frame
/// would leave three faders dead whenever a hand was on a fourth.
#[test]
fn two_continuous_controls_in_one_frame_are_one_operation_each() {
    let mut r = router("cc 1 -> gain 0\ncc 2 -> gain 1");
    let messages: Vec<Message> = (0..64).flat_map(|v| [cc(1, v), cc(2, 127 - v)]).collect();
    let out = routed(&mut r, &messages, 4);
    assert_eq!(out.len(), 2, "{out:?}");
    // Each carries its own last value, and the first control keeps the
    // place it first spoke in.
    assert_eq!(
        out,
        vec![
            Operation::SetGain {
                deck: 0,
                gain: 63.0 / 127.0
            },
            Operation::SetGain {
                deck: 1,
                gain: 64.0 / 127.0
            },
        ]
    );
}

/// Coalescing is per frame and not a filter on change. The same value on the
/// next frame is asked for again, because nothing here holds what a control
/// last sent and nothing could: MIDI out is not built, so a transition can move
/// the mask front under a hand that is not moving, and a fader re-asserting its
/// position is asking for somewhere the deck may no longer be.
#[test]
fn a_value_repeated_on_the_next_frame_is_not_swallowed() {
    let mut r = router("cc 9 -> mask-position 0");
    let held = Operation::SetMaskPosition {
        deck: 0,
        position: 1.0,
    };
    for _ in 0..3 {
        // A knob held against its stop keeps sending; three frames of it.
        let out = routed(&mut r, &[cc(9, 127), cc(9, 127)], 4);
        assert_eq!(out, vec![held.clone()], "{out:?}");
    }
}

/// A pad hit during a sweep keeps its place in the frame. The fader holds the
/// position it first spoke in and carries the value it ended at, so a press
/// that arrived between two of its messages is still applied after it — the
/// order a frame's operations are given is the order they are applied in.
#[test]
fn a_press_between_two_fader_messages_keeps_its_order() {
    let mut r = router("cc 1 -> gain 0\nnote 36 -> residency 1 live");
    let out = routed(&mut r, &[cc(1, 0), note(36), cc(1, 127)], 4);
    assert_eq!(
        out,
        vec![
            Operation::SetGain { deck: 0, gain: 1.0 },
            Operation::SetResidency {
                deck: 1,
                residency: karakuri_operation::Residency::Live
            },
        ]
    );
}

/// The two tiers, in order, and neither of them being there. The operator's own
/// map wins, which is the only order that lets a learned map matter — a preset
/// that shadowed it would make learning a gesture with no effect the next time
/// the program started.
#[test]
fn the_operators_own_map_wins_over_the_one_that_ships_and_neither_is_a_fault() {
    let store = tempfile::tempdir().expect("store");
    let presets = tempfile::tempdir().expect("presets");
    // A machine with a store that has never been learned into and no
    // preset library at all: a state, not a failure.
    assert_eq!(map_for(store.path(), None), None);
    // A preset library with no map in it is the same nothing.
    assert_eq!(map_for(store.path(), Some(presets.path())), None);

    let shipped = presets.path().join(SHIPPED_MAP);
    std::fs::write(&shipped, "cc 1 -> gain 0\n").expect("write");
    assert_eq!(map_for(store.path(), Some(presets.path())), Some(shipped));

    let own = store.path().join(MAPS);
    std::fs::create_dir_all(&own).expect("mkdir");
    let learned = own.join(format!("{DEFAULT_MAP}.{MAP_SUFFIX}"));
    std::fs::write(&learned, "cc 2 -> gain 1\n").expect("write");
    assert_eq!(
        map_for(store.path(), Some(presets.path())),
        Some(learned.clone()),
        "the operator's own map has to win, or a learned map is overwritten \
             by the preset on every start"
    );
    // And with no preset library at all it is still found.
    assert_eq!(map_for(store.path(), None), Some(learned));
}

/// A mapped knob lands as the record a press lands, which is the whole claim
/// this crate's header makes and the one nothing here checked: the tests above
/// stop at an [`Operation`], and *a session recorded from a controller replays
/// with neither controller nor map attached* (P-0092, P-0090) is about what
/// reaches the stream.
///
/// So this goes one crate further on — through
/// [`karakuri_operation_record::written`], the one exhaustive match every
/// surface's operation goes through — and asserts the record itself. A route
/// that produced its own record beside this one would be two spellings of a
/// `gain`, and a replay would then depend on which surface wrote it.
#[test]
fn a_mapped_control_change_lands_as_the_record_a_press_lands() {
    use karakuri_operation_record::{written, Current, Written};
    use karakuri_store::record::DeckSlot;
    use karakuri_store::Record;

    let mut r = router("cc 1 -> gain 0");
    let out = routed(&mut r, &[cc(1, 127)], 4);
    assert_eq!(out.len(), 1, "{out:?}");

    // The same operation a fader on the panel and the `]` key emit, so
    // the record is the same record by construction rather than by
    // resemblance.
    let by_hand = Operation::SetGain { deck: 0, gain: 1.0 };
    assert_eq!(out[0], by_hand);

    let Written::Records(records) = written(&out[0], &Current::default()) else {
        panic!(
            "a gain writes a record: {:?}",
            written(&out[0], &Current::default())
        );
    };
    assert_eq!(
        records,
        vec![Record::Gain {
            slot: DeckSlot(0),
            value: 1.0
        }]
    );
}

/// A `param` line is resolved against the deck's published interface, and it is
/// the one target the map cannot finish on its own — so this is the seam that
/// makes *Write a parameter* reachable from a knob at all.
///
/// The value is scaled over the range the Set published, not over a default
/// this crate chose: a control declared `0 – 8` reaches 8 at the top of the
/// fader, and a knob that stopped at 1.0 would be a fader that cannot reach
/// what the procedure says is in range.
#[test]
fn a_param_line_resolves_to_the_control_at_that_position_over_the_sets_own_range() {
    let deck = Fake(vec![
        vec![("radius", [0.0, 8.0]), ("twist", [-1.0, 1.0])],
        vec![("glow.x", [0.0, 4.0])],
    ]);
    let mut r = router("cc 30 -> param 0 1\ncc 31 -> param 0 2\ncc 32 -> param 1 1");
    let out = over(&mut r, &[cc(30, 127), cc(31, 0), cc(32, 127)], 4, &deck);
    assert_eq!(
        out,
        vec![
            Operation::WriteParam {
                deck: 0,
                param: ParamAt {
                    node: None,
                    key: "radius".to_owned()
                },
                value: ParamValue::Scalar(8.0),
            },
            Operation::WriteParam {
                deck: 0,
                param: ParamAt {
                    node: None,
                    key: "twist".to_owned()
                },
                value: ParamValue::Scalar(-1.0),
            },
            Operation::WriteParam {
                deck: 1,
                param: ParamAt {
                    node: None,
                    key: "glow.x".to_owned()
                },
                value: ParamValue::Scalar(4.0),
            },
        ]
    );
    assert!(r.notices().is_empty(), "{:?}", r.notices());
}

/// Two knobs on two parameters of one deck are two operations.
///
/// The coalescing key is the discriminant and the deck for every other
/// continuous control, because a deck has one gain and one exposure. It has as
/// many parameters as its Set published, so the position is in the key too —
/// without it a hand on one knob would swallow the other, and the Set's *third*
/// control would be written with the *fifth*'s value.
#[test]
fn two_knobs_on_two_parameters_of_one_deck_do_not_coalesce_into_one() {
    let deck = Fake(vec![vec![
        ("radius", [0.0, 1.0]),
        ("twist", [0.0, 1.0]),
        ("glow.x", [0.0, 1.0]),
    ]]);
    let mut r = router("cc 30 -> param 0 1\ncc 31 -> param 0 3");
    // Both swept in one frame, interleaved, as two hands would.
    let messages: Vec<Message> = (0..64).flat_map(|v| [cc(30, v), cc(31, 127 - v)]).collect();
    let out = over(&mut r, &messages, 4, &deck);
    assert_eq!(out.len(), 2, "{out:?}");
    assert_eq!(
        out[0],
        Operation::WriteParam {
            deck: 0,
            param: ParamAt {
                node: None,
                key: "radius".to_owned()
            },
            value: ParamValue::Scalar(63.0 / 127.0),
        }
    );
    assert_eq!(
        out[1],
        Operation::WriteParam {
            deck: 0,
            param: ParamAt {
                node: None,
                key: "glow.x".to_owned()
            },
            value: ParamValue::Scalar(64.0 / 127.0),
        }
    );
    // And a sweep of one is still one operation carrying its last value,
    // which is what coalescing is for.
    let mut r = router("cc 30 -> param 0 1");
    let sweep: Vec<Message> = (0..128).map(|v| cc(30, v)).collect();
    assert_eq!(over(&mut r, &sweep, 4, &deck).len(), 1);
}

/// A position the Set has no control at is said once, and it is not the same
/// sentence a missing slot gets.
///
/// This is the ordinary state after a load rather than a typo: a map learned
/// against a Set with nine controls has five dead lines against one with four.
/// A knob that goes quiet with nothing said is what P-0094 rules out, and a
/// sentence per message is the blocking write this router exists to keep off
/// the frame path.
#[test]
fn a_position_past_the_end_of_an_interface_is_said_once_and_not_as_a_missing_slot() {
    let deck = Fake(vec![vec![("radius", [0.0, 1.0])]]);
    let mut r = router("cc 30 -> param 0 9\ncc 31 -> param 0 1");
    let sweep: Vec<Message> = (0..128).map(|v| cc(30, v)).collect();
    let out = over(&mut r, &sweep, 4, &deck);
    assert!(out.is_empty(), "{out:?}");
    assert_eq!(r.notices().len(), 1, "{:?}", r.notices());
    assert!(r.notices()[0].contains("param 0 9"), "{:?}", r.notices());
    assert!(r.notices()[0].contains("Inspector"), "{:?}", r.notices());
    // Nothing more to say on the next frame, and the good line still works.
    let out = over(&mut r, &[cc(30, 64), cc(31, 127)], 4, &deck);
    assert!(r.notices().is_empty(), "{:?}", r.notices());
    assert_eq!(out.len(), 1, "{out:?}");

    // **And a deck the deck does not have gets the keys' own words**, not
    // this one: two different facts, two sentences, two sets.
    let mut r = router("cc 30 -> param 9 1");
    over(&mut r, &[cc(30, 64)], 4, &deck);
    assert_eq!(
        r.notices(),
        [crate::no_such_slot(9, 4)],
        "a missing deck was reported as a missing control"
    );
}

/// A learn writes the operator's own map and never the one that ships, and it
/// appends rather than rewriting — so the file an operator started from keeps
/// its comments, and the later line wins on the next load.
#[test]
fn a_learn_appends_to_the_operators_map_and_seeds_it_from_what_is_playing() {
    // No port here, so this is the half of a learn that has no device in
    // it: the map, and the file.
    let (map, notes) = Map::parse("cc 1 -> gain 0\nnote 61 -> tap");
    assert!(notes.is_empty(), "{notes:?}");
    let mut map = map;

    // The line a learn makes is a line the grammar accepts, and it is the
    // whole of what is written.
    let line = map
        .learn(cc(30, 64), "param 0 3")
        .expect("a param target is one this grammar knows");
    assert_eq!(line, "cc 30 -> param 0 3");
    assert_eq!(map.bound("param 0 3").as_deref(), Some("cc 30"));

    // **A learn cannot put a line in a map the map could not be loaded
    // with**, which is what keeps the file editable by hand.
    assert!(
        map.learn(note(36), "param 0 3").is_err(),
        "a note was learned onto a control that takes a position"
    );
    assert!(map.learn(cc(30, 0), "wobble 2").is_err());

    // Re-learning one knob onto another control replaces it here, and
    // appending replaces it on the next load.
    map.learn(cc(30, 0), "gain 2").expect("a second learn");
    assert_eq!(map.bound("gain 2").as_deref(), Some("cc 30"));
    assert_eq!(map.bound("param 0 3"), None);
}

/// What a learn leaves in the file, which is the half of it that has no device
/// in it.
///
/// It appends and never rewrites. The shipped map an operator starts from is
/// two-thirds prose explaining what a line means, and a learn that rewrote the
/// file from the table would turn the one document that teaches the format into
/// forty bare lines on the first press.
///
/// And a file that is not there yet is seeded with what is playing, rather than
/// created holding one line: the lines in force are the ones the run loaded,
/// and a map that shrank to a single control on a press would be the map going
/// quiet.
#[test]
fn a_learn_appends_and_seeds_a_file_that_is_not_there_with_what_is_playing() {
    let seed = "# seeded\ncc 1 -> gain 0\n";
    // No file yet: the seed, then the line.
    assert_eq!(
        appended(None, seed, "cc 30 ", "cc 30 -> param 0 3"),
        "# seeded\ncc 1 -> gain 0\ncc 30 -> param 0 3\n"
    );
    // A file the operator has: their bytes, untouched, then the line.
    let theirs = "# my own map, hands off\ncc 7 -> opacity 1\n";
    assert_eq!(
        appended(
            Some(theirs.to_owned()),
            seed,
            "cc 30 ",
            "cc 30 -> param 0 3"
        ),
        "# my own map, hands off\ncc 7 -> opacity 1\ncc 30 -> param 0 3\n"
    );
    // A file somebody left without a trailing newline does not get two
    // lines run together, which is the one way an append can lose a line.
    assert_eq!(
        appended(
            Some("cc 7 -> opacity 1".to_owned()),
            seed,
            "cc 30 ",
            "cc 30 -> tap"
        ),
        "cc 7 -> opacity 1\ncc 30 -> tap\n"
    );
    // An empty file is not given a blank first line.
    assert_eq!(
        appended(Some(String::new()), seed, "cc 30 ", "cc 30 -> tap"),
        "cc 30 -> tap\n"
    );

    // **A re-learn replaces the knob's own line where it sits**, and
    // leaves the comments, the blank lines and every other knob alone.
    // Without this the file grows on a gesture made dozens of times a
    // session, and `Map::parse` reports the shadowed line on every start.
    let theirs = "# the strip\ncc 1  ->  gain 0\n\n# the pads\nnote 61 -> tap\n";
    assert_eq!(
        appended(Some(theirs.to_owned()), seed, "cc 1", "cc 1 -> gain 2"),
        "# the strip\ncc 1 -> gain 2\n\n# the pads\nnote 61 -> tap\n",
        "a re-learn did not replace the line it was about"
    );

    // **And what comes out loads with nothing to complain about**, which
    // is the property the replacement buys.
    let text = appended(Some(theirs.to_owned()), seed, "cc 1", "cc 1 -> gain 2");
    let (map, notes) = Map::parse(&text);
    assert!(
        notes.is_empty(),
        "a learned file reported something: {notes:?}"
    );
    assert_eq!(map.bound("gain 2").as_deref(), Some("cc 1"));
    assert_eq!(map.bound("gain 0"), None, "the old line survived");
    assert_eq!(
        map.bound("tap").as_deref(),
        Some("note 61"),
        "another knob moved"
    );

    // A line the operator commented out is a line they took out, so a
    // learn on that knob appends rather than reviving it.
    let out = appended(
        Some("# cc 1 -> gain 0\n".to_owned()),
        seed,
        "cc 1",
        "cc 1 -> tap",
    );
    assert_eq!(out, "# cc 1 -> gain 0\ncc 1 -> tap\n");
}

/// A release is not a discovery. Every pad acts on the press, so reporting the
/// release would print the other half of every hit as something the operator
/// had not mapped.
#[test]
fn a_release_is_not_reported_as_unmapped() {
    let mut r = router("note 36 -> residency 0 live");
    routed(
        &mut r,
        &[
            Message::NoteOff {
                channel: 0,
                note: 36,
            },
            Message::NoteOff {
                channel: 0,
                note: 99,
            },
        ],
        4,
    );
    assert!(r.notices().is_empty(), "{:?}", r.notices());
}

/// A 14-bit pair moves the control at 16384 positions, and the two halves are
/// two messages with a frame boundary free to fall between them — so the MSB is
/// held here rather than in the map, which is a pure function of one message.
#[test]
fn a_pair_assembles_across_two_messages_and_the_msb_alone_moves_the_control() {
    let mut r = router("cc14 1 33 -> gain 0");
    // The MSB alone moves the fader coarsely, at exactly the reading a
    // 7-bit line would give: both ends exact, so nothing is left between
    // two values by a pair that has not finished.
    assert_eq!(
        routed(&mut r, &[cc(1, 127)], 4),
        vec![Operation::SetGain { deck: 0, gain: 1.0 }]
    );
    // The LSB refines the MSB that is held, in a later frame.
    assert_eq!(
        routed(&mut r, &[cc(33, 0)], 4),
        vec![Operation::SetGain {
            deck: 0,
            gain: 16256.0 / 16383.0
        }]
    );
    // And within one frame the two coalesce, so what reaches the deck is
    // the refined value alone.
    let mut r = router("cc14 1 33 -> gain 0");
    assert_eq!(
        routed(&mut r, &[cc(1, 64), cc(33, 3)], 4),
        vec![Operation::SetGain {
            deck: 0,
            gain: 8195.0 / 16383.0
        }],
        "a pair inside one frame was not one operation"
    );
}

/// A lone LSB moves nothing and is not a discovery. There is nothing to refine
/// until an MSB has been seen for that control, and the line that names it *is*
/// loaded — so reporting it as unmapped would tell an operator to write a line
/// they have already written.
#[test]
fn a_lone_lsb_moves_nothing_and_is_not_reported_as_unmapped() {
    let mut r = router("cc14 1 33 -> gain 0");
    assert_eq!(routed(&mut r, &[cc(33, 100)], 4), vec![]);
    assert_eq!(
        r.notices(),
        &[] as &[String],
        "a loaded line was called unmapped"
    );
}

/// A deck change writes the mapped control's value to the surface, and an
/// unmapped one writes nothing. The map is the list: a control no line names
/// has no message to send, and MIDI out cannot reach further than MIDI in does.
#[test]
fn a_deck_change_shows_a_mapped_control_and_an_unmapped_one_shows_nothing() {
    let mut r = router("cc 1 -> gain 0\nnote 32 -> residency 0 live");
    let mut held = Held::default()
        .at("gain 0", 0.0)
        .at("gain 1", 0.0)
        .at("opacity 0", 0.5)
        .on("residency 0 live", false);
    let mut wire = Vec::new();

    // The first pass states what is mapped, once.
    r.shown(&held, &mut wire);
    assert_eq!(wire, vec![[0xb0, 1, 0], [0x90, 32, 0]]);

    // A deck nothing moved says nothing at all.
    r.shown(&held, &mut wire);
    assert_eq!(
        wire,
        Vec::<[u8; 3]>::new(),
        "a still deck wrote to the surface"
    );

    // A mapped control moved is one message.
    held.set("gain 0", 1.0);
    r.shown(&held, &mut wire);
    assert_eq!(wire, vec![[0xb0, 1, 127]]);

    // **An unmapped control moved is nothing.** `gain 1` and `opacity 0`
    // are on the deck and on no line of this map.
    held.set("gain 1", 1.0);
    held.set("opacity 0", 1.0);
    r.shown(&held, &mut wire);
    assert_eq!(
        wire,
        Vec::<[u8; 3]>::new(),
        "a control no line names was sent"
    );

    // And a pad follows the state it names, both ways.
    held.0
        .insert("residency 0 live".to_string(), Shown::On(true));
    r.shown(&held, &mut wire);
    assert_eq!(wire, vec![[0x90, 32, 127]]);
}

/// A 14-bit control is shown as a pair, MSB first, and a move too small to
/// change its 7-bit half still moves the fine one — which is the whole of what
/// the second seven bits buy on the way out.
#[test]
fn a_pair_is_shown_as_two_messages_and_a_fine_move_still_says_something() {
    let mut r = router("cc14 2 34 -> opacity 1");
    let mut held = Held::default().at("opacity 1", 0.0);
    let mut wire = Vec::new();
    r.shown(&held, &mut wire);
    assert_eq!(wire, vec![[0xb0, 2, 0], [0xb0, 34, 0]]);
    // A move of one 14-bit step: the MSB half does not change and the
    // fader still follows.
    held.set("opacity 1", 1.0 / 16383.0);
    r.shown(&held, &mut wire);
    assert_eq!(wire, vec![[0xb0, 2, 0], [0xb0, 34, 1]]);
    // A move too small even for that says nothing.
    held.set("opacity 1", 1.0 / 16383.0 + 1e-6);
    r.shown(&held, &mut wire);
    assert_eq!(wire, Vec::<[u8; 3]>::new());
}

/// A control this program cannot read is skipped rather than zeroed. `tap` is
/// the permanent case — a beat has no state — and darkening a pad because a
/// value could not be read would be the surface asserting something about the
/// deck.
#[test]
fn a_control_with_nothing_to_show_is_skipped_rather_than_darkened() {
    let mut r = router("note 61 -> tap\ncc 1 -> gain 0");
    let held = Held::default().at("gain 0", 1.0);
    let mut wire = Vec::new();
    r.shown(&held, &mut wire);
    assert_eq!(
        wire,
        vec![[0xb0, 1, 127]],
        "a control with no value was sent"
    );
}
