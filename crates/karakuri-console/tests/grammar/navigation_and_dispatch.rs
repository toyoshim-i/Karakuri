#![allow(unused_imports)]

use super::grammar_common::*;

// ---------------------------------------------------------------------------
// The table
// ---------------------------------------------------------------------------

/// Verifies valid key routes per bay, checking universal fold bindings (`focus::ANY`) and bay contents.
#[test]
fn the_dispatch_table_is_the_nine_bays_the_record_walks() {
    let reaches = focus::reaches();
    assert_eq!(
        reaches,
        vec![
            (focus::ANY, Grammar::Space),
            ("transport", Grammar::Digit),
            ("transport", Grammar::Arrows),
            ("transport", Grammar::Space),
            ("transport", Grammar::Enter),
            ("library", Grammar::Digit),
            ("library", Grammar::Arrows),
            ("library", Grammar::Space),
            ("library", Grammar::Enter),
            ("staging", Grammar::Digit),
            ("staging", Grammar::Arrows),
            ("staging", Grammar::Enter),
            ("program", Grammar::Digit),
            ("program", Grammar::Arrows),
            ("program", Grammar::Space),
            ("inspector", Grammar::Digit),
            ("inspector", Grammar::Arrows),
            ("inspector", Grammar::Space),
            ("inspector", Grammar::Enter),
            ("mixer", Grammar::Digit),
            ("mixer", Grammar::Arrows),
            ("mixer", Grammar::Space),
            ("mixer", Grammar::Enter),
            ("master", Grammar::Digit),
            ("master", Grammar::Arrows),
            ("master", Grammar::Space),
            ("master", Grammar::Enter),
            ("sequencer", Grammar::Digit),
            ("sequencer", Grammar::Arrows),
            ("sequencer", Grammar::Space),
            ("sequencer", Grammar::Enter),
            ("outputs", Grammar::Digit),
            ("outputs", Grammar::Arrows),
            ("outputs", Grammar::Space),
        ],
        "the grammar the console declares is not the one the records walk. `enter` reaches \
         nothing in three of the nine — no item there performs — and `space` reaches nothing in \
         Staging below the fold, which is ADR-0259's own finding about a lane of things that \
         happened"
    );
    assert_eq!(
        focus::BUILT.len(),
        9,
        "the manual's *What each region is standing on* lists nine bays and the dispatch table \
         has a different number of rows — a bay absent from it is a bay the four keys decline in"
    );
}

/// Asserts which bays use all active navigation keys vs partial sets (ADR-0259, ADR-0343, ADR-0350-0352).
#[test]
fn six_bays_use_all_four_of_the_keys_that_act_and_three_do_not() {
    let all: Vec<&str> = focus::BUILT
        .iter()
        .filter(|bay| Grammar::ALL.iter().all(|key| bay.reaches(*key)))
        .map(|bay| bay.bay)
        .collect();
    assert_eq!(
        all,
        vec![
            "transport",
            "library",
            "inspector",
            "mixer",
            "master",
            "sequencer"
        ],
        "which bays answer all four of the acting keys has changed. ADR-0259 found that none \
         did; the Library's star, the Mixer's `go`, the Sequencer's lane chooser, the \
         Transport's two cards and the Master chain's list are what made five of them, and the \
         Inspector's parameter rows are a level that also performs. A seventh is a finding to \
         write down rather than one to pass quietly"
    );
}

// ---------------------------------------------------------------------------
// A digit
// ---------------------------------------------------------------------------

#[test]
fn a_digit_names_the_nth_thing_below_the_address_and_zero_the_head() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    assert_eq!(at(&view, "mixer"), Vec::<usize>::new());

    // `2` is the second strip — deck B — and naming it is the deck selection.
    assert_eq!(
        press(&mut view, &panel, Press::Digit(2)),
        Asked::Emitted(Operation::SelectDeck { deck: 1 }),
        "a digit that names a strip did not name the deck selection with it. ADR-0259: *naming a \
         strip is the deck selection*, which is why that row keeps a key badge"
    );
    assert_eq!(at(&view, "mixer"), vec![2], "the address did not descend");
    assert_eq!(view.selection(), 1, "the selection did not move with it");

    // `3` under it is that strip's third control — the fader. `2 3` is deck B's
    // fader, which is the record's own example.
    assert_eq!(press(&mut view, &panel, Press::Digit(3)), Asked::Moved);
    assert_eq!(at(&view, "mixer"), vec![2, 3]);
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Up)),
        Asked::Stepped {
            level: Level::Fader(1),
            step: Step::Up
        },
        "`2 3` is not deck B's fader — the digits count the controls a strip draws, in the order \
         it draws them: the tally, the trim, the fader, the blend chip and the mask mini"
    );
}

