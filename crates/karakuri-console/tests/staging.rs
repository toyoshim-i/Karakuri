//! The Staging lane: a card, a head, and a row per node a build changed — and,
//! with nothing outstanding, a card and a head and nothing else.
//!
//! The lane draws four of the six things a candidate row could carry — the
//! deck, the node's address, what that node's procedure calls itself, and
//! whether it is on screen — and the argument for the two it omits is written
//! out in `view`'s module documentation and at `view::staging`. A row per
//! changed node rather than per slot is
//! [ADR-0326](../../../docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md),
//! and it is what gives the row its two presses: the row is *keep a candidate*
//! and the `back` capsule at its end is *put a node's previous version back*.
//! ADR-0200 is why what has no value is omitted outright rather than drawn
//! hollow.
//!
//! The empty case is still the case this file leads with, because it is still
//! this lane's ordinary state: *"a staging lane with nothing in it is simply a
//! run in which nobody has rewritten a procedure yet — which is every run at
//! its start and most runs throughout"*. A decision to draw nothing needs a
//! test more than a decision to draw something does: nothing about deleting the
//! argument would make a suite fail, and what would be added back is what the
//! page refuses.
//!
//! Seven things:
//!
//! 1. With nothing outstanding the bay draws the shapes of a bay with no body,
//! asserted against the Sequencer bay — the lane's twin in this console's
//! furniture: a title, no pill, no grip, and nothing in its body at all. It is
//! a claim about the paint pass, so it is made by drawing a frame and counting.
//! 2. Nothing else the console is handed puts anything there. The Library's
//! names, the mixer's strips, the Inspector's panes and a picture are all
//! written onto the `View` and the lane is the same two shapes after, which is
//! what says the bay reads none of them: the one field it reads is
//! `View::staging`. 3. A candidate draws a row, and the rows are where the
//! arithmetic says — the list box off `.stage-list`'s three paddings, a stride
//! of a `.cand` and `.stage-list`'s gap, every row inside the list. 4. The lane
//! holds the mock's three and no more, which is the one piece of boundary
//! arithmetic in this bay: `lib.rs` pins it at 125 because the mock draws three
//! rows and two gaps, and a fourth candidate is counted and not drawn. 5. The
//! head is untouched by any of it, asserted with the lane full as well as
//! empty: the mock's `2 waiting` is a readout and a bay head's pills are its
//! controls. 6. What in it is a control and what is not, asked of a full lane
//! as well as an empty one: the bay's own ground is `egui`'s, a row that offers
//! a keep is the panel's, and the `back` capsule inside it is asked first
//! because it is the smaller box. 7. A row the checker turned down carries what
//! it said, which is the one row that draws a sentence: nothing was built for
//! it, so the word alone says a save did not take and nothing about why
//! (ADR-0310). 8. Each press asks for the operation its row is addressed by,
//! and the rows that offer neither say so: an overloaded row has nothing to
//! keep and a row that names no node has nothing to keep and nothing to step
//! back.
//!
//! And the four words a row can end in are the manual's own, which is ADR-0159
//! asked of this bay: *"whether it is on screen: landed, overloaded for costing
//! more than one frame may, refused, or did not compile"*. This quoted *refused
//! by the checker* until 2026-09-08, which was a paraphrase of a sentence the
//! page does not have and had the two refusals the other way round: *refused*
//! is a build that failed, and the checker's is the fourth word.
//!
//! None of it needs a window or a device.

mod common;

use common::{drawn_once, rect_of, showing, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::Panel;
use karakuri_console::room::Room;
use karakuri_console::view::{
    staging, Candidate, Kind, Level, Pane, Stage, Strip, Tally, View, DECK_LETTERS, PANES,
};
use karakuri_layout::{Point, Rect};

/// A panel at a viewport, solved — the pair every test here starts from.
fn console(viewport: Rect) -> Panel {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    panel
}

/// `egui`'s rectangle, from `karakuri_layout`'s.
fn to_egui(r: Rect) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h))
}

/// One mixer strip, live and settled — `mixer.rs`'s own, with everything this
/// file does not care about at rest.
fn strip(name: &str) -> Strip {
    Strip {
        name: name.to_owned(),
        tally: Tally::Live,
        requested: Tally::Live,
        gain: 0.72,
        gain_to: None,
        opacity: 1.0,
        opacity_to: None,
        blend: karakuri_operation::BlendMode::Add,
        mask: karakuri_console::view::Mask::None,
        mask_angle: 0.0,
        level: Some(Level {
            mean: 0.74,
            peak: 0.82,
        }),
    }
}

/// One candidate: a node one build changed on a deck slot, not yet ruled on.
///
/// `L1:0` on every one of them unless a test says otherwise — which node it is
/// only matters where the operation a press asks for is being read, and those
/// tests build their own.
fn candidate(deck: usize, name: &str, stage: Stage) -> Candidate {
    at_node(deck, karakuri_operation::Layer::L1, 0, name, stage)
}

