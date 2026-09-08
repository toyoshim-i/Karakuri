//! **The Library bay: a listing, a count, and nothing at all where there is no
//! store.**
//!
//! Five things, and the first two are why this file exists rather than a few
//! more assertions in `view.rs`:
//!
//! 1. **That a console with no store behind it draws nothing there** — not an
//!    empty list and not a `0 of 0` under one, either of which is a reading of
//!    a library nobody opened. It is asserted by drawing a frame and counting
//!    what landed in the bay, because *nothing is drawn* is a claim about the
//!    paint pass and not about a rectangle.
//! 2. **That the foot's two numbers are the two the mock's are** — how many
//!    rows are listed against how many the store holds — which is the whole of
//!    what the bay says about a library taller than its list.
//! 3. Where the rows and the foot are, derived from the bay's own geometry and
//!    the mock's boxes.
//! 4. **That nothing in it is a control**, stated rather than inferred from the
//!    absence of a hit test: `claim` gives every point of this bay to `egui`
//!    unless a boundary has it.
//! 5. That the names are the harness's and the console keeps no copy.
//!
//! None of it needs a window or a device, and unlike `transport.rs` and
//! `outputs.rs` most of it does not need `egui`'s fonts either: every box in
//! this bay is the full width of the list, so no rectangle here is the width
//! of the type in it. The two tests that draw a frame need a context all the
//! same, because painting type is what they count.

mod common;

use common::{drawn_once, id_of, near, rect_of, solved, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    library, mcp_pill, Field, Filters, LibraryBay, Published, Read, Reading, Scope, View,
    DECK_LETTERS, HOLDS_UNSET, LAYERS, LAYER_UNSET,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::gate::{Class, Open};
use karakuri_operation::Operation;

/// **The mock's own library, as names**: five Sets, in the order it draws
/// them.
///
/// The mock's rows carry a star and a time beside each name and this carries
/// neither, which is `view::library`'s own list of what is not drawn — a
/// favourite is a fact nothing in this workspace keeps, and the time has a
/// value and no spelling. What is left is what the store can name.
fn mock() -> Vec<String> {
    [
        "drift_night",
        "lattice_veil",
        "glass_shell",
        "night01",
        "strand_bloom",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

/// **The scopes a host with a store hands in**: the four the mock draws.
///
/// Every call here passes these rather than none, because a console handed no
/// scopes is a console nobody has told what libraries there are — which is
/// what `View::new` starts at, and what the two tests that draw a frame are
/// about.
const SCOPES: &[Scope] = &Scope::ALL;

/// A panel at a viewport, solved — the pair every test here starts from.
fn console(viewport: Rect) -> Panel {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    panel
}

/// The bay, laid out with the mock's names.
fn bay(panel: &Panel) -> LibraryBay {
    library(panel.layout(), SCOPES, &mock(), None, None).expect("the library bay lists its rows")
}

/// `egui`'s rectangle, from `karakuri_layout`'s.
fn to_egui(r: Rect) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h))
}

// ---------------------------------------------------------------------------
// Where the rows and the foot are
// ---------------------------------------------------------------------------

/// **The bay's furniture is the bay's rectangle and the mock's own boxes**,
/// and every number here is read off `style.css` rather than off the panel.
///
/// `.lib-foot { padding: 5px 10px; font-size: 10px; border-top: 1px }` is a
/// 26-tall row along the bottom; `.scopes { padding: 7px 9px; border-bottom:
/// 1px }` is a 31.5-tall row under the bay head; `.lib-list { padding: 3px }` is
/// what is left between the two; and `.lib-row { padding: 3px 7px }` around
/// type at the console's 11px and `line-height: 1.5` is 22.5 each, stacked
/// with no gap because `.lib-list` states none.
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
    // width — and the two fields sharing what is left of it after the padding
    // and the one gap, which is `.field`'s `flex: 1`.
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
    let layer = bay.field(Field::Layer).expect("the `layer` field");
    assert!(
        near(holds.width(), layer.width()) && near(holds.height(), size::FIELD_H),
        "the two fields are {} and {} wide at {} tall",
        holds.width(),
        layer.width(),
        holds.height()
    );
    assert!(
        near(holds.min.x, filters.min.x + size::LIB_FILTERS_PAD_X)
            && near(layer.max.x, filters.max.x - size::LIB_FILTERS_PAD_X)
            && near(layer.min.x - holds.max.x, size::LIB_FILTERS_GAP),
        "the fields are {holds:?} and {layer:?} in a row spanning {} to {}",
        filters.min.x,
        filters.max.x
    );
    assert!(
        near(holds.min.y, filters.min.y + size::LIB_FILTERS_PAD_Y),
        "the fields sit at {} in a row starting at {}",
        holds.min.y,
        filters.min.y
    );

    // The list: under the filter row, inside `.lib-list`'s padding on all four
    // sides, and up to the foot.
    assert!(
        near(bay.list.min.y, filters.max.y + size::LIB_LIST_PAD),
        "the list starts at {} and the filter row ends at {}",
        bay.list.min.y,
        filters.max.y
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

/// **The rows tile the list and never overlap**, which is the claim a stride
/// makes and the one a wrong stride breaks: consecutive rows meet exactly, and
/// the last of them is inside the list.
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
    let full = library(panel.layout(), SCOPES, &many, None, None).expect("the bay lists its rows");
    assert!(full.rows < full.total, "200 names all fitted");
    let over = full.row(full.rows);
    assert!(
        !full.list.contains_rect(over),
        "row {} at {over:?} would have fitted in {:?} and was not drawn",
        full.rows,
        full.list
    );
}

