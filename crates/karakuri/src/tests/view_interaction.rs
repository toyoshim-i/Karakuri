use super::*;
use karakuri_console::view::{self, Scope, View};
use karakuri_store::record::Record;
use karakuri_store::store::Store;

/// The answer is written into the view, under the row the cursor is on, and a
/// Set that cannot be read says so and draws nothing.
///
/// [`read_reading`] is the glue the window loop runs on a press that asked for
/// a reading — [`listing`]'s shape one control along — and the two halves worth
/// a test are the ones a caller cannot see: that what is opened is the row
/// under the cursor rather than the first row, and that a failure closes the
/// block. A reading that failed to read and a Set that declares nothing must
/// not draw the same, which is `library`'s own rule one bay up.
///
/// A CPU test: a store is a directory and a `View` takes no device.
#[test]
fn a_reading_is_written_into_the_view_under_the_row_the_cursor_is_on() {
    let root = scratch_dir("read-set-view");
    let store = Store::open(&root).expect("a store to read");
    store.write_set("night01", &[]).expect("a Set to read");
    store.write_set("morph01", &[]).expect("a Set to read");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = vec!["night01".to_owned(), "morph01".to_owned()];
    assert!(
        view.walk(1, 0..2),
        "the cursor did not move off the first row"
    );

    let said = read_reading(&mut view, &root);
    assert!(
        said.contains("morph01"),
        "the reading names the first row rather than the one under the cursor: `{said}`"
    );
    let open = view.opened().expect("the reading is open under the cursor");
    assert_eq!(open.at, 1);
    assert_eq!(open.reading.id, "morph01");
    // A Set file with no `slot` record in it names no material, which is a
    // reading with nothing in it rather than a failure.
    assert_eq!(open.reading.nodes, 0);
    assert_eq!(open.reading.knobs_word(), "0 knobs");

    // **And a row naming a Set this store does not hold puts the block
    // away and says why**, where leaving the last reading drawn would
    // describe one Set under another's name.
    view.library = vec!["night01".to_owned(), "gone01".to_owned()];
    let said = read_reading(&mut view, &root);
    assert!(
        said.contains("gone01") && said.contains("could not be read"),
        "a Set that is not there was read as `{said}`"
    );
    assert!(
        !view.reading_open(),
        "a reading that failed to read left a block open"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// [`reread_if_open`] re-reads on a move with a reading open, and does nothing
/// on any other press — ADR-0265's rule, as a check on the one function both
/// `App::window_event` call sites share, rather than on the two copies of it
/// the window loop used to carry.
///
/// Until 2026-09-11 the two call sites were two verbatim statements, held equal
/// to each other only by a text scan
/// (`reading_follows_the_cursor::both_surfaces_re_read_the_row_the_cursor_arrived_at`)
/// that read this file and matched each one whole. Now there is one statement
/// and not two to keep in step, and this presses it directly: three presses,
/// only the middle one of which is a move, and only the third of which should
/// read anything.
///
/// A CPU test: a store is a directory and a `View` takes no device.
#[test]
fn reread_if_open_re_reads_only_on_a_move_with_a_reading_open() {
    let root = scratch_dir("reread-if-open");
    let store = Store::open(&root).expect("a store to read");
    store.write_set("night01", &[]).expect("a Set to read");
    store.write_set("morph01", &[]).expect("a Set to read");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = vec!["night01".to_owned(), "morph01".to_owned()];

    // **No reading is open**, so a move re-reads nothing — there is
    // nothing for the rule to keep following.
    assert!(
        view.walk(1, 0..2),
        "the cursor did not move off the first row"
    );
    assert_eq!(
        reread_if_open(true, &mut view, &root),
        None,
        "a move with no reading open re-read something anyway"
    );

    // A reading opens on the row the cursor is on now (`morph01`).
    let _ = read_reading(&mut view, &root);
    assert!(view.reading_open());

    // **A press that did not move the cursor**, with a reading open: the
    // rule is the cursor's, so this is the one call `Readout::took` used
    // to get wrong by discarding the `bool` `View::point_at` handed back.
    assert_eq!(
        reread_if_open(false, &mut view, &root),
        None,
        "a press that did not move the cursor re-read anyway"
    );
    assert_eq!(
        view.opened().expect("still open").reading.id,
        "morph01",
        "a press that did not move the cursor changed which row is open"
    );

    // **A move, with the reading still open**: the row the cursor
    // arrives at is the one that comes back, on whichever surface's
    // `moved` said so.
    assert!(view.walk(-1, 0..2), "the cursor did not move back to row 0");
    let said =
        reread_if_open(true, &mut view, &root).expect("a reading was open and the cursor moved");
    assert!(
        said.contains("night01"),
        "reread_if_open read the row the cursor left rather than the one it arrived at: {said}"
    );
    assert_eq!(view.opened().expect("still open").reading.id, "night01");

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A press on the `params` chip asks for the Set under the cursor, and a second
/// press puts the reading away.
///
/// The seam, driven the way an operator drives it: the pointer arrives, the
/// panel claims it, and what comes back is
/// [`Operation::ReadSet`](karakuri_operation::Operation::ReadSet) naming the
/// row the cursor is on. `press_handler` cannot see this — it reads text and
/// asks whether the control is asked — and `karakuri-console`'s own tests
/// cannot see it either, because the press handler is here.
///
/// The close emits nothing, which is the half worth a test: a second press
/// changes which rows the bay draws and asks no question, so a `ReadSet` here
/// would be this program saying a Set was read at the moment one stopped being.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_the_params_chip_asks_for_the_set_under_the_cursor() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));

    // The capsule, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let chip = |readout: &mut Readout| {
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
        .params_chip(&ctx, readout.view.target());
        Point::new(at.min.x + 2.0, at.center().y)
    };

    let at = chip(&mut readout);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::ReadSet {
            id: "drift_night".to_owned()
        })),
        "the chip did not ask for the Set under the cursor"
    );

    // **The reading is the caller's to answer**, and this file's press
    // handler holds no store — so the block is opened here the way the
    // window loop opens it, and the second press is what is under test.
    readout.view.read(Reading {
        id: "drift_night".to_owned(),
        ..Reading::default()
    });
    let at = chip(&mut readout);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Nothing,
        "closing a reading emitted an operation, which says a Set was read"
    );
    assert!(
        !readout.view.reading_open(),
        "the second press did not put the reading away"
    );
}

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

