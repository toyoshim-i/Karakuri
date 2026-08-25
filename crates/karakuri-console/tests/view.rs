//! What the console draws, and who gets a pointer event.
//!
//! None of this needs a window or a device: what to draw is a walk of the
//! arrangement, and who gets an event is a hit test. The one test here that
//! does need a device is not here at all — it is `examples/panel.rs`'s
//! `gpu::egui_paints_the_console_onto_a_device`, under `mod gpu` like every
//! other one in the workspace.

mod common;

use common::{drawn_once, id_of, near, rect_of, solved, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::Panel;
use karakuri_console::room::{Palette, Room};
use karakuri_console::view::{
    picture_rect, plan_into, preview_rects, region, Kind, Placed, DECKS, REGIONS,
};
use karakuri_layout::{NodeId, Point};

/// The seven bays, read off the mock's `.bay-head`s — `console.html` heads
/// exactly these and nothing else.
const BAYS: &[&str] = &[
    "library",
    "staging",
    "program",
    "inspector",
    "mixer",
    "master",
    "sequencer",
];

/// The transport and the outputs, which carry `class="bay"` for the card
/// styling and no `.bay-head` at all (ADR-0159). Two kinds and one rule: the
/// outputs row is a [`Kind::Outputs`] because it has a control in it, and it
/// is as headless as the transport.
const ROWS: &[&str] = &["transport", "outputs"];

fn planned(panel: &mut Panel) -> Vec<Placed> {
    let mut out = Vec::new();
    plan_into(panel, &mut out);
    out
}

// ---------------------------------------------------------------------------
// What is drawn
// ---------------------------------------------------------------------------

/// **Every leaf of the arrangement is asked to be drawn, and none is skipped.**
///
/// The failure this exists for is silent: a region left out of the table
/// leaves a hole in the panel with nothing anywhere saying so, and the hole is
/// the ground showing through, which is what a divider looks like. So this
/// asks the arrangement rather than the table — every visible leaf has to be
/// in the plan — and then asks the other way, so a region invented in the
/// table that the arrangement does not have is caught too.
#[test]
fn every_leaf_of_the_arrangement_is_drawn_and_none_is_skipped() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let placed = planned(&mut panel);
    let layout = panel.layout();

    for node in panel.nodes() {
        if layout.is_view(node.id) && layout.visible(node.id) {
            assert!(
                placed.iter().any(|p| p.id == node.id),
                "{:?} is a leaf of the arrangement and nothing draws it",
                layout.name(node.id)
            );
        }
    }

    // Two bays are splits: their heads sit above the regions that tile them,
    // so each is drawn and its parts are drawn inside it. The inspector is one
    // and the Program bay is the other — the picture and the deck previews
    // fold apart, so the bay around them is a split.
    let split_bays: Vec<NodeId> = ["inspector", "program"]
        .iter()
        .map(|name| id_of(layout, name))
        .collect();
    for (name, id) in ["inspector", "program"].iter().zip(&split_bays) {
        assert!(
            placed.iter().any(|p| p.id == *id),
            "the {name} bay's own head is not drawn"
        );
    }

    // And nothing else. `left-pane`, `centre` and `right-pane` are splits an
    // operator folds, not things with a face.
    for p in &placed {
        assert!(
            layout.is_view(p.id) || split_bays.contains(&p.id),
            "{:?} is a split and is being drawn as a region",
            layout.name(p.id)
        );
        assert_eq!(
            layout.name(p.id),
            Some(p.region.name),
            "a placed region carries a name the arrangement does not agree with"
        );
    }

    // Twelve leaves and the inspector, which is every row of the table.
    assert_eq!(placed.len(), REGIONS.len());

    // Tree order, which is what puts a split bay's card under the regions that
    // tile it — the inspector's panes, and the Program bay's picture and
    // preview row.
    let at = |name: &str| placed.iter().position(|p| p.region.name == name).unwrap();
    assert!(at("inspector") < at("inspector-1"));
    assert!(at("inspector-1") < at("inspector-2"));
    assert!(at("program") < at("program-view"));
    assert!(at("program-view") < at("deck-previews"));
}

