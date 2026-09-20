use super::*;

/// A stored artifact has a card beside it, and the card says what the `.kir`
/// declares — every param with its range, the capacity range and its default,
/// and the emitted attributes.
///
/// `docs/ir-spec.md`'s metadata section long described records that nothing
/// wrote and nothing read, so every claim it made was unenforced prose. This is
/// the half a compile pass can produce, pinned against a source a reader can
/// check it against by eye.
///
/// Including a negative default, which is the one declaration a second reader
/// of defaults gets wrong — see
/// [`the_engine_and_the_metadata_writer_cannot_disagree_about_a_default`],
/// which is the other half of that and needs a GPU. This one needs none:
/// putting an artifact is a compile and two files.
///
/// Through [`Placed::put`] rather than around it. That is the funnel every
/// stored artifact goes through — the watcher's builds, a recorded run's
/// seeding, and `--save-set` — so a test that assembled the write by hand would
/// be a test of a copy.
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

/// A card that will not write is said out loud and does not fail the save.
///
/// The policy [`Placed::put`] states: the artifact is the thing, its card is
/// derived from the `.kir` plus a compile pass and regenerates on the next one,
/// so a store holding the source and no card holds everything that cannot be
/// recovered — and failing the put instead would lose an operator a save over a
/// file nothing has read yet. Both halves were prose until here, and the half
/// that rots quietly is the second: a `put` that started returning the card's
/// error would be caught by any test that saves, while a `put_meta` that
/// swallowed it would be caught by none.
///
/// The card's own path taken by a directory, because that is the one failure
/// that reaches the card and nothing else. A read-only store root would fail
/// `put_artifact` first and prove the opposite thing. The path is spelled here
/// the way `Store::meta_path` spells it — it is private — as `karakuri-store`'s
/// own metadata tests spell it.
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

/// A build whose sources never reached the store is still a swap.
///
/// The watcher compiles, fails to `put_artifact`, says so, and returns the
/// `Request` anyway — so the build goes on screen with no address anybody can
/// name. `Live::took_up` used to answer that by returning before touching
/// [`Running`] at all, which left the slot reading as the version it was
/// running *before* the one on screen: `k` wrote that version down and a
/// recorded run put `procedure` records naming it into the stream, with nothing
/// anywhere saying the file and the picture had come apart.
///
/// A slot showing a version nobody can name has no address, and saying so is
/// the whole of the repair: `None` is what a save refuses on and what a record
/// is not written from, and both of those are better than a hash that points at
/// the wrong material.
///
/// No GPU and no window: the transition is the whole subject, which is what
/// [`Running`] being its own type is for.
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

    // **And the next build that can be named puts the slot back on an
    // address**, which is what makes `None` a state rather than a dead end:
    // nothing has to remember A for the slot to become savable again.
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

/// A windowed run creates no store until something is saved.
///
/// `docs/manual.md` promises that a run which cannot be edited "copies nothing
/// and creates no directory", and the launch seeding falsified it: it opened
/// the store — which `create_dir_all`s the root and its subdirectories — and
/// wrote one artifact per node, so a plain `karakuri-cli examples/...` left a
/// `.karakuri` behind in whatever directory it was run from.
///
/// The seeding costs nothing because an address is not a file. The second
/// assertion is what makes the first mean something: the slot knows exactly
/// what it is running, it simply has not written it anywhere. The bytes reach
/// the store at the moment a file names them — see [`Sources::into_nodes`],
/// which
/// [`a_rewrite_between_the_compile_and_the_first_frame_cannot_reach_a_save`]
/// reads back off the disk.
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

/// A recorded run puts every slot's launch sources where a replay looks.
///
/// A replay rebuilds each slot by reading its sources back out of the store,
/// and a record naming bytes nobody kept is the same silence as no record at
/// all.
///
/// What the seeding was argued from has moved, and the seeding is left where it
/// is rather than removed on this test's authority: the reader named was a
/// rollback onto the launch version, which emitted `procedure` records naming
/// these hashes, and ADR-0316 removed rollbacks. Whether a recorded run still
/// owes the store its launch bytes is a decision about the record vocabulary
/// and is the maintainer's; what this pins is that the seeding does what it
/// says.
///
/// Slot 1, deliberately. `session_head` writes slot 0's material and says out
/// loud that a session stream cannot describe a deck, so slot 0 would pass on
/// the head's I/O alone and prove nothing about the seeding. A `procedure`
/// record, unlike a head, does address any slot.
///
/// Paired with its control: nothing is in the store before the seeding, so this
/// is about the seeding rather than about a store that had the bytes from
/// somewhere else.
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
    // **The control.** Opening a store does not fill it, so everything
    // below is about the seeding.
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

    // **Slot 1's launch addresses, which are what it is recorded as running
    // until something rebuilds it.**
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

