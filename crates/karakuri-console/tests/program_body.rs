//! How the Program bay's body arranges itself in the rectangle it has, and the
//! arithmetic checked against the mock rather than against the code that
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
use karakuri_console::view::{
    caption_of, picture_rect, preview_rects, program_body, program_body_with_row_h, Placement,
    DECKS,
};

/// The canvas the picture is fitted to, and it is the workspace's reference
/// workload — 1280x720, which `crates/karakuri/src/main.rs` names `CANVAS` and
/// builds its `Present` at.
const CANVAS: (u32, u32) = (1280, 720);

/// A canvas that is not the mock's shape, so a picture fitted to a hard-coded
/// 16:9 and one fitted to *the canvas* can be told apart, and so can a cell
/// that followed the canvas when it should not.
const SQUARISH: (u32, u32) = (1024, 768);

/// The Program bay's body at the narrowest console the mock will draw.
///
/// `.console`'s `min-width: 1010px` less its 10px of padding either side is
/// 990; the centre track is 990 - 218 - 268 - two 10px gaps = 484; the bay is
/// that wide, and `.program-body`'s 9px padding leaves 466. The bay is 395
/// tall, less the 27 of bay head painted over it and 9 of that padding top and
/// bottom, which is 350.
const NARROWEST: (f32, f32) = (466.0, 350.0);

/// The same body in a 1920 window. The two side tracks and the four dividers do
/// not move, so the centre track takes the whole of the extra width: 1920 - 340
/// - 400 - 20 = 1160, less the same 18 of padding = 1142 (ADR-0239). The bay is
/// `Sizing::Fixed` along its column, so the height is still 350.
const WIDE: (f32, f32) = (1142.0, 350.0);

/// A body of that size, and not at the origin: every rectangle this answers
/// with is inside the bay somebody solved, so an arrangement that had quietly
/// assumed a zero origin would tile a panel that has none.
fn body(size: (f32, f32)) -> Rect {
    Rect::from_min_size(Pos2::new(37.0, 61.0), egui::vec2(size.0, size.1))
}

/// The size of the largest `aspect` box that fits in `w` x `h`, in whole pixels
/// — `view::fitted`'s rule, restated rather than reached for. The decider is
/// the thing under test here, and a test that asked the code for both of the
/// sizes it is choosing between would be comparing the code with itself.
fn largest(w: f32, h: f32, aspect: (f32, f32)) -> Option<(f32, f32)> {
    let scale = (w / aspect.0).min(h / aspect.1);
    let box_w = (aspect.0 * scale).round().min(w.floor());
    let box_h = (aspect.1 * scale).round().min(h.floor());
    match box_w > 0.0 && box_h > 0.0 {
        true => Some((box_w, box_h)),
        false => None,
    }
}

