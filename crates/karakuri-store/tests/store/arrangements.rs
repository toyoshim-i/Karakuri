use super::*;

/// An arrangement with the two things that have historically not survived a
/// wire: an unbounded maximum, which JSON cannot spell, and a fold, which is
/// the flag a solo replaces. Both are in the console's own arrangement, so this
/// is a small stand-in for it rather than an exotic case.
fn an_arrangement() -> karakuri_layout::Layout {
    use karakuri_layout::{Rect, Spec};

    let mut l = karakuri_layout::Layout::new(Spec::row(
        4.0,
        vec![
            Spec::view("library").fixed(240.0).min(120.0).max(400.0),
            Spec::column(
                4.0,
                vec![
                    Spec::view("program").flex(3.0),
                    Spec::view("mixer").fixed(180.0),
                ],
            )
            .named("centre")
            .flex(1.0),
            Spec::view("inspector").fixed(300.0),
        ],
    ));
    l.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
    l.solve();
    l
}

/// Every rectangle, in a fixed order, so two arrangements can be compared as
/// what they draw rather than as what they store.
fn drawn(l: &karakuri_layout::Layout) -> Vec<(Option<String>, karakuri_layout::Rect, (f32, f32))> {
    fn walk(
        l: &karakuri_layout::Layout,
        id: karakuri_layout::NodeId,
        out: &mut Vec<(Option<String>, karakuri_layout::Rect, (f32, f32))>,
    ) {
        out.push((l.name(id).map(str::to_string), l.rect(id), l.bounds(id)));
        for child in l.children(id).to_vec() {
            walk(l, child, out);
        }
    }
    let mut out = Vec::new();
    walk(l, l.root(), &mut out);
    out
}

#[test]
fn an_arrangement_round_trips_through_a_file() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let mut before = an_arrangement();
    let mixer = before.find("mixer").unwrap();
    before.collapse(mixer);
    before.solve();

    store
        .write_arrangement("four_deck", serde_json::to_vec(&before).unwrap().as_slice())
        .unwrap();
    let text = store.read_arrangement("four_deck").unwrap();
    let after: karakuri_layout::Layout = serde_json::from_slice(&text).unwrap();

    assert_eq!(drawn(&before), drawn(&after), "the file changed the panel");
    assert_eq!(before.viewport(), after.viewport());
    assert!(
        after.is_collapsed(after.find("mixer").unwrap()),
        "a fold did not survive the file"
    );
    // The unbounded maxima are the reason this asserts `bounds` at all: JSON
    // has no spelling for an infinity, and one lost on the way out comes back
    // as a region that will not grow.
    assert_eq!(
        after.bounds(after.find("program").unwrap()).1,
        f32::INFINITY
    );
}

#[test]
fn a_soloed_arrangement_comes_back_soloed_and_still_undoes() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let open = an_arrangement();
    let mut before = an_arrangement();
    let program = before.find("program").unwrap();
    before.solo(program);
    before.solve();

    store
        .write_arrangement("solo", serde_json::to_vec(&before).unwrap().as_slice())
        .unwrap();
    let mut after: karakuri_layout::Layout =
        serde_json::from_slice(&store.read_arrangement("solo").unwrap()).unwrap();

    assert!(after.is_soloed(), "the solo did not survive the file");
    after.unsolo();
    after.solve();
    assert_eq!(
        drawn(&after),
        drawn(&open),
        "the arrangement the solo was covering did not survive the file"
    );
}

#[test]
fn an_arrangement_is_the_bytes_it_was_given_and_nothing_appended() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // One document, no trailing newline. A store that added one would be
    // editing a format it has just declared it does not parse.
    let written = br#"{"nodes":[],"root":0}"#;
    store.write_arrangement("bytes", written).unwrap();
    assert_eq!(store.read_arrangement("bytes").unwrap(), written);
    assert_eq!(
        fs::read(
            dir.path()
                .join("arrangements")
                .join("bytes.arrangement.json")
        )
        .unwrap(),
        written
    );
}

#[test]
fn a_name_saved_twice_is_the_second_arrangement() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    store.write_arrangement("desk", b"first").unwrap();
    store.write_arrangement("desk", b"second").unwrap();

    assert_eq!(store.read_arrangement("desk").unwrap(), b"second");
    assert_eq!(store.list_arrangements().unwrap().len(), 1);
}

#[test]
fn reading_an_arrangement_nobody_saved_names_it() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    match store.read_arrangement("four_deck") {
        Err(StoreError::NoArrangement(name)) => assert_eq!(name, "four_deck"),
        other => panic!("expected a named refusal, got {other:?}"),
    }
    // And it says the name back, because that is the whole of what an operator
    // who mistyped one needs.
    assert_eq!(
        store.read_arrangement("four-deck").unwrap_err().to_string(),
        "no arrangement named `four-deck`"
    );
}

