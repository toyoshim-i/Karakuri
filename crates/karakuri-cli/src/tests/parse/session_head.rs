use super::*;

// -- defaults and the positional pair --------------------------------

/// A session recorded with no flags carries its material and replays.
///
/// The head used to be the `--load-set` file or nothing, and "or nothing" meant
/// a timeline of ticks with no material under it: `--replay` refused it with
/// "has no L1 slot" long after the set was over, and nothing said so at the
/// time. This is the round trip that catches it — the head is written the way
/// the recorder writes it and read back the way the replay reads it, so a head
/// that describes nothing fails here rather than on stage.
#[test]
fn a_session_head_carries_the_material_a_replay_needs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let l1 = root.join("examples/drift_shell.kir");
    let l4 = root.join("examples/soft_points.kir");
    let dir = tempfile::tempdir().expect("tempdir");
    let store = karakuri_store::store::Store::open(dir.path()).expect("store");

    let mut args = parse(&[]).expect("parses");
    args.sets = vec![(Named::bare(&l1), vec![Named::bare(&l4)])];
    // **And the edge that makes it buildable.** `morph` takes a geometry it
    // calls `far` and a Set that does not say which one is refused where it
    // is built — so a head recorded without it is a head no replay can
    // open, which is exactly the failure this test is about.
    args.edges = vec![karakuri_engine::set::Edge {
        node: "morph".to_string(),
        slot: "far".into(),
        to: "sphere_shell".to_string(),
    }];
    args.store = dir.path().to_path_buf();
    let (material, placed) = sort_slot(0, &args.sets[0].0, &args.sets[0].1);

    let head = session_head(&args, &[placed], &material.l1s, &store, "a_set");
    assert!(!head.is_empty(), "the head describes nothing");

    // Read back the way `--replay` reads it, which is the whole claim: the
    // artifacts resolve out of the store and both slots are there.
    let loaded = setfile::from_lines(&store, "a_set", &head)
        .expect("the head a recording writes is a head a replay can load");
    assert_eq!(loaded.l1s[0].kind, karakuri_ir::Kind::L1);
    assert_eq!(loaded.l4s[0].kind, karakuri_ir::Kind::L4);
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

/// **A head written for a two-slot deck splits back into the material and the
/// deck**, and the material still loads clean.
///
/// The writer's own output through the reader's own rule. `session_head` writes
/// the head slot's Set file, `session::head` puts the deck's records after it,
/// and `session::split` is what `--replay` sorts the two with — so a head whose
/// deck records leaked into the material, or whose material leaked into the
/// deck, fails here rather than on the way back from a set.
///
/// What this cannot check is the *reading* of a live deck — `held_deck` takes a
/// `Deck` and a deck takes a device. The values below are written by hand for
/// that reason, and the far end of the claim is
/// `karakuri-cli/tests/replay.rs`'s two-slot head driven through the binary.
#[test]
fn a_two_slot_head_splits_into_the_material_and_the_deck() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("tempdir");
    let store = karakuri_store::store::Store::open(dir.path()).expect("store");

    let mut args = parse(&[]).expect("parses");
    args.sets = vec![(
        Named::bare(root.join("examples/coil_vortex.kir")),
        vec![Named::bare(root.join("examples/star_flares.kir"))],
    )];
    args.store = dir.path().to_path_buf();
    let (material, placed) = sort_slot(0, &args.sets[0].0, &args.sets[0].1);
    // The same material in the second slot, which is what this program's own
    // deck holds when it is given one `--set`: four slots of one pair.
    let nodes: Vec<_> = placed
        .iter()
        .map(|node| {
            (
                karakuri_environment::meta::layer_of(node.layer),
                node.index,
                node.hash(),
            )
        })
        .collect();

    let head = session::head(
        session_head(&args, &[placed], &material.l1s, &store, "two"),
        &session::Held {
            canvas: (640, 360),
            look: args.look,
            master_out: 1.0,
            master_chain: Vec::new(),
            slots: (0..2)
                .map(|_| session::SlotHeld {
                    nodes: nodes.clone(),
                    gain: 1.0,
                    opacity: 1.0,
                    blend: karakuri_engine::deck::Blend::Add,
                    residency: karakuri_engine::deck::Residency::Live,
                    policy: karakuri_operation::SlotPolicy::Auto,
                    mask: karakuri_engine::deck::Mask::default(),
                    transport: karakuri_engine::transport::Transport::default(),
                })
                .collect(),
        },
    );

    let stream = session::split(head);
    // The material loads the way `--replay` loads it, with nothing of the deck's
    // among it: a `gain` reaching `from_lines` comes back as a note.
    let loaded = setfile::from_lines(&store, "two", &stream.head)
        .expect("the head a recording writes is a head a replay can load");
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
    assert_eq!(loaded.l1s[0].kind, karakuri_ir::Kind::L1);

    let named: Vec<usize> = stream
        .opening
        .iter()
        .filter_map(|record| match record {
            karakuri_store::record::Record::Procedure { slot, .. } => Some(slot.index()),
            _ => None,
        })
        .collect();
    assert_eq!(
        named,
        vec![1, 1],
        "the second slot's two nodes, and none for the first — whose Set file is the \
         material above"
    );
    assert!(stream.frames.is_empty(), "a head is not a frame");
}

