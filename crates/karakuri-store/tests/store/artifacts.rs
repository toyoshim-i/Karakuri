use super::*;

#[test]
fn a_procedure_is_not_an_artifact_and_an_artifact_is_not_a_procedure() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // Every build's sources live at the root under a hash; a kept procedure
    // lives under `procedures/` under a name. Neither listing sees the other,
    // which is the whole of ADR-0338's *the artifacts are not the library*.
    store.put_artifact(b"proc orbit_wide { kind L3 }").unwrap();
    fs::write(
        dir.path().join("procedures").join("orbit_wide.kir"),
        b"kind L3\n",
    )
    .unwrap();

    assert_eq!(store.list_artifacts().unwrap().len(), 1);
    let procedures = store.list_procedures().unwrap();
    assert_eq!(procedures.len(), 1);
    assert_eq!(procedures[0].name, "orbit_wide");
}

#[test]
fn put_get_round_trips() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let source = b"proc drift_shell { kind L1 }";
    let hash = store.put_artifact(source).unwrap();
    let back = store.get_artifact(&hash).unwrap();

    assert_eq!(back, source);
}

#[test]
fn put_is_content_addressed_and_idempotent() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let source = b"proc drift_shell { kind L1 }";
    let hash1 = store.put_artifact(source).unwrap();
    let hash2 = store.put_artifact(source).unwrap();
    assert_eq!(hash1, hash2);

    // The file on disk must still hold exactly the original bytes: a
    // second put must not rewrite (or corrupt) what is there.
    let back = store.get_artifact(&hash1).unwrap();
    assert_eq!(back, source);

    // Different content gets a different address.
    let other_hash = store.put_artifact(b"proc other { kind L1 }").unwrap();
    assert_ne!(hash1, other_hash);
}

#[test]
fn put_writes_directly_under_root_by_bare_hex() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let hash = store.put_artifact(b"proc p { kind L1 }").unwrap();

    let expected = dir.path().join(format!("{}.kir", hash.short(64)));
    assert!(expected.is_file(), "expected artifact at {expected:?}");
}

#[test]
fn get_missing_artifact_is_not_found() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let hash = Hash::of(b"never written");
    match store.get_artifact(&hash) {
        Err(StoreError::NotFound(h)) => assert_eq!(h, hash),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

/// The same claim for artifacts, plus the ordering that makes a listing
/// reproducible. Expected order is built from the hex spellings rather than
/// from `Hash`'s own `Ord`, so the test says "ascending hex" rather than
/// agreeing with whatever the implementation sorted by.
#[test]
fn list_artifacts_orders_by_hash_and_repeats_that_order() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let mut expected: Vec<String> = ["proc a {}", "proc b {}", "proc c {}", "proc d {}"]
        .iter()
        .map(|s| store.put_artifact(s.as_bytes()).unwrap().short(64))
        .collect();
    expected.sort();

    let listed: Vec<String> = store
        .list_artifacts()
        .unwrap()
        .into_iter()
        .map(|e| e.hash.short(64))
        .collect();
    assert_eq!(listed, expected);
    assert_eq!(
        store.list_artifacts().unwrap(),
        store.list_artifacts().unwrap()
    );
}

/// **An artifact without a card is an ordinary artifact.**
///
/// `read_meta` already says so, and this is the same statement made in bulk:
/// the uncarded one is listed, not skipped and not an error, and the flag is
/// the difference. A caller that had to discover this by calling `read_meta`
/// per artifact would be reading an ordinary answer out of an error variant,
/// once per artifact, opening a file each time to do it.
#[test]
fn list_artifacts_flags_the_carded_and_the_uncarded() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let carded = store.put_artifact(b"proc carded { kind L1 }").unwrap();
    let bare = store.put_artifact(b"proc bare { kind L1 }").unwrap();
    store
        .write_meta(
            &carded,
            &[Line::new(Record::Meta {
                hash: carded,
                name: "carded".into(),
                kind: Layer::L1,
                v: 1,
            })],
        )
        .unwrap();

    let listed = store.list_artifacts().unwrap();
    assert_eq!(listed.len(), 2, "the uncarded artifact was dropped");
    let flag = |h| {
        listed
            .iter()
            .find(|e| e.hash == h)
            .unwrap_or_else(|| panic!("{h} missing from the listing"))
            .has_meta
    };
    assert!(flag(carded), "an artifact with a card was reported bare");
    assert!(!flag(bare), "an artifact with no card was reported carded");
}