/// The same, addressed.
fn at_node(
    deck: usize,
    layer: karakuri_operation::Layer,
    index: u32,
    name: &str,
    stage: Stage,
) -> Candidate {
    Candidate {
        deck,
        at: Some(karakuri_operation::NodeAddress { layer, index }),
        addr: format!("{}:{index}", layer_word(layer)),
        name: name.to_owned(),
        stage,
        // **Empty, because every verdict but one is about a build.** The row
        // that carries a sentence is `Stage::NotCompiled`'s, and
        // [`refused_candidate`] is the one that builds it.
        said: Vec::new(),
    }
}

/// A row that names no node — a build that did not happen, or a rebuild that
/// restated the stack and changed nothing in it. It is the slot's row and
/// offers neither press.
fn slot_row(deck: usize, name: &str, stage: Stage) -> Candidate {
    Candidate {
        deck,
        at: None,
        addr: String::new(),
        name: name.to_owned(),
        stage,
        said: Vec::new(),
    }
}

/// The mock's `.addr` spelling, which the host writes and this file restates
/// because it is building the value the host would hand in.
///
/// Kept in step with `karakuri/src/main.rs`'s `layer_word` by hand, which is
/// what a fixture that restates a host's spelling costs: the two are one
/// spelling in two places, and a row addressed by one and drawn by the other
/// would be a row an operator cannot press back.
fn layer_word(layer: karakuri_operation::Layer) -> &'static str {
    match layer {
        karakuri_operation::Layer::L1 => "L1",
        karakuri_operation::Layer::L2 => "L2",
        karakuri_operation::Layer::L3 => "L3",
        karakuri_operation::Layer::L4 => "L4",
        karakuri_operation::Layer::Field => "F",
        // **Answered here and reached by nothing today.** A candidate is a node
        // of a *Set* that a build produced, and a Set holds no L5 node: a frame
        // effect runs in the master chain, which is one level out from every
        // Set, and the chain is still three fixed passes. So no Staging row is
        // ever addressed `L5:0` and this arm builds a value nothing asks for.
        //
        // **Named rather than left to a wildcard anyway**, on the terms the
        // match itself exists for: what this fixture is checking is that an
        // address a row is drawn with is an address a press can type back, and
        // a catch-all is how the fifth layer went wrong one kind ago
        // (`setfile::layer_from_ordinal`). `L5` and not a bare letter, matching
        // the host's own spelling above.
        karakuri_operation::Layer::L5 => "L5",
    }
}

/// One candidate the checker turned down, with what it said.
///
/// It names no node and cannot: nothing was built, so there is no list of nodes
/// to hold against the one before it (`view::Candidate::at`).
///
/// `karakuri_engine::swap::Refusal` carries one line per diagnostic, formatted
/// on the build worker; these are the shape those lines have — where, which
/// stage, and what.
fn refused_candidate(deck: usize, name: &str, said: &[&str]) -> Candidate {
    Candidate {
        deck,
        at: None,
        addr: String::new(),
        name: name.to_owned(),
        stage: Stage::NotCompiled,
        said: said.iter().map(|line| (*line).to_owned()).collect(),
    }
}

/// Every shape the console paints wholly inside `rect`, on one frame.
///
/// `library.rs`'s helper, and `transport.rs` is where the reasoning is written
/// out: containment rather than intersection, so the panel's ground and the
/// card's drop shadow are not counted as things drawn in the bay.
fn shapes_inside(view: &mut View, panel: &mut Panel, rect: egui::Rect) -> usize {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter(|clipped| {
            let bounds = clipped.shape.visual_bounding_rect();
            bounds.is_finite() && rect.contains_rect(bounds)
        })
        .count()
}

// ---------------------------------------------------------------------------
// The bay has no body
// ---------------------------------------------------------------------------

/// The Staging lane draws what a bay with no body draws, and the Sequencer bay
/// is what that is.
///
/// Not a count of shapes — `egui` is free to tessellate a card differently
/// tomorrow — but two bays held against each other. The Sequencer is the lane's
/// exact twin in the console's own furniture: a title, no pill, no grip, and
/// nothing in its body at all. `library.rs` holds the Library against the
/// Master for the same reason and picks a different twin because the Library
/// has a grip and the Master has one; a Staging bay held against the Master
/// would be six grip dots short and the assertion would be about the grip.
///
/// So the two draw the same shapes, or Staging is drawing something a lane with
/// no candidate does not have: a `.cand` row, a placeholder, or the head's `2
/// waiting` where there is nothing to count.
///
/// Asked at both windows, because a bay that started drawing a body would be
/// most likely to do it at the taller one.
#[test]
fn the_staging_lane_draws_no_body() {
    let mut asked = 0;
    for viewport in [PLAUSIBLE, SMALLEST] {
        let mut panel = console(viewport);
        let staging = to_egui(rect_of(panel.layout(), "staging"));
        let twin = to_egui(rect_of(panel.layout(), "sequencer"));

        let mut view = View::new(Room::Day);
        let lane = shapes_inside(&mut view, &mut panel, staging);
        let bare = shapes_inside(&mut view, &mut panel, twin);
        assert_eq!(
            lane, bare,
            "at {}x{} the Staging bay draws {lane} shapes and the Sequencer bay, which has \
             the same head and no body at all, draws {bare}",
            viewport.w, viewport.h
        );
        // A guard, so this cannot pass by both bays being off the panel and
        // drawing nothing at all: a card and a head is at least two shapes.
        assert!(
            bare >= 2,
            "the Sequencer bay drew {bare} shapes, which is not a card and a head"
        );
        asked += 1;
    }
    assert_eq!(asked, 2, "not every window was asked");
}