/// A secondary press on a Library row puts that row's menu down, and a primary
/// press on the same row does not.
///
/// This is the one thing neither crate could assert on its own.
/// `karakuri-console` has never known which button a press was — rules 1 to 4
/// of `input::claim` are all about where the pointer is — so the distinction
/// lives here, in the arm that turns a `winit` button into a [`Pointer`]. Both
/// halves are asserted because a handler that opened the menu on either button
/// would pass a test made only of the first, and would take the row's ordinary
/// press away: a primary press picks a Set up to carry it, which is what the
/// drag onto a strip is.
///
/// And the item is picked with either button, which is the other half of the
/// same seam: once the card is down it is `input::claim`'s rule 2, and that
/// rule is about a card being down rather than about what put it there. So the
/// pick here is a *primary* press on a card a secondary press opened.
///
/// A CPU test: a `Readout` takes no device.
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

    // **A press on another control's capsule dismisses this card rather
    // than opening that one.** The `load` button is the sharpest case
    // there is: it is in this bay's own foot, it is a control the pointer
    // reaches, and a handler that asked it before the card would have
    // opened the pulldown with a menu still down — two cards down at once,
    // which is the one thing `input::claim`'s rule 2 exists to make
    // impossible. This is the assertion that says which was asked first.
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

/// The marks a reading is spelled with are in the face the panel draws with.
///
/// [`spelled`] writes `0 – 8 · 2` with an en dash and a middle dot, and the
/// Library bay's old `load → A` pill is what this test exists because of: its
/// arrow was typed with a U+2192 `egui`'s default face does not carry, and the
/// panel drew `load □ A` for a release — *a readout of where a press lands,
/// with a tofu where the lands was*. A range with a tofu in it would be the
/// same failure on every row of every reading.
///
/// The minus is here too, because a declared range can start below zero — the
/// mock's own `twist` is `−2 – 2 · 0` — and it is the sign `format!` writes
/// rather than the typographic one, which is asserted so that the two are not
/// quietly swapped.
///
/// A CPU test: fonts are `egui`'s and take no device.
#[test]
fn the_marks_a_reading_is_spelled_with_are_in_the_face() {
    let ctx = drawn_once();
    let spelling = spelled("-2", "2", Some("0".to_owned()));
    assert_eq!(spelling, "-2 – 2 · 0");
    let font = egui::FontId::new(
        karakuri_console::room::size::BASE,
        egui::FontFamily::Proportional,
    );
    for mark in spelling.chars() {
        assert!(
            ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, &mark.to_string())),
            "`{mark}` is not in the default face, and the panel would draw a tofu where a \
             reading's range is"
        );
    }
}

/// A press on the outputs dot, through the window loop's own routing.
///
/// The other half of the test above: that one is a boundary the panel claims
/// and `egui` never sees, and this is the console's one control, which the
/// panel claims for a different reason — `egui` owns no widget anywhere here,
/// so a press routed to it would reach nothing at all.
///
/// What is asserted is the round trip an operator makes: the picture is on
/// screen, a click on the dot folds it away by name, and a click on the same
/// dot brings it back. The dot is where it is drawn and the press is the
/// panel's at every step.
#[test]
fn a_press_on_the_outputs_dot_folds_the_picture_and_unfolds_it() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let picture = readout
        .panel
        .layout()
        .find("program-view")
        .expect("program-view");
    let dot = |readout: &mut Readout| {
        readout.panel.solve();
        let row =
            outputs(&ctx, readout.panel.layout(), Open::CLOSED).expect("the row draws its sink");
        (Point::new(row.sink.center().x, row.sink.center().y), row.on)
    };

    let (at, on) = dot(&mut readout);
    assert!(on, "the picture is on screen, so the sink is on");

    // The pointer arrives, and the control is the panel's.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Operated(Outcome::Folded {
            id: picture,
            folded: true,
            root: false
        }),
        "the press did not reach the sink"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);

    // And the dot is dark, where it still is, and turns the picture back
    // on rather than unfolding whatever else is folded.
    let (at, on) = dot(&mut readout);
    assert!(!on, "the picture is folded and the sink is still lit");
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Operated(Outcome::Folded {
            id: picture,
            folded: false,
            root: false
        }),
        "the dark dot did not turn the picture back on"
    );
    assert!(
        dot(&mut readout).1,
        "the picture is back and the dot is dark"
    );
}