/// **The foot's number is how many are listed of how many there are**, which
/// is the mock's `5 of 27` read the mock's way.
///
/// The two halves come from different places on purpose — the total is what
/// the harness handed over and the count is what the bay had room for — so
/// this asks it at two window heights, one where every name fits and one where
/// they do not.
#[test]
fn the_foot_says_how_many_are_listed_of_how_many_there_are() {
    let names = mock();

    // A window with room for all five.
    let tall = console(PLAUSIBLE);
    let bay = library(tall.layout(), SCOPES, &names, None, None).expect("the bay lists its rows");
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
    let bay = library(tall.layout(), SCOPES, &many, None, None).expect("the bay lists its rows");
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
    let bay = library(small.layout(), SCOPES, &many, None, None).expect("the bay lists its rows");
    let region = to_egui(rect_of(small.layout(), "library"));
    let room = region.height()
        - size::HEAD_H
        - size::SCOPES_H
        - size::LIB_FILTERS_H
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

/// Every shape the console paints **wholly inside** `rect`, on one frame.
///
/// `transport.rs`'s own helper, and the reasoning is written out there:
/// containment rather than intersection so the panel's ground and the card's
/// drop shadow are not counted as things drawn in the bay.
fn shapes_inside(view: &mut View, panel: &mut Panel, rect: egui::Rect) -> Vec<egui::Shape> {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter(|clipped| {
            let bounds = clipped.shape.visual_bounding_rect();
            bounds.is_finite() && rect.contains_rect(bounds)
        })
        .map(|clipped| clipped.shape)
        .collect()
}

/// **With no store behind the console the bay is a card and a head, and that
/// is asserted by drawing it.**
///
/// `View::library` is empty in every test in this crate and in the whole of
/// `cargo test -p karakuri-console`, which is a console with no library
/// opened. What it draws then is the card, the head and the head's rule —
/// **not** an empty list with `0 of 0` under it, which is a reading of a
/// library nobody opened, and not a row of dashes, which is the same invention
/// with a different glyph.
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

/// **Nothing said about any library, no bay**, asked of the derivation rather
/// than of the paint pass — the same rule `mixer` follows, stated where a
/// caller can reach it.
///
/// **And the two ways of saying nothing are not one way.** A console handed no
/// scopes and no rows has been told nothing at all, and the bay is its card
/// and its head. A console handed the four chips and no rows has been asked a
/// question whose answer is *nothing*, and the chips are drawn over an empty
/// list — which is `console.html`'s *"An empty tier is a library nobody has
/// filled rather than something gone wrong"* and is the whole of what the
/// scope row bought.
#[test]
fn nothing_said_about_any_library_is_no_listing() {
    let panel = console(PLAUSIBLE);
    assert_eq!(library(panel.layout(), &[], &[], None, None), None);
    assert!(library(panel.layout(), &[], &mock(), None, None).is_some());
    assert!(library(panel.layout(), SCOPES, &mock(), None, None).is_some());

    let empty = library(panel.layout(), SCOPES, &[], None, None).expect(
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
        library(panel.layout(), &[], &mock(), None, None).and_then(|bay| bay.scopes),
        None,
        "a console nobody told what libraries there are drew a scope row"
    );
}

/// **The bay is drawn where the bay can hold it, and nowhere else** —
/// `picture_rect`'s rule, stated on a listing.
#[test]
fn a_folded_or_soloed_or_short_bay_lists_nothing() {
    let names = mock();
    let mut layout = solved(PLAUSIBLE);
    assert!(library(&layout, SCOPES, &names, None, None).is_some());

    layout.collapse(id_of(&layout, "library"));
    layout.solve();
    assert_eq!(
        library(&layout, SCOPES, &names, None, None),
        None,
        "the library is folded away and its rows are still being drawn"
    );

    layout.expand(id_of(&layout, "library"));
    layout.solve();
    assert!(library(&layout, SCOPES, &names, None, None).is_some());

    // A solo somewhere else takes the bay off the panel with it.
    layout.solo(id_of(&layout, "mixer"));
    layout.solve();
    assert_eq!(library(&layout, SCOPES, &names, None, None), None);
    layout.unsolo();
    layout.solve();
    assert!(library(&layout, SCOPES, &names, None, None).is_some());

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
        library(&short, SCOPES, &names, None, None),
        None,
        "a bay with no room for one row listed some"
    );

    // A hundred and twenty-five pixels of window taller is one row, which is
    // what says the answer above is the room and not the window.
    let barely = solved(Rect {
        h: 285.0,
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
        library(&barely, SCOPES, &names, None, None).map(|bay| bay.rows),
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
        library(&sliver, SCOPES, &names, None, None),
        None,
        "a bay with no room for the list's own padding listed some"
    );
}

// ---------------------------------------------------------------------------
// Which of this bay's parts answer a press, and nothing is stored
// ---------------------------------------------------------------------------

/// **The scope chips answer a press, and so do the two filter fields, the
/// `read` chip and the rows of the list — and nothing else in this bay
/// does.**
///
/// The mock draws four scope chips, two filter fields, a `+`, a row cursor,
/// the `read` chip and the `load → A` pill. **The chips are the first of them
/// that is a control here**, and the reason is the page rather than the code: `console.html`
/// puts the affordance on each chip — *"Click to show it; click another scope
/// to leave it"* — and `docs/manual/operations.html` names the row as *Choose
/// which scope the library shows*'s home.
///
/// **The two filter fields one row down are the bay's other control**, and
/// their own presses are asserted in
/// [`the_filter_fields_answer_a_press_and_the_row_around_them_does_not`]. What
/// this test adds about them is the same thing it adds about the chips: the
/// ground **around** them is nobody's, so the row's padding and the gap between
/// the two are swept here with everything else that is not a control.
///
/// **The `read` chip in the foot is the third**, and it is the one control in
/// this bay that is not in its head: the chips say which library and the
/// fields narrow it, where this reads the row the cursor is on.
/// `console.html`'s note *Reading a Set before you spend a load on it* is what
/// puts it there — *"a `read` chip sits in the foot between the count and
/// `load → A`"* — and what a press on it asks is
/// [`the_read_chip_asks_for_the_set_under_the_cursor`]'s. What this test adds
/// is that the foot's *ground* is still nobody's: the count, the space either
/// side of it and the gap between the two capsules take no press.
///
/// **The rows are the fourth, and they are the newest.** A press on one takes
/// that Set in hand — `LibraryBay::take`, and `crate::panel::Panel::carry` —
/// which is the drag *How a Set reaches a deck* specifies: *"Dragging a row
/// onto a strip is a second route to the same command, and never the first."*
/// What a row press asks for is `carry.rs`'s; what this test adds is that the
/// list's own ground under the last row is still nobody's.
///
/// **The pill is the one part of this bay that is deliberately not a
/// control**, and that is a specification rather than something left unbuilt:
/// the same note settles that the *key*'s route is *"a cursor and a key with
/// no pointer anywhere in it"*, and the pill is what says where that key
/// lands. So the two claims in this file are separate tests, because they are
/// separate sentences: this one, and
/// [`the_load_pill_takes_no_press_and_a_row_is_where_the_drag_begins`].
///
/// # The bay's corners do not reach the pill, and the two numbers say why
///
/// The bay's own rectangle inset past [`GRAB`](karakuri_console::panel::GRAB)
/// is where the sweep used to stop, and it is **two pixels short of the box
/// the pill is drawn in**. A `.lib-foot` is `padding: 5px 10px`
/// ([`size::LIB_FOOT_PAD_X`]) and a flex row whose `.sep { flex: 1 }` pushes
/// the pill to the far end of it, so the pill's right edge is 10 in from the
/// bay's — where an inset of `GRAB + 2` is 8. **10 against 6 is the pill
/// clearing the boundary's grab**, which is what makes it drawable at all and
/// is `input.rs`'s measurement one bay along; 8 is in the gap between the two,
/// and a pill that claimed presses would have gone unasked.
///
/// So the points are taken off [`LibraryBay`]'s own boxes rather than off the
/// bay's corners — and the view handed to `claim` is one that has been told
/// what libraries there are, which the sweep this replaces was not: a console
/// with no scopes has no chip row at all, and asking it whether a chip takes a
/// press is asking about a control nobody drew.
#[test]
fn the_chips_the_fields_the_read_chip_and_the_rows_are_the_bays_controls_and_nothing_else_is() {
    let (view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let region = to_egui(rect_of(panel.layout(), "library"));
    let row = bay.scopes.expect("the bay was handed scopes");

    // **Every chip, asked two pixels in from its own left edge.** Not the
    // centre: the last chip runs out past the bay and its centre can be
    // outside the row, which is the clip this row is drawn with and is
    // [`a_chip_is_pressed_only_where_it_is_drawn`]'s subject.
    let chips: Vec<(Scope, egui::Rect)> = bay.chips(&ctx, SCOPES).collect();
    assert_eq!(
        chips.len(),
        SCOPES.len(),
        "the bay was handed {} scopes and laid out {} chips",
        SCOPES.len(),
        chips.len()
    );
    for (scope, chip) in &chips {
        let probe = egui::pos2(chip.min.x + 2.0, chip.center().y);
        assert!(
            row.contains(probe),
            "`{}`'s own left edge is outside the scope row, so this test is asking about a \
             capsule nobody drew",
            scope.name()
        );
        assert!(
            !matches!(
                panel.layout().hit(Point::new(probe.x, probe.y), GRAB),
                karakuri_layout::Hit::Divider { .. }
            ),
            "a boundary grabs {probe:?}, which is on the `{}` chip — rule 3 gives it first \
             refusal and the chip would be dead there",
            scope.name()
        );
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(probe.x, probe.y)),
            Claim::Panel,
            "the console gave `egui` a press on the `{}` chip",
            scope.name()
        );
    }

    // **And every other point of the bay is `egui`'s**: the four corners
    // inside it and its middle, the ground of the scope row itself, the foot's
    // content box across its middle, and each drawn row.
    //
    // **Inset past `GRAB`**, because a boundary is claimed for a drag from six
    // pixels either side of it and three of this bay's four edges are one.
    let inset = GRAB + 2.0;
    let mut points = vec![
        egui::pos2(region.min.x + inset, region.min.y + inset),
        egui::pos2(region.max.x - inset, region.min.y + inset),
        egui::pos2(region.min.x + inset, region.max.y - inset),
        egui::pos2(region.max.x - inset, region.max.y - inset),
        region.center(),
    ];
    // **The scope row's own ground**: its left padding, the gap between the
    // first two chips, and the band above the capsules. `.scope` is a capsule
    // and not a cell — the row is not a segmented control — so the space
    // between two of them belongs to nobody.
    points.push(egui::pos2(
        row.min.x + size::SCOPES_PAD_X * 0.5,
        row.center().y,
    ));
    points.push(egui::pos2(
        chips[0].1.max.x + size::SCOPES_GAP * 0.5,
        row.center().y,
    ));
    points.push(egui::pos2(
        chips[0].1.center().x,
        row.min.y + size::SCOPES_PAD_Y * 0.5,
    ));
    // **The filter row's own ground**: its left padding and the gap between the
    // two fields. `.field` is a capsule the same way `.scope` is, so what is
    // between two of them belongs to nobody.
    let filters = bay.filters.expect("the bay draws its filter row");
    let holds = bay.field(Field::Holds).expect("the `holds` field");
    points.push(egui::pos2(
        filters.min.x + size::LIB_FILTERS_PAD_X * 0.5,
        filters.center().y,
    ));
    points.push(egui::pos2(
        holds.max.x + size::LIB_FILTERS_GAP * 0.5,
        filters.center().y,
    ));
    let (left, right) = (
        bay.foot.min.x + size::LIB_FOOT_PAD_X,
        bay.foot.max.x - size::LIB_FOOT_PAD_X,
    );
    assert!(
        right - left > 0.0,
        "the foot's content box is {left} to {right}, which is no box to sweep"
    );
    // **The two capsules at the far end of the row are stepped over**, and
    // they are stepped over for two different reasons: the `read` chip is a
    // control and is asked about below, and the `load → A` pill is a readout
    // that [`a_load_is_a_cursor_and_a_key_with_no_pointer_anywhere_in_it`]
    // sweeps on its own. What is left is the foot's ground — the count, and
    // the gap between it and them.
    let chip = bay.read_chip(&ctx, DECK_LETTERS[0]);
    let pill = bay.load(&ctx, DECK_LETTERS[0]).pill;
    let mut swept = 0;
    for step in 0..=10 {
        let t = step as f32 / 10.0;
        let p = egui::pos2(left + (right - left) * t, bay.foot.center().y);
        if chip.contains(p) || pill.contains(p) {
            continue;
        }
        points.push(p);
        swept += 1;
    }
    assert!(
        swept >= 5,
        "only {swept} points of the foot's ground were swept, and the two capsules cannot be          most of a row this wide"
    );
    // **The list's own ground under the last row**, which is where the rows
    // stop and the foot has not started: a `.lib-row` is a stride and the list
    // is whatever is left of the bay, so what is below the last of them is
    // nobody's. The rows themselves are the panel's and are asked below.
    let last = bay.row(bay.rows - 1);
    let ground = egui::pos2(last.center().x, last.max.y + size::LIB_ROW_H * 0.5);
    assert!(
        ground.y < bay.foot.min.y,
        "the sweep is asking about the foot rather than about the list's ground"
    );
    points.push(ground);

    let mut asked = 0;
    for p in &points {
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(p.x, p.y)),
            Claim::Egui,
            "the console took the pointer at {p:?}, which is inside the Library bay and is not \
             on a scope chip"
        );
        asked += 1;
    }
    // **And every drawn row is the panel's**, asked at three points across it:
    // a press on one takes that Set in hand, and a row that went to `egui`
    // would be the one gesture this bay exists for reaching nothing at all.
    for index in 0..bay.rows {
        let at = bay.row(index);
        for probe in [
            egui::pos2(at.min.x + size::LIB_ROW_PAD_X, at.center().y),
            at.center(),
            egui::pos2(at.max.x - size::LIB_ROW_PAD_X, at.center().y),
        ] {
            assert_eq!(
                claim(&mut panel, &ctx, &view, Point::new(probe.x, probe.y)),
                Claim::Panel,
                "the console gave `egui` a press on row {index} at {probe:?}"
            );
        }
    }

    // **And the `read` chip is the panel's**, asked at the same two pixels in
    // from its own left edge the chips above are — its right-hand end is the
    // gap before the load pill and its bottom rim is under the boundary's
    // grab, which is [`the_read_chip_clears_every_boundary_but_the_one_under
    // _the_bay`]'s subject and not this test's.
    let probe = egui::pos2(chip.min.x + 2.0, chip.center().y);
    assert_eq!(
        claim(&mut panel, &ctx, &view, Point::new(probe.x, probe.y)),
        Claim::Panel,
        "the console gave `egui` a press on the `read` chip at {probe:?}"
    );

    // A guard, so this cannot pass by testing nothing — and a floor under the
    // reach, so the foot's sweep, the scope row's ground and the rows cannot
    // quietly go away.
    assert_eq!(asked, points.len(), "not every point was asked");
    assert!(
        points.len() >= 1 + 5 + 3 + 2 + 5,
        "only {} points asked: the list's own ground, the bay's corners, the scope row's ground, \
         the filter row's and the foot's are the floor, and fewer is a sweep that has stopped \
         covering the foot",
        points.len()
    );
}

