use super::*;
use karakuri_console::focus::Step;
use karakuri_console::view::tracker_group;
use karakuri_environment::{audio, mix, Asked};
use karakuri_operation::{BeatSource, GridScale, Operation};
use karakuri_operation_record::{Current, Written};
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

/// Returns the printed help legend text associated with a key name in [`KEYS`].
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

/// Verifies linear incremental gain trim adjustments with floor at 0.0 and unconstrained ceiling (P-0064, ADR-0333).
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

/// Verifies that [`settle`] produces exactly one staging row per changed node
/// with its distinct node address, and replaces previous staging rows completely (ADR-0326).
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
