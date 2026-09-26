use super::library_common::*;

// ---------------------------------------------------------------------------
// The scope row: four questions, one of them marked
// ---------------------------------------------------------------------------

/// Verifies the scope row draws all five chips in order: `all`, `my sets`,
/// `presets`, `folder`, `history` (ADR-0299, ADR-0308).
#[test]
fn the_scope_row_draws_the_chips_it_was_handed_in_the_order_it_was_handed_them() {
    let (mut view, mut panel) = showing_mock();
    let bay = bay(&panel);
    assert_eq!(
        chips(&mut view, &mut panel, &bay),
        vec!["all", "my sets", "presets", "folder", "history"],
        "the scope row is not this bay's five chips in this bay's order"
    );
}

/// Verifies the marked chip follows the scope pointer across all choices.
#[test]
fn the_marked_chip_is_the_scope_the_pointer_is_on() {
    let (mut view, mut panel) = showing_mock();
    let bay = bay(&panel);

    // A host that opens on `my sets` rather than on the first chip, which is
    // what `View::select_scope` is for — the run itself opens on `all`
    // (ADR-0299), and what this asserts is that a mark can be moved off the
    // chip the row starts on.
    assert!(view.select_scope(Scope::MySets));
    assert_eq!(view.scope(), Some(Scope::MySets));
    assert_eq!(
        marked(&mut view, &mut panel, &bay).as_deref(),
        Some("my sets")
    );

    for scope in [
        Scope::Presets,
        Scope::Folder,
        Scope::History,
        Scope::AllSets,
        Scope::MySets,
    ] {
        assert!(view.step_scope(), "the scope did not step");
        assert_eq!(view.scope(), Some(scope));
        assert_eq!(
            marked(&mut view, &mut panel, &bay).as_deref(),
            Some(scope.name()),
            "the scope stepped to `{}` and the wash stayed where it was",
            scope.name()
        );
    }
}

/// A step goes to the next scope and wraps, which is what
/// `docs/manual/operations.html` says the key does — and it takes the library
/// cursor back to the top, because the listing under it is about to be a
/// different listing.
#[test]
fn stepping_the_scope_wraps_and_takes_the_cursor_back_to_the_top() {
    let mut view = View::new(Room::Day);

    // A console nobody has told what libraries there are has nothing to step,
    // and says so rather than wrapping onto a chip that is not drawn.
    assert!(!view.step_scope(), "a console with no scopes stepped one");
    assert_eq!(view.scope(), None);

    view.scopes = Scope::ALL.to_vec();
    view.library = mock();
    assert_eq!(view.scope(), Some(Scope::AllSets), "the first chip");

    assert!(view.walk(2, 0..5), "the cursor did not move");
    assert_eq!(view.cursor_row(), 2);
    assert!(view.step_scope());
    assert_eq!(view.scope(), Some(Scope::MySets));
    assert_eq!(
        view.cursor_row(),
        0,
        "the scope changed and the cursor is still pointing into the listing it left"
    );

    assert!(view.step_scope());
    assert!(view.step_scope());
    assert!(view.step_scope());
    assert_eq!(view.scope(), Some(Scope::History), "the last chip");
    assert!(view.step_scope());
    assert_eq!(
        view.scope(),
        Some(Scope::AllSets),
        "the step off the last chip did not wrap round to the first"
    );

    // One chip is a cycle of one, and stepping it changes nothing at all: a
    // press that moved nothing costs no frame.
    let mut one = View::new(Room::Day);
    one.scopes = vec![Scope::MySets];
    assert!(!one.step_scope());
    assert_eq!(one.scope(), Some(Scope::MySets));
}

/// A scope this console was not handed is refused rather than marked, which is
/// `View::select` s rule one control along: a mark on a chip nobody drew is a
/// mark drawn nowhere, over a listing with no question above it.
#[test]
fn a_scope_that_is_not_on_the_row_is_refused() {
    let mut view = View::new(Room::Day);
    view.scopes = vec![Scope::MySets, Scope::Presets];
    assert_eq!(view.scope(), Some(Scope::MySets));

    assert!(
        !view.select_scope(Scope::Folder),
        "a scope with no chip on this console was marked"
    );
    assert_eq!(
        view.scope(),
        Some(Scope::MySets),
        "the refusal moved the mark anyway"
    );

    assert!(view.select_scope(Scope::Presets));
    assert_eq!(view.scope(), Some(Scope::Presets));
    assert!(
        !view.select_scope(Scope::Presets),
        "marking the scope that is already marked moved something"
    );
}

