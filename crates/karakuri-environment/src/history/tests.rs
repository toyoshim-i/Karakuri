use super::*;

fn files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Verifies that declared_kind reads back every layer in LAYERS, including trailing comments.
#[test]
fn declared_kind_reads_back_every_layer_including_l5() {
    for layer in LAYERS {
        let source = format!("proc p {{\n  kind {layer}\n}}\n");
        assert_eq!(
            declared_kind(source.as_bytes()),
            Some(layer),
            "`kind {layer}` has to read back as `{layer}`"
        );
    }
    assert_eq!(
        declared_kind(b"proc feedback {\n  kind L5   // a frame effect\n  retains\n}\n"),
        Some("L5"),
        "a trailing comment is not part of the answer"
    );
    // Layer prefix matching: layer kind names are distinct tokens without mutual prefix overlaps.
    assert_eq!(declared_kind(b"proc p {\n  kind L55\n}\n"), None);
    assert_eq!(declared_kind(b"proc p {\n  kind L6\n}\n"), None);
}

/// The load-bearing claim: what was there before an edit is still readable
/// after it.
#[test]
fn a_snapshot_holds_the_source_it_was_given() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());

    let written = snaps
        .record(0, "L4", 0, None, "soft_points", b"the first version")
        .expect("record")
        .expect("a first version is always new");
    assert_eq!(std::fs::read(&written).expect("read"), b"the first version");

    let second = snaps
        .record(0, "L4", 0, None, "soft_points", b"the second version")
        .expect("record")
        .expect("changed");
    assert_ne!(written, second, "the second snapshot overwrote the first");
    assert_eq!(std::fs::read(&written).expect("read"), b"the first version");
}

/// A rebuild recompiles both layers whichever one was saved. Without this the
/// untouched layer's chain is a row of identical files, and the one question a
/// history has to answer — what changed — is the one it stops being able to
/// answer.
#[test]
fn an_unchanged_source_is_not_written_again() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());

    assert!(snaps
        .record(0, "L1", 0, None, "field", b"same")
        .expect("record")
        .is_some());
    assert!(
        snaps
            .record(0, "L1", 0, None, "field", b"same")
            .expect("record")
            .is_none(),
        "an identical source was written a second time"
    );
    assert_eq!(files(tmp.path()).len(), 1);
}

/// And "unchanged" is per slot and per layer, not global — two slots holding
/// the same source are two chains, and an edit to one must not be mistaken for
/// the other having already been recorded.
#[test]
fn each_slot_and_layer_has_its_own_chain() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());

    assert!(snaps
        .record(0, "L1", 0, None, "field", b"same")
        .expect("record")
        .is_some());
    assert!(
        snaps
            .record(1, "L1", 0, None, "field", b"same")
            .expect("record")
            .is_some(),
        "slot 1's first snapshot was skipped because slot 0 had the same source"
    );
    assert!(
        snaps
            .record(0, "L4", 0, None, "field", b"same")
            .expect("record")
            .is_some(),
        "the L4 chain was skipped because the L1 chain had the same source"
    );
    assert_eq!(files(tmp.path()).len(), 3);
}

/// Verifies that multiple renderers within the same layer maintain distinct snapshot chains.
#[test]
fn each_renderer_of_a_stack_has_its_own_chain() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());

    assert!(snaps
        .record(0, "L4", 0, None, "sprites", b"first")
        .expect("record")
        .is_some());
    assert!(
        snaps
            .record(0, "L4", 1, None, "strokes", b"second")
            .expect("record")
            .is_some(),
        "the second renderer's first snapshot was skipped as the first renderer's"
    );
    assert!(
        snaps
            .record(0, "L4", 1, None, "strokes", b"second")
            .expect("record")
            .is_none(),
        "the second renderer's unchanged source was written again"
    );
    assert!(
        snaps
            .record(0, "L4", 0, None, "sprites", b"first")
            .expect("record")
            .is_none(),
        "the first renderer looked changed because the second had written since"
    );
    assert_eq!(files(tmp.path()).len(), 2);
}

