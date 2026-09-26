#![allow(unused_imports)]

use super::common::*;

// -- Bundling --------------------------------------------------------

/// The text a bundle is written out as, back through the reader — so a test
/// round-trips through the *file*, which is what `--package >` writes and what
/// `--take-in` reads, rather than through records held in memory that could not
/// have survived a serialisation.
fn as_a_file(lines: &[Line]) -> Vec<Line> {
    parsed(
        &lines
            .iter()
            .map(|line| format!("{}\n", line.as_str()))
            .collect::<String>(),
    )
}

/// Verifies round-trip bundling and unbundling across separate store instances.
#[test]
fn a_bundle_loads_in_a_store_that_has_never_seen_the_artifacts() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let sent = as_a_file(&bundle(&store, "s1").expect("bundle"));

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let said = unbundle(&bare, CameFrom::Somebody, &sent).expect("the file carries its own source");
    assert!(said.contains("`s1`"), "{said}");

    let loaded = load(&bare, "s1").expect("the set is filed and its artifacts are here");
    assert_eq!(loaded.l1s[0].name, "ring");
    assert_eq!(loaded.l4s[0].name, "points");
    for hash in [
        Hash::of(&std::fs::read(&l1).expect("read")),
        Hash::of(&std::fs::read(&l4).expect("read")),
    ] {
        bare.read_meta(&hash)
            .unwrap_or_else(|e| panic!("{}: {e}", hash.short(12)));
    }
}

/// Ensures bundling is refused if any referenced procedure is missing from the store.
#[test]
fn a_bundle_is_refused_when_the_store_lacks_a_source() {
    let (_dir, store, l1, _l4) = fixture();
    let here = stored(&store, &l1);
    let missing = Hash::of(b"a renderer that was never put in this store");
    let text = format!(
        r#"{{"t":"set","id":"gone","v":1}}
{{"t":"slot","layer":"L1","proc":"{here}"}}
{{"t":"slot","layer":"L4","name":"veil","proc":"{missing}"}}
"#
    );
    store.write_set("gone", &parsed(&text)).expect("write");

    let e = bundle(&store, "gone").expect_err("a bundle cannot carry what is not there");
    assert!(e.contains("veil"), "the node is not named: {e}");
    assert!(e.contains(&missing.short(12)), "{e}");
}

/// Two nodes over one artifact inline it once. The reader keys `src` by hash,
/// so a second run would be a second copy of the same bytes that nothing ever
/// reads — and this Set is one geometry drawn twice by the same renderer, which
/// is the ordinary way that happens.
#[test]
fn one_artifact_referenced_twice_is_inlined_once() {
    let (_dir, store, l1, l4) = fixture();
    let twice = vec![l4.clone(), l4.clone()];
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, &twice), &[]),
    )
    .expect("save");
    let bundled = bundle(&store, "s1").expect("bundle");

    let renderer = Hash::of(&std::fs::read(&l4).expect("read"));
    let slots = bundled
            .iter()
            .filter(|line| matches!(line.record(), Record::Slot { proc_hash, .. } if *proc_hash == renderer))
            .count();
    assert_eq!(slots, 2, "the fixture is meant to name the renderer twice");
    // One run, so line 0 appears once.
    let heads = bundled
        .iter()
        .filter(
            |line| matches!(line.record(), Record::Src { hash, line: 0, .. } if *hash == renderer),
        )
        .count();
    assert_eq!(heads, 1, "the renderer's source was inlined {heads} times");
    // And the run is whole: as many `src` records as the source has lines.
    let run = bundled
        .iter()
        .filter(|line| matches!(line.record(), Record::Src { hash, .. } if *hash == renderer))
        .count();
    assert_eq!(run, L4.split('\n').count());
}

