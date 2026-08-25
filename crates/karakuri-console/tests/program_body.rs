//! **How the Program bay's body arranges itself in the rectangle it has**, and
//! the arithmetic checked against the mock rather than against the code that
//! produced it.
//!
//! None of this needs a window or a device, and none of it needs a solved
//! layout either: `view::program_body` takes a rectangle and answers where the
//! picture and the four cells go, so every case here is two numbers in and a
//! handful out. The one test that does read a layout is the last one, and it is
//! there to hold the new arithmetic against what the console draws today.
//!
//! # Why the expected numbers are written out rather than derived
//!
//! Every rectangle below is a literal arrived at by hand from the mock's own
//! figures, and the derivation is written above it. A test that recomputed the
//! answer from the same constants the code uses would pass against a wrong
//! constant, which is exactly the failure
//! [ADR-0179](../../../docs/adr/0179-a-transcribed-number-cites-the-rule-it-was-copied-from.md)
//! records; [`largest`] is the one piece of shared arithmetic and it is
//! deliberately a second implementation of the fit rather than a call to the
//! first.

mod common;

use common::{near, rect_of, solved, PLAUSIBLE, SMALLEST};
use egui::{Pos2, Rect};
use karakuri_console::room::size;
use karakuri_console::view::{picture_rect, preview_rects, program_body, Placement, DECKS};

/// **The canvas the picture is fitted to**, and it is the workspace's
/// reference workload — 1280x720, which `examples/panel.rs` names `CANVAS` and
/// builds its `Present` at.
const CANVAS: (u32, u32) = (1280, 720);

/// **A canvas that is not the mock's shape**, so a picture fitted to a
/// hard-coded 16:9 and one fitted to *the canvas* can be told apart, and so
/// can a cell that followed the canvas when it should not.
const SQUARISH: (u32, u32) = (1024, 768);

/// **The Program bay's body at the narrowest console the mock will draw.**
///
/// `.console`'s `min-width: 1010px` less its 10px of padding either side is
/// 990; the centre track is 990 - 218 - 268 - two 10px gaps = 484; the bay is
/// that wide, and `.program-body`'s 9px padding leaves **466**. The bay is 378
/// tall, less the 27 of bay head painted over it and 9 of that padding top and
/// bottom, which is **333**.
const NARROWEST: (f32, f32) = (466.0, 333.0);

/// **The same body in a 1920 window.** The two side tracks and the four
/// dividers do not move, so the centre track takes the whole of the extra
/// width: 1920 - 218 - 268 - 20 = 1414, less the same 18 of padding = **1396**.
/// The bay is `Sizing::Fixed` along its column, so the height is still 333.
const WIDE: (f32, f32) = (1396.0, 333.0);

/// A body of that size, **and not at the origin**: every rectangle this answers
/// with is inside the bay somebody solved, so an arrangement that had quietly
/// assumed a zero origin would tile a panel that has none.
fn body(size: (f32, f32)) -> Rect {
    Rect::from_min_size(Pos2::new(37.0, 61.0), egui::vec2(size.0, size.1))
}

/// The size of the largest `aspect` box that fits in `w` x `h`, in whole
/// pixels — `view::fitted`'s rule, **restated rather than reached for**. The
/// decider is the thing under test here, and a test that asked the code for
/// both of the sizes it is choosing between would be comparing the code with
/// itself.
fn largest(w: f32, h: f32, aspect: (f32, f32)) -> Option<(f32, f32)> {
    let scale = (w / aspect.0).min(h / aspect.1);
    let box_w = (aspect.0 * scale).round().min(w.floor());
    let box_h = (aspect.1 * scale).round().min(h.floor());
    match box_w > 0.0 && box_h > 0.0 {
        true => Some((box_w, box_h)),
        false => None,
    }
}

/// The two pictures the decider is choosing between, `None` for one that
/// cannot be drawn at all.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Hand {
    below: Option<(f32, f32)>,
    beside: Option<(f32, f32)>,
}

/// What each arrangement's picture would be, by hand: below is the body less
/// the row of cells and the divider above it, beside is the body less two
/// columns and the divider either side of the picture.
fn by_hand(size: (f32, f32), canvas: (f32, f32)) -> Hand {
    let (w, h) = size;
    let column = (h - 6.0) / 2.0 * 16.0 / 9.0;
    Hand {
        below: largest(w, h - 8.0 - 63.0, canvas),
        beside: largest(w - column * 2.0 - 16.0, h, canvas),
    }
}

