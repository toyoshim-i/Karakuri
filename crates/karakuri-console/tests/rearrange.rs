//! Program bay rearrangement between row and column cell layouts at the crossover width
//! (ADR-0182, ADR-0183).

mod common;

use common::{
    arranged, assert_sane, assert_within_bounds, id_of, near, rect_of, rects, PLAUSIBLE, SMALLEST,
};
use karakuri_console::panel::{Op, Outcome, Panel};
use karakuri_console::view::{picture_rect, preview_rects, program_bay, rearrange, Placement};
use karakuri_layout::Rect;

/// The canvas the picture is fitted to, and it is the workspace's reference
/// workload — 1280x720, which `crates/karakuri/src/main.rs` names `CANVAS` and
/// builds its `Present` at.
const CANVAS: (u32, u32) = (1280, 720);

/// The last window whose Program bay is the mock's arrangement, and the first
/// whose is not — 778 is the sum of side tracks and padding (ADR-0239).
const BELOW: f32 = 1483.0;
const BESIDE: f32 = 1484.0;

/// A console at `width`, arranged the way a frame arranges it.
fn at(width: f32) -> Panel {
    arranged(
        Rect {
            w: width,
            ..SMALLEST
        },
        CANVAS,
    )
}

/// Which way round the bay arranged itself, asked of the console rather than of
/// the arithmetic.
fn placement(panel: &Panel) -> Placement {
    program_bay(panel.layout(), CANVAS)
        .expect("the Program bay is on screen")
        .placement
}

/// Whether the row is out of the layout for the reason that is not the
/// operator's.
fn set_aside(panel: &Panel) -> bool {
    panel
        .layout()
        .is_set_aside(id_of(panel.layout(), "deck-previews"))
}

// ---------------------------------------------------------------------------
// The crossover
// ---------------------------------------------------------------------------

/// Crossing width boundaries shifts cell layout and transfers row height to the program view.
#[test]
fn the_bay_rearranges_at_the_crossover() {
    // Below crossover: previews row is positioned below picture.
    let panel = at(BELOW);
    assert_eq!(placement(&panel), Placement::Below);
    assert!(
        !set_aside(&panel),
        "the row is set aside below the crossover"
    );
    let row = rect_of(panel.layout(), "deck-previews");
    assert!(
        near(row.h, 89.0),
        "the row is {} tall and not its 89",
        row.h
    );
    let cells = preview_rects(panel.layout(), CANVAS).expect("the cells are on screen");
    assert!(near(cells[0].height(), 63.0));
    // Four across: every cell has the same top edge and a different left one.
    assert!(near(cells[0].min.y, cells[3].min.y));
    assert!(cells[3].min.x > cells[0].max.x);
    assert_sane(panel.layout());
    assert_within_bounds(panel.layout());

    // Above crossover: previews row is placed beside picture.
    let panel = at(BESIDE);
    assert_eq!(placement(&panel), Placement::Beside);
    assert!(
        set_aside(&panel),
        "the cells went beside the picture and the row was left in the layout"
    );
    let row = rect_of(panel.layout(), "deck-previews");
    assert!(
        near(row.h, 0.0),
        "the row still takes {} of height under a picture that is using it",
        row.h
    );
    let cells = preview_rects(panel.layout(), CANVAS).expect("the cells are on screen");
    // Two columns of two: A over B on the left, C over D on the right.
    assert!(near(cells[0].min.x, cells[1].min.x));
    assert!(near(cells[2].min.x, cells[3].min.x));
    assert!(cells[1].min.y > cells[0].max.y);
    assert!(cells[2].min.x > cells[0].max.x);
    assert!(
        near(cells[0].height(), 63.0),
        "a cell beside the picture preserves its 63 height (ADR-0239)",
    );
    assert_sane(panel.layout());
    assert_within_bounds(panel.layout());

    // The program view gains released row space at crossover, ensuring smooth visual transition.
    let below = picture_rect(at(BELOW).layout(), CANVAS).expect("on screen");
    let beside = picture_rect(at(BESIDE).layout(), CANVAS).expect("on screen");
    assert!(
        near(below.width(), 473.0) && near(below.height(), 266.0),
        "{:?}",
        below.size()
    );
    assert!(
        near(beside.width(), 474.0) && near(beside.height(), 267.0),
        "{:?}",
        beside.size()
    );
    assert!(
        beside.width() * beside.height() > below.width() * below.height(),
        "the bay rearranged itself into a smaller picture: {:?} against {:?}",
        beside.size(),
        below.size()
    );

    // Verify picture sizing at 1920 window width (ADR-0182).
    let wide = picture_rect(at(PLAUSIBLE.w).layout(), CANVAS).expect("on screen");
    assert!(
        near(wide.width(), 622.0) && near(wide.height(), 350.0),
        "{:?}",
        wide.size()
    );
    assert!(wide.width() * wide.height() > below.width() * below.height() * 1.5);

    // Picture rectangle is contained within its clipped region in both placements.
    for panel in [at(BELOW), at(BESIDE)] {
        let rect = picture_rect(panel.layout(), CANVAS).expect("on screen");
        let region = rect_of(panel.layout(), "program-view");
        assert!(
            rect.min.y >= region.y - 1e-3 && rect.max.y <= region.y + region.h + 1e-3,
            "the picture runs from {} to {} outside the region it is clipped to, {region:?}",
            rect.min.y,
            rect.max.y
        );
    }
}

