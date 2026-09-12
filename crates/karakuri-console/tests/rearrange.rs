//! The Program bay rearranging itself, and the one bit that follows from it.
//!
//! `view::program_body` answers where the picture and the four cells go for a
//! body rectangle and `tests/program_body.rs` holds that arithmetic against the
//! mock. This is the other half: the console asking it of the bay it solved to,
//! setting `deck-previews` aside where the cells went down the sides, and
//! putting it back where they did not — `view::rearrange`, which is what a
//! frame does before it reads a rectangle.
//!
//! None of it needs a window or a device. What it needs is a [`Panel`], because
//! the bit is written through one, and the whole of what is asserted is
//! rectangles and two flags.
//!
//! # The crossover in windows rather than in bodies
//!
//! ADR-0182's flip is at a body 1064 wide. The body is the window less 524: the
//! left pane's 218, the right pane's 268, the two 10px dividers between the
//! three tracks — which is the 990 - 218 - 268 - 20 = 484 the narrowest console
//! is derived from, read at any width — and `.program-body`'s 9px of padding
//! either side, which is 18. So the last window with the cells in a row is 1587
//! and the first with them down the sides is 1588. Both are asserted rather
//! than assumed, here and in `tests/view.rs`: a crossover that moved is a test
//! that fails rather than a test that quietly starts asserting one arrangement
//! twice.

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

