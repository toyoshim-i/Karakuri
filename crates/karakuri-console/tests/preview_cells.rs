//! Deck preview cell hit-testing and boundary clearances across layouts (ADR-0240, ADR-0182).

mod common;

use common::{arranged, drawn_once, near, point, rect_of, showing, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{program_bay, Placement, DECKS, DECK_LETTERS, MOCK_CANVAS};
use karakuri_layout::Hit;

fn console(viewport: karakuri_layout::Rect) -> (Panel, egui::Context) {
    (arranged(viewport, MOCK_CANVAS), drawn_once())
}

/// The four cells at `viewport`, and which way round the bay arranged itself.
fn cells(panel: &Panel) -> (Placement, [egui::Rect; DECKS]) {
    let bay = program_bay(panel.layout(), MOCK_CANVAS).expect("the Program bay is laid out");
    (bay.placement, bay.cells.expect("the bay draws its cells"))
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// Which cell a point is on is the cell at its own rectangle, and never the
/// deck the pointer happens to have selected: a cell is the one thing on this
/// console that carries a letter, so what it answers for is its own.
#[test]
fn a_cell_is_the_cell_at_its_own_rectangle() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, _) = console(viewport);
        let bay = program_bay(panel.layout(), MOCK_CANVAS).expect("the bay");
        let (_, cells) = cells(&panel);
        for (deck, cell) in cells.into_iter().enumerate() {
            assert_eq!(
                bay.cell(point(cell.center())),
                Some(deck as u8),
                "the cell lettered {} is not the cell at its own rectangle",
                DECK_LETTERS[deck]
            );
        }
    }
}

/// A press between two cells asks for nothing and is not claimed — a control
/// claims what it acts on and no more, and the ground between the cells is the
/// bay's card showing through.
#[test]
fn a_press_between_two_cells_asks_for_nothing_and_is_not_claimed() {
    let (mut panel, ctx) = console(SMALLEST);
    let bay = program_bay(panel.layout(), MOCK_CANVAS).expect("the bay");
    let (placement, cells) = cells(&panel);
    assert_eq!(
        placement,
        Placement::Below,
        "this test wants the row arrangement, where the gap between two cells is a gap"
    );
    let gap = egui::pos2((cells[0].max.x + cells[1].min.x) * 0.5, cells[0].center().y);
    assert!(
        !bay.owns(point(gap)),
        "the cells claim the {}px gap between two of them",
        size::PREVIEW_GAP
    );
    assert_eq!(
        bay.cell(point(gap)),
        None,
        "the gap between two cells answers as one of them"
    );
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), point(gap)),
        Claim::Egui,
        "the ground between two cells is on no control and on no boundary"
    );
}

// ---------------------------------------------------------------------------
// Both arrangements
// ---------------------------------------------------------------------------

/// Cells remain targetable across horizontal and side-column layout orientations (ADR-0182).
#[test]
fn every_cell_is_a_control_in_both_arrangements() {
    let mut seen = Vec::new();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (mut panel, ctx) = console(viewport);
        let (placement, cells) = cells(&panel);
        seen.push(placement);
        for (deck, cell) in cells.into_iter().enumerate() {
            assert_eq!(
                claim(&mut panel, &ctx, &showing(&[]), point(cell.center())),
                Claim::Panel,
                "cell {} is not the panel's at {viewport:?}",
                DECK_LETTERS[deck]
            );
        }
    }
    assert!(
        seen.contains(&Placement::Below) && seen.contains(&Placement::Beside),
        "both viewports arranged the bay the same way ({seen:?}), so this test only \
         demonstrated one of the two"
    );
}