/// A press on a scope chip, through the window loop's own routing — which is
/// what ADR-0213 makes the *panel* badge mean.
///
/// `karakuri-console`'s `tests/library.rs` asserts everything up to the
/// operation with no window anywhere: that the chips answer a press, that a
/// press names the chip it landed on, and that nothing else in the bay takes
/// one. This is the half that badge is actually about — *"the row is claimed
/// the day a person who launched the instrument can perform that operation from
/// the panel in front of them"* — and a control demonstrated in that crate and
/// never wired here would pass there and be a lie this page tells on its own
/// authority.
///
/// What is asserted is the whole press and not the routing alone: the mark
/// moves to the chip that was pressed, the operation that leaves is
/// `SelectScope` with the payload it is specified to carry, and the library
/// cursor goes back to the top — because the listing under a new scope is a
/// listing this cursor has never seen, and a cursor left where it was would sit
/// on a Set nobody chose under a pill saying a press will load it.
///
/// And the chip that is already marked is pressed too, because that is the case
/// a step cannot reach: a step would go somewhere else, and a pointer names —
/// so the press is answered rather than refused, and the mark stays where it
/// is.
#[test]
fn a_press_on_a_scope_chip_names_the_library_the_bay_reads() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    // A console that has been told what libraries there are and handed a
    // listing for the one it opens on, which is what `resumed` does.
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));

    // The capsule, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let chip = |readout: &mut Readout, want: Scope| {
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
        let (_, at) = bay
            .chips(&ctx, &readout.view.scopes)
            .find(|(scope, _)| *scope == want)
            .expect("the scope is on the row");
        // Two pixels in from its own left edge: the last chip in the row
        // is clipped by the pane, so its centre can be off the row.
        Point::new(at.min.x + 2.0, at.center().y)
    };

    // **The cursor is somewhere other than the top**, so that the move
    // back to it is a move and not the state it was already in.
    assert!(readout.view.walk(1, 0..2), "the cursor did not move");
    assert_eq!(readout.view.cursor_row(), 1);

    let at = chip(&mut readout, Scope::Presets);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SelectScope { scope: Undecided })),
        "the press did not reach the chip"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.scope(),
        Some(Scope::Presets),
        "the press was routed and the mark stayed where it was"
    );
    assert_eq!(
        readout.view.cursor_row(),
        0,
        "the scope changed and the cursor is still pointing into the listing it left"
    );

    // **The marked chip, pressed** — answered rather than refused, and the
    // mark does not step off it the way the key would.
    let at = chip(&mut readout, Scope::Presets);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::SelectScope { scope: Undecided })),
        "a press on the chip that is already marked was not answered"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.scope(),
        Some(Scope::Presets),
        "a press on the marked chip stepped somewhere"
    );
}

/// A press on the star at the left of a library row, through the window loop's
/// own routing — the half of the badge ADR-0213 makes a badge mean, beside
/// [`a_press_on_a_filter_field_asks_the_store_for_a_narrower_listing`].
///
/// `karakuri-console`'s `tests/library.rs` says where the mark is and that it
/// answers a press; this says an operator reaches it — a control demonstrated
/// in that crate and never wired here would pass there and be a lie the page
/// tells on its own authority.
///
/// What is asserted is that the press names a state and not a step (ADR-0299):
/// the same mark pressed twice asks for two different things, because the
/// control reads the row's present mark and asks for the other one. That is the
/// failure a toggle hides completely — a press that always emitted `true` would
/// pass every assertion about the first press and never take a star off.
///
/// And that the star has not swallowed the row it sits in: a press on the row's
/// own ground still takes the Set in hand and names no operation, which is rule
/// 4's *a control claims what it acts on and no more* asked of the two boxes
/// that overlap.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_a_star_names_the_state_the_row_is_not_in() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];

    // The mark's box, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let star = |readout: &mut Readout, index: usize| {
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
        .star(index);
        Point::new(at.center().x, at.center().y)
    };

    // **Nothing starred, so the press asks for the star to go on.**
    let at = star(&mut readout, 1);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: true,
        })),
        "the press did not reach the star, or it named the wrong row"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);

    // **And with the row starred it asks for the star to come off**, which
    // is the same control reading the state it is drawn from. The marks
    // are the host's answer, so this is what `listing` would have written
    // after the write.
    readout.view.starred.insert("lattice_veil".to_owned());
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: false,
        })),
        "a starred row was asked to be starred again"
    );
    readout.pointer(&ctx, Pointer::Up);

    // **The row's own ground is still the row's.** A press at the far end
    // of the same row takes the Set in hand and names no operation, which
    // is what a carry is (ADR-0265).
    readout.panel.solve();
    let row = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("the bay lists its rows")
    .row(1);
    let ground = Point::new(row.max.x - 4.0, row.center().y);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(ground)).0,
        Claim::Panel
    );
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        !matches!(did, Acted::Emitted(Some(Operation::SetFavourite { .. }))),
        "a press on the row's own ground was answered by the star in it: {did:?}"
    );
    readout.pointer(&ctx, Pointer::Up);
}

