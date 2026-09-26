use super::*;

/// Verifies that a stored artifact has an accompanying metadata card declaring its parameters and types.
#[test]
fn a_stored_artifact_has_a_card_saying_what_its_source_declares() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = vec![
        kir(&dir, "a.kir", &signed_l1()),
        kir(&dir, "r.kir", &example("soft_points.kir")),
    ];
    let (_material, placed) = slot(&paths);
    let store =
        karakuri_store::store::Store::open(dir.path().join("store")).expect("a store opens");

    let hash = placed[0].put(&store).expect("the artifact is stored");
    let card = store
        .read_meta(&hash)
        .expect("an artifact that was put has a card beside it");
    let records: Vec<Record> = card.iter().map(|l| l.record().clone()).collect();

    assert_eq!(
        records.first(),
        Some(&Record::Meta {
            hash,
            name: "drift_shell".to_string(),
            kind: Layer::L1,
            v: 1,
        }),
        "a card's head names the artifact it describes and the procedure's \
         own name"
    );

    let declared: Vec<(&str, f32, f32, Option<f32>)> = records
        .iter()
        .filter_map(|r| match r {
            Record::ParamDecl {
                key,
                ty,
                min,
                max,
                default,
            } => {
                assert_eq!(ty, "float", "`{key}` is declared a float in the source");
                Some((key.as_str(), *min, *max, *default))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        declared,
        vec![
            ("radius", 0.1, 8.0, Some(2.6)),
            ("turbulence", 0.0, 3.0, Some(1.1)),
            ("swirl", 0.0, 2.0, Some(0.35)),
            ("drift", 0.0, 2.0, Some(0.6)),
            // The one the fold exists for: `= -0.35` is a negation of a
            // literal, and a card that read `Expr::Lit` alone would carry
            // no `default` here at all.
            ("signed", -1.0, 1.0, Some(-0.35)),
        ],
        "the params a card declares are not the source's, in the source's order"
    );

    assert!(
        records.contains(&Record::CapacityDecl {
            min: 4096,
            max: 1048576,
            default: 262144,
        }),
        "the declared capacity range is not on the card: {records:?}"
    );
    assert!(
        records.contains(&Record::Emit {
            attrs: vec![
                "position".to_string(),
                "velocity".to_string(),
                "age".to_string(),
            ],
        }),
        "the emitted attributes are not on the card: {records:?}"
    );
}

/// Asserts that a failure to write a metadata card logs a warning but does not
/// fail the save operation itself.
#[test]
fn a_card_that_will_not_write_is_said_and_does_not_fail_the_save() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = vec![
        kir(&dir, "a.kir", &example("drift_shell.kir")),
        kir(&dir, "r.kir", &example("soft_points.kir")),
    ];
    let (_material, placed) = slot(&paths);
    let root = dir.path().join("store");
    let store = karakuri_store::store::Store::open(&root).expect("a store opens");

    let hash = placed[0].hash();
    std::fs::create_dir(root.join(format!("{}.meta.ndjson", hash.short(64))))
        .expect("the card's path is taken");

    assert_eq!(
        placed[0]
            .put(&store)
            .expect("the save is not failed by a card"),
        hash,
        "a put that survived its card came back with another artifact"
    );
    assert!(
        store.get_artifact(&hash).is_ok(),
        "the source did not reach the store, so nothing here was recoverable"
    );
    assert!(
        store.read_meta(&hash).is_err(),
        "the card was written after all, and this test proves nothing"
    );

    // The same call the put makes, read back rather than watched on a
    // terminal — see [`put_meta`], which returns what it printed for this.
    let said =
        put_meta(&store, &hash, &placed[0].meta).expect("a card that would not write is reported");
    assert!(
        said.contains(&hash.short(12)),
        "the operator is not told which artifact: {said}"
    );
    assert!(
        said.contains("the artifact is stored"),
        "the operator is not told the save survived: {said}"
    );
}

