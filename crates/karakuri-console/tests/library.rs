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
use karakuri_console::input::{claim, wheeled, Claim, Turned};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    library, mcp_pill, Aim, Field, Filters, KindChip, LibraryBay, Opened, Picked, Published, Read,
    Reading, RowItem, RowKind, Rows, Scope, Target, View, DECK_LETTERS, HOLDS_UNSET, LAYERS,
};

/// **A listing of Sets and nothing else**, which is what every test in this
/// file that says nothing about kinds is about: a row with no entry in the
/// kinds beside it is a Set with no badge, which is what this bay drew before
/// ADR-0338 (`view::RowKind`).
fn listed(names: &[String]) -> Rows<'_> {
    Rows { names, kinds: &[] }
}
use karakuri_layout::{Point, Rect};
use karakuri_operation::gate::{Class, Open};
use karakuri_operation::{Operation, SetTransfer};

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

/// **What the foot's load control is aimed at**, everywhere in this file that
/// is not about the pulldown: deck A, no strips behind it and the list shut,
/// which is what [`View::new`] answers and what every console here that has
/// not been handed a mixer is.
const AIMED: Target = Target {
    deck: 0,
    decks: 0,
    open: false,
};

/// The bay, laid out with the mock's names.
fn bay(panel: &Panel) -> LibraryBay {
    library(panel.layout(), SCOPES, &mock(), None, None, 0.0)
        .expect("the library bay lists its rows")
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

/// **The bay is drawn where the bay can hold it, and nowhere else** —
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

// ---------------------------------------------------------------------------
// Which of this bay's parts answer a press, and nothing is stored
// ---------------------------------------------------------------------------

/// **The scope chips answer a press, and so do the two filter fields, the
/// three capsules in the foot and the rows of the list — and nothing else in
/// this bay does.**
///
/// The mock draws four scope chips, two filter fields, a `+`, a row cursor,
/// and the foot's `read`, `load`, `→` and `A ▾`. **The chips are the first of
/// them that is a control here**, and the reason is the page rather than the code: `console.html`
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
/// **The `params` chip in the foot is the third**, and it is the one control in
/// this bay that is not in its head: the chips say which library and the
/// fields narrow it, where this reads the row the cursor is on.
/// `console.html`'s note *Reading a Set before you spend a load on it* is what
/// puts it there — *"a `params` chip sits in the foot between the count and
/// `load → A`"* — and what a press on it asks is
/// [`the_params_chip_asks_for_the_set_under_the_cursor`]'s. What this test adds
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
/// **The `load` button and the deck pulldown beside it are the fifth and the
/// sixth**, and they were one readout until 2026-09-08: `load → A` said where
/// the *key* would land and answered no pointer at all. ADR-0305 split it, so
/// what this test says about them is what it says about the `params` chip — and
/// what is left in the foot that is *not* a control is the `→` between them,
/// which is swept with the ground. Where each of the two lands is
/// [`a_press_on_load_asks_for_the_deck_the_pulldown_names`]'s and
/// [`a_pick_in_the_pulldown_names_a_deck_and_asks_for_nothing`]'s; the label's
/// own silence is
/// [`the_label_between_the_two_capsules_takes_no_press_and_a_row_is_where_the_drag_begins`]'s.
///
/// # The bay's corners do not reach the foot's capsules, and the two numbers say why
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
fn the_chips_the_fields_the_params_chip_and_the_rows_are_the_bays_controls_and_nothing_else_is() {
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
    // **The three capsules at the far end of the row are stepped over**, and
    // all three are controls: the `params` chip and the `load` button are asked
    // about below, and the pulldown is
    // [`the_label_between_the_two_capsules_takes_no_press_and_a_row_is_where_the_drag_begins`]'s.
    // What is left is the foot's ground — the count, the gap between it and
    // them, and the `→` label between the two capsules, which is the one thing
    // in this row that is drawn and is not a control.
    let chip = bay.params_chip(&ctx, AIMED);
    let load = bay.load(&ctx, AIMED);
    let mut swept = 0;
    for step in 0..=10 {
        let t = step as f32 / 10.0;
        let p = egui::pos2(left + (right - left) * t, bay.foot.center().y);
        if chip.contains(p) || load.button.contains(p) || load.deck.contains(p) {
            continue;
        }
        points.push(p);
        swept += 1;
    }
    assert!(
        swept >= 4,
        "only {swept} points of the foot's ground were swept, and the three capsules cannot be \
         most of a row this wide"
    );
    // **The label's own box**, asked explicitly rather than left to the sweep
    // above: it is between two controls and a press on it must belong to
    // neither of them.
    points.push(load.arrow.center());
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

    // **And the three capsules in the foot are the panel's**, each asked at
    // the same two pixels in from its own left edge the chips above are —
    // their right-hand ends are the gaps between them and their bottom rims
    // are under the boundary's grab, which is
    // [`the_params_chip_clears_every_boundary_but_the_one_under_the_bay`]'s
    // subject and not this test's.
    for (what, capsule) in [("read", chip), ("load", load.button), ("deck", load.deck)] {
        let probe = egui::pos2(capsule.min.x + 2.0, capsule.center().y);
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(probe.x, probe.y)),
            Claim::Panel,
            "the console gave `egui` a press on the `{what}` capsule at {probe:?}"
        );
    }

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