// ---------------------------------------------------------------------------
// The load route: a cursor, a letter, and no pointer
// ---------------------------------------------------------------------------

/// Returns row indices of filled rectangles matching the bay's row boxes.
fn washed(shapes: &[egui::Shape], bay: &LibraryBay) -> Vec<usize> {
    (0..bay.rows)
        .filter(|index| {
            let row = bay.row(*index);
            shapes.iter().any(|shape| match shape {
                egui::Shape::Rect(at) => {
                    near(at.rect.min.x, row.min.x)
                        && near(at.rect.min.y, row.min.y)
                        && near(at.rect.width(), row.width())
                        && near(at.rect.height(), row.height())
                }
                _ => false,
            })
        })
        .collect()
}

/// Verifies the cursor is drawn as a wash on the active row and moves with the pointer.
#[test]
fn the_cursor_is_one_row_and_it_is_the_row_it_is_on() {
    let (mut view, mut panel) = showing_mock();
    let bay = bay(&panel);
    let list = bay.list;
    assert!(
        bay.rows >= 3,
        "the bay listed {} rows at the plausible console, which is too few to walk",
        bay.rows
    );

    let drawn = shapes_inside(&mut view, &mut panel, list);
    assert_eq!(
        washed(&drawn, &bay),
        vec![0],
        "a fresh console draws the cursor somewhere other than the first row, or on more than \
         one of them"
    );

    assert!(
        view.walk(2, bay.drawn()),
        "the cursor did not move two rows"
    );
    let drawn = shapes_inside(&mut view, &mut panel, list);
    assert_eq!(
        washed(&drawn, &bay),
        vec![2],
        "the cursor moved and the wash stayed where it was"
    );

    assert!(view.walk(-2, bay.drawn()), "the cursor did not move back");
    let drawn = shapes_inside(&mut view, &mut panel, list);
    assert_eq!(washed(&drawn, &bay), vec![0], "the walk back drew nothing");
}

/// Verifies the cursor is bounded by the displayed rows and does not wrap.
#[test]
fn the_cursor_stays_inside_the_rows_that_are_listed() {
    let mut view = View::new(Room::Day);
    view.library = mock();

    // A bay with room for two of the five.
    assert!(
        !view.walk(-1, 0..2),
        "the cursor walked above the first row"
    );
    assert_eq!(view.cursor_row(), 0);
    assert!(view.walk(1, 0..2));
    assert_eq!(view.cursor_row(), 1);
    assert!(
        !view.walk(1, 0..2),
        "the cursor left the two rows the bay listed"
    );
    assert_eq!(view.cursor_row(), 1, "and it wrapped instead of stopping");

    // A bay with room for all five, and a step past the end of the store.
    assert!(view.walk(99, 0..5));
    assert_eq!(view.cursor_row(), 4, "a long step left the listing");
    assert!(!view.walk(1, 0..5));

    // And no listing at all is no cursor to move: a console with no store
    // behind it, which is every other test in this crate.
    let mut empty = View::new(Room::Day);
    assert!(
        !empty.walk(1, 0..3),
        "a console with no store moved a cursor"
    );
    assert_eq!(empty.cursor_row(), 0);
}

