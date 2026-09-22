use super::library_common::*;

// ---------------------------------------------------------------------------
// Where the rows and the foot are
// ---------------------------------------------------------------------------

/// The bay's furniture is the bay's rectangle and the mock's own boxes, and
/// every number here is read off `style.css` rather than off the panel.
///
/// `.lib-foot { padding: 5px 10px; font-size: 10px; border-top: 1px }` is a
/// 26-tall row along the bottom; `.scopes { padding: 7px 9px; border-bottom:
/// 1px }` is a 31.5-tall row under the bay head; `.lib-list { padding: 3px }`
/// is what is left between the two; and `.lib-row { padding: 3px 7px }` around
/// type at the console's 11px and `line-height: 1.5` is 22.5 each, stacked with
/// no gap because `.lib-list` states none.
#[test]
fn the_rows_and_the_foot_are_the_bays_own_geometry() {
    let panel = console(PLAUSIBLE);
    let region = to_egui(rect_of(panel.layout(), "library"));
    let bay = bay(&panel);

    // **The transcription itself, against the stylesheet.** Everything below
    // is a *relation* — the list starts one padding under the head — and every
    // one of them holds just as well with a padding transcribed wrong, so the
    // numbers the relations are stated in are asserted here as the literals
    // `style.css` has. This is the half of the bay that is checked against the
    // mock rather than against itself.
    assert!(
        near(size::LIB_LIST_PAD, 3.0),
        "`.lib-list` is `padding: 3px`"
    );
    assert!(
        near(size::LIB_ROW_PAD_X, 7.0),
        "`.lib-row` is `padding: 3px 7px`"
    );
    assert!(
        near(size::LIB_ROW_PAD_Y, 3.0),
        "`.lib-row` is `padding: 3px 7px`"
    );
    assert!(
        near(size::LIB_FOOT_PAD_X, 10.0),
        "`.lib-foot` is `padding: 5px 10px`"
    );
    assert!(
        near(size::LIB_FOOT_PAD_Y, 5.0),
        "`.lib-foot` is `padding: 5px 10px`"
    );
    assert!(
        near(size::LIB_FOOT_SIZE, 10.0),
        "`.lib-foot` is `font-size: 10px`"
    );
    assert!(
        near(size::SCOPES_PAD_X, 9.0) && near(size::SCOPES_PAD_Y, 7.0),
        "`.scopes` is `padding: 7px 9px`"
    );
    assert!(near(size::SCOPES_GAP, 4.0), "`.scopes` is `gap: 4px`");
    assert!(near(size::SCOPE_PAD_X, 8.0), "`.scope` is `padding: 0 8px`");
    // And the two boxes those numbers add up to, which is where every relation
    // below is stated in: a row is 3 + 16.5 + 3 and the foot is 5 + 15 + 5 and
    // its one-pixel rule.
    assert!(
        near(size::LIB_ROW_H, 22.5),
        "a `.lib-row` is {} tall and the mock's is 3 + 11 * 1.5 + 3",
        size::LIB_ROW_H
    );
    assert!(
        near(size::LIB_FOOT_H, 26.0),
        "the `.lib-foot` is {} tall and the mock's is 5 + 10 * 1.5 + 5 + 1",
        size::LIB_FOOT_H
    );
    // And the third box, which is the one this bay grew: a chip is type at
    // `BASE` with no border of its own, and the row is that inside `.scopes`'
    // padding with its own rule under it.
    assert!(
        near(size::SCOPE_H, 16.5),
        "a `.scope` is {} tall and the mock's is 11 * 1.5, with no border to count",
        size::SCOPE_H
    );
    assert!(
        near(size::SCOPES_H, 31.5),
        "the `.scopes` row is {} tall and the mock's is 7 + 16.5 + 7 + 1",
        size::SCOPES_H
    );

    // The foot: along the bottom edge of the bay, its own height, the bay's
    // full width.
    assert!(
        near(bay.foot.max.y, region.max.y) && near(bay.foot.height(), size::LIB_FOOT_H),
        "the foot is {:?} and the bay ends at {}",
        bay.foot,
        region.max.y
    );
    assert!(
        near(bay.foot.min.x, region.min.x) && near(bay.foot.max.x, region.max.x),
        "the foot does not span the bay: {:?} in {region:?}",
        bay.foot
    );

    // The scope row: under the bay head, its own height, the bay's full width.
    let scopes = bay.scopes.expect("the bay was handed four scopes");
    assert!(
        near(scopes.min.y, region.min.y + size::HEAD_H) && near(scopes.height(), size::SCOPES_H),
        "the scope row is {scopes:?} and the head below {} leaves {}",
        region.min.y,
        region.min.y + size::HEAD_H
    );
    assert!(
        near(scopes.min.x, region.min.x) && near(scopes.max.x, region.max.x),
        "the scope row does not span the bay: {scopes:?} in {region:?}"
    );

    // The filter row: under the scope row, its own height, the bay's full
    // width — and the field taking what is left of it after the padding, which
    // is `.field`'s `flex: 1` with nothing beside it (ADR-0338).
    let filters = bay.filters.expect("the bay draws its filter row");
    assert!(
        near(filters.min.y, scopes.max.y) && near(filters.height(), size::LIB_FILTERS_H),
        "the filter row is {filters:?} and the scope row ends at {}",
        scopes.max.y
    );
    assert!(
        near(filters.min.x, region.min.x) && near(filters.max.x, region.max.x),
        "the filter row does not span the bay: {filters:?} in {region:?}"
    );
    let holds = bay.field(Field::Holds).expect("the `holds` field");
    assert!(
        near(holds.height(), size::FIELD_H),
        "the field is {} tall",
        holds.height()
    );
    assert!(
        near(holds.min.x, filters.min.x + size::LIB_FILTERS_PAD_X)
            && near(holds.max.x, filters.max.x - size::LIB_FILTERS_PAD_X),
        "the field is {holds:?} in a row spanning {} to {}",
        filters.min.x,
        filters.max.x
    );

    // The kind row: under the filter row, its own height, the bay's full
    // width, and six chips from its left padding each as wide as the word in
    // it (ADR-0338).
    let kinds = bay.kinds.expect("the bay draws its kind row");
    assert!(
        near(kinds.min.y, filters.max.y) && near(kinds.height(), size::LIB_KINDS_H),
        "the kind row is {kinds:?} and the filter row ends at {}",
        filters.max.y
    );
    assert!(
        near(kinds.min.x, region.min.x) && near(kinds.max.x, region.max.x),
        "the kind row does not span the bay: {kinds:?} in {region:?}"
    );
    let chips: Vec<(KindChip, egui::Rect)> = bay.kind_chips(&drawn_once()).collect();
    assert_eq!(
        chips.len(),
        KindChip::ALL.len(),
        "the row draws {} of the six chips",
        chips.len()
    );
    assert!(
        near(chips[0].1.min.x, kinds.min.x + size::LIB_KINDS_PAD_X)
            && near(chips[0].1.min.y, kinds.min.y + size::LIB_KINDS_PAD_Y)
            && near(chips[0].1.height(), size::KIND_H),
        "the first chip is {:?} in a row starting at {:?}",
        chips[0].1,
        kinds.min
    );
    for pair in chips.windows(2) {
        assert!(
            near(pair[1].1.min.x - pair[0].1.max.x, size::LIB_KINDS_GAP),
            "{:?} and {:?} are not one gap apart",
            pair[0].0,
            pair[1].0
        );
    }
    assert!(
        near(holds.min.y, filters.min.y + size::LIB_FILTERS_PAD_Y),
        "the fields sit at {} in a row starting at {}",
        holds.min.y,
        filters.min.y
    );

    // The list: under the kind row, inside `.lib-list`'s padding on all four
    // sides, and up to the foot.
    assert!(
        near(bay.list.min.y, kinds.max.y + size::LIB_LIST_PAD),
        "the list starts at {} and the kind row ends at {}",
        bay.list.min.y,
        kinds.max.y
    );
    assert!(
        near(bay.list.min.x, region.min.x + size::LIB_LIST_PAD)
            && near(bay.list.max.x, region.max.x - size::LIB_LIST_PAD),
        "the list is {:?} and the padding leaves {} to {}",
        bay.list,
        region.min.x + size::LIB_LIST_PAD,
        region.max.x - size::LIB_LIST_PAD
    );
    assert!(
        near(bay.list.max.y, bay.foot.min.y - size::LIB_LIST_PAD),
        "the list ends at {} and the foot starts at {}",
        bay.list.max.y,
        bay.foot.min.y
    );

    // The rows: the width of the list, one `LIB_ROW_H` apart, from its top.
    for index in 0..bay.rows {
        let row = bay.row(index);
        assert!(
            near(row.min.y, bay.list.min.y + size::LIB_ROW_H * index as f32),
            "row {index} is at {} and the stride puts it at {}",
            row.min.y,
            bay.list.min.y + size::LIB_ROW_H * index as f32
        );
        assert!(
            near(row.height(), size::LIB_ROW_H) && near(row.width(), bay.list.width()),
            "row {index} is {:?} and the list is {:?}",
            row,
            bay.list
        );
    }
}