/// Nothing the console is handed *except its own field* reaches the lane.
///
/// The four things a caller writes onto a `View` that this bay might have been
/// reading are the Library's names, the mixer's strips, the Inspector's panes
/// and the canvas; a fifth, the picture, takes a device and is `None` in every
/// test in this crate. A console with all four filled is the fullest this crate
/// can make one, and the lane is the same bay it was empty — which is what says
/// a row comes from `View::staging` and from nothing else. That field is the
/// one thing deliberately left alone here; the tests below are what fill it.
#[test]
fn a_full_console_stages_nothing() {
    let mut panel = console(PLAUSIBLE);
    let staging = to_egui(rect_of(panel.layout(), "staging"));

    let mut view = View::new(Room::Day);
    let empty = shapes_inside(&mut view, &mut panel, staging);

    view.library = ["drift_night", "lattice_veil", "glass_shell"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    view.mixer = vec![strip("drift_night"), strip("lattice_veil")];
    view.inspector = vec![
        Pane {
            deck: 0,
            material: "drift_night".to_owned(),
            sync: karakuri_operation::Sync::Free,
            // Nothing refused, which is what a pane handed in to fill the
            // console says about material this file is not asking after.
            allows: [true; karakuri_console::view::SYNCS.len()],
            anchor_bpm: 128.0,
            scrub_beats: 0.0,
            composite: false,
            // Not this test's row: the deck head's two build chips are
            // drawn from this and nothing here is about them.
            aimed: None,
            nodes: Vec::new(),
        };
        PANES
    ];
    let full = shapes_inside(&mut view, &mut panel, staging);
    assert_eq!(
        full,
        empty,
        "the console was handed {} names, {} strips and {} panes and the Staging bay went from \
         {empty} shapes to {full}",
        view.library.len(),
        view.mixer.len(),
        view.inspector.len()
    );
}

/// The lane's head is the Sequencer's head, with the lane full and with it
/// empty.
///
/// The mock's Staging head reads `2 waiting` and this one reads nothing, for
/// the reason `view::staging` gives: a bay head's pills are the mock's
/// *controls* — every readout in a head is undrawn, `previews 3 of 4` included
/// — and the number would say what the rows already say, this lane having no
/// truncation to report where the Library's foot has.
///
/// Asked with three candidates in the lane, which is the state a count would be
/// drawn in and is the reason this is a paint-level assertion rather than the
/// table read it used to be: the lane is `Kind::Staging` now, so the pills it
/// does not draw are not in the table to be counted. The head strip is the top
/// [`HEAD_H`] of each bay, and the Sequencer's is the same head over an empty
/// bay — so the two are equal, or this one has grown a pill.
#[test]
fn the_lane_head_counts_nothing() {
    let mut panel = console(PLAUSIBLE);
    let lane = to_egui(rect_of(panel.layout(), "staging"));
    let twin = to_egui(rect_of(panel.layout(), "sequencer"));
    let head = |bay: egui::Rect| {
        egui::Rect::from_min_size(
            bay.min,
            egui::vec2(bay.width(), karakuri_console::room::size::HEAD_H),
        )
    };

    let mut view = View::new(Room::Day);
    let bare = shapes_inside(&mut view, &mut panel, head(twin));
    // A guard, so this cannot pass by both heads being empty. It is one shape
    // and not two: the word is wholly inside the strip and the rule under it
    // is a hairline centred on the strip's own bottom edge, so half of it
    // hangs below and containment does not count it — which is
    // `shapes_inside`'s rule and is why this is a floor rather than a figure.
    assert!(
        bare >= 1,
        "the Sequencer's head drew {bare} shapes, which is not even the word in it"
    );

    for waiting in 0..=3 {
        view.staging = (0..waiting)
            .map(|deck| candidate(deck, "drift_shell + soft_points", Stage::Landed))
            .collect();
        let drawn = shapes_inside(&mut view, &mut panel, head(lane));
        assert_eq!(
            drawn, bare,
            "with {waiting} waiting the Staging head draws {drawn} shapes and the Sequencer's, \
             which is a word and a rule and nothing else, draws {bare}"
        );
    }

    // And it is a kind of its own, which is what tells `View::draw` where the
    // rows go without comparing a name on the frame path.
    assert_eq!(
        karakuri_console::view::region("staging")
            .expect("the lane is a region")
            .kind,
        Kind::Staging,
        "the lane is drawn from the table's own arm or from a string in the loop"
    );
}

// ---------------------------------------------------------------------------
// A candidate draws a row
// ---------------------------------------------------------------------------

/// A candidate is a row, and the rows are where `.stage-list` and `.cand` put
/// them.
///
/// The whole box, term for term: the list is the bay under its head, inset by
/// `.stage-list`'s `padding: 6px 9px 8px` — which is the one padding in this
/// console that is not the same top and bottom — and a row is [`CAND_H`] tall
/// with [`STAGE_GAP`] between one and the next and none above the first.
///
/// Asked at both windows, though the bay is the same 218 x 125 at each:
/// `lib.rs` pins it, and a lane that started deriving its own height would say
/// so here first.
#[test]
fn a_candidate_is_a_row_where_the_mock_puts_it() {
    use karakuri_console::room::size;
    let mut asked = 0;
    for viewport in [PLAUSIBLE, SMALLEST] {
        let panel = console(viewport);
        let bay = to_egui(rect_of(panel.layout(), "staging"));
        let rows = vec![
            candidate(0, "drift_shell + soft_points", Stage::Overloaded),
            candidate(1, "drift_shell + soft_points", Stage::Landed),
        ];
        let lane = staging(panel.layout(), &rows).expect("two candidates and a bay to draw in");

        assert_eq!(lane.rows, 2, "two candidates and {} rows", lane.rows);
        assert_eq!(lane.total, 2, "the lane was handed two candidates");
        assert_eq!(
            lane.list.min.x,
            bay.min.x + size::STAGE_LIST_PAD_X,
            "`.stage-list`'s side padding"
        );
        assert_eq!(
            lane.list.max.x,
            bay.max.x - size::STAGE_LIST_PAD_X,
            "`.stage-list`'s side padding"
        );
        assert_eq!(
            lane.list.min.y,
            bay.min.y + size::HEAD_H + size::STAGE_LIST_PAD_TOP,
            "the list starts under the bay head, inside `.stage-list`'s top padding"
        );
        assert_eq!(
            lane.list.max.y,
            bay.max.y - size::STAGE_LIST_PAD_BOTTOM,
            "`.stage-list`'s bottom padding is not its top"
        );

        let first = lane.row(0);
        let second = lane.row(1);
        assert_eq!(first.min.y, lane.list.min.y, "the first row is flush");
        assert_eq!(first.height(), size::CAND_H, "a `.cand` is 4 + 16.5 + 4");
        assert_eq!(
            second.min.y - first.max.y,
            size::STAGE_GAP,
            "`.stage-list`'s `gap: 5px` between one row and the next"
        );
        for index in 0..lane.rows {
            assert!(
                lane.list.contains_rect(lane.row(index)),
                "row {index} is outside the list it was laid in"
            );
        }
        asked += 1;
    }
    assert_eq!(asked, 2, "not every window was asked");
}

/// The lane holds the mock's three and counts the fourth, which is the only
/// piece of boundary arithmetic in this bay.
///
/// `lib.rs` pins the lane at 125 and writes that number from the mock — *"27 of
/// bay head, 6 + 8 of `.stage-list` padding, three `.cand` rows at 4 + 16.5 +
/// 4, and two 5px gaps"* — so the list is 125 - 27 - 6 - 8 = 84 and `(84 + 5) /
/// (24.5 + 5)` is 3.01. Three rows, and the extra hundredth is the half-pixel
/// the arrangement rounded up (124.5 to 125), which is less than a gap and so
/// buys nothing.
///
/// A fourth candidate is counted and not drawn, which is why `total` is carried
/// beside `rows`: a lane fuller than its height is a fact, and it is the one
/// the mock's `2 waiting` would be about.
#[test]
fn the_lane_has_room_for_the_mocks_three() {
    let panel = console(PLAUSIBLE);
    let mut held = 0;
    for total in 1..=4 {
        let rows: Vec<Candidate> = (0..total)
            .map(|deck| candidate(deck, "drift_shell + soft_points", Stage::Landed))
            .collect();
        let lane = staging(panel.layout(), &rows).expect("a candidate and a bay to draw it in");
        assert_eq!(
            lane.rows,
            total.min(3),
            "{total} candidates and the lane drew {} rows",
            lane.rows
        );
        assert_eq!(lane.total, total, "the lane was handed {total} candidates");
        assert!(
            lane.list.contains_rect(lane.row(lane.rows - 1)),
            "the last of {} rows is outside the list",
            lane.rows
        );
        held += 1;
    }
    assert_eq!(held, 4, "not every count was asked");
}

/// A row is drawn, and it is drawn inside the list.
///
/// The geometry above is what `staging` answers; this is what the paint pass
/// does with it. Each candidate adds shapes wholly inside the bay — a well and
/// the type in it — and the fourth adds none, which is the boundary above
/// asserted from the other side.
#[test]
fn each_candidate_adds_shapes_and_the_fourth_adds_none() {
    let mut panel = console(PLAUSIBLE);
    let lane = to_egui(rect_of(panel.layout(), "staging"));
    let mut view = View::new(Room::Day);

    let mut counts = Vec::new();
    for total in 0..=4 {
        view.staging = (0..total)
            .map(|deck| candidate(deck, "drift_shell + soft_points", Stage::Landed))
            .collect();
        counts.push(shapes_inside(&mut view, &mut panel, lane));
    }
    for waiting in 1..=3 {
        assert!(
            counts[waiting] > counts[waiting - 1],
            "candidate {waiting} drew nothing: {} shapes against {}",
            counts[waiting],
            counts[waiting - 1]
        );
    }
    assert_eq!(
        counts[4], counts[3],
        "a fourth candidate was drawn in a lane with room for three: {} against {}",
        counts[4], counts[3]
    );
}

/// A row is a well and four things in it: the deck it landed on, the node's
/// address, what that node's procedure calls itself, and the verdict.
///
/// The deck is what tells two rows apart in the program this panel is drawn by:
/// it plays one pair of files in four slots, each from its own copy, so one
/// save produces four builds whose names are the same string. (This said *both
/// its slots* until 2026-09-08, from a two-slot deck that is long gone.) The
/// address is what tells two rows of one build apart, and it arrived with
/// ADR-0326.
///
/// Counted rather than read, because a galley's text is not something a shape
/// carries: a row that names a node is five shapes wholly inside its own
/// rectangle — `.cand`'s well, which is exactly the row, and one galley each —
/// and the two subtractions below are what say which shape is which. A row
/// handed an empty name is four, and a row that names no node is four with the
/// name back, so a row that stopped drawing either would be caught by the count
/// it did not fall to.
///
/// The `back` capsule is two of the seven, a stroked capsule and the word in
/// it, and it is drawn on every row that names a node and has room — which at
/// `PLAUSIBLE`'s width is both of the rows below. What it is *not* drawn on is
/// the row that names none, which is the third subtraction here and the one
/// that says the capsule follows `Candidate::at` rather than the verdict.
#[test]
fn a_row_is_a_well_and_four_things_in_it() {
    let mut panel = console(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    view.staging = vec![
        at_node(
            0,
            karakuri_operation::Layer::L4,
            0,
            "soft_points",
            Stage::Overloaded,
        ),
        at_node(
            1,
            karakuri_operation::Layer::L1,
            0,
            "drift_shell",
            Stage::Landed,
        ),
    ];
    let lane = staging(panel.layout(), &view.staging).expect("two candidates and a lane");
    assert_eq!(lane.rows, 2, "two candidates and {} rows", lane.rows);

    let mut asked = 0;
    for index in 0..lane.rows {
        let drawn = shapes_inside(&mut view, &mut panel, lane.row(index));
        assert_eq!(
            drawn, 7,
            "row {index} draws {drawn} shapes and a candidate row is a well, a deck, an \
             address, a name, a verdict and a `back` capsule of two"
        );
        asked += 1;
    }
    assert_eq!(asked, 2, "not every row was asked");

    // **The name is one of the seven**, so the six beside it are the well, the
    // deck's letter, the address, the verdict and the capsule's two — none of
    // which an empty name takes with it.
    view.staging = vec![at_node(
        0,
        karakuri_operation::Layer::L4,
        0,
        "",
        Stage::Overloaded,
    )];
    let bare = shapes_inside(&mut view, &mut panel, lane.row(0));
    assert_eq!(
        bare, 6,
        "a row with no name draws {bare} shapes, so the name is not the one shape that went"
    );

    // **And a row that names no node loses three of the seven**: the address,
    // and the capsule's two. It is what says both follow `Candidate::at` — the
    // address is drawn from `Candidate::addr` rather than off the deck letter
    // beside it, and the capsule is offered by the node rather than by the
    // verdict.
    view.staging = vec![slot_row(0, "drift_shell + soft_points", Stage::Refused)];
    let unaddressed = shapes_inside(&mut view, &mut panel, lane.row(0));
    assert_eq!(
        unaddressed, 4,
        "a row that names no node draws {unaddressed} shapes, and it should be a well, a \
         deck, a name and a verdict"
    );
}

/// A row the checker turned down draws what it said, and says how many more
/// there are.
///
/// The three verdicts above this one are about a build an operator can see the
/// result of; this one has produced nothing to look at, so the word alone says
/// that a save did not take and nothing whatever about why — and *a refusal
/// carries what the next attempt needs*
/// (`docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md`).
///
/// Read off the frame rather than counted, unlike the row test above: what is
/// being checked is that a particular sentence is painted, and a version that
/// laid the diagnostic out and drew a blank galley would satisfy any count. The
/// count is asserted beside it, because *one* diagnostic and *four* are the
/// same row otherwise, and the second is the one where a repair takes more than
/// one edit.
#[test]
fn a_row_the_checker_turned_down_draws_the_first_diagnostic_and_counts_the_rest() {
    let mut panel = console(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    let lane = to_egui(rect_of(panel.layout(), "staging"));

    let words = |view: &mut View, panel: &mut Panel| -> Vec<String> {
        let ctx = drawn_once();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
        out.textures_delta.clear();
        out.shapes
            .into_iter()
            .filter(|clipped| {
                let bounds = clipped.shape.visual_bounding_rect();
                bounds.is_finite() && lane.contains_rect(bounds)
            })
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    };

    // One diagnostic: the line itself, and no count beside it — there is
    // nothing to count.
    view.staging = vec![refused_candidate(
        1,
        "drift_shell.kir",
        &["3:5: parse: expected `}`"],
    )];
    let one = words(&mut view, &mut panel);
    assert!(
        one.iter().any(|line| line == "3:5: parse: expected `}`"),
        "the row drew {one:?} and the checker said `3:5: parse: expected `}}``"
    );
    assert!(
        one.iter().any(|line| line == "did not compile"),
        "the row drew {one:?} and the verdict is `did not compile`"
    );
    assert!(
        !one.iter().any(|line| line.contains("more")),
        "one diagnostic and the row counted others: {one:?}"
    );

    // Three: the first of them, and two more said as a number rather than
    // drawn — a row is one line.
    view.staging = vec![refused_candidate(
        1,
        "drift_shell.kir",
        &[
            "3:5: parse: expected `}`",
            "7:1: type: unknown builtin `curl2`",
            "9:2: cost: 6344 ops/element exceeds the 4096 ops/element ceiling",
        ],
    )];
    let three = words(&mut view, &mut panel);
    assert!(
        three
            .iter()
            .any(|line| line.starts_with("3:5: parse: expected `}`")),
        "the row drew {three:?} and the first diagnostic is `3:5: parse: expected `}}``"
    );
    assert!(
        three.iter().any(|line| line.contains("2 more")),
        "three diagnostics and the row does not say two are not drawn: {three:?}"
    );
    assert!(
        !three.iter().any(|line| line.contains("curl2")),
        "the second diagnostic was drawn on a row that has room for one: {three:?}"
    );

    // **And no other row carries one.** A landed candidate with the same name
    // draws the name and the verdict and nothing else, which is what says the
    // sentence belongs to the stage rather than to the row.
    view.staging = vec![candidate(1, "drift_shell.kir", Stage::Landed)];
    let landed = words(&mut view, &mut panel);
    assert!(
        !landed.iter().any(|line| line.contains("parse")),
        "a landed row drew a diagnostic: {landed:?}"
    );
}

/// The four words a row can end in are the manual's own, which is ADR-0159
/// asked of this bay: the console's words are the manual's.
///
/// `console.html` says what a row's third thing is — *"whether it is on screen:
/// landed, overloaded for costing more than one frame may, refused, or did not
/// compile"* — and the transport's health capsule names the same four answers
/// in the same words. A word invented here would be the specification written
/// backwards.
///
/// The fourth is the one this most needs to hold. *Refused* is a build that
/// failed and *did not compile* is a source the checker turned down, and the
/// two are one keystroke away from being spelled the same on this side and
/// argued as different on the page (ADR-0310).
#[test]
fn the_verdicts_are_the_manuals_words() {
    let page = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual/console.html"),
    )
    .expect("the console page is the specification");
    let mut asked = 0;
    for stage in [
        Stage::Landed,
        Stage::Overloaded,
        Stage::Refused,
        Stage::NotCompiled,
    ] {
        let word = stage.word();
        assert!(
            page.contains(word),
            "the lane draws `{word}` and the console page does not say it"
        );
        asked += 1;
    }
    assert_eq!(asked, 4, "not every verdict was asked");
    // And they are four words rather than one written four times.
    let words = [
        Stage::Landed.word(),
        Stage::Overloaded.word(),
        Stage::Refused.word(),
        Stage::NotCompiled.word(),
    ];
    for (index, word) in words.iter().enumerate() {
        assert!(
            !words[..index].contains(word),
            "two verdicts read `{word}`, so a row cannot say which it is"
        );
    }
    // The letter a row is addressed by is the deck's own, and the deck a
    // candidate names is a slot of the deck the previews letter.
    assert_eq!(
        DECK_LETTERS[0], "A",
        "a candidate on slot 0 is on the deck the first preview cell calls A"
    );
}

// ---------------------------------------------------------------------------
// What in it is a control, and what is not
// ---------------------------------------------------------------------------

/// The Staging bay's own ground takes no press, with the lane full as well as
/// empty, stated rather than inferred from the absence of a hit test.
///
/// `library.rs`'s test one bay up, and the inset is its inset for its reason: a
/// boundary is claimed for a drag from `GRAB` either side of it, and that is
/// the panel taking a *divider* rather than anything in the bay.
///
/// The rows here are `Stage::Overloaded` on purpose, and that is the half of
/// this test that is about the lane rather than about the card: an overloaded
/// row offers no keep — a slot that has stopped is not a candidate anyone is
/// choosing between (ADR-0316) — so a press on one is `egui`'s, and this is
/// where that is held. The row that *does* take a press is
/// `a_press_on_a_candidate_row_asks_to_keep_it`, and the capsule inside it is
/// `the_back_capsule_is_the_smaller_box_inside_the_row`.
#[test]
fn the_staging_bays_ground_is_not_a_control() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let region = to_egui(rect_of(panel.layout(), "staging"));
    let strips = Vec::new();
    // The lane at its fullest, so the points below land on rows rather than on
    // bare card. A closure rather than one `View`, because a `View` holds a
    // frame's plan and is not `Clone`.
    let full = || {
        let mut view = showing(&strips);
        view.staging = (0..3)
            .map(|deck| candidate(deck, "drift_shell + soft_points", Stage::Overloaded))
            .collect();
        view
    };

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
        for (lane, view) in [("empty", showing(&strips)), ("full", full())] {
            assert_eq!(
                claim(&mut panel, &ctx, &view, Point::new(p.x, p.y)),
                Claim::Egui,
                "the console took the pointer at {p:?}, which is inside the {lane} Staging bay"
            );
        }
        asked += 1;
    }
    // A guard, so this cannot pass by testing nothing.
    assert_eq!(asked, points.len(), "not every point was asked");
    // **And the rows themselves**, which the five points above do not cover:
    // four of them are in the corners of the bay and the fifth is its centre,
    // so a control drawn *on a row* could sit under none of them. Each row is
    // asked at both ends and in the middle — the three places the mock's
    // `.cand` puts something.
    let lane = staging(panel.layout(), &full().staging).expect("three candidates and a lane");
    let mut rows = 0;
    for index in 0..lane.rows {
        let row = lane.row(index);
        for p in [
            egui::pos2(row.min.x + 1.0, row.center().y),
            row.center(),
            egui::pos2(row.max.x - 1.0, row.center().y),
        ] {
            assert_eq!(
                claim(&mut panel, &ctx, &full(), Point::new(p.x, p.y)),
                Claim::Egui,
                "the console took the pointer at {p:?}, which is on overloaded candidate row \
                 {index} — a stopped slot offers no keep"
            );
        }
        rows += 1;
    }
    // A guard, so this cannot pass by there being no rows to ask about.
    assert_eq!(rows, 3, "the lane drew {rows} rows and was handed three");

    // **The negative control.** It was written when there was nothing in this
    // bay to break, which made the assertions above the trivially-passing kind
    // `docs/contributing.md` §3, *A test is watched to fail before it is kept*,
    // is about; a lane with rows in it has a defect to be run against, and the
    // rows above were — `claim` given a candidate row to answer `Panel` for
    // fails at the bay's centre. This stays all the same, because it is the
    // half that says `claim` is not answering `Egui` for every point it is
    // asked: the one point around here it does *not* answer `Egui` for is the
    // boundary the lane shares with the Library above it, which the panel
    // takes for a drag.
    let above = region.min.y - karakuri_console::panel::GRAB * 0.5;
    assert_eq!(
        claim(
            &mut panel,
            &ctx,
            &showing(&strips),
            Point::new(region.center().x, above)
        ),
        Claim::Panel,
        "the boundary over the Staging bay is not the panel's, so `claim` is answering \
         `Egui` for every point it is asked"
    );
}