/// Verifies the foot indicates target deck from pulldown rather than selection (ADR-0305).
#[test]
fn the_foot_says_which_deck_a_press_would_land_on() {
    let (mut view, mut panel) = showing_mock();
    let bay = bay(&panel);
    // Four strips, so all four decks can be aimed at. What a strip *reads* is
    // not this bay's business — only that there is one.
    view.mixer = std::iter::repeat_with(strip).take(4).collect();

    for deck in 0..4u8 {
        assert!(
            view.aim_at(deck) || deck == 0,
            "deck {deck} could not be aimed at with four strips"
        );
        let want = DECK_LETTERS[usize::from(deck)];
        let drawn = shapes_inside(&mut view, &mut panel, bay.foot);
        assert!(
            drawn.iter().any(|shape| matches!(
                shape,
                egui::Shape::Text(at) if at.galley.text() == want
            )),
            "the load is aimed at deck {deck} and the foot does not read `{want}`: {drawn:#?}"
        );
        // **And the word is still beside it**, so a letter drawn alone in an
        // empty foot cannot pass this.
        assert!(
            drawn.iter().any(|shape| matches!(
                shape,
                egui::Shape::Text(at) if at.galley.text() == "load"
            )),
            "the foot does not say `load`: {drawn:#?}"
        );
    }

    // **The other mark, moved on its own.** The keys go to deck C and the load
    // stays aimed at deck A, which is the state the readout this replaced
    // could not be in: the letter must not follow the ring.
    assert!(view.aim_at(0), "the load did not come back to deck A");
    assert!(view.select(2), "the selection did not move to deck C");
    let drawn = shapes_inside(&mut view, &mut panel, bay.foot);
    assert!(
        drawn.iter().any(|shape| matches!(
            shape,
            egui::Shape::Text(at) if at.galley.text() == "A"
        )),
        "the load is aimed at deck A and the foot stopped saying so: {drawn:#?}"
    );
    assert!(
        !drawn.iter().any(|shape| matches!(
            shape,
            egui::Shape::Text(at) if at.galley.text() == "C"
        )),
        "the foot's letter followed the deck selection, which is the readout ADR-0305 replaced: \
         {drawn:#?}"
    );
}

/// Verifies foot arrow is drawn via path rather than U+2192 glyph to avoid tofu rendering.
#[test]
fn the_foots_arrow_is_drawn_rather_than_typed() {
    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let at = bay.load(&ctx, AIMED);

    assert!(
        at.text.max.x <= at.arrow.min.x && at.arrow.max.x <= at.letter.min.x,
        "the arrow at {:?} is not between the word at {:?} and the letter at {:?}",
        at.arrow,
        at.text,
        at.letter
    );

    let drawn = shapes_inside(&mut view, &mut panel, bay.foot);
    for shape in &drawn {
        if let egui::Shape::Text(text) = shape {
            assert!(
                !text.galley.text().contains('\u{2192}'),
                "the foot types U+2192 in `{}`, which the default face draws as a tofu",
                text.galley.text()
            );
        }
    }
    assert!(
        drawn.iter().any(|shape| matches!(
            shape,
            egui::Shape::Path(path) if path.points.len() == 3
                && at.arrow.expand(1.0).contains_rect(path.visual_bounding_rect())
        )),
        "no triangle is painted in the arrow's box at {:?}: {drawn:#?}",
        at.arrow
    );
}

/// Verifies that clicking load targets the deck specified by the pulldown (ADR-0305, P-0083).
#[test]
fn a_press_on_load_asks_for_the_deck_the_pulldown_names() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(4).collect();
    assert!(view.select(2), "the keys did not go to deck C");
    assert!(view.aim_at(1), "the load was not aimed at deck B");

    let at = view.target();
    let load = bay.load(&ctx, at);
    let probe = Point::new(load.button.center().x, load.button.center().y);
    assert_eq!(
        bay.aim(
            &ctx,
            viewport(&panel),
            at,
            listed(&view.library),
            view.cursor_row(),
            probe
        ),
        Some(Aim::Load(Operation::LoadSet {
            deck: 1,
            set: "drift_night".to_owned(),
        })),
        "the press did not ask to load the cursor's Set onto the pulldown's deck"
    );
    assert_eq!(
        view.selection(),
        2,
        "asking for the load moved the deck selection"
    );

    // **A library that lists nothing has no Set to load**, and the button is
    // still drawn because the foot is what the count is in.
    let empty =
        library(panel.layout(), SCOPES, &[], None, None, 0.0).expect("the bay draws its foot");
    let load = empty.load(&ctx, at);
    assert_eq!(
        empty.aim(
            &ctx,
            viewport(&panel),
            at,
            Rows::NONE,
            0,
            Point::new(load.button.center().x, load.button.center().y)
        ),
        Some(Aim::NoSet),
        "a press on `load` over a listing with nothing in it asked for a Set with no name"
    );
}

