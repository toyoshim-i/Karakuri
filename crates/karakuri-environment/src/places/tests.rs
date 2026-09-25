use super::*;

/// Creates a test library directory containing parts and a set file.
fn library(path: PathBuf) -> PathBuf {
    parts(path.clone());
    set(&path, "drift_cloud");
    path
}

/// A directory of parts and no listing — `.kir` files, which is every preset
/// root there was before today and is no longer a library.
fn parts(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).expect("mkdir");
    std::fs::write(path.join("drift_shell.kir"), "kind L1\n").expect("write");
    path
}

/// Verifies that list_procedures enumerates valid `.kir` files with their declared kind.
#[test]
fn a_presets_root_lists_its_procedures_with_the_kind_each_declares() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = library(tmp.path().join("shipped"));
    std::fs::write(
        dir.join("orbit_wide.kir"),
        "proc orbit_wide {\n  kind L3\n}\n",
    )
    .expect("write");
    std::fs::write(dir.join("late_bloom.kir"), "  kind  L2\n").expect("write");
    std::fs::write(dir.join("no_kind.kir"), "// nothing declared\n").expect("write");
    std::fs::write(dir.join("orbit_wide.kir~"), "kind L3\n").expect("write");
    std::fs::write(dir.join("half.kir.tmp"), "kind L3\n").expect("write");
    std::fs::create_dir(dir.join("old.kir")).expect("mkdir");

    let listed = root(dir).list_procedures().expect("the root is there");
    let names: Vec<&str> = listed.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(
        names,
        ["drift_shell", "late_bloom", "no_kind", "orbit_wide"],
        "the listing is {names:?}"
    );
    let kinds: Vec<Option<&str>> = listed.iter().map(|row| row.kind).collect();
    assert_eq!(kinds, [Some("L1"), Some("L2"), None, Some("L3")]);
    assert!(
        listed[3].file.ends_with("orbit_wide.kir"),
        "the row does not carry the file a load reads: {:?}",
        listed[3].file
    );

    // A root that has gone since it was resolved is an error, not an empty
    // listing — `list_sets`' own answer one extension along.
    let gone = root(tmp.path().join("nowhere"));
    assert!(gone.list_procedures().is_err());
}

/// One authoring Set file named `id`, with the two records a real one opens
/// with.
fn set(dir: &Path, id: &str) -> PathBuf {
    let file = dir.join(format!("{id}{AUTHORING_SUFFIX}"));
    std::fs::write(
        &file,
        format!(
            "{{\"t\":\"set\",\"id\":\"{id}\",\"v\":1}}\n\
                 {{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"drift_shell.kir\"}}\n"
        ),
    )
    .expect("write");
    file
}

/// A root as [`presets`] would have answered with one, so the listing is asked
/// of the thing the program actually holds.
fn root(dir: PathBuf) -> Presets {
    Presets {
        dir,
        found: Found::Given,
    }
}

/// A path an operator typed is the answer, and nothing else is consulted.
#[test]
fn a_typed_presets_path_that_exists_is_the_root_and_says_it_was_given() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = library(tmp.path().join("shipped"));

    let found = presets(Some(&dir))
        .expect("a directory that is there is not a refusal")
        .expect("a directory that is there is a library");
    assert_eq!(found.dir, dir);
    assert_eq!(
        found.found,
        Found::Given,
        "a typed path was reported as something the program went looking for"
    );

    // And it is theirs even with nothing in it: an empty library an
    // operator named is not a reason to go and play something else.
    let empty = tmp.path().join("empty");
    std::fs::create_dir_all(&empty).expect("mkdir");
    assert_eq!(
        presets(Some(&empty)).expect("an empty directory is not a refusal"),
        Some(Presets {
            dir: empty,
            found: Found::Given
        })
    );
}