/// A press on one of the Library bay's two filter fields, through the window
/// loop's own routing — the half of the badge ADR-0213 makes a badge mean, one
/// row under [`a_press_on_a_scope_chip_names_the_library_the_bay_reads`].
///
/// `karakuri-console`'s `tests/library.rs` asserts everything up to the
/// operation with no window anywhere: where the two fields are, that each steps
/// its own cycle, that the operation names where it arrived and carries the
/// other field untouched, and that nothing between them takes a press. This is
/// the half that says an operator reaches it — a control demonstrated in that
/// crate and never wired here would pass there and be a lie the page tells on
/// its own authority, which is exactly what `press_handler` was written after.
///
/// What is asserted is the whole press. The operation that leaves is `ListSets`
/// carrying the step; `View::narrow` has been called, so the field the panel
/// draws next frame reads the new value; and the library cursor is back at the
/// top, because the listing under a narrower filter is one this cursor has
/// never seen.
///
/// Both fields, because the operation carries both halves: the press on `layer`
/// has to come back with the `holds` the press before it set, and a route that
/// rebuilt the operation from one field would lose it.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_a_filter_field_asks_the_store_for_a_narrower_listing() {
    use karakuri_console::view::Field;

    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    // A console told what libraries there are, handed the listing for the
    // one it opens on and told what that listing's Sets are made of —
    // which is what `listing` does on this side of the seam.
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    readout.view.holds = vec!["drift_shell".to_owned(), "soft_points".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));
    // **The cursor is somewhere other than the top**, so that the move back
    // to it is a move and not the state it was already in.
    assert!(readout.view.walk(1, 0..2), "the cursor did not move");

    // The box, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let field = |readout: &mut Readout, which: Field| {
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
        .field(which)
        .expect("the bay draws its filter field");
        Point::new(at.center().x, at.center().y)
    };
    // **And one kind chip's, asked of the walk that paints them**, which
    // is the same rule one band down: a chip is as wide as the word in it,
    // so where it is is `egui`'s answer and never a remembered number.
    let kind_chip = |readout: &mut Readout, which: usize| {
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
        let at = bay
            .kind_chips(&ctx)
            .nth(which)
            .expect("the bay draws six kind chips")
            .1;
        Point::new(at.center().x, at.center().y)
    };

    let at = field(&mut readout, Field::Holds);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::ListSets {
            holds: Some("drift_shell".to_owned()),
            layer: None,
        })),
        "the press did not reach the `holds` field"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.filters().holds,
        Some("drift_shell"),
        "the press was routed and the field it stepped does not read it"
    );
    assert_eq!(
        readout.view.cursor_row(),
        0,
        "the listing narrowed and the cursor is still pointing into the one it left"
    );

    // **And the kind chips under it, which have to carry the field
    // through**: a press on a chip names all six and says nothing about
    // `holds`, so what the console is narrowed to afterwards is the pair as
    // it now stands (ADR-0338).
    let at = kind_chip(&mut readout, 2);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::FilterLibrary {
            kinds: karakuri_operation::LibraryKinds {
                l3: true,
                ..karakuri_operation::LibraryKinds::EVERYTHING
            },
        })),
        "the press did not reach the `L3` chip, or it named something other than all six"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert!(readout.view.filters().kinds.l3);
    assert_eq!(readout.view.filters().holds, Some("drift_shell"));
}

/// A press on a strip's ground addresses the keys to that deck — the whole
/// route, through the same `Readout::pointer` a hand goes through.
///
/// # Why it is here and can be nowhere else
///
/// `karakuri-console` owns both halves and cannot put them together:
/// `input::claim` there says a press on a strip is the panel's, and
/// `Mixer::select` says which deck it names, and nothing in that crate joins
/// the two. This file's press arm is the join, and the defect this was written
/// against lived exactly in the gap: `on_strip` asked four questions where the
/// bay has five, so every press on a strip's ground was routed to `egui`, the
/// `(Pointer::Down, Claim::Panel)` arm never ran, and the `bay.select(at)` call
/// at the end of it was unreachable — while `operations.html`'s *"click a
/// strip"*, `console.html`'s strip tip, `Mixer::select` and that call all said
/// it worked.
///
/// Deleting `|| bay.select(p).is_some()` from `input::on_strip` is the
/// injection this was watched to fail against: the claim comes back `Egui` and
/// the press asks for nothing.
///
/// The point is the strip's name box, which is the affordance's own words — *"a
/// press anywhere on this strip that no knob under the pointer claimed"* — and
/// the four that could have claimed it are asked here so that the press under
/// test is the leftover rather than a chip.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_a_strips_ground_selects_that_deck() {
    let ctx = drawn_once();
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
    assert_eq!(
        readout.view.selection(),
        0,
        "the selection is not deck A, so a press naming deck C could be naming the \
         selection it started on"
    );

    // The rectangle, off the derivation that draws it — the rule the whole
    // of `input` is written to, and the reason a test presses where the
    // paint painted.
    let bay = mixer_bay(&ctx, readout.panel.layout(), &readout.view.mixer)
        .expect("the mixer bay draws its strips");
    let name = bay.strip(2).name;
    let at = Point::new(name.center().x, name.center().y);
    assert!(
        bay.grab(at).is_none()
            && bay.blend(at).is_none()
            && bay.tally(at).is_none()
            && bay.mask(at).is_none(),
        "the name box is one of the four controls inside the column, so this press is \
         not the leftover the selection is made of"
    );

    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(at)).0,
        Claim::Panel,
        "a press on a strip went to egui, so the panel's own press arm never runs"
    );
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel, "a press on a strip went to egui");
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SelectDeck { deck: 2 })),
        "the press did not address the keys to the deck the strip is"
    );

    // **The selection moves where the operation is performed**, which is
    // `pointed` and not the press arm: `SelectDeck` writes no record, so
    // the surface that emits it is what performs it (ADR-0198).
    assert_eq!(
        readout.view.selection(),
        0,
        "the press moved the pointer itself"
    );
    assert!(pointed(&mut readout.view, &Operation::SelectDeck { deck: 2 }).is_some());
    assert_eq!(readout.view.selection(), 2);
}

