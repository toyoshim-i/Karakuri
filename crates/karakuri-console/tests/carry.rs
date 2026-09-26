//! Drag-and-drop interactions carrying a Set from a Library row onto a mixer strip or deck preview cell.

mod common;

use common::{drawn_once, point, rect_of, showing, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Landing, Panel, Released};
use karakuri_console::room::size;
use karakuri_console::view::{
    library, mixer, program_bay, rearrange, LibraryBay, Mixer, ProgramBay, Rows, Scope, Strip,
    Tally, View,
};
use karakuri_layout::Point;
use karakuri_operation::{BlendMode, Operation};

/// A listing of Sets and nothing else, which is what every test in this file is
/// about: a row with no entry in the kinds beside it is a Set with no badge,
/// which is the seam's own default (`view::RowKind`).
fn listed(names: &[String]) -> Rows<'_> {
    Rows { names, kinds: &[] }
}

/// The mock's own library, as names — `library.rs`'s list, and the same five: a
/// drag that named the wrong row has four wrong answers to give.
fn sets() -> Vec<String> {
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

/// Helper creating four strips with distinct residencies, including one live deck.
fn strips() -> Vec<Strip> {
    [Tally::Live, Tally::Priming, Tally::Allocated, Tally::Live]
        .into_iter()
        .enumerate()
        .map(|(slot, tally)| Strip {
            name: format!("slot{slot}"),
            tally,
            requested: tally,
            gain: 0.2 + 0.15 * slot as f32,
            gain_to: None,
            opacity: 0.8 - 0.15 * slot as f32,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: karakuri_console::view::Mask::None,
            mask_angle: 0.0,
            level: None,
            is_muted: false,
            is_soloed: false,
        })
        .collect()
}

/// A console with a store and a deck behind it: the mock's five rows, its four
/// scope chips, and four strips to drop onto.
fn console() -> (View, Panel, egui::Context) {
    let mut view = showing(&strips());
    view.library = sets();
    // The chips as well as the rows, because they are the same bay: a console
    // handed a listing and no scopes draws its list a scope row higher up.
    view.scopes = Scope::ALL.to_vec();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    (view, panel, drawn_once())
}

fn bay(panel: &Panel, view: &View) -> LibraryBay {
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
    .expect("the library bay lists its rows")
}

fn strips_bay<'a>(panel: &Panel, ctx: &egui::Context, view: &'a View) -> Mixer<'a> {
    mixer(ctx, panel.layout(), &view.mixer).expect("the mixer bay draws its strips")
}

/// The middle of the `index`th drawn row.
fn row(bay: &LibraryBay, index: usize) -> Point {
    point(bay.row(index).center())
}

/// The middle of the `deck`th strip.
fn strip_at(bay: &Mixer<'_>, deck: u8) -> Point {
    point(
        bay.selected(deck)
            .unwrap_or_else(|| panic!("the bay draws a strip for deck {deck}"))
            .center(),
    )
}

// ---------------------------------------------------------------------------
// What a press takes in hand
// ---------------------------------------------------------------------------