/// A renderer's index is in the name only when it is not the first, so every
/// file a one-renderer run has ever written keeps the name it had. A history is
/// read by a person looking for what they changed, and a `_0` on every L4 of
/// every ordinary run is noise in the way of that.
#[test]
fn only_a_renderer_past_the_first_carries_its_index_in_the_name() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());

    let first = snaps
        .record(0, "L4", 0, None, "sprites", b"a")
        .expect("record")
        .expect("written");
    let second = snaps
        .record(0, "L4", 1, None, "strokes", b"b")
        .expect("record")
        .expect("written");
    let name = |p: &std::path::Path| p.file_name().expect("named").to_string_lossy().to_string();

    assert!(
        name(&first).contains("_L4_"),
        "the first renderer grew an index: {}",
        name(&first)
    );
    assert!(
        name(&second).contains("_L41_"),
        "the second renderer is not distinguishable from the first: {}",
        name(&second)
    );
}

/// The name has to say which slot and which layer it came from, or a directory
/// of a night's work is unreadable.
#[test]
fn the_name_carries_the_day_the_slot_the_layer_and_the_procedure() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    let path = snaps
        .record(2, "L4", 0, None, "beat_strokes", b"x")
        .expect("record")
        .expect("new");

    let name = path
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .to_string();
    assert!(name.contains("slot2"), "{name}");
    assert!(name.contains("L4"), "{name}");
    assert!(name.contains("beat_strokes"), "{name}");
    assert!(name.ends_with(".kir"), "{name}");

    // Nested a directory per day, which is what makes `rm -rf` the cleanup.
    let rel = path
        .strip_prefix(tmp.path().join(DIR))
        .expect("under the history root");
    assert_eq!(
        rel.components().count(),
        4,
        "expected YYYY/MM/DD/name, got {rel:?}"
    );

    // And the day is the operator's day, not UTC's — the whole reason the
    // dependency is here.
    let today = chrono::Local::now().format("%Y/%m/%d").to_string();
    assert!(
        path.to_string_lossy().contains(&today),
        "{} is not under today's local date {today}",
        path.display()
    );
}

/// What makes the first edit undoable. Without the seed, the first rebuild
/// records the version that replaced the original and the original is nowhere.
#[test]
fn seeding_writes_the_version_a_run_starts_with() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let l1 = tmp.path().join("a.kir");
    let l4 = tmp.path().join("b.kir");
    std::fs::write(&l1, "proc field_one {\n  kind L1\n}").expect("write");
    std::fs::write(&l4, "proc draw_one {\n  kind L4\n}").expect("write");

    let shared = Snapshots::shared(tmp.path());
    seed(
        &shared,
        std::iter::once((0, None, vec![l1.as_path(), l4.as_path()])),
    );

    let names: Vec<String> = files(&tmp.path().join(DIR))
        .iter()
        .map(|p| p.file_name().expect("name").to_string_lossy().to_string())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names.iter().any(|n| n.contains("field_one")), "{names:?}");
    assert!(names.iter().any(|n| n.contains("draw_one")), "{names:?}");
}

/// Verifies that initial seed snapshots preserve declared layer kinds rather than defaulting to L4.
#[test]
fn a_seeded_chain_is_filed_under_the_layer_its_file_declares() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, body: &str| {
        let path = tmp.path().join(name);
        std::fs::write(&path, body).expect("write");
        path
    };
    let l1 = write("a.kir", "proc gen {\n  kind L1\n}");
    let l2 = write("b.kir", "proc warp {\n  kind L2\n}");
    let fld = write("c.kir", "proc blob {\n  kind Field\n}");
    let near = write("d.kir", "proc near {\n  kind L4\n}");
    let far = write("e.kir", "proc far {\n  kind L4\n}");

    let shared = Snapshots::shared(tmp.path());
    seed(
        &shared,
        std::iter::once((
            0,
            None,
            vec![
                l1.as_path(),
                l2.as_path(),
                fld.as_path(),
                near.as_path(),
                far.as_path(),
            ],
        )),
    );

    let names: Vec<String> = files(&tmp.path().join(DIR))
        .iter()
        .map(|p| p.file_name().expect("name").to_string_lossy().to_string())
        .collect();
    let has = |part: &str| names.iter().any(|n| n.contains(part));
    assert_eq!(names.len(), 5, "{names:?}");
    assert!(has("L1_gen"), "{names:?}");
    assert!(has("L2_warp"), "{names:?}");
    assert!(has("Field_blob"), "{names:?}");
    // And the index counts within a layer, so the second renderer is the
    // one that carries a number — not the second file.
    assert!(has("L4_near"), "{names:?}");
    assert!(has("L41_far"), "{names:?}");
}