// ---------------------------------------------------------------------------
// The two presses a row offers
// ---------------------------------------------------------------------------

/// A press on a candidate row asks to keep that candidate, addressed by the
/// node the row is for.
///
/// The row *is* the control — `console.html`'s *The control is the row itself*
/// — which is the Library bay's list one bay up read the other way round: there
/// a row press takes a Set in hand and names no operation, and here it names
/// one outright. What decided that this act is the large target is what it
/// costs: keeping moves nothing, writes nothing, and takes a line off a list
/// (ADR-0326).
///
/// Both halves are asserted, because they fail differently: `claim` says the
/// console took the press at all, and `StagingBay::keep` says what it asked
/// for. A row that was claimed and handed back the wrong node would pass the
/// first alone.
#[test]
fn a_press_on_a_candidate_row_asks_to_keep_it() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let strips = Vec::new();
    let rows = || {
        vec![
            at_node(
                0,
                karakuri_operation::Layer::L1,
                0,
                "drift_shell",
                Stage::Landed,
            ),
            at_node(
                0,
                karakuri_operation::Layer::L4,
                1,
                "soft_points",
                Stage::Landed,
            ),
        ]
    };
    let mut view = showing(&strips);
    view.staging = rows();
    let lane = staging(panel.layout(), &view.staging).expect("two candidates and a lane");
    assert_eq!(lane.rows, 2, "two candidates and {} rows", lane.rows);

    // The left-hand end of each row, which is over the address and never over
    // the capsule at the other end.
    let mut asked = 0;
    for (index, want) in [
        (0, karakuri_operation::Layer::L1, 0u32),
        (1, karakuri_operation::Layer::L4, 1u32),
    ]
    .map(|(index, layer, at)| (index, karakuri_operation::NodeAddress { layer, index: at }))
    {
        let row = lane.row(index);
        let p = Point::new(row.min.x + 2.0, row.center().y);
        assert_eq!(
            claim(&mut panel, &ctx, &view, p),
            Claim::Panel,
            "the console handed row {index} to `egui`, and the row is the keep"
        );
        assert_eq!(
            lane.keep(&ctx, &view.staging, p),
            Some(karakuri_operation::Operation::KeepCandidate {
                deck: 0,
                node: want
            }),
            "row {index} asked for the wrong candidate"
        );
        asked += 1;
    }
    assert_eq!(asked, 2, "not every row was asked");

    // **The negative control, and it is the division the page draws.** A row
    // on `overloaded` is a slot that has stopped rather than a candidate
    // anyone is choosing between, and a row that names no node has nothing to
    // settle — neither takes this press, and a `keep` that answered off the
    // rectangle alone would hand back an operation for both.
    let mut view = showing(&strips);
    view.staging = vec![
        at_node(
            0,
            karakuri_operation::Layer::L1,
            0,
            "drift_shell",
            Stage::Overloaded,
        ),
        slot_row(1, "drift_shell + soft_points", Stage::Refused),
    ];
    let lane = staging(panel.layout(), &view.staging).expect("two candidates and a lane");
    for (index, why) in [(0, "an overloaded row"), (1, "a row that names no node")] {
        let row = lane.row(index);
        let p = Point::new(row.min.x + 2.0, row.center().y);
        assert_eq!(
            lane.keep(&ctx, &view.staging, p),
            None,
            "{why} answered a keep"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &view, p),
            Claim::Egui,
            "the console took a press on {why}"
        );
    }
}