/// A session opens with a Set file, so what a Set file cannot hold is a
/// performance that cannot be recorded.
///
/// A cube morphing into a sphere is two geometries, an L2 that pairs them and a
/// renderer — a chain `--set` has spelled for a while and the head could not
/// carry: `session_head` wrote every path after the first as an `L4` slot, so
/// `--record-session` refused it outright rather than recording a performance
/// nobody could replay. This is that chain through the head and back, layer by
/// layer.
#[test]
fn a_session_head_carries_a_whole_chain_and_not_just_a_pair() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("tempdir");
    let store = karakuri_store::store::Store::open(dir.path()).expect("store");

    let mut args = parse(&[]).expect("parses");
    args.sets = vec![(
        Named::bare(root.join("examples/lattice_shell.kir")),
        vec![
            Named::bare(root.join("examples/sphere_shell.kir")),
            Named::bare(root.join("examples/morph.kir")),
            Named::bare(root.join("examples/soft_points.kir")),
        ],
    )];
    args.store = dir.path().to_path_buf();
    let (material, placed) = sort_slot(0, &args.sets[0].0, &args.sets[0].1);

    let head = session_head(&args, &[placed], &material.l1s, &store, "a_chain");
    let loaded = setfile::from_lines(&store, "a_chain", &head)
        .expect("the head a recording writes is a head a replay can load");
    assert_eq!(
        loaded
            .l1s
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["lattice_shell", "sphere_shell"],
        "the geometries, in the order they were named"
    );
    assert_eq!(
        loaded
            .l2s
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["morph"],
        "the deformer that pairs them"
    );
    assert_eq!(
        loaded
            .l4s
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["soft_points"]
    );
    assert_eq!(loaded.edges, args.edges, "and the wiring between them");
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

/// One slot's examples, compiled, as [`sort_compiled`] takes them — with the
/// text each was compiled from, which is what a node is addressed by.
fn compiled(files: &[&str]) -> Vec<(Named, karakuri_ir::typed::Checked, std::sync::Arc<str>)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    files
        .iter()
        .map(|file| {
            let path = root.join(file);
            let (checked, src) = compile::load(&path).expect("the examples compile");
            (
                Named::bare(path),
                checked,
                std::sync::Arc::from(src.as_str()),
            )
        })
        .collect()
}