fn size_of(rect: Rect) -> (f32, f32) {
    (rect.width(), rect.height())
}

fn assert_rect(rect: Rect, at: (f32, f32), size: (f32, f32), what: &str) {
    let want = Rect::from_min_size(
        Pos2::new(37.0 + at.0, 61.0 + at.1),
        egui::vec2(size.0, size.1),
    );
    assert!(
        near(rect.min.x, want.min.x)
            && near(rect.min.y, want.min.y)
            && near(rect.width(), want.width())
            && near(rect.height(), want.height()),
        "{what} is {rect:?} and should be {want:?}"
    );
}

// ---------------------------------------------------------------------------
// The three worked cases
// ---------------------------------------------------------------------------

/// **At the mock's own body the arrangement is the mock's**, and every number
/// in it is the mock's too.
///
/// The picture is 466 x 262 — the box `.program-view` occupies at that width,
/// which is `aspect-ratio: 16/9` on 466 rounded to the whole pixel the mock
/// rasterises it at. The row under it is 63 tall, and its four tracks are
/// (466 - three 6px gaps) / 4 = 112 wide, which at 16:9 is 63 exactly: a cell
/// fills its track here and nowhere wider.
///
/// **Beside is not merely worse here, it cannot be drawn at all**: a column is
/// 290.67 wide, and two of them with the two dividers come to 597.33 in a body
/// 466 wide. So this case also holds the `None` branch of the other
/// arrangement.
#[test]
fn the_mocks_narrowest_body_is_the_mocks_own_arrangement() {
    let arranged = program_body(body(NARROWEST), CANVAS).expect("the body has room for a picture");

    assert_eq!(arranged.placement, Placement::Below);
    assert_rect(arranged.picture, (0.0, 0.0), (466.0, 262.0), "the picture");
    for (deck, cell) in arranged.cells.iter().enumerate() {
        assert_rect(
            *cell,
            (118.0 * deck as f32, 333.0 - 63.0),
            (112.0, 63.0),
            &format!("cell {deck}"),
        );
    }

    // And the other arrangement really is unavailable rather than merely
    // smaller, which is what the `None` in the comparison covers.
    let beside = by_hand(NARROWEST, (1280.0, 720.0)).beside;
    assert_eq!(beside, None, "beside is drawable at the mock's own body");
}

/// **At a 1920 window the cells go down the sides, and the picture is 61%
/// larger for it.**
///
/// Below gives 466 x 262 whatever the width is, because the height is what
/// limits it: 122,092 texels, with 930 pixels of the bay's width empty either
/// side. Beside gives the picture the whole 333 of the body's height —
/// 333 x 16/9 = **592 x 333**, 197,136 texels — and puts the four cells in the
/// ground the picture is not using.
///
/// A column is (333 - 6) / 2 = 163.5 tall at 16:9, so **290.67 wide**, and a
/// cell is that track in whole pixels: 290 x 163, centred in it. The picture's
/// box is what is left between them, 1396 - 581.33 - 16 = 798.67, and the
/// picture is centred in that.
#[test]
fn a_nineteen_twenty_window_puts_the_cells_down_the_sides() {
    let arranged = program_body(body(WIDE), CANVAS).expect("the body has room for a picture");

    assert_eq!(arranged.placement, Placement::Beside);
    // 298.667 of column and divider, plus (798.667 - 592) / 2 = 103.333 of
    // centring.
    assert_rect(
        arranged.picture,
        (402.0, 0.0),
        (592.0, 333.0),
        "the picture",
    );

    // **A and B down the left, C and D down the right**, each column top to
    // bottom, with one 6px gap between the two of them: the tracks are
    // [0, 163.5] and [169.5, 333], and a 290 x 163 cell sits centred in each.
    let left = 0.333_333;
    let right = 1396.0 - 290.666_66 + left;
    let lower = 169.5 + 0.25;
    assert_rect(arranged.cells[0], (left, 0.25), (290.0, 163.0), "cell A");
    assert_rect(arranged.cells[1], (left, lower), (290.0, 163.0), "cell B");
    assert_rect(arranged.cells[2], (right, 0.25), (290.0, 163.0), "cell C");
    assert_rect(arranged.cells[3], (right, lower), (290.0, 163.0), "cell D");

    // The claim in the sentence above, as a number: 197,136 against 122,092.
    let below = program_body(body(NARROWEST), CANVAS)
        .expect("on screen")
        .picture;
    let gain =
        (arranged.picture.width() * arranged.picture.height()) / (below.width() * below.height());
    assert!(
        near(gain, 1.614_6),
        "beside gives a picture {gain} times the size of below's, and the claim is 1.61"
    );
}