/// The `back` capsule is the smaller box inside the row, and it asks for the
/// node's previous version.
///
/// It is the Library bay's star and its row: the capsule is asked before the
/// row it sits in, so a press on it reaches the capsule and not the keep
/// underneath — `input::claim`'s rule 4, *a control claims what it acts on and
/// no more*. The act that writes a file is the small target and the act that
/// writes nothing is the large one, which is the whole of why they are this way
/// round (ADR-0326).
///
/// The overloaded row is the one that matters, and it is why the capsule is
/// offered on more rows than the keep is: landing an earlier version is one of
/// the three ways out of a stopped slot, and it is the only one of the three
/// this bay can offer.
#[test]
fn the_back_capsule_is_the_smaller_box_inside_the_row() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let strips = Vec::new();
    let mut view = showing(&strips);
    view.staging = vec![
        at_node(
            2,
            karakuri_operation::Layer::L4,
            1,
            "soft_points",
            Stage::Overloaded,
        ),
        at_node(
            3,
            karakuri_operation::Layer::L2,
            0,
            "curl_drift",
            Stage::Landed,
        ),
    ];
    let lane = staging(panel.layout(), &view.staging).expect("two candidates and a lane");

    let mut asked = 0;
    for (index, deck, want) in [
        (
            0,
            2u8,
            karakuri_operation::NodeAddress {
                layer: karakuri_operation::Layer::L4,
                index: 1,
            },
        ),
        (
            1,
            3u8,
            karakuri_operation::NodeAddress {
                layer: karakuri_operation::Layer::L2,
                index: 0,
            },
        ),
    ] {
        let capsule = lane
            .back_capsule(&ctx, &view.staging, index)
            .unwrap_or_else(|| panic!("row {index} draws no `back` capsule"));
        assert!(
            lane.row(index).contains(capsule.center()),
            "row {index}'s capsule is not inside the row"
        );
        let p = Point::new(capsule.center().x, capsule.center().y);
        assert_eq!(
            claim(&mut panel, &ctx, &view, p),
            Claim::Panel,
            "the console handed row {index}'s capsule to `egui`"
        );
        assert_eq!(
            lane.back(&ctx, &view.staging, p),
            Some(karakuri_operation::Operation::RestoreProcedure {
                deck,
                revision: karakuri_operation::Revision::Previous(want),
            }),
            "row {index}'s capsule asked for the wrong version"
        );
        // **And the keep does not answer for the same point**, which is
        // structural rather than an ordering: `StagingBay::keep` refuses a
        // point its own capsule claims, so a press on the capsule cannot also
        // be a keep whatever order a caller asks the two in.
        assert_eq!(
            lane.keep(&ctx, &view.staging, p),
            None,
            "row {index}'s capsule is also a keep, so a press on it does two things"
        );
        asked += 1;
    }
    assert_eq!(asked, 2, "not every capsule was asked");

    // **The negative control**: a row that names no node draws no capsule and
    // answers no press, because both operations are spelled with a node.
    let mut view = showing(&strips);
    view.staging = vec![refused_candidate(
        0,
        "drift_shell.kir",
        &["3:5: parse: expected `}`"],
    )];
    let lane = staging(panel.layout(), &view.staging).expect("one candidate and a lane");
    assert_eq!(
        lane.back_capsule(&ctx, &view.staging, 0),
        None,
        "a row that names no node drew a `back` capsule"
    );
    let row = lane.row(0);
    assert_eq!(
        lane.back(
            &ctx,
            &view.staging,
            Point::new(row.max.x - 60.0, row.center().y)
        ),
        None,
        "a row that names no node answered a press where a capsule would have been"
    );
}