#[test]
fn an_arrangement_and_a_set_of_the_same_name_are_two_files() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    store.write_set("tonight", &a_set()).unwrap();
    store.write_arrangement("tonight", b"{}").unwrap();

    assert_eq!(store.read_set("tonight").unwrap(), a_set());
    assert_eq!(store.read_arrangement("tonight").unwrap(), b"{}");
    assert_eq!(
        store
            .list_sets()
            .unwrap()
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        ["tonight"]
    );
    assert_eq!(
        store
            .list_arrangements()
            .unwrap()
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        ["tonight"]
    );
}

#[test]
fn list_arrangements_orders_by_name_and_skips_what_the_layout_does_not_claim() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    for name in ["wide", "four_deck", "a-b"] {
        store.write_arrangement(name, b"{}").unwrap();
    }
    let arrangements = dir.path().join("arrangements");
    // An editor's backup, a write that died, and somebody's directory.
    fs::write(arrangements.join("four_deck.arrangement.json~"), b"{}").unwrap();
    fs::write(arrangements.join("wide.arrangement.json.tmp"), b"{}").unwrap();
    fs::write(arrangements.join("notes.txt"), b"{}").unwrap();
    fs::create_dir(arrangements.join("old.arrangement.json")).unwrap();

    let listed: Vec<String> = store
        .list_arrangements()
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(listed, ["a-b", "four_deck", "wide"]);
    // Repeatable, which is the whole reason the key is the name rather than
    // the time three files written in one millisecond all share.
    assert_eq!(
        store.list_arrangements().unwrap(),
        store.list_arrangements().unwrap()
    );
}

#[test]
fn an_arrangement_carries_when_it_was_written_and_an_empty_store_carries_none() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    assert_eq!(store.list_arrangements().unwrap(), Vec::new());

    let before = SystemTime::now() - Duration::from_secs(2);
    store.write_arrangement("desk", b"{}").unwrap();
    let entry = store.list_arrangements().unwrap().pop().unwrap();
    assert_eq!(entry.name, "desk");
    assert!(entry.written > before, "the write time is not the file's");

    // The directory taken away under the store is an error, not "nothing kept"
    // — the caller asked what is there and there is no answer.
    fs::remove_dir_all(dir.path().join("arrangements")).unwrap();
    assert!(matches!(
        store.list_arrangements(),
        Err(StoreError::Io { .. })
    ));
}

#[test]
fn open_establishes_the_arrangements_directory_beside_the_other_three() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("library");
    Store::open(&root).unwrap();
    assert!(root.join("arrangements").is_dir());

    // A store written by a build that had no arrangements gains the directory
    // the first time this one opens it.
    fs::remove_dir_all(root.join("arrangements")).unwrap();
    Store::open(&root).unwrap();
    assert!(root.join("arrangements").is_dir());
}

/// **A Set written into the sandbox lands there and nowhere else**, and the
/// library is written by the other method.
///
/// The store half of
/// `docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md`:
/// `sets/` is the operator's library, so a save asked for over MCP is written
/// with `write_sandbox_set` and the library is left exactly as it was.
///
/// **The negative control is the second half.** A test that only checked the
/// sandbox file appeared would pass against a writer that wrote both, which is
/// the failure that matters here — the operator's preset gone. So this asserts
/// what is *not* in `sets/` and then that the same id put through
/// `write_set` does land there, which is what stops the whole thing passing
/// against a store that writes everything into one directory.
#[test]
fn a_sandbox_set_is_written_to_sandbox_and_the_library_is_untouched() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let lines = vec![Line::new(Record::Set {
        id: "night01".into(),
        v: 1,
    })];
    store.write_sandbox_set("night01", &lines).unwrap();

    assert!(
        dir.path().join("sandbox").join("night01.kbset").exists(),
        "a save asked for over MCP did not reach the sandbox"
    );
    assert!(
        !dir.path().join("sets").join("night01.kbset").exists(),
        "a save asked for over MCP wrote the operator's library"
    );
    assert!(
        matches!(store.read_set("night01"), Err(StoreError::Io(_))),
        "the library answered for a set only the sandbox holds"
    );

    // The control: the same id, the same lines, through the library's own
    // writer. Without this the test above would pass against a store whose two
    // writers were one.
    store.write_set("night01", &lines).unwrap();
    assert!(dir.path().join("sets").join("night01.kbset").exists());
}