/// Verifies that a build whose sources never reached the store is still recorded as a swap with None address.
#[test]
fn a_build_whose_sources_never_reached_the_store_is_still_a_swap() {
    let dir = tempfile::tempdir().expect("tempdir");
    let at_start = example("drift_shell.kir");
    let renderer = example("soft_points.kir");
    let paths = vec![kir(&dir, "a.kir", &at_start), kir(&dir, "r.kir", &renderer)];
    let (_material, placed) = slot(&paths);
    let root = dir.path().join("store");
    let store = karakuri_store::store::Store::open(&root).expect("store");

    let launch: Vec<Landed> = vec![
        ("L1", 0, karakuri_store::hash::Hash::of(at_start.as_bytes())),
        ("L4", 0, karakuri_store::hash::Hash::of(renderer.as_bytes())),
    ];

    let mut running = launched(&placed);
    assert_eq!(
        running.playing(0),
        Some(&launch),
        "the slot did not start on what it launched with, so nothing below \
         is about the transition"
    );

    // Build A: it compiled, the watcher stored it, the swap landed.
    let build_a = at_start.replace("proc drift_shell", "proc build_a");
    let a: Vec<Landed> = vec![
        stored(&store, "L1", &build_a),
        stored(&store, "L4", &renderer),
    ];
    running.landed(0, Some(a.clone()));

    assert_eq!(
        running.playing(0),
        Some(&a),
        "a build that was stored is not what the slot reads as running"
    );

    // Build B: it compiled, the store refused it, and the deck installed it
    // regardless — which is what `watch::Watch::poll` does, and what makes
    // this a state the run can actually be in.
    running.landed(0, None);
    assert!(
        running.playing(0).is_none(),
        "a slot showing a version nothing can name reported an address, and \
         whatever it reported is not what is on screen"
    );

    // Subsequent named builds restore concrete slot addresses.
    let c: Vec<Landed> = vec![
        stored(
            &store,
            "L1",
            &at_start.replace("proc drift_shell", "proc build_c"),
        ),
        stored(&store, "L4", &renderer),
    ];
    running.landed(0, Some(c.clone()));
    assert_eq!(
        running.playing(0),
        Some(&c),
        "a build after an unnameable one did not put the slot back on an address"
    );
}

/// Verifies that a windowed run creates no store directory until something is explicitly saved.
#[test]
fn a_windowed_run_creates_no_store_until_something_is_saved() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = vec![
        kir(&dir, "a.kir", &example("drift_shell.kir")),
        kir(&dir, "r.kir", &example("soft_points.kir")),
    ];
    let (_material, placed) = slot(&paths);
    let root = dir.path().join("store");

    let running = launched(&placed);
    assert!(
        !root.exists(),
        "a run that was asked to draw a window and nothing else created a \
         store, which `docs/manual.md` promises it does not"
    );
    assert_eq!(
        running.playing(0).map(Vec::len),
        Some(2),
        "the slot has no address, so `k` would refuse — which is not the \
         way to create no directory"
    );
}

/// Asserts that a recorded run writes the launch sources of all slots to the store
/// so they remain available for replays.
#[test]
fn a_recorded_run_puts_its_launch_sources_where_a_replay_looks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let at_start = example("drift_shell.kir");
    let renderer = example("soft_points.kir");
    // Slot 1's geometry is a different procedure, so the two slots are two
    // addresses rather than one shared by a content-addressed store.
    let other = at_start.replace("proc drift_shell", "proc slot_one_shell");
    let first = vec![kir(&dir, "a.kir", &at_start), kir(&dir, "r.kir", &renderer)];
    let second = vec![kir(&dir, "b.kir", &other), kir(&dir, "s.kir", &renderer)];
    let (_m0, slot0) = slot(&first);
    let (_m1, slot1) = slot(&second);
    let placed = vec![slot0, slot1];

    let root = dir.path().join("store");
    let store = karakuri_store::store::Store::open(&root).expect("store");
    // Control test: verifying store state before seeding.
    for nodes in &placed {
        for node in nodes {
            assert!(
                store.get_artifact(&node.hash()).is_err(),
                "the store already held this run's sources, so the seeding \
                 below could not be what put them there"
            );
        }
    }

    seed_store_for_replay(&store, &placed);

    // Initial launch addresses for slot 1.
    let running = Running::at_launch(&placed, 2);
    let at_launch = running
        .playing(1)
        .expect("slot 1 launched with files behind it")
        .clone();
    for (_, _, hash) in &at_launch {
        assert!(
            store.get_artifact(hash).is_ok(),
            "a record naming slot 1's material names a source this session's \
             replay cannot resolve, so the replay rebuilds nothing where the \
             run showed the version it launched with"
        );
    }
}

/// Verifies that every `Kind` variant round-trips correctly through `setfile::kind_name` and `layer_named`.
#[test]
fn every_kind_survives_the_round_trip_a_saved_node_makes() {
    use karakuri_ir::Kind;
    for kind in Kind::ALL {
        match kind {
            Kind::L1 | Kind::L2 | Kind::L3 | Kind::L4 | Kind::Field | Kind::L5 => {}
        }
        assert_eq!(
            layer_named(setfile::kind_name(kind)),
            Some(kind),
            "a node on this layer is written into a Set file under a name \
             nothing reads back, so saving it refuses at the key press"
        );
    }
}