/// The rows tile the list and never overlap, which is the claim a stride makes
/// and the one a wrong stride breaks: consecutive rows meet exactly, and the
/// last of them is inside the list.
#[test]
fn the_rows_tile_the_list_and_stay_inside_it() {
    let panel = console(PLAUSIBLE);
    let bay = bay(&panel);
    assert!(bay.rows > 1, "one row tiles nothing: {} rows", bay.rows);
    for index in 1..bay.rows {
        assert!(
            near(bay.row(index).min.y, bay.row(index - 1).max.y),
            "row {index} starts at {} and row {} ended at {}",
            bay.row(index).min.y,
            index - 1,
            bay.row(index - 1).max.y
        );
    }
    let last = bay.row(bay.rows - 1);
    assert!(
        bay.list.contains_rect(last),
        "the last row {last:?} is outside the list {:?}",
        bay.list
    );

    // **And where the list is full, one more would not have fitted** — which
    // is what makes `rows` a count of what fits rather than a count somebody
    // chose. Asked of a store with more in it than the bay can hold, because
    // with five names the bay stops at five for the other reason.
    let many: Vec<String> = (0..200).map(|n| format!("set_{n:03}")).collect();
    let full =
        library(panel.layout(), SCOPES, &many, None, None, 0.0).expect("the bay lists its rows");
    assert!(full.rows < full.total, "200 names all fitted");
    let over = full.row(full.rows);
    assert!(
        !full.list.contains_rect(over),
        "row {} at {over:?} would have fitted in {:?} and was not drawn",
        full.rows,
        full.list
    );
}