/// The bay rearranges itself at the crossover, and the row goes with it.
///
/// Three things have to happen together and each fails on its own: the cells
/// move, the row's node stops taking height, and the picture takes what the row
/// gave up. The defect this exists for is any one of the three without the
/// others — cells drawn down the sides with the row still 89 tall underneath
/// them is a bay with a strip of ground where a row used to be and four
/// thumbnails over the picture, and nothing in `program_body` can see it,
/// because `program_body` is not told what the layout did with its answer.
#[test]
fn the_bay_rearranges_at_the_crossover() {
    // **Below**, and the row is where the arrangement put it.
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

    // **One pixel of window later, beside** — and the same three things the
    // other way round.
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

    // **And the picture took what the row gave up**, which is the whole of why
    // the arrangement exists. At the crossover itself it is one pixel of
    // height — ADR-0182's own arithmetic: 466 x 262 is 122,092 texels and
    // 466 x 263 is 122,558, and *the larger picture wins* is decided by that
    // 466. **A crossover is a hair either way and that is the point**: the
    // flip an operator sees while dragging is a pixel and not a jump.
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

    // **And the width past it is what the arrangement is for**: at a 1920
    // window the body is 1396 x 350, the picture beside the cells is
    // **622 x 350**, and the same bay with the cells under it would have the
    // mock's 466 x 262 — 78% more picture, which is ADR-0182's motivating
    // number reached through the console rather than through the arithmetic.
    // The body is 17 taller than it was because the preview captions grew the
    // bay by that much, and beside is the arrangement that spends a bay's
    // height on the picture rather than on the row.
    let wide = picture_rect(at(PLAUSIBLE.w).layout(), CANVAS).expect("on screen");
    assert!(
        near(wide.width(), 622.0) && near(wide.height(), 350.0),
        "{:?}",
        wide.size()
    );
    assert!(wide.width() * wide.height() > below.width() * below.height() * 1.5);

    // **The picture is inside the region it is clipped to**, in both. Beside,
    // that region is the whole bay — the row is not in the layout to divide it
    // — and a picture drawn from the bay while the region was still 298 tall
    // would be clipped through the middle by `View::draw`.
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

/// Out past the crossover and back, and every rectangle returns.
///
/// [P-0082](../../../docs/principles/0082-looking-never-writes-back.md) reached
/// through the rearrangement: the bit is a function of the geometry and nothing
/// is stored, so a window dragged wide and back comes back to the arrangement
/// it left rather than near it. It is
/// [ADR-0174](../../../docs/adr/0174-a-node-claims-only-what-its-visible-content-can-use.md)'s
/// round trip with a second bit in it, and the failure it is written against is
/// hysteresis: a stored mode, a remembered placement, a `set_aside` cleared
/// somewhere other than where it was written, and the panel comes back with the
/// row 89 tall inside a bay that no longer has room for it.
///
/// Every rectangle in the arena, not the two the bay draws: a bit that was left
/// set on the way back moves the inspector under it as well.
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

/// Asking twice changes nothing, which is the whole of *nothing re-enters the
/// solve*.
///
/// The bit is derived from the bay's rectangle and the bay's rectangle does not
/// depend on the bit — `program` is `Fixed(395)` over a flexible
/// `program-view`, so what it can use is unbounded either way. That makes one
/// write a fixed point rather than the first step of a chase, and the failure
/// if it were not is a panel that alternates between two arrangements for as
/// long as anything asks it to draw.
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

/// With the picture folded the row is below, whatever the window is doing.
///
/// The rule is the console's and `karakuri-layout` deliberately does not hold
/// it: a split can use nothing when none of its children is laid out, so a row
/// set aside under a folded picture leaves the Program bay claiming zero — the
/// bay, its head and both its regions gone from the panel, and the inspector
/// swelling into the space. And the manual promises the opposite in as many
/// words: the deck previews *"are auditions of their own, so they stay when it
/// goes"*.
///
/// So this asserts the bay is still there, still 89 tall, and still drawing
/// four cells — at widths either side of the crossover, because the defect is a
/// rule that only bites past it.
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

        // **The bay is the row's 89 and not zero**, which is ADR-0174's answer
        // unchanged: the picture gave its height to the inspector and the row
        // kept its own.
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

/// The picture folded while the cells are beside it, which is the guard rule as
/// a gesture rather than as a width.
///
/// The other order, and the one an operator reaches: the bay is already
/// rearranged, the four cells are down the sides, and then the picture goes.
/// Everything has to come back — the row into the layout, the cells into the
/// row — on the frame the fold happened, because the frame after it is drawn
/// from a bay that claims nothing.
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

/// Folding the row works in both arrangements, and unfolding brings it back to
/// the right one.
///
/// Two bits and they are independent in both directions
/// ([ADR-0183](../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)):
/// the operator's fold takes the cells off the panel wherever they are, and the
/// rearrangement decides where they come back to. The defect is one bit for
/// both, and it shows here in whichever direction it is written: an unfold that
/// clears the console's bit puts an empty row back under a picture that is
/// using the height, and a rearrangement that clears the operator's fold
/// unfolds a row nobody asked for the moment the window is dragged.
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

        // **And `Op::Unfold` brings it back to the arrangement the width asks
        // for**, which is the same one it left.
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

/// A frame on which nothing moved marks nothing dirty and asks for nothing.
///
/// [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s
/// still-panel clause, at the one place in the console that writes to the arrangement
/// on every frame. Two halves, and the second is the one the injection is
/// about:
///
/// - The answer is `false`, so `Change::Rearranged` asks for no frame — a
///   `rearrange` that reported what the caller asked rather than what changed
///   would ask for one on every frame, for ever, and the still window would
///   never sleep.
/// - The write itself does not dirty the layout. `Layout::rect` refuses to
///   answer from a dirty solve, so the rectangle read below with no `solve`
///   between it and the write is what says so: an unconditional
///   `set_aside` panics here with *rect() read a stale solve*, which is the
///   same assertion ADR-0183 records from the other side of the seam.
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

/// Solo the row while it is beside the picture, and the console is what says it
/// is not beside the picture any more.
///
///
/// [ADR-0183](../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)
/// leaves this here on purpose: a solo saves and restores the operator's fold
/// and nothing else, so a node that is set aside stays set aside through a
/// solo, the soloed node included — left holding the whole viewport and still
/// not laid out, *"until whoever set it aside says otherwise"*. This is the
/// console saying otherwise, and it happens before a rectangle is read because
/// `rearrange` is the first thing a frame does.
///
/// Without it the panel is empty: the row holds the viewport with no height in
/// it and everything else is folded away, which is a black window and a key
/// that appeared to do nothing.
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