/// **The load pill takes no press, and a row is where the drag begins.**
///
/// `console.html`'s two sentences about the same row, and they are one test
/// because they are one decision taken twice. The `load → A` pill is a
/// **readout** — *"what the control owes instead is to say where it lands
/// before the press"* — and the *key*'s route is *"a cursor and a key with no
/// pointer anywhere in it"*, so a press on the pill would add a pointer to the
/// one gesture that page describes as having none, and it would name the deck
/// from the selection, which is exactly what `l` already does.
///
/// **The panel's own route is the drag**, which that page calls *"a second
/// route to the same command, and never the first"* — *"it names both operands
/// in the one gesture, which makes it the only way to load a deck without
/// selecting it first"*. So the row a hand presses is the panel's and the
/// capsule beside it is not, and that asymmetry is the specification rather
/// than a thing left unbuilt. What a row press then asks for is `carry.rs`'s.
///
/// The pill's own box is asked rather than the foot's middle, because the box
/// is where a press would land: it is measured off the same galley the paint
/// lays out, so the rectangle asserted here and the capsule drawn are one
/// statement.
#[test]
fn the_load_pill_takes_no_press_and_a_row_is_where_the_drag_begins() {
    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(4).collect();

    // **The pill, as wide as what is in it** — the word, the arrow's box and
    // the letter, inside a `.pill`'s padding either side. Asked of
    // `LibraryBay::load`, which is the derivation the paint uses, so the box
    // swept here is the capsule drawn.
    let pill = bay.load(&ctx, DECK_LETTERS[0]).pill;
    assert!(
        bay.foot.contains_rect(pill),
        "the pill at {pill:?} is not inside the foot at {:?}",
        bay.foot
    );

    // **Across the pill at the foot's own centre line**, and not its top and
    // bottom edges: `.lib-foot` is 26 tall and a `.pill` is 16.5, so the
    // capsule's edges are 4.75 off the foot's — and the foot's bottom edge is
    // the bay's, which is a boundary. 4.75 against a `GRAB` of 6 means a
    // boundary would take a press on the capsule's own rim, which is
    // `input.rs`'s rule 3 rather than anything about this row. **It is also
    // the plainest evidence this was never meant to be a control**: every
    // capsule on this console that *is* one clears the grab, and this one does
    // not.
    let points: Vec<egui::Pos2> = (0..=6)
        .map(|step| {
            let t = step as f32 / 6.0;
            egui::pos2(pill.min.x + pill.width() * t, pill.center().y)
        })
        .collect();
    assert!(
        bay.rows > 0,
        "the bay drew no rows, so the contrast below is measuring nothing"
    );

    for p in &points {
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(p.x, p.y)),
            Claim::Egui,
            "the console took the pointer at {p:?} — the pill says where a load lands and the \
             key is what puts it there"
        );
    }

    // **And the contrast, on the row the drag does begin on**, so that this
    // cannot pass by a bay that takes no press anywhere: the capsule is
    // `egui`'s and the row two lines above it is the panel's, in one console
    // in one state.
    let first = bay.row(0).center();
    assert_eq!(
        claim(&mut panel, &ctx, &view, Point::new(first.x, first.y)),
        Claim::Panel,
        "the row the drag begins on went to `egui`, so the sweep above is measuring a bay that \
         takes no press at all"
    );
}

/// **The star is inside its row, clear of every boundary, and it names the
/// state the row is not in.**
///
/// Three claims about one control because they are the three a new one owes
/// (`input.rs`'s rule 4): where it is, that a hand can reach it, and what a
/// press on it asks for.
///
/// **Inside the row**, because a star that overhung the row above or the list's
/// own padding would be a mark drawn on somebody else's ground. **Clear of the
/// grab**, because the list's left edge is the bay's and the bay's is a
/// boundary: `.lib-row` is `padding: 3px 7px` inside a `.lib-list` of `3`,
/// which is 10 in from the bay against a [`GRAB`] of 6 — the same measurement
/// the two filter fields clear the same edge by.
///
/// **And it names the state the row is not in**, which is ADR-0299's *not a
/// toggle* met at the control: the operation carries the state, so what reads
/// the present mark is the surface. A star that always asked for `true` would
/// pass every assertion about the first press and never take one off, so both
/// directions are asked of one bay in one state.
#[test]
fn a_star_is_inside_its_row_and_names_the_state_the_row_is_not_in() {
    let (view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    assert!(bay.rows >= 2, "the bay drew {} rows", bay.rows);

    for index in 0..bay.rows {
        let star = bay.star(index);
        let row = bay.row(index);
        assert!(
            row.contains_rect(star),
            "the star at {star:?} is not inside row {index} at {row:?}"
        );
        assert!(
            star.min.x - bay.list.min.x >= 0.0,
            "the star at {star:?} hangs off the left of the list at {:?}",
            bay.list
        );
        // **A boundary would take the press before any control did**, which is
        // `input::claim`'s rule 3 and is what the two filter fields are
        // measured against one row up.
        for p in [
            egui::pos2(star.min.x, star.center().y),
            egui::pos2(star.max.x, star.center().y),
        ] {
            assert!(
                !matches!(
                    panel.layout().hit(Point::new(p.x, p.y), GRAB),
                    karakuri_layout::Hit::Divider { .. }
                ),
                "a boundary grabs {p:?}, which is on row {index}'s star"
            );
        }
        assert_eq!(
            claim(
                &mut panel,
                &ctx,
                &view,
                Point::new(star.center().x, star.center().y)
            ),
            Claim::Panel,
            "the console gave `egui` a press on row {index}'s star"
        );
    }

    // **Nothing starred: every press asks for the star to go on.**
    let none = std::collections::BTreeSet::new();
    let at = bay.star(1).center();
    assert_eq!(
        bay.starred(&mock(), &none, Point::new(at.x, at.y)),
        Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: true,
        }),
        "the star did not name the row it is drawn on"
    );

    // **And with that row starred it asks for the star to come off**, which is
    // the same derivation reading the state it is drawn from.
    let one: std::collections::BTreeSet<String> =
        std::iter::once("lattice_veil".to_owned()).collect();
    assert_eq!(
        bay.starred(&mock(), &one, Point::new(at.x, at.y)),
        Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: false,
        }),
        "a starred row was asked to be starred again"
    );

    // **The row's own ground is not the star's**, which is rule 4's *a control
    // claims what it acts on and no more*: the far end of the same row is
    // where the carry begins and the mark answers nothing there.
    let ground = egui::pos2(bay.row(1).max.x - size::LIB_ROW_PAD_X, at.y);
    assert_eq!(
        bay.starred(&mock(), &none, Point::new(ground.x, ground.y)),
        None,
        "the star answered a press at the far end of its row"
    );

    // **And a listing shorter than the rows drawn takes nothing**, which is
    // `take`'s refusal rather than a clamp: a star answered bare would name a
    // Set nobody can see.
    assert_eq!(
        bay.starred(&[], &none, Point::new(at.x, at.y)),
        None,
        "the star named a Set in a listing with nothing in it"
    );
}

/// **A press names the chip it landed on, and never the next one.**
///
/// This is the whole of what the pointer adds and it is `e`'s opposite:
/// `docs/manual/operations.html` binds the key to *step to the next scope and
/// wrap* because *"a bare press cannot type a name"*, and a pointer press
/// **can** — it lands on one capsule and on no other. So the chip that was
/// pressed is the chip that is asked for, which is
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)'s
/// division met by two surfaces rather than an inconsistency between them.
///
/// **The operation beside it still carries `Undecided`**, and that is asserted
/// rather than left implied: the press knows which chip and the *vocabulary*
/// does not, because a payload naming one of four would assert that the list
/// of scopes can be finished. Settling that is a decision about the operations
/// page and `karakuri-operation`, and this test is what fails the day somebody
/// takes it — deliberately, so that it is taken on purpose.
#[test]
fn a_press_names_the_chip_it_landed_on_and_never_the_next_one() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let row = bay.scopes.expect("the bay was handed scopes");

    for (scope, chip) in bay.chips(&ctx, SCOPES) {
        let probe = egui::pos2(chip.min.x + 2.0, chip.center().y);
        assert!(
            row.contains(probe),
            "`{}`'s left edge is off the row",
            scope.name()
        );
        let chosen = bay
            .chip(&ctx, SCOPES, Point::new(probe.x, probe.y))
            .unwrap_or_else(|| panic!("no chip answered a press on `{}`", scope.name()));
        assert_eq!(
            chosen.scope,
            scope,
            "a press on `{}` asked for `{}`",
            scope.name(),
            chosen.scope.name()
        );
        assert_eq!(
            chosen.operation,
            karakuri_operation::Operation::SelectScope {
                scope: karakuri_operation::Undecided
            },
            "the chip asked for something other than `Choose which scope the library shows`"
        );
    }

    // **The chip that is already marked asks for itself**, where the key would
    // step off it. The two are asked of one console in one state, so this is
    // the contrast and not two facts side by side.
    assert!(view.select_scope(Scope::MySets));
    let (_, chip) = bay
        .chips(&ctx, SCOPES)
        .find(|(scope, _)| *scope == Scope::MySets)
        .expect("`my sets` is on the row");
    let probe = egui::pos2(chip.min.x + 2.0, chip.center().y);
    assert_eq!(
        bay.chip(&ctx, SCOPES, Point::new(probe.x, probe.y))
            .map(|chosen| chosen.scope),
        Some(Scope::MySets),
        "a press on the marked chip stepped somewhere"
    );
    assert!(view.step_scope());
    assert_eq!(
        view.scope(),
        Some(Scope::Presets),
        "the key does not step where the pointer names, so this contrast is measuring nothing"
    );
}

