use super::library_common::*;

// ---------------------------------------------------------------------------
// A row's own menu (ADR-0311)
// ---------------------------------------------------------------------------

/// Secondary clicks open the targeted row's context menu, while list margins and inert rows claim nothing.
#[test]
fn a_secondary_press_names_a_row_and_a_press_on_nothing_names_none() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(4).collect();
    assert!(bay.rows >= 4, "the mock's bay drew {} rows", bay.rows);

    let shut = view.menued();
    assert_eq!(shut.row, None, "a console opens with a menu already down");
    for row in 0..bay.rows.min(view.sets().len()) {
        let at = bay.row(row).center();
        assert_eq!(
            bay.menu_ask(
                &ctx,
                viewport(&panel),
                shut,
                view.rows(),
                Point::new(at.x, at.y)
            ),
            Some(Picked::Open(row)),
            "a secondary press on row {row} did not name it"
        );
    }

    // The list's own ground, below the last row the bay drew.
    let ground = Point::new(bay.list.center().x, bay.list.max.y - 1.0);
    assert!(
        (0..bay.rows).all(|row| !bay.row(row).contains(egui::pos2(ground.x, ground.y))),
        "the point below the last row is on one, so this test measures nothing"
    );
    assert_eq!(
        bay.menu_ask(&ctx, viewport(&panel), shut, view.rows(), ground),
        None,
        "a press on the list's own ground put a menu down"
    );

    // And a `history` row, which is a version and not a Set: `View::sets` is
    // empty under that scope, so there is nothing for an item to name.
    assert!(view.select_scope(Scope::History));
    let at = bay.row(0).center();
    assert_eq!(
        bay.menu_ask(
            &ctx,
            viewport(&panel),
            view.menued(),
            view.rows(),
            Point::new(at.x, at.y)
        ),
        None,
        "a menu came down on a row that is a version rather than a Set"
    );
}

/// Context menu contains one load item per active mixer deck and a universal export option.
#[test]
fn the_row_menu_offers_the_drawn_decks_and_always_the_send() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(3).collect();
    assert!(view.open_menu(3), "the menu did not come down on row 3");

    let menu = bay
        .menu(&ctx, viewport(&panel), view.menued())
        .expect("the menu is down");
    assert_eq!(
        menu.loads, 3,
        "the card carries {} loads against three strips",
        menu.loads
    );
    let row = bay.row(3);
    assert!(
        menu.card.min.y >= row.max.y,
        "the card at {:?} hangs up over the rows above rather than down off the row it was opened \
         on at {row:?}",
        menu.card
    );
    assert!(
        viewport(&panel).contains_rect(menu.card),
        "the card at {:?} is outside the viewport at {:?}",
        menu.card,
        viewport(&panel)
    );
    for deck in 0..3u8 {
        assert_eq!(
            menu.picked(near_centre(menu.load(usize::from(deck)))),
            Some(RowItem::Load(deck)),
            "the {deck}th item is not that deck's load"
        );
    }
    assert_eq!(
        menu.picked(near_centre(
            menu.save.expect("a Set row's menu carries a send")
        )),
        Some(RowItem::Save),
        "the last item is not the send"
    );
    // The separator is nothing: it names no item, and a press on it is the
    // dismissal rather than a pick.
    assert_eq!(
        menu.picked(near_centre(
            menu.rule.expect("a Set row's menu carries a separator")
        )),
        None,
        "the separator answered a press"
    );

    // Send option remains selectable even when mixer strips are absent.
    let mut bare = View::new(Room::Day);
    bare.library = mock();
    bare.scopes = Scope::ALL.to_vec();
    assert!(
        bare.open_menu(0),
        "a menu was refused on a console with no strip, where the send needs none"
    );
    let menu = bay
        .menu(&ctx, viewport(&panel), bare.menued())
        .expect("the menu is down");
    assert_eq!(menu.loads, 0, "a card with no strip offered a load");
    assert_eq!(
        menu.picked(near_centre(
            menu.save.expect("a Set row's menu carries a send")
        )),
        Some(RowItem::Save)
    );
}

/// Selecting a menu item preserves cursor position and active deck targets without unintended side effects.
#[test]
fn a_row_menu_pick_names_its_deck_and_its_own_row_and_moves_no_mark() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(4).collect();
    assert!(view.select(2), "the keys were not addressed to deck C");
    assert!(view.aim_at(1), "the load was not aimed at deck B");
    assert!(view.open_menu(3), "the menu did not come down on row 3");

    let menu = bay
        .menu(&ctx, viewport(&panel), view.menued())
        .expect("the menu is down");
    // Select Deck C to verify the item is resolved rather than defaulting to Deck A.
    let want = Operation::LoadSet {
        deck: 2,
        set: "night01".to_owned(),
    };
    assert_eq!(
        bay.menu_ask(
            &ctx,
            viewport(&panel),
            view.menued(),
            view.rows(),
            near_centre(menu.load(2))
        ),
        Some(Picked::Load(want)),
        "`Load to Slot C` did not ask for the fourth Set onto deck C"
    );

    // And performing it moves nothing else at all.
    assert!(view.shut_menu(), "there was no menu down to take away");
    assert_eq!(view.selection(), 2, "a pick moved the deck selection");
    assert_eq!(view.target_deck(), 1, "a pick moved the load's target");
    assert_eq!(view.cursor_row(), 0, "a pick moved the library cursor");
    assert!(!view.menu_open(), "the card stayed down after a pick");
}

