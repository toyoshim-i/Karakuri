//! **The Staging lane: a card, a head, and a row per deck slot with a verdict
//! outstanding — and, with nothing outstanding, a card and a head and nothing
//! else.**
//!
//! The lane's first pass under
//! [ADR-0200](../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
//! draws three of the seven things a candidate row could carry — the deck,
//! what the build calls itself, and whether it is on screen — and the argument
//! for the four it omits is written out in `view`'s module documentation and
//! at `view::staging`.
//!
//! **The empty case is still the case this file leads with**, because it is
//! still this lane's ordinary state: *"a staging lane with nothing in it is
//! simply a run in which nobody has rewritten a procedure yet — which is every
//! run at its start and most runs throughout"*. A decision to draw nothing
//! needs a test more than a decision to draw something does: nothing about
//! deleting the argument would make a suite fail, and what would be added back
//! is what the page refuses.
//!
//! Six things:
//!
//! 1. **With nothing outstanding the bay draws the shapes of a bay with no
//!    body**, asserted against the Sequencer bay — the lane's twin in this
//!    console's furniture: a title, no pill, no grip, and nothing in its body
//!    at all. It is a claim about the paint pass, so it is made by drawing a
//!    frame and counting.
//! 2. **Nothing else the console is handed puts anything there.** The
//!    Library's names, the mixer's strips, the Inspector's panes and a picture
//!    are all written onto the `View` and the lane is the same two shapes
//!    after, which is what says the bay reads none of them: the one field it
//!    reads is `View::staging`.
//! 3. **A candidate draws a row**, and the rows are where the arithmetic says
//!    — the list box off `.stage-list`'s three paddings, a stride of a
//!    `.cand` and `.stage-list`'s gap, every row inside the list.
//! 4. **The lane holds the mock's three and no more**, which is the one piece
//!    of boundary arithmetic in this bay: `lib.rs` pins it at 125 because the
//!    mock draws three rows and two gaps, and a fourth candidate is counted
//!    and not drawn.
//! 5. **The head is untouched by any of it**, asserted with the lane full as
//!    well as empty: the mock's `2 waiting` is a readout and a bay head's
//!    pills are its controls.
//! 6. **Nothing in it is a control**, asked of a full lane as well as an empty
//!    one — a row is a readout, and *keep* and *put a node's previous version
//!    back* are the two controls this pass does not add.
//!
//! And the three words a row can end in are the manual's own, which is
//! ADR-0159 asked of this bay: *"landed, rolled back for costing too much, or
//! refused by the checker"*.
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

/// One candidate: a build that landed on a deck slot and has not been judged.
fn candidate(deck: usize, name: &str, stage: Stage) -> Candidate {
    Candidate {
        deck,
        name: name.to_owned(),
        stage,
    }
}

/// Every shape the console paints **wholly inside** `rect`, on one frame.
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

/// **The Staging lane draws what a bay with no body draws, and the Sequencer
/// bay is what that is.**
///
/// Not a count of shapes — `egui` is free to tessellate a card differently
/// tomorrow — but two bays held against each other. The Sequencer is the
/// lane's exact twin in the console's own furniture: a title, no pill, **no
/// grip**, and nothing in its body at all. `library.rs` holds the Library
/// against the Master for the same reason and picks a different twin because
/// the Library has a grip and the Master has one; a Staging bay held against
/// the Master would be six grip dots short and the assertion would be about
/// the grip.
///
/// So the two draw the same shapes, or Staging is drawing something a lane
/// with no candidate does not have: a `.cand` row, a placeholder, or the
/// head's `2 waiting` where there is nothing to count.
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

/// **Nothing the console is handed *except its own field* reaches the lane.**
///
/// The four things a caller writes onto a `View` that this bay might have been
/// reading are the Library's names, the mixer's strips, the Inspector's panes
/// and the canvas; a fifth, the picture, takes a device and is `None` in every
/// test in this crate. A console with all four filled is the fullest this
/// crate can make one, and the lane is the same bay it was empty — which is
/// what says a row comes from `View::staging` and from nothing else. That
/// field is the one thing deliberately left alone here; the tests below are
/// what fill it.
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

/// **The lane's head is the Sequencer's head, with the lane full and with it
/// empty.**
///
/// The mock's Staging head reads `2 waiting` and this one reads nothing, for
/// the reason `view::staging` gives: a bay head's pills are the mock's
/// *controls* — every readout in a head is undrawn, `previews 3 of 4`
/// included — and the number would say what the rows already say, this lane
/// having no truncation to report where the Library's foot has.
///
/// **Asked with three candidates in the lane**, which is the state a count
/// would be drawn in and is the reason this is a paint-level assertion rather
/// than the table read it used to be: the lane is `Kind::Staging` now, so the
/// pills it does not draw are not in the table to be counted. The head strip
/// is the top [`HEAD_H`] of each bay, and the Sequencer's is the same head
/// over an empty bay — so the two are equal, or this one has grown a pill.
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

/// **A candidate is a row, and the rows are where `.stage-list` and `.cand`
/// put them.**
///
/// The whole box, term for term: the list is the bay under its head, inset by
/// `.stage-list`'s `padding: 6px 9px 8px` — which is the one padding in this
/// console that is not the same top and bottom — and a row is [`CAND_H`] tall
/// with [`STAGE_GAP`] between one and the next and none above the first.
///
/// **Asked at both windows**, though the bay is the same 218 x 125 at each:
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
            candidate(0, "drift_shell + soft_points", Stage::RolledBack),
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