/// **A chip is pressed only where it is drawn.**
///
/// `.scopes` carries a wrapping flex and this console draws one row of it and
/// clips, so a pane narrower than the four words leaves the last chip starting
/// inside the bay and finishing outside. **What is not drawn is not a
/// target**: the point is held to the row before any chip is asked about, so
/// the tail hanging over the centre column belongs to whatever is drawn there
/// and not to a capsule the operator cannot see.
///
/// **The pane is narrower than the mock's own 218 now**, and that is ADR-0299
/// rather than a number tuned to make a test pass: the first chip was
/// `favourites` and is `all`, which is seven characters shorter, so the four
/// words fit at the width they used to overrun. The clip is still the
/// derivation's rule and an operator still reaches it with a divider, so this
/// asks the same question of a pane that has been dragged in.
///
/// The overrun is asserted rather than assumed, because a pane wide enough to
/// hold all four would make the rest of this test measure nothing.
#[test]
fn a_chip_is_pressed_only_where_it_is_drawn() {
    let mut layout = solved(PLAUSIBLE);
    let left = id_of(&layout, "left-pane");
    let split = layout.parent(left).expect("left-pane has a parent split");
    layout.set_divider(split, 0, 190.0);
    layout.solve();
    let ctx = drawn_once();
    let bay =
        library(&layout, SCOPES, &mock(), None, None).expect("the library bay lists its rows");
    let row = bay.scopes.expect("the bay was handed scopes");

    let (scope, last) = bay
        .chips(&ctx, SCOPES)
        .last()
        .expect("the row has chips in it");
    assert!(
        last.max.x > row.max.x,
        "`{}` ends at {} and the row ends at {} — the pane is wide enough to hold every chip, \
         so there is no clipped capsule to ask about",
        scope.name(),
        last.max.x,
        row.max.x
    );
    assert!(
        last.min.x < row.max.x,
        "`{}` starts outside the row entirely, so it is not the half-drawn chip this is about",
        scope.name()
    );

    let over = egui::pos2((last.max.x + row.max.x) * 0.5, last.center().y);
    assert!(
        last.contains(over) && !row.contains(over),
        "{over:?} is not on the part of `{}` that hangs outside the row",
        scope.name()
    );
    assert_eq!(
        bay.chip(&ctx, SCOPES, Point::new(over.x, over.y)),
        None,
        "the console answered a press on the part of `{}` it does not draw",
        scope.name()
    );

    // **And the ground of the row answers nothing either** — its left padding
    // and the gap between two capsules, which is what makes the row a row of
    // chips rather than a segmented control.
    for (p, what) in [
        (
            egui::pos2(row.min.x + size::SCOPES_PAD_X * 0.5, row.center().y),
            "the row's left padding",
        ),
        (
            egui::pos2(row.min.x + 1.0, row.min.y + 1.0),
            "the row's top-left corner",
        ),
    ] {
        assert_eq!(
            bay.chip(&ctx, SCOPES, Point::new(p.x, p.y)),
            None,
            "a chip answered a press on {what}"
        );
    }
}

/// **The capsule a press lands on is the capsule the wash is drawn in.**
///
/// One derivation asked twice, which is this crate's rule for every control:
/// [`LibraryBay::chips`] is what the paint walks and what the hit test walks,
/// so a chip cannot be pressed anywhere the mark is not drawn. It is worth a
/// test of its own because a chip is as wide as the word in it — a second
/// measurement would agree on `favourites` and be wrong about `folder` by the
/// sum of three words' widths.
#[test]
fn the_capsule_a_press_lands_on_is_the_capsule_the_wash_is_drawn_in() {
    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let row = bay.scopes.expect("the bay was handed scopes");

    for want in Scope::ALL {
        assert!(view.select_scope(want) || view.scope() == Some(want));
        let washes: Vec<egui::Rect> = shapes_across(&mut view, &mut panel, row)
            .iter()
            .filter_map(|shape| match shape {
                egui::Shape::Rect(at) => Some(at.rect),
                _ => None,
            })
            .collect();
        assert_eq!(
            washes.len(),
            1,
            "`{}` is marked and {} chips are washed",
            want.name(),
            washes.len()
        );
        let (_, chip) = bay
            .chips(&ctx, SCOPES)
            .find(|(scope, _)| *scope == want)
            .expect("the marked scope is on the row");
        assert!(
            near(washes[0].min.x, chip.min.x)
                && near(washes[0].min.y, chip.min.y)
                && near(washes[0].width(), chip.width())
                && near(washes[0].height(), chip.height()),
            "`{}` is washed at {:?} and hit-tested at {chip:?}",
            want.name(),
            washes[0]
        );
    }
}

/// **The chips clear every boundary, and the row's own clip is what costs the
/// last one its tail.**
///
/// `input.rs`'s rule 3 gives a boundary first refusal, so a control inside a
/// grab is dead there — which is why every control on this console owes this
/// measurement off its own rectangle rather than inheriting one. The scope
/// row's is a **sum and not a centring**: `.scopes` is drawn under the bay
/// head, so what holds the chips off the boundary above is `HEAD_H` plus
/// `SCOPES_PAD_Y` — 27 + 7 = **34** — and down the left it is
/// `SCOPES_PAD_X`'s **9** off an edge that is the viewport's rather than a
/// divider's.
///
/// **The last chip is the exception and it is the ordinary price**, the same
/// one the `solo` capsule and a preview cell pay: the row clips at the pane's
/// edge, the last six pixels of what is drawn are the pane divider's, and
/// what brings the whole capsule in is widening the pane.
#[test]
fn the_chips_clear_every_boundary_but_the_one_the_row_is_clipped_by() {
    let panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let region = to_egui(rect_of(panel.layout(), "library"));
    let row = bay.scopes.expect("the bay was handed scopes");

    let above = row.min.y + size::SCOPES_PAD_Y - region.min.y;
    assert!(
        near(above, size::HEAD_H + size::SCOPES_PAD_Y),
        "the chips are {above} below the bay's top edge and the bay head plus `.scopes`' padding \
         is {}",
        size::HEAD_H + size::SCOPES_PAD_Y
    );
    assert!(
        above > GRAB,
        "the chips are {above} below the bay's top edge and a boundary grabs {GRAB}"
    );

    for (scope, chip) in bay.chips(&ctx, SCOPES) {
        let left = chip.min.x - region.min.x;
        assert!(
            left >= size::SCOPES_PAD_X,
            "`{}` starts {left} in from the bay's left edge and `.scopes`' padding is {}",
            scope.name(),
            size::SCOPES_PAD_X
        );
        assert!(
            left > GRAB,
            "`{}` starts {left} in from the bay's left edge and a boundary grabs {GRAB}",
            scope.name()
        );
        // **Where the chip is drawn, no boundary has it.** The tail of the
        // last one is outside the row and is nobody's business here.
        let drawn = chip.intersect(row);
        if drawn.width() <= 0.0 {
            continue;
        }
        let probe = egui::pos2(drawn.min.x + 1.0, drawn.center().y);
        assert!(
            !matches!(
                panel.layout().hit(Point::new(probe.x, probe.y), GRAB),
                karakuri_layout::Hit::Divider { .. }
            ),
            "a boundary grabs {probe:?}, which is on `{}` where it is drawn",
            scope.name()
        );
    }

    // **And the band at the far end of the row is still a boundary's**, which
    // is what says the clearances above are clearances rather than the grab
    // having gone missing.
    let into = Point::new(row.max.x - 1.0, row.center().y);
    assert!(
        matches!(
            panel.layout().hit(into, GRAB),
            karakuri_layout::Hit::Divider { .. }
        ),
        "the pixel at the far end of the scope row is not the pane divider's, so nothing here \
         measured a grab"
    );
}

/// **The names are the harness's and the console keeps no copy.**
///
/// The bay is derived from the slice it is handed on the frame it is handed
/// it, so a listing that changed between two frames is two different bays and
/// never one bay remembering the first.
#[test]
fn the_names_are_the_harnesss_and_are_stored_nowhere() {
    let panel = console(PLAUSIBLE);
    let one = vec!["morph01".to_owned()];
    let two = vec!["morph01".to_owned(), "night01".to_owned()];
    assert_eq!(
        library(panel.layout(), SCOPES, &one, None, None).map(|b| b.total),
        Some(1)
    );
    assert_eq!(
        library(panel.layout(), SCOPES, &two, None, None).map(|b| b.total),
        Some(2)
    );
    assert_eq!(
        library(panel.layout(), SCOPES, &one, None, None).map(|b| b.total),
        Some(1)
    );

    // And what a `View` holds is what it was handed, unchanged by drawing it.
    let mut view = View::new(Room::Day);
    view.library = two.clone();
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, &mut panel));
    out.textures_delta.clear();
    assert_eq!(
        view.library, two,
        "the frame rewrote the listing it was given"
    );
}

// ---------------------------------------------------------------------------
// The scope row: four questions, one of them marked
// ---------------------------------------------------------------------------

/// Every shape the scope row painted, which is **not** `shapes_inside`.
///
/// The four chips are 246 wide laid end to end and the mock's own left pane is
/// 218, so the last of them hangs out of the bay and is clipped — one row and
/// not a wrap, which is `view::scopes_into`'s rule and `REND_ROW_H`'s before
/// it. A shape wholly inside the row would therefore be three chips and the
/// fourth would look undrawn, which is the one thing these tests must not
/// conclude. So a shape counts where it sits **in** the row down the column
/// and reaches it across: the bay's card is the shape that rules out, since it
/// is the height of the whole bay.
fn shapes_across(view: &mut View, panel: &mut Panel, row: egui::Rect) -> Vec<egui::Shape> {
    shapes_inside(view, panel, egui::Rect::EVERYTHING)
        .into_iter()
        .filter(|shape| {
            let bounds = shape.visual_bounding_rect();
            bounds.is_finite()
                && bounds.min.y >= row.min.y - 1.0
                && bounds.max.y <= row.max.y + 1.0
                && bounds.intersects(row)
        })
        .collect()
}