/// Ensures unbundling rejects inlined source content that does not hash to its slot address.
#[test]
fn an_unbundle_refuses_a_source_that_does_not_hash_to_its_address() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let renderer = Hash::of(&std::fs::read(&l4).expect("read"));
    // One line of the renderer's inlined source rewritten, everything else
    // — the `slot` record's hash included — left exactly as written.
    let tampered: Vec<Line> = bundle(&store, "s1")
        .expect("bundle")
        .into_iter()
        .map(|line| match line.record() {
            Record::Src { hash, line: at, .. } if *hash == renderer && *at == 1 => {
                Line::new(Record::Src {
                    hash: renderer,
                    line: 1,
                    s: "proc points_but_not_really {".to_string(),
                })
            }
            _ => line,
        })
        .collect();

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let e = unbundle(&bare, CameFrom::Somebody, &as_a_file(&tampered))
        .expect_err("text that is not the bytes its address names");
    // The node, by the address the file gives it and by what it is called
    // — which in a store with no card for it is its short hash.
    assert!(e.contains("L4:0"), "the node is not named: {e}");
    assert!(e.contains(&renderer.short(12)), "{e}");
    assert!(
        bare.list_artifacts().expect("list").is_empty(),
        "a refused bundle left an artifact behind"
    );
    assert!(
        bare.list_sets().expect("list").is_empty(),
        "a refused bundle left a Set file behind"
    );
}

