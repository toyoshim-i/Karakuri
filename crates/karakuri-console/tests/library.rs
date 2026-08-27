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
use karakuri_console::panel::Panel;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{library, LibraryBay, View};
use karakuri_layout::{Point, Rect};

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

/// A panel at a viewport, solved — the pair every test here starts from.
fn console(viewport: Rect) -> Panel {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    panel
}

/// The bay, laid out with the mock's names.
fn bay(panel: &Panel) -> LibraryBay {
    library(panel.layout(), &mock()).expect("the library bay lists its rows")
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
/// 26-tall row along the bottom; `.lib-list { padding: 3px }` is what is left
/// between the bay head and it; and `.lib-row { padding: 3px 7px }` around
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

    // The list: under the bay head, inside `.lib-list`'s padding on all four
    // sides, and up to the foot.
    assert!(
        near(
            bay.list.min.y,
            region.min.y + size::HEAD_H + size::LIB_LIST_PAD
        ),
        "the list starts at {} and the head and the padding leave {}",
        bay.list.min.y,
        region.min.y + size::HEAD_H + size::LIB_LIST_PAD
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
    let full = library(panel.layout(), &many).expect("the bay lists its rows");
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
    let bay = library(tall.layout(), &names).expect("the bay lists its rows");
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
    let bay = library(tall.layout(), &many).expect("the bay lists its rows");
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
    let bay = library(small.layout(), &many).expect("the bay lists its rows");
    let region = to_egui(rect_of(small.layout(), "library"));
    let room = region.height() - size::HEAD_H - size::LIB_LIST_PAD * 2.0 - size::LIB_FOOT_H;
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
    let master = to_egui(rect_of(panel.layout(), "master"));
    assert_eq!(
        bare,
        shapes_inside(&mut view, &mut panel, master).len(),
        "the Library bay draws {bare} shapes with no store behind it and the Master bay, \
         which has no body at all, draws {}",
        shapes_inside(&mut view, &mut panel, master).len()
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

/// **No names, no listing**, asked of the derivation rather than of the paint
/// pass — the same rule `mixer` follows, stated where a caller can reach it.
#[test]
fn no_names_is_no_listing() {
    let panel = console(PLAUSIBLE);
    assert_eq!(library(panel.layout(), &[]), None);
    assert!(library(panel.layout(), &mock()).is_some());
}

/// **The bay is drawn where the bay can hold it, and nowhere else** —
/// `picture_rect`'s rule, stated on a listing.
#[test]
fn a_folded_or_soloed_or_short_bay_lists_nothing() {
    let names = mock();
    let mut layout = solved(PLAUSIBLE);
    assert!(library(&layout, &names).is_some());

    layout.collapse(id_of(&layout, "library"));
    layout.solve();
    assert_eq!(
        library(&layout, &names),
        None,
        "the library is folded away and its rows are still being drawn"
    );

    layout.expand(id_of(&layout, "library"));
    layout.solve();
    assert!(library(&layout, &names).is_some());

    // A solo somewhere else takes the bay off the panel with it.
    layout.solo(id_of(&layout, "mixer"));
    layout.solve();
    assert_eq!(library(&layout, &names), None);
    layout.unsolo();
    layout.solve();
    assert!(library(&layout, &names).is_some());

    // **And a bay with no room for the foot and one row lists nothing**, which
    // is the same answer and not a special case.
    //
    // It takes a window under the arrangement's own minimum to reach: the
    // library declares a minimum of 132 and the solve honours it, so at every
    // window this console is claimed to work at there is room for three rows.
    // Below 632 the solve stops honouring minima and scales everything down
    // together (`common::SMALLEST` says so), and that is where a bay too short
    // for a row exists at all.
    let chrome = size::HEAD_H + size::LIB_FOOT_H + size::LIB_LIST_PAD * 2.0;
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
        library(&short, &names),
        None,
        "a bay with no room for one row listed some"
    );

    // Twenty pixels of window taller is one row, which is what says the answer
    // above is the room and not the window.
    let barely = solved(Rect {
        h: 180.0,
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
        library(&barely, &names).map(|bay| bay.rows),
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
        library(&sliver, &names),
        None,
        "a bay with no room for the list's own padding listed some"
    );
}

// ---------------------------------------------------------------------------
// Nothing here is a control, and nothing is stored
// ---------------------------------------------------------------------------

/// **Nothing in the Library bay answers a pointer**, and that is the answer
/// rather than an omission: the mock draws four scope chips, two filter
/// fields, a `+`, a row cursor and a `load → C` pill, and every one of them is
/// a control over machinery that does not exist. So `claim` hands every point
/// of this bay to `egui`, exactly as it does the transport row's.
#[test]
fn nothing_in_the_library_is_a_control() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let region = to_egui(rect_of(panel.layout(), "library"));
    let strips = Vec::new();

    // The four corners inside the bay, and its middle — which between them
    // are a row, the list, the foot and the head.
    //
    // **Inset past `GRAB`**, because a boundary is claimed for a drag from six
    // pixels either side of it and three of this bay's four edges are one. That
    // is the panel taking a *divider*, which is the one thing it takes
    // anywhere on the console; what this test is about is whether anything
    // *in* the bay takes a press, and a point on a boundary is not in it.
    let inset = karakuri_console::panel::GRAB + 2.0;
    let points = [
        egui::pos2(region.min.x + inset, region.min.y + inset),
        egui::pos2(region.max.x - inset, region.min.y + inset),
        egui::pos2(region.min.x + inset, region.max.y - inset),
        egui::pos2(region.max.x - inset, region.max.y - inset),
        region.center(),
    ];
    let mut asked = 0;
    for p in points {
        let claimed = claim(&mut panel, &ctx, &strips, Point::new(p.x, p.y));
        assert_eq!(
            claimed,
            Claim::Egui,
            "the console took the pointer at {p:?}, which is inside the Library bay"
        );
        asked += 1;
    }
    // A guard, so this cannot pass by testing nothing.
    assert_eq!(asked, points.len(), "not every point was asked");
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
    assert_eq!(library(panel.layout(), &one).map(|b| b.total), Some(1));
    assert_eq!(library(panel.layout(), &two).map(|b| b.total), Some(2));
    assert_eq!(library(panel.layout(), &one).map(|b| b.total), Some(1));

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