/// A slot that cannot be assembled refuses with a sentence, and the two callers
/// decide what to do about it — a startup prints it and stops, a rebuild prints
/// it and leaves the Set that is running alone. That difference is the only
/// thing the two paths do differently, and it is the reason this sort was
/// written twice before it was written once.
///
/// The sentence names the file, because the file is what an operator can fix;
/// the slot is prefixed by whichever caller is reporting it.
#[test]
fn a_slot_refuses_nothing_that_draws_and_keeps_a_second_camera_and_field() {
    // Matched rather than `expect_err`, which would want `Material` to be
    // `Debug` — a derive on a production type to print something no test
    // reaching here ever prints.
    let refused = |files: &[&str]| match sort_compiled(compiled(files)) {
        Err(e) => e,
        Ok(_) => panic!("this slot cannot be assembled"),
    };
    // **A second camera is not a refusal, and this is where that stopped
    // being one.** It said a slot looks from one viewpoint, which was true
    // of the plumbing and not of the material: a renderer declares `uses
    // view : Camera` and an `edge` names which node fills it, so two
    // cameras are two nodes addressed as `L3:0` and `L3:1`.
    let (two_cameras, placed_cameras) = match sort_compiled(compiled(&[
        "drift_shell.kir",
        "beat_jump.kir",
        "beat_jump.kir",
        "soft_points.kir",
    ])) {
        Ok(sorted) => sorted,
        Err(e) => panic!("two cameras are two nodes: {e}"),
    };
    assert_eq!(two_cameras.l3s.len(), 2, "both were kept");
    assert_eq!(
        placed_cameras
            .iter()
            .filter(|p| p.layer == karakuri_ir::Kind::L3)
            .map(|p| (p.layer, p.index))
            .collect::<Vec<_>>(),
        [(karakuri_ir::Kind::L3, 0), (karakuri_ir::Kind::L3, 1)],
        "the cameras are numbered from 0 with no gaps"
    );
    // **A second field is not a refusal, and this is where that stopped
    // being one.** The sort was the last thing in the tree saying a Set
    // holds one, and it said so about the plumbing rather than about the
    // material: two fields are two nodes, addressed as `Field:0` and
    // `Field:1` and bound by name.
    let two_fields = sort_compiled(compiled(&[
        "drift_shell.kir",
        "melt_blob.kir",
        "melt_blob.kir",
        "soft_points.kir",
    ]));
    let (material, placed) = match two_fields {
        Ok(sorted) => sorted,
        Err(e) => panic!("two fields are two nodes: {e}"),
    };
    assert_eq!(material.fields.len(), 2, "both were kept");
    // **At its own index**, which is what `--param Field:1:x` and a Set
    // file's `slot` record both address it by. The second used to be
    // dropped, and before the refusal above it was dropped silently.
    let addresses: Vec<(karakuri_ir::Kind, u32)> = placed
        .iter()
        .filter(|p| p.layer == karakuri_ir::Kind::Field)
        .map(|p| (p.layer, p.index))
        .collect();
    assert_eq!(
        addresses,
        [(karakuri_ir::Kind::Field, 0), (karakuri_ir::Kind::Field, 1)],
        "the fields are numbered from 0 with no gaps"
    );
    let no_renderer = refused(&["drift_shell.kir", "swirl_warp.kir"]);
    assert!(no_renderer.contains("nothing here draws"), "{no_renderer}");
    // The file the slot was spelled with, which is the one an operator looks
    // at first — and by the time this is said the list has been sorted past.
    assert!(no_renderer.contains("drift_shell.kir"), "{no_renderer}");

    // ...and one of each is a slot, so none of the above is a refusal of
    // cameras and fields as such.
    let one_of_each = sort_compiled(compiled(&[
        "drift_shell.kir",
        "beat_jump.kir",
        "melt_blob.kir",
        "soft_points.kir",
    ]));
    assert!(
        one_of_each.is_ok(),
        "one camera and one field is a slot, not a refusal"
    );
}

/// Any part of a `--set` may carry a name. A name addresses the node from every
/// surface that can reach one, and it is written where the file is spelled
/// because it belongs to the *use* — the same lattice twice is one `proc` name
/// and two nodes.
#[test]
fn a_set_may_name_any_of_its_files() {
    let args = parse(&["--set", "near=a.kir,far=b.kir,c.kir"]).expect("parses");
    assert_eq!(
        args.sets,
        vec![(
            Named {
                name: Some("near".to_string()),
                path: PathBuf::from("a.kir")
            },
            vec![
                Named {
                    name: Some("far".to_string()),
                    path: PathBuf::from("b.kir")
                },
                Named::bare("c.kir"),
            ]
        )]
    );
}

/// Two written names that collide are refused rather than resolved, and this is
/// the *early* refusal — the one that names the slot, before a GPU is asked for
/// anything. `Set::build_many` refuses the same thing again where every node is
/// in hand, which is where a derived name could also collide with a written
/// one.
#[test]
fn two_nodes_cannot_share_a_name() {
    let names = Names {
        l1s: vec![Some("near".to_string())],
        l4s: vec![Some("near".to_string())],
        ..Names::default()
    };
    let e = names.check_unique().expect_err("refused");
    assert!(e.contains("both called `near`"), "{e}");

    // And the scope is the slot, because a Set is what holds the nodes.
    let fine = Names {
        l1s: vec![Some("near".to_string())],
        l4s: vec![Some("draw".to_string())],
        ..Names::default()
    };
    assert!(fine.check_unique().is_ok());
}