/// Every word the scope row painted, left to right.
fn chips(view: &mut View, panel: &mut Panel, bay: &LibraryBay) -> Vec<String> {
    let row = bay.scopes.expect("the bay was handed scopes");
    let mut found: Vec<(f32, String)> = shapes_across(view, panel, row)
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(at) => Some((at.pos.x, at.galley.text().to_owned())),
            _ => None,
        })
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().map(|(_, text)| text).collect()
}

/// The word inside the one washed chip, or `None` where nothing is washed —
/// and it panics where more than one is, which is the half of the claim a
/// `Vec` of them would let pass.
fn marked(view: &mut View, panel: &mut Panel, bay: &LibraryBay) -> Option<String> {
    let row = bay.scopes.expect("the bay was handed scopes");
    let shapes = shapes_across(view, panel, row);
    let washes: Vec<egui::Rect> = shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(at) => Some(at.rect),
            _ => None,
        })
        .collect();
    assert!(
        washes.len() <= 1,
        "{} chips are washed, and `.scope.sel` is one of them",
        washes.len()
    );
    let wash = washes.first()?;
    shapes.iter().find_map(|shape| match shape {
        egui::Shape::Text(at) if wash.contains(at.pos) => Some(at.galley.text().to_owned()),
        _ => None,
    })
}

/// **The scope row draws the chips it was handed, in the order it was handed
/// them** — which is the bay's own row: `all`, `my sets`, `presets`, `folder`.
///
/// **It is the mock's row with its first chip renamed and moved**, which is
/// [ADR-0299](../../../docs/adr/0299-my-sets-is-the-starred-subset-and-the-star-is-kept-beside-the-sets.md):
/// `my sets` is the starred subset now, so the chip that lists everything the
/// store holds is `all` and it comes first, because it is the listing the
/// other three are questions about.
///
/// The `+` the mock draws after them is not one of them, and that is asserted
/// rather than left to a reader counting four: it is the arena's own gap drawn
/// a fifth time, and a chip for it would be a control over adding a region.
#[test]
fn the_scope_row_draws_the_chips_it_was_handed_in_the_order_it_was_handed_them() {
    let (mut view, mut panel) = showing_mock();
    let bay = bay(&panel);
    assert_eq!(
        chips(&mut view, &mut panel, &bay),
        vec!["all", "my sets", "presets", "folder"],
        "the scope row is not this bay's four chips in this bay's order"
    );
}

/// **One chip is marked and it is the one the scope pointer is on**, asserted
/// by drawing the bay and finding the wash.
///
/// The mark follows the pointer rather than being painted at a fixed chip,
/// which is the failure a first-chip default hides completely — so the pointer
/// is stepped the whole way round and the wash is read off the frame each
/// time.
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

    for scope in [Scope::Presets, Scope::Folder, Scope::AllSets, Scope::MySets] {
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

/// **A step goes to the next scope and wraps**, which is what
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

    assert!(view.walk(2, 5), "the cursor did not move");
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
    assert_eq!(view.scope(), Some(Scope::Folder), "the last chip");
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

/// **A scope this console was not handed is refused rather than marked**,
/// which is `View::select` s rule one control along: a mark on a chip nobody
/// drew is a mark drawn nowhere, over a listing with no question above it.
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

/// Every filled rectangle painted at one of the bay's own row boxes, as the
/// row index it landed on.
///
/// The cursor's mark is a wash and nothing else — `.lib-row.cursor` sets a
/// `background` and a `color`, and no rule, caret or chevron — so a row that
/// is not under it paints no rectangle of its own at all. That is what makes
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

/// A view over the mock's library, at a solved console.
fn showing_mock() -> (View, Panel) {
    let mut view = View::new(Room::Day);
    view.library = mock();
    // **The chips as well as the rows**, because they are the same bay: a
    // console handed a listing and no scopes draws its list one scope row
    // higher up, and every rectangle asserted below would be off by that row.
    view.scopes = Scope::ALL.to_vec();
    (view, console(PLAUSIBLE))
}

/// **The cursor is one row and the row is the one it is on**, asserted by
/// drawing the bay and finding the wash.
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

    assert!(view.walk(2, bay.rows), "the cursor did not move two rows");
    let drawn = shapes_inside(&mut view, &mut panel, list);
    assert_eq!(
        washed(&drawn, &bay),
        vec![2],
        "the cursor moved and the wash stayed where it was"
    );

    assert!(view.walk(-2, bay.rows), "the cursor did not move back");
    let drawn = shapes_inside(&mut view, &mut panel, list);
    assert_eq!(washed(&drawn, &bay), vec![0], "the walk back drew nothing");
}

/// **The cursor is held inside the rows the bay drew, and it does not wrap.**
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
    assert!(!view.walk(-1, 2), "the cursor walked above the first row");
    assert_eq!(view.cursor_row(), 0);
    assert!(view.walk(1, 2));
    assert_eq!(view.cursor_row(), 1);
    assert!(
        !view.walk(1, 2),
        "the cursor left the two rows the bay listed"
    );
    assert_eq!(view.cursor_row(), 1, "and it wrapped instead of stopping");

    // A bay with room for all five, and a step past the end of the store.
    assert!(view.walk(99, 5));
    assert_eq!(view.cursor_row(), 4, "a long step left the listing");
    assert!(!view.walk(1, 5));

    // And no listing at all is no cursor to move: a console with no store
    // behind it, which is every other test in this crate.
    let mut empty = View::new(Room::Day);
    assert!(!empty.walk(1, 3), "a console with no store moved a cursor");
    assert_eq!(empty.cursor_row(), 0);
}

/// **The foot's pill says where a press would land, and the letter follows the
/// deck selection.**
///
/// `console.html`: *"The letter on the pill is the whole warning … What the
/// control owes instead is to say where it lands before the press."* So this
/// asserts what is painted rather than a rectangle, and asserts it again after
/// the selection moves — a pill that read `load → A` whatever was selected
/// would pass the first half and be a lie for the other three decks.
///
/// **The letter is its own galley now**, because the arrow between the word
/// and it is drawn rather than typed — see
/// [`the_foots_arrow_is_drawn_rather_than_typed`]. So the reading this asserts
/// is the letter alone, which is the half of the pill that moves.
#[test]
fn the_foot_says_which_deck_a_press_would_land_on() {
    let (mut view, mut panel) = showing_mock();
    let bay = bay(&panel);
    // Four strips, so all four decks can be selected. What a strip *reads* is
    // not this bay's business — only that there is one.
    view.mixer = std::iter::repeat_with(strip).take(4).collect();

    for deck in 0..4u8 {
        assert!(
            view.select(deck) || deck == 0,
            "deck {deck} could not be selected with four strips"
        );
        let want = DECK_LETTERS[usize::from(deck)];
        let drawn = shapes_inside(&mut view, &mut panel, bay.foot);
        assert!(
            drawn.iter().any(|shape| matches!(
                shape,
                egui::Shape::Text(at) if at.galley.text() == want
            )),
            "the selection is deck {deck} and the foot's pill does not read `{want}`: {drawn:#?}"
        );
        // **And the word is still beside it**, so a letter drawn alone in an
        // empty capsule cannot pass this.
        assert!(
            drawn.iter().any(|shape| matches!(
                shape,
                egui::Shape::Text(at) if at.galley.text() == "load"
            )),
            "the foot's pill does not say `load`: {drawn:#?}"
        );
    }
}

