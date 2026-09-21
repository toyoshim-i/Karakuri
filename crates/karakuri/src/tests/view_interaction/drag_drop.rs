use super::*;

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
///    gesture names one operand.
/// 2. The cursor mark follows the hand, and `Acted::Pointed` is the press saying
///    so. It is no longer the whole of what this console draws for a carry — the
///    rectangle under the pointer is ringed and the pointer is a grab, which is
///    `View::draw`'s and is held by `karakuri-console/tests/carry.rs`; the mock
///    still draws no ghost. What is owed on that answer when a reading is open is
///    [`a_carry_that_moves_the_cursor_re_reads_the_row_it_arrived_at`]; here it is
///    the mark alone, and `Acted::Nothing` in its place would be a press that
///    moved the cursor and told nobody.
/// 3. Every move on the way is `Acted::Nothing`, over two strips that are not
///    the one it lands on.
/// 4. The release over strip C asks for `LoadSet` naming deck C and the Set from
///    row 2 — not the selection, which is deck A throughout, and not the row the
///    cursor started on.
/// 5. A second carry let go over nothing asks for nothing, which is the outcome
///    no other drag on this panel has.
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
///    `Readout::took` can do about it: the readout holds no store, so the press
///    says *the cursor moved* and the caller reads the file. A `took` that drops
///    the `bool` again answers `Acted::Nothing` here.
/// 2. The block is gone until it is re-read, which is the defect itself,
///    asserted so that step 3 cannot pass by the reading never having moved.
/// 3. `read_reading` — the call the window loop makes on that answer — puts the
///    reading under the row the hand took, naming that row's Set.
/// 4. A press on the row the cursor is already on answers `Acted::Nothing`, so
///    a carry that moved nothing costs no file read.
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