/// Verifies that the footer displays the count of visible items versus total items.
#[test]
fn the_foot_says_how_many_are_listed_of_how_many_there_are() {
    let names = mock();

    // A window with room for all five.
    let tall = console(PLAUSIBLE);
    let bay =
        library(tall.layout(), SCOPES, &names, None, None, 0.0).expect("the bay lists its rows");
    assert_eq!(bay.total, names.len(), "the total is not the store's");
    assert_eq!(
        bay.rows,
        names.len(),
        "a 1080-tall window has room for {} rows and listed {}",
        names.len(),
        bay.rows
    );
    assert_eq!(bay.count(), "5 of 5");

    // And a store with more in it than the bay can show: the total follows the
    // store and the count stops at what fits.
    let many: Vec<String> = (0..200).map(|n| format!("set_{n:03}")).collect();
    let bay =
        library(tall.layout(), SCOPES, &many, None, None, 0.0).expect("the bay lists its rows");
    assert_eq!(bay.total, 200, "the total is not the store's");
    assert!(
        bay.rows < 200,
        "a 1080-tall window listed all 200 of them, at {} a row",
        size::LIB_ROW_H
    );
    assert_eq!(bay.count(), format!("{} of 200", bay.rows));

    // The narrowest window the arrangement is claimed to work at fits fewer,
    // and the number is the library region's own height read the same way.
    let small = console(SMALLEST);
    let bay =
        library(small.layout(), SCOPES, &many, None, None, 0.0).expect("the bay lists its rows");
    let region = to_egui(rect_of(small.layout(), "library"));
    let room = region.height()
        - size::HEAD_H
        - size::SCOPES_H
        - size::LIB_FILTERS_H
        - size::LIB_KINDS_H
        - size::LIB_LIST_PAD * 2.0
        - size::LIB_FOOT_H;
    assert_eq!(
        bay.rows,
        (room / size::LIB_ROW_H).floor() as usize,
        "the bay is {} tall and listed {} rows",
        region.height(),
        bay.rows
    );
}

// ---------------------------------------------------------------------------
// No store behind the console
// ---------------------------------------------------------------------------