/// Verifies that unbundling rejects overwriting existing Set IDs from external sources.
#[test]
fn an_unbundle_refuses_an_id_already_taken_and_leaves_the_set_alone() {
    let (dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let sent = as_a_file(&bundle(&store, "s1").expect("bundle"));

    // Somebody else's store, with a Set of their own under that word: one
    // geometry drawn by two renderers, where the bundle names one.
    let elsewhere = tempfile::tempdir().expect("tempdir");
    let theirs = Store::open(elsewhere.path()).expect("store");
    let l2 = beside(&dir, "theirs.kir", L2);
    let mine = vec![
        Node {
            hash: stored(&theirs, &l1),
            layer: Kind::L1,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&theirs, &l2),
            layer: Kind::L2,
            index: 0,
            name: Some("preset".to_string()),
        },
        Node {
            hash: stored(&theirs, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
    ];
    save(&theirs, Asked::Operator, "s1", plain(&mine, &[])).expect("save");
    let before = written(&theirs, "s1");

    let e = unbundle(&theirs, CameFrom::Somebody, &sent)
        .expect_err("an id that arrived in a file is not typed");
    assert!(e.contains("`s1`"), "the id is not named: {e}");
    assert_eq!(before, written(&theirs, "s1"), "the preset was overwritten");
}

/// Verifies that shipped library updates replace matching IDs while archiving existing versions.
#[test]
fn a_shipped_row_replaces_a_taken_id_and_what_was_there_is_kept() {
    let (dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let sent = as_a_file(&bundle(&store, "s1").expect("bundle"));

    // An `s1` of its own: the same geometry under a second renderer, so the id
    // is taken and the material differs.
    let elsewhere = tempfile::tempdir().expect("tempdir");
    let theirs = Store::open(elsewhere.path()).expect("store");
    let other = beside(&dir, "other.kir", L4);
    save(
        &theirs,
        Asked::Operator,
        "s1",
        plain(&ordinary(&theirs, &l1, &[l4.clone(), other]), &[]),
    )
    .expect("save");
    let before = written(&theirs, "s1");

    let said = unbundle(&theirs, CameFrom::TheShippedLibrary, &sent)
        .expect("the shipped library may replace its own row");

    let held: Vec<String> = theirs
        .list_sets()
        .expect("list")
        .into_iter()
        .map(|entry| entry.id)
        .collect();
    let retired: Vec<&String> = held.iter().filter(|id| *id != "s1").collect();
    assert_eq!(
        retired.len(),
        1,
        "the copy that was replaced was not kept: {held:?}"
    );
    let retired = retired[0];
    assert!(
        retired.starts_with("s1-"),
        "the retired copy is filed as `{retired}`"
    );
    assert!(
        said.contains(retired) && said.contains("kept as"),
        "the operator is not told where it went: `{said}`"
    );

    assert_eq!(
        written(&theirs, retired),
        before.replace("\"id\":\"s1\"", &format!("\"id\":\"{retired}\"")),
        "the retired copy is not the Set that was there, under its new id"
    );
    assert_eq!(
        written(&theirs, "s1"),
        written(&store, "s1"),
        "the plain id does not hold what the library ships"
    );
}

/// Ensures taking in an identical shipped library row avoids redundant writes.
#[test]
fn a_shipped_row_this_store_already_holds_writes_nothing_and_says_so() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let sent = as_a_file(&bundle(&store, "s1").expect("bundle"));

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let theirs = Store::open(elsewhere.path()).expect("store");
    unbundle(&theirs, CameFrom::TheShippedLibrary, &sent).expect("the first take-in");
    let before = written(&theirs, "s1");

    let said = unbundle(&theirs, CameFrom::TheShippedLibrary, &sent)
        .expect("a preset already current is not a refusal");
    assert!(
        said.contains("already what the preset library ships"),
        "what the operator reads is `{said}`"
    );
    assert_eq!(
        before,
        written(&theirs, "s1"),
        "an unchanged preset rewrote the Set file"
    );
    assert_eq!(
        theirs.list_sets().expect("list").len(),
        1,
        "an unchanged preset filed a retired copy of itself"
    );
}

/// Verifies that unbundling retains non-compiling sources and records diagnostic notes.
#[test]
fn an_unbundle_stores_a_source_that_does_not_compile_and_says_so() {
    let broken = "proc veil {\n  kind L4\n  this is not a renderer\n}\n";
    let renderer = Hash::of(broken.as_bytes());
    let geometry = Hash::of(L1.as_bytes());
    let mut lines = vec![
        Line::new(Record::Set {
            id: "sent".to_string(),
            v: VERSION,
        }),
        Line::new(Record::Slot {
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            name: None,
            proc_hash: geometry,
        }),
        Line::new(Record::Slot {
            at: NodeAddress {
                layer: Layer::L4,
                index: 0,
            },
            name: Some("veil".to_string()),
            proc_hash: renderer,
        }),
    ];
    for (hash, src) in [(geometry, L1), (renderer, broken)] {
        for (n, text) in src.split('\n').enumerate() {
            lines.push(Line::new(Record::Src {
                hash,
                line: n as u32,
                s: text.to_string(),
            }));
        }
    }

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let said = unbundle(&bare, CameFrom::Somebody, &as_a_file(&lines))
        .expect("one bad source is not a refusal");

    let report = crate::compile::check(broken).expect_err("the fixture must not compile");
    let first = report.lines().next().expect("a diagnostic").trim();
    assert!(said.contains("veil"), "the node is not named: {said}");
    assert!(
        said.contains(first),
        "the checker's own words are not in the note: {said}"
    );
    bare.get_artifact(&renderer)
        .expect("a source that will not compile is still stored");
    assert!(
        bare.read_meta(&renderer).is_err(),
        "a card was written for a source that never compiled"
    );
    bare.read_meta(&geometry).expect("the good one is carded");
    let filed = bare.read_set("sent").expect("the set is filed");
    assert_eq!(
        filed
            .iter()
            .filter(|line| matches!(line.record(), Record::Slot { .. }))
            .count(),
        2,
        "the node that will not compile lost its slot"
    );
}

// -- The authoring form: resolution, and the wall around it -----------

/// Writes a test .kset file referencing parts by relative path.
fn authored(dir: &tempfile::TempDir, name: &str, parts: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(
        &path,
        format!("{{\"t\":\"set\",\"id\":\"authored\",\"v\":1}}\n{parts}"),
    )
    .expect("write");
    path
}

/// Verifies .kset resolution converts relative part entries into content-addressed slot records.
#[test]
fn a_kset_resolves_to_the_kbset_it_names_with_its_parts_in_the_store() {
    let (dir, store, l1, l4) = fixture();
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
             {\"t\":\"part\",\"layer\":\"L4\",\"name\":\"veil\",\"path\":\"l4.kir\"}\n\
             {\"t\":\"capacity\",\"layer\":\"L1\",\"value\":8192}\n",
    );

    let resolved = resolve(&store, &kset).expect("every part is beside the file");

    let l1_hash = Hash::of(&std::fs::read(&l1).expect("read"));
    let l4_hash = Hash::of(&std::fs::read(&l4).expect("read"));
    let records: Vec<&Record> = resolved.iter().map(Line::record).collect();
    assert!(
        matches!(records[1], Record::Slot { at: NodeAddress { layer: Layer::L1, index: 0 }, name: None, proc_hash } if *proc_hash == l1_hash),
        "the L1 part became a slot naming its source's address: {:?}",
        records[1]
    );
    assert!(
        matches!(records[2], Record::Slot { at: NodeAddress { layer: Layer::L4, .. }, name: Some(name), proc_hash, .. } if name == "veil" && *proc_hash == l4_hash),
        "the L4 part kept the name this Set gave it: {:?}",
        records[2]
    );
    // Unmodified records pass through unchanged.
    assert!(
        matches!(records[0], Record::Set { id, v: 1 } if id == "authored"),
        "{:?}",
        records[0]
    );
    assert!(
        matches!(
            records[3],
            Record::Capacity {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0
                },
                value: 8192
            }
        ),
        "{:?}",
        records[3]
    );
    assert_eq!(records.len(), 4, "no record was added or dropped");

    for hash in [l1_hash, l4_hash] {
        assert!(
            store.get_artifact(&hash).is_ok(),
            "{}: resolution puts the bytes in the store, not only their address",
            hash.short(12)
        );
    }
}

/// Verifies resolved .kbset bundles function independently of authoring files and directories.
#[test]
fn a_kbset_made_from_a_kset_loads_with_the_authoring_file_deleted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let beside_it = dir.path().join("parts");
    std::fs::create_dir(&beside_it).expect("mkdir");
    std::fs::write(beside_it.join("l1.kir"), L1).expect("write l1");
    std::fs::write(beside_it.join("l4.kir"), L4).expect("write l4");
    std::fs::write(
        beside_it.join("night.kset"),
        "{\"t\":\"set\",\"id\":\"night\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
             {\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}\n",
    )
    .expect("write kset");

    let author = Store::open(dir.path().join("store")).expect("store");
    let sent = as_a_file(
        &bundle_authored(&author, &beside_it.join("night.kset"))
            .expect("bundle the authoring file"),
    );

    // The authoring file, the parts, and the store that resolved them: all
    // gone. What is left is the text in `sent`.
    std::fs::remove_dir_all(dir.path()).expect("remove the whole directory");

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    unbundle(&bare, CameFrom::Somebody, &sent).expect("the bundle carries its own sources");
    let loaded = load(&bare, "night").expect("the set is filed and its artifacts are here");
    assert_eq!(loaded.l1s[0].name, "ring");
    assert_eq!(loaded.l4s[0].name, "points");
}

