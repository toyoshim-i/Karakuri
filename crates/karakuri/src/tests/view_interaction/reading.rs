use super::*;

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
