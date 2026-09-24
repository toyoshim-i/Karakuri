use super::*;

#[test]
fn list_procedures_orders_by_name_and_skips_what_the_layout_does_not_claim() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let procedures = dir.path().join("procedures");

    for name in ["orbit_wide", "beat_jump", "a-b"] {
        fs::write(procedures.join(format!("{name}.kir")), b"kind L3\n").unwrap();
    }
    // An editor's backup, a write that died, somebody's notes, and a
    // directory named like a procedure.
    fs::write(procedures.join("orbit_wide.kir~"), b"").unwrap();
    fs::write(procedures.join("half_written.kir.tmp"), b"").unwrap();
    fs::write(procedures.join("notes.txt"), b"").unwrap();
    fs::create_dir(procedures.join("old.kir")).unwrap();

    let listed: Vec<String> = store
        .list_procedures()
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(listed, ["a-b", "beat_jump", "orbit_wide"]);
    // Repeatable, which is why the key is the name rather than the time three
    // files written in one millisecond all share.
    assert_eq!(
        store.list_procedures().unwrap(),
        store.list_procedures().unwrap()
    );
}

#[test]
fn a_procedure_carries_when_it_was_written_and_is_read_back_whole() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // An empty directory lists nothing, which is a library nobody has kept
    // into rather than something gone wrong.
    assert_eq!(store.list_procedures().unwrap(), Vec::new());

    let before = SystemTime::now() - Duration::from_secs(2);
    let source = b"proc orbit_wide {\n  kind L3\n}\n";
    fs::write(dir.path().join("procedures").join("orbit_wide.kir"), source).unwrap();

    let entry = store.list_procedures().unwrap().pop().unwrap();
    assert_eq!(entry.name, "orbit_wide");
    assert!(entry.written > before, "the write time is not the file's");

    // Bytes, whole and unparsed — the store keeps what it does not read.
    assert_eq!(store.read_procedure("orbit_wide").unwrap(), source);
    assert!(matches!(
        store.read_procedure("no_such_thing"),
        Err(StoreError::NoProcedure(name)) if name == "no_such_thing"
    ));

    // The directory taken away under the store is an error and not "nothing
    // kept": the caller asked what is there and there is no answer.
    fs::remove_dir_all(dir.path().join("procedures")).unwrap();
    assert!(
        matches!(store.list_procedures(), Err(StoreError::Io(_))),
        "a missing procedures/ reads as an empty library"
    );
}

/// Verifies that writing a procedure saves the file in the procedures directory under the given name.
#[test]
fn a_keep_writes_the_procedure_under_the_name_it_was_given() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let source = b"proc orbit_wide {\n  kind L3\n}\n";

    let path = store.write_procedure("orbit_wide", source).unwrap();
    assert_eq!(
        path,
        dir.path().join("procedures").join("orbit_wide.kir"),
        "a keep landed somewhere other than the operator's own tier"
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        source,
        "the bytes were not kept whole"
    );
    assert_eq!(store.read_procedure("orbit_wide").unwrap(), source);
    assert_eq!(
        store
            .list_procedures()
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect::<Vec<_>>(),
        vec!["orbit_wide".to_string()],
        "what a keep wrote is not a row of the listing that reads it"
    );
}

/// Verifies that writing a procedure with an existing name fails with `ProcedureTaken`.
#[test]
fn a_keep_never_overwrites_a_name_already_there() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let first = b"proc orbit_wide {\n  kind L3\n}\n";
    store.write_procedure("orbit_wide", first).unwrap();

    match store.write_procedure("orbit_wide", b"proc other {\n  kind L1\n}\n") {
        Err(StoreError::ProcedureTaken(name)) => assert_eq!(name, "orbit_wide"),
        other => panic!("expected ProcedureTaken, got {other:?}"),
    }
    assert_eq!(
        store.read_procedure("orbit_wide").unwrap(),
        first,
        "a refused keep wrote over what was there"
    );
}

/// Verifies that model-generated procedures are written to the sandbox directory.
#[test]
fn a_models_keep_lands_in_the_sandbox_and_not_in_the_library() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let source = b"proc orbit_wide {\n  kind L3\n}\n";

    let path = store
        .write_sandbox_procedure("20260910-120000", source)
        .unwrap();
    assert_eq!(
        path,
        dir.path().join(Store::SANDBOX).join("20260910-120000.kir")
    );
    assert_eq!(fs::read(&path).unwrap(), source);
    assert!(
        store.list_procedures().unwrap().is_empty(),
        "a model's keep turned up in the listing the operator's library is read from"
    );
    assert!(matches!(
        store.write_sandbox_procedure("20260910-120000", b"x"),
        Err(StoreError::ProcedureTaken(_))
    ));
}

/// Verifies that listing an empty store returns empty collections without error.
#[test]
fn listing_an_empty_store_is_empty_and_not_an_error() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    assert_eq!(store.list_sets().unwrap(), []);
    assert_eq!(store.list_artifacts().unwrap(), []);
}

/// Verifies that listing a non-existent or removed directory returns an IO error.
#[test]
fn listing_a_removed_directory_is_an_error() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.write_set("drift_01", &a_set()).unwrap();
    store.put_artifact(b"proc p { kind L1 }").unwrap();

    fs::remove_dir_all(dir.path().join("sets")).unwrap();
    match store.list_sets() {
        Err(StoreError::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
        other => panic!("expected an io error for a missing sets/, got {other:?}"),
    }

    fs::remove_dir_all(dir.path()).unwrap();
    match store.list_artifacts() {
        Err(StoreError::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
        other => panic!("expected an io error for a missing root, got {other:?}"),
    }
}

/// Verifies that directory listings read filenames without opening or parsing file contents.
#[test]
fn listing_reads_names_and_not_contents() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    fs::write(
        dir.path().join("sets").join("garbage.kbset"),
        b"this is not ndjson\n",
    )
    .unwrap();
    let hash = store.put_artifact(b"proc p { kind L1 }").unwrap();
    fs::write(
        dir.path().join(format!("{}.meta.ndjson", hash.short(64))),
        b"neither is this\n",
    )
    .unwrap();

    let sets = store.list_sets().unwrap();
    assert_eq!(
        sets.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
        ["garbage"]
    );
    let artifacts = store.list_artifacts().unwrap();
    assert_eq!(artifacts.len(), 1);
    assert!(
        artifacts[0].has_meta,
        "an unparsable card is still a card — listing does not read it"
    );

    assert!(
        matches!(store.read_set("garbage"), Err(StoreError::Record { .. })),
        "the file parsed after all, so listing it proved nothing"
    );
}
