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

/// Two `slot L1` lines are two geometries, and one address is one node.
///
/// The index used to be destructured and thrown away on this arm: every `slot
/// L1` landed in the same entry, so a file describing two sources loaded as
/// one. Index 1 is now the second geometry it always described, and what is
/// left to report is the collision — two lines claiming index 0, which the
/// projection folds to one whatever this loader does.
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

/// A name the file recorded comes back, where it used to be reported and
/// dropped.
///
/// "Nothing this build points at a node by name" was true until an `edge` did:
/// an edge names the node that declares a slot and the node bound to it, so a
/// load that dropped the names is a load whose edges resolve against the wrong
/// spellings — or, for a name nobody wrote, against the procedure's own and by
/// luck.
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

/// The whole chain, through the file and back.
///
/// A Set file recorded an L1 and its renderers: [`save`] refused an L2, an L3
/// or a `kind Field` outright, so a cube morphing into a sphere was a Set that
/// could be played and not kept — and `--record-session` refused it with the
/// same message, because a session opens with a Set file. Every layer is
/// asserted separately, because writing them all as `L4` slots is exactly what
/// used to happen and a count would not have noticed.
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

/// Two fields, each at its own index, through the file and back.
///
/// The format could always say it — a `slot` record carries a layer and an
/// index, and Field is a layer like any other — and the loader would not: it
/// took `field_srcs.first()` and filed the rest under a note. So a Set whose
/// marcher took a shape and a cutter saved as a Set that came back with one of
/// them, and the edge naming the missing one no longer resolved.
///
/// Index as well as count, because a pair that came back in the other order is
/// a Set whose `--param Field:1:…` moves the wrong shape, and a count would not
/// have noticed.
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

/// Each geometry runs at the number written against it.
///
/// The capacity was keyed by node in the format and by Set in this loader: a
/// `capacity` on L1 index 1 was reported and dropped, so a Set whose two
/// sources were sized differently came back with the second at whatever its
/// `.kir` declared. The engine takes one per source and now so does this.
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

/// A salt is recorded per geometry and comes back per geometry, which is what
/// makes a saved Set reproduce its colours whatever order its records are in —
/// `docs/ir-spec.md`, "A `source` value is assigned and recorded, never
/// derived". This wrote one `seed` for the whole Set and read node 0's, so the
/// second geometry's randomness was a function of where its path sat on the
/// command line and of nothing in the file.
///
/// The bytes are asserted, not just the round trip. Index 0 is absent and index
/// 1 is written, which is the whole of what keeps the file a Set of one
/// geometry has always written unchanged.
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

/// A Set file written when a seed salted the whole Set still loads, and says so
/// by carrying one salt for the geometry it was written against.
///
/// That is the older file's shape: one `seed` record, no index on it, and
/// however many geometries. The geometry it names keeps the colours it was
/// saved with; the ones it does not are salted the way an unsaved run is, which
/// is what `None` in [`Loaded::salts`] asks the engine for. Refusing or
/// defaulting either half would be a file that loads and draws something nobody
/// saved.
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

/// A one-geometry Set is byte for byte the file it has always been, except for
/// the one field the format has grown since.
///
/// Every Set file ever written is one L1 and its renderers, and the fields that
/// carry a chain — `index` on every layer, `name` on a slot — are absent rather
/// than defaulted for exactly this reason. A literal, not a re-save compared
/// against itself: a round trip through one writer agrees with itself however
/// far both halves have drifted.
///
/// `height` on the `camera` line is the exception, and it is a decision rather
/// than drift (ADR-0318, 2026-09-09): the record carried two of the orbit's
/// three placement numbers, so a camera saved looking down came back looking
/// along the equator. This literal grew the field; a file written without it
/// still reads as the 2.0 it meant. What this test is for is that nothing grows
/// one *silently*, and it did its job.
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

/// A composited Set is saved as one and comes back as one, folded to the
/// renderer it was folded to.
///
/// This is the round trip the `merge` record exists for. A Set file could not
/// say that a slot composites, so a variant pool written out came back
/// overdrawing: no L5, no edges into one, and `Set::select_renderer` with
/// nothing to select between. Both halves are asserted here because either one
/// alone is useless — a layering that survived without its selection comes up
/// folding every alternative at once, which is a different picture from the one
/// that was saved.
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

/// A Set that overdraws writes no `merge` line at all, and loads back
/// overdrawing.
///
/// The record's absence is how overdraw has always been spelled — there is no
/// boolean field, because a record that could say `false` would be a second
/// spelling of not writing one. So this asserts the *bytes*: a Set that
/// overdraws is byte for byte the file it was before the record existed, and
/// every file written by an older build reads as what it was.
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

/// A save a model asked for lands in the sandbox and never in the operator's
/// library.
///
/// This is
/// `docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md`
/// at the one line that decides it — [`save`]'s match on [`Asked`] — and the
/// property it holds is a *negative* one: the file that must not be there.
/// Delete the `Asked::Model` arm and the sandbox assertion still passes on
/// nothing, so the assertion that matters is the second: `sets/` is where the
/// operator's presets are, and a model writing an id one of them already has is
/// the loss P-0096 exists against.
///
/// The control is the same call with the other actor, which is what stops this
/// passing against a `save` that had stopped writing anywhere the library can
/// see.
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

/// Two saves a model asked for under one name are two files.
///
/// `filed_as` is where this is decided and the reason is written there: the
/// sandbox holds snapshots, and a snapshot a later snapshot can replace is not
/// one. Asserted here rather than beside that function because the property is
/// about what is on the disk afterwards — a rule about an id that never reached
/// a store would be a rule about a string.
///
/// The control is the operator's own name, which overwrites, and that is
/// ADR-0128 unchanged: an id an operator types is an instruction.
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

/// A composited Set nobody selected in writes no `live`, and comes back with
/// every renderer live.
///
/// Absent is *not* renderer 0. Read that way it would silence every renderer
/// but the first in every composited Set ever saved without a selection — a
/// picture nobody asked for, from a file that said nothing had changed.
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

/// A fold naming a renderer the file does not have is said and dropped.
///
/// A hand-written or hand-edited file can name one; the Set is whole either
/// way, so the honest answer is to load it with every renderer live — the state
/// it would have come up in — and say which line was not honoured. Checked
/// where the `camera` index is checked and for its reason: how many renderers a
/// file names is only known once every `slot` record has been met.
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