/// **A bay gets a head and a row does not.**
///
/// Seven bays, two rows and two panes, and the title of each bay is the mock's
/// own word for it — so a bay renamed in the manual and not here shows the old
/// word on the face of the panel, which is exactly what ADR-0159 is about.
#[test]
fn a_bay_gets_a_head_and_a_row_does_not() {
    for name in BAYS {
        match region(name)
            .unwrap_or_else(|| panic!("no region named {name}"))
            .kind
        {
            Kind::Bay { title, .. } => assert_eq!(
                title.to_lowercase(),
                *name,
                "the bay head says {title} and the arrangement calls it {name}"
            ),
            other => panic!("{name} is a bay in the mock and a {other:?} here"),
        }
    }
    for name in ROWS {
        assert!(
            matches!(region(name).unwrap().kind, Kind::Transport | Kind::Outputs),
            "{name} has no heading in the mock and is being given one"
        );
    }
    // And the difference between the two rows is what is *in* each of them,
    // not a head on either: one draws four readouts and the other draws the
    // console's one control, and `View::draw` has to be told which is which by
    // the table rather than by comparing a name on the frame path.
    assert_eq!(
        region("transport").unwrap().kind,
        Kind::Transport,
        "the transport row is where the tempo, the beat and the frame readout are drawn"
    );
    assert_eq!(
        region("outputs").unwrap().kind,
        Kind::Outputs,
        "the Outputs row is where the one sink is drawn"
    );
    for name in ["inspector-1", "inspector-2"] {
        assert_eq!(
            region(name).unwrap().kind,
            Kind::Pane,
            "{name} sits inside a bay and has no head of its own"
        );
    }
    // The preview row sits inside a bay and has no head either, and it is a
    // kind of its own for `Kind::Picture`'s reason: `View::draw` has to know
    // which pane the cells go in.
    assert_eq!(
        region("deck-previews").unwrap().kind,
        Kind::Previews,
        "the row under the picture is where the four deck cells are drawn"
    );
    // *"The picture carries no label of its own"* — so it is not a bay, and it
    // is the one region that draws a texture.
    assert_eq!(
        region("program-view").unwrap().kind,
        Kind::Picture,
        "the picture in the Program bay is what a texture is drawn into"
    );
    assert_eq!(
        REGIONS.iter().filter(|r| r.kind == Kind::Picture).count(),
        1,
        "a second picture region: `View::draw` has one texture to give"
    );
    assert_eq!(
        REGIONS
            .iter()
            .filter(|r| matches!(r.kind, Kind::Bay { .. }))
            .count(),
        BAYS.len(),
        "the bay head has seven call sites, which is the whole of why it is a component"
    );
}

/// **The picture's rectangle comes off its own region, and it is 16:9 at the
/// width the mock draws.**
///
/// This is the rectangle a caller sizes a texture from, so getting it from
/// anything but `program-view` is a texture the wrong size — and the wrong
/// size in a way nothing on screen shows, because the picture fills whatever
/// rectangle it is given either way. At `SMALLEST` the bay's own derivation
/// says exactly what it should be: the centre track is 484, `.program-body`'s
/// 9px padding leaves 466, and 466 at 16:9 is 262. Those are the two numbers
/// the arrangement's 378 was built from, arrived at from the other end.
#[test]
fn the_pictures_rectangle_is_its_region_less_the_head_and_the_padding() {
    let layout = solved(SMALLEST);
    let rect = picture_rect(&layout).expect("the picture is on screen");

    assert!(near(rect.width(), 466.0), "{} wide", rect.width());
    assert!(near(rect.height(), 262.0), "{} tall", rect.height());
    assert!(
        (rect.width() / rect.height() - 16.0 / 9.0).abs() < 0.01,
        "the mock's own picture is 16:9 and this is {}:{}",
        rect.width(),
        rect.height()
    );

    // And it is that region's, term for term: the bay head is painted over the
    // top of it and `.program-body`'s padding is inside it.
    let region = rect_of(&layout, "program-view");
    assert!(near(rect.min.x, region.x + 9.0));
    assert!(near(rect.min.y, region.y + 27.0 + 9.0));
    assert!(near(rect.max.x, region.x + region.w - 9.0));
    // Nothing under it: the 9 below the picture in the CSS is the gap to the
    // previews, which is the split's divider, and the other 9 is under the
    // preview row and belongs to `deck-previews`.
    assert!(near(rect.max.y, region.y + region.h));
}

