use super::*;

/// different one and no flag can stand in for it: what reaches the file is what
/// is on screen, after a parameter has moved and after the file under a slot
/// has moved without it. So these build a real Set, disturb it the way a run
/// disturbs one, and read the file back through the loader `--load-set` uses.
#[cfg(test)]
mod live_save_tests {
    use super::*;
    // The card writer, reached only by the test that pins what it says when a
    // disk refuses it.
    use karakuri_environment::meta::put_meta;
    use std::path::Path;

    /// Integration tests get the *package* as their working directory, and unit
    /// tests get it too — so `examples/` is two levels up from here.
    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root")
    }

    fn example(name: &str) -> String {
        std::fs::read_to_string(workspace().join("examples").join(name)).expect("an example")
    }

    /// Write `src` into `dir` and hand back the path, so a test can edit a
    /// procedure the way an operator does — by replacing the file.
    fn kir(dir: &tempfile::TempDir, name: &str, src: &str) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, src).expect("write");
        path
    }

    /// One slot's files, compiled and sorted exactly as a run compiles them — so
    /// `Placed` here carries the layers and indices the real thing carries.
    fn slot(paths: &[PathBuf]) -> (Material, Vec<Placed>) {
        let rest: Vec<Named> = paths[1..].iter().cloned().map(Named::bare).collect();
        sort_slot(0, &Named::bare(paths[0].clone()), &rest)
    }

    /// A Set built from that material, at the capacities, salts and camera given —
    /// which is what a run hands `build`, and what a save has to read back off the
    /// Set rather than off these arguments.
    ///
    /// `camera` is an `Option` because `build`'s is: `None` is a slot no `camera`
    /// record reached, which leaves the Set at the built-in orbit's defaults. See
    /// `recorded_camera`.
    fn set_of(
        gpu: &Gpu,
        material: &Material,
        capacities: &[u32],
        salts: &[u32],
        overrides: &[ParamWrite],
        camera: Option<karakuri_engine::camera::Orbit>,
    ) -> Set {
        let mut attached = vec![false; 0];
        build(
            gpu,
            &material.l1s,
            &material.l2s,
            &material.l3s,
            &material.fields,
            &material.l4s,
            karakuri_engine::set::Layering::Overdraw,
            &material.names,
            &[],
            capacities,
            &mut attached,
            overrides,
            &[],
            &[],
            salts[0],
            salts,
            camera,
            // No selection: these Sets overdraw, and a fold nobody built has
            // no renderer to be folded to. See `recorded_live`.
            None,
        )
    }

    /// What a run does to one slot before its first frame: the slot recorded as
    /// running the material that was compiled for it.
    ///
    /// Through [`Running::at_launch`] rather than around it, for the reason
    /// [`save_and_load`] goes through [`Save::run`]: assembling the seeding by hand
    /// here would be a second copy of it, and the whole of what these tests are
    /// about is that there is only one.
    ///
    /// No store root, because there is no store. A windowed run touches one when a
    /// save happens and not before — see [`Running::at_launch`].
    fn launched(placed: &[Placed]) -> Running {
        Running::at_launch(&[placed.to_vec()], 1)
    }

    /// One node as the watcher hands it over when a build lands: the source in the
    /// store, and its address beside the hash.
    fn stored(store: &karakuri_store::store::Store, layer: &'static str, src: &str) -> Landed {
        (layer, 0, store.put_artifact(src.as_bytes()).expect("put"))
    }

    type Landed = (&'static str, u32, karakuri_store::hash::Hash);

    /// Gather, write, and read back — the whole of what pressing `k` does, minus
    /// the thread and the channel.
    ///
    /// Through [`Save::run`] rather than around it, so that what a test exercises
    /// is the function the spawned thread calls. Assembling the same three steps by
    /// hand here would be a second copy of the save, and a test of a copy is a test
    /// of nothing.
    fn save_and_load(store_root: &Path, id: &str, set: &Set, sources: Sources) -> setfile::Loaded {
        Save {
            slot: 0,
            asked: Asked::Operator,
            id: id.to_string(),
            root: store_root.to_path_buf(),
            sources,
            values: playing_values(set, &[]),
        }
        .run()
        .expect("the Set file is written");
        let store = karakuri_store::store::Store::open(store_root).expect("store");
        setfile::load(&store, id).expect("and reads back")
    }

    /// A geometry with a negative default, which is the declaration the fold exists
    /// for and the one no example in the tree carries.
    ///
    /// `drift_shell` with one param added and read, so the procedure still
    /// compiles, still draws, and now declares a default that is `Unary { Neg, Lit
    /// }` rather than a literal.
    fn signed_l1() -> String {
        let src = example("drift_shell.kir");
        let with_param = src.replace(
            "  param drift      : float [0.0, 2.0] = 0.6",
            "  param drift      : float [0.0, 2.0] = 0.6\n  \
             param signed     : float [-1.0, 1.0] = -0.35",
        );
        assert_ne!(with_param, src, "the example's params moved");
        let with_use = with_param.replace(
            "    position = p;",
            "    position = p + vec3(signed, 0.0, 0.0);",
        );
        assert_ne!(with_use, with_param, "the example's element block moved");
        with_use
    }

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
        let said = put_meta(&store, &hash, &placed[0].meta)
            .expect("a card that would not write is reported");
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

    /// This file's own source, at compile time. The scanner below reads
    /// [`Live::key`] out of it rather than being told what the keys are — the shape
    /// `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs` set: read the
    /// checked-in source, and carry a floor so the scan cannot silently match
    /// nothing.
    const SOURCE: &str = include_str!("../live/interactive.rs");

    /// How [`BINDINGS`] spells the arms whose pattern is a name rather than a
    /// character. Nothing in `Key::Named(NamedKey::Escape)` says `esc`, so this one
    /// mapping cannot be derived and is stated — but it is stated as a *table*, and
    /// a `NamedKey` arm missing from it fails
    /// [`every_key_the_live_path_acts_on_is_documented`] by name rather than being
    /// passed over. That is the whole difference from the list of characters that
    /// used to be here: a named key added tomorrow is a test failure that says
    /// which key, not a silence.
    const NAMED_KEY_SPELLINGS: &[(&str, &str)] = &[("Escape", "esc"), ("Space", "space")];

    /// The end of the character literal starting at `at`, or `None` if what is
    /// there is not one.
    ///
    /// A lifetime has to be told from a literal — `'a` is one, `'a'` and `'\n'` are
    /// the other — because getting it wrong lets a `'"'` open a string that
    /// swallows the rest of the file. `'\''` is why an escape cannot simply look
    /// for the next quote: the escaped quote *is* the next quote.
    fn char_literal_end(src: &str, at: usize) -> Option<usize> {
        let b = src.as_bytes();
        if b.get(at) != Some(&b'\'') {
            return None;
        }
        if b.get(at + 1) == Some(&b'\\') {
            // A two-byte escape — `\\`, `\'`, `\n`, `\0` — closes at `at + 3`.
            if b.get(at + 3) == Some(&b'\'') {
                return Some(at + 4);
            }
            // `\x41`, `\u{2026}`: the first quote after the escape. Bounds
            // first, so a file ending mid-literal is `None` and not a panic.
            if at + 3 > src.len() {
                return None;
            }
            return src[at + 3..].find('\'').map(|n| at + 3 + n + 1);
        }
        let c = src[at + 1..].chars().next()?;
        let close = at + 1 + c.len_utf8();
        (b.get(close) == Some(&b'\'')).then_some(close + 1)
    }

    /// The character a literal's inside spells, the way rustc reads it.
    ///
    /// Panics rather than returning nothing on a spelling it does not know: a key
    /// quietly dropped here is a key quietly undocumented, which is the exact
    /// failure this file is closing.
    fn unescape(lit: &str) -> char {
        let mut cs = lit.chars();
        let first = cs.next().expect("a character literal is not empty");
        if first != '\\' {
            return first;
        }
        match cs.next().expect("an escape has a second character") {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            c @ ('\\' | '\'' | '"') => c,
            _ => panic!("`'{lit}'` is a key spelling this scanner cannot read"),
        }
    }

    /// Comments and string literals blanked to spaces, keeping length and line
    /// structure. Character literals are left exactly as written — they are the
    /// payload.
    ///
    /// Load-bearing in both directions. Comments are prose and prose is full of
    /// apostrophes; string literals hold `"{BINDINGS}"` and every message the arms
    /// print. Cut down from the same function in
    /// `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`; this file has no
    /// block comments and no raw strings, and if one arrives the floors below are
    /// what refuses the mis-scan.
    fn blank_comments_and_strings(src: &str) -> String {
        let b = src.as_bytes();
        let mut out = b.to_vec();
        let mut i = 0;
        while i < b.len() {
            if b[i..].starts_with(b"//") {
                let end = src[i..].find('\n').map_or(b.len(), |n| i + n);
                for c in out[i..end].iter_mut() {
                    *c = b' ';
                }
                i = end;
                continue;
            }
            if b[i] == b'"' {
                let start = i;
                i += 1;
                while i < b.len() {
                    match b[i] {
                        b'\\' => i += 2,
                        b'"' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
                let end = i.min(b.len());
                for c in out[start..end].iter_mut() {
                    if *c != b'\n' {
                        *c = b' ';
                    }
                }
                continue;
            }
            if let Some(end) = char_literal_end(src, i) {
                i = end;
                continue;
            }
            i += 1;
        }
        String::from_utf8(out).expect("blanking only ever writes spaces")
    }

    /// [`Live::key`]'s body, blanked, found by its signature.
    ///
    /// The signature and not a line number, and not the name alone — `key` is a
    /// common word. It is spelled with `concat!` so the joined needle exists
    /// nowhere in this file except the function it names: a source scanner that
    /// finds itself is the classic way one of these comes back green. That it
    /// occurs exactly once is asserted, so a renamed or reformatted signature fails
    /// here instead of leaving the scan with nothing to read.
    fn live_key_body() -> String {
        let blanked = blank_comments_and_strings(SOURCE);
        let needle = concat!("fn ", "key(&mut self, key: &Key) -> bool {");
        assert_eq!(
            blanked.matches(needle).count(),
            1,
            "`{needle}` is not in this file exactly once — the scanner is \
             reading nothing, or reading the wrong function"
        );
        let open = blanked.find(needle).expect("just counted one") + needle.len();
        let b = blanked.as_bytes();
        let (mut i, mut depth) = (open, 1usize);
        let end = loop {
            if i >= b.len() {
                panic!("`{needle}` never closes");
            }
            // A braced character literal is a key like any other: stepped over
            // whole, so binding `{` could not open a block here.
            if let Some(skip) = char_literal_end(&blanked, i) {
                i = skip;
                continue;
            }
            match b[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break i;
                    }
                }
                _ => {}
            }
            i += 1;
        };
        blanked[open..end].to_string()
    }

    /// One arm of [`Live::key`]'s match, as its pattern reads.
    enum Arm {
        /// A character it acts on: `'x' => …`.
        Char(char),
        /// A span of them: `'0'..='3' => …`.
        Range(char, char),
        /// A `NamedKey`, whose pattern is a name and not a character at all.
        Named(String),
    }

    /// Every arm of [`Live::key`], read off the pattern side of each `=>` in its
    /// body.
    ///
    /// The pattern side and not the line, because everything after the first `=>`
    /// is the arm's *body* — where `unwrap_or('\0')` lives, and `\0` is not a
    /// binding. Line by line, because a pattern and its `=>` share a line; an arm
    /// body that grew an `=>` of its own would be read as a pattern, which can only
    /// ever demand documentation for a key nobody binds, and that fails loudly
    /// rather than passing quietly.
    fn live_key_arms(body: &str) -> Vec<Arm> {
        let mut arms = Vec::new();
        for line in body.lines() {
            let Some(cut) = line.find("=>") else {
                continue;
            };
            let pattern = &line[..cut];
            for (at, marker) in pattern.match_indices("NamedKey::") {
                let name: String = pattern[at + marker.len()..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    arms.push(Arm::Named(name));
                }
            }
            let mut chars = Vec::new();
            let mut i = 0;
            while i < pattern.len() {
                match char_literal_end(pattern, i) {
                    Some(end) => {
                        chars.push(unescape(&pattern[i + 1..end - 1]));
                        i = end;
                    }
                    None => i += 1,
                }
            }
            // `'0'..='3'` is one binding spanning four keys, not two bindings.
            if pattern.contains("..=") {
                assert_eq!(
                    chars.len(),
                    2,
                    "`{pattern}` is a range with {} endpoints",
                    chars.len()
                );
                arms.push(Arm::Range(chars[0], chars[1]));
            } else {
                arms.extend(chars.into_iter().map(Arm::Char));
            }
        }
        arms
    }

    /// The key column of [`BINDINGS`]: two spaces, the keys, then the gap before
    /// the description. Keys are documented in pairs where they come in pairs — `[
    /// ]`, `u i`, `h ?` — so it is the column that is read and not the first
    /// character of a line.
    ///
    /// A column and never a substring of the whole constant, which is the trap this
    /// text is laid out to defuse: `?` occurs in the prose of the `, .` line, so
    /// `BINDINGS.contains("?")` is true whether or not `?` is bound to anything.
    /// See [`a_key_named_only_in_prose_is_not_documented`].
    fn documented_keys(bindings: &str) -> Vec<&str> {
        bindings
            .lines()
            .filter_map(|line| line.strip_prefix("  "))
            .filter(|line| !line.starts_with(' '))
            .flat_map(|line| line.split("  ").next().unwrap_or_default().split(' '))
            .filter(|key| !key.is_empty())
            .collect()
    }

    /// Every key `Live::key` acts on is in [`BINDINGS`], which is the only thing
    /// standing between a control and being undiscoverable: there is no on-screen
    /// UI, and this text is both what `--help` prints and what `h` does.
    ///
    /// The keys are *read out of the match arms* — see [`live_key_body`] — and not
    /// restated here. The version of this test that restated them iterated a
    /// hard-coded array of 32 characters, so what it enforced was "these 32 keys
    /// are documented"; `'h' | '?'` had been a live arm with no entry in the column
    /// the whole time and this test passed on every run. A check weaker than its
    /// own name is worse than no check, because the name is what stops anyone
    /// looking again — nobody re-reads a green
    /// `every_key_the_live_path_acts_on_is_documented`.
    ///
    /// Which is also why the counts are asserted. A scanner whose pattern stops
    /// matching finds nothing and then passes everything, and that is the same
    /// failure a second time.
    #[test]
    fn every_key_the_live_path_acts_on_is_documented() {
        let documented = documented_keys(BINDINGS);
        let (mut chars, mut ranges, mut named) = (0, 0, 0);

        for arm in live_key_arms(&live_key_body()) {
            match arm {
                Arm::Char(c) => {
                    chars += 1;
                    let key = c.to_string();
                    assert!(
                        documented.contains(&key.as_str()),
                        "`{c}` is a key `Live::key` acts on and has no entry in the \
                         bindings — an operator has no way to find it"
                    );
                }
                // Documented as the span it is, `0-3`, and the span is built
                // from the arm's own endpoints rather than being spelled here.
                Arm::Range(from, to) => {
                    ranges += 1;
                    let key = format!("{from}-{to}");
                    assert!(
                        documented.contains(&key.as_str()),
                        "`{key}` is a range `Live::key` acts on and has no entry in \
                         the bindings"
                    );
                }
                // A named arm carries no character to look for, so its spelling
                // comes from the one table there is — and an unknown name stops
                // the run rather than being skipped.
                Arm::Named(name) => {
                    named += 1;
                    let (_, spelling) = NAMED_KEY_SPELLINGS
                        .iter()
                        .find(|(known, _)| *known == name)
                        .unwrap_or_else(|| {
                            panic!(
                                "`NamedKey::{name}` is a key `Live::key` acts on and \
                                 NAMED_KEY_SPELLINGS does not say how the bindings \
                                 spell it"
                            )
                        });
                    assert!(
                        documented.contains(spelling),
                        "`{spelling}` (`NamedKey::{name}`) has no entry in the bindings"
                    );
                }
            }
        }

        // Floors, not counts. They are what `Live::key` actually holds today —
        // 32 characters, one range, two named keys — rather than a round number
        // under them, because a control surface is small enough that losing one
        // key is news and the scan going quiet is the thing being guarded
        // against. Three of them because they fail apart: a signature change
        // gives no arms at all, a broken literal reader gives named arms and no
        // characters, and a `NamedKey` renamed away gives characters and no
        // named ones. Raise them when a key is added; lowering one is a claim
        // that a control was deliberately removed.
        //
        // **It was 33 and is 32**, and that is the claim being made: ADR-0240
        // retired *Choose what the output shows*, so `v` is not a key any
        // more. The floor came down with the control rather than the control
        // being kept alive to hold a number up.
        assert!(
            chars >= 32,
            "only {chars} character keys read out of `Live::key` — the scan is not \
             seeing the match arms"
        );
        assert!(
            ranges >= 1,
            "no `'a'..='b'` arm read out of `Live::key` — slot focus is one"
        );
        assert!(
            named >= 2,
            "only {named} `NamedKey` arms read out of `Live::key` — escape and space \
             are two"
        );
    }

    /// A character that occurs only in a binding's prose is not documented.
    ///
    /// The reason [`documented_keys`] parses a column instead of asking
    /// `BINDINGS.contains(key)`, and it is not hypothetical: `?` appears inside the
    /// `, .` entry's description, so the substring form of this check would have
    /// called `?` documented while it was bound to nothing. That version would have
    /// looked stronger than the hard-coded array it replaced and enforced less.
    #[test]
    fn a_key_named_only_in_prose_is_not_documented() {
        // `?` twice in prose and never in the column: once in the description
        // beside a key, once on the continuation line under it. Both are places
        // the real text puts it, and each one is a different way the parse
        // could go wrong.
        let prose = concat!(
            "keys:\n",
            "  s          print the status line — the tracker writes ? there when\n",
            "             it is unsure, and ? again if it stays unsure\n",
        );
        // What a substring check sees, and it is a lie.
        assert!(prose.contains('?'));
        assert!(
            !documented_keys(prose).contains(&"?"),
            "a character in a description was counted as a documented key"
        );
        // The column, where a real binding lives, still reads.
        assert!(documented_keys(prose).contains(&"s"));
        // And in the real text `?` is now in the column, not only the prose.
        assert!(documented_keys(BINDINGS).contains(&"?"));
    }

    /// Every method of [`Live`] that writes a record without going through
    /// [`Live::operate`], with the reason its operation cannot convert.
    ///
    /// This is the key handler's form of the single arm [`Live::run_surface`] keeps
    /// for `TapBeat`: a table rather than a comment, so a key that grows a second
    /// derivation of a record is a failing test naming the function instead of a
    /// line nobody reads. Every operation named here answers `Owed::NotSettled`,
    /// which is asserted separately — see
    /// [`the_keys_that_keep_their_own_path_are_the_ones_whose_record_is_not_settled`]
    /// — so an entry that stops being owed fails there rather than lingering here
    /// as a stale excuse.
    const OWED_RECORD_PATHS: &[(&str, &str)] = &[(
        "performed",
        "the route itself: this is where `written`'s records are written, and \
         `Live::operate` is the wrapper over it for a caller with nobody waiting on an \
         answer",
    )];

    /// The method a byte offset falls inside, read off the nearest `fn` above it at
    /// `impl` indentation.
    ///
    /// Four spaces and not any `fn `, because a closure or a nested helper would
    /// otherwise answer for the method it sits in. The needle is spelled with a
    /// leading newline so a `fn` inside an expression cannot match.
    fn enclosing_method(blanked: &str, at: usize) -> &str {
        let (start, prefix_len) = [
            "\n    fn ",
            "\n    pub(super) fn ",
            "\n    pub(crate) fn ",
            "\n    pub fn ",
        ]
        .into_iter()
        .filter_map(|prefix| blanked[..at].rfind(prefix).map(|pos| (pos, prefix.len())))
        .max_by_key(|(pos, _)| *pos)
        .expect("every record written in this file is inside a method");
        let start = start + prefix_len;
        let name_end = blanked[start..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map_or(blanked.len(), |n| start + n);
        &blanked[start..name_end]
    }

    /// A key reaches the deck through [`Live::operate`], or it is one of the owed
    /// ones and says why.
    ///
    /// The route `karakuri-midi` took is *operation → `written` → record*, and the
    /// reason it could take it whole is that every operation a map line produces
    /// converts. A key handler cannot: three of its operations owe a record nobody
    /// can write yet, and routing one of those through `operate` would print the
    /// gap where the gesture used to be. It was seven, and four have left —
    /// `SetSync`, then `FadeDeck`, `Crossfade` and `SelectRenderer` together when
    /// the transition settings became a reading — each a row deleted here rather
    /// than kept, which is what the last loop below is for. So the ones that keep
    /// their own path are named here rather than left to be noticed, and this is
    /// what stops the list growing by accident — a record built beside the
    /// conversion is exactly the drift `karakuri-operation-record` exists to end,
    /// and `mix::gain_record`, `mix::opacity_record` and their neighbours went one
    /// at a time as each operation landed.
    ///
    /// Read out of the checked-in source, in the shape
    /// [`every_key_the_live_path_acts_on_is_documented`] set, with a floor so a
    /// scan that stops matching fails instead of passing everything.
    #[test]
    fn every_record_written_outside_operate_is_a_path_whose_conversion_is_owed() {
        let blanked = blank_comments_and_strings(SOURCE);
        let needle = concat!("self.", "record(");
        let mut reached: Vec<&str> = Vec::new();
        for (at, _) in blanked.match_indices(needle) {
            let name = enclosing_method(&blanked, at);
            assert!(
                OWED_RECORD_PATHS.iter().any(|(known, _)| *known == name),
                "`Live::{name}` writes a record without going through `Live::operate`, \
                 and OWED_RECORD_PATHS does not say why its operation cannot convert — \
                 a surface that derives a record beside the conversion is the drift \
                 `karakuri-operation-record` exists to end"
            );
            if !reached.contains(&name) {
                reached.push(name);
            }
        }
        // A floor rather than a count, and a low one: what is guarded against
        // is the scan going quiet, which would let every direct write through.
        // **One, where it was five, then four, then two.** `cycle_sync` was
        // the fifth; `fade_slot` and `cycle_renderer` were the third and
        // fourth and went together when the transition settings became a
        // reading; `wipe` was the second and went when the front shape joined
        // them. What is left is `operate` itself, which is the route rather
        // than a path around it — so this floor is now as low as it can go,
        // and the loop below is what actually keeps the table honest. A floor
        // above what is left would fail as a dead scan on the day a row was
        // correctly deleted — which is the one failure a guard against a dead
        // scan must not invent.
        assert!(
            !reached.is_empty(),
            "no method reads as a record writer — the scan is not seeing `Live`'s \
             bodies, and `Live::operate` itself is one: {reached:?}"
        );
        // And nothing in the table is a leftover. A path whose last direct
        // write moved to `operate` is a row to delete, not a permission to
        // keep.
        for (name, why) in OWED_RECORD_PATHS {
            assert!(
                reached.contains(name),
                "OWED_RECORD_PATHS excuses `Live::{name}` — {why} — and it writes no \
                 record of its own any more, so the row outlived what it was for"
            );
        }
    }

    /// The keys that keep their own path are exactly the ones whose record is not
    /// settled, and this is what will say so the day one changes.
    ///
    /// `b` and `, .` reach the beat tracker where they stand because `written`
    /// answers `Owed::NotSettled` for the operation each of them names. `y`, `f g`,
    /// `x`, `r` and `c` are the five that have already gone, and every one of them
    /// went the way this test names: `SetSync` stopped being owed when the
    /// conversion took a session tempo, `FadeDeck`, `Crossfade` and
    /// `SelectRenderer` stopped when it took the transition settings, and `Wipe`
    /// stopped when the front shape went over with them and the soft edge turned
    /// out to be the arriving deck's — so each key moved through [`Live::operate`]
    /// and its line here came out, taking `Live::fade_slot` and the last hand-built
    /// record with it. That is a statement about `karakuri-operation-record` rather
    /// than about this file, so it is checked against that crate: the day somebody
    /// settles one of these conversions, this fails and names the key that is now
    /// due to move through [`Live::operate`] — the promise `run_surface`'s
    /// `TapBeat` arm makes, kept by a test rather than by anyone remembering.
    #[test]
    fn the_keys_that_keep_their_own_path_are_the_ones_whose_record_is_not_settled() {
        use karakuri_operation_record::Owed;
        let owed = [
            ("b", Operation::TapBeat),
            (
                ", .",
                Operation::ScaleGrid {
                    by: karakuri_operation::GridScale::Double,
                },
            ),
        ];
        for (keys, operation) in owed {
            assert_eq!(
                karakuri_operation_record::written(&operation, &Current::default()),
                Written::Owed(Owed::NotSettled),
                "`{}` no longer owes its record, so `{keys}` reaches the deck by a \
                 derivation of its own where `Live::operate` would now do — see \
                 `Live::key`'s survey",
                operation.title()
            );
        }
    }

    /// A save still being written when the run ends is waited for, so the `save`
    /// record promised for every save that reached the disk
    /// (`docs/adr/0120-a-record-may-reach-outside-the-stream.md`) is in the stream.
    ///
    /// The window is small and it is exactly the one an operator is in: press `k`,
    /// read that it took, quit. The wait is bounded — see [`SAVE_WAIT`] — and the
    /// bound is what the second half of this checks: a save that never reports back
    /// costs the quit the bound and no more.
    #[test]
    fn a_save_in_flight_at_the_end_of_a_run_is_waited_for() {
        let (tx, rx) = std::sync::mpsc::channel();
        let late = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(120));
            tx.send(Saved {
                slot: 0,
                asked: Asked::Operator,
                id: "late".to_string(),
                outcome: Ok(()),
                // A hand pressed the key; nobody is waiting on a socket.
                reply: None,
            })
        });
        let landed = drained_saves(&rx, 1, Instant::now() + SAVE_WAIT);
        late.join().expect("the save thread").expect("it sent");
        assert_eq!(
            landed.len(),
            1,
            "the run quit before the save it was told to wait for reported back"
        );

        // **The bound, held against a save that never arrives.** The sender is
        // still alive, so there is nothing but the deadline to end this.
        let (_alive, rx) = std::sync::mpsc::channel::<Saved>();
        let started = Instant::now();
        let landed = drained_saves(&rx, 1, started + Duration::from_millis(80));
        assert!(landed.is_empty());
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "a save that never reports back held the quit past its deadline"
        );
    }

    /// Both halves of the save path sit above everything in [`Live::frame`] that
    /// could stop them, which is the whole of what makes an answer to a waiting
    /// client independent of there being a swapchain.
    ///
    /// [`Live::run_requests`] argues this for itself: a client asking to keep what
    /// is playing should not be waiting on a surface. The *outcome* drain used to
    /// sit below `frame::compose`, past three returns, so the argument was made and
    /// then half applied. On a window latched to `frame::Skip::Fault`, or returning
    /// `Outdated` every frame, the request was taken and the save thread wrote the
    /// file successfully — and the client waited out `mcp::SAVE_REPLY` to be told
    /// the outcome was neither success nor failure about a save already on disk,
    /// the terminal never said "saved as set X", and the `save` record promised for
    /// every save that reached the disk was withheld from the stream until the run
    /// quit.
    ///
    /// This used to demand an early return and assert the calls were above it.
    /// There is no longer one: a sink with no target stopped being a reason to
    /// leave the function when a frame stopped belonging to one sink — see
    /// `frame`'s module documentation — so a frame that publishes nowhere now runs
    /// to the end. Demanding a `return` would make this test fail for the reason
    /// the defect it guards was fixed, which is the wrong question. What it asks
    /// instead is the property that outlives either shape: the two calls come above
    /// `frame::compose`, which is the GPU work and everything that has ever been a
    /// reason to bail out, and above the first `return` if one ever comes back. The
    /// second half is dormant today and is the half that matters on the day it is
    /// not.
    ///
    /// Read off the source, and that is the honest description of what this can
    /// reach. `Live::frame` needs a window, a GPU and an event loop, and the two
    /// replay defects this project found both lived in this one function's
    /// statement order
    /// (`docs/adr/0078-a-frame-that-is-discarded-must-not-already-have-been-recorded.md`).
    /// The defect here is a statement order too, and this asserts it where it is,
    /// rather than asserting nothing and calling it untestable. Comment lines are
    /// dropped first, so prose about returning cannot stand in for a `return`.
    #[test]
    fn a_frame_attends_to_its_saves_before_anything_can_stop_them() {
        let source = include_str!("../live/mod.rs");
        let body = source
            .split_once("\n    pub(crate) fn frame(&mut self) {")
            .or_else(|| source.split_once("\n    fn frame(&mut self) {"))
            .expect("`Live::frame` is no longer spelled that way")
            .1;
        let body = body
            .split("\n    fn ")
            .next()
            .expect("the end of `Live::frame`");
        let code: Vec<&str> = body
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        let code = code.join("\n");

        // The end of the function if it never returns early, which is what it
        // does today — every position below is genuinely above it either way.
        let first_return = code.find("return").unwrap_or(code.len());
        let composed = code
            .find("frame::compose(")
            .expect("`Live::frame` no longer composes a frame, and this test is about where");
        for call in ["self.run_requests();", "self.finished_saves();"] {
            let at = code
                .find(call)
                .unwrap_or_else(|| panic!("`Live::frame` no longer calls `{call}`"));
            assert!(
                at < composed,
                "`{call}` is below `frame::compose` in `Live::frame`: a save's outcome has \
                 nothing to do with whether there is a surface to draw on"
            );
            assert!(
                at < first_return,
                "`{call}` is below an early return in `Live::frame`: a frame that publishes \
                 nowhere takes saves and never answers for them"
            );
        }
    }

    // The eight that build a Set to save from. A live save is read out of a running
    // deck, and there is no deck without a device — the ones above test the record
    // and the bookkeeping around what a slot is running, and take none.
    // See `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`.
    mod gpu {
        use super::*;

        /// One fold, so the card and the uniform cannot disagree.
        ///
        /// The number in a `param_decl` and the number the engine loads into a node's
        /// uniform are the same declaration read twice, and until this commit there was
        /// exactly one reader of it — private to `karakuri-engine`, with a note saying
        /// a second evaluator elsewhere would agree with the shader by coincidence. The
        /// metadata writer is that elsewhere. So the fold moved to
        /// `karakuri_ir::Param::default_scalar` and both call it, and this is what
        /// fails if either grows a reader of its own: a card claiming `0.0` where the
        /// run loaded `-0.35` describes a procedure nobody ran, and nothing downstream
        /// could say which of the two was wrong.
        ///
        /// Both directions. Every default the card states must be the value the built
        /// Set is running, *and* every value the Set is running must be stated — one of
        /// those alone passes when a writer silently drops the declaration it cannot
        /// fold.
        ///
        /// The Set is built with no overrides and no Set file, so what a node holds is
        /// exactly what its `.kir` declared.
        #[test]
        fn the_engine_and_the_metadata_writer_cannot_disagree_about_a_default() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let paths = vec![
                kir(&dir, "a.kir", &signed_l1()),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[16384], &[3], &[], None);
            let store = karakuri_store::store::Store::open(dir.path().join("store"))
                .expect("a store opens");
            let hash = placed[0].put(&store).expect("the artifact is stored");

            let mut on_the_card: Vec<(String, f32)> = store
                .read_meta(&hash)
                .expect("a card beside the artifact")
                .iter()
                .filter_map(|l| match l.record() {
                    Record::ParamDecl {
                        key,
                        default: Some(v),
                        ..
                    } => Some((key.clone(), *v)),
                    _ => None,
                })
                .collect();
            let mut in_the_uniform: Vec<(String, f32)> = set
                .params()
                .filter(|(layer, index, ..)| *layer == karakuri_ir::Kind::L1 && *index == 0)
                .map(|(_, _, key, value)| (key.to_string(), value))
                .collect();
            on_the_card.sort_by(|a, b| a.0.cmp(&b.0));
            in_the_uniform.sort_by(|a, b| a.0.cmp(&b.0));

            assert_eq!(
                on_the_card, in_the_uniform,
                "the card and the uniform read the same declaration and came back with \
             different numbers, which means there are two folds again"
            );
            // Not a vacuous agreement: both have to have folded the negation. Two
            // readers that both dropped it would be equal and both wrong.
            assert!(
                on_the_card.contains(&("signed".to_string(), -0.35)),
                "neither reader folded the negation, so they agree about nothing: \
             {on_the_card:?}"
            );
        }
        /// A live save writes a file that loads back into the same material — and does
        /// it for a Set of *two* geometries, which is where the numbers stop being
        /// interchangeable.
        ///
        /// Two geometries at two different capacities and two different salts, because
        /// that is the shape a single number cannot describe: a saver reaching for
        /// `Set::capacity` gets the sum, which is neither geometry's, and the file it
        /// writes is refused for having one capacity where the Set has two.
        /// `Set::source_capacities` is the reading that is per geometry, which is what
        /// the record is.
        #[test]
        fn a_live_save_reads_back_into_the_material_it_was_taken_from() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let l1 = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &l1),
                kir(
                    &dir,
                    "b.kir",
                    &l1.replace("proc drift_shell", "proc drift_two"),
                ),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            // Different from each other and from the declared default, so that a
            // file which recorded either the sum or the declaration is visibly
            // wrong rather than accidentally right.
            let capacities = [8192, 16384];
            let salts = [11, 22];
            let set = set_of(&gpu, &material, &capacities, &salts, &[], None);

            let root = dir.path().join("store");
            let running = launched(&placed);
            let loaded = save_and_load(
                &root,
                "live",
                &set,
                live_sources(running.playing(0), &placed),
            );

            assert_eq!(
                loaded
                    .l1s
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                ["drift_shell", "drift_two"],
                "the geometries came back as something else"
            );
            assert_eq!(
                loaded
                    .l4s
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                ["soft_points"]
            );
            assert_eq!(
                loaded.capacities,
                vec![Some(8192), Some(16384)],
                "each geometry's own capacity is what the file has to carry"
            );
            assert_eq!(
                loaded.salts,
                vec![Some(11), Some(22)],
                "each geometry's own salt is what the file has to carry"
            );
        }
        /// A save after a rebuild records the camera the slot was loaded with, and not
        /// the built-in orbit's defaults.
        ///
        /// This is the whole path and deliberately not a piece of it: a slot aimed by a
        /// `camera` record, the watcher a `--watch` run gives it, an edit to a file,
        /// the build worker, the swap — and then the same `playing_values` the `k` key
        /// reads through. Every link in it was correct on its own while the chain
        /// silently re-aimed the slot, because the one that was missing was the request
        /// in the middle: `Set::build_many` starts every Set from `Orbit::default()`,
        /// so the rebuilt Set was aimed at the defaults and the saver recorded exactly
        /// what it found. That is why the loss stopped being a wrong picture and became
        /// a file — the operator's next preset was written with a camera nobody had
        /// chosen.
        ///
        /// Driven through `HotSwap` rather than by calling the worker's code, for the
        /// reason [`save_and_load`] goes through [`Save::run`]: a rebuild assembled by
        /// hand here would be a second copy of the rebuild, and a test of a copy is a
        /// test of nothing. `begin_frame` is the only place a build is installed, and
        /// it is enough on its own — nothing here has to render.
        #[test]
        fn a_save_after_a_rebuild_records_the_camera_the_slot_was_loaded_with() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let l1 = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &l1),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, _placed) = slot(&paths);
            // Six numbers no default produces. A `camera` record spells two of them
            // and the loader fills the rest from `Orbit::default()`, so a Set file
            // cannot actually deliver these four — they are here because what is
            // under test is the *carrying*, and a value that differs in every field
            // says which fields were carried.
            let aimed = karakuri_engine::camera::Orbit {
                radius: 3.25,
                speed: 0.75,
                height: -1.5,
                fov_y: 0.9,
                near: 0.25,
                far: 250.0,
            };
            let capacity = 8192;
            let salts = [11];
            let set = set_of(&gpu, &material, &[capacity], &salts, &[], Some(aimed));

            // The watcher `build_deck` gives the slot, stating what the slot is
            // running at — the camera among it, from the one reading the Set above
            // was built with.
            let watcher = watch::Watch::new(
                0,
                Named::bare(paths[0].clone()),
                vec![Named::bare(paths[1].clone())],
                karakuri_engine::set::Layering::Overdraw,
                None,
                Some(capacity),
                salts[0],
                salts.to_vec(),
                aimed,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
            let mut swap = HotSwap::new(
                &gpu.device,
                &gpu.queue,
                set,
                DEFAULT_BUDGET_MS,
                Box::new(watcher),
            );

            // The edit an operator makes, in the one form that changes nothing
            // about the material: the watcher compares contents, so a comment is
            // enough to make this a save it wakes on — and it keeps the rebuilt Set
            // the same Set, so the only thing that can differ is what was carried.
            std::fs::write(&paths[0], format!("{l1}\n// an edit\n")).expect("edit the geometry");

            // Bounded, and generous: this covers two poll intervals of debouncing,
            // four compiler stages, two shader modules and a whole-capacity upload,
            // on whatever machine is running the suite.
            let deadline = Instant::now() + Duration::from_secs(30);
            let mut seen: Vec<String> = Vec::new();
            loop {
                swap.begin_frame(&gpu.device);
                let mut swapped = false;
                for event in swap.events() {
                    swapped |= matches!(event, Event::Swapped { .. });
                    seen.push(event.to_string());
                }
                // **Read at the swap and not a frame later.** The outgoing Set is
                // aimed correctly too, so anything that put it back would put the
                // right camera back and hide exactly the defect this is about.
                if swapped {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "the edit never rebuilt; saw {seen:?}"
                );
                std::thread::sleep(Duration::from_millis(5));
            }

            let recorded = playing_values(swap.set(), &[]).camera;
            let six = |o: &karakuri_engine::camera::Orbit| {
                (o.radius, o.speed, o.height, o.fov_y, o.near, o.far)
            };
            assert_eq!(
                six(&recorded),
                six(&aimed),
                "the save after a rebuild would have written a camera the operator never aimed"
            );
        }
        /// A save after a parameter moved records the moved value.
        ///
        /// The Set is built with `radius` at 1.0, which is what a `--param radius=1.0`
        /// would have put there, and then moved to 2.6 the way a record moves it
        /// mid-run. The file has to say 2.6: a writer holding its own copy of what the
        /// run was started with records a number nobody has seen, which is the failure
        /// `saving_capacities` was fixed over, arriving one surface further along.
        ///
        /// Addressed rather than wildcard, because that is what the Set holds — a value
        /// per node — and because it is the only form that can be checked against the
        /// node that has it.
        #[test]
        fn a_live_save_records_a_param_where_the_run_moved_it_to() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let paths = vec![
                kir(&dir, "a.kir", &example("drift_shell.kir")),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            let flag = ParamWrite::everywhere("radius", 1.0);
            let mut set = set_of(
                &gpu,
                &material,
                &[4096],
                &[7],
                std::slice::from_ref(&flag),
                None,
            );
            assert_eq!(
                set.param("radius"),
                Some(1.0),
                "the run started at the flag"
            );

            // What a `param` record does mid-set, through the one entry point both
            // a record and a key press come through.
            set.write_param(&ParamWrite::everywhere("radius", 2.6))
                .expect("every node of a Set nobody has spoken for is manual");

            let root = dir.path().join("store");
            let running = launched(&placed);
            let loaded = save_and_load(
                &root,
                "moved",
                &set,
                live_sources(running.playing(0), &placed),
            );

            let radius: Vec<&ParamWrite> =
                loaded.params.iter().filter(|p| p.key == "radius").collect();
            assert_eq!(
                radius.len(),
                1,
                "one geometry declares `radius`, so one line records it: {radius:?}"
            );
            assert_eq!(
                radius[0].value, 2.6,
                "the file recorded the value the run was started with, not the one it \
             was playing"
            );
            assert_eq!(
                radius[0].at,
                Some((karakuri_ir::Kind::L1, 0)),
                "a param is recorded against the node that declares it"
            );
        }
        // **Two tests stood here and were deleted with the state they were
        // about** (ADR-0316). Both were a save taken after a rollback: a build
        // compiled, was refused for cost, and the engine put the previous Set
        // back — so the path held one version and the picture another, and a
        // save that read the path wrote down material nobody had seen. Nothing
        // is put back now, so in that case the path and the picture agree and
        // neither test could be set up. The rule they held — **a save records
        // the version in the slot, whatever the path holds** — is unchanged and
        // is what the two tests below assert, through the states that do still
        // separate the two: a run with no watcher, and an edit that arrives
        // between the compile and the first frame.

        /// A run with no watcher saves the version it is still drawing, however far the
        /// file underneath it has moved.
        ///
        /// Nothing picks a `.kir` up without `--watch`: an editor writing over the
        /// path, an `--mcp` write with no watcher behind it — which the program's
        /// `mcp.rs` says out loud when it takes one — and an edit that failed to
        /// compile all leave the same state, and it is a state that never resolves on
        /// its own. The disk moved and the picture did not.
        ///
        /// This is why the launch addresses are seeded for every windowed run and not
        /// only an editable one. There is no watcher here to hash anything later, so a
        /// run gated on `editable()` would have nothing to save from and would go on
        /// answering with the path. The bytes behind those addresses reach the store
        /// here, at the save — see
        /// [`a_windowed_run_creates_no_store_until_something_is_saved`] for the other
        /// half of that, which is that nothing reaches it before.
        #[test]
        fn a_run_with_no_watcher_saves_the_version_it_is_still_drawing() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let at_start = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &at_start),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[4096], &[7], &[], None);
            let root = dir.path().join("store");

            let running = launched(&placed);

            // Something else rewrote the file. Nothing in this run is watching it,
            // so no build is requested, nothing lands, and the deck goes on drawing
            // what it built at startup for as long as the run lasts.
            std::fs::write(
                &paths[0],
                at_start.replace("proc drift_shell", "proc edited_outside"),
            )
            .expect("the edit nothing picked up");

            let kept = save_and_load(
                &root,
                "kept",
                &set,
                live_sources(running.playing(0), &placed),
            );
            assert_eq!(
                kept.l1s[0].name, "drift_shell",
                "the save read the path and wrote down an edit this run never \
             compiled, let alone drew"
            );
        }
        /// A `.kir` rewritten between the compile and the first frame cannot reach a
        /// save.
        ///
        /// The window is real and it is not short: between `sort_slot` and the first
        /// frame sit the adapter request, the deck build, `measure_slots`, and the
        /// audio, MIDI, tempo and MCP server starts. The launch seeding used to
        /// `std::fs::read` each path again at the end of that, so anything rewriting a
        /// file in between moved the slot's address onto bytes the deck had never
        /// compiled. If the rewrite did not compile, no watcher ever corrected it — `k`
        /// then wrote a Set naming a procedure that had never been on screen, and the
        /// file did not load back at all.
        ///
        /// Distinct from
        /// [`a_run_with_no_watcher_saves_the_version_it_is_still_drawing`], which
        /// rewrites the file after the run is under way. This one rewrites it inside
        /// the startup sequence, which is the window a second read opens and carrying
        /// the bytes closes.
        ///
        /// Two assertions, the first crisp and the second end to end: the address the
        /// slot reports, and the file that comes back off the disk.
        #[test]
        fn a_rewrite_between_the_compile_and_the_first_frame_cannot_reach_a_save() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let at_start = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &at_start),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            // The compile. Everything the run says about these nodes from here on
            // is a function of the bytes this read.
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[4096], &[7], &[], None);

            // Startup is not finished. Something rewrites the file — a formatter on
            // save, an editor, a model over MCP that connected as the server came
            // up — and this one does not compile, so nothing will ever correct it.
            std::fs::write(
                &paths[0],
                at_start.replace("proc drift_shell", "proc rewritten_between {{{"),
            )
            .expect("the rewrite inside the startup sequence");

            let root = dir.path().join("store");
            let running = launched(&placed);
            let sources = live_sources(running.playing(0), &placed);
            assert_eq!(
                sources.0[0].hash,
                karakuri_store::hash::Hash::of(at_start.as_bytes()),
                "the slot is addressed by bytes this run never compiled, so what it \
             saves is a version that was never on screen"
            );

            let kept = save_and_load(&root, "kept", &set, sources);
            assert_eq!(
                kept.l1s[0].name, "drift_shell",
                "the save wrote down the rewrite rather than the material the deck \
             was built from"
            );
        }
    }
}
