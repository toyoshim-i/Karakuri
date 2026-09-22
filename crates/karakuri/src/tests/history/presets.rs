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

/// The `presets` scope lists the Set files and not the parts beside them, which
/// is `console.html`'s *"a directory of `.kir` files is a directory of parts,
/// and a library lists what you can put on a deck"*.
///
/// It is asserted against `examples/`, which is the directory this program
/// actually opens on: thirty-five parts and twenty-three Set files in one place
/// is exactly the mixture the rule is about, and a listing that took the parts
/// would draw fifty-eight rows of which thirty-five name nothing this
/// vocabulary can load.
///
/// A CPU test: a preset library is a directory.
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

    // **The bundle form, which a `presets` root never offers**, and it is
    // what a *send* writes: `setfile::bundle` is `--package`'s own reading
    // — the Set with every source it names inlined — where the `.kbset`
    // sitting in `<store>/sets/` is a projection whose material is the
    // artifacts beside it and is **not** self-contained. So this is the
    // loop the send half closes, driven with the half that exists: a
    // package written into a directory the bay can be pointed at, and
    // taken in from a row of it. It is where a send's dialog opens
    // (ADR-0311), so the pair is the ordinary one rather than a contrived
    // one.
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

/// Loading a preset takes it into the store, so `all` gains a row nobody made —
/// `console.html`'s *"which is why opening a preset leaves one of your own
/// behind"*. It gains no row under `my sets`, which is ADR-0299's answer to the
/// roadmap's symptom: a preset packaged on load is a Set of the operator's and
/// is not one they chose to keep.
///
/// The whole of the press is asserted here except the aim, which is
/// [`loading`]'s and has its own test below: what a preset row adds is the
/// packaging in front of it, and the claim is that after it the id is one the
/// store holds and one `my sets` lists — which is what makes the load after it
/// the same route a `my sets` row takes rather than a second one.
///
/// A CPU test: a store is a directory, and resolving a `.kset` is a read, a
/// hash and a store put.
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

/// A press on a `presets` row is *Send a Set to somebody, and take one in*
/// performed at the second of its two moments, and then the load — so it emits
/// both, in that order.
///
/// `docs/manual/operations.html`: *"Taking one in is not a second row — opening
/// a preset is this row"*, and `console.html`: *"That is one press rather than
/// two because taking it in is what gives it the name the load needs."* Two
/// rows of the page and one press, and what this defends is that the press
/// names both of them: emitting only the load would be a press that performs
/// two rows and names one, and the row it dropped is the one nothing else in
/// this workspace constructs.
///
/// The titles are asked of [`Operation::title`] rather than written out here,
/// so a heading that moves on the page moves in one place.
///
/// A CPU test: it builds two values.
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

/// The take-in the press names writes no record, and that is settled — which is
/// why emitting it is a naming rather than a second route into anything.
///
/// `karakuri-operation-record` answers `Silent(NoRecord)` for a transfer:
/// nothing in the session vocabulary carries a Set arriving from somewhere
/// else. So [`App::performed`] performs nothing for it, exactly as it performs
/// nothing for the scope step `space` emits, and [`unwritten`] is what an
/// operator reads. If that ever became `Owed`, the press would be emitting a
/// gap rather than a settled silence and this file would be the place to say
/// so.
///
/// A CPU test: it is a `match` on an operation.
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

/// A preset this store already holds unchanged is loaded rather than refused,
/// and a row the library does not hold is still refused by name.
///
/// Before ADR-0347 a second press was a refusal, so nothing was taken in and
/// nothing loaded. Asserted here: the press succeeds *and* wrote nothing —
/// "it loaded" alone would pass on a press that filed a dated copy each time.
/// The sentence is `setfile::unbundle`'s; what is checked is that the press
/// goes through it.
///
/// A CPU test, for the test above's reason.
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

/// A preset library that has moved on since the press that took it in replaces
/// its own row, and the copy that was there is kept.
///
/// The library is a copy of `examples/` this test edits, rather than a Set
/// hand-built to differ: a `.kset` names its parts by relative path and the
/// stored file names them by content, so editing a `.kir` beside the `.kset` is
/// how the two come apart in practice.
///
/// A CPU test, for the test above's reason.
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

/// A scope is a listing on this side, and stepping to one answers it — two of
/// the four with rows here,ng to one answers it — two of
/// the four with rows here, and two with nothing and a sentence saying which
/// kind of nothing it is.
///
/// The two that answer nothing are the whole point of the test: they are empty
/// for two *different* reasons — nothing in this store is starred, and this
/// console has not been pointed at a folder — and a program that said the same
/// thing about both would be hiding one of them.
///
/// `folder` is here because of the console and not because of the scope: it
/// answers with rows the moment one is dropped on the window, which is
/// `a_folder_dropped_on_the_window_points_the_bay_at_it`, and the sentence it
/// answers with here is the third of the three — a scope nobody has pointed
/// anywhere.
///
/// A CPU test: a `View` takes no device.
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

    // The two that answer nothing here, and the sentences they answer
    // with are not one sentence. **`folder` is in this list because this
    // console has been pointed nowhere**, and not because the scope cannot
    // be answered: point it at a directory and it lists what is in it,
    // which is `a_folder_dropped_on_the_window_points_the_bay_at_it`.
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
    // **It says how a directory is chosen, and it used to say `operation`.**
    // This asserted that word until ADR-0275, on the reading that the chip
    // waited on one — which `ListSets`' own shape refutes: both its fields
    // narrow what a store already holds, and which store is asked at all
    // never was the operation's. What the sentence owes now is the way in.
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