/// **The foot's arrow is drawn rather than typed, because `egui`'s default
/// face has no U+2192.**
///
/// The pill was `"load \u{2192} "` with the deck's letter appended, and the
/// panel drew `load □ A`: a readout of *where a press would land* with a tofu
/// where the arrow was. `CHEVRON_W` three bays along records the answer for
/// this whole class of question — whether a glyph is in the default face has
/// no good answer, so the mark is drawn — and this is that answer applied
/// here.
///
/// # What it asserts
///
/// 1. **Nothing painted in the foot carries U+2192**, which is the defect
///    itself and is asserted over every galley rather than over the constant:
///    a character typed back into the word would fail here.
/// 2. **A triangle is painted in the arrow's box**, so the mark did not simply
///    go away — a pill reading `load A` says nothing about where the letter
///    stands to the word.
/// 3. **The box is between the word and the letter**, which is what makes the
///    three one reading rather than a mark parked at one end.
#[test]
fn the_foots_arrow_is_drawn_rather_than_typed() {
    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let at = bay.load(&ctx, DECK_LETTERS[0]);

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

/// **A strip, as far as this bay cares**: something for a deck to be selected
/// on. Every reading in it is beside the point here — `mixer.rs` is where a
/// strip's values are asserted — and what matters is that there are as many of
/// them as there are decks to name.
fn strip() -> karakuri_console::view::Strip {
    karakuri_console::view::Strip {
        name: String::new(),
        tally: karakuri_console::view::Tally::Allocated,
        requested: karakuri_console::view::Tally::Allocated,
        gain: 0.0,
        gain_to: None,
        opacity: 0.0,
        opacity_to: None,
        blend: karakuri_operation::BlendMode::Over,
        mask: karakuri_console::view::Mask::None,
        mask_angle: 0.0,
        level: None,
    }
}

// ---------------------------------------------------------------------------
// The two filter fields
// ---------------------------------------------------------------------------

/// **Every layer of the vocabulary is one the `layer` field can be stepped
/// to**, which is what makes `View::narrow` right to accept any of them.
///
/// The match is the enforcement rather than the assertion under it: a sixth
/// variant on `karakuri_operation::Layer` does not compile here until somebody
/// has decided what this field calls it and where in the cycle it goes.
#[test]
fn the_layer_field_steps_through_every_layer_there_is() {
    use karakuri_operation::Layer;
    for layer in [Layer::L1, Layer::L2, Layer::L3, Layer::L4, Layer::Field] {
        let spelled = match layer {
            Layer::L1 => "L1",
            Layer::L2 => "L2",
            Layer::L3 => "L3",
            Layer::L4 => "L4",
            Layer::Field => "FIELD",
        };
        let found = LAYERS
            .iter()
            .find(|(kind, _)| *kind == layer)
            .unwrap_or_else(|| panic!("{layer:?} is not on the `layer` field's cycle"));
        assert_eq!(found.1, spelled, "{layer:?} is spelled two ways");
    }
    assert_eq!(
        LAYERS.len(),
        5,
        "the cycle has {} entries and the vocabulary has five layers",
        LAYERS.len()
    );
}

/// **A press on a field steps it, and the operation names where it arrived.**
///
/// The whole cycle of each field, both ways round the wrap, because the state a
/// step cannot reach is the one a control quietly loses: `layer` from `FIELD`
/// back to unset, and `holds` from the last candidate back to unset.
///
/// **`Operation::ListSets` carries both halves**, so a press on one field says
/// what the *other* one is as well — which is what makes the pair a filter
/// rather than two. That is asserted here by setting one and pressing the
/// other.
#[test]
fn a_press_on_a_filter_field_steps_it_and_names_where_it_arrived() {
    use karakuri_operation::Layer;
    let panel = console(PLAUSIBLE);
    let bay = bay(&panel);
    let holds: Vec<String> = ["drift_shell", "soft_points"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let at = |field: Field| {
        let box_ = bay.field(field).expect("the field is drawn");
        Point::new(box_.center().x, box_.center().y)
    };

    // The `layer` field, all the way round: unset, the five layers in order,
    // and unset again.
    let mut layer = None;
    for want in [
        Some(Layer::L1),
        Some(Layer::L2),
        Some(Layer::L3),
        Some(Layer::L4),
        Some(Layer::Field),
        None,
    ] {
        let asked = bay
            .filter(&holds, Filters { holds: None, layer }, at(Field::Layer))
            .expect("the `layer` field did not answer a press on it");
        assert_eq!(
            asked,
            karakuri_operation::Operation::ListSets {
                holds: None,
                layer: want
            },
            "the `layer` field stepped from {layer:?} to something else"
        );
        layer = want;
    }

    // The `holds` field, over the two candidates and back to unset — and with
    // a layer set, which the operation has to carry through untouched.
    let mut set = None;
    for want in [Some("drift_shell"), Some("soft_points"), None] {
        let asked = bay
            .filter(
                &holds,
                Filters {
                    holds: set,
                    layer: Some(Layer::L4),
                },
                at(Field::Holds),
            )
            .expect("the `holds` field did not answer a press on it");
        assert_eq!(
            asked,
            karakuri_operation::Operation::ListSets {
                holds: want.map(str::to_owned),
                layer: Some(Layer::L4)
            },
            "the `holds` field stepped from {set:?} to something else, or dropped the layer"
        );
        set = want;
    }
}

/// **A `holds` field with nothing to step to asks the listing again**, which is
/// the state every console in this crate that has not been handed candidates is
/// in — and it is a question rather than a no-op, the same one a press on the
/// chip that is already marked asks.
#[test]
fn a_holds_field_with_nothing_to_step_to_asks_the_listing_again() {
    let panel = console(PLAUSIBLE);
    let bay = bay(&panel);
    let box_ = bay.field(Field::Holds).expect("the field is drawn");
    assert_eq!(
        bay.filter(
            &[],
            Filters::NONE,
            Point::new(box_.center().x, box_.center().y)
        ),
        Some(karakuri_operation::Operation::ListSets {
            holds: None,
            layer: None
        }),
        "a press on `holds…` with no candidates behind it was not answered"
    );
}

/// **The two fields answer a press and the row around them does not.**
///
/// The padding either side and the gap between them are bare card, exactly as
/// the gaps between the scope chips are — and `claim` is asked as well as the
/// offer, because a control that acts on a press `egui` was given is a control
/// nobody can reach.
#[test]
fn the_filter_fields_answer_a_press_and_the_row_around_them_does_not() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let row = bay.filters.expect("the bay draws its filter row");
    let mut view = View::new(karakuri_console::room::Room::Day);
    view.scopes = SCOPES.to_vec();
    view.library = mock();
    view.holds = vec!["drift_shell".to_owned()];

    for field in Field::ALL {
        let box_ = bay.field(field).expect("the field is drawn");
        let on = Point::new(box_.center().x, box_.center().y);
        assert!(
            bay.filter(&view.holds, view.filters(), on).is_some(),
            "{field:?} did not answer a press in the middle of it"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &view, on),
            Claim::Panel,
            "{field:?} is drawn and `egui` was given the press on it"
        );
    }

    // The gap between the two, which is 5 wide and bare card.
    let holds = bay.field(Field::Holds).expect("the `holds` field");
    let gap = Point::new(holds.max.x + size::LIB_FILTERS_GAP * 0.5, holds.center().y);
    assert_eq!(
        bay.filter(&view.holds, view.filters(), gap),
        None,
        "the gap between the two fields answered a press"
    );
    assert_eq!(
        claim(&mut panel, &ctx, &view, gap),
        Claim::Egui,
        "the gap between the two fields is claimed by the panel"
    );

    // And the row's own left padding, which is 9 of it.
    let pad = Point::new(row.min.x + 1.0, row.center().y);
    assert_eq!(
        bay.filter(&view.holds, view.filters(), pad),
        None,
        "the row's padding answered a press"
    );
}

/// **Both fields clear every boundary's grab**, which the chip at the end of
/// the scope row above them does not — `.lib-filters` is padded 9 in from each
/// side edge against a `GRAB` of 6, and the row above and the list below are
/// both the bay's own.
#[test]
fn the_filter_fields_clear_every_boundary() {
    let panel = console(PLAUSIBLE);
    let bay = bay(&panel);
    let region = to_egui(rect_of(panel.layout(), "library"));

    for field in Field::ALL {
        let box_ = bay.field(field).expect("the field is drawn");
        for probe in [
            Point::new(box_.min.x + 1.0, box_.center().y),
            Point::new(box_.max.x - 1.0, box_.center().y),
            Point::new(box_.center().x, box_.min.y + 1.0),
            Point::new(box_.center().x, box_.max.y - 1.0),
        ] {
            assert!(
                !matches!(
                    panel.layout().hit(probe, GRAB),
                    karakuri_layout::Hit::Divider { .. }
                ),
                "a boundary grabs {probe:?}, which is inside {field:?}"
            );
        }
        assert!(
            box_.min.x - region.min.x >= size::LIB_FILTERS_PAD_X - f32::EPSILON
                && region.max.x - box_.max.x >= size::LIB_FILTERS_PAD_X - f32::EPSILON,
            "{field:?} is {:?} in a bay spanning {} to {}",
            box_,
            region.min.x,
            region.max.x
        );
    }

    // **And the pixel past the row's right-hand padding is still a
    // boundary's**, which is what says the clearances above are clearances
    // rather than the grab having gone missing.
    let row = bay.filters.expect("the bay draws its filter row");
    assert!(
        matches!(
            panel
                .layout()
                .hit(Point::new(row.max.x - 1.0, row.center().y), GRAB),
            karakuri_layout::Hit::Divider { .. }
        ),
        "the pixel at the far end of the filter row is not the pane divider's, so nothing here \
         measured a grab"
    );
}

/// **What the fields read, and what `View::narrow` does with an answer.**
///
/// Three things the bay could get wrong and one of them is the reason
/// `holds_at` is a position: a host that rewrites the candidates leaves the
/// position pointing past the end, and what that has to read as is **unset**
/// rather than the last candidate there is — a listing narrowed by something
/// nobody chose is the failure this avoids.
#[test]
fn the_fields_read_what_is_set_and_a_stale_candidate_reads_as_unset() {
    use karakuri_operation::Layer;
    let mut view = View::new(karakuri_console::room::Room::Day);
    assert_eq!(view.filters().holds_word(), HOLDS_UNSET);
    assert_eq!(view.filters().layer_word(), LAYER_UNSET);

    view.holds = vec!["drift_shell".to_owned(), "soft_points".to_owned()];
    assert!(view.narrow(Some("soft_points"), Some(Layer::L4)));
    assert_eq!(view.filters().holds_word(), "soft_points");
    assert_eq!(view.filters().layer_word(), "L4");
    assert!(
        !view.narrow(Some("soft_points"), Some(Layer::L4)),
        "narrowing to what it was already narrowed to moved something"
    );

    // **A `holds` this console cannot draw is refused, and refused whole**:
    // the layer beside it is not written either.
    assert!(
        !view.narrow(Some("no_such_node"), Some(Layer::L1)),
        "a filter the field cannot draw was accepted"
    );
    assert_eq!(view.filters().holds_word(), "soft_points");
    assert_eq!(view.filters().layer_word(), "L4");

    // **The candidates go, and the field reads unset** — not `drift_shell`,
    // which is what clamping would have answered.
    view.holds = vec!["drift_shell".to_owned()];
    assert_eq!(view.filters().holds_word(), HOLDS_UNSET);
    assert_eq!(view.filters().holds, None);
    assert_eq!(
        view.filters().layer,
        Some(Layer::L4),
        "the layer is the console's own and did not survive the candidates going"
    );
}

/// **A console that was told about no library draws no filter row**, which is
/// the scope row's own condition rather than a second one: a filter is a
/// question about a listing, and there is no listing to ask it of.
#[test]
fn a_console_with_no_scopes_draws_no_filter_row() {
    let panel = console(PLAUSIBLE);
    let bay = library(panel.layout(), &[], &mock(), None, None).expect("the bay lists its rows");
    assert_eq!(bay.scopes, None);
    assert_eq!(
        bay.filters, None,
        "a bay with no scope row drew a filter row"
    );
    assert_eq!(bay.field(Field::Holds), None);
    assert_eq!(
        bay.filter(
            &[],
            Filters::NONE,
            Point::new(bay.list.min.x, bay.list.min.y)
        ),
        None,
        "a bay with no filter row answered a press on one"
    );
}