/// **Eight hundred wide still goes below**, which is the half of the decider
/// that says there is no silly flip as soon as the window leaves the mock.
///
/// Beside is drawable here and is much worse: the two columns still cost
/// 597.33, so the picture's box is 202.67 wide and the picture in it is
/// 202 x 114 — a sixth of what below gives.
#[test]
fn an_eight_hundred_wide_body_keeps_the_mocks_arrangement() {
    let arranged = program_body(body((800.0, 333.0)), CANVAS).expect("on screen");

    assert_eq!(arranged.placement, Placement::Below);
    // Centred in the 800 the body has: (800 - 466) / 2 = 167.
    assert_rect(
        arranged.picture,
        (167.0, 0.0),
        (466.0, 262.0),
        "the picture",
    );

    // And the loser is drawable rather than absent, so this is the comparison
    // choosing and not the other branch defaulting.
    let beside = by_hand((800.0, 333.0), (1280.0, 720.0)).beside;
    assert_eq!(beside, Some((202.0, 114.0)));
}

// ---------------------------------------------------------------------------
// The decider
// ---------------------------------------------------------------------------

/// **The larger picture wins at every width, and a tie goes to the mock's.**
///
/// The property rather than a consequence of it: at every body width from the
/// mock's narrowest to a 3440 window, both arrangements are worked out here by
/// hand and the one this picked has to be the larger of the two — with
/// [`Placement::Below`] on a tie and wherever the other cannot be drawn.
///
/// This is the assertion that catches the decider inverted, and it catches it
/// at 1,935 widths rather than at one.
#[test]
fn the_larger_picture_wins_at_every_width_and_a_tie_goes_below() {
    let mut below_won = 0;
    let mut beside_won = 0;

    for w in 466..=2400 {
        let size = (w as f32, 333.0);
        let arranged = program_body(body(size), CANVAS).expect("on screen");
        let Hand { below, beside } = by_hand(size, (1280.0, 720.0));
        let below = below.expect("below is drawable at every width from 466 up");

        let want = match beside {
            Some(beside) if beside.0 * beside.1 > below.0 * below.1 => (Placement::Beside, beside),
            _ => (Placement::Below, below),
        };
        assert_eq!(
            arranged.placement, want.0,
            "at {w} wide the arrangement is {:?} where below is {below:?} and beside is {beside:?}",
            arranged.placement
        );
        let (want_w, want_h) = want.1;
        assert!(
            near(arranged.picture.width(), want_w) && near(arranged.picture.height(), want_h),
            "at {w} wide the picture is {:?} and the winner works out at {:?}",
            size_of(arranged.picture),
            want.1
        );

        match arranged.placement {
            Placement::Below => below_won += 1,
            Placement::Beside => beside_won += 1,
        }
    }

    // Both branches were actually taken, so the loop is not asserting one
    // answer 1,935 times.
    assert!(
        below_won > 100 && beside_won > 100,
        "{below_won} and {beside_won}"
    );
}

/// **The two curves cross once**, so an operator dragging a window wider passes
/// through one change of arrangement and never through a flicker.
///
/// Below's picture stops growing with the width at 466 x 262 — the height is
/// what limits it from there on — and beside's never shrinks as the body
/// widens, so there is exactly one crossover by construction. The number is
/// **1064**: at 1063 the picture's box beside is 465.67 and the picture in it
/// is 465 x 262, which is 121,830 against below's 122,092; one pixel wider it
/// is 466 x 263, which is 122,558.
///
/// A body 1064 wide is a **1588-wide window** — 1064 + 18 of padding + the two
/// fixed tracks and the two gaps — which is inside an ordinary desktop and well
/// clear of both ends of it. That is the answer to *is the crossover anywhere
/// an operator lives*, and it is why a stored mode is not needed to keep the
/// arrangement still.
#[test]
fn there_is_exactly_one_crossover_and_it_is_at_1064() {
    let placement = |w: i32| {
        program_body(body((w as f32, 333.0)), CANVAS)
            .expect("on screen")
            .placement
    };

    let mut changes = Vec::new();
    for w in 467..=3440 {
        if placement(w) != placement(w - 1) {
            changes.push(w);
        }
    }
    assert_eq!(
        changes,
        vec![1064],
        "the arrangement changes at {changes:?}"
    );
    assert_eq!(placement(1063), Placement::Below);
    assert_eq!(placement(1064), Placement::Beside);
}