#[test]
fn a_digit_past_what_the_bay_drew_is_refused_and_the_address_stays_put() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    view.mixer.truncate(2);
    let refused = press(&mut view, &panel, Press::Digit(4));
    assert!(
        matches!(refused, Asked::Nothing(_)),
        "deck D was named at a two-strip mixer and was not refused: {refused:?}"
    );
    assert_eq!(
        at(&view, "mixer"),
        Vec::<usize>::new(),
        "the address descended onto a strip the mixer is not drawing. `View::select` is what \
         refuses a deck there is no strip for, and a press it turns down is a press that named \
         nothing"
    );
    assert_eq!(
        view.selection(),
        0,
        "the selection moved on a refused press"
    );
}

/// `0` is the head where a bay draws one and the row itself where it does not,
/// which is ADR-0259's reading of the two headless rows.
#[test]
fn zero_names_the_head_and_a_headless_row_says_it_has_none() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    assert_eq!(press(&mut view, &panel, Press::Digit(HEAD)), Asked::Moved);
    assert_eq!(at(&view, "library"), vec![HEAD]);

    // The Mixer's head is the transition row since ADR-0343, so `0` descends
    // there too — and its first control is the shape pill.
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    assert_eq!(press(&mut view, &panel, Press::Digit(HEAD)), Asked::Moved);
    assert_eq!(at(&view, "mixer"), vec![HEAD]);

    // The Transport draws no head, so `0` reaches nothing and says so — and
    // the address stays at the bay rather than descending onto a rung that is
    // not there.
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "transport");
    let said = press(&mut view, &panel, Press::Digit(HEAD));
    match said {
        Asked::Nothing(why) => assert!(
            why.contains("headless") || why.contains("no head"),
            "a headless row declined `0` without saying it draws no head: {why}"
        ),
        other => panic!("the Transport's `0` descended into a head it does not draw: {other:?}"),
    }
    assert_eq!(at(&view, "transport"), Vec::<usize>::new());
}

/// The Inspector is what proves the address is a path, and it is three rungs: a
/// pane, the thing under it, and the control under that.
#[test]
fn the_inspectors_address_is_three_rungs_deep() {
    let (panel, mut view) = console();
    // `1 2 3` — the first pane's second thing (its one node group, the deck
    // head being the first) and that group's third control.
    walk_to(&mut view, &panel, "inspector", &[1, 2, 3]);
    assert_eq!(at(&view, "inspector"), vec![1, 2, 3]);
    // A node group's controls are its authority chip, its renderer chips and
    // then its parameter rows, so `3` is the first parameter.
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Up)),
        Asked::Emitted(Operation::WriteParam {
            deck: 0,
            param: ParamAt {
                node: Some(NodeAddress {
                    layer: Layer::L1,
                    index: 0
                }),
                key: "radius".to_owned(),
            },
            // 2.0 of `[0, 4]` is halfway, and a tenth of the published range
            // up is 2.4.
            value: ParamValue::Scalar(2.4),
        }),
        "`1 2 3 ↑` did not write the first parameter of the first pane's node a tenth up"
    );
    // And a fourth rung reaches nothing: a parameter's rows are its own.
    let said = press(&mut view, &panel, Press::Digit(1));
    assert!(matches!(said, Asked::Nothing(_)), "{said:?}");
}

// ---------------------------------------------------------------------------
// The arrows, and the axis
// ---------------------------------------------------------------------------

/// The mixer's strips are a row and the library's rows are a column, so each
/// bay answers one pair and declines the other — which is ADR-0259's *the
/// neighbour of the addressed thing, along the axis it is drawn on*.
#[test]
fn the_arrows_walk_a_bays_items_along_the_axis_it_draws_them_on() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Right)),
        Asked::Emitted(Operation::SelectDeck { deck: 1 }),
        "`→` did not walk the strips"
    );
    let up = press(&mut view, &panel, Press::Arrow(Arrow::Up));
    assert!(
        matches!(up, Asked::Nothing(_)),
        "`↑` walked a row of strips: {up:?}"
    );

    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Moved
    );
    assert_eq!(view.cursor_row(), 1, "`↓` did not walk the library's rows");
    let right = press(&mut view, &panel, Press::Arrow(Arrow::Right));
    assert!(
        matches!(right, Asked::Nothing(_)),
        "`→` walked a column of rows: {right:?}"
    );
}