/// A typed path that is not there is refused by name, rather than searched past
/// — which would play material the operator did not name and say nothing about
/// it (P-0094).
#[test]
fn a_typed_presets_path_that_is_not_there_is_refused_naming_the_flag() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("nowhere");

    let why = presets(Some(&missing)).expect_err(
        "a `--presets` that is not there was accepted, so the search would have quietly \
             answered with somewhere else",
    );
    assert_eq!(why, no_presets_at(&missing));
    assert!(
        why.contains("--presets"),
        "the refusal `{why}` does not name the flag the operator would have to fix"
    );
    assert!(
        why.contains(&missing.display().to_string()),
        "the refusal `{why}` does not name the path it refused"
    );
    assert!(
        !missing.exists(),
        "the refusal created the directory it was complaining about"
    );
}

/// Each of the four places answers when it is the one that has a library, and
/// says which it was.
#[test]
fn each_candidate_answers_in_its_own_turn_and_names_itself() {
    // `<exe dir>` is `<root>/bin`, so `..` is a real directory in every
    // one of these rather than a path that only normalises to one.
    for (relative, expected) in [
        ("Resources/examples", Found::Bundle),
        ("share/karakuri/examples", Found::Prefix),
        ("bin/examples", Found::Beside),
    ] {
        let tmp = tempfile::tempdir().expect("tempdir");
        let exe_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).expect("mkdir");
        let dir = library(tmp.path().join(relative));

        let found = searched(Some(&exe_dir), &tmp.path().join("no-workspace"))
            .unwrap_or_else(|| panic!("a library at {relative} was not found"));
        assert_eq!(
            found.found, expected,
            "a library at {relative} was reported as {:?}",
            found.found
        );
        assert!(
            std::fs::canonicalize(&found.dir).expect("canonicalize")
                == std::fs::canonicalize(&dir).expect("canonicalize"),
            "{relative} answered with {}",
            found.dir.display()
        );
    }

    // The development entry, which is the one that is not a function of
    // where the binary is — and it answers with no binary location at all,
    // which is the platform that will not say where it is.
    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path().join("tree");
    library(workspace.join("examples"));
    let found = searched(None, &workspace).expect("the workspace tree holds a library");
    assert_eq!(found.found, Found::Workspace);
}

/// Order decides, and it is the order of the table: a machine that has two of
/// these has the earlier one.
#[test]
fn an_earlier_candidate_wins_over_every_later_one() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let exe_dir = tmp.path().join("bin");
    std::fs::create_dir_all(&exe_dir).expect("mkdir");
    let workspace = tmp.path().join("tree");
    library(workspace.join("examples"));
    library(tmp.path().join("bin/examples"));
    library(tmp.path().join("share/karakuri/examples"));
    library(tmp.path().join("Resources/examples"));

    // All four are libraries. Peel them off in order and the next one down
    // answers, which checks the order rather than only its first entry.
    for expected in [
        Found::Bundle,
        Found::Prefix,
        Found::Beside,
        Found::Workspace,
    ] {
        let found = searched(Some(&exe_dir), &workspace).expect("one of the four holds a library");
        assert_eq!(
            found.found,
            expected,
            "{} answered ahead of its turn",
            found.dir.display()
        );
        std::fs::remove_dir_all(&found.dir).expect("rmdir");
    }
}

/// A directory called `examples` is not a library by its name, and this is the
/// case that is not hypothetical: `cargo` builds example binaries into
/// `<target>/<profile>/examples`, which is exactly where the portable candidate
/// looks for a `cargo run`.
#[test]
fn a_directory_of_something_else_is_not_a_library_however_it_is_named() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let exe_dir = tmp.path().join("target/debug");
    std::fs::create_dir_all(exe_dir.join("examples")).expect("mkdir");
    std::fs::write(exe_dir.join("examples/panel-36562087879dc46c"), "elf").expect("write");
    let workspace = tmp.path().join("tree");
    library(workspace.join("examples"));

    let found = searched(Some(&exe_dir), &workspace).expect("the workspace tree holds one");
    assert_eq!(
        found.found,
        Found::Workspace,
        "cargo's own `examples` directory answered as a preset library, and the pair \
             this program opens on is not in it"
    );
}