/// **A body with nothing in it has no arrangement**, either way round.
///
/// `picture_rect`'s rule, stated over the pair: a rectangle with no extent, or
/// one the insets turned inside out, is nothing to draw and nothing to render
/// into — not an arrangement with a degenerate picture in it.
#[test]
fn a_body_with_no_room_has_no_arrangement() {
    assert_eq!(program_body(body((466.0, 0.0)), CANVAS), None);
    assert_eq!(program_body(body((0.0, 333.0)), CANVAS), None);
    assert_eq!(program_body(body((-100.0, -100.0)), CANVAS), None);
}

// ---------------------------------------------------------------------------
// The column, the cells and the gaps
// ---------------------------------------------------------------------------

/// **A side column is the body's height and never its width.**
///
/// The rule as an experiment: hold the height and widen the body, and the four
/// cells must not move or change size at all — every pixel of the extra width
/// belongs to the picture. A column read off the width would grow with the
/// thing it is competing against, which is the defect this catches, and it
/// would catch it in the direction that matters: the picture would stop gaining
/// and beside would eventually stop winning.
#[test]
fn the_columns_are_the_bodys_height_and_never_its_width() {
    let at = |w: f32| program_body(body((w, 333.0)), CANVAS).expect("on screen");
    let reference = at(1396.0);
    assert_eq!(reference.placement, Placement::Beside);

    for w in [1100.0, 1200.0, 1396.0, 1920.0, 3440.0] {
        let arranged = at(w);
        assert_eq!(arranged.placement, Placement::Beside, "at {w} wide");
        for deck in 0..DECKS {
            // The left column's cells are in the same place at every width;
            // the right column's are the same size and against the far edge.
            assert!(
                near(arranged.cells[deck].width(), reference.cells[deck].width())
                    && near(
                        arranged.cells[deck].height(),
                        reference.cells[deck].height()
                    ),
                "at {w} wide cell {deck} is {:?} and at 1396 it is {:?}",
                arranged.cells[deck].size(),
                reference.cells[deck].size()
            );
        }
        assert!(
            near(arranged.cells[0].min.x, reference.cells[0].min.x)
                && near(arranged.cells[0].min.y, reference.cells[0].min.y),
            "at {w} wide the left column moved"
        );
    }

    // And the height is what does move them: a taller body gives a taller cell.
    let taller = program_body(body((1396.0, 380.0)), CANVAS).expect("on screen");
    assert_eq!(taller.placement, Placement::Beside);
    assert!(
        taller.cells[0].height() > reference.cells[0].height() + 20.0,
        "a body 47 taller gave a cell {} tall against {}",
        taller.cells[0].height(),
        reference.cells[0].height()
    );
}

/// **A bay dragged tall goes back to the mock's arrangement, and that is the
/// column rule's cost written as a test.**
///
/// The column follows the height, so a taller body makes both columns wider
/// while the picture's own box is what has to pay for them: at 1396 wide the
/// picture beside stops gaining at a body 391 tall and shrinks from there,
/// while below's picture keeps growing with every pixel of height. They cross
/// at **427**, which is a Program bay 472 tall — an operator can drag to it,
/// and what they see there is the four cells going back under the picture.
///
/// **It is one flip and not a flicker**, which is the same claim the width
/// sweep makes and is the reason it is checked rather than argued: beside's
/// picture is a single-peaked function of the height and below's is monotone,
/// so the two cross once. A rule whose column was capped somewhere would cross
/// twice and the arrangement would flip, settle and flip back inside one drag.
#[test]
fn a_bay_dragged_tall_goes_back_to_the_mocks_arrangement() {
    let placement = |h: i32| {
        program_body(body((1396.0, h as f32)), CANVAS)
            .expect("on screen")
            .placement
    };

    let mut changes = Vec::new();
    for h in 73..=900 {
        if placement(h) != placement(h - 1) {
            changes.push(h);
        }
    }
    assert_eq!(changes, vec![427], "the arrangement changes at {changes:?}");
    assert_eq!(placement(333), Placement::Beside);
    assert_eq!(placement(426), Placement::Beside);
    assert_eq!(placement(427), Placement::Below);
}