/// Export menu item emits `TransferSet::Send` targeting the row where the menu was opened (ADR-0260).
#[test]
fn the_row_menus_send_names_the_row_it_was_opened_on() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(4).collect();
    assert!(view.open_menu(2), "the menu did not come down on row 2");

    let menu = bay
        .menu(&ctx, viewport(&panel), view.menued())
        .expect("the menu is down");
    assert_eq!(
        bay.menu_ask(
            &ctx,
            viewport(&panel),
            view.menued(),
            view.rows(),
            near_centre(menu.save.expect("a Set row's menu carries a send"))
        ),
        Some(Picked::Send(Operation::TransferSet {
            transfer: SetTransfer::Send {
                id: "glass_shell".to_owned()
            }
        })),
        "`Save as a kbset` did not ask to send the third Set"
    );
}

/// Open context menus capture all pointer events across the console to handle dismissal or selection.
#[test]
fn while_a_row_menu_is_down_every_press_is_part_of_that_gesture() {
    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(4).collect();
    assert!(view.open_menu(1), "the menu did not come down");

    let at = view.menued();
    let menu = bay
        .menu(&ctx, viewport(&panel), at)
        .expect("the menu is down");
    // An item, the card's own padding, the separator, and a point far away
    // from the bay altogether.
    let elsewhere = bay.row(0).center();
    assert!(
        !menu.card.contains(elsewhere),
        "the point off the card is on it, so this test measures nothing"
    );
    for probe in [
        near_centre(menu.load(2)),
        Point::new(
            menu.card.center().x,
            menu.card.min.y + size::LIB_LIST_PAD * 0.5,
        ),
        near_centre(menu.rule.expect("a Set row's menu carries a separator")),
        Point::new(elsewhere.x, elsewhere.y),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, probe),
            Claim::Panel,
            "the console gave `egui` a press at {probe:?} with a row menu down"
        );
    }

    // And what the three that are not an item ask for is the dismissal.
    for probe in [
        Point::new(
            menu.card.center().x,
            menu.card.min.y + size::LIB_LIST_PAD * 0.5,
        ),
        near_centre(menu.rule.expect("a Set row's menu carries a separator")),
        Point::new(elsewhere.x, elsewhere.y),
    ] {
        assert_eq!(
            bay.menu_ask(&ctx, viewport(&panel), at, view.rows(), probe),
            Some(Picked::Shut),
            "a press at {probe:?} with the menu down did not take it away"
        );
    }
    assert!(view.shut_menu(), "there was no menu down to take away");
    assert!(!view.shut_menu(), "shutting nothing said it shut something");
}

/// A point inside a rectangle, as this crate's own `Point`.
fn near_centre(at: egui::Rect) -> Point {
    Point::new(at.center().x, at.center().y)
}

// ---------------------------------------------------------------------------
// The bay scrolls (ADR-0307, ADR-0312)
// ---------------------------------------------------------------------------

/// A listing longer than any bay here can hold, so that there is something to
/// scroll to at all — a `PLAUSIBLE` console's Library bay holds about thirty
/// rows, so this is four times that. The names are `set000`..`set119` because
/// what matters about them is the order and the count.
fn long() -> Vec<String> {
    (0..120).map(|n| format!("set{n:03}")).collect()
}

/// The bay, laid out over a given listing at a given position.
fn bay_at(panel: &Panel, sets: &[String], open: Option<Opened<'_>>, scroll: f32) -> LibraryBay {
    library(panel.layout(), SCOPES, sets, open, None, scroll)
        .expect("the library bay lists its rows")
}