/// Verifies pulldown selection names target deck without moving selection or cursor (ADR-0305).
#[test]
fn a_pick_in_the_pulldown_names_a_deck_and_asks_for_nothing() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(4).collect();

    // The capsule, with the list shut.
    let shut = view.target();
    let load = bay.load(&ctx, shut);
    assert_eq!(load.rows, 0, "a list is drawn under a shut pulldown");
    assert_eq!(load.list(viewport(&panel)), None);
    assert_eq!(
        bay.aim(
            &ctx,
            viewport(&panel),
            shut,
            listed(&mock()),
            0,
            Point::new(load.deck.center().x, load.deck.center().y)
        ),
        Some(Aim::Open),
        "a press on the pulldown did not put its list down"
    );

    assert!(view.open_target(), "the list did not come down");
    let open = view.target();
    let load = bay.load(&ctx, open);
    let card = load.list(viewport(&panel)).expect("the list is down");
    assert!(
        card.max.y <= load.deck.min.y,
        "the card at {card:?} hangs down over the bay below rather than up over this bay's own \
         list, which is what a foot has room for"
    );
    assert!(
        viewport(&panel).contains_rect(card),
        "the card at {card:?} is outside the viewport at {:?}",
        viewport(&panel)
    );
    assert_eq!(load.rows, 4, "the list is not one row per strip");

    let row = load.row(card, 2);
    assert_eq!(
        bay.aim(
            &ctx,
            viewport(&panel),
            open,
            listed(&mock()),
            0,
            Point::new(row.center().x, row.center().y)
        ),
        Some(Aim::Deck(2)),
        "a press on the third row did not name deck C"
    );

    // And performing it moves this bay's mark and nothing else.
    assert!(view.aim_at(2), "the pick did not move the target");
    assert_eq!(view.target_deck(), 2);
    assert_eq!(
        view.selection(),
        0,
        "a pick in the pulldown moved the deck selection"
    );
    assert_eq!(view.cursor_row(), 0, "a pick moved the library cursor");
    assert!(!view.target_open(), "the list stayed down after a pick");
}

/// Verifies pulldown offers only decks with mixer strips and refuses open if empty.
#[test]
fn the_pulldown_offers_the_decks_the_mixer_is_drawing_and_no_others() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(3).collect();

    assert!(view.open_target(), "the list did not come down");
    let load = bay.load(&ctx, view.target());
    assert_eq!(
        load.rows, 3,
        "the list offers {} decks against three strips",
        load.rows
    );
    assert!(
        !view.aim_at(3),
        "the load was aimed at deck D, which the mixer is drawing no strip for"
    );
    assert_eq!(view.target_deck(), 0, "a refused pick moved the target");
    assert!(
        view.target_open(),
        "a refused pick put the list away, so the refusal reads as a pick"
    );

    // **A console with no deck behind it**, which is every other test in this
    // file: there is nothing to offer, so there is no list to put down.
    let mut bare = View::new(Room::Day);
    bare.library = mock();
    bare.scopes = Scope::ALL.to_vec();
    assert!(
        !bare.open_target(),
        "a list came down over a console the mixer is drawing nothing for"
    );
    assert_eq!(bay.load(&ctx, bare.target()).rows, 0);
    assert_eq!(bay.load(&ctx, bare.target()).list(viewport(&panel)), None);
}

/// Verifies that while target pulldown is open, all presses are captured or dismiss it.
#[test]
fn while_the_list_is_down_every_press_is_part_of_that_gesture() {
    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(4).collect();
    assert!(view.open_target(), "the list did not come down");

    let at = view.target();
    let load = bay.load(&ctx, at);
    let card = load.list(viewport(&panel)).expect("the list is down");
    // A row of the card, a point on the card's own padding, and a point far
    // away from the bay altogether.
    let elsewhere = bay.row(0).center();
    assert!(
        !card.contains(elsewhere),
        "the point off the card is on it, so this test measures nothing"
    );
    for probe in [
        load.row(card, 1).center(),
        egui::pos2(card.center().x, card.min.y + size::LIB_LIST_PAD * 0.5),
        elsewhere,
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(probe.x, probe.y)),
            Claim::Panel,
            "the console gave `egui` a press at {probe:?} with the deck list down"
        );
    }

    // And what the two that are not a row ask for is the dismissal.
    for probe in [
        egui::pos2(card.center().x, card.min.y + size::LIB_LIST_PAD * 0.5),
        elsewhere,
    ] {
        assert_eq!(
            bay.aim(
                &ctx,
                viewport(&panel),
                at,
                listed(&mock()),
                0,
                Point::new(probe.x, probe.y)
            ),
            Some(Aim::Shut),
            "a press at {probe:?} with the list down did not take it away"
        );
    }
    assert!(view.shut_target(), "there was no list down to take away");
    assert!(
        !view.shut_target(),
        "shutting nothing said it shut something"
    );
}
