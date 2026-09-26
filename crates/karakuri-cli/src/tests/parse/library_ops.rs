use super::*;

// -- --list-sets -----------------------------------------------------

/// Verifies that `--list-sets` parses as a list action without initiating a runtime engine pass.
#[test]
fn list_sets_prints_and_is_never_a_run() {
    for spelling in [
        vec!["--list-sets", "--store", "/tmp/library"],
        vec!["--store", "/tmp/library", "--list-sets"],
    ] {
        match parse_args_from(spelling.iter().map(|s| s.to_string())) {
            Ok(ParseOutcome::ListSets(root)) => {
                assert_eq!(
                    root,
                    PathBuf::from("/tmp/library"),
                    "{spelling:?} listed a store nobody asked for"
                );
            }
            Ok(ParseOutcome::Run(_)) => panic!(
                "{spelling:?} came back as a run: a listing would then compile the \
                 default material and open a window to print a directory"
            ),
            Ok(ParseOutcome::Help) => panic!("{spelling:?} came back as --help"),
            Ok(ParseOutcome::Package { .. }) | Ok(ParseOutcome::TakeIn { .. }) => {
                panic!("{spelling:?} came back as a transfer")
            }
            Err(e) => panic!("{spelling:?} was refused: {e}"),
        }
    }
}

/// Verifies that `--package` and `--take-in` parse into transfer outcomes rather than runs.
#[test]
fn package_and_take_in_print_and_are_never_a_run() {
    for spelling in [
        vec!["--package", "night01", "--store", "/tmp/library"],
        vec!["--store", "/tmp/library", "--package", "night01"],
    ] {
        match parse_args_from(spelling.iter().map(|s| s.to_string())) {
            Ok(ParseOutcome::Package { store, id }) => {
                assert_eq!(store, PathBuf::from("/tmp/library"), "{spelling:?}");
                assert_eq!(id, "night01", "{spelling:?}");
            }
            Ok(ParseOutcome::Run(_)) => panic!(
                "{spelling:?} came back as a run: writing a package to stdout would then \
                 compile material and open a window to do it"
            ),
            other => panic!(
                "{spelling:?} came back as something else: {}",
                named(&other)
            ),
        }
    }
    for spelling in [
        vec!["--take-in", "sent.kbset", "--store", "/tmp/library"],
        vec!["--store", "/tmp/library", "--take-in", "sent.kbset"],
    ] {
        match parse_args_from(spelling.iter().map(|s| s.to_string())) {
            Ok(ParseOutcome::TakeIn { store, file }) => {
                assert_eq!(store, PathBuf::from("/tmp/library"), "{spelling:?}");
                assert_eq!(file, PathBuf::from("sent.kbset"), "{spelling:?}");
            }
            Ok(ParseOutcome::Run(_)) => panic!(
                "{spelling:?} came back as a run: taking a file into the store would \
                 then build a Set and open a window"
            ),
            other => panic!(
                "{spelling:?} came back as something else: {}",
                named(&other)
            ),
        }
    }
}

/// Which outcome, for a panic message. `ParseOutcome` holds GPU-adjacent config
/// and is deliberately not `Debug`; see `value_tests::parse`.
fn named(outcome: &Result<ParseOutcome, String>) -> String {
    match outcome {
        Ok(ParseOutcome::Run(_)) => "a run".to_string(),
        Ok(ParseOutcome::Help) => "--help".to_string(),
        Ok(ParseOutcome::ListSets(_)) => "--list-sets".to_string(),
        Ok(ParseOutcome::Package { .. }) => "--package".to_string(),
        Ok(ParseOutcome::TakeIn { .. }) => "--take-in".to_string(),
        Err(e) => format!("a refusal: {e}"),
    }
}

/// Creates a test Store populated with two Sets timestamped with specific mtimes.
fn library(root: &std::path::Path) -> karakuri_store::store::Store {
    let store = karakuri_store::store::Store::open(root).expect("store");
    let hash = store.put_artifact(b"not compiled here").expect("put");
    let slot = |layer: Layer, index: u32, name: &str| {
        karakuri_store::ndjson::Line::new(Record::Slot {
            at: karakuri_store::record::NodeAddress { layer, index },
            name: Some(name.to_string()),
            proc_hash: hash,
        })
    };
    store
        .write_set(
            "older",
            &[slot(Layer::L1, 0, "shell"), slot(Layer::L4, 0, "dots")],
        )
        .expect("set");
    store
        .write_set(
            "newer",
            &[
                slot(Layer::L1, 0, "shell"),
                slot(Layer::L2, 0, "bend"),
                slot(Layer::L4, 0, "dots"),
                slot(Layer::L4, 1, "strokes"),
            ],
        )
        .expect("set");
    for (id, secs) in [("older", 1_000u64), ("newer", 2_000)] {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(root.join("sets").join(format!("{id}.kbset")))
            .expect("open the set file");
        file.set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(secs)),
        )
        .expect("set the mtime");
    }
    store
}

/// Verifies that `listed_sets` outputs one line per Set, ordered newest to oldest.
#[test]
fn the_listing_is_a_line_per_set_most_recent_first() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("store");
    let store = library(&root);
    let said = listed_sets(&store, &root).expect("the listing");

    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(lines.len(), 2, "one line per set, and no more: {said}");
    assert!(
        lines[0].starts_with("newer") && lines[1].starts_with("older"),
        "the sets are not most recent first: {said}"
    );
    assert!(
        lines[0].contains("1 L1, 1 L2, 2 L4") && !lines[0].contains("L3"),
        "the line does not say what the set holds by layer, or counts a layer \
         nothing is on: {said}"
    );
    assert!(
        lines[1].contains("1 L1, 1 L4"),
        "the line does not say what the set holds by layer: {said}"
    );
    // The times are the ones the fixture set, in the operator's own clock,
    // and they are what tells two takes of one evening apart.
    for (line, secs) in lines.iter().zip([2_000u64, 1_000]) {
        let written = setfile::written_at(std::time::UNIX_EPOCH + Duration::from_secs(secs));
        assert!(
            line.contains(&written),
            "the line does not say when the set was written: {line}"
        );
    }
}

