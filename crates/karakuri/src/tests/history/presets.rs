use super::*;
use karakuri_console::view::{Scope, View};
use karakuri_environment::Asked;
use karakuri_operation::{Operation, SetTransfer};
use karakuri_operation_record::Written;

/// `examples/`, as a preset library this run was told about.
fn shipped_presets() -> karakuri_environment::places::Presets {
    karakuri_environment::places::Presets {
        dir: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples"),
        found: karakuri_environment::places::Found::Given,
    }
}

/// Verifies the `presets` scope lists runnable Set files while omitting raw `.kir` shader parts.
#[test]
fn the_presets_scope_lists_the_kset_files_and_not_the_parts_beside_them() {
    let presets = shipped_presets();
    let listed = presets_listing(Some(&presets));
    assert!(
        listed.len() >= 20,
        "`examples/` holds twenty-three `.kset` files and the listing found {}",
        listed.len()
    );
    for preset in &listed {
        assert!(
            preset.path.extension().and_then(|e| e.to_str()) == Some("kset"),
            "`{}` is listed and is not a Set file",
            preset.path.display()
        );
        assert!(
            !preset.id.ends_with(".kset") && !preset.id.is_empty(),
            "the row reads `{}`, which is a file name rather than a name",
            preset.id
        );
    }
    assert!(
        listed.iter().any(|preset| preset.id == "beat_cloud"),
        "`beat_cloud.kset` is in `examples/` and the listing does not have it"
    );
    assert!(
        !listed
            .iter()
            .any(|preset| preset.id.contains("drift_shell")),
        "`drift_shell.kir` is a part and the listing took it for a row"
    );

    // **Sorted, because a directory read is not.** Two runs that drew the
    // rows in two orders would be a bay nobody can point at.
    let mut sorted = listed.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
    sorted.sort();
    assert_eq!(
        listed.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
        sorted,
        "the listing is not in name order"
    );

    // And no preset library at all is no rows, which is a state rather
    // than a failure.
    assert!(presets_listing(None).is_empty());
}

/// Verifies taking in a Set from a folder directory and loading it (ADR-0267, ADR-0275).
#[test]
fn a_folder_row_is_taken_in_by_the_same_press_a_preset_row_is() {
    let root = scratch_dir("folder-take-in");
    Store::open(&root).expect("a store to take into");
    let examples = shipped_presets().dir;

    // **The authored form, out of a folder rather than out of `presets`.**
    // The rows are the same words the bay draws, and the file behind one
    // is found by asking the directory again on the press.
    assert!(
        folder_listing(Some(&examples))
            .iter()
            .any(|id| id == "beat_cloud"),
        "the folder scope does not list `beat_cloud` in `examples/`"
    );
    let taken = taking_in(&root, Taking::Folder(Some(&examples)), "beat_cloud")
        .expect("a folder row is taken in");
    assert_eq!(taken.id, "beat_cloud");
    assert_eq!(
        library(&root)
            .into_iter()
            .map(|set| set.id)
            .collect::<Vec<_>>(),
        vec!["beat_cloud".to_owned()],
        "the folder row was taken in and `all` does not list it"
    );
    // And a second press on it is refused: the half of ADR-0347 that did not
    // change, asserted at the press because that is where the two scopes were
    // confused.
    let refused = taking_in(&root, Taking::Folder(Some(&examples)), "beat_cloud")
        .expect_err("a folder row overwrote a Set this store holds");
    assert!(
        refused.contains("already in this store") && refused.contains("beat_cloud"),
        "the refusal an operator reads is `{refused}`"
    );
    assert_eq!(
        library(&root)
            .into_iter()
            .map(|set| set.id)
            .collect::<Vec<_>>(),
        vec!["beat_cloud".to_owned()],
        "a refused folder row left something behind"
    );
    // **And the two operations one press performs**, in the order they
    // happen: the transfer names the *file* and the load names the id the
    // file filed itself under.
    let [take, load] = taken_in_press(1, taken);
    assert!(
        matches!(&take, Operation::TransferSet { transfer: SetTransfer::Take { file } }
            if file.starts_with(&examples)),
        "the transfer does not name the file the row came off: {take:?}"
    );
    assert_eq!(
        load,
        Operation::LoadSet {
            deck: 1,
            set: "beat_cloud".to_owned()
        }
    );

    // Verifies bundles with inlined sources are accepted through the library import path (ADR-0311).
    let second = scratch_dir("folder-take-in-bundle");
    Store::open(&second).expect("a second store");
    let sent = scratch_dir("folder-take-in-sent");
    std::fs::create_dir_all(&sent).expect("a folder to send into");
    let package = karakuri_environment::setfile::bundle(
        &Store::open(&root).expect("the store"),
        "beat_cloud",
    )
    .expect("a package to send");
    karakuri_store::ndjson::write(
        &sent.join(format!("beat_cloud{}", Store::SET_FILE_SUFFIX)),
        &package,
    )
    .expect("the package is written where the bay is pointed");
    assert_eq!(folder_listing(Some(&sent)), vec!["beat_cloud".to_owned()]);
    let taken = taking_in(&second, Taking::Folder(Some(&sent)), "beat_cloud")
        .expect("a `.kbset` row is taken in");
    assert_eq!(taken.id, "beat_cloud");
    karakuri_environment::setfile::load(
        &Store::open(&second).expect("the second store"),
        "beat_cloud",
    )
    .expect("the Set that was just taken in cannot be read back");

    // **A folder nobody has pointed anywhere holds no row**, which is the
    // same refusal a preset root that has gone gives.
    assert!(taking_in(&second, Taking::Folder(None), "beat_cloud").is_err());

    std::fs::remove_dir_all(&root).expect("clean up");
    std::fs::remove_dir_all(&second).expect("clean up");
    std::fs::remove_dir_all(&sent).expect("clean up");
}