/// And the seed shares the dedup with the watcher, or the first rebuild writes
/// the untouched procedure all over again — which is the duplicate this
/// arrangement exists to avoid.
#[test]
fn a_rebuild_after_seeding_does_not_rewrite_an_untouched_procedure() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let l1 = tmp.path().join("a.kir");
    let l4 = tmp.path().join("b.kir");
    std::fs::write(&l1, "proc field_one {}").expect("write");
    std::fs::write(&l4, "proc draw_one {}").expect("write");

    let shared = Snapshots::shared(tmp.path());
    seed(
        &shared,
        std::iter::once((0, None, vec![l1.as_path(), l4.as_path()])),
    );

    let mut snapshots = shared.lock().expect("lock");
    assert!(
        snapshots
            .record(0, "L1", 0, None, "field_one", b"proc field_one {}")
            .expect("record")
            .is_none(),
        "the untouched L1 was written a second time"
    );
    assert!(
        snapshots
            .record(0, "L4", 0, None, "draw_one", b"proc draw_one { edited }")
            .expect("record")
            .is_some(),
        "the edited L4 was skipped"
    );
}

/// Verifies that sub-millisecond saves receive unique disambiguated identifiers via `unused`.
#[test]
fn two_ids_taken_off_one_millisecond_are_two_ids() {
    let mut issued = std::collections::HashSet::new();
    let stamp = "20260816-143052-271".to_string();
    let ids: Vec<String> = (0..3).map(|_| unused(&mut issued, stamp.clone())).collect();

    assert_eq!(
        ids,
        vec![
            "20260816-143052-271",
            "20260816-143052-271-1",
            "20260816-143052-271-2"
        ],
        "a second save in the same millisecond was handed the first one's \
             id, so its Set file wrote over a file the operator was told had \
             been kept"
    );

    // Control test: distinct millisecond timestamps require no discriminator index.
    assert_eq!(
        unused(&mut issued, "20260816-143052-272".to_string()),
        "20260816-143052-272"
    );
}

/// A procedure name reaches this from a file somebody else wrote. It is an
/// identifier by the time it gets here, and a path component is not where to
/// discover that the parser and this module disagree about that.
#[test]
fn a_procedure_name_cannot_escape_the_history_directory() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    let path = snaps
        .record(0, "L1", 0, None, "../../etc/passwd", b"x")
        .expect("record")
        .expect("new");

    assert!(
        path.starts_with(tmp.path().join(DIR)),
        "{} escaped the history root",
        path.display()
    );
    assert!(!path.to_string_lossy().contains(".."), "{}", path.display());
}

// Test fixtures constructed via `Snapshots::record` to ensure consistency with runtime formatting.

/// Helper copying a generated snapshot into an alternate date directory to test multi-day walks.
fn on(day: &str, store: &Path, path: &Path) -> PathBuf {
    let dir = store.join(DIR).join(day);
    std::fs::create_dir_all(&dir).expect("the day directory");
    let moved = dir.join(path.file_name().expect("a name"));
    std::fs::rename(path, &moved).expect("move");
    moved
}

/// Verifies that querying an uninitialized or empty history directory succeeds with zero versions.
#[test]
fn a_store_that_has_never_been_edited_lists_no_versions() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let listing = list(tmp.path(), 100).expect("a store with no history is not a failure");
    assert_eq!(listing, Listing::default());
}

/// Verifies round-trip fidelity of node addresses and snapshot metadata parsed from file names.
#[test]
fn a_row_reads_back_the_address_the_snapshot_was_recorded_under() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    for (slot, layer, index, name) in [
        (2, "L4", 0, "beat_strokes"),
        (0, "L4", 10, "far"),
        (1, "Field", 0, "blob"),
    ] {
        snaps
            .record(slot, layer, index, None, name, name.as_bytes())
            .expect("record")
            .expect("new");
    }

    let listing = list(tmp.path(), 100).expect("listed");
    assert_eq!(listing.versions.len(), 3, "{listing:?}");
    assert_eq!(listing.unclaimed, 0, "{listing:?}");
    assert!(!listing.stopped_short, "{listing:?}");

    let found = |name: &str| {
        listing
            .versions
            .iter()
            .find(|v| v.proc_name == name)
            .unwrap_or_else(|| panic!("no row for {name}: {listing:?}"))
    };
    let address = |v: &Version| (v.slot, v.layer, v.index);
    assert_eq!(address(found("beat_strokes")), (2, "L4", 0));
    assert_eq!(
        address(found("far")),
        (0, "L4", 10),
        "`L410` did not read back as the eleventh renderer of L4"
    );
    assert_eq!(address(found("blob")), (1, "Field", 0));
    // Recorded with no Set, so read back with none — and not with a word
    // standing in for one, which is what the `Option` is for.
    assert!(
        listing.versions.iter().all(|v| v.set.is_none()),
        "a version written with no Set came back filed under one: {listing:?}"
    );

    // And the row points at the snapshot rather than describing it.
    assert_eq!(
        std::fs::read(&found("blob").file).expect("read"),
        b"blob",
        "the row's file is not the one the snapshot went into"
    );
    // `at` is `stamped_id`'s spelling: eight digits of date, then `TIME`.
    let at = &found("blob").at;
    assert_eq!(at.len(), 19, "{at}");
    assert!(
        at.starts_with(&chrono::Local::now().format("%Y%m%d").to_string()),
        "{at} is not today's local date"
    );
}