/// An empty store is an ordinary answer and says where sets come from — not a
/// blank terminal, which reads as a flag that did nothing.
#[test]
fn an_empty_store_says_so_rather_than_printing_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("store");
    let store = karakuri_store::store::Store::open(&root).expect("store");
    let said = listed_sets(&store, &root).expect("the listing");
    assert!(
        said.contains("no sets in") && said.contains("--save-set"),
        "an empty store printed something an operator cannot act on: {said:?}"
    );

    // **And a path with no store at it is a different answer, and creates
    // nothing.** `Store::open` would establish the layout under it, so a
    // listing of a mistyped `--store` would answer "empty" and leave a
    // directory behind saying so — a read-only flag that writes.
    let missing = dir.path().join("nowhere");
    let said = listed_sets_at(&missing).expect("the listing");
    assert!(
        said.contains("no store at") && said.contains("--store"),
        "a path with no store at it was not told apart from an empty one: {said:?}"
    );
    assert!(
        !missing.exists(),
        "a listing created the store it was asked to read"
    );
}

/// Verifies that `--package` converts an authored `.kset` file into a self-contained bundle.
#[test]
fn package_takes_an_authoring_file_in_and_packages_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1 = "proc ring { kind L1 }\n";
    let l4 = "proc points { kind L4 }\n";
    std::fs::write(dir.path().join("l1.kir"), l1).expect("write");
    std::fs::write(dir.path().join("l4.kir"), l4).expect("write");
    let kset = dir.path().join("night.kset");
    std::fs::write(
        &kset,
        "{\"t\":\"set\",\"id\":\"night\",\"v\":1}\n\
         {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
         {\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}\n",
    )
    .expect("write");

    let root = dir.path().join("store");
    let said = packaged_set(&root, kset.to_str().expect("utf-8")).expect("the packaging");

    assert!(
        !said.contains("\"t\":\"part\""),
        "a bundle carries no part: {said}"
    );
    assert_eq!(
        said.matches("\"t\":\"slot\"").count(),
        2,
        "each part became a slot naming an address: {said}"
    );
    for source in [l1, l4] {
        let hash = karakuri_store::hash::Hash::of(source.as_bytes());
        assert!(
            said.contains(&hash.to_string()),
            "the slot names the address of the bytes on disk: {said}"
        );
        assert!(
            karakuri_store::store::Store::open(&root)
                .expect("store")
                .get_artifact(&hash)
                .is_ok(),
            "the store this wrote into holds the part: {said}"
        );
    }
    assert!(
        said.contains("\"t\":\"src\""),
        "the sources are inlined, which is what makes it a bundle: {said}"
    );

    // **And an id is still an id.** The extension is the whole of what
    // tells the two apart, so a value without one is looked up in the store
    // and says so when it is not there.
    let refused = packaged_set(&root, "night").expect_err("no set is filed under that id");
    assert!(refused.contains("night"), "{refused}");
}

/// Verifies that `--take-in` resolves an authoring `.kset` file and imports it into the store.
#[test]
fn take_in_resolves_an_authoring_file_and_then_takes_it_in() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Sources that actually compile, because a take-in runs the checker to
    // write a metadata card and a source it will not compile is stored
    // without one.
    let l1 = r#"
proc ring {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  emit position

  element {
position = vec3(cos(t), sin(t), 0.0);
  }
}
"#;
    let l4 = r#"
proc points {
  kind  L4
  blend additive

  consumes position

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.016;
  }

  fragment {
color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    std::fs::write(dir.path().join("l1.kir"), l1).expect("write");
    std::fs::write(dir.path().join("l4.kir"), l4).expect("write");
    let kset = dir.path().join("night.kset");
    std::fs::write(
        &kset,
        "{\"t\":\"set\",\"id\":\"night\",\"v\":1}\n\
         {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
         {\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}\n",
    )
    .expect("write");

    let root = dir.path().join("store");
    let said = taken_in_file(&root, &kset).expect("the take-in");
    assert!(
        said.contains("`night`") && said.contains("2 sources stored"),
        "the report does not say what was taken in: {said}"
    );
    assert!(
        said.contains("2 metadata cards written"),
        "a take-in writes a card per source, whichever form it came in: {said}"
    );

    let store = karakuri_store::store::Store::open(&root).expect("store");
    assert!(
        store
            .list_sets()
            .expect("sets")
            .iter()
            .any(|e| e.id == "night"),
        "the set is not filed under the id the file carries"
    );
    for source in [l1, l4] {
        let hash = karakuri_store::hash::Hash::of(source.as_bytes());
        assert!(
            store.get_artifact(&hash).is_ok(),
            "the store does not hold the part the authoring file named"
        );
    }

    // **And the id in the file is still somebody else's word.** Taking the
    // same authoring file in twice is the refusal a `.kbset` gets, for the
    // same reason: nothing here was typed by the operator.
    let refused = taken_in_file(&root, &kset).expect_err("the id is taken");
    assert!(
        refused.contains("already in this store"),
        "a second take-in overwrote a Set: {refused}"
    );
}