/// **The sandbox refuses what the library refuses**, because what may be in a
/// Set file is a property of the format and not of the directory.
///
/// One of the three is enough to check that the shared scan is reached — the
/// three sentences and the order they are asked in are `write_set`'s own tests
/// above — and a `part` is the one chosen because it is the check that makes
/// `.kbset` mean *already resolved*, which a sandbox file claims by carrying
/// the extension.
#[test]
fn a_sandbox_set_refuses_a_part_the_way_the_library_does() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let lines = vec![
        Line::new(Record::Set {
            id: "night01".into(),
            v: 1,
        }),
        Line::new(Record::Part {
            layer: Layer::L1,
            index: 0,
            name: None,
            path: "drift_shell.kir".into(),
        }),
    ];

    match store.write_sandbox_set("night01", &lines) {
        Err(StoreError::PartInSet { index }) => assert_eq!(index, 1),
        other => panic!("expected PartInSet, got {other:?}"),
    }
    assert!(!dir.path().join("sandbox").join("night01.kbset").exists());
}

/// **A store nobody has starred in answers with nothing, and is not an error.**
///
/// `favourites.json` is not established by `Store::open` the way the four
/// directories are, because an empty file and no file say the same thing and
/// only one of them is a write into a store somebody only wanted to read.
#[test]
fn an_unstarred_store_has_no_favourites_and_no_file() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    assert!(store.favourites().unwrap().is_empty());
    assert!(!dir.path().join(Store::FAVOURITES_FILE).exists());
}

/// **A star round-trips, and a second press of the same state writes nothing.**
///
/// The `false` back is the state already being the one asked for, which is what
/// keeps `Operation::SetFavourite` a state rather than a toggle: pressing
/// *star this* twice says the same thing twice, and the second one must not
/// touch the file.
#[test]
fn a_star_round_trips_and_the_same_state_twice_writes_nothing() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .write_set(
            "drift_night",
            &[Line::new(Record::Set {
                id: "drift_night".into(),
                v: 1,
            })],
        )
        .unwrap();

    assert!(store.set_favourite("drift_night", true).unwrap());
    assert_eq!(
        store.favourites().unwrap().into_iter().collect::<Vec<_>>(),
        vec!["drift_night".to_string()]
    );
    assert!(
        !store.set_favourite("drift_night", true).unwrap(),
        "starring what is already starred reported a write"
    );

    assert!(store.set_favourite("drift_night", false).unwrap());
    assert!(store.favourites().unwrap().is_empty());
    assert!(
        !store.set_favourite("drift_night", false).unwrap(),
        "unstarring what is not starred reported a write"
    );
}

/// **Starring a Set this store does not hold is refused with the id back**, and
/// nothing is written — the star is a control on a row, and a row is a Set the
/// store holds.
#[test]
fn starring_a_set_the_store_does_not_hold_is_refused() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    match store.set_favourite("nothing_here", true) {
        Err(StoreError::NoSet(id)) => assert_eq!(id, "nothing_here"),
        other => panic!("expected NoSet, got {other:?}"),
    }
    assert!(!dir.path().join(Store::FAVOURITES_FILE).exists());
}

/// **A stale mark survives the Set leaving and can be taken off**, which is the
/// whole of what happens to one.
///
/// Nothing in this program deletes or renames a Set, so a mark goes stale only
/// when a hand removes the file — and the answer is that the id stays, a
/// listing that intersects it with `list_sets` draws no row for it, and the
/// star can still be taken off without the Set coming back first.
#[test]
fn a_star_outlives_the_set_and_can_still_be_taken_off() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .write_set(
            "gone",
            &[Line::new(Record::Set {
                id: "gone".into(),
                v: 1,
            })],
        )
        .unwrap();
    store.set_favourite("gone", true).unwrap();

    fs::remove_file(dir.path().join("sets").join("gone.kbset")).unwrap();

    assert!(
        store.favourites().unwrap().contains("gone"),
        "a question pruned a mark, and a question writes nothing"
    );
    assert!(
        store.list_sets().unwrap().is_empty(),
        "the listing this is intersected with still holds the Set"
    );
    assert!(
        store.set_favourite("gone", false).unwrap(),
        "a stale mark could not be taken off without the Set coming back"
    );
    assert!(store.favourites().unwrap().is_empty());
}

/// **A favourites file that will not parse is said out loud**, because a
/// silently empty answer reads exactly like a library nobody has starred in and
/// the whole of `my sets` would go quiet with nothing to notice.
#[test]
fn a_favourites_file_that_is_not_a_list_of_ids_is_an_error() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    fs::write(dir.path().join(Store::FAVOURITES_FILE), b"{ not a list }").unwrap();

    match store.favourites() {
        Err(StoreError::Favourites { .. }) => {}
        other => panic!("expected Favourites, got {other:?}"),
    }
}