/// Clicking a library row selects its Set into hand; clicks below drawn rows claim nothing.
#[test]
fn a_press_on_a_row_takes_that_rows_set_in_hand() {
    let (view, panel, _ctx) = console();
    let bay = bay(&panel, &view);
    assert!(
        bay.rows >= 3,
        "the bay listed {} rows at the plausible console, which is too few to tell one from \
         another",
        bay.rows
    );

    for index in 0..bay.rows {
        let taken = bay
            .take(listed(&view.library), row(&bay, index))
            .unwrap_or_else(|| panic!("no row answered a press on row {index}"));
        assert_eq!(taken.row, index, "a press on row {index} took another row");
        assert_eq!(
            taken.set, view.library[index],
            "row {index} took a Set that is not the one on it"
        );
    }

    // Space below the last row, before the foot.
    let below = bay.row(bay.rows - 1);
    let ground = Point::new(below.center().x, below.max.y + size::LIB_ROW_H * 0.5);
    assert!(
        ground.y < bay.foot.min.y,
        "the sweep is asking about the foot rather than about the list's ground"
    );
    assert_eq!(
        bay.take(listed(&view.library), ground),
        None,
        "the list's own ground took a Set in hand"
    );

    // Undrawn rows past the bottom of the list and below the foot are unreachable.
    let long: Vec<String> = (0..80).map(|n| format!("set{n:02}")).collect();
    let tall = library(panel.layout(), &view.scopes, &long, None, None, 0.0)
        .expect("a bay with rows in it");
    assert!(
        tall.rows < long.len(),
        "the bay drew all {} rows, so there is no undrawn row to ask about",
        long.len()
    );
    // The first row completely below the list is neither drawn nor pressable.
    let undrawn = point(
        (tall.rows..long.len())
            .map(|index| tall.row(index))
            .find(|row| row.min.y >= tall.list.max.y)
            .expect("every row of a listing of eighty reaches the list")
            .center(),
    );
    assert!(
        undrawn.y > tall.list.max.y,
        "the row after the last drawn one is still inside the list, so the rows do not fill it"
    );
    assert_eq!(
        tall.take(listed(&long), undrawn),
        None,
        "a press below the list took a Set the bay never drew"
    );

    // Rows beyond the listing length hit-test to None.
    assert_eq!(
        bay.take(Rows::NONE, row(&bay, 0)),
        None,
        "a bay with nothing listed under it still handed a Set over"
    );
}

/// Verifies that pressing on an open reading block does not initiate a drag/carry.
#[test]
fn a_press_on_an_open_reading_takes_nothing_in_hand() {
    use karakuri_console::view::{Published, Reading};
    let (mut view, panel, _ctx) = console();
    view.read(Reading {
        id: view.library[0].clone(),
        knobs: ["radius", "turbulence", "amount"]
            .iter()
            .map(|key| Published {
                key: (*key).to_owned(),
                range: "0 – 1 · 0.5".to_owned(),
            })
            .collect(),
        capacity: None,
        emits: None,
        nodes: 1,
        described: 1,
    });
    let bay = bay(&panel, &view);
    let block = bay.reading.expect("the reading is open under the cursor");
    assert!(block.rows >= 3, "the block is too short to press into");

    for line in 0..block.rows {
        let at = point(block.row(line).center());
        assert!(
            at.y > bay.row(0).max.y,
            "line {line} of the reading is above the row it opened under"
        );
        assert_eq!(
            bay.take(listed(&view.library), at),
            None,
            "a press on line {line} of the reading took a Set in hand"
        );
    }

    // Rows below the reading remain unchanged, shifted down by the block height.
    for index in (block.under)..bay.rows {
        let taken = bay
            .take(listed(&view.library), row(&bay, index))
            .unwrap_or_else(|| panic!("row {index} under the block answered nothing"));
        assert_eq!(
            taken.set, view.library[index],
            "row {index} under the block took the wrong Set"
        );
    }
}

/// Active carry gesture reports in-hand status without behaving as a fader or showing resize cursors.
#[test]
fn a_carry_is_in_hand_until_it_is_let_go() {
    use karakuri_console::panel::InHand;
    let (view, mut panel, _ctx) = console();
    let bay = bay(&panel, &view);
    let at = row(&bay, 1);

    assert_eq!(
        panel.in_hand(),
        None,
        "something was in hand before a press"
    );
    let taken = bay.take(listed(&view.library), at).expect("row 1 is drawn");
    panel.carry(at, taken.set, taken.procedure);
    assert_eq!(
        panel.in_hand(),
        Some(InHand::Carrying),
        "a carried Set is not what the panel says it has hold of"
    );
    assert!(panel.dragging(), "a carry is not a drag in hand");
    panel.released(None);
    assert_eq!(panel.in_hand(), None, "the release let go of nothing");
}

// ---------------------------------------------------------------------------
// What a move does, which is nothing
// ---------------------------------------------------------------------------