// ---------------------------------------------------------------------------
// The reading, and the chip that opens it
// ---------------------------------------------------------------------------

/// **The mock's own reading of `drift_night`**: six knobs, a capacity, what it
/// emits, and five nodes all of which had a card.
///
/// The ranges are the mock's own strings, because that is what crosses the
/// seam — `view::Published::range` is what the host spelled and this crate
/// draws (ADR-0156). What they are spelled *from* is a card's `param` record,
/// and `karakuri/src/main.rs` is where that spelling lives.
fn reading() -> Reading {
    Reading {
        id: "drift_night".to_owned(),
        knobs: [
            ("radius", "0 – 8 · 2"),
            ("turbulence", "0 – 3 · 0.4"),
            ("amount", "0 – 1 · 0.5"),
            ("twist", "−2 – 2 · 0"),
            ("hue", "0 – 1 · 0.5"),
            ("exposure", "0 – 1 · 0.4"),
        ]
        .iter()
        .map(|(key, range)| Published {
            key: (*key).to_owned(),
            range: (*range).to_owned(),
        })
        .collect(),
        capacity: Some("16384 – 1048576 · 262144".to_owned()),
        emits: Some("position, size, life".to_owned()),
        nodes: 5,
        described: 5,
    }
}

/// A view over the mock's library with that reading open under the first row.
fn showing_reading() -> (View, Panel) {
    let (mut view, panel) = showing_mock();
    view.read(reading());
    (view, panel)
}

/// **What a press on the `read` chip asks for is the Set under the cursor**,
/// which is the operand the pill beside it already uses.
///
/// `console.html`'s note: *"Its operand is the cursor, which is the same
/// operand the pill beside it already uses — so the route costs one chip in
/// the foot and nothing else."* So the id in the operation follows the cursor,
/// and this walks it rather than asserting one row: a chip that named the
/// first row whatever was under the cursor would pass half of this.
///
/// **And a second press closes rather than asking again.** That is
/// `view::Read`'s own division — closing changes which rows this bay draws,
/// which is the console's own state and is no more an operation than a fold
/// is — and it is asserted here because it is the half a chip that emitted
/// `ReadSet` twice would get wrong in silence.
#[test]
fn the_read_chip_asks_for_the_set_under_the_cursor() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let chip = bay.read_chip(&ctx, DECK_LETTERS[0]);
    let probe = Point::new(chip.min.x + 2.0, chip.center().y);
    let ask = |view: &View, bay: &LibraryBay| {
        bay.read(
            &ctx,
            DECK_LETTERS[0],
            view.library.get(view.cursor_row()).map(String::as_str),
            probe,
        )
    };

    assert_eq!(
        ask(&view, &bay),
        Some(Read::Open(Operation::ReadSet {
            id: "drift_night".to_owned()
        })),
        "the chip did not ask for the Set under the cursor"
    );

    assert!(view.walk(2, bay.rows), "the cursor did not move two rows");
    assert_eq!(
        ask(&view, &bay),
        Some(Read::Open(Operation::ReadSet {
            id: "glass_shell".to_owned()
        })),
        "the cursor moved and the chip went on naming the row it started on"
    );

    // **The gap before the chip is nobody's**, which is `.lib-foot`'s `gap`
    // rather than a target: the count, the space beside it and the ground
    // between the two capsules take no press.
    let gap = Point::new(chip.min.x - size::LIB_FOOT_GAP * 0.5, chip.center().y);
    assert!(
        ask(&view, &bay).is_some(),
        "the chip itself stopped answering, so the gap beside it measures nothing"
    );
    assert_eq!(
        bay.read(&ctx, DECK_LETTERS[0], Some("drift_night"), gap),
        None,
        "the gap between the `read` chip and the load pill answered a press"
    );

    // **A library that lists nothing has no Set to read**, and the chip is
    // still drawn because the foot is what the count is in. A press on it asks
    // nothing rather than asking for a Set with no name.
    let empty = library(panel.layout(), SCOPES, &[], None, None).expect("the bay draws its foot");
    let chip = empty.read_chip(&ctx, DECK_LETTERS[0]);
    assert_eq!(
        empty.read(
            &ctx,
            DECK_LETTERS[0],
            None,
            Point::new(chip.center().x, chip.center().y)
        ),
        None,
        "a bay listing nothing asked for a Set"
    );

    // **And with a reading open the same press closes it**, which is the
    // block's own state read off the bay rather than off anything the chip is
    // told.
    let (view, panel) = showing_reading();
    let open = library(panel.layout(), SCOPES, &view.library, view.opened(), None)
        .expect("the bay lists its rows");
    assert!(open.reading.is_some(), "nothing was open to close");
    let chip = open.read_chip(&ctx, DECK_LETTERS[0]);
    assert_eq!(
        open.read(
            &ctx,
            DECK_LETTERS[0],
            Some("drift_night"),
            Point::new(chip.min.x + 2.0, chip.center().y)
        ),
        Some(Read::Shut),
        "a press with a reading open asked for it again instead of closing it"
    );
}

/// **The reading opens under the cursor's row, and the rest of the listing
/// moves down by exactly what it takes.**
///
/// `console.html`: *"a reading opens under the cursor row inside `.lib-list`,
/// one row at a time, following the cursor"*, and *"the height it costs lands
/// on the bay in this column that absorbs"*. So the rows above it do not move,
/// the rows below it start under the block's own margin, and the count in the
/// foot falls by however many no longer fit — which is the same `n of m` a
/// library taller than its list already says.
#[test]
fn a_reading_opens_under_the_cursor_row_and_pushes_the_rest_down() {
    let (view, panel) = showing_reading();
    let shut = bay(&panel);
    let open = library(panel.layout(), SCOPES, &view.library, view.opened(), None)
        .expect("the bay lists its rows");
    let block = open.reading.expect("the reading is open under the cursor");

    // Ten rows: the head, six knobs, the capacity, what it emits, and the
    // foot.
    assert_eq!(reading().rows(), 10, "the reading is not ten lines");
    assert_eq!(block.rows, reading().rows(), "the block is not the reading");

    // The well: under the cursor's row by the block's top margin, inside the
    // list by its side margin, and as tall as its rows.
    assert!(
        near(
            block.well.min.y,
            open.row(0).max.y + size::READING_MARGIN_TOP
        ),
        "the well starts at {} and the cursor's row ends at {}",
        block.well.min.y,
        open.row(0).max.y
    );
    assert!(
        near(block.well.min.x, open.list.min.x + size::READING_MARGIN_X)
            && near(block.well.max.x, open.list.max.x - size::READING_MARGIN_X),
        "the well is {:?} in a list spanning {} to {}",
        block.well,
        open.list.min.x,
        open.list.max.x
    );
    assert!(
        near(block.well.height(), size::LIB_ROW_H * block.rows as f32),
        "the well is {} tall and ten rows are {}",
        block.well.height(),
        size::LIB_ROW_H * block.rows as f32
    );

    // Its rows tile it, at a `.lib-row`'s own height.
    for index in 1..block.rows {
        assert!(
            near(block.row(index).min.y, block.row(index - 1).max.y),
            "the reading's row {index} does not meet the one above it"
        );
    }
    assert!(
        near(block.row(block.rows - 1).max.y, block.well.max.y),
        "the reading's last row ends at {} and the well at {}",
        block.row(block.rows - 1).max.y,
        block.well.max.y
    );

    // The row above it has not moved, and the row below it starts under the
    // block's bottom margin.
    assert!(
        near(open.row(0).min.y, shut.row(0).min.y),
        "the row the reading is under moved when it opened"
    );
    assert!(
        near(
            open.row(1).min.y,
            block.well.max.y + size::READING_MARGIN_BOTTOM
        ),
        "the row under the reading is at {} and the block ends at {}",
        open.row(1).min.y,
        block.well.max.y
    );

    // **The five the mock lists all still fit**, which is what makes the
    // paragraph above about a block between two rows rather than about a
    // shorter list. The bay this column gives the Library is taller than five
    // rows and a reading of ten, and nothing was pushed out.
    assert_eq!(
        open.rows, shut.rows,
        "the mock's five rows and a ten-line reading fit the bay, and {} were listed",
        open.rows
    );

    // **A library taller than its list is where the block costs rows**, and
    // what it costs is the block's own height read in rows — the foot says so
    // in the words it already says a truncated listing in.
    let many: Vec<String> = (0..200).map(|n| format!("set_{n:03}")).collect();
    let mut deep = View::new(Room::Day);
    deep.library = many.clone();
    deep.scopes = Scope::ALL.to_vec();
    let mut of_first = reading();
    of_first.id = many[0].clone();
    deep.read(of_first);
    let full = library(panel.layout(), SCOPES, &many, None, None).expect("the bay lists its rows");
    let cut = library(panel.layout(), SCOPES, &many, deep.opened(), None)
        .expect("the bay lists its rows");
    let block = cut.reading.expect("the reading is open");
    assert!(
        full.rows < full.total,
        "200 names all fitted, so nothing here is measuring what a block costs"
    );
    let took = size::READING_MARGIN_TOP + block.well.height() + size::READING_MARGIN_BOTTOM;
    assert!(
        cut.rows < full.rows,
        "the bay listed {} rows with a {took}-tall block open and {} with none",
        cut.rows,
        full.rows
    );
    // **And it is a count of what fits and not a number chosen**: the last row
    // listed is inside the list and the next one would not have been, which is
    // `the_rows_tile_the_list_and_stay_inside_it`'s claim asked of a list with
    // a block in the middle of it.
    assert!(
        !cut.list.contains_rect(cut.row(cut.rows)),
        "row {} at {:?} would have fitted under the block in {:?} and was not drawn",
        cut.rows,
        cut.row(cut.rows),
        cut.list
    );
    assert_eq!(
        cut.count(),
        format!("{} of 200", cut.rows),
        "the foot stopped saying how many of how many"
    );
    assert!(
        cut.list.contains_rect(cut.row(cut.rows - 1)),
        "the last row the bay listed is outside the list"
    );
}