/// Ensures loading rejects unresolved part records present in a .kbset file.
#[test]
fn a_part_in_a_resolved_set_file_refuses_the_load() {
    let (_dir, store, l1, _l4) = fixture();
    let here = stored(&store, &l1);
    let text = format!(
        "{{\"t\":\"set\",\"id\":\"mixed\",\"v\":1}}\n\
             {{\"t\":\"slot\",\"layer\":\"L1\",\"proc\":\"{here}\"}}\n\
             {{\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}}\n"
    );
    let refused = from_lines(&store, "mixed", &parsed(&text)).expect_err("a part is refused");
    assert!(refused.contains("l4.kir"), "{refused}");
    assert!(refused.contains("content address"), "{refused}");
}

/// Ensures .kset resolution rejects absolute part paths.
#[test]
fn a_part_naming_an_absolute_path_is_refused() {
    let (dir, store, _l1, _l4) = fixture();
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"/etc/passwd\"}\n",
    );
    let refused = resolve(&store, &kset).expect_err("an absolute path is not relative");
    assert!(refused.contains("/etc/passwd"), "{refused}");
    assert!(refused.contains("absolute path"), "{refused}");
    assert!(
        refused.contains("Refused rather than repaired"),
        "{refused}"
    );
}

/// Ensures .kset resolution rejects parent-directory traversal (..) escaping the root directory.
#[test]
fn a_part_that_climbs_out_of_the_set_files_directory_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("l1.kir"), L1).expect("write the neighbour above");
    let inside = dir.path().join("inside");
    std::fs::create_dir(&inside).expect("mkdir");
    let kset = inside.join("night.kset");
    std::fs::write(
        &kset,
        "{\"t\":\"set\",\"id\":\"authored\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"../l1.kir\"}\n",
    )
    .expect("write");
    let store = Store::open(dir.path().join("store")).expect("store");

    let refused = resolve(&store, &kset).expect_err("`..` climbs out");
    assert!(refused.contains("../l1.kir"), "{refused}");
    assert!(refused.contains("climbs out of"), "{refused}");
    assert!(
        refused.contains(&inside.display().to_string())
            || refused.contains(
                &std::fs::canonicalize(&inside)
                    .expect("canonicalize")
                    .display()
                    .to_string()
            ),
        "the refusal names the directory that was escaped: {refused}"
    );
}