/// Carry motion emits no intermediate operations across library rows, strips, or viewport edges.
#[test]
fn a_carry_emits_nothing_until_it_is_let_go() {
    let (view, mut panel, ctx) = console();
    let bay = bay(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);
    let at = row(&bay, 0);
    let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
    panel.carry(at, taken.set, taken.procedure);

    let mut asked = 0;
    for over in [
        row(&bay, 1),
        row(&bay, 2),
        strip_at(&strips, 0),
        strip_at(&strips, 1),
        strip_at(&strips, 3),
        Point::new(-40.0, -40.0),
    ] {
        assert_eq!(
            panel.moved(over),
            None,
            "a move with a Set in hand said something happened"
        );
        asked += 1;
    }
    assert_eq!(asked, 6, "not every move was asked");
    assert!(
        panel.dragging(),
        "the carry was let go of by a move rather than by a release"
    );
}

// ---------------------------------------------------------------------------
// What the drop asks for
// ---------------------------------------------------------------------------

/// Releasing a carry over a mixer strip emits an operation loading that Set onto the targeted deck.
#[test]
fn a_drop_on_a_strip_asks_to_load_that_strips_deck() {
    let (view, mut panel, ctx) = console();
    let bay = bay(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);

    // Presses outside any strip resolve to None for all rows and decks.
    for index in 0..bay.rows {
        assert_eq!(
            strips.dropped(row(&bay, index)),
            None,
            "row {index} of the Library bay is over a mixer strip, so this test cannot tell a \
             destination resolved at the press from one resolved at the release"
        );
    }

    for deck in 0..4u8 {
        for index in 0..bay.rows {
            let at = row(&bay, index);
            let taken = bay.take(listed(&view.library), at).expect("a drawn row");
            panel.carry(at, taken.set, taken.procedure);
            let onto = strip_at(&strips, deck);
            assert_eq!(panel.moved(onto), None);
            assert_eq!(
                panel.released(strips.dropped(onto).map(Landing::Deck)),
                Some(Released::Dropped(Operation::LoadSet {
                    deck,
                    set: view.library[index].clone(),
                })),
                "row {index} let go over strip {deck} asked for something else"
            );
        }
    }
}

/// Releasing a carry outside valid drop targets cancels the gesture without emitting operations.
#[test]
fn a_drop_on_nothing_asks_for_nothing() {
    let (view, mut panel, ctx) = console();
    let bay = bay(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);
    let transport = rect_of(panel.layout(), "transport");

    let mut asked = 0;
    for over in [
        row(&bay, 0),
        Point::new(transport.x + transport.w * 0.5, transport.y + 4.0),
        Point::new(-40.0, -40.0),
    ] {
        let at = row(&bay, 0);
        let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
        panel.carry(at, taken.set, taken.procedure);
        assert_eq!(panel.moved(over), None);
        assert_eq!(
            panel.released(strips.dropped(over).map(Landing::Deck)),
            Some(Released::Nowhere {
                set: view.library[0].clone(),
            }),
            "a drop at {over:?} asked for a load"
        );
        assert!(!panel.dragging(), "the carry is still in hand");
        asked += 1;
    }
    assert_eq!(asked, 3, "not every drop was asked");
}

/// Verifies that dropping onto a live deck strip requests a load regardless of residency state.
#[test]
fn a_drop_on_a_live_deck_still_asks_for_the_load() {
    let (view, mut panel, ctx) = console();
    let bay = bay(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);
    assert_eq!(
        view.mixer[0].tally,
        Tally::Live,
        "deck A is not on air, so this test is not about a drop on one that is"
    );

    let at = row(&bay, 0);
    let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
    panel.carry(at, taken.set, taken.procedure);
    let onto = strip_at(&strips, 0);
    assert_eq!(
        panel.released(strips.dropped(onto).map(Landing::Deck)),
        Some(Released::Dropped(Operation::LoadSet {
            deck: 0,
            set: view.library[0].clone(),
        })),
        "a drop on the deck the room is watching was refused by the panel"
    );

    // Verifies destination points across inverted strip residencies.
    let mut turned = view.mixer.clone();
    for (slot, strip) in turned.iter_mut().enumerate() {
        strip.tally = [
            Tally::Allocated,
            Tally::Live,
            Tally::Priming,
            Tally::Allocated,
        ][slot];
        strip.requested = strip.tally;
    }
    let other = showing(&turned);
    let after = strips_bay(&panel, &ctx, &other);
    for deck in 0..4u8 {
        assert_eq!(
            after.dropped(strip_at(&strips, deck)),
            Some(deck),
            "strip {deck} answered a different deck once its residency changed"
        );
    }
}

