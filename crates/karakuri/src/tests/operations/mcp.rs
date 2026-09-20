use super::*;

/// What a model asking over `--mcp` is told about an operation this window
/// refused, owed or performed — and *performed* is one of the three answers
/// rather than all of them (ADR-0131, P-0083, ADR-0315).
///
/// The defect this pins was one sentence and no branch: the drain answered
/// ``was performed on the frame it arrived on`` for every operation it took,
/// so a model that asked for a fade on a fader an unmuted lane of the armed
/// pattern holds was told the move was running. Nothing was scheduled, nothing
/// moved, the lane still held the fader, and the one place that said so was
/// this run's terminal — which a model does not have (ADR-0315). It then asked
/// for the next thing.
///
/// Four answers, and the first two are the repair:
///
/// - A refusal is the refusal's own sentence, in the wording
///   `karakuri-cli` answers a refusal in. The `assert_eq!` is against
///   [`not_performed`] rather than a spelling written out here, which is
///   `no_such_slot`'s lesson (ADR-0131): four spellings of one refusal lived
///   side by side because every test asked only whether the range appeared in
///   it. Pinning both programs to the one function is what makes them one
///   sentence — there is no second string to drift from.
/// - A gap is the gap's sentence and not the refusal's, which is
///   [`unwritten`]'s distinction carried onto the socket.
/// - A record is *performed*, unchanged.
/// - **And so is a `Silent`**, which is where this program's answer differs
///   from `karakuri-cli`'s and is deliberate: that program performs an
///   operation by writing records, so a `Silent` is one it has no control for;
///   this one has the control. `SelectDeck` moves the ring, and answering
///   *nothing on this run changed* for it would be this fix writing the defect
///   it repairs the other way round.
///
/// The fifth half is the wiring, because none of the four enters the drain.
/// `App::operated` takes a `Gfx` and an `ActiveEventLoop` and this binary has
/// neither, so the source is scanned for the conversion being carried out of
/// `App::performed` and for the drain consulting it — without that, every
/// assertion above passes against a function nothing calls, which is
/// `docs/contributing.md` §3's whole subject.
#[test]
fn a_model_is_answered_the_refusal_rather_than_told_its_move_was_performed() {
    let fade = Operation::FadeDeck { deck: 1, to: 0.0 };
    let held = Current {
        transition: Some(karakuri_operation_record::Transition {
            start: 8.0,
            beats: 4.0,
            curve: karakuri_operation::Curve::Smooth,
            wipe_kind: karakuri_operation::WipeKind::None,
            wipe_angle: 0.0,
        }),
        lanes: Some(karakuri_operation_record::Lanes {
            held: vec![(3, karakuri_operation::LaneTarget::Fader { deck: 1 })],
        }),
        ..Current::default()
    };
    let refusal = karakuri_operation_record::Refusal { lane: 3, deck: 1 };
    let refused = written(&fade, &held);
    assert_eq!(
        refused,
        Written::Refused(refusal),
        "a fade onto a deck whose fader a lane holds was converted into records, so the \
         answer this test is about is not the one being asserted"
    );
    let said = unperformed(fade.title(), &refused).expect(
        "a fade this window refused answered a model nothing at all, so the drain falls \
         through to `was performed` for a move that was never scheduled",
    );
    assert_eq!(
        said,
        not_performed(fade.title(), &refusal.why()),
        "the window answered `{said}`, which is not the sentence `karakuri-cli` answers \
         the same refusal in — one mistake, one explanation, whichever program a model \
         came through"
    );
    assert!(
        !said.contains("was performed"),
        "a refused move was reported to a model as performed: `{said}`"
    );

    // **A gap, and it is not the refusal's sentence.** A scrub with no
    // transport read is the reading this window fails to take when the deck it
    // names is not one it holds.
    let scrub = Operation::ScrubDeck {
        deck: 0,
        beats: 0.25,
    };
    let gap = written(&scrub, &Current::default());
    assert_eq!(
        gap,
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Transport)),
        "a scrub with no transport read no longer owes a record, so the gap this test is \
         about is not the one being asserted"
    );
    let owed = unperformed(scrub.title(), &gap)
        .expect("a scrub this window could not convert answered a model nothing at all");
    assert_eq!(
        owed,
        not_performed(
            scrub.title(),
            Owed::NotRead(karakuri_operation_record::Reading::Transport).why()
        ),
        "the window answered `{owed}`, which is not the words the crate that owes the \
         record says the gap in"
    );
    assert_ne!(
        owed, said,
        "a decision taken and a gap nobody has closed went back over the socket as the \
         same sentence"
    );

    // **A record is performed, and so is a `Silent`.** `None` here is the
    // drain falling through to the sentence that says so.
    let gain = Operation::SetGain { deck: 0, gain: 0.5 };
    assert_eq!(
        unperformed(gain.title(), &written(&gain, &Current::default())),
        None,
        "an operation that wrote a record was answered as though nothing on this run \
         changed"
    );
    let select = Operation::SelectDeck { deck: 0 };
    let silent = written(&select, &Current::default());
    assert_eq!(
        silent,
        Written::Silent(Silent::Surface),
        "a deck selection no longer writes no record, so the arm this test is about is \
         not the one being asserted"
    );
    assert_eq!(
        unperformed(select.title(), &silent),
        None,
        "an operation this window performs on its own surface — the ring moves — was \
         answered `nothing on this run changed`, which is the defect this test is about \
         written the other way round"
    );

    // **And the drain, read out of its own source**, because nothing above
    // enters it: the conversion has to be carried out of `App::performed` and
    // consulted before the `Ok` is built, and either half missing is four
    // green assertions over a function with no caller.
    // Read with the whitespace taken out, so that a reformat of the file is
    // not a failing test and a line wrapped by `cargo fmt` is not a silence.
    const APP: &str = include_str!("../../app/mod.rs");
    let app: String = APP.split_whitespace().collect();
    for wanted in [
        // Carried out of the performer, at the one place the readings the
        // conversion needs are true (ADR-0323).
        "converted=Some(written);",
        // And consulted by the drain, ahead of the sentence that says it was
        // performed.
        "unperformed(title,written)",
        "reply.settled(Err(refused));",
    ] {
        assert!(
            app.contains(wanted),
            "`App::operated` no longer carries `{wanted}` — the outcome not reaching the \
             reply is a model told its refused move was performed, with every assertion \
             in this test still green"
        );
    }
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