/// Verifies that `Version::filed_as` matches the canonical base filename format.
#[test]
fn a_rows_name_is_the_snapshots_own_name_without_the_suffix_or_the_set() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    for (index, set, name) in [
        (0, Some("night01"), "beat_strokes"),
        (10, Some("night01"), "far"),
        (0, None, "nobodys"),
    ] {
        snaps
            .record(1, "L4", index, set, name, name.as_bytes())
            .expect("record")
            .expect("new");
    }

    let listing = list(tmp.path(), 100).expect("listed");
    assert_eq!(listing.versions.len(), 3, "{listing:?}");
    for version in &listing.versions {
        let on_disk = version
            .file
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a snapshot's name");
        let tail = match &version.set {
            Some(set) => format!("@{set}.kir"),
            None => ".kir".to_string(),
        };
        // Date is determined by day directory path rather than filename.
        let row = version.filed_as();
        let (date, rest) = row.split_at(9);
        let date = &date[..8];
        assert_eq!(
            format!("{rest}{tail}"),
            on_disk,
            "the row a surface hands back is not the name the store filed it under"
        );
        assert!(
            version
                .file
                .ancestors()
                .any(|dir| dir
                    .strip_prefix(tmp.path().join(DIR))
                    .is_ok_and(|under| under
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("")
                        == date)),
            "the row's date is not the day directory the file is in: {row}"
        );
    }
    let far = listing
        .versions
        .iter()
        .find(|version| version.proc_name == "far")
        .expect("the eleventh renderer's row");
    assert!(
        far.filed_as().ends_with("_L410_far"),
        "an index past the first is not in the row: {}",
        far.filed_as()
    );
}

/// Most recent first, and across the day directories rather than within one. A
/// walk starts at the newest version, and the newest version is in yesterday's
/// directory as often as in today's.
#[test]
fn versions_are_listed_most_recent_first_across_two_days() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    let write = |snaps: &mut Snapshots, name: &str| {
        snaps
            .record(0, "L4", 0, None, name, name.as_bytes())
            .expect("record")
            .expect("new")
    };
    // One chain, four versions of it — which is what a walk walks.
    let first = write(&mut snaps, "v1");
    let second = write(&mut snaps, "v2");
    write(&mut snaps, "v3");
    write(&mut snaps, "v4");
    on("2000/01/02", tmp.path(), &first);
    on("2000/01/02", tmp.path(), &second);

    let listing = list(tmp.path(), 100).expect("listed");
    let ats: Vec<&str> = listing.versions.iter().map(|v| v.at.as_str()).collect();
    assert_eq!(ats.len(), 4, "{listing:?}");
    // Order is non-ascending by timestamp, with filename breaking sub-millisecond ties.
    assert!(
        ats.windows(2).all(|pair| pair[0] >= pair[1]),
        "not most recent first: {ats:?}"
    );
    assert!(
        listing
            .versions
            .windows(2)
            .all(|pair| pair[0].at > pair[1].at || pair[0].file < pair[1].file),
        "a tie was left to `read_dir`, which is not an order: {listing:?}"
    );
    let today = chrono::Local::now().format("%Y%m%d").to_string();
    assert!(
        ats[0].starts_with(&today) && ats[1].starts_with(&today),
        "today's versions are not at the top: {ats:?}"
    );
    assert!(
        ats[2].starts_with("20000102") && ats[3].starts_with("20000102"),
        "the older day is not at the bottom: {ats:?}"
    );
}