/// Scrolling with pointer wheel shifts visible row window and updates foot count readout.
#[test]
fn the_wheel_over_the_bay_scrolls_the_listing_and_the_foot_counts_it() {
    let mut panel = console(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    view.library = long();
    view.scopes = Scope::ALL.to_vec();
    let ctx = drawn_once();

    let before = bay_at(&panel, &view.library, None, view.library_scroll());
    assert_eq!(
        before.drawn().start,
        0,
        "an unscrolled bay is not at the top"
    );
    assert!(
        before.rows < before.total,
        "the listing fits, so this test has nothing to scroll"
    );
    assert_eq!(before.count(), format!("{} of 120", before.rows));

    // The wheel is aimed at the bay, and the bay is what it names — the whole
    // region, so a point over the scope chips turns the listing under them.
    for probe in [
        before.list.center(),
        before.scopes.expect("the chips are drawn").center(),
        before.foot.center(),
    ] {
        assert_eq!(
            wheeled(&mut panel, &view, Point::new(probe.x, probe.y)),
            Some(Turned::Library),
            "a wheel at {probe:?} is not this bay's"
        );
    }

    assert!(
        view.scroll_library_by(size::LIB_ROW_H * 3.0),
        "the wheel moved nothing"
    );
    let after = bay_at(&panel, &view.library, None, view.library_scroll());
    assert_eq!(
        after.drawn().start,
        3,
        "three rows of wheel did not take three rows off the top"
    );
    assert_eq!(
        after.rows, before.rows,
        "the same bay at the same size is showing a different number of whole rows"
    );
    assert_eq!(after.count(), format!("{} of 120", after.rows));
    // And the row under the top of the list is the row the position names.
    let top = after.row(after.drawn().start);
    assert!(
        near(top.min.y, after.list.min.y),
        "the first drawn row is at {} against a list starting at {}",
        top.min.y,
        after.list.min.y
    );
    let _ = ctx;
}

/// Partially visible rows clipped at viewport edges are rendered but excluded from whole-row count.
#[test]
fn a_row_cut_by_an_edge_is_drawn_and_is_not_counted() {
    let panel = console(PLAUSIBLE);
    let sets = long();

    // Scrolling by exact multiples of row height aligns the top row flush with the view.
    let flush = bay_at(&panel, &sets, None, size::LIB_ROW_H * 2.0);
    assert!(
        near(flush.row(flush.drawn().start).min.y, flush.list.min.y),
        "a bay scrolled two whole rows is not flush at the top"
    );

    let cut = bay_at(&panel, &sets, None, size::LIB_ROW_H * 2.5);
    assert!(
        cut.row(cut.drawn().start).min.y < cut.list.min.y,
        "half a row of scroll left the top row flush, so nothing is cut"
    );
    assert!(
        cut.drawn().len() > cut.rows,
        "a bay with a row hanging over its top edge draws {} rows and says it is showing {}, so \
         the foot is counting what is painted rather than what is whole",
        cut.drawn().len(),
        cut.rows
    );
    assert!(
        cut.rows < flush.rows || cut.drawn().len() > flush.drawn().len(),
        "half a row of scroll changed neither what is drawn nor what is counted"
    );
    assert_eq!(cut.count(), format!("{} of 120", cut.rows));
}

/// Viewport resize preserves library scroll offset without clamp reset (P-0082).
#[test]
fn a_library_position_survives_the_bay_growing_and_shrinking() {
    let short = console(SMALLEST);
    let tall = console(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    // Stored scroll offset exceeds short bay capacity to verify clamp persistence.
    view.library = (0..20).map(|n| format!("set{n:02}")).collect();
    view.scopes = Scope::ALL.to_vec();
    // Far past anything either bay can use, and it lands at the content —
    // which is the clamp `scroll_library_by` does keep.
    assert!(view.scroll_library_by(10_000.0));
    let stored = view.library_scroll();

    let before = bay_at(&short, &view.library, None, stored);
    assert!(
        before.scroll > 0.0,
        "the short bay is not scrolled, so this test has nothing to lose"
    );
    assert!(
        stored > before.scroll,
        "the stored position {stored} is not past what the short bay can use, so a clamp written \
         back would not show"
    );

    let grown = bay_at(&tall, &view.library, None, stored);
    assert_eq!(
        view.library_scroll(),
        stored,
        "growing the bay rewrote the stored position"
    );
    assert!(grown.scroll <= stored);

    let after = bay_at(&short, &view.library, None, view.library_scroll());
    assert_eq!(
        view.library_scroll(),
        stored,
        "shrinking the bay rewrote the stored position"
    );
    assert_eq!(
        after.scroll, before.scroll,
        "the bay came back to a different place than it left"
    );
    assert_eq!(
        after.drawn(),
        before.drawn(),
        "the same bay at the same size is drawing different rows"
    );
}

/// Hit-testing ignores portions of scrolled rows clipped outside visible bounds.
#[test]
fn a_press_above_the_list_reaches_no_row_even_where_one_is_drawn() {
    let panel = console(PLAUSIBLE);
    let sets = long();
    // Half a row, so the row at the top is cut by the list's own edge.
    let bay = bay_at(&panel, &sets, None, size::LIB_ROW_H * 4.5);
    let first = bay.drawn().start;
    let cut = bay.row(first);
    assert!(
        cut.min.y < bay.list.min.y && cut.max.y > bay.list.min.y,
        "the first drawn row at {cut:?} is not cut by the list's top edge at {}",
        bay.list.min.y
    );

    // The half of it that is above the list — inside the row's rectangle and
    // outside the list.
    let hidden = Point::new(cut.center().x, cut.min.y + 1.0);
    assert!(
        !bay.list.contains(egui::pos2(hidden.x, hidden.y)),
        "the probe is inside the list, so this test measures nothing"
    );
    assert_eq!(
        bay.take(listed(&sets), hidden),
        None,
        "a press above the list took a Set in hand"
    );
    assert_eq!(
        bay.land(&sets, AIMED, hidden),
        None,
        "a press above the list landed a version"
    );

    // And the half that is inside the list is that row, by the same call.
    let shown = Point::new(cut.center().x, bay.list.min.y + 1.0);
    assert_eq!(
        bay.take(listed(&sets), shown).map(|taken| taken.row),
        Some(first),
        "the visible half of the cut row was not taken in hand"
    );

    // A row scrolled entirely off the top is not drawn at all, so the walk
    // never reaches it — which is the same refusal one step earlier.
    let gone = bay.row(0);
    assert!(gone.max.y <= bay.list.min.y);
    assert_eq!(
        bay.take(listed(&sets), Point::new(gone.center().x, gone.center().y)),
        None,
        "a row nothing draws was taken in hand"
    );
}

/// Expanded reading block scrolls inline with rows and contributes to scroll clamping bounds.
#[test]
fn the_reading_scrolls_with_the_rows_and_is_in_what_bounds_the_scroll() {
    let panel = console(PLAUSIBLE);
    let sets = long();
    let block = reading();
    let open = Opened {
        at: 1,
        reading: &block,
    };

    let shut = bay_at(&panel, &sets, None, 0.0);
    let down = bay_at(&panel, &sets, Some(open), 0.0);
    assert!(
        down.content > shut.content,
        "a reading costs nothing in the content it is drawn inside"
    );
    let well = down.reading.expect("the reading is open").well;

    let scrolled = bay_at(&panel, &sets, Some(open), size::LIB_ROW_H);
    let moved = scrolled.reading.expect("the reading is open").well;
    assert!(
        near(well.min.y - moved.min.y, size::LIB_ROW_H),
        "one row of wheel moved the block by {} rather than by a row",
        well.min.y - moved.min.y
    );
    // And the rows under it moved with it, by the same distance.
    assert!(
        near(down.row(2).min.y - scrolled.row(2).min.y, size::LIB_ROW_H),
        "the block and the rows under it did not move together"
    );
}

/// Navigation cursor remains constrained to the currently visible window of rows (ADR-0307).
#[test]
fn the_cursor_is_held_inside_the_window_the_bay_is_drawing() {
    let panel = console(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    view.library = long();
    view.scopes = Scope::ALL.to_vec();
    assert!(view.scroll_library_by(size::LIB_ROW_H * 4.0));
    let bay = bay_at(&panel, &view.library, None, view.library_scroll());
    let drawn = bay.drawn();
    assert_eq!(drawn.start, 4);

    // Up from the top of the window goes nowhere, and down walks inside it.
    assert!(
        view.point_at(drawn.start),
        "the cursor did not move to the window"
    );
    assert!(
        !view.walk(-1, drawn.clone()),
        "the cursor walked above the first row the bay is drawing"
    );
    assert!(view.walk(1, drawn.clone()));
    assert_eq!(view.cursor_row(), drawn.start + 1);
    assert!(view.walk(99, drawn.clone()));
    assert_eq!(
        view.cursor_row(),
        drawn.end - 1,
        "the cursor walked past the last row the bay is drawing"
    );
}

/// Changing scope resets scroll offset and cursor to top of the new listing (P-0082).
#[test]
fn a_scope_press_puts_the_listing_back_at_the_top() {
    let mut view = View::new(Room::Day);
    view.library = long();
    view.scopes = Scope::ALL.to_vec();
    assert!(view.scroll_library_by(size::LIB_ROW_H * 6.0));
    assert!(view.library_scroll() > 0.0);
    assert!(view.walk(2, 0..10));
    assert!(view.cursor_row() > 0);

    assert!(view.select_scope(Scope::Presets), "the chip did not move");
    assert_eq!(
        view.library_scroll(),
        0.0,
        "a scope press left the bay scrolled into a listing it has never seen"
    );
    assert_eq!(view.cursor_row(), 0);

    // And pressing the chip that is already marked moves nothing, which is
    // what makes this a reset rather than a second clamp.
    assert!(view.scroll_library_by(size::LIB_ROW_H * 3.0));
    let held = view.library_scroll();
    assert!(!view.select_scope(Scope::Presets), "the chip moved twice");
    assert_eq!(
        view.library_scroll(),
        held,
        "a press on the scope already marked put the listing back at the top"
    );
}
