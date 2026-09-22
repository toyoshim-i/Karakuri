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

/// A version, written where `history::list` reads them.
///
/// The name is `Snapshots::record`'s own — a `HHMMSS-mmm` stamp, the slot, the
/// layer with its index where it is not the first, the procedure name, and
/// `@<set>` where there was a Set — and it is spelled here rather than recorded
/// through that type because what these tests are about is the *reading*: a
/// version filed under a Set, one filed under none, and the difference between
/// them.
fn version_file(root: &std::path::Path, day: &str, name: &str, body: &str) {
    let dir = root.join("history").join(day);
    std::fs::create_dir_all(&dir).expect("a day directory");
    std::fs::write(dir.join(name), body).expect("a version");
}

/// The `history` scope lists the versions of the Set the load pulldown's deck
/// is running, and a version filed under no Set is not one of them.
///
/// That last clause is the one ADR-0276 wrote down and ADR-0308 had to obey:
/// *"a narrowing must treat a `None` row as matching no Set rather than as a
/// wildcard."* A run launched on a pair somebody typed files every version it
/// writes under none, so a wildcard would put the whole of that run's editing
/// under whatever Set the operator loaded afterwards — silently, in a bay whose
/// rows are names.
///
/// And a deck running nothing lists nothing, with the sentence saying which
/// nothing it is. A CPU test: `listing` reaches a disk and no device.
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
    // **Under no Set at all**, which is the row that must not match.
    version_file(
        &root,
        "2026/09/08",
        "142800-000_slot1_L4_soft_points.kir",
        "kind L4\n",
    );
    // **And under another Set**, so that *narrowed to `x`* is a claim with
    // something to be wrong about.
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

    // **A deck running the pair the run launched with**, which is every
    // deck of a fresh run: the versions under `None` are exactly the rows
    // this would list if a `None` matched anything, so an empty listing
    // here is the same claim as above read from the other side.
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

/// A landing writes the version's bytes over the node's working copy, and a
/// file that is gone is refused by name.
///
/// The write is the whole of what a landing does on this side: nothing touches
/// a deck, nothing sends an aim, and the watcher already looking at that file
/// is what builds it — ADR-0228's argument met from the other end. So what this
/// asserts is the bytes and the path: the file the version was of, resolved
/// through the slot's own nodes rather than through the copies the run launched
/// with.
///
/// The refusal names the file, because `rm -rf history/2026/07` is this store's
/// whole retention policy: a row whose file an operator deleted by hand is an
/// ordinary state, and P-0083 says a refusal carries what the next attempt
/// needs.
///
/// A CPU test: [`put_back`] takes a store, a slot and a `Slots`, and no device.
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

    // **The file behind the row is gone**, which is the retention policy
    // being used: the name is still in nothing this program keeps, so the
    // walk is re-asked and the row is simply not there.
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
    // **A newer version of another node of the same Set**, which is what
    // says the walk narrows by the node.
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

    // **A node with one version has nothing before what it is playing**,
    // which is an ordinary state rather than a fault — the first edit of a
    // node files one version, and a step back at that point has nowhere to
    // go. P-0083: the sentence names what is in the way.
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

    // **And a node the history has never heard of**, which is the same
    // refusal one step further out: nothing is filed, so there is not even
    // a version running.
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

/// A keep takes its own row off the lane and leaves every other row standing.
///
/// The half of the press that is not the operation: [`kept`] is where a
/// `KeepCandidate` is performed, because `written` answers
/// `Silent(Silent::Surface)` for it and there is no record for `apply` to move
/// a deck with. What it changes is one line in one list.
///
/// Two rows of one slot is the case worth the test, and it is what ADR-0326
/// made possible: a build that changed two nodes draws two rows on one deck, so
/// a keep that retired by *slot* would take a node nobody ruled on off the lane
/// with the one they did.
///
/// And a keep on a node with no row says so rather than reporting a press that
/// did nothing — no control on this panel can ask it, so the line is for the
/// day a map or a model reaches this row.
///
/// A CPU test: a `View` takes no device.
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

    // **A second keep on the same node has no row to take**, which is the
    // negative control: a version that retired by slot, or one that
    // ignored the node, would go on answering as though it had done
    // something.
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