/// Verifies that a directory containing only `.kir` part files is not recognized as a preset library.
#[test]
fn a_directory_of_parts_is_not_a_library_because_a_library_lists_what_goes_on_a_deck() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let exe_dir = tmp.path().join("bin");
    std::fs::create_dir_all(&exe_dir).expect("mkdir");
    parts(exe_dir.join("examples"));
    let workspace = tmp.path().join("tree");
    library(workspace.join("examples"));

    let found = searched(Some(&exe_dir), &workspace).expect("the workspace tree holds one");
    assert_eq!(
        found.found,
        Found::Workspace,
        "a directory holding only parts answered as a preset library, and there is \
             nothing in it the Library bay can draw a row for"
    );
}

/// Nothing found is a value, and the sentence about it names the flag.
#[test]
fn no_library_anywhere_is_an_answer_rather_than_an_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let exe_dir = tmp.path().join("bin");
    std::fs::create_dir_all(&exe_dir).expect("mkdir");

    assert_eq!(
        searched(Some(&exe_dir), &tmp.path().join("no-workspace")),
        None,
        "a machine with no preset library reported one"
    );
    assert!(
        no_preset_library().contains("--presets"),
        "the sentence about having no library does not say how to name one"
    );
    assert!(
        no_launch_pair().contains("--presets"),
        "the refusal for having nothing to play does not say how to name a library"
    );
}

/// A root's Sets are listed under the id a store would give them, with the file
/// beside each one, in ascending id order whatever order the filesystem hands
/// them back in.
#[test]
fn a_root_of_set_files_lists_them_by_id_with_the_file_to_take_in() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = parts(tmp.path().join("examples"));
    // Written out of alphabetical order, so a listing that passed
    // `read_dir`'s order through would have to be lucky to pass.
    for id in ["glow_lattice", "beat_cloud", "drift_cloud"] {
        set(&dir, id);
    }

    let listed = root(dir.clone()).list_sets().expect("the root is there");
    assert_eq!(
        listed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        ["beat_cloud", "drift_cloud", "glow_lattice"],
        "the listing is not the ids in ascending order"
    );
    for entry in &listed {
        assert_eq!(
            entry.file,
            dir.join(format!("{}{AUTHORING_SUFFIX}", entry.id)),
            "the file beside `{}` is not the one the id names",
            entry.id
        );
        assert!(
            entry.file.is_file(),
            "the listing named `{}`, which is not a file anything can take in",
            entry.file.display()
        );
    }
}

/// A root of parts holds no Sets, and that is a value rather than a failure —
/// an operator may type `--presets` at a directory of `.kir` files, and the
/// answer is an empty library rather than an error about it.
#[test]
fn a_root_of_parts_lists_nothing_and_is_not_a_refusal() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = parts(tmp.path().join("examples"));

    assert_eq!(
        root(dir)
            .list_sets()
            .expect("a readable directory is not a refusal"),
        Vec::new(),
        "a directory of parts reported Sets in it"
    );
}

/// A name the layout does not claim is skipped rather than repaired: each of
/// these would become a row nothing can open if its id were guessed at, and
/// every one of them is a file somebody else put there.
#[test]
fn a_name_the_layout_does_not_claim_is_skipped_rather_than_repaired() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = parts(tmp.path().join("examples"));
    set(&dir, "beat_cloud");
    for name in [
        "notes.txt",               // not a Set by any reading
        "beat_glow.kbset",         // the resolved form: a store's, not a root's
        "beat_glow.kset~",         // an editor's backup
        "half_written.kset.tmp",   // a copy that died
        "GLASS_CLOUD.KSET",        // `setfile::resolve` refuses this spelling
        "beat_glow.kset.disabled", // switched off by rename
    ] {
        std::fs::write(dir.join(name), "{}\n").expect("write");
    }

    let listed = root(dir).list_sets().expect("the root is there");
    assert_eq!(
        listed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        ["beat_cloud"],
        "a name the layout does not claim was listed as a Set"
    );
}

