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

use common::{drawn_once, id_of, near, rect_of, showing, solved, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::Panel;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{library, mcp_pill, LibraryBay, Scope, View, DECK_LETTERS};
use karakuri_layout::{Point, Rect};
use karakuri_operation::gate::{Class, Open};

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
    library(panel.layout(), SCOPES, &mock()).expect("the library bay lists its rows")
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

    // The list: under the scope row, inside `.lib-list`'s padding on all four
    // sides, and up to the foot.
    assert!(
        near(bay.list.min.y, scopes.max.y + size::LIB_LIST_PAD),
        "the list starts at {} and the scope row ends at {}",
        bay.list.min.y,
        scopes.max.y
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
    let full = library(panel.layout(), SCOPES, &many).expect("the bay lists its rows");
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
    let bay = library(tall.layout(), SCOPES, &names).expect("the bay lists its rows");
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
    let bay = library(tall.layout(), SCOPES, &many).expect("the bay lists its rows");
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
    let bay = library(small.layout(), SCOPES, &many).expect("the bay lists its rows");
    let region = to_egui(rect_of(small.layout(), "library"));
    let room = region.height()
        - size::HEAD_H
        - size::SCOPES_H
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
    assert_eq!(library(panel.layout(), &[], &[]), None);
    assert!(library(panel.layout(), &[], &mock()).is_some());
    assert!(library(panel.layout(), SCOPES, &mock()).is_some());

    let empty = library(panel.layout(), SCOPES, &[]).expect(
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
        library(panel.layout(), &[], &mock()).and_then(|bay| bay.scopes),
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
    assert!(library(&layout, SCOPES, &names).is_some());

    layout.collapse(id_of(&layout, "library"));
    layout.solve();
    assert_eq!(
        library(&layout, SCOPES, &names),
        None,
        "the library is folded away and its rows are still being drawn"
    );

    layout.expand(id_of(&layout, "library"));
    layout.solve();
    assert!(library(&layout, SCOPES, &names).is_some());

    // A solo somewhere else takes the bay off the panel with it.
    layout.solo(id_of(&layout, "mixer"));
    layout.solve();
    assert_eq!(library(&layout, SCOPES, &names), None);
    layout.unsolo();
    layout.solve();
    assert!(library(&layout, SCOPES, &names).is_some());

    // **And a bay with no room for the foot and one row lists nothing**, which
    // is the same answer and not a special case.
    //
    // It takes a window under the arrangement's own minimum to reach: the
    // library declares a minimum of 158 and the solve honours it, so at every
    // window this console is claimed to work at there is room for three rows.
    // Below 632 the solve stops honouring minima and scales everything down
    // together (`common::SMALLEST` says so), and that is where a bay too short
    // for a row exists at all.
    let chrome = size::HEAD_H + size::SCOPES_H + size::LIB_FOOT_H + size::LIB_LIST_PAD * 2.0;
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
        library(&short, SCOPES, &names),
        None,
        "a bay with no room for one row listed some"
    );

    // Twenty pixels of window taller is one row, which is what says the answer
    // above is the room and not the window.
    let barely = solved(Rect {
        h: 250.0,
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
        library(&barely, SCOPES, &names).map(|bay| bay.rows),
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
        library(&sliver, SCOPES, &names),
        None,
        "a bay with no room for the list's own padding listed some"
    );
}

// ---------------------------------------------------------------------------
// Nothing here is a control, and nothing is stored
// ---------------------------------------------------------------------------

/// **Nothing in the Library bay answers a pointer**, and for the `load → A`
/// pill that is the specification rather than a thing not built yet.
///
/// The mock draws four scope chips, two filter fields, a `+`, a row cursor and
/// the pill. Seven of the eight are controls over machinery that does not
/// exist, which is `view::library`'s own list. **The pill is the one that is
/// not**: `console.html`'s *How a Set reaches a deck* settles that a load is
/// *"a cursor and a key with no pointer anywhere in it"*, so the pill names
/// where a press would land and is never itself pressed — and a drag from a
/// row onto a strip is *"a second route to the same command, and never the
/// first"*. So `claim` hands every point of this bay to `egui`, exactly as it
/// does the transport row's.
///
/// # The bay's corners do not reach the pill, and the two numbers say why
///
/// The bay's own rectangle inset past [`GRAB`](karakuri_console::panel::GRAB)
/// is where this test used to stop, and it is **two pixels short of the box
/// the pill is drawn in**. A `.lib-foot` is `padding: 5px 10px`
/// ([`size::LIB_FOOT_PAD_X`]) and a flex row whose `.sep { flex: 1 }` pushes
/// the pill to the far end of it, so the pill's right edge is 10 in from the
/// bay's — where an inset of `GRAB + 2` is 8. **10 against 6 is the pill
/// clearing the boundary's grab**, which is what makes it drawable at all and
/// is `input.rs`'s measurement one bay along; 8 is in the gap between the two,
/// and a pill that claimed presses would have gone unasked.
///
/// So the points are taken off [`LibraryBay`]'s own boxes rather than off the
/// bay's corners: the foot's content box across its middle, and each drawn
/// row's. That is the same rule the geometry test above is written to — ask
/// the derivation that draws it — and it is why this file, not `claim`, is
/// where the reach is stated.
#[test]
fn nothing_in_the_library_is_a_control() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let region = to_egui(rect_of(panel.layout(), "library"));
    let bay = bay(&panel);
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
    let mut points = vec![
        egui::pos2(region.min.x + inset, region.min.y + inset),
        egui::pos2(region.max.x - inset, region.min.y + inset),
        egui::pos2(region.min.x + inset, region.max.y - inset),
        egui::pos2(region.max.x - inset, region.max.y - inset),
        region.center(),
    ];

    // **The foot's content box, across its middle** — the count at one end and
    // the pill at the other, with `.sep` between them. Eleven points at the
    // foot's centre y, which is 13 off the bay's bottom edge and so clear of
    // the boundary under it.
    let (left, right) = (
        bay.foot.min.x + size::LIB_FOOT_PAD_X,
        bay.foot.max.x - size::LIB_FOOT_PAD_X,
    );
    assert!(
        right - left > 0.0,
        "the foot's content box is {left} to {right}, which is no box to sweep"
    );
    for step in 0..=10 {
        let t = step as f32 / 10.0;
        points.push(egui::pos2(left + (right - left) * t, bay.foot.center().y));
    }

    // **And each drawn row's own box**, which is where a cursor would be and
    // where a drag onto a strip would start. The row is the full width of the
    // list, so its text box is one `LIB_ROW_PAD_X` in from either end.
    for index in 0..bay.rows {
        let row = bay.row(index);
        points.push(egui::pos2(row.min.x + size::LIB_ROW_PAD_X, row.center().y));
        points.push(egui::pos2(row.center().x, row.center().y));
        points.push(egui::pos2(row.max.x - size::LIB_ROW_PAD_X, row.center().y));
    }

    let mut asked = 0;
    for p in &points {
        let claimed = claim(&mut panel, &ctx, &showing(&strips), Point::new(p.x, p.y));
        assert_eq!(
            claimed,
            Claim::Egui,
            "the console took the pointer at {p:?}, which is inside the Library bay"
        );
        asked += 1;
    }
    // A guard, so this cannot pass by testing nothing — and a floor under the
    // reach, so the foot's sweep and the rows cannot quietly go away.
    assert_eq!(asked, points.len(), "not every point was asked");
    assert!(
        points.len() >= 5 + 11 + 3,
        "only {} points asked: the bay's corners, the foot's box and at least one row are the \
         floor, and fewer is a sweep that has stopped covering the pill",
        points.len()
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
        library(panel.layout(), SCOPES, &one).map(|b| b.total),
        Some(1)
    );
    assert_eq!(
        library(panel.layout(), SCOPES, &two).map(|b| b.total),
        Some(2)
    );
    assert_eq!(
        library(panel.layout(), SCOPES, &one).map(|b| b.total),
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
/// them** — which is the mock's own row: `favourites`, `my sets`, `presets`,
/// `folder`.
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
        vec!["favourites", "my sets", "presets", "folder"],
        "the scope row is not the mock's four chips in the mock's order"
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

    // A host that opens on `my sets`, which is what a program with a store
    // does: the mock marks `favourites` and that is the scope nothing here can
    // answer.
    assert!(view.select_scope(Scope::MySets));
    assert_eq!(view.scope(), Some(Scope::MySets));
    assert_eq!(
        marked(&mut view, &mut panel, &bay).as_deref(),
        Some("my sets")
    );

    for scope in [
        Scope::Presets,
        Scope::Folder,
        Scope::Favourites,
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
    assert_eq!(view.scope(), Some(Scope::Favourites), "the first chip");

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
        Some(Scope::Favourites),
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
/// asserts the words rather than a rectangle, and asserts them again after the
/// selection moves — a pill that read `load → A` whatever was selected would
/// pass the first half and be a lie for the other three decks.
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
        let want = format!("load \u{2192} {}", DECK_LETTERS[usize::from(deck)]);
        let drawn = shapes_inside(&mut view, &mut panel, bay.foot);
        assert!(
            drawn.iter().any(|shape| matches!(
                shape,
                egui::Shape::Text(at) if at.galley.text() == want
            )),
            "the selection is deck {deck} and the foot does not read `{want}`: {drawn:#?}"
        );
    }
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