/// A folded preview row has no cell to press, and the picture beside it is
/// untouched — the two regions fold apart, which is what the manual promises
/// and what the arrangement is a split for.
#[test]
fn a_folded_preview_row_has_no_cell_to_press() {
    let (mut panel, ctx) = console(SMALLEST);
    let (_, cells) = cells(&panel);
    let where_it_was = cells[0].center();

    let row = panel
        .layout()
        .find("deck-previews")
        .expect("the arrangement names the row");
    panel.op(Op::Fold(row));
    karakuri_console::view::rearrange(&mut panel, MOCK_CANVAS);
    let bay = program_bay(panel.layout(), MOCK_CANVAS).expect("the bay keeps its picture");
    assert!(
        bay.cells.is_none(),
        "a folded preview row still laid its cells out"
    );
    assert!(!bay.owns(point(where_it_was)));
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), point(where_it_was)),
        Claim::Egui,
        "where a cell used to be is still claimed with the row folded away"
    );

    panel.op(Op::Unfold(row));
    karakuri_console::view::rearrange(&mut panel, MOCK_CANVAS);
    assert!(
        program_bay(panel.layout(), MOCK_CANVAS)
            .expect("the bay")
            .owns(point(where_it_was)),
        "unfolding the row left the cell dead"
    );
}

// ---------------------------------------------------------------------------
// The boundary gets first refusal
// ---------------------------------------------------------------------------

/// In the horizontal layout, a cell's top edge yields 6px to boundary `GRAB`,
/// while the vertical layout clears all boundary grabs completely.
#[test]
fn what_a_boundary_keeps_of_a_cell_is_the_arrangement_it_is_in() {
    // --- under the picture, where there is a boundary over the row ---------
    let (mut panel, ctx) = console(SMALLEST);
    let (placement, cells) = cells(&panel);
    assert_eq!(placement, Placement::Below);
    let region = rect_of(panel.layout(), "deck-previews");
    for (deck, cell) in cells.into_iter().enumerate() {
        assert!(
            near(cell.min.y, region.y),
            "cell {}'s top is at {} and the row's region starts at {} — the padding is under \
             the cells, so the two are the same edge",
            DECK_LETTERS[deck],
            cell.min.y,
            region.y
        );
        assert!(
            matches!(
                panel.layout().hit(point(cell.center_top()), GRAB),
                Hit::Divider { .. }
            ),
            "the top of cell {} is not inside the boundary's band, so this test's arithmetic \
             is about something that is not happening",
            DECK_LETTERS[deck]
        );
        // Everything past the grab is the control's, which is the press an
        // operator actually makes.
        let below = egui::pos2(cell.center().x, cell.min.y + GRAB + 0.5);
        for probe in [below, cell.center(), cell.center_bottom()] {
            assert!(
                !matches!(panel.layout().hit(point(probe), GRAB), Hit::Divider { .. }),
                "a boundary grabs {probe:?}, which is past the sliver it is owed"
            );
            assert_eq!(
                claim(&mut panel, &ctx, &showing(&[]), point(probe)),
                Claim::Panel,
                "the panel does not get a press at {probe:?}, which is on its own control"
            );
        }
        assert!(
            cell.height() > GRAB * 2.0,
            "cell {} is {} tall and a boundary keeps {GRAB} of it — the control is most of \
             the way to dead and `input`'s rule is what has to change",
            DECK_LETTERS[deck],
            cell.height()
        );
    }

    // --- beside it, where there is none -----------------------------------
    let (mut panel, ctx) = console(PLAUSIBLE);
    let (placement, cells) = self::cells(&panel);
    assert_eq!(placement, Placement::Beside);
    for (deck, cell) in cells.into_iter().enumerate() {
        for probe in [
            cell.left_top(),
            cell.right_top(),
            cell.left_bottom(),
            cell.right_bottom(),
            cell.center(),
        ] {
            assert!(
                !matches!(panel.layout().hit(point(probe), GRAB), Hit::Divider { .. }),
                "a boundary grabs {probe:?} on cell {} — beside the picture the cells sit \
                 inside {} of the bay's padding and nothing should reach them",
                DECK_LETTERS[deck],
                size::PROGRAM_BODY_PAD
            );
            assert_eq!(
                claim(&mut panel, &ctx, &showing(&[]), point(probe)),
                Claim::Panel,
                "the panel does not get a press at {probe:?}, which is on its own control"
            );
        }
    }
}
