use super::*;

/// A strip, as far as the Library bay cares: something for a deck to be named
/// on. Every reading in it is beside the point here.
fn bare_strip() -> view::Strip {
    view::Strip {
        name: String::new(),
        tally: view::Tally::Allocated,
        requested: view::Tally::Allocated,
        gain: 0.0,
        gain_to: None,
        opacity: 0.0,
        opacity_to: None,
        blend: BlendMode::Add,
        mask: view::Mask::None,
        mask_angle: 0.0,
        level: None,
        is_muted: false,
        is_soloed: false,
    }
}

/// Verifies that secondary pointer press opens context menu on library rows while primary press initiates drag.
#[test]
fn a_secondary_press_opens_a_rows_menu_and_a_primary_press_does_not() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    readout.view.mixer = std::iter::repeat_with(bare_strip).take(4).collect();
    assert!(readout.view.select_scope(Scope::MySets));

    // The second row, asked of the derivation that draws it.
    let row = |readout: &mut Readout| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .row(1);
        Point::new(at.center().x, at.center().y)
    };

    // **A primary press takes the Set in hand and opens nothing.**
    let row_at = row(&mut readout);
    let at = row_at;
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    readout.pointer(&ctx, Pointer::Down);
    assert!(
        matches!(readout.panel.in_hand(), Some(InHand::Carrying)),
        "a primary press on a row did not take the Set in hand"
    );
    assert!(
        !readout.view.menu_open(),
        "a primary press on a row put that row's menu down, which takes the carry away"
    );
    // Let the carry go again, over nothing, so the gesture does not run on
    // into the presses below.
    readout.pointer(&ctx, Pointer::Up);

    // **A secondary press on the same row puts the menu down.**
    assert_eq!(readout.pointer(&ctx, Pointer::Secondary).1, Acted::Nothing);
    assert!(
        readout.view.menu_open(),
        "a secondary press on a row did not put that row's menu down"
    );
    assert_eq!(
        readout.view.menued().row,
        Some(1),
        "the menu came down on a row the press was not on"
    );

    // Pressing an outside control dismisses active context card before handling new control.
    readout.panel.solve();
    let button = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("the bay draws its foot")
    .load(&ctx, readout.view.target())
    .button;
    let at = Point::new(button.center().x, button.center().y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Nothing);
    assert!(
        !readout.view.menu_open(),
        "a press on the `load` button with a menu down did not dismiss it"
    );
    assert!(
        !readout.view.target_open(),
        "a press on the `load` button with a menu down opened the pulldown as well, so two \
         cards were down at once"
    );

    // Open it again, on the same row, for the pick below.
    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(row_at)).0,
        Claim::Panel
    );
    assert_eq!(readout.pointer(&ctx, Pointer::Secondary).1, Acted::Nothing);
    assert_eq!(readout.view.menued().row, Some(1));

    // **And the send is picked with a primary press on the card**, which
    // is rule 2: the card is down, so the press is the card's whichever
    // button it was.
    readout.panel.solve();
    let bay = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("the bay lists its rows");
    let menu = bay
        .menu(
            &ctx,
            view::to_egui(readout.panel.layout().viewport()),
            readout.view.menued(),
        )
        .expect("the menu is down");
    let save = menu.save.expect("a Set row's menu carries a send").center();
    let at = Point::new(save.x, save.y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::TransferSet {
            transfer: SetTransfer::Send {
                id: "lattice_veil".to_owned()
            }
        })),
        "`Save as a kbset` did not ask to send the row the menu was opened on"
    );
    assert!(
        !readout.view.menu_open(),
        "the card stayed down after an item was picked"
    );
}

/// A send that reached the disk says where it went, and a dismissed dialog
/// writes nothing and says so.
///
/// Three outcomes, one sentence each, and the third is the one worth the test:
/// a press that opened a window over the panel and then wrote nothing is
/// exactly the case a reader would otherwise read as a fault, and rule 04 of
/// the manual is that nothing is hidden quietly.
///
/// The dialog is not driven here and does not need to be. What a save dialog
/// answers is a path or nothing, so [`sent`] takes that answer and the platform
/// stays outside the test — the same split [`Save::run`] is on one act along,
/// where the thread is the caller's and the write is a function.
///
/// `None` is asserted to have written nothing at all, by counting the directory
/// rather than by trusting the sentence: a `sent` that bundled first and threw
/// the bytes away would print the same words.
///
/// A CPU test: a store read and a file written.
#[test]
fn a_send_says_where_it_went_and_a_dismissed_dialog_writes_nothing_and_says_so() {
    let root = scratch_dir("send-set");
    let store = Store::open(&root).expect("a store");
    // One Set with one node, so a bundle has a source to inline.
    let hash = store
        .put_artifact(b"proc p { }\n")
        .expect("the source is stored");
    store
        .write_set(
            "night01",
            &[karakuri_store::ndjson::Line::new(Record::Slot {
                at: karakuri_store::record::NodeAddress {
                    layer: karakuri_store::record::Layer::L1,
                    index: 0,
                },
                name: Some("geo".to_owned()),
                proc_hash: hash,
            })],
        )
        .expect("the set is written");

    let out = root.join("outbox");
    std::fs::create_dir_all(&out).expect("an outbox");

    // **Dismissed**: nothing is asked of the disk and the sentence says so.
    let said = sent(&root, "night01".to_owned(), None);
    assert_eq!(said.to, None);
    assert!(
        said.said().contains("dismissed") && said.said().contains("was not written"),
        "a dismissed dialog was reported as `{}`",
        said.said()
    );
    assert_eq!(
        std::fs::read_dir(&out).expect("the outbox").count(),
        0,
        "a dismissed dialog left a file behind"
    );

    // **Written**: the file is where the operator sent it and carries the
    // source inlined, which is what makes it a bundle rather than a copy.
    let to = out.join("night01.kbset");
    let said = sent(&root, "night01".to_owned(), Some(to.clone()));
    assert_eq!(
        said.outcome,
        Ok(()),
        "the send was refused: {}",
        said.said()
    );
    assert_eq!(
        said.said(),
        format!("  send: `night01` written to `{}`", to.display())
    );
    let text = std::fs::read_to_string(&to).expect("the bundle is on the disk");
    assert!(
        text.contains("proc p"),
        "the file names the source rather than carrying it: {text}"
    );

    // **Refused**: a Set this store does not hold, and the words are the
    // bundler's rather than a second copy of them.
    let said = sent(&root, "gone01".to_owned(), Some(out.join("gone01.kbset")));
    assert!(said.outcome.is_err(), "a Set nobody holds was packaged");
    assert!(
        said.said().contains("gone01") && said.said().contains("was not written to"),
        "a refused send was reported as `{}`",
        said.said()
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}
