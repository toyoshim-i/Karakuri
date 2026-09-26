use super::*;

/// Clicking a mixer strip's empty area selects that deck, routing keyboard focus accordingly.
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

/// Dragging a Set from the library and dropping it onto a mixer strip emits a `LoadSet` operation for the target deck.
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

/// Dropping a dragged Set onto a program preview cell emits `LoadSet` for that cell's deck while declining drops on empty cells (ADR-0273).
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

/// Moving the library selection during an open reading re-reads the landed row and preserves reading state.
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
