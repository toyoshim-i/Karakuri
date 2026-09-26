use super::*;

/// Displays parameter reading details for the selected cursor row and closes reading blocks on read errors.
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

/// Cursor navigation moves refresh open parameter readings while non-navigational clicks leave readings unchanged (ADR-0265).
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

/// Verifies that pressing the params chip toggles `Operation::ReadSet` for the selected set.
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

/// Verifies reading display typography uses en-dashes and middle dots supported by console fonts without glyph fallback tofu.
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