/// **It follows the region and not the window**, which is the failure a
/// texture sized once from the window looks exactly like until somebody drags
/// something.
#[test]
fn the_pictures_rectangle_follows_its_region_rather_than_the_window() {
    let narrow = picture_rect(&solved(SMALLEST)).expect("on screen");
    let wide = picture_rect(&solved(PLAUSIBLE)).expect("on screen");

    // A window nearly twice as wide gives the picture the width and none of
    // the height — the manual's "sized by height", seen in the rectangle a
    // texture is made from.
    assert!(wide.width() > narrow.width() + 900.0);
    assert!(near(wide.height(), narrow.height()));
    assert!(
        wide.width() < PLAUSIBLE.w && wide.height() < PLAUSIBLE.h,
        "the picture is the window's size, so it was taken from the window"
    );

    // Dragging the program's bottom edge is the operator changing the
    // picture's height, and the rectangle says so.
    let mut layout = solved(PLAUSIBLE);
    let centre = id_of(&layout, "centre");
    layout.set_divider(centre, 0, rect_of(&layout, "program").y + 600.0);
    layout.solve();
    let dragged = picture_rect(&layout).expect("on screen");
    assert!(near(dragged.height(), wide.height() + 222.0));
    assert!(near(dragged.width(), wide.width()));
}

/// **No rectangle where the picture is folded away**, which is the manual's
/// *"there is no state where it is hidden and still costing a pass"*: a caller
/// that renders into this rectangle records no pass at all when there is none.
///
/// **What carries it is the size test and not a visibility test**, and that
/// was learnt from this test rather than assumed: written with both, deleting
/// the visibility check left it passing, because a folded region keeps its
/// rectangle and loses its extent. So the check went and this is what holds
/// the remaining line — including for a picture folded by its bay rather than
/// by itself, which a visibility test and a size test answer alike and which
/// is asserted here so that the equivalence is not left as a belief.
#[test]
fn a_folded_picture_has_no_rectangle() {
    let mut layout = solved(PLAUSIBLE);
    assert!(picture_rect(&layout).is_some());

    layout.collapse(id_of(&layout, "program-view"));
    layout.solve();
    assert_eq!(picture_rect(&layout), None);

    // The previews are still there, so this is the picture being folded and
    // not the bay.
    assert!(layout.visible(id_of(&layout, "deck-previews")));

    layout.expand(id_of(&layout, "program-view"));
    layout.solve();
    assert!(picture_rect(&layout).is_some());

    // The bay folded around it, which is `g` over the picture rather than `f`.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "program"));
    layout.solve();
    assert_eq!(picture_rect(&layout), None);

    // And a solo on the previews, which leaves the picture out rather than
    // folding anything above it.
    let mut layout = solved(PLAUSIBLE);
    layout.solo(id_of(&layout, "deck-previews"));
    layout.solve();
    assert_eq!(picture_rect(&layout), None);
}

/// **The four preview cells are the mock's own, at the width the mock draws
/// them.**
///
/// Every figure here is `lib.rs`'s Program bay derivation read from the other
/// end. At `SMALLEST` the centre track is 484, `.program-body`'s 9px padding
/// either side leaves 466, and 466 less three 6px gaps over four tracks is
/// **112** — which at 16:9 is **63**, which is exactly the 63 the arrangement
/// gave `deck-previews` before its 9px of padding underneath. The sum the bay
/// was built from and the rectangles it solves to are the same numbers or the
/// bay is wrong.
///
/// The insets are three of the four on purpose: nothing at the top, because
/// the 9 above the cells in the CSS is the split's 8px divider plus
/// `program-view`'s own bottom and belongs to neither this region nor the
/// picture.
#[test]
fn the_preview_cells_are_the_mocks_at_the_width_the_mock_draws() {
    let layout = solved(SMALLEST);
    let cells = preview_rects(&layout).expect("the previews are on screen");
    let region = rect_of(&layout, "deck-previews");

    for (deck, cell) in cells.iter().enumerate() {
        assert!(
            near(cell.width(), 112.0),
            "cell {deck} is {} wide",
            cell.width()
        );
        assert!(
            near(cell.height(), 63.0),
            "cell {deck} is {} tall",
            cell.height()
        );
        // Nothing above the row, and `.program-body`'s padding under it.
        assert!(
            near(cell.min.y, region.y),
            "cell {deck} starts at {} and the region at {}",
            cell.min.y,
            region.y
        );
        assert!(
            near(cell.max.y, region.y + region.h - 9.0),
            "cell {deck} ends at {} and the region's padding leaves {}",
            cell.max.y,
            region.y + region.h - 9.0
        );
    }

    // `.previews`'s `gap: 6px`, between the tracks and nowhere else.
    for deck in 1..DECKS {
        let gap = cells[deck].min.x - cells[deck - 1].max.x;
        assert!(
            near(gap, 6.0),
            "the gap before cell {deck} is {gap} and not 6"
        );
    }

    // The row is the region less a pad either side, so the first cell's left
    // edge and the last cell's right edge are that pad in from the region.
    assert!(
        near(cells[0].min.x, region.x + 9.0),
        "the row starts at {} and the region's padding leaves {}",
        cells[0].min.x,
        region.x + 9.0
    );
    assert!(
        near(cells[DECKS - 1].max.x, region.x + region.w - 9.0),
        "the row ends at {} and the region's padding leaves {}",
        cells[DECKS - 1].max.x,
        region.x + region.w - 9.0
    );
}