/// Verifies dual-axis arrow navigation in Sequencer (vertical lanes vs horizontal cells) (ADR-0333).
#[test]
fn the_sequencers_lanes_are_a_column_and_its_cells_are_a_row() {
    let sequencer = focus::built("sequencer").expect("the sequencer's grammar");
    assert!(
        !sequencer.across,
        "the Sequencer's lanes are stacked, so `↑↓` walk them"
    );
    assert_eq!(
        Control::Step.answers(),
        focus::Answers::Cells { across: true },
        "a lane's cells are a row, so `←→` walk them — the one control on the panel whose axis \
         is not its bay's"
    );

    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "sequencer");
    let across = press(&mut view, &panel, Press::Arrow(Arrow::Right));
    match across {
        Asked::Nothing(why) => assert!(
            why.contains("column") && why.contains("up and down"),
            "`→` on the Sequencer's lanes declined without naming the axis that works: {why}"
        ),
        other => panic!("`→` walked a column of lanes: {other:?}"),
    }
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Moved
    );

    // And on a cell the two swap over: `↑↓` are the ones with nothing to do.
    walk_to(&mut view, &panel, "sequencer", &[1, 2]);
    let down = press(&mut view, &panel, Press::Arrow(Arrow::Down));
    match down {
        Asked::Nothing(why) => assert!(
            why.contains("row") && why.contains("left and right"),
            "`↓` on a lane's cell declined without naming the axis that works: {why}"
        ),
        other => panic!("`↓` walked a row of cells: {other:?}"),
    }
}

/// The walk starts from the item the bay remembers, at bay level, which is what
/// keeps the two keys the Library already had (ADR-0333).
#[test]
fn the_arrows_walk_from_the_remembered_item_without_a_digit_first() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    assert_eq!(at(&view, "library"), Vec::<usize>::new());
    press(&mut view, &panel, Press::Arrow(Arrow::Down));
    press(&mut view, &panel, Press::Arrow(Arrow::Down));
    assert_eq!(
        view.cursor_row(),
        2,
        "two presses of `↓` with the address at the bay did not walk two rows. Before the \
         grammar these were `up` and `down` and needed nothing pressed first; a walk that \
         declined until a digit had been pressed would take that away"
    );
    assert_eq!(
        at(&view, "library"),
        Vec::<usize>::new(),
        "walking at bay level descended. The arrows move along a level and never into one"
    );
}

/// A walk is not a cycle, which is `View::walk`'s rule arriving at the row of
/// strips: a key held down must not jump the length of it.
#[test]
fn the_strips_are_walked_and_clamped_rather_than_wrapped() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    for _ in 0..8 {
        press(&mut view, &panel, Press::Arrow(Arrow::Right));
    }
    assert_eq!(
        view.selection(),
        3,
        "the walk did not stop at the last strip"
    );
    for _ in 0..8 {
        press(&mut view, &panel, Press::Arrow(Arrow::Left));
    }
    assert_eq!(view.selection(), 0, "the walk did not stop at the first");
}

/// A level takes the arrows and a state does not. A closed list has no axis, so
/// the arrows decline on the three chips and say why.
#[test]
fn the_arrows_step_a_level_and_decline_on_a_state() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(1));
    // The trim is a strip's second control, the tally its first.
    press(&mut view, &panel, Press::Digit(2));
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Stepped {
            level: Level::Trim(0),
            step: Step::Down
        }
    );
    assert!(view.focus().address("mixer").map(|a| a.at()) == Some(&[1, 2][..]));

    // Back up to the strip and onto the tally, which is a state.
    view.focus_up(&panel);
    press(&mut view, &panel, Press::Digit(1));
    let refused = press(&mut view, &panel, Press::Arrow(Arrow::Up));
    assert!(
        matches!(refused, Asked::Nothing(_)),
        "an arrow stepped a residency, whose three values are a closed list rather than a \
         continuum: {refused:?}"
    );
}

/// The four levels the world holds are the host's, and the console says which
/// one and which way rather than reading it — ADR-0333's seam, met in the three
/// bays ADR-0343 adds.
#[test]
fn the_levels_the_world_holds_are_named_and_not_read() {
    let cases: [(&str, &[usize], Level); 3] = [
        ("master", &[1], Level::Out),
        ("transport", &[10], Level::Exposure),
        ("transport", &[3], Level::Offset),
    ];
    for (bay, path, level) in cases {
        let (panel, mut view) = console();
        walk_to(&mut view, &panel, bay, path);
        assert_eq!(
            press(&mut view, &panel, Press::Arrow(Arrow::Up)),
            Asked::Stepped {
                level,
                step: Step::Up
            },
            "`{path:?} ↑` in `{bay}` did not ask the host to step {level:?}"
        );
        assert_eq!(
            press(&mut view, &panel, Press::AltEnter),
            Asked::Stepped {
                level,
                step: Step::Default
            },
            "`alt-enter` on {level:?} is not the value it was declared at"
        );
    }
}

/// A level with nothing behind it says so rather than asking the host to step a
/// value no session is holding — P-0083, on the one control of the three that
/// can be absent while its bay is drawn.
#[test]
fn the_offset_says_so_with_no_session_open() {
    let (panel, mut view) = console();
    view.tracker = None;
    walk_to(&mut view, &panel, "transport", &[3]);
    match press(&mut view, &panel, Press::Arrow(Arrow::Up)) {
        Asked::Nothing(why) => assert!(
            why.contains("audio-in"),
            "an offset with no session did not name the control that opens one: {why}"
        ),
        other => panic!("an offset with no session behind it was stepped: {other:?}"),
    }
}