// ---------------------------------------------------------------------------
// The claim, and the mark on the row
// ---------------------------------------------------------------------------

/// Pressing a library row captures pointer events across boundaries until released.
#[test]
fn a_carry_keeps_its_claim_while_the_pointer_leaves_the_bay() {
    let (view, mut panel, ctx) = console();
    let bay = bay(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);
    let at = row(&bay, 2);

    assert_eq!(
        claim(&mut panel, &ctx, &view, at),
        Claim::Panel,
        "a press on a library row went to `egui`"
    );
    let taken = bay.take(listed(&view.library), at).expect("row 2 is drawn");
    panel.carry(at, taken.set, taken.procedure);

    let mut asked = 0;
    for over in [
        strip_at(&strips, 1),
        Point::new(-40.0, -40.0),
        Point::new(PLAUSIBLE.w + 200.0, PLAUSIBLE.h * 0.5),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, over),
            Claim::Panel,
            "the carry lost its claim at {over:?}"
        );
        panel.moved(over);
        asked += 1;
    }
    assert_eq!(asked, 3, "not every point was asked");

    let away = Point::new(-40.0, -40.0);
    panel.released(strips.dropped(away).map(Landing::Deck));
    assert_eq!(
        claim(&mut panel, &ctx, &view, away),
        Claim::Egui,
        "the panel kept the pointer after the gesture ended"
    );
}

/// Taking a row updates the library cursor mark, which persists across release.
#[test]
fn the_row_a_hand_takes_is_the_row_the_cursor_marks() {
    let (mut view, panel, _ctx) = console();
    let bay = bay(&panel, &view);
    assert_eq!(view.cursor_row(), 0, "the cursor does not start at the top");

    for index in (0..bay.rows).rev() {
        let taken = bay
            .take(listed(&view.library), row(&bay, index))
            .expect("a drawn row");
        let was = view.cursor_row();
        assert_eq!(
            view.point_at(taken.row),
            index != was,
            "the mark answered a move it did not make"
        );
        assert_eq!(
            view.cursor_row(),
            index,
            "the mark is not on the row the hand took"
        );
    }

    // The same row again is not a move, and costs no frame.
    assert!(
        !view.point_at(0),
        "pointing at the row the cursor is already on read as a move"
    );
    // And a row the listing does not have is refused rather than clamped.
    assert!(!view.point_at(view.library.len()));
    assert_eq!(
        view.cursor_row(),
        0,
        "a refused press moved the mark anyway"
    );
}

// ---------------------------------------------------------------------------
// The other set of rectangles a drop can land on
// ---------------------------------------------------------------------------

/// Helper creating a console with store, strips, and arranged Program bay cells.
fn with_cells(strips: &[Strip]) -> (View, Panel, egui::Context) {
    let mut view = showing(strips);
    view.library = sets();
    view.scopes = Scope::ALL.to_vec();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    rearrange(&mut panel, view.canvas);
    panel.solve();
    (view, panel, drawn_once())
}

fn program(panel: &Panel, view: &View) -> ProgramBay {
    program_bay(panel.layout(), view.canvas).expect("the Program bay is on screen")
}

/// The middle of the `deck`th preview cell's image.
fn cell_at(bay: &ProgramBay, deck: u8) -> Point {
    point(bay.cells.expect("the preview row is on screen")[usize::from(deck)].center())
}

/// Dropping onto a deck preview cell loads the Set into that cell's deck.
#[test]
fn a_drop_on_a_preview_cell_asks_to_load_that_cells_deck() {
    let (view, mut panel, ctx) = with_cells(&strips());
    let bay = bay(&panel, &view);
    let cells = program(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);

    for deck in 0..4u8 {
        let onto = cell_at(&cells, deck);
        assert_eq!(
            strips.dropped(onto),
            None,
            "cell {deck} is inside a mixer strip, so a release there has two answers"
        );
        for index in 0..bay.rows {
            let at = row(&bay, index);
            let taken = bay.take(listed(&view.library), at).expect("a drawn row");
            panel.carry(at, taken.set, taken.procedure);
            assert_eq!(panel.moved(onto), None);
            assert_eq!(
                panel.released(cells.dropped(onto, view.mixer.len()).map(Landing::Deck)),
                Some(Released::Dropped(Operation::LoadSet {
                    deck,
                    set: view.library[index].clone(),
                })),
                "row {index} let go over cell {deck} asked for something else"
            );
        }
    }
}

