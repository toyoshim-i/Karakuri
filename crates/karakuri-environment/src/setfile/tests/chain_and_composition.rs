#![allow(unused_imports)]

use super::common::*;

/// The two diagnostics the decoder owes, asserted against the decoder rather
/// than against the flag that used to hold them.
#[test]
fn the_decoder_carries_the_diagnostics_the_flag_used_to_hold_alone() {
    let bpm = Record::Bind {
        layer: Layer::L1,
        index: None,
        key: "radius".to_string(),
        signal: "bpm".to_string(),
        curve: "lin".to_string(),
        range: [0.0, 1.0],
        noise: None,
    };
    let err = binding_from_record(&bpm).expect_err("a tempo is not a [0,1] signal");
    assert!(err.contains("bar"), "{err}");

    let octaves = Record::Bind {
        layer: Layer::L1,
        index: None,
        key: "radius".to_string(),
        signal: NOISE_SIGNAL.to_string(),
        curve: "lin".to_string(),
        range: [0.0, 1.0],
        noise: Some(BindNoise {
            kind: "white".to_string(),
            octaves: 6,
            ..BindNoise::default()
        }),
    };
    let err = binding_from_record(&octaves).expect_err("white has no octaves");
    assert!(err.contains("fbm"), "{err}");
}

/// Verifies multiple geometry slots are preserved by index and colliding slot addresses are reported.
#[test]
fn a_second_geometry_is_carried_and_two_slots_at_one_address_are_reported() {
    let (_dir, store, l1, l4) = fixture();
    let put = |bytes: &[u8]| store.put_artifact(bytes).expect("put");
    let first = put(&std::fs::read(&l1).expect("read"));
    // A second geometry, distinguishable from the first by name so that
    // "the later one is used" is checked rather than asserted.
    let second = put(L1.replace("proc ring", "proc ring_two").as_bytes());
    let renderer = put(&std::fs::read(&l4).expect("read"));
    let lines = parsed(&format!(
        r#"{{"t":"set","id":"two","v":1}}
{{"t":"slot","layer":"L1","proc":"{first}"}}
{{"t":"slot","layer":"L1","proc":"{second}"}}
{{"t":"slot","layer":"L1","index":1,"proc":"{second}"}}
{{"t":"slot","layer":"L4","proc":"{renderer}"}}
"#
    ));

    let loaded = from_lines(&store, "two", &lines).expect("load");
    let notes = loaded.notes.join("\n");
    assert!(
        notes.contains("two L1 slots both claim index 0"),
        "a second geometry replaced the first in silence; notes were {notes:?}"
    );
    // Both geometries, in index order — the file named two sources and two
    // is what a Set holds.
    assert_eq!(
        loaded
            .l1s
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["ring_two", "ring_two"],
        "a geometry at index 1 was dropped; notes were {notes:?}"
    );
    // The later of the colliding pair is what was built, which is what the
    // note promises and the only reading under which the note is true.
    assert_eq!(loaded.l1s[0].name, "ring_two");
}

/// Verifies recorded node names are preserved for edge resolution.
#[test]
fn a_name_the_file_recorded_comes_back_with_the_node() {
    let (_dir, store, l1, l4) = fixture();
    let put = |path: &std::path::Path| {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    };
    let (geometry, renderer) = (put(&l1), put(&l4));
    let lines = parsed(&format!(
        r#"{{"t":"set","id":"named","v":1}}
{{"t":"slot","layer":"L1","name":"veil","proc":"{geometry}"}}
{{"t":"slot","layer":"L4","proc":"{renderer}"}}
"#
    ));

    let loaded = from_lines(&store, "named", &lines).expect("a named slot still loads");
    assert_eq!(
        loaded.l1s[0].name, "ring",
        "the material is unaffected by the name"
    );
    assert_eq!(
        loaded.names.l1s,
        [Some("veil".to_string())],
        "the name the file recorded is what the node is called"
    );
    // **And an unnamed node stays unnamed** rather than being filled in
    // here: a name nobody wrote is derived where the Set is built, and
    // deriving it here as well would be the second place one fact lives.
    assert_eq!(loaded.names.l4s, [None]);
    let notes = loaded.notes.join("\n");
    assert!(
        !notes.contains("veil"),
        "a name that was honoured is not a note; notes were {notes:?}"
    );
}

