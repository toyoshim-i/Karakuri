use super::library_common::*;

// ---------------------------------------------------------------------------
// The scope row: four questions, one of them marked
// ---------------------------------------------------------------------------

/// The scope row draws the chips it was handed, in the order it was handed them
/// — which is the bay's own row: `all`, `my sets`, `presets`, `folder`,
/// `history`.
///
/// It is the mock's row with its first chip renamed and moved, which is
/// [ADR-0299](../../../docs/adr/0299-my-sets-is-the-starred-subset-and-the-star-is-kept-beside-the-sets.md):
/// `my sets` is the starred subset now, so the chip that lists everything the
/// store holds is `all` and it comes first, because it is the listing the other
/// three are questions about.
///
/// `history` is last and is not a library of Sets (ADR-0308): its rows are the
/// versions of the Set the load pulldown's deck is running, which is why it
/// sits after the four rather than among them.
///
/// The `+` the mock draws after them is not one of them, and that is asserted
/// rather than left to a reader counting the chips: it is the arena's own gap
/// drawn a fifth time, and a chip for it would be a control over adding a
/// region.
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

/// One chip is marked and it is the one the scope pointer is on, asserted by
/// drawing the bay and finding the wash.
///
/// The mark follows the pointer rather than being painted at a fixed chip,
/// which is the failure a first-chip default hides completely — so the pointer
/// is stepped the whole way round and the wash is read off the frame each time.
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

/// Every filled rectangle painted at one of the bay's own row boxes, as the row
/// index it landed on.
///
/// The cursor's mark is a wash and nothing else — `.lib-row.cursor` sets a
/// `background` and a `color`, and no rule, caret or chevron — so a row that is
/// not under it paints no rectangle of its own at all. That is what makes
/// counting them the whole assertion: one washed row, and it is the cursor's.
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

/// The cursor is one row and the row is the one it is on, asserted by drawing
/// the bay and finding the wash.
///
/// The mock puts `.lib-row.cursor` on the first row and this console starts
/// there, so the untouched case is the mock's own picture. What the walk then
/// proves is that the mark follows the pointer rather than being painted at a
/// fixed row — the failure a first-row default hides completely.
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

/// The cursor is held inside the rows the bay drew, and it does not wrap.
///
/// This bay has no scroll position, so the Sets an operator can reach are the
/// ones the foot counts as listed — a cursor past them would sit on a row
/// nobody can see, under a pill saying a press will load it. And a walk that
/// wrapped would jump the length of the list on one press of a key somebody is
/// leaning on.
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

/// The foot says where a press would land, and the letter follows the pulldown
/// rather than the deck selection.
///
/// `console.html`: *"The letter in the pulldown is the whole warning … What the
/// control owes instead is to say where it lands before the press."* So this
/// asserts what is painted rather than a rectangle, and asserts it again after
/// the mark moves — a foot that read `A` whatever was aimed at would pass the
/// first half and be a lie for the other three decks.
///
/// And it asserts which of the two marks the letter is, which is the whole of
/// ADR-0305: the letter used to be [`View::selection`], so a test that only
/// walked one mark would pass against the readout this replaced. The second
/// half moves the *selection* with the target standing still and reads the foot
/// again.
///
/// The letter is its own galley, because the arrow beside it is drawn rather
/// than typed — see [`the_foots_arrow_is_drawn_rather_than_typed`].
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

/// The foot's arrow is drawn rather than typed, because `egui`'s default face
/// has no U+2192.
///
/// The pill was `"load \u{2192} "` with the deck's letter appended, and the
/// panel drew `load □ A`: a readout of *where a press would land* with a tofu
/// where the arrow was. `CHEVRON_W` three bays along records the answer for
/// this whole class of question — whether a glyph is in the default face has no
/// good answer, so the mark is drawn — and this is that answer applied here.
///
/// # What it asserts
///
/// 1. Nothing painted in the foot carries U+2192, which is the defect
///    itself and is asserted over every galley rather than over the
///    constant: a character typed back into the word would fail here.
/// 2. A triangle is painted in the arrow's box, so the mark did not simply
///    go away — a pill reading `load A` says nothing about where the letter
///    stands to the word.
/// 3. The box is between the word and the letter, which is what makes the
///    three one reading rather than a mark parked at one end.
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

/// A pick in the pulldown names a deck, asks for nothing, and moves neither the
/// ring nor the cursor.
///
/// Three claims because they are the three the control owes: the capsule puts
/// the list down, a row of that list is `Aim::Deck` and no `Operation` at all,
/// and performing it leaves [`View::selection`] where it was. The third is the
/// one a reader will doubt — *surely picking a deck selects it* — and it is
/// exactly what ADR-0305 refused: `Operation::SelectDeck` moves the keys, and
/// this mark is the one that does not.
///
/// The list goes away with the pick, which is the gesture ending: nothing is
/// emitted, so there is no host arm to end it in, and a card left down would go
/// on claiming every press on the console (`input::claim`'s rule 2).
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

/// The pulldown offers the decks the mixer is drawing strips for, and no others
/// — and a console with no strip at all cannot put a list down.
///
/// `console.html`: *"A deck the mixer is drawing no strip for is not in the
/// list, which is the count `0`–`3` are refused on"*. So this is
/// [`View::select`]'s own refusal read a second time, asserted in both
/// directions: three strips list three decks, and deck D is turned down rather
/// than clamped to the last one there is.
///
/// The empty case is the one that would bite, and it is why `open_target`
/// refuses: a card with no rows in it offers nothing to pick and nothing to
/// leave by, and rule 2 would hand it every press on the console until a second
/// press shut it.
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

/// While the list is down, every press on the console is part of that gesture.
///
/// `input::claim`'s rule 2, which the two cards in the transport row are
/// already under: the card is drawn over this bay's own rows, so a press inside
/// it belongs to the card and a press anywhere else is the dismissal. Both
/// halves are asserted, because a rule that only claimed the card would leave
/// the first press outside it doing whatever it does the rest of the time —
/// loading a deck, or moving a fader.
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
