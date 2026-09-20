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

/// **A keep writes the file, under the name it was given and nowhere else** —
/// the act that makes the operator's tier exist at all
/// (`docs/principles/0096-…`, ADR-0338 decision 4).
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

/// **A name already kept is refused and never overwritten**, which is where a
/// procedure differs from a Set id and an arrangement's name: those two are
/// instructions to replace what is under them, and this one would replace a
/// part of somebody's library with a different node's source.
///
/// The refusal carries the name back, which is the whole of what the next
/// attempt needs (P-0083).
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

/// **A model's keep lands in the sandbox**, which is a Set save's own division
/// one file kind along: the operator's library is written by an operator's own
/// act, and what a model keeps is a file with a sandbox form to land in
/// (ADR-0261, ADR-0301).
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
    // And it refuses a taken name there too, on the library's terms: a stamp
    // met twice is a clock that has not moved, and overwriting would lose the
    // earlier of the two.
    assert!(matches!(
        store.write_sandbox_procedure("20260910-120000", b"x"),
        Err(StoreError::ProcedureTaken(_))
    ));
}

/// A store with nothing in it lists nothing — including the three
/// subdirectories `Store::open` just made, which sit in the artifact root and
/// are not artifacts.
#[test]
fn listing_an_empty_store_is_empty_and_not_an_error() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    assert_eq!(store.list_sets().unwrap(), []);
    assert_eq!(store.list_artifacts().unwrap(), []);
}

/// **A store whose directory went away is an error, not an empty library.**
///
/// The two answers look alike and mean opposite things: one says nothing is
/// kept, the other says we could not find out. An operator who is told the
/// first will generate the thing they already had.
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

/// **Listing reads names, never contents.**
///
/// A library of two thousand Sets should cost one directory read, and the way
/// that claim is checked without timing anything is to make the contents
/// unreadable: every file here fails to parse, and the listing has to come back
/// whole anyway. The `read_set` at the end is what keeps the test honest — it
/// proves the bytes really are unparsable, so the listing's success means it
/// never looked.
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
