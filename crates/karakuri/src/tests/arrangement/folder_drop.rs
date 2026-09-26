use super::*;

/// Verifies dropping a folder atomically points the bay at it, selects the chip,
/// and lists its contained Sets in a single pass (ADR-0275).
#[test]
fn a_folder_dropped_on_the_window_points_the_bay_at_it() {
    let root = scratch_dir("folder-drop");
    let store = root.join("store");
    let handed = root.join("from-somebody");
    std::fs::create_dir_all(&handed).expect("a folder to drop");
    for name in ["night01.kbset", "sketch.kset", "adrift.kbset"] {
        std::fs::write(handed.join(name), "").expect("a Set file");
    }
    // What a folder scope must not list: a part, and a directory wearing a
    // Set file's name.
    std::fs::write(handed.join("blur.kir"), "").expect("a part");
    std::fs::create_dir_all(handed.join("unpacked.kbset")).expect("a directory");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert!(view.select_scope(Scope::MySets));
    let mut folder: Option<std::path::PathBuf> = None;

    let said = folder_dropped(&mut view, &mut folder, &store, None, &[handed.as_path()])
        .expect("a drop of one folder was answered");

    assert_eq!(folder.as_deref(), Some(handed.as_path()));
    assert_eq!(
        view.folder.as_deref(),
        Some(handed.display().to_string().as_str()),
        "the panel was not handed the line to draw in the `.path` row"
    );
    assert_eq!(
        view.scope(),
        Some(Scope::Folder),
        "the drop set the directory and left another chip marked"
    );
    assert_eq!(
        view.library,
        vec![
            "adrift".to_owned(),
            "night01".to_owned(),
            "sketch".to_owned()
        ],
        "the folder scope lists {:?}",
        view.library
    );
    assert!(
        said.contains(&handed.display().to_string())
            && said.contains("the `folder` chip is marked"),
        "the drop said `{said}`"
    );

    // **And the row is not drawn as a hover**: the release is what was
    // read, so nothing is on its way in afterwards.
    assert_eq!(view.pointed().map(|at| at.incoming), Some(false));

    // A second drop re-points it, which is the whole of *re-pointing the
    // bay is another drop*.
    let empty = root.join("empty");
    std::fs::create_dir_all(&empty).expect("a second folder");
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[empty.as_path()])
        .expect("a second drop was answered");
    assert_eq!(folder.as_deref(), Some(empty.as_path()));
    assert!(view.library.is_empty(), "{:?}", view.library);
    assert!(
        said.contains(why_nothing(Scope::Folder, true, false)),
        "a folder holding no Set said `{said}`"
    );
    // **The chip is marked on this one too**, and the sentence says so:
    // the second drop moved the listing without moving the mark, which is
    // the one case a *whether it moved* answer would have got backwards.
    assert!(
        said.contains("the `folder` chip is marked"),
        "a second drop said `{said}`"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies refusal of invalid drops (non-directories, multi-item drops) and
/// ensures library targeting remains unchanged (ADR-0275, P-0083).
#[test]
fn a_drop_that_is_not_one_folder_is_refused_and_the_bay_keeps_what_it_had() {
    let root = scratch_dir("folder-refusal");
    let store = root.join("store");
    let handed = root.join("from-somebody");
    std::fs::create_dir_all(&handed).expect("a folder to drop");
    std::fs::write(handed.join("night01.kbset"), "").expect("a Set file");
    let second = root.join("another");
    std::fs::create_dir_all(&second).expect("a second folder");
    let loose = root.join("notes.txt");
    std::fs::write(&loose, "").expect("a file to drop");
    let set = root.join("handover.kbset");
    std::fs::write(&set, "").expect("a Set file to drop");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    let mut folder: Option<std::path::PathBuf> = None;
    folder_dropped(&mut view, &mut folder, &store, None, &[handed.as_path()])
        .expect("the folder this bay is pointed at");
    let listed = view.library.clone();

    // A file, and it is not read as the directory it sits in.
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[loose.as_path()])
        .expect("a file was answered");
    assert!(
        said.contains("notes.txt") && said.contains("folder"),
        "the refusal an operator reads is `{said}`"
    );
    // **And it says where the bay is still pointed**, which is the half
    // that makes the refusal readable rather than the operator having to
    // look at the row to find out whether anything moved.
    assert!(
        said.contains(&format!("still pointed at `{}`", handed.display())),
        "the refusal does not say where the library is still pointed: `{said}`"
    );

    // A Set file, and the refusal carries the press that takes one in.
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[set.as_path()])
        .expect("a Set file was answered");
    assert!(
        said.contains("handover.kbset") && said.contains("row"),
        "a `.kbset` let go on the window said `{said}`"
    );

    // Two folders at once, and neither of them is taken.
    let said = folder_dropped(
        &mut view,
        &mut folder,
        &store,
        None,
        &[handed.as_path(), second.as_path()],
    )
    .expect("two paths were answered");
    assert!(
        said.contains('2') && said.contains("none of them"),
        "two folders at once said `{said}`"
    );

    // A path that is not there at all, which is a third thing and is said
    // as one: whether it is a folder was never learned.
    let gone = root.join("no-such-folder");
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[gone.as_path()])
        .expect("a path that is not there was answered");
    assert!(
        said.contains("no-such-folder") && said.contains("could not be examined"),
        "a path that is not there said `{said}`"
    );

    // **And after all four the bay is where it was**, which every one of
    // them said it would be.
    assert_eq!(folder.as_deref(), Some(handed.as_path()));
    assert_eq!(view.library, listed);
    assert_eq!(view.scope(), Some(Scope::Folder));

    // Nothing dropped is not a refusal and is not an answer.
    assert_eq!(
        folder_dropped(&mut view, &mut folder, &store, None, &[]),
        None
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies hovered file path display: single folders show path; multiple or empty hovers show none.
#[test]
fn a_folder_over_the_window_reads_in_the_path_row_and_two_read_as_none() {
    let one = karakuri_console::egui::HoveredFile {
        path: Some(std::path::PathBuf::from("/Volumes/stick/handover")),
        mime: String::new(),
    };
    let two = karakuri_console::egui::HoveredFile {
        path: Some(std::path::PathBuf::from("/Volumes/stick/another")),
        mime: String::new(),
    };

    let mut view = View::new(Room::Day);
    folder_over(&mut view, std::slice::from_ref(&one));
    assert_eq!(view.incoming.as_deref(), Some("/Volumes/stick/handover"));
    assert_eq!(view.pointed().map(|at| at.incoming), Some(true));

    // **Two at once say nothing**, because there is no path a release
    // would set — and picking the first would be the choice the refusal
    // above exists to refuse.
    folder_over(&mut view, &[one.clone(), two]);
    assert_eq!(view.incoming, None);

    // Out of the window again, and what was chosen is what is drawn.
    view.folder = Some("/Users/somebody/sets".to_owned());
    folder_over(&mut view, std::slice::from_ref(&one));
    assert_eq!(
        view.pointed().map(|at| at.path),
        Some("/Volumes/stick/handover")
    );
    folder_over(&mut view, &[]);
    assert_eq!(
        view.pointed(),
        Some(karakuri_console::view::Pointed {
            path: "/Users/somebody/sets",
            incoming: false,
        })
    );
}