/// **A card is not an artifact, and neither is a directory.**
///
/// Both live under the same root as the `.kir` files, so both are in front of
/// any implementation that lists that directory. The stray card is the case
/// `write_meta` documents — it writes one without checking the artifact exists
/// — and counting it would put a hash in the library that `get_artifact` cannot
/// serve. The directory is named `<hash>.kir` on purpose: the suffix and the
/// hash both check out, and it is still not an artifact.
#[test]
fn list_artifacts_reports_neither_a_stray_card_nor_a_directory() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let real = store.put_artifact(b"proc real { kind L1 }").unwrap();
    let never_put = Hash::of(b"proc never_put { kind L1 }");
    store.write_meta(&never_put, &[]).unwrap();
    fs::create_dir(
        dir.path()
            .join(format!("{}.kir", Hash::of(b"a directory").short(64))),
    )
    .unwrap();

    let listed = store.list_artifacts().unwrap();
    assert_eq!(
        listed.iter().map(|e| e.hash).collect::<Vec<_>>(),
        [real],
        "something that is not an artifact was listed as one"
    );
}

/// **A name this store would not have written is not an artifact**, and none
/// of these may panic on the way to being ignored.
///
/// The uppercase case is the subtle one: it parses to a valid address, whose
/// `.kir` path is then the lowercase spelling — so listing it would report a
/// hash `get_artifact` immediately fails to find. The non-UTF-8 name is the one
/// that punishes an `unwrap` on `to_str`, and nothing stops a user from
/// creating it.
#[test]
fn list_artifacts_skips_names_this_store_would_not_have_written() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let real = store.put_artifact(b"proc real { kind L1 }").unwrap();
    // Uppercase hex, under an address nothing was ever put at — writing the
    // real artifact's address in uppercase would land on the real artifact's
    // own file on a case-insensitive volume, which is most of macOS.
    let shouty = Hash::of(b"proc shouty { kind L1 }")
        .short(64)
        .to_uppercase();
    fs::write(dir.path().join(format!("{shouty}.kir")), b"upper case hex").unwrap();
    fs::write(dir.path().join("proc_drift_shell.kir"), b"not hex at all").unwrap();
    fs::write(
        dir.path().join(format!("{}.kir", "z".repeat(64))),
        b"64 non-hex",
    )
    .unwrap();
    fs::write(
        dir.path().join(format!("{}.kir", real.short(32))),
        b"too short",
    )
    .unwrap();
    fs::write(dir.path().join(".kir"), b"no stem").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        // Best-effort: APFS validates filenames and refuses this outright
        // (EILSEQ), so on macOS the case cannot be staged at all and the
        // listing is left to prove the rest. On a filesystem that does allow
        // it — ext4, and every volume this store might be kept on over a
        // network share — the file lands and an `unwrap` on `to_str` dies here.
        let name = std::ffi::OsStr::from_bytes(b"\xff\xfe.kir");
        let _ = fs::write(dir.path().join(name), b"not utf-8");
    }

    let listed = store.list_artifacts().unwrap();
    assert_eq!(listed.iter().map(|e| e.hash).collect::<Vec<_>>(), [real]);
    // And what was listed is what the store can actually serve.
    assert_eq!(
        store.get_artifact(&listed[0].hash).unwrap(),
        b"proc real { kind L1 }"
    );
}