/// With no store behind the console the bay is a card and a head, and that is
/// asserted by drawing it.
///
/// `View::library` is empty in every test in this crate and in the whole of
/// `cargo test -p karakuri-console`, which is a console with no library opened.
/// What it draws then is the card, the head and the head's rule — not an empty
/// list with `0 of 0` under it, which is a reading of a library nobody opened,
/// and not a row of dashes, which is the same invention with a different glyph.
///
/// So this counts what lands inside the bay on a frame drawn each way, and the
/// difference is the whole claim.
#[test]
fn a_console_with_no_store_lists_nothing() {
    let mut panel = console(PLAUSIBLE);
    let region = to_egui(rect_of(panel.layout(), "library"));

    let mut view = View::new(Room::Day);
    assert!(
        view.library.is_empty(),
        "a fresh view has a store behind it before anybody listed one"
    );
    let empty = shapes_inside(&mut view, &mut panel, region);
    let bare = empty.len();

    // **And *bare* is a bay with no body**, which is asserted rather than
    // taken as whatever this bay happened to draw: the Master bay is the other
    // bay in the mock with a title, no pill and a grip, and it has nothing in
    // its body at all. The two draw the same shapes or the Library is drawing
    // something a library nobody opened does not have — an empty list, or a
    // rule and a `0 of 0` under one.
    //
    // **The Master bay draws one thing the Library does not, and it is
    // counted rather than the comparison being given up.** It carries the
    // class pill that opens the master effects to a model and the Library
    // carries none, because the Library is one of the four bays with no class
    // of its own — ADR-0235 leaves *"whether a bay that carries no class draws
    // the indicator at all"* open and the console draws nothing there. So the
    // pill's own shapes are counted where they are and taken off: a number
    // written down here instead would be this test's claim quietly becoming a
    // claim about how `egui` tessellates a capsule.
    let master = to_egui(rect_of(panel.layout(), "master"));
    let capsule = mcp_pill(
        &drawn_once(),
        panel.layout(),
        Class::MasterEffects,
        Open::CLOSED,
    )
    .expect("the Master bay draws its class pill")
    .pill;
    let pill = shapes_inside(&mut view, &mut panel, capsule).len();
    assert!(
        pill > 0,
        "nothing at all was drawn inside the Master bay's class pill"
    );
    let in_master = shapes_inside(&mut view, &mut panel, master).len();
    assert_eq!(
        bare,
        in_master - pill,
        "the Library bay draws {bare} shapes with no store behind it and the Master bay, \
         which has no body at all, draws {in_master} less the {pill} of its class pill"
    );

    // With the store's names, the same frame draws the rows and the foot.
    // Strictly more, and the assertion is the direction rather than a count of
    // shapes `egui` is free to tessellate differently.
    view.library = mock();
    let drawn = shapes_inside(&mut view, &mut panel, region);
    assert!(
        drawn.len() > bare,
        "the bay was handed {} names and drew {} shapes, the same as with no store behind it",
        view.library.len(),
        drawn.len()
    );

    // And taking the store away again empties it: the bay keeps nothing from
    // the frame it was drawn with.
    view.library = Vec::new();
    assert_eq!(
        shapes_inside(&mut view, &mut panel, region).len(),
        bare,
        "the store went away and the bay is still drawing what it last read"
    );
}

/// Nothing said about any library, no bay, asked of the derivation rather than
/// of the paint pass — the same rule `mixer` follows, stated where a caller can
/// reach it.
///
/// And the two ways of saying nothing are not one way. A console handed no
/// scopes and no rows has been told nothing at all, and the bay is its card and
/// its head. A console handed the four chips and no rows has been asked a
/// question whose answer is *nothing*, and the chips are drawn over an empty
/// list — which is `console.html`'s *"An empty tier is a library nobody has
/// filled rather than something gone wrong"* and is the whole of what the scope
/// row bought.
#[test]
fn nothing_said_about_any_library_is_no_listing() {
    let panel = console(PLAUSIBLE);
    assert_eq!(library(panel.layout(), &[], &[], None, None, 0.0), None);
    assert!(library(panel.layout(), &[], &mock(), None, None, 0.0).is_some());
    assert!(library(panel.layout(), SCOPES, &mock(), None, None, 0.0).is_some());

    let empty = library(panel.layout(), SCOPES, &[], None, None, 0.0).expect(
        "a scope with nothing in it is a question that has been answered, and the chips \
         saying which question it was are still drawn",
    );
    assert_eq!(empty.rows, 0, "a listing of nothing drew rows");
    assert_eq!(empty.count(), "0 of 0");
    assert!(
        empty.scopes.is_some(),
        "the scope row went away with the listing under it"
    );

    // And a console handed no scopes draws no scope row, whatever it lists:
    // the row is the chips it was given and never a band of empty card.
    assert_eq!(
        library(panel.layout(), &[], &mock(), None, None, 0.0).and_then(|bay| bay.scopes),
        None,
        "a console nobody told what libraries there are drew a scope row"
    );
}