/// Verifies round-trip persistence across all pipeline layers (L1 through L4 and Field).
#[test]
fn the_whole_chain_survives_the_file_it_is_written_to() {
    let (dir, store, l1, l4) = fixture();
    let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
    let l2 = beside(&dir, "l2.kir", L2);
    let l3 = beside(&dir, "l3.kir", L3);
    let field = beside(&dir, "field.kir", FIELD);
    let l4b = beside(&dir, "l4b.kir", &L4.replace("proc points", "proc streaks"));

    let nodes = vec![
        Node {
            hash: stored(&store, &l1),
            layer: Kind::L1,
            index: 0,
            // A name the operator wrote, which is what `--set veil=l1.kir`
            // spells and what nothing here could record before.
            name: Some("veil".to_string()),
        },
        Node {
            hash: stored(&store, &l1b),
            layer: Kind::L1,
            index: 1,
            name: None,
        },
        Node {
            hash: stored(&store, &l2),
            layer: Kind::L2,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l3),
            layer: Kind::L3,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &field),
            layer: Kind::Field,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l4b),
            layer: Kind::L4,
            index: 1,
            name: None,
        },
    ];
    save(
        &store,
        Asked::Operator,
        "chain",
        Saving {
            nodes: &nodes,
            capacities: &[4096, 8192],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1, 2],
        },
    )
    .expect("save");
    assert!(
        written(&store, "chain").contains(r#""name":"veil""#),
        "a name the operator wrote went nowhere: {}",
        written(&store, "chain")
    );

    let loaded = load(&store, "chain").expect("load");
    let named = |procs: &[Checked]| procs.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    assert_eq!(named(&loaded.l1s), ["ring", "ring_two"]);
    assert_eq!(named(&loaded.l2s), ["warp"]);
    assert_eq!(named(&loaded.l3s), ["look"]);
    assert_eq!(named(&loaded.fields), ["blob"]);
    assert_eq!(named(&loaded.l4s), ["points", "streaks"]);
    // Node order, which is what the scratch places them in — see
    // [`Loaded::nodes`].
    assert_eq!(
        loaded
            .nodes()
            .map(|(checked, _)| checked.name.clone())
            .collect::<Vec<_>>(),
        ["ring", "ring_two", "warp", "look", "points", "streaks", "blob"]
    );
    assert!(
        loaded.srcs[2].contains("proc warp"),
        "a node was paired with another node's source: {}",
        loaded.srcs[2]
    );
    // And the name it was written with, placed on the node it belongs to
    // rather than on whichever node the walk happened to reach.
    assert_eq!(loaded.names.l1s, [Some("veil".to_string()), None]);
    assert_eq!(loaded.names.l2s, [None]);
}