/// Resizing past the crossover and back restores original layout rectangles without hysteresis (P-0082, ADR-0174).
#[test]
fn a_window_dragged_out_and_back_comes_back_to_the_same_rectangles() {
    let mut panel = at(SMALLEST.w);
    let before = rects(panel.layout());
    let picture = picture_rect(panel.layout(), CANVAS).expect("on screen");
    let cells = preview_rects(panel.layout(), CANVAS).expect("on screen");
    assert_eq!(placement(&panel), Placement::Below);

    // Out, one window at a time, so the way out passes through the crossover
    // rather than jumping over it.
    for w in (SMALLEST.w as u32..=2400).step_by(37) {
        panel.set_viewport(w as f32, SMALLEST.h);
        rearrange(&mut panel, CANVAS);
    }
    panel.set_viewport(2400.0, SMALLEST.h);
    rearrange(&mut panel, CANVAS);
    assert_eq!(placement(&panel), Placement::Beside);
    assert!(set_aside(&panel));

    // And back the same way.
    for w in (SMALLEST.w as u32..=2400).rev().step_by(37) {
        panel.set_viewport(w as f32, SMALLEST.h);
        rearrange(&mut panel, CANVAS);
    }
    panel.set_viewport(SMALLEST.w, SMALLEST.h);
    rearrange(&mut panel, CANVAS);

    assert_eq!(placement(&panel), Placement::Below);
    assert!(!set_aside(&panel), "the row came back set aside");
    assert_eq!(
        rects(panel.layout()),
        before,
        "a window dragged out past the crossover and back did not come back to the \
         arrangement it left"
    );
    assert_eq!(picture_rect(panel.layout(), CANVAS), Some(picture));
    assert_eq!(preview_rects(panel.layout(), CANVAS), Some(cells));
}

/// Rearrangement operations reach a stable fixed point without re-entering layout solves.
#[test]
fn the_second_ask_finds_nothing_to_do() {
    for width in [SMALLEST.w, BELOW, BESIDE, 2400.0, PLAUSIBLE.w] {
        let mut panel = at(width);
        let settled = rects(panel.layout());
        assert!(
            !rearrange(&mut panel, CANVAS),
            "at {width} wide the bay rearranged itself a second time, so the placement \
             it writes changes the rectangle it is derived from"
        );
        assert_eq!(rects(panel.layout()), settled);
        assert!(!rearrange(&mut panel, CANVAS));
    }
}