/// **The cells never overlap, never leave the region, and never stop being
/// 16:9** — at any width, and especially at one much wider than the mock's.
///
/// This is the test the gap arithmetic is checked by, and the wrong version it
/// exists for is a plausible one: divide the row by four and take a gap off
/// each cell, which loses a gap's worth of width and leaves the last cell
/// short of the region's padding — four tracks have **three** gaps between
/// them and not four.
///
/// The wide end is the other half. `deck-previews` is pinned at 72 tall, so a
/// wider window widens the track and not the row, and a cell that filled its
/// track would stop being 16:9 the moment the window left `SMALLEST`. It is
/// 16:9 and centred instead, which is what these assertions say.
#[test]
fn the_preview_cells_tile_their_region_and_stay_sixteen_by_nine() {
    for width in [990.0, 1010.0, 1280.0, 1920.0, 3440.0] {
        let layout = solved(karakuri_layout::Rect {
            w: width,
            ..SMALLEST
        });
        let cells = preview_rects(&layout).expect("the previews are on screen");
        let region = rect_of(&layout, "deck-previews");
        let row_left = region.x + 9.0;
        let row_right = region.x + region.w - 9.0;

        for (deck, cell) in cells.iter().enumerate() {
            assert!(
                (cell.width() / cell.height() - 16.0 / 9.0).abs() < 0.01,
                "at {width} wide, cell {deck} is {}x{} and a preview is 16:9",
                cell.width(),
                cell.height()
            );
            assert!(
                cell.min.x >= row_left - 1e-3 && cell.max.x <= row_right + 1e-3,
                "at {width} wide, cell {deck} runs from {} to {} outside the row's {row_left}..{row_right}",
                cell.min.x,
                cell.max.x
            );
            assert!(
                cell.min.y >= region.y - 1e-3 && cell.max.y <= region.y + region.h - 9.0 + 1e-3,
                "at {width} wide, cell {deck} runs from {} to {} outside the region",
                cell.min.y,
                cell.max.y
            );
        }
        for deck in 1..DECKS {
            assert!(
                cells[deck].min.x >= cells[deck - 1].max.x - 1e-3,
                "at {width} wide, cell {deck} starts at {} and cell {} ends at {}",
                cells[deck].min.x,
                deck - 1,
                cells[deck - 1].max.x
            );
        }

        // Centred in its track: the ground either side of a cell is equal, and
        // it is the same for every cell.
        let track = (row_right - row_left - 6.0 * (DECKS - 1) as f32) / DECKS as f32;
        for (deck, cell) in cells.iter().enumerate() {
            let track_x = row_left + (track + 6.0) * deck as f32;
            let before = cell.min.x - track_x;
            let after = track_x + track - cell.max.x;
            assert!(
                near(before, after),
                "at {width} wide, cell {deck} has {before} before it and {after} after it in its track"
            );
        }

        // The row is pinned at 63 tall by the arrangement, so a wider window
        // buys width and no height at all — which is the whole reason a cell
        // cannot both fill its track and stay 16:9.
        assert!(
            near(cells[0].height(), 63.0),
            "at {width} wide the row is {} tall",
            cells[0].height()
        );
    }
}