/// Verifies that ambiguous Set IDs sharing identical names across multiple formats in a folder are refused (P-0083).
#[test]
fn a_folder_row_two_files_wear_is_refused_with_both_names() {
    let dir = scratch_dir("folder-two-forms");
    std::fs::create_dir_all(&dir).expect("a folder to point at");
    std::fs::write(dir.join("night.kbset"), b"").expect("a bundle");
    std::fs::write(dir.join("night.kset"), b"").expect("an authoring file");

    assert_eq!(
        folder_listing(Some(&dir)),
        vec!["night".to_owned(), "night".to_owned()],
        "a directory of two forms of one Set draws two rows"
    );
    let refused = Taking::Folder(Some(&dir))
        .file("night")
        .expect_err("a word two files wear was resolved to one of them");
    assert!(
        refused.contains("night.kbset") && refused.contains("night.kset"),
        "the refusal does not name both files: {refused}"
    );

    std::fs::remove_dir_all(&dir).expect("clean up");
}

/// Verifies loading a preset records it in the store and lists it under `all` without polluting `my sets` (ADR-0299).
#[test]
fn loading_a_preset_takes_it_in_and_leaves_it_under_my_sets() {
    let root = scratch_dir("preset-take-in");
    let presets = shipped_presets();
    Store::open(&root).expect("a store to take a preset into");

    assert!(
        library(&root).is_empty(),
        "a fresh store lists something under `all`"
    );
    let taken = taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud")
        .unwrap_or_else(|e| panic!("`beat_cloud` was not taken in: {e}"));
    assert_eq!(
        taken.id, "beat_cloud",
        "the id is the file's own `set` record and not the row's word"
    );
    assert!(
        taken.said.contains("took `beat_cloud` in"),
        "the report the operator reads is `{}`",
        taken.said
    );
    assert_eq!(
        library(&root)
            .into_iter()
            .map(|set| set.id)
            .collect::<Vec<_>>(),
        vec!["beat_cloud".to_owned()],
        "the preset was taken in and `all` does not list it"
    );

    // **And the parts are in the store**, which is what makes the load
    // after this a load of material the store holds: `setfile::load` is
    // what the aim is built from and it reads them by address.
    karakuri_environment::setfile::load(&Store::open(&root).expect("the store"), "beat_cloud")
        .expect("the Set that was just taken in cannot be read back");

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies that `SetTransfer::Take` records the actual file path loaded.
#[test]
fn a_take_in_names_the_file_it_read_because_that_is_what_the_operation_carries() {
    let root = scratch_dir("preset-take-in-file");
    let presets = shipped_presets();
    Store::open(&root).expect("a store to take a preset into");

    let taken = taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud")
        .unwrap_or_else(|e| panic!("`beat_cloud` was not taken in: {e}"));
    assert_eq!(
        taken.file.file_name().and_then(|n| n.to_str()),
        Some("beat_cloud.kset"),
        "the take-in named `{}`, which is not the row's own authoring file",
        taken.file.display()
    );
    assert!(
        taken.file.starts_with(&presets.dir),
        "the take-in named `{}`, which is outside the preset library at `{}`",
        taken.file.display(),
        presets.dir.display()
    );
    assert!(
        taken.file.is_file(),
        "the take-in named `{}` and there is no file there",
        taken.file.display()
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies activating a preset row emits import and load operations in sequence using resolved operation titles.
#[test]
fn a_press_on_a_preset_row_names_the_take_in_and_then_the_load() {
    let root = scratch_dir("preset-press");
    let presets = shipped_presets();
    Store::open(&root).expect("a store to take a preset into");

    let taken = taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud")
        .unwrap_or_else(|e| panic!("`beat_cloud` was not taken in: {e}"));
    let file = taken.file.clone();
    let [take, load] = taken_in_press(2, taken);

    assert_eq!(
        take,
        Operation::TransferSet {
            transfer: SetTransfer::Take { file }
        },
        "the first of the two is not the take-in, or it does not name the file it read"
    );
    assert_eq!(
        take.title(),
        "Send a Set to somebody, and take one in",
        "the first of the two does not name the row the press performed"
    );
    assert_eq!(
        load,
        Operation::LoadSet {
            deck: 2,
            set: "beat_cloud".to_owned()
        },
        "the second of the two is not the load, or it does not name the id the file filed \
         itself under"
    );
    assert_eq!(
        load.title(),
        "Load material into a deck",
        "the second of the two does not name the row the press performed"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies preset import operations return `Silent(NoRecord)` and perform without recording history records.
#[test]
fn the_take_in_the_press_names_writes_no_record_and_that_is_settled() {
    let take = Operation::TransferSet {
        transfer: SetTransfer::Take {
            file: std::path::PathBuf::from("night01.kset"),
        },
    };
    assert_eq!(
        written(&take, &Current::default()),
        Written::Silent(Silent::NoRecord),
        "a transfer the press emits no longer writes a settled nothing"
    );
    let said = unwritten(&take, &written(&take, &Current::default()))
        .expect("a press that wrote no record says so");
    assert!(
        said.contains("no record, and that is settled"),
        "what the operator reads is `{said}`"
    );
}

/// Verifies re-loading an existing identical preset succeeds without duplicate disk writes (ADR-0347).
#[test]
fn a_preset_this_store_already_holds_is_loaded_rather_than_refused() {
    let root = scratch_dir("preset-current");
    let presets = shipped_presets();
    Store::open(&root).expect("a store to take a preset into");

    taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud").expect("the first take-in");
    let held = library(&root);

    let again = taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud")
        .expect("a preset already current is not a refusal");
    assert!(
        again.said.contains("already what the preset library ships"),
        "what the operator reads is `{}`",
        again.said
    );
    assert_eq!(again.id, "beat_cloud", "the load would name `{}`", again.id);
    assert_eq!(
        library(&root),
        held,
        "an unchanged preset changed what the store holds"
    );

    // And a row that is not in the preset library at all is refused
    // saying so, which is the other way a press finds nothing: the listing
    // is asked again on the press, so a file that has moved is met here
    // rather than inside the packaging.
    let gone = taking_in(&root, Taking::Presets(Some(&presets)), "no_such_preset")
        .expect_err("a preset that is not there was taken in");
    assert!(
        gone.contains("no_such_preset"),
        "the refusal an operator reads is `{gone}`"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies modified library presets replace active listings while retaining previous copies.
#[test]
fn a_preset_the_library_has_moved_on_from_is_replaced_and_the_old_one_kept() {
    let root = scratch_dir("preset-moved-on");
    let library_dir = root.join("examples");
    std::fs::create_dir_all(&library_dir).expect("a library of this test's own");
    for entry in std::fs::read_dir(shipped_presets().dir).expect("read examples") {
        let entry = entry.expect("entry");
        if entry.file_type().expect("file type").is_file() {
            std::fs::copy(entry.path(), library_dir.join(entry.file_name())).expect("copy");
        }
    }
    let presets = karakuri_environment::places::Presets {
        dir: library_dir.clone(),
        found: karakuri_environment::places::Found::Given,
    };
    Store::open(&root).expect("a store to take a preset into");

    taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud").expect("the first take-in");
    let before = std::fs::read_to_string(root.join("sets/beat_cloud.kbset")).expect("the Set file");

    // The library moves on: a part the `.kset` names is edited where it lies.
    // A comment, because the assertion is about the address changing.
    let part = library_dir.join("beat_shell.kir");
    let source = std::fs::read_to_string(&part).expect("the part");
    std::fs::write(&part, format!("// the library moved on\n{source}")).expect("edit the part");

    let said = taking_in(&root, Taking::Presets(Some(&presets)), "beat_cloud")
        .expect("the shipped library may replace its own row")
        .said;

    let held: Vec<String> = library(&root).into_iter().map(|set| set.id).collect();
    let retired: Vec<&String> = held.iter().filter(|id| *id != "beat_cloud").collect();
    assert_eq!(
        retired.len(),
        1,
        "the copy that was replaced was not kept: {held:?}"
    );
    let retired = retired[0];
    assert!(
        retired.starts_with("beat_cloud-"),
        "the retired copy is filed as `{retired}`"
    );
    assert!(
        said.contains(retired) && said.contains("kept as"),
        "the operator is not told where it went: `{said}`"
    );
    assert_eq!(
        std::fs::read_to_string(root.join(format!("sets/{retired}.kbset"))).expect("the kept Set"),
        before.replace("\"id\":\"beat_cloud\"", &format!("\"id\":\"{retired}\"")),
        "the kept copy is not the Set that was there, under its new id"
    );
    assert_ne!(
        std::fs::read_to_string(root.join("sets/beat_cloud.kbset")).expect("the Set file"),
        before,
        "`beat_cloud` still names the material it named before the library moved on"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies preset scope switching updates listings and supplies informative empty-state messages for unconfigured scopes.
#[test]
fn every_scope_is_answered_and_the_two_that_answer_nothing_say_which_nothing() {
    let root = scratch_dir("scope-listing");
    let presets = shipped_presets();
    let store = Store::open(&root).expect("a store to list");
    store.write_set("night01", &[]).expect("a Set to list");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    // **`all` is the first chip and is where a row of chips starts**, so
    // this is asserted rather than marked: `select_scope` answers whether
    // the mark *moved*, and a console handed this row is already on it.
    assert_eq!(view.scope(), Some(Scope::AllSets));

    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(view.library, vec!["night01".to_owned()]);
    assert!(said.contains("all") && said.contains('1'), "{said}");

    // **And `my sets` is that listing starred, which is nothing yet**
    // (ADR-0299): the store holds a Set and the operator has not chosen
    // it, so the subset is empty for an answer rather than for an absence.
    assert!(view.select_scope(Scope::MySets));
    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        view.library.is_empty(),
        "`my sets` listed {:?} and nothing has been starred",
        view.library
    );
    assert!(said.contains("my sets"), "{said}");

    // **A star put on it puts the row there**, which is the whole of what
    // the subset is: the same store, the same listing, one file beside it.
    assert!(
        favourite(
            &root,
            Asked::Operator,
            &Operation::SetFavourite {
                id: "night01".to_owned(),
                favourite: true,
            },
        )
        .is_some(),
        "`favourite` answered nothing for a `SetFavourite`"
    );
    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(view.library, vec!["night01".to_owned()], "{said}");
    assert!(
        view.starred.contains("night01"),
        "the bay was not told which rows are starred: {:?}",
        view.starred
    );
    // **And taking it off takes the row away again**, which is the state
    // this test goes on to assert the empty sentence of.
    favourite(
        &root,
        Asked::Operator,
        &Operation::SetFavourite {
            id: "night01".to_owned(),
            favourite: false,
        },
    )
    .expect("`favourite` answered nothing for a `SetFavourite`");

    assert!(view.select_scope(Scope::Presets));
    assert_eq!(view.scope(), Some(Scope::Presets));
    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        view.library.iter().any(|id| id == "beat_cloud"),
        "the `presets` scope lists {:?}",
        view.library
    );
    assert!(said.contains("presets"), "{said}");

    // Unpointed folder scope returns an informative empty-state message rather than generic empty listings.
    for scope in [Scope::MySets, Scope::Folder] {
        assert!(view.select_scope(scope));
        let said = listing(&mut view, &root, Some(&presets), None, None);
        assert!(
            view.library.is_empty(),
            "`{}` listed {:?}, and nothing in this run put a row there",
            scope.name(),
            view.library
        );
        assert!(
            said.contains(scope.name()) && said.contains(why_nothing(scope, false, false)),
            "`{}` lists nothing and says `{said}`",
            scope.name()
        );
    }
    assert_ne!(
        why_nothing(Scope::MySets, false, false),
        why_nothing(Scope::Folder, false, false),
        "the two scopes that answer nothing are empty for two different reasons and this \
         program gives one sentence for both"
    );
    // **It says what to press**, which is the difference between a scope
    // that is empty and a scope that is broken: `my sets` is the starred
    // subset, so the way to fill it is a star and the sentence names one.
    assert!(
        why_nothing(Scope::MySets, false, false).contains("star"),
        "the `my sets` sentence does not say what fills it: {}",
        why_nothing(Scope::MySets, false, false)
    );
    assert_ne!(
        why_nothing(Scope::AllSets, false, false),
        why_nothing(Scope::MySets, false, false),
        "a store nobody has saved into and a store nobody has starred in are given one \
         sentence"
    );
    // Reports directory selection instructions for empty folder scopes (ADR-0275).
    assert!(
        why_nothing(Scope::Folder, false, false).contains("drag")
            && why_nothing(Scope::Folder, false, false).contains("dropped"),
        "the `folder` sentence does not say how a directory is chosen: {}",
        why_nothing(Scope::Folder, false, false)
    );
    // **And a folder that *has* been pointed somewhere is a third kind of
    // nothing**, which is the sentence that arrived with the drop: an
    // empty scope for want of a gesture and one for want of a Set file in
    // the directory are the same drawing and not the same fact.
    assert_ne!(
        why_nothing(Scope::Folder, false, false),
        why_nothing(Scope::Folder, true, false),
        "a folder nobody has pointed anywhere and a folder holding no Set are given one \
         sentence"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}