/// **The lane holds the mock's three and counts the fourth**, which is the
/// only piece of boundary arithmetic in this bay.
///
/// `lib.rs` pins the lane at 125 and writes that number from the mock — *"27 of
/// bay head, 6 + 8 of `.stage-list` padding, three `.cand` rows at 4 + 16.5 + 4,
/// and two 5px gaps"* — so the list is 125 - 27 - 6 - 8 = **84** and
/// `(84 + 5) / (24.5 + 5)` is 3.01. Three rows, and the extra hundredth is the
/// half-pixel the arrangement rounded up (124.5 to 125), which is less than a
/// gap and so buys nothing.
///
/// **A fourth candidate is counted and not drawn**, which is why `total` is
/// carried beside `rows`: a lane fuller than its height is a fact, and it is
/// the one the mock's `2 waiting` would be about.
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

/// **A row is drawn, and it is drawn inside the list.**
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

/// **A row is a well and three things in it: the deck it landed on, what the
/// build calls itself, and the verdict.**
///
/// The deck is what tells two rows apart in the program this panel is drawn
/// by: it plays one pair of files in both its slots, so one save produces two
/// builds whose names are the same string.
///
/// **Counted rather than read**, because a galley's text is not something a
/// shape carries: every row is four shapes wholly inside its own rectangle —
/// `.cand`'s well, which is exactly the row, and one galley each — and a row
/// handed an empty name is three, which is what says the name is one of the
/// three and the deck and the verdict are the other two. A row that stopped
/// saying which deck it landed on would be three and two.
#[test]
fn a_row_is_a_well_and_three_things_in_it() {
    let mut panel = console(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    view.staging = vec![
        candidate(0, "drift_shell + soft_points", Stage::RolledBack),
        candidate(1, "drift_shell + soft_points", Stage::Landed),
    ];
    let lane = staging(panel.layout(), &view.staging).expect("two candidates and a lane");
    assert_eq!(lane.rows, 2, "two candidates and {} rows", lane.rows);

    let mut asked = 0;
    for index in 0..lane.rows {
        let drawn = shapes_inside(&mut view, &mut panel, lane.row(index));
        assert_eq!(
            drawn, 4,
            "row {index} draws {drawn} shapes and a candidate row is a well, a deck, a name \
             and a verdict"
        );
        asked += 1;
    }
    assert_eq!(asked, 2, "not every row was asked");

    // **The name is one of the four**, so the two beside the well and the name
    // are the deck's letter and the verdict — neither of which an empty name
    // takes with it.
    view.staging = vec![candidate(0, "", Stage::RolledBack)];
    let bare = shapes_inside(&mut view, &mut panel, lane.row(0));
    assert_eq!(
        bare, 3,
        "a row with no name draws {bare} shapes, so the name is not the one shape that went"
    );
}

/// **The three words a row can end in are the manual's own**, which is
/// ADR-0159 asked of this bay: the console's words are the manual's.
///
/// `console.html` says what a row's third thing is — *"whether it is on
/// screen: landed, rolled back for costing too much, or refused by the
/// checker"* — and the deck head's `landed` pill names the same three answers
/// in the same words. A word invented here would be the specification written
/// backwards.
#[test]
fn the_verdicts_are_the_manuals_words() {
    let page = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual/console.html"),
    )
    .expect("the console page is the specification");
    let mut asked = 0;
    for stage in [Stage::Landed, Stage::RolledBack, Stage::Refused] {
        let word = stage.word();
        assert!(
            page.contains(word),
            "the lane draws `{word}` and the console page does not say it"
        );
        asked += 1;
    }
    assert_eq!(asked, 3, "not every verdict was asked");
    // And they are three words rather than one written three times.
    let words = [
        Stage::Landed.word(),
        Stage::RolledBack.word(),
        Stage::Refused.word(),
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
// Nothing in it is a control
// ---------------------------------------------------------------------------

/// **Nothing in the Staging bay takes a press, with the lane full as well as
/// empty**, stated rather than inferred from the absence of a hit test.
///
/// `library.rs`'s test one bay up, and the inset is its inset for its reason:
/// a boundary is claimed for a drag from `GRAB` either side of it, and that is
/// the panel taking a *divider* rather than anything in the bay.
///
/// **The full lane is the half that is not trivial now.** A row is a readout
/// and the two controls the page draws on one — *keep a candidate* and *put a
/// node's previous version back* — are not added by this pass: both address a
/// node, and the node is the thing nothing here can name (`view::staging`). So
/// a press anywhere on a row is `egui`'s, and a lane that had grown a control
/// would say so at exactly the points a row is drawn at.
#[test]
fn nothing_in_the_staging_lane_is_a_control() {
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
            .map(|deck| candidate(deck, "drift_shell + soft_points", Stage::RolledBack))
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
                "the console took the pointer at {p:?}, which is on candidate row {index}"
            );
        }
        rows += 1;
    }
    // A guard, so this cannot pass by there being no rows to ask about.
    assert_eq!(rows, 3, "the lane drew {rows} rows and was handed three");

    // **The negative control.** It was written when there was nothing in this
    // bay to break, which made the assertions above the trivially-passing kind
    // [P-0089](../../../docs/principles/0089-a-check-you-have-not-watched-fail-is-guessing.md)
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