/// **No rectangles where the row is folded away**, which is `picture_rect`'s
/// rule stated once more on the other half of the bay: a caller that renders
/// four auditions into these rectangles records no pass at all when there are
/// none, and *"a priming deck draws only while something auditions it"*.
///
/// Both folds, because they are different operations on different nodes and
/// the manual promises the previews survive one of them: `f` over the row
/// folds the row, and `g` over the bay folds the picture with it.
#[test]
fn a_folded_preview_row_has_no_rectangles() {
    let mut layout = solved(PLAUSIBLE);
    assert!(preview_rects(&layout).is_some());

    layout.collapse(id_of(&layout, "deck-previews"));
    layout.solve();
    assert_eq!(preview_rects(&layout), None);

    // The picture is still there, so this is the row being folded and not the
    // bay — *"The deck previews under it are auditions of their own, so they
    // stay when it goes"*, read the other way round.
    assert!(picture_rect(&layout).is_some());

    layout.expand(id_of(&layout, "deck-previews"));
    layout.solve();
    assert!(preview_rects(&layout).is_some());

    // **And the fold the other way round, which is the sentence the example's
    // readout now prints**: *"fold the picture away (f over it) and deck A
    // keeps the loop awake on its own"*. The manual's own words are the same
    // claim — *"The deck previews under it are auditions of their own, so they
    // stay when it goes"* — and a readout that says a thing the arrangement
    // does not do is how this project has been wrong twice about what folding
    // the picture costs.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "program-view"));
    layout.solve();
    assert_eq!(picture_rect(&layout), None);
    assert!(
        preview_rects(&layout).is_some(),
        "the picture is folded and the previews went with it, so an audition \
         that should still be running has nowhere to go"
    );

    // The bay folded around it.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "program"));
    layout.solve();
    assert_eq!(preview_rects(&layout), None);

    // And a solo on the picture, which leaves the row out rather than folding
    // it.
    let mut layout = solved(PLAUSIBLE);
    layout.solo(id_of(&layout, "program-view"));
    layout.solve();
    assert_eq!(preview_rects(&layout), None);
}

// ---------------------------------------------------------------------------
// The palette
// ---------------------------------------------------------------------------

/// Every colour in a palette, so a transcription that left one behind is one
/// comparison rather than fourteen.
fn colours(p: &Palette) -> Vec<(&'static str, egui::Color32)> {
    vec![
        ("ground", p.ground),
        ("panel", p.panel),
        ("well", p.well),
        ("line", p.line),
        ("hair", p.hair),
        ("text", p.text),
        ("dim", p.dim),
        ("faint", p.faint),
        ("mint", p.mint),
        ("pink", p.pink),
        ("lav", p.lav),
        ("sun", p.sun),
        ("tint", p.tint),
        ("glow", p.glow),
        ("glow_pink", p.glow_pink),
    ]
}