/// Verifies that unrecognized files and non-directory entries are skipped and tallied in `unclaimed`.
#[test]
fn a_name_the_layout_does_not_claim_is_counted_and_not_listed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    let kept = snaps
        .record(0, "L4", 0, None, "sprites", b"kept")
        .expect("record")
        .expect("new");
    let root = tmp.path().join(DIR);
    let day = kept.parent().expect("a day directory").to_path_buf();

    // Two at the top: a note somebody left, and a file with a year's name
    // on it — which is claimed by name and refused by type.
    std::fs::write(root.join("README"), "mine").expect("write");
    std::fs::write(root.join("1999"), "not a year").expect("write");
    // And six in the day directory: a note, an editor's leftover, a
    // directory, a directory named exactly like a snapshot, a slot that is
    // not a number, and a layer this module does not spell.
    std::fs::write(day.join("notes.txt"), "mine").expect("write");
    std::fs::write(day.join("120000-000_slot0_L4_x.kir.tmp"), "half").expect("write");
    std::fs::create_dir(day.join("scratch")).expect("mkdir");
    std::fs::create_dir(day.join("120000-000_slot0_L4_x.kir")).expect("mkdir");
    std::fs::write(day.join("120000-000_slotX_L4_x.kir"), "x").expect("write");
    std::fs::write(day.join("120000-000_slot0_L9_x.kir"), "x").expect("write");
    // Verify multi-byte UTF-8 sequences do not trigger slicing panics during timestamp validation.
    std::fs::write(day.join("123456é90_slot0_L4_x.kir"), "x").expect("write");
    // Reject invalid Set suffix characters not conforming to sanitize rules.
    std::fs::write(day.join("120000-000_slot0_L4_x@a.b.kir"), "x").expect("write");

    let listing = list(tmp.path(), 100).expect("listed");
    assert_eq!(
        listing.versions.len(),
        1,
        "something that is not a snapshot was listed as one: {listing:?}"
    );
    assert_eq!(listing.versions[0].file, kept);
    assert_eq!(
        listing.unclaimed, 10,
        "what was passed over was not reported: {listing:?}"
    );
}

/// Verifies that Set identifiers are preserved and parsed round-trip through snapshot filenames.
#[test]
fn a_version_written_under_a_set_reads_back_under_it() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    let written = snaps
        .record(0, "L4", 0, Some("star_vortex"), "beat_strokes", b"x")
        .expect("record")
        .expect("new");

    let name = written
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .to_string();
    assert!(
        name.ends_with("_beat_strokes@star_vortex.kir"),
        "the Set is not in the name: {name}"
    );

    let listing = list(tmp.path(), 100).expect("listed");
    assert_eq!(listing.versions.len(), 1, "{listing:?}");
    let row = &listing.versions[0];
    assert_eq!(
        row.set.as_deref(),
        Some("star_vortex"),
        "the row does not say which Set this was a version of: {listing:?}"
    );
    // And the rest of the address survived the new field rather than being
    // eaten by it — `@` is the separator and `_` is still the other one.
    assert_eq!(
        (row.slot, row.layer, row.index, row.proc_name.as_str()),
        (0, "L4", 0, "beat_strokes"),
        "{listing:?}"
    );
    assert_eq!(row.file, written);
}

/// Verifies that loading distinct Sets maintains separate snapshot chains even for identical source code.
#[test]
fn two_sets_on_one_slot_are_two_chains() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    assert!(snaps
        .record(0, "L4", 0, Some("drift_cloud"), "shell", b"same")
        .expect("record")
        .is_some());
    assert!(
        snaps
            .record(0, "L4", 0, Some("star_vortex"), "shell", b"same")
            .expect("record")
            .is_some(),
        "the Set loaded onto this slot had its first version skipped as the \
             Set before it having already written those bytes"
    );
    // And the dedup is not lost, only re-keyed: the same Set writing the
    // same bytes again is still one row.
    assert!(
        snaps
            .record(0, "L4", 0, Some("star_vortex"), "shell", b"same")
            .expect("record")
            .is_none(),
        "an unchanged source was written again once the key grew a field"
    );
    // Dedup against previous source per chain when switching back.
    assert!(
        snaps
            .record(0, "L4", 0, Some("drift_cloud"), "shell", b"same")
            .expect("record")
            .is_none(),
        "coming back to a Set wrote its unchanged source a second time"
    );

    let listing = list(tmp.path(), 100).expect("listed");
    assert_eq!(listing.versions.len(), 2, "{listing:?}");
    let mut sets: Vec<&str> = listing
        .versions
        .iter()
        .map(|v| v.set.as_deref().unwrap_or("<none>"))
        .collect();
    sets.sort();
    assert_eq!(
        sets,
        vec!["drift_cloud", "star_vortex"],
        "the two sides of a load are not distinguishable: {listing:?}"
    );
    // Same node, same source, same millisecond — the Set is the only thing
    // telling these two rows apart, which is the point.
    assert_ne!(listing.versions[0].file, listing.versions[1].file);
}