/// A Set dragged from a library row onto a mixer strip loads the strip it was
/// let go over — the whole gesture, through the same `Readout::pointer` a hand
/// goes through.
///
/// # Why it is here and can be nowhere else
///
/// `karakuri-console` has both halves of the gesture and cannot put them
/// together: `carry.rs` there presses the model and the view directly, and
/// hands the destination in itself, because that crate has no press handler to
/// ask. The property that matters is which *moment* resolves the deck, and that
/// is this file's: the press is over the Library bay, where there is no strip
/// at all, and the release is over one. So a destination taken at the press
/// names nothing and the drop is cancelled, and a destination taken at the
/// release names the strip under the hand.
///
/// Deleting the `Mixer::dropped` ask from the release arm is the injection this
/// was watched to fail against, and moving it into the press arm is the second
/// — the first answers `Nowhere` for every drop and the second answers it for
/// every drop that began in the library, which is all of them.
///
/// # What it asserts, in the order a hand does it
///
/// 1. A press on the third row is the panel's, and it emits nothing: half a
/// gesture names one operand. 2. The cursor mark follows the hand, and
/// `Acted::Pointed` is the press saying so. It is no longer the whole of what
/// this console draws for a carry — the rectangle under the pointer is ringed
/// and the pointer is a grab, which is `View::draw`'s and is held by
/// `karakuri-console/tests/carry.rs`; the mock still draws no ghost. What is
/// owed on that answer when a reading is open is
/// [`a_carry_that_moves_the_cursor_re_reads_the_row_it_arrived_at`]; here it is
/// the mark alone, and `Acted::Nothing` in its place would be a press that
/// moved the cursor and told nobody. 3. Every move on the way is
/// `Acted::Nothing`, over two strips that are not the one it lands on. 4. The
/// release over strip C asks for `LoadSet` naming deck C and the Set from row 2
/// — not the selection, which is deck A throughout, and not the row the cursor
/// started on. 5. A second carry let go over nothing asks for nothing, which is
/// the outcome no other drag on this panel has.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_drop_on_a_strip_loads_the_strip_it_was_let_go_over() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec![
        "drift_night".to_owned(),
        "lattice_veil".to_owned(),
        "glass_shell".to_owned(),
    ];
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
    assert_eq!(
        readout.view.selection(),
        0,
        "the selection is not deck A, so a drop naming deck C could be naming the selection"
    );

    // The rectangles, off the derivations that draw them — the rule the
    // whole of `input` is written to, and the reason a test presses where
    // the paint painted.
    let row = |readout: &mut Readout, index: usize| {
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
        .row(index);
        Point::new(at.center().x, at.center().y)
    };
    let strip = |readout: &mut Readout, deck: u8| {
        readout.panel.solve();
        let at = mixer_bay(&ctx, readout.panel.layout(), &readout.view.mixer)
            .expect("the mixer bay draws its strips")
            .selected(deck)
            .expect("a strip for the deck");
        Point::new(at.center().x, at.center().y)
    };

    // 1 and 2: the press takes row 2 in hand, asks for nothing, and moves
    // the mark to the row the hand is on.
    let at = row(&mut readout, 2);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel, "a press on a library row went to egui");
    assert_eq!(
        did,
        Acted::Pointed,
        "the press asked for an operation, or moved the mark without answering that it did"
    );
    assert_eq!(
        readout.view.cursor_row(),
        2,
        "the mark did not follow the hand to the row it took"
    );

    // 3: nothing is emitted on the way, including over two strips it does
    // not land on.
    for over in [strip(&mut readout, 0), strip(&mut readout, 1)] {
        let (claim, did) = readout.pointer(&ctx, Pointer::Moved(over));
        assert_eq!(claim, Claim::Panel, "the carry lost its claim");
        assert_eq!(
            did,
            Acted::Nothing,
            "a move with a Set in hand asked for something"
        );
    }

    // 4: and the release names the strip it is over.
    let onto = strip(&mut readout, 2);
    readout.pointer(&ctx, Pointer::Moved(onto));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Emitted(Some(Operation::LoadSet {
            deck: 2,
            set: "glass_shell".to_owned(),
        })),
        "the drop did not name the strip it was let go over"
    );
    assert!(!readout.panel.dragging(), "the carry is still in hand");

    // 5: and one let go over nothing asks for nothing at all.
    let at = row(&mut readout, 0);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Pointed);
    let away = Point::new(-40.0, -40.0);
    readout.pointer(&ctx, Pointer::Moved(away));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Nothing,
        "a drop over nothing asked for a load"
    );
}

/// A Set let go on a deck preview cell loads that cell's deck, through the same
/// `Readout::pointer` a hand goes through — and a cell whose letter names no
/// slot loads nothing.
///
/// # Why it is here and not in `karakuri-console`
///
/// `carry.rs` there asks the two bays itself and hands the destination to
/// `Panel::released`. What this file owns is that the release asks the second
/// bay at all: the press handler resolved the drop against `Mixer::dropped`
/// alone until ADR-0273, so a carry that crossed to the centre column and let
/// go on a cell was answered `Nowhere` — the panel drawing a ring round a
/// rectangle the release then declined to use. Deleting the
/// `ProgramBay::dropped` ask from the release arm is the injection this was
/// watched to fail against.
///
/// # And the slot count is asked with it
///
/// The deck here has three slots and the row is four cells, so cell D is drawn
/// with nothing behind the letter on it. A release there names no deck, which
/// is the refusal `3` already gets from the keyboard — `pointed`, off the same
/// `View::mixer` length. Passing `DECKS` instead of that length is the second
/// injection, and it asks for `LoadSet { deck: 3 }` on a deck that has no slot
/// 3.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_drop_on_a_preview_cell_loads_the_deck_its_letter_names() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    // The Program bay's cells are read from the bit `rearrange` writes, so
    // a frame's own first act is what puts them anywhere at all.
    view::rearrange(&mut readout.panel, readout.view.canvas);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec![
        "drift_night".to_owned(),
        "lattice_veil".to_owned(),
        "glass_shell".to_owned(),
    ];
    readout.view.mixer = (0..3)
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
    assert_eq!(
        readout.view.selection(),
        0,
        "the selection is not deck A, so a drop naming deck B could be naming the selection"
    );

    let row = |readout: &mut Readout, index: usize| {
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
        .row(index);
        Point::new(at.center().x, at.center().y)
    };
    let cell = |readout: &mut Readout, deck: usize| {
        readout.panel.solve();
        let cells = preview_rects(readout.panel.layout(), readout.view.canvas)
            .expect("the preview row is on screen");
        assert_eq!(cells.len(), DECKS, "the row is not four cells");
        let at = cells[deck];
        Point::new(at.center().x, at.center().y)
    };

    // Row 1 onto cell B: not the selection, not the row's index as a deck.
    let at = row(&mut readout, 1);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Pointed);
    let onto = cell(&mut readout, 1);
    readout.pointer(&ctx, Pointer::Moved(onto));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Emitted(Some(Operation::LoadSet {
            deck: 1,
            set: "lattice_veil".to_owned(),
        })),
        "the drop did not name the cell it was let go over"
    );
    assert!(!readout.panel.dragging(), "the carry is still in hand");

    // And cell D, which this deck has no slot for, loads nothing.
    let at = row(&mut readout, 2);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Pointed);
    let onto = cell(&mut readout, 3);
    readout.pointer(&ctx, Pointer::Moved(onto));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Nothing,
        "a drop on the fourth cell of a three-slot deck asked for a load"
    );
}