// ---------------------------------------------------------------------------
// The guard rule
// ---------------------------------------------------------------------------

/// With program view folded, preview cells remain visible in a horizontal row regardless of width.
#[test]
fn a_folded_picture_keeps_the_row_below_it_at_every_width() {
    for width in [SMALLEST.w, BELOW, BESIDE, 2400.0, 3440.0] {
        let mut panel = at(width);
        let picture = id_of(panel.layout(), "program-view");
        assert!(matches!(
            panel.op(Op::Fold(picture)),
            Outcome::Folded { folded: true, .. }
        ));
        rearrange(&mut panel, CANVAS);

        assert!(
            !set_aside(&panel),
            "at {width} wide the picture is folded and the row is set aside, so the bay \
             has nothing left that is laid out"
        );
        assert_eq!(placement(&panel), Placement::Below);

        // Program bay retains previews row height (89px) with picture folded (ADR-0174).
        let bay = rect_of(panel.layout(), "program");
        assert!(
            near(bay.h, 89.0),
            "at {width} wide the Program bay is {} tall with the picture folded",
            bay.h
        );
        assert!(near(rect_of(panel.layout(), "deck-previews").h, 89.0));
        assert_eq!(picture_rect(panel.layout(), CANVAS), None);
        assert!(
            preview_rects(panel.layout(), CANVAS).is_some(),
            "at {width} wide, folding the picture took the auditions with it"
        );
        assert_sane(panel.layout());

        // And unfolding puts the bay back to whichever arrangement the width
        // asks for, which is the round trip through a fold rather than through
        // a resize.
        assert!(matches!(
            panel.op(Op::Unfold(picture)),
            Outcome::Folded { folded: false, .. }
        ));
        rearrange(&mut panel, CANVAS);
        assert_eq!(
            placement(&panel),
            match width < BESIDE {
                true => Placement::Below,
                false => Placement::Beside,
            }
        );
        assert_eq!(rects(panel.layout()), rects(at(width).layout()));
    }
}

/// Folding the program view while cells are arranged vertically restores the row immediately.
#[test]
fn folding_the_picture_beside_the_cells_brings_the_row_back() {
    let mut panel = at(PLAUSIBLE.w);
    assert_eq!(placement(&panel), Placement::Beside);
    assert!(set_aside(&panel));

    let picture = id_of(panel.layout(), "program-view");
    panel.op(Op::Fold(picture));
    rearrange(&mut panel, CANVAS);

    assert!(!set_aside(&panel));
    let bay = rect_of(panel.layout(), "program");
    assert!(
        near(bay.h, 89.0),
        "the Program bay is {} tall, so folding the picture beside the cells took the \
         whole bay off the panel",
        bay.h
    );
    let cells = preview_rects(panel.layout(), CANVAS).expect("the auditions are still on screen");
    assert!(
        near(cells[0].height(), 63.0),
        "a cell is {} tall, so the cells are still arranged around a picture that is not \
         there",
        cells[0].height()
    );

    // Back, exactly.
    panel.op(Op::Unfold(picture));
    rearrange(&mut panel, CANVAS);
    assert_eq!(placement(&panel), Placement::Beside);
    assert_eq!(rects(panel.layout()), rects(at(PLAUSIBLE.w).layout()));
}

// ---------------------------------------------------------------------------
// The operator's own fold, which is the other bit
// ---------------------------------------------------------------------------