/// Verifies that runs without Sets record versions with None and are not retroactively renamed.
#[test]
fn a_run_with_no_set_files_under_none_and_a_later_id_does_not_reach_back() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    let before = snaps
        .record(0, "L4", 0, None, "flares", b"before")
        .expect("record")
        .expect("new");
    let after = snaps
        .record(0, "L4", 0, Some("kept_take"), "flares", b"after")
        .expect("record")
        .expect("new");

    // The name a run with no Set writes is the name it has always written,
    // which is what keeps every version recorded before there was a field
    // readable.
    assert!(
        !before
            .file_name()
            .expect("named")
            .to_string_lossy()
            .contains('@'),
        "a version with no Set was filed under one: {}",
        before.display()
    );

    let listing = list(tmp.path(), 100).expect("listed");
    let row = |file: &Path| {
        listing
            .versions
            .iter()
            .find(|v| v.file == file)
            .unwrap_or_else(|| panic!("no row for {}: {listing:?}", file.display()))
    };
    assert_eq!(
        row(&before).set,
        None,
        "the earlier version was re-filed under the Set a later save made: {listing:?}"
    );
    assert_eq!(row(&after).set.as_deref(), Some("kept_take"), "{listing:?}");
    // The bytes each row points at are still each row's, which is what
    // says the two are versions and not one row rewritten.
    assert_eq!(std::fs::read(&before).expect("read"), b"before");
    assert_eq!(std::fs::read(&after).expect("read"), b"after");
}

/// A Set id reaches this from a name an operator typed, and `mcp`'s
/// `checked_id` is not on every route in. A path component is not where to find
/// out that the two disagree about what an id is.
#[test]
fn a_set_id_cannot_escape_the_history_directory() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    let path = snaps
        .record(0, "L1", 0, Some("../../etc"), "field", b"x")
        .expect("record")
        .expect("new");

    assert!(
        path.starts_with(tmp.path().join(DIR)),
        "{} escaped the history root",
        path.display()
    );
    assert!(!path.to_string_lossy().contains(".."), "{}", path.display());
}

/// Verifies that history scanning respects the day limit cap and marks truncated listings.
/// See Principle 0091.
#[test]
fn the_cap_bounds_the_walk_and_a_short_listing_says_so() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut snaps = Snapshots::new(tmp.path());
    for name in ["v1", "v2", "v3"] {
        snaps
            .record(0, "L4", 0, None, name, name.as_bytes())
            .expect("record")
            .expect("new");
    }
    let day = tmp
        .path()
        .join(DIR)
        .join(chrono::Local::now().format("%Y/%m/%d").to_string());
    std::fs::write(day.join("notes.txt"), "mine").expect("write");

    let whole = list(tmp.path(), 3).expect("listed");
    assert_eq!(whole.versions.len(), 3, "{whole:?}");
    assert!(
        !whole.stopped_short,
        "a listing that reached the end said it had not: {whole:?}"
    );
    assert_eq!(whole.unclaimed, 1, "{whole:?}");

    let cut = list(tmp.path(), 2).expect("listed");
    assert_eq!(cut.versions.len(), 2, "{cut:?}");
    assert!(
        cut.stopped_short,
        "a truncated listing reads as the whole history: {cut:?}"
    );
    assert_eq!(
        cut.versions,
        whole.versions[..2],
        "the cap kept the wrong end — a walk starts at the newest"
    );

    let none = list(tmp.path(), 0).expect("listed");
    assert!(none.versions.is_empty(), "{none:?}");
    assert!(none.stopped_short, "{none:?}");
    assert_eq!(
        none.unclaimed, 0,
        "the day directory was read for a listing that asked for nothing: {none:?}"
    );
}