/// **A reading is drawn under the row it is a reading of, and under no
/// other.**
///
/// The listing under an open reading can be rewritten by any press on a scope
/// chip or a filter field, and the cursor is a position in it. So the two are
/// put together in one place — `View::opened` — and what it answers where they
/// have come apart is *nothing open*, which is the list going back to being a
/// list rather than one Set described under another's name.
#[test]
fn a_reading_is_drawn_only_under_the_row_it_is_a_reading_of() {
    let (mut view, panel) = showing_reading();
    let listed = bay(&panel).rows;
    assert_eq!(
        view.opened().map(|open| open.at),
        Some(0),
        "the reading did not open under the row it was read of"
    );

    // The cursor moves and the reading does not follow it by itself: what is
    // under the cursor is a different Set, and a block left drawn there would
    // be `lattice_veil` described as `drift_night`.
    assert!(view.walk(1, listed), "the cursor did not move");
    assert!(
        view.opened().is_none(),
        "the reading of `drift_night` is drawn under `lattice_veil`"
    );
    assert!(view.walk(-1, listed), "the cursor did not move back");
    assert!(
        view.opened().is_some(),
        "the reading did not come back with the cursor"
    );

    // And a listing rewritten under it closes it for the same reason, even
    // where the cursor has not moved at all.
    view.library = vec!["night01".to_owned(), "drift_night".to_owned()];
    assert!(
        view.opened().is_none(),
        "the reading is drawn under whatever took the row it was read of"
    );

    // Putting it away is the console's own state and says whether there was
    // one to put away.
    assert!(view.shut_reading(), "there was nothing open to shut");
    assert!(
        !view.shut_reading(),
        "shutting nothing said it shut something"
    );
    assert!(view.opened().is_none());
}

/// **The `read` chip clears every boundary but the one under the bay**, and
/// the number is the same 4.75 the load pill beside it stands at.
///
/// `input.rs`'s rule 3 gives a boundary first refusal, so what this measures is
/// what that costs here: a `.lib-foot` is 26 tall and a `.pill` is 16.5,
/// centred, so the capsule's own rim is 4.75 off the foot's — and the foot's
/// bottom edge is the bay's, which is a boundary with a `GRAB` of 6. **The rim
/// is the price and the chip is not**: everything from its middle upwards is
/// the panel's, which is the same arrangement the `solo` capsule and the deck
/// preview cells are already in.
#[test]
fn the_read_chip_clears_every_boundary_but_the_one_under_the_bay() {
    let (view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let chip = bay.read_chip(&ctx, DECK_LETTERS[0]);
    let region = to_egui(rect_of(panel.layout(), "library"));

    // The three edges that clear it: the two ends of the capsule and its top.
    for probe in [
        Point::new(chip.min.x + 1.0, chip.center().y),
        Point::new(chip.max.x - 1.0, chip.center().y),
        Point::new(chip.center().x, chip.min.y + 1.0),
    ] {
        assert!(
            !matches!(
                panel.layout().hit(probe, GRAB),
                karakuri_layout::Hit::Divider { .. }
            ),
            "a boundary grabs {probe:?}, which is on the `read` chip"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &view, probe),
            Claim::Panel,
            "the console gave `egui` a press at {probe:?} on the `read` chip"
        );
    }

    // And the rim that does not, with the arithmetic said out loud so that a
    // change to either number is a change to this sentence.
    let rim = region.max.y - chip.max.y;
    assert!(
        near(rim, (size::LIB_FOOT_H - size::PILL_H) * 0.5),
        "the chip's bottom rim is {rim} off the bay's edge and the foot leaves {}",
        (size::LIB_FOOT_H - size::PILL_H) * 0.5
    );
    assert!(
        rim < GRAB,
        "the chip clears the boundary by {rim} against a grab of {GRAB}, so this test is \
         measuring nothing"
    );
    assert!(
        matches!(
            panel
                .layout()
                .hit(Point::new(chip.center().x, chip.max.y - 0.5), GRAB),
            karakuri_layout::Hit::Divider { .. }
        ),
        "the pixel at the bottom of the chip is not the boundary's, so nothing here measured a \
         grab"
    );
}

/// **What a reading draws: a line per declaration, and a word for what it
/// could not read.**
///
/// Asserted by drawing a frame and reading the galleys inside the block,
/// because *what is in the box* is a claim about the paint pass. The three
/// blocks the mock draws are the three here — the knobs, the capacity and the
/// emitted attributes — and the element-storage figure the MCP tool volunteers
/// is deliberately not among them: sizing it fetches every source in the Set
/// and compiles it, which is exactly the cost the row's tip says a reading
/// does not pay.
///
/// **The foot counts the nodes and says how many declared nothing**, which is
/// the one thing that keeps a knob missing for want of a card from being a
/// knob missing in silence.
#[test]
fn a_reading_draws_a_line_per_declaration_and_counts_what_it_could_not_read() {
    let (mut view, mut panel) = showing_reading();
    let open = library(panel.layout(), SCOPES, &view.library, view.opened(), None)
        .expect("the bay lists its rows");
    let block = open.reading.expect("the reading is open");
    let drawn = shapes_inside(&mut view, &mut panel, block.well);
    let words: Vec<String> = drawn
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(at) => Some(at.galley.text().to_owned()),
            _ => None,
        })
        .collect();

    for want in [
        "declares",
        "6 knobs",
        "radius",
        "0 – 8 · 2",
        "exposure",
        "capacity",
        "16384 – 1048576 · 262144",
        "emits",
        "position, size, life",
        "5 nodes",
        "all described",
    ] {
        assert!(
            words.iter().any(|word| word == want),
            "the reading does not say `{want}`: {words:?}"
        );
    }
    // **The well itself**, so a box of type standing on the card cannot pass
    // this: the mock draws these rows on the recess the staging lane's
    // candidates stand on.
    assert!(
        drawn.iter().any(|shape| matches!(
            shape,
            egui::Shape::Rect(at) if near(at.rect.min.y, block.well.min.y)
                && near(at.rect.height(), block.well.height())
        )),
        "nothing filled the reading's well at {:?}: {drawn:#?}",
        block.well
    );
    // **And nothing in it is a figure this reading did not pay for.** The
    // element storage is bytes, and no line here says so.
    assert!(
        !words.iter().any(|word| word.contains("bytes")),
        "the reading draws an element-storage figure, which needs every source in the Set \
         fetched and compiled: {words:?}"
    );

    // A Set with a node the store holds no card for: the knob list is short by
    // whatever that node declared, and the foot is what says so.
    let mut short = reading();
    short.described = 3;
    view.read(short);
    let open = library(panel.layout(), SCOPES, &view.library, view.opened(), None)
        .expect("the bay lists its rows");
    let block = open.reading.expect("the reading is open");
    let drawn = shapes_inside(&mut view, &mut panel, block.well);
    assert!(
        drawn.iter().any(|shape| matches!(
            shape,
            egui::Shape::Text(at) if at.galley.text() == "2 without a card"
        )),
        "two nodes declared nothing this could read and the foot did not say so: {drawn:#?}"
    );
}

/// **A reading with nothing in it is an answer**, and it is drawn as one: the
/// head says `0 knobs` and the rows that would have nothing after them are not
/// drawn at all.
///
/// A blank row is the one thing the mock's own tip refuses — *"a knob missing
/// from the list without a word is the one thing that would make this lie"* —
/// so a Set with no geometry in it draws no capacity line rather than a line
/// with nothing beside it.
#[test]
fn a_reading_with_nothing_to_declare_says_so_rather_than_drawing_blanks() {
    let (mut view, mut panel) = showing_mock();
    view.read(Reading {
        id: "drift_night".to_owned(),
        knobs: Vec::new(),
        capacity: None,
        emits: None,
        nodes: 1,
        described: 1,
    });
    let open = library(panel.layout(), SCOPES, &view.library, view.opened(), None)
        .expect("the bay lists its rows");
    let block = open.reading.expect("the reading is open");
    assert_eq!(block.rows, 2, "a head and a foot are the whole of it");

    let drawn = shapes_inside(&mut view, &mut panel, block.well);
    let words: Vec<String> = drawn
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(at) => Some(at.galley.text().to_owned()),
            _ => None,
        })
        .collect();
    assert!(
        words.iter().any(|word| word == "0 knobs") && words.iter().any(|word| word == "1 node"),
        "the head and the foot do not say what there is: {words:?}"
    );
    assert!(
        !words
            .iter()
            .any(|word| word == "capacity" || word == "emits"),
        "a row was drawn with nothing after it: {words:?}"
    );
}

/// **The chip reads `read` and does not light**, open or shut.
///
/// `console.html`'s note: *"It carries no lit state of its own, because the
/// block above it is the state … the one colour this bay spends on an act is
/// spent on the load beside it."* So this asserts the word is painted in the
/// foot and that the capsule's own fill is not there in either state — a
/// second lav capsule beside `load → A` would make the colour mean two things
/// at a width of eight characters.
#[test]
fn the_read_chip_says_read_and_never_lights() {
    let (mut view, mut panel) = showing_mock();
    for open in [false, true] {
        if open {
            view.read(reading());
        }
        let foot = bay(&panel).foot;
        let drawn = shapes_inside(&mut view, &mut panel, foot);
        assert!(
            drawn.iter().any(|shape| matches!(
                shape,
                egui::Shape::Text(at) if at.galley.text() == READ_WORD
            )),
            "the foot does not say `{READ_WORD}` with the reading {}: {drawn:#?}",
            match open {
                true => "open",
                false => "shut",
            }
        );
        let ctx = drawn_once();
        let chip = bay(&panel).read_chip(&ctx, DECK_LETTERS[0]);
        assert!(
            !drawn.iter().any(|shape| matches!(
                shape,
                egui::Shape::Rect(at) if near(at.rect.min.x, chip.min.x)
                    && near(at.rect.width(), chip.width())
                    && at.fill != egui::Color32::TRANSPARENT
            )),
            "the `read` chip is filled, which is the one colour this bay spends on the load"
        );
    }
}

/// The word the chip is drawn with, which is the mock's own and is not a
/// constant this crate exports: it is asserted here because a chip reading
/// something else is a control the note does not describe.
const READ_WORD: &str = "read";