/// **A cell is the mock's shape and the picture is the canvas's**, in both
/// arrangements.
///
/// The seam ADR-0181 recorded and left standing: `PREVIEW_ASPECT` is 16:9 off
/// `.preview` and the picture's aspect is whatever the Set is rendering at, so
/// a 4:3 canvas changes the picture and must change nothing about a cell. It is
/// the column's width that would give it away — the column is sized from a
/// cell, so a column that had taken the canvas would move all four of them.
#[test]
fn a_cell_is_the_mocks_shape_and_never_the_canvass() {
    for size in [NARROWEST, WIDE, (1100.0, 333.0)] {
        let mock = program_body(body(size), CANVAS).expect("on screen");
        let squarish = program_body(body(size), SQUARISH).expect("on screen");

        assert_eq!(
            mock.placement, squarish.placement,
            "a 4:3 canvas changed which arrangement won at {size:?}"
        );
        assert_eq!(
            mock.cells, squarish.cells,
            "a 4:3 canvas moved the cells at {size:?}"
        );
        for cell in mock.cells {
            assert!(
                (cell.width() / cell.height() - 16.0 / 9.0).abs() < 0.01,
                "a cell is {}x{} and `.preview` is 16:9",
                cell.width(),
                cell.height()
            );
        }
        assert!(
            (squarish.picture.width() / squarish.picture.height() - 4.0 / 3.0).abs() < 0.01,
            "the picture is {}x{} and the canvas is 4:3",
            squarish.picture.width(),
            squarish.picture.height()
        );
    }
}

/// **The two gaps beside the picture are the mock's own**, and this is the case
/// that can see them.
///
/// At 1396 the picture is limited by the height and floats in a box 200 pixels
/// wider than it is, so nothing there measures a gap. At **1100** the picture is
/// limited by the width instead, and every pixel of both gaps comes off it:
/// a column is 290.67, so the box is 1100 - 581.33 - 16 = 502.67 and the
/// picture is **502 x 283**, sat 299 from the body's left edge.
///
/// Six instead of eight between the picture and a column gives 506 x 285;
/// eight instead of six between the two stacked cells gives a 288.89 column and
/// 506 x 285 again. **Both wrong gaps are caught by the same four numbers**,
/// which is why the case is here rather than in a comment.
#[test]
fn the_gaps_beside_the_picture_are_the_mocks_own() {
    let arranged = program_body(body((1100.0, 333.0)), CANVAS).expect("on screen");

    assert_eq!(arranged.placement, Placement::Beside);
    assert_rect(
        arranged.picture,
        (299.0, 25.0),
        (502.0, 283.0),
        "the picture",
    );

    // Said the other way round, so the failure reads as a gap rather than as a
    // coordinate: the picture starts one `.program-body` divider past the
    // column, plus what centring it in the leftover 0.67 of a pixel gives.
    let column = (333.0 - size::PREVIEW_GAP) / 2.0 * 16.0 / 9.0;
    let gap = arranged.picture.min.x - (arranged.cells[0].min.x - 0.333_333) - column;
    assert!(
        near(gap, 8.0 + 0.333_333),
        "the gap to the picture is {gap}"
    );
    // And the gap between the two stacked cells is the row's own six, less the
    // quarter pixel each of them gives up to being a whole number.
    let stacked = arranged.cells[1].min.y - arranged.cells[0].max.y;
    assert!(
        near(stacked, size::PREVIEW_GAP + 0.5),
        "the gap between two stacked cells is {stacked}"
    );
}

// ---------------------------------------------------------------------------
// Against what the console draws today
// ---------------------------------------------------------------------------