/// **The label between the two capsules takes no press, and a row is where the
/// drag begins.**
///
/// **This test's premise moved on 2026-09-08 and the half that is gone is
/// named here rather than deleted**, because a reader meeting it will
/// otherwise re-propose what it used to say. It was *the load pill takes no
/// press*: `load → A` was a readout, the *key*'s route was *"a cursor and a
/// key with no pointer anywhere in it"*, and a press on the capsule would have
/// added a pointer to the one gesture both pages described as having none.
/// ADR-0305 split the readout into a button, a label and a pulldown, so two of
/// those three now take a press and are asserted in
/// [`the_chips_the_fields_the_params_chip_and_the_rows_are_the_bays_controls_and_nothing_else_is`].
///
/// **What is left of the first half is the label**, and it is the same
/// sentence about a smaller thing: the `→` is punctuation on the foot's own
/// ground, untipped in the mock like the `5 of 27` at the other end of the
/// row, and a press on it belongs to neither capsule beside it.
///
/// **The second half is unchanged.** The panel's own route into *Load material
/// into a deck* was and is the drag — `console.html` calls it *"a third route
/// to the same command, and never the first"* — so the row a hand presses is
/// the panel's, and what a row press asks for is `carry.rs`'s.
///
/// The label's own box is asked rather than the foot's middle, because the box
/// is where a press would land: it is measured off the same derivation the
/// paint lays out, so the rectangle asserted here and the mark drawn are one
/// statement.
#[test]
fn the_label_between_the_two_capsules_takes_no_press_and_a_row_is_where_the_drag_begins() {
    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(4).collect();

    // **The three, as wide as what is in them** — asked of `LibraryBay::load`,
    // which is the derivation the paint uses, so the boxes swept here are the
    // boxes drawn.
    let load = bay.load(&ctx, view.target());
    for (what, box_) in [
        ("load", load.button),
        ("→", load.arrow),
        ("deck", load.deck),
    ] {
        assert!(
            bay.foot.contains_rect(box_),
            "the `{what}` box at {box_:?} is not inside the foot at {:?}",
            bay.foot
        );
    }

    // **Across the label at the foot's own centre line**, and not its top and
    // bottom edges: `.lib-foot` is 26 tall and a `.pill` is 16.5, so a
    // capsule's edges are 4.75 off the foot's — and the foot's bottom edge is
    // the bay's, which is a boundary. 4.75 against a `GRAB` of 6 means a
    // boundary would take a press on a capsule's own rim, which is
    // `input.rs`'s rule 3 and is the price every capsule in this foot pays.
    let points: Vec<egui::Pos2> = (0..=4)
        .map(|step| {
            let t = step as f32 / 4.0;
            egui::pos2(
                load.arrow.min.x + load.arrow.width() * t,
                load.arrow.center().y,
            )
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
            "the console took the pointer at {p:?} — the `→` says how to read the two capsules \
             either side of it and is not one of them"
        );
    }

    // **And the contrast, on the two capsules the label sits between**, so
    // that this cannot pass by a foot that takes no press anywhere: the label
    // is `egui`'s and both capsules are the panel's, in one console in one
    // state.
    for (what, capsule) in [("load", load.button), ("deck", load.deck)] {
        let at = capsule.center();
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(at.x, at.y)),
            Claim::Panel,
            "the `{what}` capsule beside the label went to `egui`, so the sweep above is \
             measuring a foot that takes no press at all"
        );
    }

    // **And the row the drag begins on**, which is the half of this test that
    // did not move.
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
        bay.starred(listed(&mock()), &none, Point::new(at.x, at.y)),
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
        bay.starred(listed(&mock()), &one, Point::new(at.x, at.y)),
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
        bay.starred(listed(&mock()), &none, Point::new(ground.x, ground.y)),
        None,
        "the star answered a press at the far end of its row"
    );

    // **And a listing shorter than the rows drawn takes nothing**, which is
    // `take`'s refusal rather than a clamp: a star answered bare would name a
    // Set nobody can see.
    assert_eq!(
        bay.starred(Rows::NONE, &none, Point::new(at.x, at.y)),
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
        // **Four chips ask one row and the fifth asks another**, which is
        // `view::Chosen`'s own section: `history` is not a library of Sets and
        // *Walk the edit history* is the row that describes what a press on it
        // asked for. A `SelectScope` there would be indistinguishable from a
        // press on `all`, because that payload cannot say which.
        assert_eq!(
            chosen.operation,
            match scope {
                Scope::History => karakuri_operation::Operation::WalkHistory {
                    step: karakuri_operation::Undecided
                },
                _ => karakuri_operation::Operation::SelectScope {
                    scope: karakuri_operation::Undecided
                },
            },
            "a press on `{}` asked for the wrong row of the page",
            scope.name()
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
/// hold every chip would make the rest of this test measure nothing.
///
/// **The clipped chip is looked for rather than assumed to be the last one**,
/// which is ADR-0308's fifth chip arriving: `history` starts past the row's
/// own right edge at this width and is not drawn at all, so the capsule this
/// test is about — the one that starts inside and finishes outside — is
/// `folder`. Asking for the straddling chip is the property; asking for the
/// last one was an arithmetic that happened to name it.
#[test]
fn a_chip_is_pressed_only_where_it_is_drawn() {
    let mut layout = solved(PLAUSIBLE);
    let left = id_of(&layout, "left-pane");
    let split = layout.parent(left).expect("left-pane has a parent split");
    layout.set_divider(split, 0, 190.0);
    layout.solve();
    let ctx = drawn_once();
    let bay =
        library(&layout, SCOPES, &mock(), None, None, 0.0).expect("the library bay lists its rows");
    let row = bay.scopes.expect("the bay was handed scopes");

    let (scope, last) = bay
        .chips(&ctx, SCOPES)
        .find(|(_, chip)| chip.min.x < row.max.x && chip.max.x > row.max.x)
        .expect(
            "no chip straddles the row's right edge — the pane is wide enough to hold every \
             one of them, so there is no clipped capsule to ask about",
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
        library(panel.layout(), SCOPES, &one, None, None, 0.0).map(|b| b.total),
        Some(1)
    );
    assert_eq!(
        library(panel.layout(), SCOPES, &two, None, None, 0.0).map(|b| b.total),
        Some(2)
    );
    assert_eq!(
        library(panel.layout(), SCOPES, &one, None, None, 0.0).map(|b| b.total),
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
/// them** — which is the bay's own row: `all`, `my sets`, `presets`, `folder`,
/// `history`.
///
/// **It is the mock's row with its first chip renamed and moved**, which is
/// [ADR-0299](../../../docs/adr/0299-my-sets-is-the-starred-subset-and-the-star-is-kept-beside-the-sets.md):
/// `my sets` is the starred subset now, so the chip that lists everything the
/// store holds is `all` and it comes first, because it is the listing the
/// other three are questions about.
///
/// **`history` is last and is not a library of Sets** (ADR-0308): its rows are
/// the versions of the Set the load pulldown's deck is running, which is why it
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

/// **The foot says where a press would land, and the letter follows the
/// pulldown rather than the deck selection.**
///
/// `console.html`: *"The letter in the pulldown is the whole warning … What the
/// control owes instead is to say where it lands before the press."* So this
/// asserts what is painted rather than a rectangle, and asserts it again after
/// the mark moves — a foot that read `A` whatever was aimed at would pass the
/// first half and be a lie for the other three decks.
///
/// **And it asserts which of the two marks the letter is**, which is the whole
/// of ADR-0305: the letter used to be [`View::selection`], so a test that only
/// walked one mark would pass against the readout this replaced. The second
/// half moves the *selection* with the target standing still and reads the
/// foot again.
///
/// **The letter is its own galley**, because the arrow beside it is drawn
/// rather than typed — see [`the_foots_arrow_is_drawn_rather_than_typed`].
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

/// The viewport the pulldown's list is held inside, as `egui`'s rectangle —
/// the same one `View::draw` hands it.
fn viewport(panel: &Panel) -> egui::Rect {
    to_egui(panel.layout().viewport())
}

/// **A press on `load` asks for the deck the pulldown names, and never the one
/// the selection is on.**
///
/// This is ADR-0305's decision at the seam it crosses: the operation carries
/// the deck, and which deck it carries is the whole of what the split bought.
/// **The two marks are pulled apart before the press** — the keys addressed to
/// deck C, the load aimed at deck B — because a console where they agree
/// cannot tell the two readings apart, and that is exactly the state the
/// readout this replaced was always in.
///
/// **And a press with no row under the cursor says so rather than emitting.**
/// A load names a Set and a deck; with the listing empty there is no Set, and
/// a `LoadSet` carrying a name nobody chose would be worse than a press that
/// declines out loud (`Aim::NoSet`, and P-0083).
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

/// **A pick in the pulldown names a deck, asks for nothing, and moves neither
/// the ring nor the cursor.**
///
/// Three claims because they are the three the control owes: the capsule puts
/// the list down, a row of that list is `Aim::Deck` and no `Operation` at all,
/// and performing it leaves [`View::selection`] where it was. The third is the
/// one a reader will doubt — *surely picking a deck selects it* — and it is
/// exactly what ADR-0305 refused: `Operation::SelectDeck` moves the keys, and
/// this mark is the one that does not.
///
/// **The list goes away with the pick**, which is the gesture ending: nothing
/// is emitted, so there is no host arm to end it in, and a card left down
/// would go on claiming every press on the console (`input::claim`'s rule 2).
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

/// **The pulldown offers the decks the mixer is drawing strips for, and no
/// others** — and a console with no strip at all cannot put a list down.
///
/// `console.html`: *"A deck the mixer is drawing no strip for is not in the
/// list, which is the count `0`–`3` are refused on"*. So this is
/// [`View::select`]'s own refusal read a second time, asserted in both
/// directions: three strips list three decks, and deck D is turned down rather
/// than clamped to the last one there is.
///
/// **The empty case is the one that would bite**, and it is why `open_target`
/// refuses: a card with no rows in it offers nothing to pick and nothing to
/// leave by, and rule 2 would hand it every press on the console until a
/// second press shut it.
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

/// **While the list is down, every press on the console is part of that
/// gesture.**
///
/// `input::claim`'s rule 2, which the two cards in the transport row are
/// already under: the card is drawn over this bay's own rows, so a press
/// inside it belongs to the card and a press anywhere else is the dismissal.
/// Both halves are asserted, because a rule that only claimed the card would
/// leave the first press outside it doing whatever it does the rest of the
/// time — loading a deck, or moving a fader.
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
// A row's badges, and what a procedure row is
// ---------------------------------------------------------------------------

/// **The mock's own listing with a procedure in it**, which is the bay
/// ADR-0338 draws: four Sets carrying the layers their files fill, and
/// `orbit_wide` between two of them carrying its one `kind`.
fn with_a_procedure() -> (Vec<String>, Vec<RowKind>) {
    use karakuri_operation::Layer;
    let set = |badges: Vec<Layer>| RowKind {
        badges,
        procedure: false,
    };
    (
        [
            "drift_night",
            "lattice_veil",
            "orbit_wide",
            "glass_shell",
            "night01",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect(),
        vec![
            set(vec![Layer::L1, Layer::L2, Layer::L4]),
            set(vec![Layer::L1, Layer::L4]),
            RowKind {
                badges: vec![Layer::L3],
                procedure: true,
            },
            set(vec![Layer::L1, Layer::L4]),
            set(vec![Layer::L1, Layer::L2, Layer::L4]),
        ],
    )
}

/// **A row carries its kinds, and the words are the chips' own** — a Set's are
/// the layers its files fill and a procedure's is the one kind it declares
/// (ADR-0338).
///
/// **And a row with no kinds beside it is a Set with no badge**, which is what
/// the seam's default buys: every other test in this file hands a listing and
/// no kinds, and this asserts that reads as it did before this row existed.
#[test]
fn a_row_carries_the_words_of_the_layers_it_implements() {
    let (names, kinds) = with_a_procedure();
    let rows = Rows {
        names: &names,
        kinds: &kinds,
    };
    assert_eq!(rows.badges(0), vec!["L1", "L2", "L4"]);
    assert_eq!(rows.badges(2), vec!["L3"]);
    assert!(
        rows.procedure(2),
        "the procedure row does not say it is one"
    );
    assert!(!rows.procedure(0), "a Set row says it is a procedure");

    // **The star, the reading and the send take a Set and a procedure row hands
    // them nothing**, which is one question rather than three.
    assert_eq!(rows.set(0), Some("drift_night"));
    assert_eq!(rows.set(2), None, "a procedure row was offered as a Set id");
    assert_eq!(rows.set(9), None, "a row past the end was offered as a Set");

    let bare = listed(&names);
    assert!(
        bare.badges(0).is_empty(),
        "a row with no kinds drew a badge"
    );
    assert!(
        !bare.procedure(2),
        "a row with no kinds beside it is not a Set"
    );
    assert_eq!(bare.set(2), Some("orbit_wide"));
}

/// **The badges are laid out from the right of the row, one gap apart**, which
/// is where the mock puts them: after the name and before the row's own
/// padding, so a longer name never moves them.
#[test]
fn the_badges_end_where_the_rows_padding_starts() {
    let panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let (names, kinds) = with_a_procedure();
    let rows = Rows {
        names: &names,
        kinds: &kinds,
    };
    let row = bay.row(0);
    let drawn: Vec<(&str, egui::Rect)> = bay.badges(&ctx, 0, &rows.badges(0)).collect();
    assert_eq!(
        drawn.iter().map(|(word, _)| *word).collect::<Vec<_>>(),
        vec!["L1", "L2", "L4"]
    );
    assert!(
        near(
            drawn[2].1.max.x,
            row.max.x - karakuri_console::room::size::LIB_ROW_PAD_X
        ),
        "the last badge ends at {} in a row ending at {}",
        drawn[2].1.max.x,
        row.max.x
    );
    for pair in drawn.windows(2) {
        assert!(
            near(
                pair[1].1.min.x - pair[0].1.max.x,
                karakuri_console::room::size::BADGE_GAP
            ),
            "{:?} and {:?} are not one gap apart",
            pair[0].0,
            pair[1].0
        );
    }
    assert!(
        near(drawn[0].1.height(), karakuri_console::room::size::BADGE_H)
            && drawn[0].1.center().y == row.center().y,
        "a badge is {:?} in a row centred at {}",
        drawn[0].1,
        row.center().y
    );
    // A row with no badges draws none, and asks `egui` for nothing to say so.
    assert_eq!(bay.badges(&ctx, 1, &[]).count(), 0);
}

/// **A procedure row has no star, and every load off it names
/// `LoadProcedure`** — the button, the row menu's items and the carry, which
/// are the three routes ADR-0338 names.
#[test]
fn a_procedure_row_has_no_star_and_loads_one_layer() {
    let panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let (names, kinds) = with_a_procedure();
    let rows = Rows {
        names: &names,
        kinds: &kinds,
    };
    let none = std::collections::BTreeSet::new();

    // The star on the procedure row answers nothing, and the star on the Set
    // row above it still does.
    let star = bay.star(2);
    assert_eq!(
        bay.starred(rows, &none, Point::new(star.center().x, star.center().y)),
        None,
        "a press on a procedure row's star asked for a favourite"
    );
    let star = bay.star(0);
    assert_eq!(
        bay.starred(rows, &none, Point::new(star.center().x, star.center().y)),
        Some(Operation::SetFavourite {
            id: "drift_night".to_owned(),
            favourite: true
        })
    );

    // The `load` button, with the cursor on the procedure row.
    let load = bay.load(&ctx, AIMED);
    assert_eq!(
        bay.aim(
            &ctx,
            viewport(&panel),
            AIMED,
            rows,
            2,
            Point::new(load.button.center().x, load.button.center().y)
        ),
        Some(Aim::Load(Operation::LoadProcedure {
            deck: 0,
            procedure: "orbit_wide".to_owned()
        })),
        "the `load` button named a Set load on a procedure row"
    );

    // A row menu's `Load to Slot B`, and the menu on such a row carries no
    // send at all — a bare `.kir` is not a Set and nothing takes one in.
    let at = karakuri_console::view::Menued {
        row: Some(2),
        decks: 4,
        sends: false,
    };
    let menu = bay
        .menu(&ctx, viewport(&panel), at)
        .expect("the menu is down");
    assert_eq!(menu.save, None, "a procedure row's menu carries a send");
    assert_eq!(menu.rule, None, "a procedure row's menu draws a separator");
    let item = menu.load(1);
    assert_eq!(
        bay.menu_ask(
            &ctx,
            viewport(&panel),
            at,
            rows,
            Point::new(item.center().x, item.center().y)
        ),
        Some(Picked::Load(Operation::LoadProcedure {
            deck: 1,
            procedure: "orbit_wide".to_owned()
        }))
    );

    // And the carry, which names the operand and says which load a drop will
    // ask for.
    let row = bay.row(2);
    let taken = bay
        .take(rows, Point::new(row.center().x, row.center().y))
        .expect("the row was not taken in hand");
    assert_eq!(taken.set, "orbit_wide");
    assert!(taken.procedure, "the carry does not say it is a procedure");
}

// ---------------------------------------------------------------------------
// The filter field and the six kind chips
// ---------------------------------------------------------------------------

/// **The six chips are the five kinds a procedure declares and the Sets**, each
/// spelled once, which is what makes a badge and the chip that filters by it
/// read the same word.
///
/// The match is the enforcement rather than the assertion under it: a variant
/// added to `karakuri_operation::Layer` does not compile here until somebody has
/// decided what this row calls it and where in it the chip goes. `L5` is
/// ADR-0340's own pass and is deliberately not on this row yet.
#[test]
fn the_kind_chips_are_the_five_kinds_and_the_sets() {
    use karakuri_operation::Layer;
    for layer in [Layer::L1, Layer::L2, Layer::L3, Layer::L4, Layer::Field] {
        let spelled = match layer {
            Layer::L1 => "L1",
            Layer::L2 => "L2",
            Layer::L3 => "L3",
            Layer::L4 => "L4",
            Layer::Field => "FIELD",
            Layer::L5 => unreachable!("L5 is not on this row"),
        };
        let found = LAYERS
            .iter()
            .find(|(kind, _)| *kind == layer)
            .unwrap_or_else(|| panic!("{layer:?} has no word on the kind row"));
        assert_eq!(found.1, spelled, "{layer:?} is spelled two ways");
        assert_eq!(
            KindChip::Layer(layer).word(),
            spelled,
            "the chip and the badge spell {layer:?} two ways"
        );
    }
    assert_eq!(
        LAYERS.len(),
        5,
        "the row draws a chip per layer and no more"
    );
    assert_eq!(
        KindChip::ALL.len(),
        6,
        "the row is five kinds and the Sets — {} chips",
        KindChip::ALL.len()
    );
    assert_eq!(KindChip::Sets.word(), "SET");
}

/// **A press on a kind chip names all six**, which is what makes the row a
/// destination rather than six statements two hands can disagree about
/// (ADR-0338).
///
/// The whole row, one chip at a time: each press turns its own chip on and says
/// what every other chip is, and a second press on the same chip turns it off
/// again — so none-on, which shows everything, is a state a press can always
/// get back to.
#[test]
fn a_press_on_a_kind_chip_names_all_six_and_turns_that_one_over() {
    use karakuri_operation::{LibraryKinds, Operation};
    let panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let boxes: Vec<(KindChip, egui::Rect)> = bay.kind_chips(&ctx).collect();
    let mut at = LibraryKinds::EVERYTHING;
    for (chip, box_) in &boxes {
        let asked = bay
            .kind(
                &ctx,
                Filters {
                    holds: None,
                    kinds: at,
                },
                Point::new(box_.center().x, box_.center().y),
            )
            .expect("the chip did not answer a press on it");
        let Operation::FilterLibrary { kinds } = asked else {
            panic!("a kind chip named something other than `FilterLibrary`");
        };
        assert!(chip.on(kinds), "a press on {chip:?} did not turn it on");
        for (other, _) in &boxes {
            if other != chip {
                assert_eq!(
                    other.on(kinds),
                    other.on(at),
                    "a press on {chip:?} moved {other:?}"
                );
            }
        }
        at = kinds;
    }
    assert!(
        at.narrowing(),
        "six presses left the row showing everything"
    );
    // And back off again, chip by chip, to the state a run opens in.
    for (chip, box_) in &boxes {
        let asked = bay
            .kind(
                &ctx,
                Filters {
                    holds: None,
                    kinds: at,
                },
                Point::new(box_.center().x, box_.center().y),
            )
            .expect("the chip did not answer a press on it");
        let Operation::FilterLibrary { kinds } = asked else {
            panic!("a kind chip named something other than `FilterLibrary`");
        };
        assert!(
            !chip.on(kinds),
            "a press on a lit {chip:?} did not turn it off"
        );
        at = kinds;
    }
    assert_eq!(
        at,
        LibraryKinds::EVERYTHING,
        "the row cannot be pressed back to showing everything"
    );
}

/// **The row's own ground answers nothing**, which is the scope row's rule one
/// band down: the gaps between the chips and the padding either side are bare
/// card, and a bay with no kind row at all answers no press anywhere in it.
#[test]
fn the_kind_rows_own_ground_is_nobodys() {
    let panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let row = bay.kinds.expect("the bay draws its kind row");
    let boxes: Vec<(KindChip, egui::Rect)> = bay.kind_chips(&ctx).collect();
    let gap = Point::new((boxes[0].1.max.x + boxes[1].1.min.x) * 0.5, row.center().y);
    assert_eq!(
        bay.kind(&ctx, Filters::NONE, gap),
        None,
        "the gap between two chips answered a press"
    );
    assert_eq!(
        bay.kind(
            &ctx,
            Filters::NONE,
            Point::new(row.min.x + 1.0, row.center().y)
        ),
        None,
        "the row's left padding answered a press"
    );

    let bare = library(panel.layout(), &[], &mock(), None, None, 0.0).expect("the bay lists rows");
    assert_eq!(bare.kinds, None, "a bay with no scope row drew a kind row");
    assert_eq!(
        bare.kind(&ctx, Filters::NONE, gap),
        None,
        "a bay with no kind row answered a press on one"
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

    // The `holds` field, over the two candidates and back to unset — and with
    // the kind row narrowed, which the press must leave exactly where it is:
    // `ListSets` carries no kinds, and the two controls on this row are two
    // questions (ADR-0338).
    let narrowed = karakuri_operation::LibraryKinds {
        l3: true,
        ..karakuri_operation::LibraryKinds::EVERYTHING
    };
    let mut set = None;
    for want in [Some("drift_shell"), Some("soft_points"), None] {
        let asked = bay
            .filter(
                &holds,
                Filters {
                    holds: set,
                    kinds: narrowed,
                },
                at(Field::Holds),
            )
            .expect("the `holds` field did not answer a press on it");
        assert_eq!(
            asked,
            karakuri_operation::Operation::ListSets {
                holds: want.map(str::to_owned),
                // **The layer goes out unset and the field it came from is
                // gone**: `Operation::ListSets` keeps the field for
                // `list_sets` and `--list-sets`, and this console stopped
                // asking it (ADR-0338).
                layer: None
            },
            "the `holds` field stepped from {set:?} to something else"
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
    use karakuri_operation::LibraryKinds;
    let cameras = LibraryKinds {
        l3: true,
        ..LibraryKinds::EVERYTHING
    };
    let mut view = View::new(karakuri_console::room::Room::Day);
    assert_eq!(view.filters().holds_word(), HOLDS_UNSET);
    assert_eq!(view.filters().kinds, LibraryKinds::EVERYTHING);

    view.holds = vec!["drift_shell".to_owned(), "soft_points".to_owned()];
    assert!(view.narrow(Some("soft_points"), cameras));
    assert_eq!(view.filters().holds_word(), "soft_points");
    assert_eq!(view.filters().kinds, cameras);
    assert!(
        !view.narrow(Some("soft_points"), cameras),
        "narrowing to what it was already narrowed to moved something"
    );

    // **A `holds` this console cannot draw is refused, and refused whole**:
    // the kinds beside it are not written either.
    assert!(
        !view.narrow(Some("no_such_node"), LibraryKinds::EVERYTHING),
        "a filter the field cannot draw was accepted"
    );
    assert_eq!(view.filters().holds_word(), "soft_points");
    assert_eq!(view.filters().kinds, cameras);

    // **The candidates go, and the field reads unset** — not `drift_shell`,
    // which is what clamping would have answered.
    view.holds = vec!["drift_shell".to_owned()];
    assert_eq!(view.filters().holds_word(), HOLDS_UNSET);
    assert_eq!(view.filters().holds, None);
    assert_eq!(
        view.filters().kinds,
        cameras,
        "the kinds are the console's own and did not survive the candidates going"
    );
}

/// **A console that was told about no library draws no filter row**, which is
/// the scope row's own condition rather than a second one: a filter is a
/// question about a listing, and there is no listing to ask it of.
#[test]
fn a_console_with_no_scopes_draws_no_filter_row() {
    let panel = console(PLAUSIBLE);
    let bay =
        library(panel.layout(), &[], &mock(), None, None, 0.0).expect("the bay lists its rows");
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

/// **What a press on the `params` chip asks for is the Set under the cursor**,
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
fn the_params_chip_asks_for_the_set_under_the_cursor() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let chip = bay.params_chip(&ctx, AIMED);
    let probe = Point::new(chip.min.x + 2.0, chip.center().y);
    let ask = |view: &View, bay: &LibraryBay| {
        bay.read(&ctx, AIMED, view.rows().set(view.cursor_row()), probe)
    };

    assert_eq!(
        ask(&view, &bay),
        Some(Read::Open(Operation::ReadSet {
            id: "drift_night".to_owned()
        })),
        "the chip did not ask for the Set under the cursor"
    );

    assert!(
        view.walk(2, bay.drawn()),
        "the cursor did not move two rows"
    );
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
        bay.read(&ctx, AIMED, Some("drift_night"), gap),
        None,
        "the gap between the `params` chip and the `load` button answered a press"
    );

    // **A library that lists nothing has no Set to read**, and the chip is
    // still drawn because the foot is what the count is in. A press on it asks
    // nothing rather than asking for a Set with no name.
    let empty =
        library(panel.layout(), SCOPES, &[], None, None, 0.0).expect("the bay draws its foot");
    let chip = empty.params_chip(&ctx, AIMED);
    assert_eq!(
        empty.read(
            &ctx,
            AIMED,
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
    let open = library(
        panel.layout(),
        SCOPES,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
    .expect("the bay lists its rows");
    assert!(open.reading.is_some(), "nothing was open to close");
    let chip = open.params_chip(&ctx, AIMED);
    assert_eq!(
        open.read(
            &ctx,
            AIMED,
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
    let open = library(
        panel.layout(),
        SCOPES,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
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
    let full =
        library(panel.layout(), SCOPES, &many, None, None, 0.0).expect("the bay lists its rows");
    let cut = library(panel.layout(), SCOPES, &many, deep.opened(), None, 0.0)
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
    assert!(view.walk(1, 0..listed), "the cursor did not move");
    assert!(
        view.opened().is_none(),
        "the reading of `drift_night` is drawn under `lattice_veil`"
    );
    assert!(view.walk(-1, 0..listed), "the cursor did not move back");
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

/// **The `params` chip clears every boundary but the one under the bay**, and
/// the number is the same 4.75 the two capsules beside it stand at.
///
/// `input.rs`'s rule 3 gives a boundary first refusal, so what this measures is
/// what that costs here: a `.lib-foot` is 26 tall and a `.pill` is 16.5,
/// centred, so the capsule's own rim is 4.75 off the foot's — and the foot's
/// bottom edge is the bay's, which is a boundary with a `GRAB` of 6. **The rim
/// is the price and the chip is not**: everything from its middle upwards is
/// the panel's, which is the same arrangement the `solo` capsule and the deck
/// preview cells are already in.
#[test]
fn the_params_chip_clears_every_boundary_but_the_one_under_the_bay() {
    let (view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let chip = bay.params_chip(&ctx, AIMED);
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
            "a boundary grabs {probe:?}, which is on the `params` chip"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &view, probe),
            Claim::Panel,
            "the console gave `egui` a press at {probe:?} on the `params` chip"
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
    let open = library(
        panel.layout(),
        SCOPES,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
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
    let open = library(
        panel.layout(),
        SCOPES,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
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
    let open = library(
        panel.layout(),
        SCOPES,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
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

/// **The chip reads `params`, and it lights while a reading is open.**
///
/// Two things, and both of them moved on 2026-09-09.
///
/// **The word.** It said `read`, which the maintainer called unclear: `read`
/// names what the press does to the file and `params` names the block, which
/// is what nine of its ten rows are. The word is asserted here rather than
/// against a constant this crate exports, because a chip reading something
/// else is a control the note does not describe.
///
/// **The light.** It carried no lit state at all, and the argument was that
/// the block below it is louder than a chip changing colour — which was an
/// argument for the block being enough and not for the chip being wrong. Rule
/// 03 asks a symbol what state it is in, so it is drawn as every other
/// two-state capsule on this panel is, and this asserts the pair rather than
/// one end: **filled while a reading is open and not filled while none is.**
/// A test of the lit end alone would pass against a chip that was always lit,
/// which is the same control with the toggle taken out
/// (ADR-0312).
#[test]
fn the_params_chip_says_params_and_lights_while_a_reading_is_open() {
    let (mut view, mut panel) = showing_mock();
    for open in [false, true] {
        if open {
            view.read(reading());
        }
        let said = match open {
            true => "open",
            false => "shut",
        };
        let foot = bay(&panel).foot;
        let drawn = shapes_inside(&mut view, &mut panel, foot);
        assert!(
            drawn.iter().any(|shape| matches!(
                shape,
                egui::Shape::Text(at) if at.galley.text() == PARAMS_WORD
            )),
            "the foot does not say `{PARAMS_WORD}` with the reading {said}: {drawn:#?}"
        );
        let ctx = drawn_once();
        let chip = bay(&panel).params_chip(&ctx, AIMED);
        let filled = drawn.iter().any(|shape| {
            matches!(
                shape,
                egui::Shape::Rect(at) if near(at.rect.min.x, chip.min.x)
                    && near(at.rect.width(), chip.width())
                    && at.fill != egui::Color32::TRANSPARENT
            )
        });
        assert_eq!(
            filled,
            open,
            "the `params` chip is {} with the reading {said}, and a toggle says which of the two \
             the next press will be",
            match filled {
                true => "filled",
                false => "not filled",
            }
        );
    }
}

/// The word the chip is drawn with, which is the mock's own and is not a
/// constant this crate exports: it is asserted here because a chip reading
/// something else is a control the note does not describe.
const PARAMS_WORD: &str = "params";

// ---------------------------------------------------------------------------
// The `history` scope: the fifth chip, its rows, and the landing
// ---------------------------------------------------------------------------

/// **Three versions as the host would hand them in**, newest first, in
/// [`version_row`]'s spelling — `crates/karakuri`'s, which is
/// `history::Version`'s own fields joined with the separators
/// `Snapshots::record` writes.
///
/// They are literals here rather than built from anything, for the reason the
/// mock's names are: this crate reads no store and what crosses the seam is a
/// row of words (ADR-0156). What matters about them here is the **order**,
/// which is the listing's and not this bay's (ADR-0263 read on a history).
fn versions() -> Vec<String> {
    [
        "20260908-143052-271_slot0_L4_beat_strokes",
        "20260908-142930-004_slot0_L1_drift_shell",
        "20260907-235959-999_slot2_L41_soft_points",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

/// **The fifth chip is `history`, and a press on it asks for the walk rather
/// than for a scope.**
///
/// Three claims, and the third is the one ADR-0308 took. The chip is **last**
/// in `Scope::ALL`, which is the order the row is drawn in and the order a step
/// goes round. A press **marks** it, exactly as a press on any of the four
/// beside it does. And what a press **asks for** is
/// `Operation::WalkHistory` — because `SelectScope`'s payload cannot say
/// which chip, so a `SelectScope` emitted here would be indistinguishable from
/// a press on `all`: a record saying a library was chosen for a press that
/// asked for an edit history.
#[test]
fn the_history_chip_is_the_fifth_and_a_press_on_it_asks_for_the_walk() {
    assert_eq!(
        Scope::ALL.last().copied(),
        Some(Scope::History),
        "`history` is not the last chip the bay draws"
    );
    assert_eq!(Scope::History.name(), "history");
    assert!(
        !Scope::History.lists_sets(),
        "a row of `history` is a version and not a Set"
    );

    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    assert_eq!(
        chips(&mut view, &mut panel, &bay)
            .last()
            .map(String::as_str),
        Some("history"),
        "the row's last word is not the fifth chip's"
    );

    let (scope, chip) = bay
        .chips(&ctx, SCOPES)
        .last()
        .expect("the row has chips in it");
    assert_eq!(scope, Scope::History);
    let probe = egui::pos2(chip.min.x + 2.0, chip.center().y);
    let chosen = bay
        .chip(&ctx, SCOPES, Point::new(probe.x, probe.y))
        .expect("no chip answered a press on `history`");
    assert_eq!(chosen.scope, Scope::History);
    assert_eq!(
        chosen.operation,
        Operation::WalkHistory {
            step: karakuri_operation::Undecided
        },
        "the `history` chip asked for something other than `Walk the edit history`"
    );

    assert!(view.select_scope(Scope::History), "the mark did not move");
    assert_eq!(
        marked(&mut view, &mut panel, &bay).as_deref(),
        Some("history"),
        "the chip was chosen and the wash stayed where it was"
    );
}

/// **A `history` listing is the rows the host handed in, in the order it handed
/// them, and the foot says how many were not drawn.**
///
/// The order is **the operation's and not this surface's**, which is ADR-0263's
/// argument met on a second listing: `history::list` answers most recent first,
/// and a bay that sorted what it was given would be a second answer to the
/// question that listing already answers. So this asserts that the words the
/// list paints are the words that went in, in that sequence — and never that
/// they are sorted, because sorting them here is exactly the defect.
///
/// **The foot is the same `n of m` a library's is**, counted on versions: a
/// history taller than the bay says so in the one place the mock puts it, and
/// this bay does not scroll.
#[test]
fn a_history_listing_is_the_rows_the_host_handed_in_and_the_foot_counts_them() {
    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = versions();
    assert!(view.select_scope(Scope::History));
    let mut panel = console(PLAUSIBLE);
    let bay = library(panel.layout(), SCOPES, &view.library, None, None, 0.0)
        .expect("the library bay lists its rows");
    assert_eq!(bay.rows, versions().len(), "the bay drew a different count");
    assert_eq!(bay.count(), "3 of 3");

    let mut down: Vec<(f32, String)> = shapes_inside(&mut view, &mut panel, bay.list)
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(at) => Some((at.pos.y, at.galley.text().to_owned())),
            _ => None,
        })
        .collect();
    down.sort_by(|a, b| a.0.total_cmp(&b.0));
    let painted: Vec<String> = down.into_iter().map(|(_, text)| text).collect();
    assert_eq!(
        painted,
        versions(),
        "the rows were drawn in an order this bay invented"
    );

    // **A history taller than the bay**, which is the foot's second number
    // doing the only thing it is for: the rows are what fit and the total is
    // what the host handed over.
    let many: Vec<String> = (0..60)
        .map(|at| format!("20260908-1430{at:02}-000_slot0_L4_beat_strokes"))
        .collect();
    let tall =
        library(panel.layout(), SCOPES, &many, None, None, 0.0).expect("the bay draws its foot");
    assert!(
        tall.rows < many.len(),
        "sixty versions fit in the bay, so the foot has nothing to report"
    );
    assert_eq!(tall.count(), format!("{} of {}", tall.rows, many.len()));
}

/// **A press on a `history` row lands that version on the pulldown's deck.**
///
/// The operation is `Operation::RestoreProcedure` carrying the row's own word
/// and the deck the load pulldown names — which is ADR-0308's whole seam, and
/// the two marks are pulled apart before the press for
/// [`a_press_on_load_asks_for_the_deck_the_pulldown_names`]'s reason: a console
/// where the keys and the load agree cannot tell the two readings apart.
///
/// **And the carry on the same rectangle answers nothing**, which is the other
/// half: one press means two things and which of them it is decided by which
/// listing goes in with the point. `View::sets` is empty under this scope, so
/// `take` cannot pick a Set up; `View::versions` is empty under every other, so
/// `land` cannot put one back off a library row.
#[test]
fn a_press_on_a_history_row_lands_that_version_on_the_pulldowns_deck() {
    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = versions();
    assert!(view.select_scope(Scope::History));
    view.mixer = std::iter::repeat_with(strip).take(4).collect();
    assert!(view.select(2), "the keys did not go to deck C");
    assert!(view.aim_at(1), "the load was not aimed at deck B");

    let panel = console(PLAUSIBLE);
    let bay = library(panel.layout(), SCOPES, &view.library, None, None, 0.0)
        .expect("the library bay lists its rows");
    let row = bay.row(1);
    let probe = Point::new(row.center().x, row.center().y);

    assert_eq!(
        bay.land(view.versions(), view.target(), probe),
        Some(Operation::RestoreProcedure {
            deck: 1,
            revision: karakuri_operation::Revision::Picked(versions()[1].clone()),
        }),
        "the press did not land the row's own version on the pulldown's deck"
    );
    assert_eq!(
        view.selection(),
        2,
        "landing a version moved the deck selection"
    );
    assert_eq!(
        bay.take(view.rows(), probe),
        None,
        "a row of `history` was taken in hand as though it were a Set"
    );

    // **And the other way round**: back on a library scope the rows are Sets,
    // so the carry answers and the landing does not.
    assert!(view.select_scope(Scope::AllSets));
    view.library = mock();
    assert!(
        bay.take(view.rows(), probe).is_some(),
        "a row of `all` was not taken in hand"
    );
    assert_eq!(
        bay.land(view.versions(), view.target(), probe),
        None,
        "a row of `all` was landed as though it were a version"
    );
}

/// **A deck running no Set draws no rows, and nothing in the list answers a
/// press.**
///
/// The host hands in an empty listing, because a version written where there
/// was no Set is filed under *none* and a narrowing to a Set matches no such
/// row — ADR-0276's own consequence, and the one thing this scope had to get
/// right. What the bay does with it is what it does with any scope that lists
/// nothing: it draws the chips, no rows, and `0 of 0`.
///
/// **The words are the host's**, which is why this asserts an empty list rather
/// than a sentence: `why_nothing` is where the sentence is, and it is checked
/// in `crates/karakuri`.
#[test]
fn a_deck_running_no_set_draws_no_history_rows() {
    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert!(view.select_scope(Scope::History));
    assert!(view.library.is_empty());

    let panel = console(PLAUSIBLE);
    let bay = library(panel.layout(), SCOPES, &view.library, None, None, 0.0)
        .expect("the bay draws its scopes and its foot");
    assert_eq!(bay.rows, 0, "a scope with nothing in it drew rows");
    assert_eq!(bay.count(), "0 of 0");
    assert_eq!(
        bay.land(
            view.versions(),
            view.target(),
            Point::new(bay.list.center().x, bay.list.min.y + 1.0)
        ),
        None,
        "a press on the empty list landed a version nobody can see"
    );
}

// ---------------------------------------------------------------------------
// A row's own menu
// ---------------------------------------------------------------------------
//
// **The card is the deck pulldown's one control along, and these tests are
// that control's read against a different rectangle** — it hangs off a *row*
// rather than off a capsule in the foot, it is opened by the secondary button
// rather than by a press, and it carries one item that names no deck at all.
// What is asserted here is the console's half: which row a press names, which
// items the card carries, what each of them asks for, and that a card that is
// down owns every press on the console. Which *button* opened it is the host's
// and is asserted in `crates/karakuri` — this crate has never known which
// button a press was (ADR-0311).

/// **A secondary press on a row puts that row's menu down, and a press on
/// nothing puts nothing down.**
///
/// Both halves, because a control that claimed every point of the list would
/// pass a test made only of the first: the list's ground below the last row
/// belongs to nobody, which is `input::claim`'s rule 4 — *a control claims what
/// it acts on and no more* — and a `history` row is a version rather than a
/// Set, which every item on this card names.
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

/// **The card carries one load per deck the mixer is drawing, and the send
/// whatever it is drawing.**
///
/// The first half is `View::select`'s own refusal read a third time — the
/// pulldown's list is cut to the same count — and the second is where this
/// card parts company with that one: `Save as a kbset` names no deck, so a
/// console with no strip at all still has something to pick and something to
/// leave by. That is why `View::open_menu` does not refuse where
/// `View::open_target` does, and both directions are asserted.
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

    // **A console the mixer is drawing nothing for**, which is where this card
    // and the pulldown's differ: the send is still there to pick.
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

/// **A pick names the item's deck and the menu's own row, and moves no mark.**
///
/// The three marks are pulled apart before the press — the keys on deck C, the
/// load aimed at deck B, the cursor on the first row and the menu on the
/// fourth — because a console where they agree cannot tell the readings apart.
/// That is `a_press_on_load_asks_for_the_deck_the_pulldown_names`' technique
/// with one more mark in it, and it is what makes this the one route that
/// reads neither of them.
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
    // **Deck C's item and not deck A's**, which is what makes this a reading
    // of the item rather than of a default: a card that named the first deck
    // whatever was pressed would answer deck A and pass a test written on it.
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

/// **The item under the separator asks for the send of the row the menu was
/// opened on.**
///
/// `Operation::TransferSet { transfer: SetTransfer::Send { id } }`, and **no
/// destination**: that is ADR-0260's shape, still standing, and what this
/// console owes is the id and nothing else. Where the answer goes is the
/// host's, which is why this test can be sure there is no path anywhere in
/// what it asserts.
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

/// **While a row's menu is down, every press on the console is part of that
/// gesture.**
///
/// `input::claim`'s rule 2, which the three other cards are already under, and
/// both halves are asserted for the pulldown's reason: a rule that only
/// claimed the card would leave the first press outside it doing whatever it
/// does the rest of the time — taking a Set in hand, or moving a fader.
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
// The bay scrolls
// ---------------------------------------------------------------------------
//
// **ADR-0307's mechanism, one bay over** — the wheel over the region, a
// position that is the console's own, clamped where the bay is laid out and
// stored unclamped, and rule 04's count in the foot for what is whole. These
// tests are the Inspector's `tests/scroll.rs` asked of a list whose rows are a
// stride rather than a walk, and the one thing that is genuinely this bay's is
// the reading block: it is between two rows rather than over them, so it
// scrolls with them and it is part of what the position is clamped against
// (ADR-0312).

/// **A listing longer than any bay here can hold**, so that there is something
/// to scroll to at all — a `PLAUSIBLE` console's Library bay holds about
/// thirty rows, so this is four times that. The names are `set000`..`set119`
/// because what matters about them is the order and the count.
fn long() -> Vec<String> {
    (0..120).map(|n| format!("set{n:03}")).collect()
}

/// The bay, laid out over a given listing at a given position.
fn bay_at(panel: &Panel, sets: &[String], open: Option<Opened<'_>>, scroll: f32) -> LibraryBay {
    library(panel.layout(), SCOPES, sets, open, None, scroll)
        .expect("the library bay lists its rows")
}

/// **The wheel over the bay scrolls it, and the foot says how much it is not
/// showing.**
///
/// Four things at once, because they are one gesture: `input::wheeled` names
/// this bay rather than an Inspector pane, `View::scroll_library_by` moves the
/// stored position, the rows that reach the picture are a **window** into the
/// listing rather than its first `n`, and the foot counts what is whole.
///
/// **The window is asserted to have moved rather than to be non-empty**, which
/// is the half a weaker test would miss: a bay that ignored the position
/// entirely would still draw rows and still say `n of m`.
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

/// **A row cut by an edge is drawn and is not counted.**
///
/// Rule 04's hard half — *says how much* — and the pair that makes it true:
/// `LibraryBay::drawn` is the wider number and `LibraryBay::rows` is the
/// readout's. Scrolled half a row, the bay draws one more than it counts,
/// because the top one is cut and so is the bottom one.
///
/// **It is asserted as a difference and not as two figures**, so that the test
/// says nothing about how tall this console happens to be.
#[test]
fn a_row_cut_by_an_edge_is_drawn_and_is_not_counted() {
    let panel = console(PLAUSIBLE);
    let sets = long();

    // **A whole number of rows of scroll leaves the top flush**, which is the
    // control the cut case is read against: nothing is hanging over the top
    // edge here, so whatever the count does below is the top edge's doing.
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

/// **The position is the bay's own, and a resize does not rewrite it.**
///
/// [P-0082], and it is the Inspector's
/// `a_position_survives_the_pane_growing_and_shrinking` asked of this bay: the
/// clamp is at the draw and the store keeps what an operator scrolled to, so
/// dragging the bay small and back reproduces the picture **exactly** rather
/// than nearly. **It fails only across time**, which is why the test drives
/// three layouts rather than looking at one.
///
/// [P-0082]: ../../docs/principles/0082-looking-never-writes-back.md
#[test]
fn a_library_position_survives_the_bay_growing_and_shrinking() {
    let short = console(SMALLEST);
    let tall = console(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    // **A listing a tall bay holds whole and a short one does not**, which is
    // the pair this is about: the stored position has to be past what the
    // short bay can use, so that a clamp written back would show.
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

/// **A press on the half of a row the bay has scrolled out of sight reaches
/// nothing.**
///
/// `LibraryBay::row` answers for every index in the listing now, so a row cut
/// by the top edge has a rectangle whose upper half is **outside the list** —
/// over the filter fields, where the row is not drawn and where a press would
/// otherwise take a Set in hand under a control that is drawn there. That is
/// `InspectorPane::grip`'s *refuses a press outside the body* one bay over,
/// and it is **the one thing here that fails silently**: nothing would look
/// wrong, and the row would answer for a press nobody aimed at it.
///
/// **The point is inside a row the bay is drawing and outside the list**,
/// which is the only case the bound is load-bearing for: a row scrolled
/// entirely off the top is not in `LibraryBay::drawn` at all, so the walk
/// never reaches it and a test aimed there would pass against a bay with no
/// bound. That is what this test was, and it was watched to pass against the
/// defect before it was aimed here.
///
/// Both directions, because a bound that refused everything would pass a test
/// made only of refusals: the visible half of the same row is still reached.
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

/// **The reading scrolls with the rows, and it is part of what the position is
/// clamped against.**
///
/// The block is between two rows rather than over them, which is what makes it
/// a mode of the list — so it moves when they move, and a listing with one open
/// is taller than the same listing without. **Both halves are asserted**,
/// because a block that scrolled but was not in the content would let an
/// operator scroll to a place the bay will not draw.
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

/// **The cursor is held inside the rows that are drawn**, which is a window
/// now rather than a prefix.
///
/// The sentence is older than the scroll — a cursor allowed past the drawn
/// rows would sit on a row nobody can see, under a pill saying a press will
/// load it — and what changed is that *drawn* has a start. So `up` at the top
/// of the window does nothing and the wheel is what moves the window, which is
/// ADR-0307's division one bay over: the keyboard's route to a scroll is owed
/// to M5.13 and is not invented here.
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

/// **A scope press puts the listing back at the top**, exactly as it puts the
/// cursor there.
///
/// One reason for both: a position is a distance into `View::library`, that
/// field is about to be rewritten by whoever answers the new scope, and a
/// listing of three read four hundred pixels down draws its last row or
/// nothing at all — the same failure `View::select_scope` already resets the
/// cursor against, a Set nobody chose sitting under a pill that says a press
/// will load it.
///
/// **It is a press moving the console's own state and not a resize rewriting
/// it**, which is where this parts company with [P-0082]: the clamp at the
/// draw is still the only clamp, and a chip pressed twice does not move it a
/// second time.
///
/// [P-0082]: ../../docs/principles/0082-looking-never-writes-back.md
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