/// User-driven folding and layout-driven rearrangement operate via independent state bits (ADR-0183).
#[test]
fn folding_the_row_works_in_both_arrangements() {
    for (width, want) in [
        (SMALLEST.w, Placement::Below),
        (PLAUSIBLE.w, Placement::Beside),
    ] {
        let mut panel = at(width);
        assert_eq!(placement(&panel), want);
        let settled = rects(panel.layout());
        let picture = picture_rect(panel.layout(), CANVAS).expect("on screen");

        let row = id_of(panel.layout(), "deck-previews");
        assert!(matches!(
            panel.op(Op::Fold(row)),
            Outcome::Folded { folded: true, .. }
        ));
        rearrange(&mut panel, CANVAS);

        // The cells are gone and the picture has the body whole — there is
        // nothing left to arrange around, so the bay is below by the same rule
        // that puts it below with no room for two columns.
        assert_eq!(preview_rects(panel.layout(), CANVAS), None);
        assert_eq!(placement(&panel), Placement::Below);
        assert!(
            !set_aside(&panel),
            "at {width} wide a folded row was also set aside, so the operator's unfold \
             has a second bit to get past"
        );
        let alone = picture_rect(panel.layout(), CANVAS).expect("on screen");
        assert!(
            alone.width() * alone.height() >= picture.width() * picture.height(),
            "at {width} wide, folding the row left the picture smaller: {:?} against {:?}",
            alone.size(),
            picture.size()
        );

        // Unfold restores original arrangement based on window width.
        assert!(matches!(
            panel.op(Op::Unfold(row)),
            Outcome::Folded { folded: false, .. }
        ));
        rearrange(&mut panel, CANVAS);
        assert_eq!(placement(&panel), want);
        assert_eq!(
            rects(panel.layout()),
            settled,
            "at {width} wide the row came back to a different arrangement"
        );
        assert_eq!(picture_rect(panel.layout(), CANVAS), Some(picture));
    }
}

// ---------------------------------------------------------------------------
// A frame on which nothing moved
// ---------------------------------------------------------------------------

/// Idempotent rearrangement does not dirty layout or request extra repaints (ADR-0164, ADR-0183).
#[test]
fn a_frame_where_nothing_moved_writes_nothing() {
    for width in [SMALLEST.w, PLAUSIBLE.w] {
        let mut panel = at(width);
        assert!(!rearrange(&mut panel, CANVAS));

        // The same value the node already carries, written the way the frame
        // writes it, and then a rectangle read with nothing in between.
        let row = id_of(panel.layout(), "deck-previews");
        let carried = panel.layout().is_set_aside(row);
        assert!(
            !panel.set_aside(row, carried),
            "at {width} wide, writing the bit a node already carries reported a change"
        );
        let _ = panel.layout().rect(row);

        // And the setting that *is* a change reports one, so the assertion
        // above is about idempotence rather than about a write that never
        // happens.
        assert!(panel.set_aside(row, !carried));
    }
}

/// Soloing the preview cells when arranged vertically clears their set-aside bit (ADR-0183).
#[test]
fn a_solo_on_a_row_that_was_beside_the_picture_lays_it_out_again() {
    let mut panel = at(PLAUSIBLE.w);
    assert!(set_aside(&panel));
    let row = id_of(panel.layout(), "deck-previews");

    // The state ADR-0183 describes, before the console has said anything: the
    // soloed node has the viewport and is not in the layout.
    panel.op(Op::Solo(row));
    panel.solve();
    assert!(!panel.layout().visible(row));
    assert!(near(rect_of(panel.layout(), "deck-previews").h, 0.0));

    // And the frame's own first act, which is what puts it back.
    rearrange(&mut panel, CANVAS);
    assert!(!set_aside(&panel));
    let solo = rect_of(panel.layout(), "deck-previews");
    assert!(
        near(solo.w, PLAUSIBLE.w) && near(solo.h, SMALLEST.h),
        "the soloed row is {solo:?} and a solo leaves it the whole viewport"
    );
    let cells = preview_rects(panel.layout(), CANVAS).expect("the soloed row draws its cells");
    assert!(cells[0].width() > 400.0, "{:?}", cells[0].size());

    // Back, exactly: the unsolo restores the operator's fold and the frame
    // re-derives the bit from the geometry that is back.
    panel.op(Op::Unsolo);
    rearrange(&mut panel, CANVAS);
    assert!(set_aside(&panel));
    assert_eq!(rects(panel.layout()), rects(at(PLAUSIBLE.w).layout()));
}