/// A carry that moves the library cursor re-reads the row it arrived at, which
/// is the rule the cursor states rather than the keyboard: *"the reading
/// follows the cursor: a move with one open is a read of the row it arrived
/// at"* (`karakuri-console/src/view.rs`, `View::reading_open`).
///
/// # The defect it was written for
///
/// `Readout::took` discarded `View::point_at`'s `moved`. So taking a row in
/// hand while a reading was open on a different row moved the cursor off that
/// row, `View::opened` answered `None` because the row under the cursor was no
/// longer the Set the reading was of, and the block disappeared — for the rest
/// of the run, because nothing on this route ever walks the cursor back. The
/// arrow keys never had it: they re-read on `moved && reading_open()`.
///
/// # Why it is here and can be nowhere else
///
/// It needs all three of a press handler, a store on a disk, and the glue
/// between them, and this file is the only place that has any two. `carry.rs`
/// in `karakuri-console` presses the bay and the view directly and that crate
/// reaches no disk at all (ADR-0156), so the half it can hold is
/// `the_row_a_hand_takes_is_the_row_the_cursor_marks` — that `point_at` answers
/// the move — and not that anything acts on the answer.
///
/// # What it asserts, and what each one fails against
///
/// 1. The press answers `Acted::Pointed`, which is the whole of what
/// `Readout::took` can do about it: the readout holds no store, so the press
/// says *the cursor moved* and the caller reads the file. A `took` that drops
/// the `bool` again answers `Acted::Nothing` here. 2. The block is gone until
/// it is re-read, which is the defect itself, asserted so that step 3 cannot
/// pass by the reading never having moved. 3. `read_reading` — the call the
/// window loop makes on that answer — puts the reading under the row the hand
/// took, naming that row's Set. 4. A press on the row the cursor is already on
/// answers `Acted::Nothing`, so a carry that moved nothing costs no file read.
///
/// What it cannot see is that the window loop makes the call, because `winit`
/// cannot be asked for an `ActiveEventLoop` outside its own loop and an event
/// handler is not something a test can drive — `Readout::pointer`'s own doc.
/// `App::window_event`'s carry arm calls [`reread_if_open`] with
/// `matches!(acted, Acted::Pointed)`, the same function
/// [`reread_if_open_re_reads_only_on_a_move_with_a_reading_open`] presses
/// directly, below — this test is the two halves either side of that call, and
/// neither reaches the call itself.
///
/// A CPU test: a store is a directory and a `Readout` takes no device.
#[test]
fn a_carry_that_moves_the_cursor_re_reads_the_row_it_arrived_at() {
    let ctx = drawn_once();
    let root = scratch_dir("carry-reading");
    let store = Store::open(&root).expect("a store to read");
    store.write_set("drift_night", &[]).expect("a Set to read");
    store.write_set("lattice_veil", &[]).expect("a Set to read");

    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));

    // The rectangles, off the derivation that draws them — and the open
    // reading goes in with them, because the block is drawn among the rows
    // and the row below it is somewhere else while it is down.
    let row = |readout: &mut Readout, index: usize| {
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
        .row(index);
        Point::new(at.center().x, at.center().y)
    };

    // The reading is opened the way the window loop opens it, on the row
    // the cursor starts on.
    read_reading(&mut readout.view, &root);
    let open = readout.view.opened().expect("a reading on the first row");
    assert_eq!((open.at, open.reading.id.as_str()), (0, "drift_night"));

    // 1: the press on the other row takes it in hand and says the mark
    // moved.
    let at = row(&mut readout, 1);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel, "a press on a library row went to egui");
    assert_eq!(
        did,
        Acted::Pointed,
        "the carry moved the library cursor and answered nothing, so the reading open on \
         the row it left has nowhere to be drawn and nothing to bring it back"
    );
    assert_eq!(readout.view.cursor_row(), 1);

    // 2: and until the answer is acted on, the block is drawn nowhere.
    assert!(
        readout.view.opened().is_none(),
        "the reading is still drawn on a row the cursor has left"
    );
    assert!(readout.view.reading_open(), "the reading was put away");

    // 3: what the window loop does with that answer.
    let said = read_reading(&mut readout.view, &root);
    assert!(
        said.contains("lattice_veil"),
        "the re-read named a row the hand is not on: `{said}`"
    );
    let open = readout
        .view
        .opened()
        .expect("the reading followed the cursor to the row the hand took");
    assert_eq!((open.at, open.reading.id.as_str()), (1, "lattice_veil"));
    readout.pointer(&ctx, Pointer::Up);

    // 4: and a press on the row the cursor is already on moves nothing,
    // so it owes no read at all.
    let at = row(&mut readout, 1);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Nothing,
        "a press on the row the cursor was already on asked for a re-read of it"
    );
    readout.pointer(&ctx, Pointer::Up);

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The filter row narrows what the bay lists, through the summary.
///
/// The other half of the same press: `a_press_on_a_filter_field_…` says the
/// operation reaches `View::narrow`, and this says the listing that comes back
/// afterwards is a narrower one — which is the whole point, and was impossible
/// while this side asked `Store::list_sets` for names.
///
/// A procedure is a row of `all` and of `presets`, with its kind on it — and of
/// neither `my sets` nor `folder` (ADR-0338, decision 1).
///
/// A CPU test: two tiers on a disk, a `View`, and no window.
#[test]
fn the_two_tiers_list_procedures_beside_sets_and_two_scopes_do_not() {
    use karakuri_operation::LibraryKinds;
    use karakuri_store::hash::Hash;
    use karakuri_store::ndjson::Line;
    use karakuri_store::record::Layer as Written;

    let root = scratch_dir("procedure-listing");
    let store = Store::open(&root).expect("a store to list");
    store
        .write_set(
            "night01",
            &[Line::new(Record::Slot {
                at: karakuri_store::record::NodeAddress {
                    layer: Written::L1,
                    index: 0,
                },
                name: Some("drift_shell".to_owned()),
                proc_hash: Hash::of(b"drift_shell"),
            })],
        )
        .expect("a Set to list");
    std::fs::write(
        root.join(Store::PROCEDURES).join("orbit_wide.kir"),
        "proc orbit_wide {\n  kind L3\n}\n",
    )
    .expect("a kept procedure");
    let shipped = root.join("shipped");
    std::fs::create_dir_all(&shipped).expect("mkdir");
    std::fs::write(shipped.join("beat_glow.kset"), "{}\n").expect("a shipped Set");
    std::fs::write(shipped.join("tunnel_eye.kir"), "  kind L3\n").expect("a shipped procedure");
    let presets = karakuri_environment::places::presets(Some(&shipped))
        .expect("the root resolves")
        .expect("a root");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();

    // `all`: the Set and the kept procedure, and the procedure carries the
    // kind its `kind` line declares while the Set carries its slots'.
    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        view.library.contains(&"orbit_wide".to_owned())
            && view.library.contains(&"night01".to_owned()),
        "`all` lists {:?} — {said}",
        view.library
    );
    let at = view
        .library
        .iter()
        .position(|row| row == "orbit_wide")
        .expect("the procedure is a row");
    assert_eq!(
        view.kinds[at],
        RowKind {
            badges: vec![karakuri_operation::Layer::L3],
            procedure: true
        },
        "the procedure row's badge is not its kind"
    );
    let at = view
        .library
        .iter()
        .position(|row| row == "night01")
        .expect("the Set is a row");
    assert_eq!(
        view.kinds[at],
        RowKind {
            badges: vec![karakuri_operation::Layer::L1],
            procedure: false
        },
        "the Set row's badges are not the layers its slots fill"
    );

    // `presets`: the shipped Set and the shipped procedure, in name order.
    assert!(view.select_scope(Scope::Presets));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["beat_glow".to_owned(), "tunnel_eye".to_owned()]
    );
    assert!(view.kinds[1].procedure, "the shipped `.kir` is not a row");

    // `my sets` lists no procedure, because a star is refused on anything
    // `sets/` does not hold; `folder` lists none, because a folder row is a
    // take and nothing takes a bare `.kir` in.
    assert!(view.select_scope(Scope::MySets));
    listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        !view.library.contains(&"orbit_wide".to_owned()),
        "`my sets` lists a procedure: {:?}",
        view.library
    );
    assert!(view.select_scope(Scope::Folder));
    listing(&mut view, &root, Some(&presets), Some(&shipped), None);
    assert!(
        !view.library.contains(&"tunnel_eye".to_owned()),
        "`folder` lists a procedure: {:?}",
        view.library
    );

    // **The kind chips narrow by OR, and none on is everything.**
    assert!(view.select_scope(Scope::AllSets));
    let cameras = LibraryKinds {
        l3: true,
        ..LibraryKinds::EVERYTHING
    };
    assert!(view.narrow(None, cameras));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["orbit_wide".to_owned()],
        "`L3` on lists {:?}",
        view.library
    );
    let sets_only = LibraryKinds {
        sets: true,
        ..LibraryKinds::EVERYTHING
    };
    assert!(view.narrow(None, sets_only));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["night01".to_owned()],
        "`SET` on lists {:?}",
        view.library
    );
    assert!(view.narrow(
        None,
        LibraryKinds {
            l3: true,
            sets: true,
            ..LibraryKinds::EVERYTHING
        }
    ));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library.len(),
        2,
        "an OR of two lists {:?}",
        view.library
    );
    assert!(view.narrow(None, LibraryKinds::EVERYTHING));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library.len(),
        2,
        "none on is not everything: {:?}",
        view.library
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// What it narrows is the store's own listing, which is `all` and is what *List
/// what the store holds* lists. `my sets` is that listing starred (ADR-0299),
/// so the same retain applies to it and the row is not a control over one chip.
///
/// It is the same retain the MCP tool applies, over the same
/// `setfile::summarise`, which is what keeps one operation from being answered
/// two ways by two surfaces.
///
/// A CPU test: a store, a `View`, and no window.
#[test]
fn the_filter_row_narrows_the_stores_listing_through_the_summary() {
    use karakuri_store::hash::Hash;
    use karakuri_store::ndjson::Line;
    use karakuri_store::record::Layer as Written;

    let root = scratch_dir("filter-listing");
    let store = Store::open(&root).expect("a store to list");
    let slot = |layer: Written, name: &str| {
        Line::new(Record::Slot {
            at: karakuri_store::record::NodeAddress { layer, index: 0 },
            name: Some(name.to_owned()),
            proc_hash: Hash::of(name.as_bytes()),
        })
    };
    store
        .write_set("night01", &[slot(Written::L1, "drift_shell")])
        .expect("a Set to list");
    store
        .write_set("veil02", &[slot(Written::L4, "soft_points")])
        .expect("a second Set to list");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert_eq!(view.scope(), Some(Scope::AllSets));

    // Unnarrowed: both Sets, and the candidates are what their nodes are
    // called — sorted, deduplicated, and read off the *unfiltered* listing.
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library.len(), 2, "the bay lists {:?}", view.library);
    assert_eq!(
        view.holds,
        vec!["drift_shell".to_owned(), "soft_points".to_owned()],
        "the `holds` field can be stepped to {:?}",
        view.holds
    );
    assert!(!said.contains("holding"), "{said}");

    // Narrowed by what a node is called.
    assert!(view.narrow(
        Some("drift_shell"),
        karakuri_operation::LibraryKinds::EVERYTHING
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library, vec!["night01".to_owned()], "{said}");
    assert!(
        said.contains("1 of 2") && said.contains("drift_shell"),
        "{said}"
    );

    // **A filter that matched nothing is a different nothing from an empty
    // store**, and the line says which: the store is not empty, and what to
    // do about it is press a field rather than save a Set.
    assert!(view.narrow(
        Some("drift_shell"),
        karakuri_operation::LibraryKinds {
            l3: true,
            ..karakuri_operation::LibraryKinds::EVERYTHING
        },
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert!(view.library.is_empty(), "the bay lists {:?}", view.library);
    assert!(
        said.contains("none of the 2 Sets here")
            && !said.contains(why_nothing(Scope::AllSets, false, false)),
        "{said}"
    );

    // **And the same retain applies to `my sets`**, which is this listing
    // starred: star one Set, mark the subset, and the filter that named
    // the other one leaves it with nothing — the narrowing is over what
    // the store holds and not over which chip is marked.
    assert!(view.narrow(None, karakuri_operation::LibraryKinds::EVERYTHING));
    favourite(
        &root,
        Asked::Operator,
        &Operation::SetFavourite {
            id: "night01".to_owned(),
            favourite: true,
        },
    )
    .expect("`favourite` answered nothing for a `SetFavourite`");
    assert!(view.select_scope(Scope::MySets));
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library, vec!["night01".to_owned()], "{said}");
    assert!(view.narrow(
        Some("soft_points"),
        karakuri_operation::LibraryKinds::EVERYTHING
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert!(
        view.library.is_empty(),
        "`my sets` lists {:?} under a filter that names the Set that is not starred",
        view.library
    );
    assert!(said.contains("none of the 2 Sets here"), "{said}");

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A press on the Program bay's `solo` pill, through the window loop's own
/// routing.
///
/// `karakuri-console`'s `tests/solo_pill.rs` and `tests/vocabulary.rs` assert
/// everything up to the operation with no window anywhere; this is the half
/// ADR-0213 makes the badge mean — *"the row is claimed the day a person who
/// launched the instrument can perform that operation from the panel in front
/// of them"* — and a control demonstrated in that crate and never wired here
/// would pass there and be a lie the page tells.
///
/// Both directions, because the pill is both. A solo takes every other control
/// off the screen, so the pill is the only thing left to press and the undo has
/// to come from it. What is asserted is the round trip an operator makes: the
/// picture is one region among many, a click on the pill leaves it holding the
/// window, and a click on the same pill — found again where it is now drawn,
/// because the solo moved every rectangle on the console — puts everything
/// back.
#[test]
fn a_press_on_the_solo_pill_solos_the_picture_and_undoes_it() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let picture = readout
        .panel
        .layout()
        .find("program-view")
        .expect("program-view");
    let library = readout.panel.layout().find("library").expect("library");
    // The capsule, asked of the derivation that draws it rather than
    // remembered — which is the rule the whole of `input` is written to,
    // and here it is load-bearing twice over.
    let pill = |readout: &mut Readout| {
        readout.panel.solve();
        let head = program_head(&ctx, readout.panel.layout(), Open::CLOSED)
            .expect("the bay draws its pill");
        (
            Point::new(head.solo.center().x, head.solo.center().y),
            head.soloed,
        )
    };

    let (at, soloed) = pill(&mut readout);
    assert!(!soloed, "something is soloed before anything was pressed");
    assert!(
        readout.panel.layout().visible(library),
        "the library is off the screen already, so soloing would prove nothing"
    );

    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Operated(Outcome::Soloed(picture)),
        "the press did not reach the pill"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    readout.panel.solve();
    assert!(
        !readout.panel.layout().visible(library),
        "the picture is soloed and the library is still on the screen"
    );

    // And the same pill, where it is now, undoes it.
    let (at, soloed) = pill(&mut readout);
    assert!(soloed, "the picture is soloed and the pill does not say so");
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Operated(Outcome::Unsoloed { was: true }),
        "the pill did not undo the solo it made"
    );
    readout.pointer(&ctx, Pointer::Up);
    readout.panel.solve();
    assert!(
        readout.panel.layout().visible(library),
        "undoing the solo left the library folded"
    );
}
