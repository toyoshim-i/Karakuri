#![allow(unused_imports)]

use super::wire_common::*;

/// Verifies that list_sets is advertised and returns Sets ordered by recency (with ID tie-breaking).
#[test]
fn the_listing_is_offered_and_comes_back_most_recent_first() {
    let server = start(true);
    let hash = stored(&server, PROBE_L1, true);
    for id in ["alpha", "beta", "gamma"] {
        set_of(&server, id, &[(Layer::L1, 0, Some("shell"), hash)]);
    }
    written_at(&server, "alpha", 1_000);
    written_at(&server, "beta", 2_000);
    written_at(&server, "gamma", 2_000);

    let (_, listed) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
    );
    let listed: Value = serde_json::from_str(&listed).expect("json");
    let names: Vec<String> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("name").to_string())
        .collect();
    assert!(
        names.iter().any(|name| name == "list_sets"),
        "a client is never told the tool exists: {names:?}"
    );

    let (failed, said) = call(server.port, "list_sets", json!({}));
    assert!(!failed, "{said}");
    assert!(
        at(&said, "beta") < at(&said, "alpha") && at(&said, "gamma") < at(&said, "alpha"),
        "the oldest set is not last: what did I just save is the question this is \
         mostly asked, and the answer is the wrong way round: {said}"
    );
    assert!(
        at(&said, "beta") < at(&said, "gamma"),
        "two sets written in one second came back in an order their ids do not \
         decide, so two calls on an unchanged store can disagree: {said}"
    );
}