/// **Below is what the console draws today, to the pixel.**
///
/// At the narrowest console the mock will draw, this function and the two the
/// panel is currently painted from have to agree exactly — the picture from
/// `picture_rect` off the `program-view` region, and the four cells from
/// `preview_rects` off `deck-previews`. They arrive at the same rectangles from
/// two different starting points: those two inset the two regions the
/// arrangement solved, and this one insets the bay and puts the row back where
/// the arrangement's own 8px divider left it.
///
/// **It is the row's height that this holds.** `size::PREVIEW_ROW_H` is the 63
/// in `deck-previews`'s 72, and there is nothing else in the crate that would
/// fail if it were 62: the region is solved from the arrangement's number and
/// this function is not.
#[test]
fn below_is_what_the_console_draws_today() {
    let layout = solved(SMALLEST);
    let bay = rect_of(&layout, "program");
    let arranged = program_body(
        Rect::from_min_max(
            Pos2::new(
                bay.x + size::PROGRAM_BODY_PAD,
                bay.y + size::HEAD_H + size::PROGRAM_BODY_PAD,
            ),
            Pos2::new(
                bay.x + bay.w - size::PROGRAM_BODY_PAD,
                bay.y + bay.h - size::PROGRAM_BODY_PAD,
            ),
        ),
        CANVAS,
    )
    .expect("on screen");

    assert_eq!(arranged.placement, Placement::Below);
    assert_eq!(
        arranged.picture,
        picture_rect(&layout, CANVAS).expect("the picture is on screen")
    );
    assert_eq!(
        arranged.cells,
        preview_rects(&layout).expect("the preview row is on screen")
    );

    // The `deck-previews` region is where the row is, arrived at from the
    // arrangement: the cells sit in the top 63 of its 72.
    let region = rect_of(&layout, "deck-previews");
    assert!(near(arranged.cells[0].min.y, region.y));
    assert!(near(
        arranged.cells[0].max.y,
        region.y + size::PREVIEW_ROW_H
    ));
    assert!(near(region.h - size::PREVIEW_ROW_H, size::PROGRAM_BODY_PAD));
}

/// **The same rectangle always gives the same answer**, and asking twice does
/// not change one.
///
/// The decider is a comparison and not a mode, which is what leaves
/// [P-0071](../../../docs/principles/0071-solving-a-layout-never-mutates-it.md)
/// untouched: there is nothing to store, so a window dragged wide and back
/// again comes back to exactly the arrangement it left. Stated over a drag
/// rather than over one call, because the failure a stored mode brings is
/// hysteresis and hysteresis needs a path to show up in.
#[test]
fn a_width_dragged_out_and_back_comes_back_to_the_same_arrangement() {
    let at = |w: f32| program_body(body((w, 333.0)), CANVAS).expect("on screen");

    let out: Vec<_> = (466..=1600).map(|w| at(w as f32)).collect();
    let back: Vec<_> = (466..=1600).rev().map(|w| at(w as f32)).collect();
    for (n, there) in out.iter().enumerate() {
        assert_eq!(
            there,
            &back[back.len() - 1 - n],
            "at {} wide the way out and the way back disagree",
            466 + n
        );
    }

    // And the panel's own two windows, so the sweep is anchored to something
    // real at both ends.
    assert_eq!(
        program_body(body(NARROWEST), CANVAS).map(|b| b.placement),
        Some(Placement::Below)
    );
    let wide = rect_of(&solved(PLAUSIBLE), "program");
    assert!(near(wide.w - size::PROGRAM_BODY_PAD * 2.0, WIDE.0));
    assert_eq!(
        program_body(body(WIDE), CANVAS).map(|b| b.placement),
        Some(Placement::Beside)
    );

    // The bay really is the 378 both of those were worked out from, so the
    // body's 333 is the arrangement's and not this file's.
    assert!(near(
        wide.h - size::HEAD_H - size::PROGRAM_BODY_PAD * 2.0,
        WIDE.1
    ));
}

/// **The crossover is a curve and not a width**, and the corner of it that an
/// operator can reach at the narrowest console is worth pinning.
///
/// The Program bay's own minimum is 200, which is a body **155** tall — and
/// there, even at the mock's narrowest 466, the cells go down the sides. Below
/// has only 155 - 8 - 63 = 84 of height left for the picture, which is 149 x 84;
/// beside gives it the whole 155 between two 132-wide columns, which is
/// 185 x 104. **A short bay is exactly what this arrangement is for**, and a
/// bay dragged to its minimum is short whatever the window is doing.
#[test]
fn a_bay_dragged_to_its_minimum_goes_beside_at_the_narrowest_console() {
    let short = program_body(body((466.0, 155.0)), CANVAS).expect("on screen");
    assert_eq!(short.placement, Placement::Beside);
    assert!(near(short.picture.width(), 185.0) && near(short.picture.height(), 104.0));
    // A cell is 132 x 74 there, which is larger than the 112 x 63 the row gives
    // at that width: the cells did not pay for the picture's extra height.
    assert!(near(short.cells[0].width(), 132.0) && near(short.cells[0].height(), 74.0));

    // And the bay at its stated height at the same width is the mock's again,
    // so this is the height doing it and not the width.
    assert_eq!(
        program_body(body(NARROWEST), CANVAS)
            .expect("on screen")
            .placement,
        Placement::Below
    );
}
