use super::*;

/// A root of this test's own, cleaned of whatever a previous run left.
fn arrangement_root(what: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "karakuri-arrangement-{what}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// A panel with something folded and something soloed — the two things an
/// arrangement is kept for, and the two a reset forgets.
fn arranged(width: f32, height: f32) -> Panel {
    let mut panel = Panel::new(width, height);
    let staging = panel.layout().find("staging").expect("a staging bay");
    panel.op(Op::Fold(staging));
    let mixer = panel.layout().find("mixer").expect("a mixer bay");
    panel.op(Op::Solo(mixer));
    panel.solve();
    panel
}

/// Verifies arrangement files are saved at `arrangements/<name>.arrangement.json`
/// under the store root by checking disk paths directly (ADR-0221 §4).
#[test]
fn an_arrangement_is_kept_at_the_path_the_stores_header_names() {
    let root = arrangement_root("kept");
    let mut panel = arranged(1280.0, 720.0);

    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::SaveArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a save is one of the two operations this route answers for");

    let at = root.join("arrangements").join("four_deck.arrangement.json");
    let bytes = std::fs::read(&at).unwrap_or_else(|e| {
        panic!(
            "nothing at {} after `{said}` — a saved arrangement is one path component of \
             name, one of what it is, and one of the format it is in, under the store's \
             fourth directory: {e}",
            at.display()
        )
    });

    // And what is at that path is this panel's arrangement rather than
    // some other document that happens to be there.
    let back: Layout =
        serde_json::from_slice(&bytes).expect("the bytes at that path are an arrangement");
    assert!(
        back.is_soloed(),
        "the file at {} did not carry the solo the panel was saved with",
        at.display()
    );

    // Nothing else was created under the store: an arrangement is a fourth
    // thing beside `sets/`, `sessions/` and the artifacts, and not one of
    // them.
    assert!(
        !root.join("sets").join("four_deck.kbset").exists(),
        "the arrangement was filed as a Set"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies restored arrangements adapt to current window viewport dimensions rather
/// than overriding them with saved viewport geometry.
#[test]
fn an_arrangement_put_back_arrives_in_the_window_this_one_already_has() {
    let root = arrangement_root("back");
    let mut saved = arranged(1920.0, 1080.0);
    arrangement(
        &root,
        &mut saved,
        &mut view::Arrangement::default(),
        &Operation::SaveArrangement {
            name: "night_b".to_owned(),
        },
    )
    .expect("a save");

    // A window of a different size, with nothing folded and nothing soloed.
    let mut window = Panel::new(1280.0, 720.0);
    let staging = window.layout().find("staging").expect("a staging bay");
    assert!(!window.layout().is_collapsed(staging));

    let said = arrangement(
        &root,
        &mut window,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "night_b".to_owned(),
        },
    )
    .expect("a restore");

    let now = window.layout();
    assert_eq!(
        (now.viewport().w, now.viewport().h),
        (1280.0, 720.0),
        "`{said}` — the arrangement brought the window it was saved at with it. The window \
         is the operator's and never the file's"
    );
    assert!(
        now.is_soloed() && now.is_collapsed(now.find("staging").expect("staging")),
        "`{said}` — the fold and the solo did not come back, so what was put back is not \
         what was kept"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies `read_arrangement` fails without altering console state when
/// targeting non-existent names, missing stores, or corrupt files (ADR-0221 §2).
#[test]
fn a_name_nothing_is_filed_under_is_said_back_and_nothing_resets() {
    let root = arrangement_root("refused");
    let mut panel = arranged(1280.0, 720.0);
    let kept: Vec<bool> = panel
        .nodes()
        .iter()
        .map(|n| panel.layout().is_collapsed(n.id))
        .collect();
    let unchanged = |panel: &Panel, said: &str| {
        let now: Vec<bool> = panel
            .nodes()
            .iter()
            .map(|n| panel.layout().is_collapsed(n.id))
            .collect();
        assert_eq!(
            now, kept,
            "`{said}` and the arrangement moved — a refusal that resets the console is the \
             one thing `read_arrangement` promises never to do"
        );
        assert!(
            panel.layout().is_soloed(),
            "`{said}` and the solo went — see above"
        );
    };

    // 1. No store at all, and asking a question does not make one.
    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a restore");
    assert!(
        said.contains("four_deck"),
        "the refusal did not say the name back: {said}"
    );
    assert!(
        !root.exists(),
        "putting an arrangement back that is not there created a store at {}",
        root.display()
    );
    unchanged(&panel, &said);

    // 2. A store, and no such name in it.
    Store::open(&root).expect("a store");
    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a restore");
    assert!(
        said.contains("four_deck"),
        "the refusal did not say the name back: {said}"
    );
    unchanged(&panel, &said);

    // 3. A file filed under the name that is not an arrangement. Refused
    //    whole rather than repaired (ADR-0158), and told apart from
    //    *there is no such arrangement*, which is the distinction the
    //    sentence carries.
    Store::open(&root)
        .expect("a store")
        .write_arrangement("four_deck", b"{\"nodes\":[]}")
        .expect("bytes the store does not have to understand");
    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a restore");
    assert!(
        said.contains("disagrees with itself"),
        "a file that will not read back was reported as a missing arrangement, which sends \
         an operator looking for a name they typed correctly: {said}"
    );
    unchanged(&panel, &said);

    std::fs::remove_dir_all(&root).expect("clean up");
}

// -- the arrangement pill's half of the family ----------------------

/// Verifies validation rejects non-single-path-component names (P-0090) while
/// accepting valid identifiers.
#[test]
fn a_typed_arrangement_name_is_refused_where_the_file_is_written() {
    for good in ["night", "four_deck", "set-2", "A9"] {
        assert!(
            checked_name(good).is_ok(),
            "`{good}` is letters, digits, `-` and `_`, and was refused"
        );
    }
    for (bad, why) in [
        ("", "nothing was typed"),
        ("../../elsewhere", "a path"),
        ("night deck", "a space"),
        ("night.json", "a suffix of its own"),
    ] {
        let refusal = checked_name(bad).expect_err(&format!("`{bad}` is {why} and was kept"));
        assert!(
            refusal.starts_with("arrangement: "),
            "the refusal does not say what it is about: {refusal}"
        );
        assert!(
            bad.is_empty() || refusal.contains(bad),
            "the refusal does not say the name back, so an operator cannot see what \
             they typed: {refusal}"
        );
    }
}

/// Verifies active arrangement naming tracks successful file operations, ignoring refused saves.
#[test]
fn the_pill_names_the_arrangement_only_once_the_file_is_there() {
    let root = arrangement_root("in-use");
    let mut panel = arranged(1280.0, 720.0);
    let mut arr = view::Arrangement::NONE;

    // Refused: the name is not one path component, so nothing was filed
    // and nothing is in use.
    let said = arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::SaveArrangement {
            name: "night/one".to_owned(),
        },
    )
    .expect("a save is one of the operations this route answers for");
    assert!(said.contains("holds `/`"), "{said}");
    assert_eq!(arr.name, None, "a refused save put a name on the pill");
    assert!(arr.filed.is_empty());

    // Kept: in use, and listed.
    arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::SaveArrangement {
            name: "night".to_owned(),
        },
    )
    .expect("a save");
    assert_eq!(arr.name.as_deref(), Some("night"));
    assert_eq!(arr.filed, vec!["night".to_owned()]);

    // A restore of a name nothing is filed under is refused where the
    // bytes are, and leaves the pill alone.
    let said = arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::RestoreArrangement {
            name: "rehearsal".to_owned(),
        },
    )
    .expect("a restore");
    assert!(said.contains("rehearsal"), "{said}");
    assert_eq!(
        arr.name.as_deref(),
        Some("night"),
        "a refused restore moved the name the pill is showing"
    );

    // And one that is filed does put it in use.
    arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::RestoreArrangement {
            name: "night".to_owned(),
        },
    )
    .expect("a restore");
    assert_eq!(arr.name.as_deref(), Some("night"));

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies reset operations clear the arrangement name and restore default state.
#[test]
fn a_reset_leaves_the_pill_naming_no_file() {
    let mut readout = Readout::new(1280.0, 720.0);
    readout.view.arrangement.name = Some("night".to_owned());

    // An operation that is not a reset leaves it alone, which is what says
    // the clearing is the reset's and not every operation's.
    let staging = readout.panel.layout().find("staging").expect("staging");
    readout.op(Op::Fold(staging));
    assert_eq!(readout.view.arrangement.name.as_deref(), Some("night"));

    assert_eq!(readout.op(Op::Reset), Outcome::Reset);
    assert_eq!(
        readout.view.arrangement.name, None,
        "the console was reset to the default and the pill still names a file"
    );
}

/// Verifies arrangement load actions; deck picking updates bay targeting without moving selection (ADR-0305).
#[test]
fn every_ask_the_load_control_makes_is_acted_on_and_moves_only_this_bays_mark() {
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.mixer = (0..4)
        .map(|slot| view::Strip {
            name: format!("slot{slot}"),
            tally: view::Tally::Live,
            requested: view::Tally::Live,
            gain: 0.5,
            gain_to: None,
            opacity: 0.5,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
            is_muted: false,
            is_soloed: false,
        })
        .collect();
    assert!(readout.view.select(1), "the keys did not go to deck B");

    assert_eq!(readout.aimed(Aim::Open), Acted::Nothing);
    assert!(readout.view.target_open(), "the list did not come down");
    assert_eq!(readout.aimed(Aim::Shut), Acted::Nothing);
    assert!(!readout.view.target_open());

    // Target deck pick selects the destination deck and closes the list without changing active selection.
    readout.aimed(Aim::Open);
    assert_eq!(readout.aimed(Aim::Deck(2)), Acted::Nothing);
    assert_eq!(
        readout.view.target_deck(),
        2,
        "the pick did not aim the load"
    );
    assert_eq!(
        readout.view.selection(),
        1,
        "a pick in the pulldown moved the deck selection, which is the one thing this \
         control must not do"
    );
    assert!(
        !readout.view.target_open(),
        "the list stayed down after a pick"
    );

    // Emitting the load routes through the standard operation path.
    let want = Operation::LoadSet {
        deck: 2,
        set: "drift_night".to_owned(),
    };
    assert_eq!(
        readout.aimed(Aim::Load(want.clone())),
        Acted::Emitted(Some(want)),
        "the load did not go down the path every other emitted operation takes"
    );
    assert_eq!(
        readout.aimed(Aim::NoSet),
        Acted::Nothing,
        "a press with no Set under the cursor emitted something"
    );
}

/// What the menu's five asks do to this program, and that every one of them
/// that acts shuts the card.
#[test]
fn every_ask_the_pill_makes_is_acted_on_and_shuts_the_menu() {
    let mut readout = Readout::new(1280.0, 720.0);
    readout.view.arrangement.filed = vec!["night".to_owned()];

    assert!(matches!(readout.arranged(Ask::Open), Acted::Nothing));
    assert!(readout.view.arrangement.open());
    assert!(matches!(readout.arranged(Ask::Shut), Acted::Nothing));
    assert!(!readout.view.arrangement.open());

    readout.arranged(Ask::Open);
    assert!(matches!(readout.arranged(Ask::Name), Acted::Nothing));
    assert_eq!(
        readout.view.arrangement.naming(),
        Some(""),
        "the one item that asks for letters left nothing asking for any"
    );

    readout.arranged(Ask::Open);
    let did = readout.arranged(Ask::Panel(Op::Reset));
    assert!(
        matches!(did, Acted::Operated(Outcome::Reset)),
        "the reset was not performed: {did:?}"
    );
    assert!(
        !readout.view.arrangement.open(),
        "the card is still standing"
    );

    readout.arranged(Ask::Open);
    let want = Operation::RestoreArrangement {
        name: "night".to_owned(),
    };
    let did = readout.arranged(Ask::Operation(want.clone()));
    assert_eq!(
        did,
        Acted::Emitted(Some(want)),
        "the operation did not go down the path every other emitted operation takes"
    );
    assert!(!readout.view.arrangement.open());
}

/// A finished name is one operation of the vocabulary, and the card is gone
/// before it is emitted — whether or not the name is any good, since the
/// refusal is said out loud by `checked_name` and a card left standing over it
/// would be the panel asking again without saying the answer.
#[test]
fn a_finished_name_is_the_save_the_menu_would_have_asked_for() {
    let mut readout = Readout::new(1280.0, 720.0);
    assert_eq!(
        readout.named(),
        Acted::Nothing,
        "a console with nothing being typed committed a name"
    );

    readout.arranged(Ask::Name);
    for c in "four_deck".chars() {
        assert!(readout.view.arrangement.typed(c));
    }
    assert_eq!(
        readout.named(),
        Acted::Emitted(Some(Operation::SaveArrangement {
            name: "four_deck".to_owned()
        }))
    );
    assert!(!readout.view.arrangement.open());

    // An empty name is emitted as one and refused where the file is
    // written, rather than being swallowed here.
    readout.arranged(Ask::Name);
    assert_eq!(
        readout.named(),
        Acted::Emitted(Some(Operation::SaveArrangement {
            name: String::new()
        }))
    );
}

/// The menu's list is the store's, and a store that is not there is listed as
/// nothing and is not created — `library`'s two rules over the fourth
/// directory.
#[test]
fn the_menu_lists_the_store_and_makes_none() {
    let root = arrangement_root("listing");
    assert!(
        arrangements(&root).is_empty(),
        "a store that is not there listed something"
    );
    assert!(
        !root.exists(),
        "listing the arrangements created a store at {}",
        root.display()
    );

    let mut panel = arranged(1280.0, 720.0);
    for name in ["rehearsal", "four_deck"] {
        keep_arrangement(&root, &panel, name);
    }
    panel.solve();
    assert_eq!(
        arrangements(&root),
        vec!["four_deck".to_owned(), "rehearsal".to_owned()],
        "the menu lists what the store holds, in the order the store sorts it"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}