/// A layer spelling cannot be a node's name, because `--param` tells the two
/// forms apart by what follows the first colon. A node called `L4` would make
/// `--param L4:0:x` ambiguous with a node named `L4` holding a param called
/// `0`.
#[test]
fn a_node_name_is_refused_where_it_would_be_read_as_something_else() {
    for bad in ["L1", "L4", "Field"] {
        let e = parse(&["--set", &format!("{bad}=a.kir,b.kir")]).expect_err("refused");
        assert!(e.contains("is a layer"), "{e}");
    }
    // A name is an address, so what may be in one is narrow and the
    // refusal names the character rather than quietly rewriting it: a
    // silently renamed node is one a `--param` written against it stops
    // finding.
    let e = parse(&["--set", "a b=a.kir,b.kir"]).expect_err("refused");
    assert!(e.contains("not allowed"), "{e}");
    let e = parse(&["--set", "=a.kir,b.kir"]).expect_err("refused");
    assert!(e.contains("empty"), "{e}");
    let e = parse(&["--set", "near=,b.kir"]).expect_err("refused");
    assert!(e.contains("no file after it"), "{e}");
}

#[test]
fn no_arguments_is_the_default_pair() {
    let args = parse(&[]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![(
            Named::bare("examples/coil_vortex.kir"),
            vec![Named::bare("examples/star_flares.kir")],
        )],
        "the pair `examples/star_vortex.kset` names, which is what a run that said \
         nothing opens on since ADR-0271"
    );
}

/// A demonstration brings the scene it is about. `--demo lines` fades each slot
/// out in turn so the other is seen alone, and on a one-slot deck that shows
/// the picture and then an empty frame — the demonstration would run, look like
/// it worked, and demonstrate nothing.
///
/// Two slots, therefore, even though a stack would fit in one. This was briefly
/// rewritten to a single slot holding both renderers, on the grounds that it is
/// the shape the milestone made possible — which broke it, because the script
/// takes one slot at a time out of the mix and a fader is per slot rather than
/// per renderer. Slot 0 carries the stack, which is what the milestone actually
/// buys here: the same two draws, over one simulation instead of two.
#[test]
fn the_lines_demo_supplies_its_own_two_slot_deck() {
    let args = parse(&["--demo", "lines"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![
            (
                Named::bare("examples/drift_shell.kir"),
                vec![
                    Named::bare("examples/soft_points.kir"),
                    Named::bare("examples/drift_streaks.kir"),
                ],
            ),
            (
                Named::bare("examples/drift_shell.kir"),
                vec![Named::bare("examples/drift_streaks.kir")],
            ),
        ],
        "there has to be more than one slot for taking one away to show anything"
    );
    // **Three moments, one per slot plus the return to the mix**, and each
    // moment is the keys it takes: a digit to focus the slot and `F` to
    // fade it out, `G` to bring the previous one back. A deck of a
    // different size leaves the script out of phase, which is what this
    // counts.
    let fades = DEMO_LINES_SCRIPT.iter().filter(|(_, k)| *k == 'F').count();
    let restores = DEMO_LINES_SCRIPT.iter().filter(|(_, k)| *k == 'G').count();
    assert_eq!(
        (fades, restores),
        (args.sets.len(), args.sets.len()),
        "the script fades one slot out per slot in the deck and brings each back; \
         a deck of a different size leaves it out of phase"
    );
    let focused: Vec<char> = DEMO_LINES_SCRIPT
        .iter()
        .filter(|(_, k)| k.is_ascii_digit())
        .map(|(_, k)| *k)
        .collect();
    assert_eq!(
        focused,
        vec!['1', '0'],
        "a fade acts on the focused slot, so every `F` needs the digit that says which"
    );
}

/// And supplies it rather than imposing it: material named on the command line
/// still wins, so `--demo lines` over someone else's pair shows their material
/// and not the examples.
#[test]
fn material_on_the_command_line_beats_a_demos_own_deck() {
    let args = parse(&["--demo", "lines", "a.kir", "b.kir"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![(Named::bare("a.kir"), vec![Named::bare("b.kir")])]
    );
}

/// The transport demonstration works on whatever is loaded, so it brings
/// nothing and the ordinary default still applies. The control for the two
/// above: a `deck()` that answered the same for every demonstration would
/// satisfy them and break this.
#[test]
fn the_transport_demo_leaves_the_default_pair_alone() {
    let args = parse(&["--demo", "transport"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![(
            Named::bare("examples/coil_vortex.kir"),
            vec![Named::bare("examples/star_flares.kir")],
        )]
    );
}

/// Both scripts must end after their last press, or the loop restarts
/// mid-gesture and what a watcher sees depends on when they looked.
#[test]
fn every_demo_script_finishes_before_it_loops() {
    for demo in [Demo::Transport, Demo::Lines] {
        let last = demo.script().last().expect("a script with entries").0;
        assert!(
            demo.loop_seconds() > last,
            "{demo:?} loops at {}s, before its last press at {last}s",
            demo.loop_seconds()
        );
    }
}
