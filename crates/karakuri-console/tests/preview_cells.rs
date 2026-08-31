//! **The four deck preview cells: the console's route into *Choose what the
//! output shows*.**
//!
//! `docs/manual/operations.html` says the row is *"the mix, or one deck
//! auditioned"* and names `preview` in the panel column; the cells are the
//! only thing on this console called that and the only thing naming a deck —
//! *"The A–D under it keep their letters, which are outside anything you would
//! capture and are the only thing naming a deck."*
//!
//! # What this file is for
//!
//! - **A press on a cell names that cell's deck**, and a press on the cell the
//!   output is already showing names the mix — which is how one row of four
//!   cells reaches all five values `Operation::SetPreview` can carry. The
//!   manual does not say which gesture asks for the mix and
//!   `ProgramBay::preview` is where that decision and its argument are.
//! - **The cells are the ones the bay drew**, in both of its arrangements, so
//!   a press lands on the cell an operator is looking at whichever way round
//!   the Program bay put itself (ADR-0182).
//! - **A row that is not laid out has no cell**, which is `mask.rs`'s sentence
//!   in another bay.
//! - **What a boundary keeps of a cell**, which is the whole of
//!   [`what_a_boundary_keeps_of_a_cell_is_the_arrangement_it_is_in`] and is
//!   the second half of the finding `solo_pill.rs` states for the pill.

mod common;

use common::{arranged, drawn_once, near, rect_of, showing, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{program_bay, Placement, DECKS, DECK_LETTERS, MOCK_CANVAS};
use karakuri_layout::{Hit, Point};
use karakuri_operation::Operation;

fn console(viewport: karakuri_layout::Rect) -> (Panel, egui::Context) {
    (arranged(viewport, MOCK_CANVAS), drawn_once())
}

fn point(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// The four cells at `viewport`, and which way round the bay arranged itself.
fn cells(panel: &Panel) -> (Placement, [egui::Rect; DECKS]) {
    let bay = program_bay(panel.layout(), MOCK_CANVAS).expect("the Program bay is laid out");
    (bay.placement, bay.cells.expect("the bay draws its cells"))
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// **A press on a cell names that cell's deck**, and never the deck the
/// pointer happens to have selected: a cell is the one thing on this console
/// that carries a letter, so what it can say is its own.
#[test]
fn a_press_on_a_cell_names_that_cells_deck() {
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
            // With the mix showing, every cell names its own deck.
            assert_eq!(
                bay.preview(None, point(cell.center())),
                Some(Operation::SetPreview {
                    showing: Some(deck as u8)
                }),
                "a press on cell {} did not ask for that deck",
                DECK_LETTERS[deck]
            );
            // And with some *other* deck on the output it still does, which
            // is what makes the mix the second press rather than a mode.
            let other = ((deck + 1) % DECKS) as u8;
            assert_eq!(
                bay.preview(Some(other), point(cell.center())),
                Some(Operation::SetPreview {
                    showing: Some(deck as u8)
                })
            );
        }
    }
}

/// **A press on the cell the output is already showing asks for the mix**,
/// which is the other half of what the row can say and the only gesture on
/// this panel that reaches it.
///
/// `Deck::set_preview` is where the argument is — *"The way out of an audition
/// is to end it"* — and the operation says the value rather than the direction:
/// `showing: None` is the mix, named outright, and *the one that is showing*
/// is this surface's translation and not the operation's (P-0074).
#[test]
fn a_press_on_the_cell_the_output_is_showing_asks_for_the_mix() {
    let (panel, _) = console(SMALLEST);
    let bay = program_bay(panel.layout(), MOCK_CANVAS).expect("the bay");
    let (_, cells) = cells(&panel);
    for (deck, cell) in cells.into_iter().enumerate() {
        assert_eq!(
            bay.preview(Some(deck as u8), point(cell.center())),
            Some(Operation::SetPreview { showing: None }),
            "a second press on cell {} did not end the audition",
            DECK_LETTERS[deck]
        );
    }
}

/// **A press between two cells asks for nothing and is not claimed** — a
/// control claims what it acts on and no more, and the ground between the
/// cells is the bay's card showing through.
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
    assert_eq!(bay.preview(None, point(gap)), None);
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), point(gap)),
        Claim::Egui,
        "the ground between two cells is on no control and on no boundary"
    );
}

// ---------------------------------------------------------------------------
// Both arrangements
// ---------------------------------------------------------------------------

/// **Every cell is a control in both of the bay's arrangements**, which is
/// what makes the press land on the cell an operator is looking at: the row
/// under the picture at a narrow window, and two columns down the sides at a
/// wide one (ADR-0182). A control derived from the `deck-previews` region
/// alone would answer for one of the two only, because beside the picture that
/// region is set aside and has no extent at all.
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

/// **A folded preview row has no cell to press**, and the picture beside it is
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

/// **What a boundary keeps of a cell is decided by which arrangement the bay
/// is in**, and one of the two costs a cell six pixels of its top edge.
///
/// Under the picture, the row of cells **is** the `deck-previews` region less
/// its padding, and the padding is under the cells rather than over them — the
/// arrangement writes the region as *"63 + 9"*, the row and the padding under
/// it. So a cell's top edge is the region's own, and the boundary between the
/// picture and the row grabs [`GRAB`] = 6 past it.
///
/// Beside the picture there is no such boundary at all: the row is set aside,
/// the cells sit in the bay's body inside `PROGRAM_BODY_PAD` = 9 of padding,
/// and 9 beats 6 on every side.
///
/// **So this asserts the overlap where there is one and its absence where
/// there is not**, in both directions — the top edge of a cell in the row
/// arrangement is the boundary's and everything below the sliver is the
/// control's. `input`'s rule 3 decides it the same way every time, so there is
/// no case where two things think they are dragging; what is lost is the
/// sliver, out of a cell that is [`size::PREVIEW_ROW_H`] tall.
///
/// It fails if the sliver grows, which is what it is for: a taller `GRAB`, a
/// shorter row or padding moved above the cells each make more of the control
/// dead, and the fix then is a change to `docs/manual/console.html` or to the
/// rule rather than a nudge.
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