/// The two pictures the decider is choosing between, `None` for one that cannot
/// be drawn at all.
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
    // A cell is the image and the caption band under it — `.cell`'s 4px gap
    // and `.caption`'s 13px — and it is the **image** that is 16:9, so the
    // column follows the image and the height a cell takes is the sum.
    let cell_h = 63.0f32;
    let band = 4.0 + 13.0;
    let column = (cell_h * 16.0 / 9.0).round();
    let min_w = column * 2.0 + 8.0;
    let total_cells_h = (cell_h + band) * 2.0 + 6.0;
    let beside = if w > min_w && h >= total_cells_h {
        largest(w - column * 2.0 - 8.0, h, canvas)
    } else {
        None
    };
    Hand {
        below: largest(w, h - 4.0 - cell_h - band, canvas),
        beside,
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

/// At the mock's own body the arrangement is the mock's, and every number in it
/// is the mock's too.
#[test]
fn the_mocks_narrowest_body_is_the_mocks_own_arrangement() {
    let arranged = program_body(body(NARROWEST), CANVAS).expect("the body has room for a picture");

    assert_eq!(arranged.placement, Placement::Below);
    assert_rect(arranged.picture, (0.0, 2.0), (466.0, 262.0), "the picture");
    for (deck, cell) in arranged.cells.iter().enumerate() {
        assert_rect(
            *cell,
            (118.0 * deck as f32, 350.0 - 80.0),
            (112.0, 63.0),
            &format!("cell {deck}"),
        );
    }
}

/// At a 1920 window the cells go down the sides, preserving their size from the
/// row, and the picture is 61% larger for it.
#[test]
fn a_nineteen_twenty_window_puts_the_cells_down_the_sides() {
    let arranged = program_body(body(WIDE), CANVAS).expect("the body has room for a picture");

    assert_eq!(arranged.placement, Placement::Beside);
    // The picture's box is the body less a column and a divider either side:
    // 1142 - 2 x 112 - 2 x 4 = 910, and 16:9 in 910 x 350 is 622 x 350
    // centred, so 116 of divider and column plus (910 - 622) / 2 = 144.
    assert_rect(
        arranged.picture,
        (260.0, 0.0),
        (622.0, 350.0),
        "the picture",
    );

    // **A and B down the left, C and D down the right**, preserving 112 x 63
    // of image with its 17 of caption band under each: two cells are
    // (63 + 17) x 2 + 6 = 166 tall, centred in 350, so the first starts at 92.
    let left = 0.0;
    let right = 1142.0 - 112.0;
    let top = 92.0;
    let lower = top + 63.0 + 17.0 + 6.0;
    assert_rect(arranged.cells[0], (left, top), (112.0, 63.0), "cell A");
    assert_rect(arranged.cells[1], (left, lower), (112.0, 63.0), "cell B");
    assert_rect(arranged.cells[2], (right, top), (112.0, 63.0), "cell C");
    assert_rect(arranged.cells[3], (right, lower), (112.0, 63.0), "cell D");

    let below = program_body(body(NARROWEST), CANVAS)
        .expect("on screen")
        .picture;
    let gain =
        (arranged.picture.width() * arranged.picture.height()) / (below.width() * below.height());
    assert!(
        near(gain, 1.783_1),
        "beside gives a picture {gain} times the size of below's, and the claim is 1.78"
    );
}

/// Eight hundred wide goes beside with preserved preview size.
#[test]
fn an_eight_hundred_wide_body_goes_beside() {
    let arranged = program_body(body((800.0, 350.0)), CANVAS).expect("on screen");

    assert_eq!(arranged.placement, Placement::Beside);
    // 800 - 232 = 568 width for picture box. Picture is 568 x 320, centred at (116.0, 15.0).
    assert_rect(
        arranged.picture,
        (116.0, 15.0),
        (568.0, 320.0),
        "the picture",
    );
}

// ---------------------------------------------------------------------------
// The decider
// ---------------------------------------------------------------------------

/// The larger picture wins at every width, and a tie goes to the mock's.
#[test]
fn the_larger_picture_wins_at_every_width_and_a_tie_goes_below() {
    let mut below_won = 0;
    let mut beside_won = 0;

    for w in 466..=2400 {
        let size = (w as f32, 350.0);
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

    assert!(
        below_won > 100 && beside_won > 100,
        "{below_won} and {beside_won}"
    );
}

/// The two curves cross once at 706px (ADR-0239), and the caption band did not
/// move it: below loses the same 17 to the taller row that the bay gained, so
/// its picture is the 466 x 262 it always was.
#[test]
fn there_is_exactly_one_crossover_and_it_is_at_706() {
    let placement = |w: i32| {
        program_body(body((w as f32, 350.0)), CANVAS)
            .expect("on screen")
            .placement
    };

    let mut changes = Vec::new();
    for w in 467..=3440 {
        if placement(w) != placement(w - 1) {
            changes.push(w);
        }
    }
    assert_eq!(changes, vec![706], "the arrangement changes at {changes:?}");
    assert_eq!(placement(705), Placement::Below);
    assert_eq!(placement(706), Placement::Beside);
}

/// A body with nothing in it has no arrangement, either way round.
#[test]
fn a_body_with_no_room_has_no_arrangement() {
    assert_eq!(program_body(body((466.0, 0.0)), CANVAS), None);
    assert_eq!(program_body(body((0.0, 350.0)), CANVAS), None);
    assert_eq!(program_body(body((-100.0, -100.0)), CANVAS), None);
}

// ---------------------------------------------------------------------------
// The column, the cells and the gaps
// ---------------------------------------------------------------------------

/// A side column preserves preview cell dimensions.
#[test]
fn the_columns_preserve_cell_dimensions() {
    let at = |w: f32| program_body(body((w, 350.0)), CANVAS).expect("on screen");
    let reference = at(1396.0);
    assert_eq!(reference.placement, Placement::Beside);

    for w in [750.0, 1100.0, 1200.0, 1396.0, 1920.0, 3440.0] {
        let arranged = at(w);
        assert_eq!(arranged.placement, Placement::Beside, "at {w} wide");
        for deck in 0..DECKS {
            assert!(
                near(arranged.cells[deck].width(), 112.0)
                    && near(arranged.cells[deck].height(), 63.0),
                "at {w} wide cell {deck} is {:?} and should be 112x63",
                arranged.cells[deck].size()
            );
        }
        assert!(
            near(arranged.cells[0].min.x, reference.cells[0].min.x)
                && near(arranged.cells[0].min.y, reference.cells[0].min.y),
            "at {w} wide the left column moved"
        );
    }
}

/// A cell is 16:9 in both arrangements.
#[test]
fn a_cell_is_the_mocks_shape_and_never_the_canvass() {
    for size in [NARROWEST, WIDE, (1100.0, 350.0)] {
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
                (cell.width() / cell.height() - 16.0 / 9.0).abs() < 0.05,
                "a cell is {}x{} and `.preview` is 16:9",
                cell.width(),
                cell.height()
            );
        }
    }
}

