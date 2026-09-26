use super::*;
use karakuri_console::view::{Scope, View};
use karakuri_environment::Asked;
use karakuri_operation::{Operation, SetTransfer};
use karakuri_operation_record::Written;

/// Verifies that an uninitialized store directory lists nothing and is not created (P-0091).
#[test]
fn a_library_is_the_store_and_a_missing_store_is_not_made() {
    let root = std::env::temp_dir().join(format!(
        "karakuri-console-library-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    // Whatever a previous run left, so the first claim is about a root
    // that is genuinely not there.
    let _ = std::fs::remove_dir_all(&root);

    assert!(
        library(&root).is_empty(),
        "a store that is not there listed something"
    );
    assert!(
        !root.exists(),
        "listing a library that is not there created one at {}",
        root.display()
    );

    // Verifies listing order: most recently written first, tie-broken by id ascending.
    let store = Store::open(&root).expect("a store to list");
    for id in ["night01", "morph01"] {
        store
            .write_set(id, &[])
            .unwrap_or_else(|e| panic!("writing {id}: {e}"));
    }
    let listed: Vec<String> = library(&root).into_iter().map(|set| set.id).collect();
    assert_eq!(
        listed,
        vec!["morph01".to_owned(), "night01".to_owned()],
        "the bay lists {listed:?}"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Writes a snapshot file formatted to match `history::list` parsing expectations.
fn version_file(root: &std::path::Path, day: &str, name: &str, body: &str) {
    let dir = root.join("history").join(day);
    std::fs::create_dir_all(&dir).expect("a day directory");
    std::fs::write(dir.join(name), body).expect("a version");
}

/// Verifies history listings filter by the active Set and exclude versions filed under no Set (ADR-0276, ADR-0308).
#[test]
fn a_history_listing_is_one_sets_versions_and_a_none_row_is_nobodys() {
    let root = scratch_dir("history-listing");
    std::fs::create_dir_all(&root).expect("a store root");
    version_file(
        &root,
        "2026/09/08",
        "143052-271_slot0_L4_beat_strokes@x.kir",
        "kind L4\n",
    );
    version_file(
        &root,
        "2026/09/08",
        "142930-004_slot0_L1_drift_shell@x.kir",
        "kind L1\n",
    );
    // Version under no Set must not match a Set-scoped query.
    version_file(
        &root,
        "2026/09/08",
        "142800-000_slot1_L4_soft_points.kir",
        "kind L4\n",
    );
    // Version under another Set ensures queries properly filter by Set ID.
    version_file(
        &root,
        "2026/09/07",
        "235959-999_slot2_L4_other_thing@y.kir",
        "kind L4\n",
    );

    let mut view = View::new(karakuri_console::room::Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert!(view.select_scope(Scope::History), "the mark did not move");

    let said = listing(&mut view, &root, None, None, Some("x"));
    assert_eq!(
        view.library,
        vec![
            "20260908-143052-271_slot0_L4_beat_strokes".to_owned(),
            "20260908-142930-004_slot0_L1_drift_shell".to_owned(),
        ],
        "the walk listed {:?} — a row of another Set, or a row filed under none, was \
         treated as one of `x`'s",
        view.library
    );
    assert!(
        said.contains("history") && said.contains('2'),
        "the line does not say what the scope listed: {said}"
    );

    // Queries with None Set ID list nothing for fresh launch pairs.
    let said = listing(&mut view, &root, None, None, None);
    assert!(
        view.library.is_empty(),
        "a deck running no Set listed {:?}",
        view.library
    );
    assert!(
        said.contains("typed pair"),
        "the line does not say why the scope is empty: {said}"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies version restoration writes snapshot bytes over the node working copy and reports missing files cleanly (ADR-0228, P-0083).
#[test]
fn a_landing_writes_the_versions_bytes_over_the_nodes_working_copy() {
    let root = scratch_dir("history-landing");
    let scratch = root.join(karakuri_environment::scratch::DIR);
    std::fs::create_dir_all(&scratch).expect("a scratch");
    let head = scratch.join("A0-drift_shell.kir");
    let rest = scratch.join("A1-soft_points.kir");
    std::fs::write(&head, "kind L1\n// what is playing\n").expect("the head");
    std::fs::write(&rest, "kind L4\n// what is playing\n").expect("the renderer");
    let slots = karakuri_mcp::Slots::of(vec![(head.clone(), vec![rest.clone()])]);

    version_file(
        &root,
        "2026/09/08",
        "143052-271_slot0_L4_soft_points@x.kir",
        "kind L4\n// the version an operator picked\n",
    );
    let picked = "20260908-143052-271_slot0_L4_soft_points";

    let said = put_back(
        &root,
        0,
        "A",
        "x",
        &karakuri_operation::Revision::Picked(picked.to_owned()),
        &slots,
    );
    assert!(
        said.contains(&rest.display().to_string()),
        "the line does not name the file it wrote: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(&rest).expect("the renderer's working copy"),
        "kind L4\n// the version an operator picked\n",
        "the version's bytes are not what the node is playing from"
    );
    assert_eq!(
        std::fs::read_to_string(&head).expect("the head's working copy"),
        "kind L1\n// what is playing\n",
        "landing on the L4 wrote over the L1 as well"
    );

    // Reverting to a deleted revision reports that the target file is gone.
    std::fs::remove_dir_all(root.join("history")).expect("the operator's own `rm -rf`");
    let said = put_back(
        &root,
        0,
        "A",
        "x",
        &karakuri_operation::Revision::Picked(picked.to_owned()),
        &slots,
    );
    assert!(
        said.contains(picked) && said.contains("nothing moved"),
        "a landing on a version that is gone did not say so: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(&rest).expect("the renderer's working copy"),
        "kind L4\n// the version an operator picked\n",
        "a refused landing wrote something"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies stepping back a node's revision restores the immediately preceding version (ADR-0326).
#[test]
fn a_step_back_lands_the_version_before_the_one_running() {
    let root = scratch_dir("history-step-back");
    let scratch = root.join(karakuri_environment::scratch::DIR);
    std::fs::create_dir_all(&scratch).expect("a scratch");
    let head = scratch.join("A0-drift_shell.kir");
    let rest = scratch.join("A1-soft_points.kir");
    std::fs::write(&head, "kind L1\n// what is playing\n").expect("the head");
    std::fs::write(&rest, "kind L4\n// the third\n").expect("the renderer");
    let slots = karakuri_mcp::Slots::of(vec![(head.clone(), vec![rest.clone()])]);

    // Oldest first here so the file reads in the order the operator wrote
    // them; `history::list` answers the other way round, which is the
    // ordering this test is about.
    for (at, body) in [
        ("143050-100", "kind L4\n// the first\n"),
        ("143051-100", "kind L4\n// the second\n"),
        ("143052-100", "kind L4\n// the third\n"),
    ] {
        version_file(
            &root,
            "2026/09/09",
            &format!("{at}_slot0_L4_soft_points@x.kir"),
            body,
        );
    }
    // Newer version of a different node ensures revision walk filters by node address.
    version_file(
        &root,
        "2026/09/09",
        "143053-100_slot0_L1_drift_shell@x.kir",
        "kind L1\n// a newer edit of the geometry\n",
    );

    let renderer = karakuri_operation::NodeAddress {
        layer: karakuri_operation::Layer::L4,
        index: 0,
    };
    let said = put_back(
        &root,
        0,
        "A",
        "x",
        &karakuri_operation::Revision::Previous(renderer),
        &slots,
    );
    assert!(
        said.contains(&rest.display().to_string()),
        "the line does not name the file it wrote: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(&rest).expect("the renderer's working copy"),
        "kind L4\n// the second\n",
        "a step back landed something other than the version before the one running"
    );
    assert_eq!(
        std::fs::read_to_string(&head).expect("the head's working copy"),
        "kind L1\n// what is playing\n",
        "stepping the L4 back wrote over the L1 as well"
    );

    // Stepping back a node with only one revision reports no previous version (P-0083).
    let geometry = karakuri_operation::NodeAddress {
        layer: karakuri_operation::Layer::L1,
        index: 0,
    };
    let said = put_back(
        &root,
        0,
        "A",
        "x",
        &karakuri_operation::Revision::Previous(geometry),
        &slots,
    );
    assert!(
        said.contains("L1:0") && said.contains("one version"),
        "a step back with nothing behind it did not say what is in the way: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(&head).expect("the head's working copy"),
        "kind L1\n// what is playing\n",
        "a refused step back wrote something"
    );

    // Stepping back an unrecorded node address is rejected.
    let field = karakuri_operation::NodeAddress {
        layer: karakuri_operation::Layer::Field,
        index: 0,
    };
    let said = put_back(
        &root,
        0,
        "A",
        "x",
        &karakuri_operation::Revision::Previous(field),
        &slots,
    );
    assert!(
        said.contains("no version"),
        "a step back on a node with nothing filed did not say so: {said}"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies keeping a candidate removes its staging lane entry while leaving other node rows intact (ADR-0326).
#[test]
fn a_keep_takes_its_own_row_off_the_lane() {
    let node = |layer: karakuri_operation::Layer, index: u32, name: &str| Changed {
        at: karakuri_operation::NodeAddress { layer, index },
        addr: node_addr(ir_layer(layer), index),
        name: name.to_owned(),
    };
    let mut view = View::new(karakuri_console::room::Room::Day);
    let both = [
        node(karakuri_operation::Layer::L1, 0, "drift_shell"),
        node(karakuri_operation::Layer::L4, 0, "soft_points"),
    ];
    settle(
        &mut view.staging,
        0,
        "drift_shell + soft_points",
        view::Stage::Landed,
        &[],
        &both,
    );
    settle(
        &mut view.staging,
        1,
        "drift_shell + soft_points",
        view::Stage::Landed,
        &[],
        &both[..1],
    );
    assert_eq!(view.staging.len(), 3, "three rows over two slots");

    let said = kept(
        &mut view,
        &Operation::KeepCandidate {
            deck: 0,
            node: both[0].at,
        },
    )
    .expect("a keep is performed here");
    assert!(
        said.contains("L1:0") && said.contains("deck A"),
        "the line does not say which row left: {said}"
    );
    assert_eq!(
        view.staging
            .iter()
            .map(|row| (row.deck, row.addr.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "L4:0"), (1, "L1:0")],
        "a keep took a row it was not addressed to"
    );

    // Duplicate keep on the same node reports no candidate row remaining.
    let said = kept(
        &mut view,
        &Operation::KeepCandidate {
            deck: 0,
            node: both[0].at,
        },
    )
    .expect("a keep is performed here");
    assert!(
        said.contains("no candidate row"),
        "a keep on a node with no row claimed to have kept one: {said}"
    );
    assert_eq!(view.staging.len(), 2, "the second keep took a row anyway");

    // And an operation that is not a keep is not this function's.
    assert_eq!(
        kept(&mut view, &Operation::SelectDeck { deck: 1 }),
        None,
        "`kept` answered for an operation that is not a keep"
    );
}