/// Ensures .kset resolution permits parent-directory segments that remain within the root.
#[test]
fn a_dotdot_that_lands_back_inside_is_a_path_and_is_allowed() {
    let (dir, store, l1, _l4) = fixture();
    std::fs::create_dir(dir.path().join("parts")).expect("mkdir");
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"parts/../l1.kir\"}\n",
    );
    let resolved = resolve(&store, &kset).expect("this path never leaves the directory");
    let l1_hash = Hash::of(&std::fs::read(&l1).expect("read"));
    assert!(
        matches!(resolved[1].record(), Record::Slot { proc_hash, .. } if *proc_hash == l1_hash),
        "{:?}",
        resolved[1].record()
    );
}

/// Ensures .kset resolution rejects symlinks pointing outside the parent directory.
#[test]
#[cfg(unix)]
fn a_part_that_is_a_symlink_out_of_the_directory_is_refused() {
    let elsewhere = tempfile::tempdir().expect("tempdir");
    let secret = elsewhere.path().join("secret.kir");
    std::fs::write(&secret, L1).expect("write the file outside");

    let (dir, store, _l1, _l4) = fixture();
    std::os::unix::fs::symlink(&secret, dir.path().join("innocent.kir")).expect("symlink");
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"innocent.kir\"}\n",
    );

    let refused = resolve(&store, &kset).expect_err("the link points out of the directory");
    assert!(refused.contains("innocent.kir"), "{refused}");
    assert!(refused.contains("symlink out"), "{refused}");
    assert!(
        refused.contains(
            &std::fs::canonicalize(&secret)
                .expect("canonicalize")
                .display()
                .to_string()
        ),
        "the refusal names where the link actually went: {refused}"
    );
    assert!(
        store
            .get_artifact(&Hash::of(&std::fs::read(&secret).expect("read")))
            .is_err(),
        "a refused part is not in the store: the wall runs before anything is read"
    );
}

/// Verifies resolution works correctly when authoring root directories are accessed via symlink.
#[test]
#[cfg(unix)]
fn a_directory_reached_through_a_symlink_still_contains_its_own_parts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let real = dir.path().join("real");
    std::fs::create_dir(&real).expect("mkdir");
    std::fs::write(real.join("l1.kir"), L1).expect("write");
    std::fs::write(
        real.join("night.kset"),
        "{\"t\":\"set\",\"id\":\"authored\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n",
    )
    .expect("write");
    let linked = dir.path().join("linked");
    std::os::unix::fs::symlink(&real, &linked).expect("symlink");
    let store = Store::open(dir.path().join("store")).expect("store");

    resolve(&store, &linked.join("night.kset"))
        .expect("the file's own directory contains the file's own parts, link or no link");
}

/// A file that is not a `.kset` is not resolved, because the extension is the
/// whole of what says which of a Set's two forms a file is.
#[test]
fn a_file_that_is_not_a_kset_is_not_resolved() {
    let (dir, store, _l1, _l4) = fixture();
    let kbset = authored(
        &dir,
        "night.kbset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n",
    );
    let refused = resolve(&store, &kbset).expect_err("a `.kbset` is read, not resolved");
    assert!(refused.contains(".kset"), "{refused}");
}

/// Ensures missing part files produce a dedicated missing file diagnostic rather than traversal error.
#[test]
fn a_part_naming_a_file_that_is_not_there_says_so() {
    let (dir, store, _l1, _l4) = fixture();
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l9.kir\"}\n",
    );
    let refused = resolve(&store, &kset).expect_err("there is no `l9.kir`");
    assert!(refused.contains("l9.kir"), "{refused}");
    assert!(
        refused.contains("no such file beside the Set file"),
        "{refused}"
    );
}
