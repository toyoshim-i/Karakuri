#![allow(unused_imports)]

use super::wire_common::*;

/// **The tool is offered, and the library comes back most recent first.**
///
/// The two halves are one test for the reason the `read_set` pair are: a
/// tool a client is never told about and a tool that answers nothing are
/// both invisible, and this is the pass that says a model can find it and
/// use it in one go.
///
/// **`beta` and `gamma` are written in the same second on purpose.** The
/// store's own order is by id and is total; this surface sorts by recency,
/// and a sort on a coarse clock's seconds has ties — so the tie-break by id
/// is the whole reason two calls on an unchanged store say the same thing.
/// Without it these two would come back in whatever order `read_dir` felt
/// like, which is not an order at all.
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

/// **A walk answers one Set's versions, and a row filed under another Set
/// or under none is never in it.**
///
/// The narrowing is this surface's — `history::list` hands over one ordered
/// listing with the whole address on every row — so the filter is written
/// here and has to be checked here. **The `None` row is the half that
/// matters**: a version written while a slot was running material nobody
/// had saved is a version of *nothing*, and a filter that let it through
/// would be inventing a history for whichever Set was asked about
/// (ADR-0276, ADR-0308).
///
/// **Watched to fail** against three defects: a narrowing on
/// `version.set.is_none() || version.set.as_deref() == Some(id)`, which is
/// the wildcard reading and puts `bend` in the answer; a walk that returned
/// the whole listing, which puts `drift_shell` in it; and a `set` argument
/// read as optional, which answers a call that named no Set with somebody
/// else's edits.
#[test]
fn a_walk_answers_one_sets_versions_and_never_a_row_filed_under_another() {
    let server = start(true);
    let root = store_root(&server.dir);
    let mut snaps = karakuri_environment::history::Snapshots::new(&root);
    for (slot, layer, set, name, source) in [
        (0usize, "L4", Some("night01"), "beat_strokes", &b"one"[..]),
        (0, "L4", Some("night01"), "beat_strokes", &b"two"[..]),
        (1, "L1", Some("day02"), "drift_shell", &b"three"[..]),
        // **Filed under no Set**, which is a run playing the pair it was
        // launched with — the row no id matches.
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
    // **A row is the name a landing names it back by**, which is the name
    // less the `@<set>` every row of one walk shares.
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

    // **A Set with no versions is an answer and not a failure**, and it is
    // a different answer from a store with no history at all.
    let (failed, said) = call(server.port, "walk_history", json!({"set":"nothing_here"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("no version of set `nothing_here`"),
        "an empty walk does not say it is empty: {said}"
    );

    // **And a call that names no Set is refused rather than answered with
    // whatever the store holds.**
    let (failed, said) = call(server.port, "walk_history", json!({}));
    assert!(failed, "a walk with no Set was answered: {said}");
    assert!(said.contains("`set` is required"), "{said}");
}

/// **Both filters, apart and together.**
///
/// `holds` is the "which of these use `drift_shell`" question and `layer` is
/// the "which of these deform something" one, and the pair is the reason
/// each is a filter rather than something a reader does by eye over twenty
/// lines. Case is folded because a model that read a name in one answer and
/// typed it back with a capital is asking the same question.
///
/// **Together they are asked of the set and not of one node.** `holds` and
/// `layer` matching the same node would answer a question nobody has — the
/// useful one is *which of the sets built on this also deform something*,
/// and there the deformation is a different node with a different name.
#[test]
fn the_filters_narrow_the_listing_and_can_be_combined() {
    let server = start(true);
    let l1 = stored(&server, PROBE_L1, true);
    let l2 = stored(&server, PROBE_L2, true);
    // **Capitals in the fixture's own name and not only in the query.**
    // Folding one side and not the other passes any fixture where the
    // stored name is already lowercase, which is most of them — so the name
    // the set carries is spelled the way an operator types a name and the
    // query is spelled the way a model shouts one.
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

/// **A capped listing says what it dropped.**
///
/// A library is not bounded by anything — a run that presses `k` between
/// takes keeps one a minute — so an answer that rendered whatever it found
/// would eventually be an answer nobody can read. The cap is not the
/// interesting half: a model told "here are your sets" over twenty of
/// twenty-five will tell its user they have twenty and then act on a
/// library it has not seen. So the count that matched, the count shown and
/// the fact that the filters narrow it are all in the text.
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

/// **An empty store and a filter that matches nothing are both answers, and
/// they are different answers.**
///
/// Neither is an error: a store nobody has saved into is what every store
/// starts as, and a filter that selects none of twenty sets is the filter
/// doing its job. They read differently because they send a reader to
/// different places — one to `save_set`, the other to a different filter —
/// and being told "nothing matches" by an empty library is being told to go
/// looking for material that was never there.
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

/// **What a node is called is one answer, and every node has one.**
///
/// The three cases are the three candidates, in order: the name this set
/// gave the node, the name its procedure gives itself, and the short hash
/// where there is neither. The last two are the ones worth building a
/// fixture for, because both are *ordinary* states of a working store —
/// `Store::put_artifact` writes no card, and a set saved on another machine
/// names artifacts this store has never had — and a listing that dropped
/// either would be a library with holes in it.
///
/// **And it is checked against `read_set`'s own answer**, which is the
/// point of the derivation being one function: a model that picks a set out
/// of a listing and then reads it must find the node it was told about.
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

    // **The same names, from the tool that reads one set.** Two derivations
    // that agree today are two answers that stop agreeing the day one is
    // edited, and this is the assertion that would notice.
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

/// **The tool is offered, and what comes back is what the source
/// declared.**
///
/// The two halves are one test on purpose: a tool that is advertised and
/// answers nothing, and one that answers without being advertised, are both
/// invisible to a client, and this is the pass that says a model can find it
/// and use it in one go.
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
    // **The declaration, number for number.** Each of these is in the
    // `.kir` above and in no other fixture, so a rendering that reached for
    // the wrong end of a range, or that answered off a card it built itself,
    // says a number that is not here.
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
    // **What the Set turned it to is not what the artifact declares.** The
    // fixture's `param` record holds 2.5 and the tool answers about
    // declarations; a reader folding the two would print it as a range or as
    // a default, and either is the `param_decl` / `param` confusion the
    // record vocabulary keeps two names to prevent.
    assert!(
        !said.contains("2.5"),
        "a value this set holds was rendered as something the artifact \
         declares: {said}"
    );
}

/// **What a saved set will allocate to hold elements, without building
/// it.**
///
/// `Set::element_storage` has reported this per node since the buffers
/// existed and nothing in this tree printed it; the figure that *was*
/// printed, at stage 4, was a second arithmetic over one procedure's `emit`
/// list and was 85% low. So the test is not that a number appears — it is
/// that the number is the one the allocation is sized by, over material
/// where a per-procedure reading would say something else:
///
/// - **`warp` is charged for `position` and it never mentions it.** An L2
///   writes everything that reached it, so it is sized at the chain's
///   stride; a figure read off its own `deform` block would be another
///   number entirely.
/// - **`dots` has no row at all.** A renderer draws from the buffer the
///   node above it allocated, so a row for it would be the same memory
///   counted twice — and a zero would be a number the reader has to work
///   out the meaning of.
/// - **Sixteen elements and not the eight the procedure defaults to.** The
///   `capacity` record is what this set was saved at, and a figure computed
///   from the declaration instead would be exactly half of every number
///   below while looking just as plausible.
///
/// The bytes are hand-walked from WGSL's placement rules, the same way
/// `karakuri-engine`'s own storage tests are, so nothing here is the engine
/// compared against itself: `emit position` lays out `seed` at 0, then
/// `birth_frac` at 4, then `position` at 16 — 28 bytes rounded up to the
/// struct's 16-byte alignment, so a stride of 32. A geometry keeps two
/// directions of the element buffer and two of the four-byte liveness flag
/// and, having neither `spawn` nor `kill()`, nothing else: `2 * 16 * (32 +
/// 4)` is 1152. The deform keeps one buffer at that stride and no flags of
/// its own: `16 * 32` is 512.
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
        // **With the indentation, because the assertion below discriminates
        // on it.** A storage row is indented and a node block's head is
        // not; a bare substring here would keep passing on the day the
        // indent went away, and the negative assertion would then be
        // asserting nothing.
        "  `shell` — 1152 bytes for 16 elements, 72 bytes each",
        "  `warp` — 512 bytes for 16 elements, 32 bytes each",
    ] {
        assert!(
            said.contains(expected),
            "the set was not costed as `{expected}`: {said}"
        );
    }
    // **The storage rows are indented and the node blocks are not**, which
    // is what tells the two apart now that a node block names the node in
    // its own head — `L4:0 \`dots\` — stored as …` is the renderer being
    // described, and `  \`dots\` — 512 bytes` would be the renderer being
    // charged for a buffer it does not own.
    assert!(
        !said.contains("  `dots` — "),
        "the renderer was charged for the buffer it draws from, which is the \
         node above it: {said}"
    );
    // **The sentence that keeps this from being read as device memory.**
    // The withdrawn figure's mistake was as much in what it was taken to
    // mean as in its arithmetic, and a number a model relays as "what this
    // costs a GPU" is that mistake in a new costume.
    assert!(
        said.contains("NOT what this set costs a GPU"),
        "an element-storage figure is offered as though it were device \
         memory: {said}"
    );
}

/// **An artifact with no card is described, not reported as a broken
/// store.**
///
/// `Store::put_artifact` writes no card of its own — it takes bytes and does
/// not compile — so this is the ordinary state of anything stored before
/// cards existed or stored without one, and `Store::read_meta` answers it
/// with the same `NotFound` it answers a damaged library with. What a model
/// must not be handed is a failed call about a store that is fine.
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

    // **A hash this store has never seen is the other absence**, and it is a
    // different fact: the Set cannot be loaded here at all. Both arrive as
    // one `NotFound`, so a reader that did not ask the second question tells
    // a model to go read a source that is not there.
    set_naming(&server, "elsewhere", Hash::of(b"stored on another machine"));
    let (failed, said) = call(server.port, "read_set", json!({"id":"elsewhere"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("does not hold that artifact"),
        "a set naming material this store has never had was answered as though \
         the source were here: {said}"
    );
}

/// **A set id from a client is one path component on the way to a card as
/// much as on the way to a save.**
///
/// `save_set` puts a client's id through [`checked_id`] and this reads a
/// file under `<store>/sets/` by the same spelling — paths never cross this
/// protocol, and a *read* is the direction that hands the file back.
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
        // **Refused before anything was opened.** The refusal names the rule
        // rather than an errno, which is also how it is told apart from the
        // one a real read of a missing file produces.
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