/// Preview cells corresponding to nonexistent deck slots decline drops.
#[test]
fn a_cell_whose_letter_names_no_slot_takes_no_drop() {
    let three: Vec<Strip> = strips().into_iter().take(3).collect();
    let (view, mut panel, ctx) = with_cells(&three);
    let bay = bay(&panel, &view);
    let cells = program(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);
    assert_eq!(view.mixer.len(), 3, "this deck is not three slots");
    assert_eq!(
        cells.cells.expect("the preview row is on screen").len(),
        4,
        "the row is not four cells, so there is no cell without a slot to test"
    );

    // The fourth cell is drawn, and `cell` still answers for it — what has
    // changed is what a *release* there means.
    let onto = cell_at(&cells, 3);
    assert_eq!(cells.cell(onto), Some(3), "the fourth cell is not drawn");
    assert!(cells.owns(onto), "the fourth cell is not claimed any more");
    assert_eq!(strips.dropped(onto), None);
    assert_eq!(
        cells.dropped(onto, view.mixer.len()),
        None,
        "a drop on deck D's cell named a deck this console has no slot for"
    );

    let at = row(&bay, 0);
    let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
    panel.carry(at, taken.set, taken.procedure);
    assert_eq!(
        panel.released(cells.dropped(onto, view.mixer.len()).map(Landing::Deck)),
        Some(Released::Nowhere {
            set: view.library[0].clone(),
        }),
        "a drop on a cell with no slot behind its letter asked for a load"
    );

    // And the three that do name a slot are unaffected.
    for deck in 0..3u8 {
        let onto = cell_at(&cells, deck);
        assert_eq!(
            cells.dropped(onto, view.mixer.len()),
            Some(deck),
            "cell {deck} stopped naming its deck"
        );
    }
}

// ---------------------------------------------------------------------------
// What the carry is drawn as
// ---------------------------------------------------------------------------

/// Collects painted drop indicator rectangles styled with `--c-text` outlines.
fn drop_marks(view: &mut View, panel: &mut Panel) -> Vec<egui::Rect> {
    let ink = view.room.palette().text;
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            egui::Shape::Rect(rect)
                if rect.stroke.color == ink
                    && rect.stroke.width == size::DROP_RING
                    && rect.stroke_kind == egui::StrokeKind::Outside =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect()
}

/// The cursor the console asked for on one frame.
fn cursor(view: &mut View, panel: &mut Panel) -> egui::CursorIcon {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.platform_output.cursor_icon
}

/// At most one drop target rectangle is highlighted, matching the hovered strip or cell.
#[test]
fn one_rectangle_is_marked_and_it_is_the_one_the_release_names() {
    let (mut view, mut panel, ctx) = with_cells(&strips());
    let bay = bay(&panel, &view);
    let cells = program(&panel, &view);
    // Target rectangles across both bays pre-sampled before View::draw mutable borrows.
    let targets: Vec<Point> = {
        let strips = strips_bay(&panel, &ctx, &view);
        (0..4u8)
            .flat_map(|deck| [strip_at(&strips, deck), cell_at(&cells, deck)])
            .collect()
    };

    // Nothing in hand: the pointer is over strip A and nothing is ringed.
    panel.moved(targets[0]);
    assert_eq!(
        drop_marks(&mut view, &mut panel).len(),
        0,
        "a rectangle is ringed with nothing in hand — the mark is a hover"
    );

    let mut asked = 0;
    for onto in targets {
        let at = row(&bay, 0);
        let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
        panel.carry(at, taken.set, taken.procedure);
        panel.moved(onto);
        let marks = drop_marks(&mut view, &mut panel);
        assert_eq!(
            marks.len(),
            1,
            "{} rectangles are ringed with the pointer at {onto:?}",
            marks.len()
        );
        assert!(
            marks[0].contains(egui::Pos2::new(onto.x, onto.y)),
            "the ring at {:?} is not round the rectangle under the pointer at {onto:?}",
            marks[0]
        );
        panel.released(None);
        asked += 1;
    }
    assert_eq!(asked, 8, "not every rectangle was asked");
}