/// Verifies multiple field procedures persist and restore at their respective indices.
#[test]
fn two_fields_come_back_at_their_own_indices() {
    let (dir, store, l1, l4) = fixture();
    let shape = beside(&dir, "shape.kir", FIELD);
    let cutter = beside(&dir, "cutter.kir", &FIELD.replace("proc blob", "proc bite"));
    let nodes = vec![
        Node {
            hash: stored(&store, &l1),
            layer: Kind::L1,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &shape),
            layer: Kind::Field,
            index: 0,
            name: None,
        },
        // Named, because an edge points with names and a Set holding two
        // fields is the first one that has to tell them apart.
        Node {
            hash: stored(&store, &cutter),
            layer: Kind::Field,
            index: 1,
            name: Some("knife".to_string()),
        },
    ];
    save(
        &store,
        Asked::Operator,
        "two_fields",
        Saving {
            nodes: &nodes,
            capacities: &[4096],
            params: &[],
            bindings: &[],
            edges: &[karakuri_engine::set::Edge {
                node: "points".to_string(),
                slot: "cutter".into(),
                to: "knife".to_string(),
            }],
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1],
        },
    )
    .expect("save");

    let loaded = load(&store, "two_fields").expect("load");
    let named = |procs: &[Checked]| procs.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    assert_eq!(named(&loaded.fields), ["blob", "bite"]);
    assert!(
        loaded.notes.is_empty(),
        "nothing was skipped: {:?}",
        loaded.notes
    );
    // The names, on the nodes they belong to — `None` for the one written
    // bare, which is what an edge pointing at `knife` needs to resolve.
    assert_eq!(loaded.names.fields, [None, Some("knife".to_string())]);
    // Node order, which is what the scratch places sources in: the fields
    // are last and they are in index order.
    assert_eq!(
        loaded
            .nodes()
            .map(|(checked, _)| checked.name.clone())
            .collect::<Vec<_>>(),
        ["ring", "points", "blob", "bite"]
    );
    assert_eq!(
        loaded.node_names().collect::<Vec<_>>(),
        [None, None, None, Some("knife".to_string())]
    );
    assert_eq!(loaded.edges.len(), 1, "the edge that binds the second");
}