/// A subdirectory named like a Set file is not one, and this is the case a
/// checked-out bundle actually produces: a directory somebody unpacked beside
/// the file it came from.
#[test]
fn a_subdirectory_that_looks_like_a_set_file_is_not_one() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = parts(tmp.path().join("examples"));
    set(&dir, "beat_cloud");
    std::fs::create_dir_all(dir.join(format!("unpacked{AUTHORING_SUFFIX}"))).expect("mkdir");
    // And one under it, so a listing that walked would find it.
    set(&dir.join(format!("unpacked{AUTHORING_SUFFIX}")), "buried");

    let listed = root(dir).list_sets().expect("the root is there");
    assert_eq!(
        listed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        ["beat_cloud"],
        "the listing walked into a subdirectory, or reported one as a Set"
    );
}

/// A root that has gone since it was resolved is refused rather than reported
/// empty — the difference between *this library holds nothing* and *this
/// library is not there* is the whole of what the operator has to act on, and
/// only the second one is fixable.
#[test]
fn a_root_that_has_gone_since_it_was_resolved_is_refused_rather_than_reported_empty() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = library(tmp.path().join("examples"));
    let presets = root(dir.clone());
    std::fs::remove_dir_all(&dir).expect("rmdir");

    let why = presets.list_sets().expect_err(
        "a presets root that is no longer there listed as an empty library, which says \
             it holds nothing to somebody whose disk it is no longer on",
    );
    assert!(
        why.contains(&dir.display().to_string()),
        "the refusal `{why}` does not name the root it could not read"
    );
    assert!(
        !dir.exists(),
        "the listing created the directory it was complaining about"
    );
}

/// One store directory, and both programs read this one. The value is what it
/// always was; what is checked here is that it is still the relative directory
/// the reason above argues for, because an absolute one or a `$HOME` one would
/// be a different decision arrived at by an edit.
#[test]
fn the_default_store_is_one_directory_beside_the_session() {
    assert_eq!(STORE, ".karakuri");
    assert!(
        Path::new(STORE).is_relative(),
        "the default store is no longer beside the session"
    );
}

#[test]
fn plugin_directory_search_peels_off_candidates_in_order() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let exe_dir = tmp.path().join("bin");
    let workspace = tmp.path().join("tree");

    let bundle = exe_dir.join("..").join("PlugIns");
    let prefix = exe_dir.join("..").join("lib").join("karakuri").join("plugins");
    let beside = exe_dir.join("plugins");
    let ws = workspace.join("plugins");

    std::fs::create_dir_all(&bundle).expect("mkdir bundle");
    std::fs::create_dir_all(&prefix).expect("mkdir prefix");
    std::fs::create_dir_all(&beside).expect("mkdir beside");
    std::fs::create_dir_all(&ws).expect("mkdir ws");

    for expected in [
        Found::Bundle,
        Found::Prefix,
        Found::Beside,
        Found::Workspace,
    ] {
        let found = searched_plugins(Some(&exe_dir), &workspace)
            .expect("one of the four holds a plugin directory");
        assert_eq!(
            found.found,
            expected,
            "{} answered ahead of its turn",
            found.dir.display()
        );
        std::fs::remove_dir_all(&found.dir).expect("rmdir");
    }
}

#[test]
fn given_plugins_directory_is_respected_or_refused() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("my_plugins");
    assert!(plugins(Some(&dir)).is_err());

    std::fs::create_dir_all(&dir).expect("mkdir");
    let res = plugins(Some(&dir)).expect("valid dir").expect("found");
    assert_eq!(res.found, Found::Given);
    assert_eq!(res.dir, dir);
}