/// Hovering over gaps, bay heads, or non-target regions produces no drop highlight.
#[test]
fn nothing_is_marked_where_a_release_would_load_nothing() {
    let three: Vec<Strip> = strips().into_iter().take(3).collect();
    let (mut view, mut panel, ctx) = with_cells(&three);
    let bay = bay(&panel, &view);
    let cells = program(&panel, &view);

    // The alley: half way between strip A's right edge and strip B's left.
    // Taken before anything is drawn, for the reason above.
    let alley = {
        let strips = strips_bay(&panel, &ctx, &view);
        let a = strips.selected(0).expect("a strip for deck A");
        let b = strips.selected(1).expect("a strip for deck B");
        let alley = Point::new((a.max.x + b.min.x) * 0.5, a.center().y);
        assert_eq!(strips.dropped(alley), None, "the alley is inside a strip");
        alley
    };
    let mixer_region = rect_of(panel.layout(), "mixer");
    let transport = rect_of(panel.layout(), "transport");

    let mut asked = 0;
    for over in [
        alley,
        // The bay's head, above the strips.
        Point::new(mixer_region.x + mixer_region.w * 0.5, mixer_region.y + 4.0),
        Point::new(transport.x + transport.w * 0.5, transport.y + 4.0),
        row(&bay, 0),
        Point::new(-40.0, -40.0),
        cell_at(&cells, 3),
    ] {
        let at = row(&bay, 0);
        let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
        panel.carry(at, taken.set, taken.procedure);
        panel.moved(over);
        let marks = drop_marks(&mut view, &mut panel);
        assert_eq!(
            marks.len(),
            0,
            "{:?} is ringed with the pointer at {over:?}, where a release loads nothing",
            marks
        );
        panel.released(None);
        asked += 1;
    }
    assert_eq!(asked, 6, "not every point was asked");
}

/// Pointer retains a grab cursor during carry, suppressing boundary resize cursors.
#[test]
fn the_pointer_is_a_grab_while_a_set_is_in_hand() {
    let (mut view, mut panel, ctx) = with_cells(&strips());
    let bay = bay(&panel, &view);
    let on_strip = {
        let strips = strips_bay(&panel, &ctx, &view);
        strip_at(&strips, 1)
    };

    // A vertical boundary, found the way the panel finds one: the left edge of
    // the mixer's own region is a divider between two panes.
    let mixer_region = rect_of(panel.layout(), "mixer");
    let on_boundary = Point::new(mixer_region.x, mixer_region.y + mixer_region.h * 0.5);
    panel.moved(on_boundary);
    let resize = cursor(&mut view, &mut panel);
    assert!(
        matches!(
            resize,
            egui::CursorIcon::ResizeHorizontal | egui::CursorIcon::ResizeVertical
        ),
        "the point picked is not on a boundary at all ({resize:?}), so the assertion below \
         would pass against nothing"
    );

    let at = row(&bay, 2);
    let taken = bay.take(listed(&view.library), at).expect("row 2 is drawn");
    panel.carry(at, taken.set, taken.procedure);

    let mut asked = 0;
    for over in [
        on_boundary,
        on_strip,
        row(&bay, 0),
        Point::new(-40.0, -40.0),
    ] {
        panel.moved(over);
        assert_eq!(
            cursor(&mut view, &mut panel),
            egui::CursorIcon::Grabbing,
            "the pointer is not a grab at {over:?} with a Set in hand"
        );
        asked += 1;
    }
    assert_eq!(asked, 4, "not every point was asked");

    // And it is gone with the gesture: the boundary is a resize again.
    panel.moved(on_boundary);
    panel.released(None);
    assert_eq!(
        cursor(&mut view, &mut panel),
        resize,
        "the grab outlived the carry"
    );
}