/// The bay is drawn where the bay can hold it, and nowhere else —
/// `picture_rect`'s rule, stated on a listing.
#[test]
fn a_folded_or_soloed_or_short_bay_lists_nothing() {
    let names = mock();
    let mut layout = solved(PLAUSIBLE);
    assert!(library(&layout, SCOPES, &names, None, None, 0.0).is_some());

    layout.collapse(id_of(&layout, "library"));
    layout.solve();
    assert_eq!(
        library(&layout, SCOPES, &names, None, None, 0.0),
        None,
        "the library is folded away and its rows are still being drawn"
    );

    layout.expand(id_of(&layout, "library"));
    layout.solve();
    assert!(library(&layout, SCOPES, &names, None, None, 0.0).is_some());

    // A solo somewhere else takes the bay off the panel with it.
    layout.solo(id_of(&layout, "mixer"));
    layout.solve();
    assert_eq!(library(&layout, SCOPES, &names, None, None, 0.0), None);
    layout.unsolo();
    layout.solve();
    assert!(library(&layout, SCOPES, &names, None, None, 0.0).is_some());

    // **And a bay with no room for the foot and one row lists nothing**, which
    // is the same answer and not a special case.
    //
    // It takes a window under the arrangement's own minimum to reach: the
    // library declares a minimum of 158 and the solve honours it, so at every
    // window this console is claimed to work at there is room for three rows.
    // Below 632 the solve stops honouring minima and scales everything down
    // together (`common::SMALLEST` says so), and that is where a bay too short
    // for a row exists at all.
    let chrome = size::HEAD_H
        + size::SCOPES_H
        + size::LIB_FILTERS_H
        + size::LIB_KINDS_H
        + size::LIB_FOOT_H
        + size::LIB_LIST_PAD * 2.0;
    let short = solved(Rect {
        h: 160.0,
        ..SMALLEST
    });
    assert!(
        to_egui(rect_of(&short, "library")).height() < chrome + size::LIB_ROW_H,
        "the bay is {} tall, which is room for a row",
        to_egui(rect_of(&short, "library")).height()
    );
    assert_eq!(
        library(&short, SCOPES, &names, None, None, 0.0),
        None,
        "a bay with no room for one row listed some"
    );

    // Two hundred and fifteen pixels of window taller is one row, which is
    // what says the answer above is the room and not the window. It was a
    // hundred and twenty-five until the kind row landed: that band is one more
    // thing between the head and the list, and the bay is held at its declared
    // minimum until the window is tall enough to give it more, so the window
    // with room for exactly one row is that much taller again (ADR-0338).
    let barely = solved(Rect {
        h: 375.0,
        ..SMALLEST
    });
    let region = to_egui(rect_of(&barely, "library"));
    assert!(
        region.height() >= chrome + size::LIB_ROW_H
            && region.height() < chrome + size::LIB_ROW_H * 2.0,
        "the bay is {} tall, which is not room for exactly one row",
        region.height()
    );
    assert_eq!(
        library(&barely, SCOPES, &names, None, None, 0.0).map(|bay| bay.rows),
        Some(1),
        "a bay with room for exactly one row listed something else"
    );

    // **And a bay narrower than `.lib-list`'s own padding lists nothing**,
    // which is the same rule across the axis rather than down it. It takes a
    // window under the arrangement's minimum for the same reason: the left
    // pane declares 160 and the solve honours it above 990.
    let sliver = solved(Rect {
        w: 30.0,
        ..PLAUSIBLE
    });
    assert!(
        to_egui(rect_of(&sliver, "library")).width() < size::LIB_LIST_PAD * 2.0,
        "the bay is {} wide, which is room for a list",
        to_egui(rect_of(&sliver, "library")).width()
    );
    assert_eq!(
        library(&sliver, SCOPES, &names, None, None, 0.0),
        None,
        "a bay with no room for the list's own padding listed some"
    );
}