/// Every `Kind` survives the round trip a saved node makes.
///
/// A node's layer is written out by [`setfile::kind_name`] and read back by
/// [`layer_named`], and nothing pinned them as inverses. A sixth `Kind` would
/// have compiled — `kind_name`'s match is exhaustive and would be updated,
/// `layer_named`'s ends in a wildcard and would not — and surfaced as `a node
/// on layer ... cannot be saved` at the moment an operator pressed `k`, which
/// is the worst place in the program to find out.
///
/// The `match` is what makes this exhaustive, not the array. A list can fall
/// one short in silence; a seventh variant stops this file compiling, which is
/// the failure that was wanted in place of the runtime one. The array is
/// `Kind::ALL` so that the walk cannot fall short either — the sixth kind is
/// exactly the case where a hand-written list and the match beside it would
/// have disagreed.
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

/// A slot with nothing in the store behind it saves nothing, rather than
/// writing a file describing no Set.
///
/// The `--load-set` without `--watch` case: the material came out of the store
/// by hash with no files behind it, and `placed` is empty for that slot. There
/// is no version to name, which is a different thing from naming the wrong one,
/// and `Live::save_set` refuses it by name — see the refusal there, which says
/// which set the run came from and what would make the slot savable.
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

/// The two refusals a save can meet name the way out of them.
///
/// Both reach a model now as well as a terminal, which is what makes them worth
/// a test: `Live::save_set` needs a window and a GPU, so the sentences live in
/// free functions and this is where they are checked. The `--mcp` half is a
/// correction that has already been made once — this refusal named `--watch`
/// alone for as long as `--mcp` also made a slot savable, and sent operators to
/// restart a set for a flag they did not need.
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

    // **The range, said without underflowing on a deck with no slots.** The
    // arm that cannot happen is the one that stops saying so quietly.
    assert_eq!(no_such_slot(9, 4), "no slot 9: this deck holds slots 0-3");
    assert!(no_such_slot(0, 0).contains("holds none"));
}

/// A renderer the slot does not draw with, refused in the shared words, and
/// refused at the boundary rather than one past it.
///
/// The sentence is pinned with an `assert_eq!` against [`no_such_renderer`]
/// rather than a `contains`, which is the lesson [`no_such_slot`] paid for:
/// four spellings of one refusal lived side by side because every test asked
/// only whether the range appeared in it. Both surfaces that can meet this —
/// the key press and a replayed `select` record — go through
/// [`renderer_in_range`], so pinning it here pins both.
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

/// A scheduled move on a fader a lane holds is refused here in the one
/// sentence, and this program hands over the reading that decides it.
///
/// Two halves, because the first alone would pass on a surface that never
/// filled the reading: [`answered`] refuses when it is handed a lane, and
/// the `Current` this program builds is scanned for the field being filled
/// at all. Nothing here can hold a fader in practice — there is no
/// sequencer on this surface — so the scan is the only thing that can catch
/// the reading going missing (ADR-0323).
///
/// The sentence is pinned with a `contains` against `Refusal::why` rather
/// than spelled out, which is [`no_such_slot`]'s lesson: a second spelling
/// of one refusal is what an operator meets as two explanations for one
/// mistake. The same sentence goes back over `--mcp`, because a model and a
/// terminal are answered from this one string.
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
    // **And the whole sentence, pinned to the one function that spells
    // it.** The instrument's own `--mcp` drain answers a refusal through
    // `not_performed` too and pins it the same way
    // (`karakuri::tests::a_model_is_answered_the_refusal_rather_than_told_its_move_was_performed`),
    // so the two programs cannot answer one refusal in two sentences
    // without one of these two tests failing (ADR-0131). A `contains` on
    // either side alone is what let four spellings of `no_such_slot` live
    // together.
    assert_eq!(
        said,
        not_performed(
            fade.title(),
            &karakuri_operation_record::Refusal { lane: 2, deck: 1 }.why()
        ),
        "this program answered `{said}`, which is not the sentence the instrument \
         answers the same refusal in"
    );
    // And the reading itself: a `Current` with no `lanes` in it refuses
    // nothing, so a surface that stopped filling the field would go on
    // scheduling moves over lanes in silence, with every test above green.
    // Whitespace taken out, so a reformat is not a failing test and a line
    // `cargo fmt` wrapped is not a silence.
    let source: String = SOURCE.split_whitespace().collect();
    assert!(
        source.contains("lanes:Some(karakuri_operation_record::Lanes::default())"),
        "the `Current` this program builds no longer fills the lanes reading — a \
         field left out is a reading nobody took, which refuses nothing"
    );
}