/// Verifies that walk_history filters history specifically to the requested Set.
#[test]
fn a_walk_answers_one_sets_versions_and_never_a_row_filed_under_another() {
    let server = start(true);
    let root = store_root(&server.dir);
    let mut snaps = karakuri_environment::history::Snapshots::new(&root);
    for (slot, layer, set, name, source) in [
        (0usize, "L4", Some("night01"), "beat_strokes", &b"one"[..]),
        (0, "L4", Some("night01"), "beat_strokes", &b"two"[..]),
        (1, "L1", Some("day02"), "drift_shell", &b"three"[..]),
        // Material not filed under any named set.
        (2, "L2", None, "bend", &b"four"[..]),
    ] {
        snaps
            .record(slot, layer, 0, set, name, source)
            .expect("a snapshot is written");
    }

    let (_, listed) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
    );
    let listed: Value = serde_json::from_str(&listed).expect("json");
    let names: Vec<String> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("name").to_string())
        .collect();
    assert!(
        names.iter().any(|name| name == "walk_history"),
        "a client is never told the tool exists: {names:?}"
    );

    let (failed, said) = call(server.port, "walk_history", json!({"set":"night01"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("beat_strokes"),
        "the Set's own versions are not in its walk: {said}"
    );
    assert!(
        !said.contains("drift_shell"),
        "another Set's version is in this one's history: {said}"
    );
    assert!(
        !said.contains("bend"),
        "a version filed under no Set was folded into a Set's history, which is the \
         wildcard reading ADR-0276 refuses: {said}"
    );
    // The row identifier matches the landing name.
    assert!(
        !said.contains("@night01") && !said.contains(".kir"),
        "a row is not the name `Revision::Picked` takes: {said}"
    );

    let (failed, said) = call(server.port, "walk_history", json!({"set":"day02"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("drift_shell") && !said.contains("beat_strokes"),
        "the other Set's walk is not its own: {said}"
    );

    // A Set with no version history returns an empty list rather than an error.
    let (failed, said) = call(server.port, "walk_history", json!({"set":"nothing_here"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("no version of set `nothing_here`"),
        "an empty walk does not say it is empty: {said}"
    );

    // A request specifying no set is refused rather than returning arbitrary entries.
    let (failed, said) = call(server.port, "walk_history", json!({}));
    assert!(failed, "a walk with no Set was answered: {said}");
    assert!(said.contains("`set` is required"), "{said}");
}

/// Verifies that list_sets filters by `holds` and `layer` independently and together.
#[test]
fn the_filters_narrow_the_listing_and_can_be_combined() {
    let server = start(true);
    let l1 = stored(&server, PROBE_L1, true);
    let l2 = stored(&server, PROBE_L2, true);
    // Use mixed case in fixture and query to verify case-folding filter behavior.
    set_of(&server, "plain", &[(Layer::L1, 0, Some("Drift_Shell"), l1)]);
    set_of(
        &server,
        "warped",
        &[
            (Layer::L1, 0, Some("Drift_Shell"), l1),
            (Layer::L2, 0, Some("bend"), l2),
        ],
    );
    set_of(
        &server,
        "other",
        &[
            (Layer::L1, 0, Some("lattice"), l1),
            (Layer::L2, 0, Some("bend"), l2),
        ],
    );

    // Case folded, and the name typed back the way a model would shout it.
    let (failed, said) = call(server.port, "list_sets", json!({"holds":"DRIFT_shell"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("`plain`") && said.contains("`warped`") && !said.contains("`other`"),
        "`holds` did not select on what the nodes are called: {said}"
    );

    let (failed, said) = call(server.port, "list_sets", json!({"layer":"L2"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("`warped`") && said.contains("`other`") && !said.contains("`plain`"),
        "`layer` did not select on the layers a set holds: {said}"
    );

    let (failed, said) = call(
        server.port,
        "list_sets",
        json!({"holds":"drift","layer":"L2"}),
    );
    assert!(!failed, "{said}");
    assert!(
        said.contains("`warped`") && !said.contains("`plain`") && !said.contains("`other`"),
        "both filters given did not mean both must hold: {said}"
    );

    // A layer nothing spells is refused with the list, as every other tool
    // refuses one — not answered as though it had matched nothing.
    let (failed, said) = call(server.port, "list_sets", json!({"layer":"L9"}));
    assert!(
        failed,
        "a layer this language does not have was accepted: {said}"
    );
    assert!(
        said.contains("Field"),
        "the refusal does not say what the layers are: {said}"
    );
}

/// Verifies that list_sets caps output and clearly indicates remaining unlisted items.
#[test]
fn a_capped_listing_can_never_be_read_as_the_whole_library() {
    let server = start(true);
    let hash = stored(&server, PROBE_L1, true);
    let total = LISTED + 5;
    for n in 0..total {
        let id = format!("set{n:02}");
        set_of(&server, &id, &[(Layer::L1, 0, Some("shell"), hash)]);
        written_at(&server, &id, 1_000 + n as u64);
    }

    let (failed, said) = call(server.port, "list_sets", json!({}));
    assert!(!failed, "{said}");
    let listed = (0..total)
        .filter(|n| said.contains(&format!("`set{n:02}`")))
        .count();
    assert_eq!(
        listed, LISTED,
        "the cap did not hold: {listed} of {total} sets were rendered\n{said}"
    );
    for expected in [
        &format!("{total} sets"),
        &format!("the {LISTED} most recently written"),
        "5 more matched and are not listed",
        "Narrow it with `holds`",
    ] {
        assert!(
            said.contains(expected),
            "a truncated listing does not say `{expected}`, so it reads as the whole \
             library: {said}"
        );
    }
    // The most recent survive the cap, because the newest is what the
    // question was about.
    assert!(
        said.contains(&format!("`set{:02}`", total - 1))
            && !said.contains(&format!("`set{:02}`", 0)),
        "the cap kept the wrong end of the library: {said}"
    );
}

/// Verifies that an empty store and a non-matching filter return distinct, informative messages.
#[test]
fn an_empty_store_and_a_filter_that_matches_nothing_read_differently() {
    let server = start(true);
    let (failed, said) = call(server.port, "list_sets", json!({}));
    assert!(
        !failed,
        "an empty store was reported as a failed call: {said}"
    );
    assert!(
        said.contains("no sets at all") && said.contains("save_set"),
        "an empty store does not say what it is or where sets come from: {said}"
    );

    let hash = stored(&server, PROBE_L1, true);
    set_of(&server, "keeper", &[(Layer::L1, 0, Some("shell"), hash)]);
    let (failed, said) = call(
        server.port,
        "list_sets",
        json!({"holds":"nothing_like_this"}),
    );
    assert!(
        !failed,
        "a filter that matched nothing was an error: {said}"
    );
    assert!(
        !said.contains("no sets at all"),
        "a filter that matched nothing was answered as an empty store, which sends a \
         reader looking for material that is right there: {said}"
    );
    assert!(
        said.contains("none of the 1 set") && said.contains("The store is not empty"),
        "the no-match answer does not say the library is not empty: {said}"
    );
}

/// Verifies that node naming in list_sets matches read_set (custom name, proc name, or short hash).
#[test]
fn a_node_is_called_here_what_read_set_calls_it() {
    let server = start(true);
    let carded = stored(&server, PROBE_KNOBS, true);
    let uncarded = stored(&server, PROBE_L1, false);
    let elsewhere = Hash::of(b"stored on another machine");
    set_of(
        &server,
        "mixed",
        &[
            (Layer::L1, 0, Some("shell"), carded),
            (Layer::L1, 1, None, carded),
            (Layer::L2, 0, None, uncarded),
            (Layer::L4, 0, None, elsewhere),
        ],
    );

    let (failed, said) = call(server.port, "list_sets", json!({}));
    assert!(!failed, "{said}");
    for expected in [
        // The set's own name, which beats the card's.
        "L1:0 `shell`",
        // No name in the file, so what the procedure calls itself.
        "L1:1 `probe_knobs`",
        // No card at all: the short hash, and the node is listed.
        &format!("L2:0 `{}`", uncarded.short(12)),
        // Not in this store at all: the same, and still listed.
        &format!("L4:0 `{}`", elsewhere.short(12)),
    ] {
        assert!(
            said.contains(expected),
            "the listing does not name a node `{expected}`: {said}"
        );
    }

    // Identical names reported by the single-set inspection tool.
    let (failed, read) = call(server.port, "read_set", json!({"id":"mixed"}));
    assert!(!failed, "{read}");
    for expected in [
        "L1:0 `shell`",
        "L1:1 `probe_knobs`",
        &format!("L2:0 `{}`", uncarded.short(12)),
        &format!("L4:0 `{}`", elsewhere.short(12)),
    ] {
        assert!(
            read.contains(expected),
            "`read_set` calls a node something the listing does not: `{expected}` is \
             not in {read}"
        );
    }
}

/// A store that cannot be opened is an error with the path in it, the way
/// `read_set` reports one — not an empty library.
#[test]
fn a_store_that_cannot_be_opened_is_an_error_naming_the_path() {
    let server = start(true);
    // A file where the store's own root has to be: `Store::open` creates
    // the layout under it and cannot, so the open itself is what fails —
    // and the path is the only thing that tells an operator which store
    // this run was pointed at.
    let root = store_root(&server.dir);
    std::fs::write(&root, b"not a directory").expect("write");
    let (failed, said) = call(server.port, "list_sets", json!({}));
    assert!(
        failed,
        "a store that cannot be read answered as though it held nothing: {said}"
    );
    assert!(
        said.contains(&root.display().to_string()),
        "the failure does not say which store: {said}"
    );
}

/// Verifies that the set tool is advertised in tool discovery and returns
/// declared source parameters upon invocation.
#[test]
fn the_set_tool_is_offered_and_a_card_says_what_the_source_declared() {
    let server = start(true);
    kept(&server, "keeper", PROBE_KNOBS, true);

    let (_, listed) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
    );
    let listed: Value = serde_json::from_str(&listed).expect("json");
    let names: Vec<String> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("name").to_string())
        .collect();
    assert!(
        names.iter().any(|name| name == "read_set"),
        "a client is never told the tool exists: {names:?}"
    );

    let (failed, said) = call(server.port, "read_set", json!({"id":"keeper"}));
    assert!(!failed, "{said}");
    // Parameter values match the set declaration exactly.
    for expected in [
        "probe_knobs",
        "L1:0",
        "`shell`",
        "param radius : float, anywhere from 0.5 to 3.5",
        "and 1.75 until something turns it",
        "between 16 and 4096 elements, and 256 of them",
        "emits position",
    ] {
        assert!(
            said.contains(expected),
            "the card does not say `{expected}`: {said}"
        );
    }
    // Verify artifact declarations are shown rather than runtime instance values.
    assert!(
        !said.contains("2.5"),
        "a value this set holds was rendered as something the artifact \
         declares: {said}"
    );
}

/// Verifies that read_set correctly reports element storage memory allocation per node.
#[test]
fn a_saved_set_says_what_it_will_allocate_to_hold_elements() {
    let server = start(true);
    let store = server.store();
    let put = |source: &str| store.put_artifact(source.as_bytes()).expect("put");
    let (l1, l2, l4) = (put(PROBE_L1), put(PROBE_L2), put(PROBE_L4));
    store
        .write_set(
            "costed",
            &[
                Line::new(Record::Slot {
                    at: karakuri_store::record::NodeAddress {
                        layer: Layer::L1,
                        index: 0,
                    },
                    name: Some("shell".into()),
                    proc_hash: l1,
                }),
                Line::new(Record::Slot {
                    at: karakuri_store::record::NodeAddress {
                        layer: Layer::L2,
                        index: 0,
                    },
                    name: Some("warp".into()),
                    proc_hash: l2,
                }),
                Line::new(Record::Slot {
                    at: karakuri_store::record::NodeAddress {
                        layer: Layer::L4,
                        index: 0,
                    },
                    name: Some("dots".into()),
                    proc_hash: l4,
                }),
                Line::new(Record::Capacity {
                    at: karakuri_store::record::NodeAddress {
                        layer: Layer::L1,
                        index: 0,
                    },
                    value: 16,
                }),
            ],
        )
        .expect("set");

    let (failed, said) = call(server.port, "read_set", json!({"id":"costed"}));
    assert!(!failed, "{said}");
    for expected in [
        "element storage: 1664 bytes in total, across the 2 nodes",
        // The output uses indentation to separate storage rows from node block headers.
        "  `shell` — 1152 bytes for 16 elements, 72 bytes each",
        "  `warp` — 512 bytes for 16 elements, 32 bytes each",
    ] {
        assert!(
            said.contains(expected),
            "the set was not costed as `{expected}`: {said}"
        );
    }
    // Verify renderer draws from node above and isn't charged as separate element storage.
    assert!(
        !said.contains("  `dots` — "),
        "the renderer was charged for the buffer it draws from, which is the \
         node above it: {said}"
    );
    // Clarify that element storage is not total device memory.
    assert!(
        said.contains("NOT what this set costs a GPU"),
        "an element-storage figure is offered as though it were device \
         memory: {said}"
    );
}

/// Verifies that an uncarded artifact returns a helpful description rather than reporting store damage.
#[test]
fn an_artifact_with_no_card_is_answered_and_not_called_a_broken_store() {
    let server = start(true);
    kept(&server, "uncarded", PROBE_KNOBS, false);
    let (failed, said) = call(server.port, "read_set", json!({"id":"uncarded"}));
    assert!(
        !failed,
        "an artifact stored without a card was reported to a model as a failed \
         call: {said}"
    );
    assert!(
        said.contains("L1:0") && said.contains("no metadata card"),
        "the node was not described at all: {said}"
    );
    assert!(
        said.contains("not a damaged store"),
        "a card nobody has written yet reads as damage: {said}"
    );

    // If an artifact hash is missing entirely from store, report that artifact is missing.
    set_naming(&server, "elsewhere", Hash::of(b"stored on another machine"));
    let (failed, said) = call(server.port, "read_set", json!({"id":"elsewhere"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("does not hold that artifact"),
        "a set naming material this store has never had was answered as though \
         the source were here: {said}"
    );
}

/// Verifies that Set IDs containing path traversal or invalid characters are rejected.
#[test]
fn a_set_id_on_the_way_to_a_card_cannot_name_a_path() {
    let server = start(true);
    for bad in [
        "../../../etc/passwd",
        "sets/../../elsewhere",
        "a/b",
        "~/mine",
    ] {
        let (failed, said) = call(server.port, "read_set", json!({"id": bad}));
        assert!(failed, "`{bad}` was accepted as a set id: {said}");
        // Request rejected before accessing storage; refusal cites constraint rule.
        assert!(
            said.contains("path component") || said.contains("letters, digits"),
            "`{bad}` was refused for something other than being a path: {said}"
        );
        assert!(
            !said.contains("reading set"),
            "`{bad}` reached the filesystem: {said}"
        );
    }
}

/// A set nobody saved is refused by the id that was asked for, and says
/// where sets come from — the answer a model can act on, against an errno
/// it cannot.
#[test]
fn a_set_this_store_never_saw_is_refused_by_its_id() {
    let server = start(true);
    let (failed, said) = call(server.port, "read_set", json!({"id":"never_saved"}));
    assert!(
        failed,
        "a set that is not there answered as though it were: {said}"
    );
    assert!(said.contains("never_saved"), "{said}");
    assert!(said.contains("save_set"), "{said}");
}