/// Verifies distinct geometry capacities are maintained per node index.
#[test]
fn each_geometry_keeps_the_capacity_it_was_saved_with() {
    let (dir, store, l1, l4) = fixture();
    let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
    let nodes = vec![
        Node {
            hash: stored(&store, &l1),
            layer: Kind::L1,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l1b),
            layer: Kind::L1,
            index: 1,
            name: None,
        },
        Node {
            hash: stored(&store, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
    ];
    save(
        &store,
        Asked::Operator,
        "two",
        Saving {
            nodes: &nodes,
            capacities: &[4096, 65_536],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1, 2],
        },
    )
    .expect("save");

    let loaded = load(&store, "two").expect("load");
    assert_eq!(loaded.capacities, vec![Some(4096), Some(65_536)]);
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

/// Verifies random salts are recorded and loaded per geometry node index.
#[test]
fn each_geometry_keeps_the_salt_it_was_saved_with() {
    let (dir, store, l1, l4) = fixture();
    let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
    let nodes = vec![
        Node {
            hash: stored(&store, &l1),
            layer: Kind::L1,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l1b),
            layer: Kind::L1,
            index: 1,
            name: None,
        },
        Node {
            hash: stored(&store, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
    ];
    save(
        &store,
        Asked::Operator,
        "two",
        Saving {
            nodes: &nodes,
            capacities: &[4096, 4096],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[7, 9],
        },
    )
    .expect("save");

    let text = written(&store, "two");
    assert!(
        text.contains("{\"t\":\"seed\",\"stream\":\"L1\",\"value\":7}\n")
            && text.contains("{\"t\":\"seed\",\"stream\":\"L1\",\"index\":1,\"value\":9}\n"),
        "{text}"
    );

    let loaded = load(&store, "two").expect("load");
    assert_eq!(loaded.salts, vec![Some(7), Some(9)]);
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

/// Verifies backwards compatibility for legacy unindexed seed records applied to the first geometry.
#[test]
fn a_file_that_salted_the_whole_set_still_loads() {
    let (dir, store, l1, l4) = fixture();
    let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
    let put = |path: &std::path::Path| {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    };
    let lines = parsed(&format!(
        r#"{{"t":"set","id":"old","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L1","index":1,"proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{{"t":"seed","stream":"L1","value":4242}}
"#,
        put(&l1),
        put(&l1b),
        put(&l4)
    ));

    let loaded = from_lines(&store, "old", &lines).expect("load");
    assert_eq!(loaded.l1s.len(), 2);
    assert_eq!(
        loaded.salts,
        vec![Some(4242)],
        "the one seed the file carries salts the geometry it names, and the other \
             geometry is left to be derived"
    );
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

/// Verifies byte-for-byte serialization format stability for standard single-geometry Sets.
#[test]
fn a_one_geometry_set_is_byte_for_byte_the_file_it_always_was() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let hash = |path: &std::path::Path| {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    };
    assert_eq!(
        written(&store, "s1"),
        format!(
            r#"{{"t":"set","id":"s1","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{{"t":"capacity","layer":"L1","value":4096}}
{{"t":"camera","kind":"orbit","radius":8.0,"speed":0.15,"height":2.0}}
{{"t":"seed","stream":"L1","value":1}}
"#,
            hash(&l1),
            hash(&l4)
        )
    );
}

/// Verifies composited Sets round-trip their layering mode and active live renderer selection.
#[test]
fn a_composited_set_comes_back_composited_and_still_folded_where_it_was() {
    let (dir, store, l1, l4) = fixture();
    let l4b = beside(
        &dir,
        "l4b.kir",
        &L4.replace("proc points", "proc points_two"),
    );
    let nodes = ordinary(&store, &l1, &[l4, l4b]);
    save(
        &store,
        Asked::Operator,
        "pool",
        Saving {
            layering: Layering::Composite,
            live: Some(1),
            ..plain(&nodes, &[])
        },
    )
    .expect("save");

    // The line itself, because the record's *presence* is the statement
    // and a test that only read the decoder back could pass on a writer
    // that wrote nothing and a reader that assumed everything.
    assert!(
        written(&store, "pool").contains(r#"{"t":"merge","live":1}"#),
        "the file does not carry the merge the Set was saved with:\n{}",
        written(&store, "pool")
    );

    let loaded = load(&store, "pool").expect("load");
    assert_eq!(
        loaded.layering,
        Layering::Composite,
        "a composited Set loaded back overdrawing"
    );
    assert_eq!(
        loaded.live,
        Some(1),
        "the fold came back with a different renderer live"
    );
    assert!(
        loaded.notes.is_empty(),
        "unexpected notes: {:?}",
        loaded.notes
    );
}

/// Ensures default overdraw layering omits redundant merge records and restores as overdraw.
#[test]
fn an_overdrawing_set_writes_no_merge_line_and_loads_back_overdrawing() {
    let (dir, store, l1, l4) = fixture();
    let l4b = beside(
        &dir,
        "l4b.kir",
        &L4.replace("proc points", "proc points_two"),
    );
    let nodes = ordinary(&store, &l1, &[l4, l4b]);
    save(&store, Asked::Operator, "stack", plain(&nodes, &[])).expect("save");

    let text = written(&store, "stack");
    assert!(
        !text.contains("merge"),
        "an overdrawing Set wrote a merge record:\n{text}"
    );

    let loaded = load(&store, "stack").expect("load");
    assert_eq!(
        loaded.layering,
        Layering::Overdraw,
        "a Set that recorded no merge loaded back compositing"
    );
    assert_eq!(loaded.live, None, "a Set with no merge recorded a fold");
}

/// Ensures model-initiated saves are routed exclusively to the sandbox directory per P-0096.
#[test]
fn a_save_a_model_asked_for_lands_in_the_sandbox_and_never_in_the_library() {
    let (dir, store, l1, l4) = fixture();
    let nodes = ordinary(&store, &l1, &[l4]);
    let root = dir.path().join("store");

    save(&store, Asked::Model, "night01", plain(&nodes, &[])).expect("save");

    assert!(
        root.join("sandbox").join("night01.kbset").exists(),
        "a save asked for over MCP did not reach the sandbox"
    );
    assert!(
        !root.join("sets").join("night01.kbset").exists(),
        "a save asked for over MCP wrote the operator's library"
    );
    assert!(
        load(&store, "night01").is_err(),
        "the library loaded a Set only the sandbox holds"
    );

    // The control. Without it this would pass against a `save` that wrote
    // no library file for anybody.
    save(&store, Asked::Operator, "night01", plain(&nodes, &[])).expect("save");
    assert!(
        root.join("sets").join("night01.kbset").exists(),
        "the operator's own save did not reach the library"
    );
    load(&store, "night01").expect("and the library loads it back");
}

/// Verifies successive model saves generate distinct snapshot files rather than overwriting.
#[test]
fn two_saves_a_model_asked_for_under_one_name_are_two_files() {
    let (dir, store, l1, l4) = fixture();
    let nodes = ordinary(&store, &l1, &[l4]);
    let root = dir.path().join("store");

    let first = crate::accepted_save(
        0,
        Asked::Model,
        Some("take".into()),
        &Sources(vec![]),
        &root,
        None,
    );
    let second = crate::accepted_save(
        0,
        Asked::Model,
        Some("take".into()),
        &Sources(vec![]),
        &root,
        None,
    );
    assert_ne!(
        first, second,
        "a model's second save was handed the first one's id, so it would \
             have written over a snapshot it had been told was kept"
    );
    save(&store, Asked::Model, &first, plain(&nodes, &[])).expect("save");
    save(&store, Asked::Model, &second, plain(&nodes, &[])).expect("save");
    let kept = std::fs::read_dir(root.join("sandbox"))
        .expect("the sandbox")
        .count();
    assert_eq!(kept, 2, "two snapshots did not leave two files");

    // The control: an operator's own name is an instruction and is reused.
    assert_eq!(
        crate::accepted_save(
            0,
            Asked::Operator,
            Some("take".into()),
            &Sources(vec![]),
            &root,
            None
        ),
        "take",
        "an operator's own id was not the id it asked for"
    );
}

/// Verifies unselected composited Sets omit `live` field and restore with all renderers active.
#[test]
fn a_composited_set_nobody_selected_in_writes_no_live() {
    let (dir, store, l1, l4) = fixture();
    let l4b = beside(
        &dir,
        "l4b.kir",
        &L4.replace("proc points", "proc points_two"),
    );
    let nodes = ordinary(&store, &l1, &[l4, l4b]);
    save(
        &store,
        Asked::Operator,
        "unselected",
        Saving {
            layering: Layering::Composite,
            live: None,
            ..plain(&nodes, &[])
        },
    )
    .expect("save");

    let text = written(&store, "unselected");
    assert!(
        text.contains(r#"{"t":"merge"}"#),
        "the bare merge record is not in the file:\n{text}"
    );

    let loaded = load(&store, "unselected").expect("load");
    assert_eq!(loaded.layering, Layering::Composite);
    assert_eq!(
        loaded.live, None,
        "a merge with no `live` came back naming a renderer"
    );
}

/// Ensures references to non-existent renderer indices in merge records are dropped and reported.
#[test]
fn a_fold_naming_a_renderer_that_is_not_there_is_reported_and_dropped() {
    let (_dir, store, l1, l4) = fixture();
    let nodes = ordinary(&store, &l1, std::slice::from_ref(&l4));
    save(
        &store,
        Asked::Operator,
        "wrong",
        Saving {
            layering: Layering::Composite,
            live: Some(3),
            ..plain(&nodes, &[])
        },
    )
    .expect("save");

    let loaded = load(&store, "wrong").expect("load");
    assert_eq!(loaded.layering, Layering::Composite, "the Set still folds");
    assert_eq!(loaded.live, None, "a fold nobody can honour was applied");
    assert!(
        loaded.notes.iter().any(|n| n.contains("renderer 3")),
        "nothing said which renderer was not there: {:?}",
        loaded.notes
    );
}