/// Verifies that a slot with no backing files produces no sources and creates no store directory.
#[test]
fn a_slot_with_no_sources_of_its_own_has_nothing_to_save() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("store");
    let running = Running::at_launch(&[Vec::new()], 1);
    assert!(
        live_sources(running.playing(0), &[]).is_empty(),
        "a slot with no files behind it has no sources to write"
    );
    assert!(
        !root.exists(),
        "a deck with no files behind it opened a store to write nothing into"
    );
}

/// Verifies that unperformable save operations return clear refusal messages detailing causes and remediation.
#[test]
fn a_save_that_cannot_happen_says_why_and_what_would_change_it() {
    let loaded = nothing_to_save(2, Some("night01"), true);
    assert!(loaded.contains("`night01`"), "{loaded}");
    assert!(loaded.contains("--watch"), "{loaded}");
    assert!(
        loaded.contains("--mcp"),
        "a slot `--mcp` alone would have made savable was told to restart for \
         `--watch`: {loaded}"
    );

    // A slot that has files behind it and nothing in the store is a
    // different fact, and neither flag is the answer to it.
    let unstored = nothing_to_save(2, Some("night01"), false);
    assert!(
        !unstored.contains("night01"),
        "a slot with files of its own was told it came out of a set file: {unstored}"
    );

    // Range refusal formatting without underflow on empty deck.
    assert_eq!(no_such_slot(9, 4), "no slot 9: this deck holds slots 0-3");
    assert!(no_such_slot(0, 0).contains("holds none"));
}

/// Verifies that selecting an out-of-range renderer is refused with the valid index range.
#[test]
fn a_renderer_a_slot_does_not_draw_with_is_refused_with_the_range_it_missed() {
    assert_eq!(
        renderer_in_range(1, 3, 3).expect_err("renderer 3 of three"),
        "no renderer 3: slot 1 draws with renderers 0-2"
    );
    // The boundary either side of it, which is where the off-by-one would
    // live: the last renderer is `count - 1` and it is legal.
    assert!(renderer_in_range(1, 2, 3).is_ok());
    assert!(renderer_in_range(0, 0, 1).is_ok());
    assert!(renderer_in_range(0, 1, 1).is_err());
    // And the arm that cannot happen, said without underflowing.
    assert_eq!(
        no_such_renderer(0, 0, 0),
        "no renderer 0: slot 0 draws with none"
    );
}

/// Verifies that scheduled operations targeting held lanes are consistently refused.
#[test]
fn a_move_a_lane_holds_is_refused_here_in_the_one_sentence() {
    let current = Current {
        transition: Some(karakuri_operation_record::Transition {
            start: 12.0,
            beats: 4.0,
            curve: karakuri_operation::Curve::Smooth,
            wipe_kind: karakuri_operation::WipeKind::None,
            wipe_angle: 0.0,
        }),
        lanes: Some(karakuri_operation_record::Lanes {
            held: vec![(2, karakuri_operation::LaneTarget::Fader { deck: 1 })],
        }),
        ..Current::default()
    };
    let fade = Operation::FadeDeck { deck: 1, to: 0.0 };
    let said = answered(&fade, &current).expect_err(
        "a fade onto a deck whose fader a lane holds \
                                              was performed",
    );
    assert!(
        said.contains(&karakuri_operation_record::Refusal { lane: 2, deck: 1 }.why()),
        "this surface said `{said}`, which is not the sentence the refusal is \
         worded in — one mistake, one explanation, whichever surface meets it"
    );
    // Verify refusal message matches `not_performed` format across tools (ADR-0131).
    assert_eq!(
        said,
        not_performed(
            fade.title(),
            &karakuri_operation_record::Refusal { lane: 2, deck: 1 }.why()
        ),
        "this program answered `{said}`, which is not the sentence the instrument \
         answers the same refusal in"
    );
    // Verifies that the Current struct constructed in this program populates `lanes`.
    let source: String = SOURCE.split_whitespace().collect();
    assert!(
        source.contains("lanes:Some(karakuri_operation_record::Lanes::default())"),
        "the `Current` this program builds no longer fills the lanes reading — a \
         field left out is a reading nobody took, which refuses nothing"
    );
}