/// Perceived lightness, near enough to tell a white room from a black one.
fn luma(c: egui::Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

/// **The palette answers for both rooms**, and answers differently in each.
///
/// The failure worth catching is a colour transcribed once and left: the two
/// blocks in `style.css` are fifteen near-identical lines each, and one line
/// copied from the wrong block is invisible until somebody switches rooms in a
/// dark hall.
#[test]
fn the_palette_answers_for_both_rooms() {
    let day = Room::Day.palette();
    let night = Room::Night.palette();

    for ((name, a), (_, b)) in colours(&day).into_iter().zip(colours(&night)) {
        assert_ne!(
            a, b,
            "--c-{name} is the same colour in both rooms, so one of them was not transcribed"
        );
    }

    // "Pastel on white by day and lit on black in a dark hall."
    assert!(luma(day.ground) > 0.8, "the day ground is not white");
    assert!(luma(day.text) < 0.3, "the day ink is not dark");
    assert!(luma(night.ground) < 0.1, "the night ground is not black");
    assert!(luma(night.text) > 0.8, "the night ink is not light");

    // A bay is a card standing off the ground, in both rooms. This is what
    // makes an empty bay visible at all, which is the whole of this pass.
    assert!(luma(day.panel) > luma(day.ground));
    assert!(luma(night.panel) > luma(night.ground));

    // The room key toggles, and toggling twice is where it started.
    assert_eq!(Room::Day.other(), Room::Night);
    assert_eq!(Room::Night.other().other(), Room::Night);
    assert_eq!(Room::Day.palette(), Palette::DAY);
    assert_eq!(Room::Night.palette(), Palette::NIGHT);
}

// ---------------------------------------------------------------------------
// The input rule
// ---------------------------------------------------------------------------

/// The middle of the first boundary the arrangement has.
fn on_a_boundary(panel: &mut Panel) -> Point {
    panel.solve();
    let layout = panel.layout();
    let (split, index) = layout.boundaries().next().expect("no boundary at all");
    let gap = layout.boundary(split, index).expect("no pair");
    Point::new(gap.x + gap.w * 0.5, gap.y + gap.h * 0.5)
}

/// Where the panel draws the one control it draws: the `solo` pill in the
/// Program bay's head, near the right end of it. **The place a boundary and a
/// widget are closest**, and so the place the rule is worth stating.
fn on_the_solo_pill(panel: &mut Panel) -> Point {
    panel.solve();
    let program = rect_of(panel.layout(), "program");
    Point::new(program.x + program.w - 24.0, program.y + 14.0)
}

/// **A pointer on a boundary reaches the panel, and `egui` does not see it.**
#[test]
fn a_pointer_on_a_boundary_is_the_panels() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let p = on_a_boundary(&mut panel);
    assert_eq!(claim(&mut panel, &drawn_once(), p), Claim::Panel);
}

/// **A pointer anywhere else is `egui`'s** — including on the one thing the
/// panel draws that a hand would reach for.
#[test]
fn a_pointer_off_a_boundary_is_eguis() {
    let ctx = drawn_once();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();

    let library = rect_of(panel.layout(), "library");
    let middle = Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);
    assert_eq!(claim(&mut panel, &ctx, middle), Claim::Egui);

    let pill = on_the_solo_pill(&mut panel);
    assert_eq!(
        claim(&mut panel, &ctx, pill),
        Claim::Egui,
        "the solo pill is not on a boundary, so it is egui's"
    );

    // Outside the window entirely: nothing to grab, so egui's.
    assert_eq!(
        claim(&mut panel, &ctx, Point::new(-40.0, -40.0)),
        Claim::Egui
    );
}

/// **A drag in hand keeps its claim, wherever the pointer wanders.**
///
/// This is the one that is a bug waiting to happen. A claim re-decided from
/// the pointer on every event hands the middle of a drag to `egui` the moment
/// the pointer leaves the six pixels either side of the boundary — which it
/// does immediately, because a drag is how a boundary gets anywhere — and then
/// two things think they are dragging. The release matters just as much: asked
/// after `released`, the claim sees no drag and hands `egui` a button-up it
/// never saw the button-down for.
#[test]
fn a_drag_in_hand_keeps_its_claim_wherever_the_pointer_goes() {
    let ctx = drawn_once();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let start = on_a_boundary(&mut panel);
    let pill = on_the_solo_pill(&mut panel);
    panel.solve();
    let library = rect_of(panel.layout(), "library");
    let middle = Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);

    // Before the press, the two elsewhere-points are egui's.
    assert_eq!(claim(&mut panel, &ctx, pill), Claim::Egui);
    assert_eq!(claim(&mut panel, &ctx, middle), Claim::Egui);

    panel.press(start);
    assert!(panel.dragging());

    for wandered in [pill, middle, Point::new(-500.0, 4000.0), start] {
        assert_eq!(
            claim(&mut panel, &ctx, wandered),
            Claim::Panel,
            "a boundary is in hand and the claim was given away at {wandered:?}"
        );
        panel.moved(wandered);
    }

    // The release is still the panel's, and it has to be asked before
    // `released` takes the drag out of hand.
    assert_eq!(claim(&mut panel, &ctx, middle), Claim::Panel);
    panel.released();
    assert!(!panel.dragging());

    // And afterwards the claim is back where it was.
    assert_eq!(claim(&mut panel, &ctx, pill), Claim::Egui);
    assert_eq!(claim(&mut panel, &ctx, middle), Claim::Egui);
    let boundary = on_a_boundary(&mut panel);
    assert_eq!(claim(&mut panel, &ctx, boundary), Claim::Panel);
}