/// The two gaps beside the picture are the mock's own.
#[test]
fn the_gaps_beside_the_picture_are_the_mocks_own() {
    let arranged = program_body(body((750.0, 350.0)), CANVAS).expect("on screen");

    assert_eq!(arranged.placement, Placement::Beside);
    assert_rect(
        arranged.picture,
        (116.0, 29.5),
        (518.0, 291.0),
        "the picture",
    );

    let gap = arranged.picture.min.x - arranged.cells[0].max.x;
    assert!(near(gap, 4.0), "the gap to the picture is {gap}");
    // **Between two stacked cells, and a cell ends at its caption** — the
    // image's own bottom edge is 17 short of that, and measuring from there
    // would read the caption band as part of the gap.
    let stacked = arranged.cells[1].min.y - caption_of(arranged.cells[0]).max.y;
    assert!(
        near(stacked, size::PREVIEW_GAP),
        "the gap between two stacked cells is {stacked}"
    );
}

// ---------------------------------------------------------------------------
// Against what the console draws today
// ---------------------------------------------------------------------------

/// Below is what the console draws today, to the pixel.
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
        preview_rects(&layout, CANVAS).expect("the preview row is on screen")
    );

    let region = rect_of(&layout, "deck-previews");
    assert!(near(arranged.cells[0].min.y, region.y));
    // The **image** ends at its own height and the caption band fills the
    // rest of the row, so the row's height is the cell's and the image's is
    // one term of it.
    assert!(near(
        arranged.cells[0].max.y,
        region.y + size::PREVIEW_IMAGE_H
    ));
    assert!(near(
        caption_of(arranged.cells[0]).max.y,
        region.y + size::PREVIEW_ROW_H
    ));
    assert!(near(region.h - size::PREVIEW_ROW_H, size::PROGRAM_BODY_PAD));
}

/// The same rectangle always gives the same answer.
#[test]
fn a_width_dragged_out_and_back_comes_back_to_the_same_arrangement() {
    let at = |w: f32| program_body(body((w, 350.0)), CANVAS).expect("on screen");

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

    assert!(near(
        wide.h - size::HEAD_H - size::PROGRAM_BODY_PAD * 2.0,
        WIDE.1
    ));
}

/// A bay dragged to its minimum goes beside at the narrowest console.
#[test]
fn a_bay_dragged_to_its_minimum_goes_beside_at_the_narrowest_console() {
    let short = program_body_with_row_h(body((466.0, 172.0)), CANVAS, size::PREVIEW_ROW_H)
        .expect("on screen");
    assert_eq!(short.placement, Placement::Beside);
    // The bay's own minimum of 217 less its head and padding is a body 172
    // tall. A column is 112x63 of image. Picture box is 466 - 232 = 234 x 172,
    // and 16:9 in that is 234 x 132.
    assert!(near(short.picture.width(), 234.0) && near(short.picture.height(), 132.0));
    assert!(near(short.cells[0].width(), 112.0) && near(short.cells[0].height(), 63.0));

    assert_eq!(
        program_body(body(NARROWEST), CANVAS)
            .expect("on screen")
            .placement,
        Placement::Below
    );
}
