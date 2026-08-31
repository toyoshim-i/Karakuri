//! **The Outputs row, and the console's first clickable control.**
//!
//! Four things, and the middle two are the reason this file exists rather than
//! a few more assertions in `view.rs`:
//!
//! 1. Where the control is, derived from the row's own geometry.
//! 2. **That it clears every boundary's grab.** `karakuri_console::input` has
//!    said since it was written that its rule holds *"only while the gaps stay
//!    empty"* — [`GRAB`] widens every boundary by six pixels either side, and
//!    those twelve pixels are inside the bays, over whatever a bay draws at its
//!    edge. This is the first control drawn near one, and this is the guard
//!    that documentation has been asking for.
//! 3. That the dot's state is read from the arrangement and not kept beside it.
//! 4. That a press on it is the same fold the keyboard performs, and that the
//!    round trip restores the arrangement exactly — ADR-0174's own claim,
//!    reached from the control instead of from the layout.
//!
//! None of it needs a window or a device. It does need `egui`'s fonts, because
//! the chip is as wide as the name in it — see `common::drawn_once`.

mod common;

use common::{drawn_once, id_of, near, rect_of, rects, showing, solved, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Outcome, Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::outputs;
use karakuri_layout::{Axis, Layout, Point, Rect};
use karakuri_operation::gate::Open;

/// A panel at a viewport, solved, with a context that has drawn once — the
/// pair every test here starts from.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

// ---------------------------------------------------------------------------
// Where the control is
// ---------------------------------------------------------------------------

/// **The row's furniture is the row's rectangle and the mock's own boxes**,
/// and every number here is read off `style.css` rather than off the panel.
///
/// `.outputs { padding: 8px 11px; gap: 8px; align-items: center }` around a
/// `.sink` of `padding: 1px 10px; gap: 6px` holding a 7px `.dot` — so the word
/// starts one padding in, the chip starts one gap after the word, and the chip
/// is 18.5 tall whatever the row is, which is the 18.5 the arrangement's 34 was
/// written from (`lib.rs`).
#[test]
fn the_control_is_the_rows_own_geometry() {
    let (panel, ctx) = console(SMALLEST);
    let row = rect_of(panel.layout(), "outputs");
    let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("the row draws its sink");

    // The word: `.outputs`'s left padding, and centred in the row.
    assert!(
        near(sink.label.min.x, row.x + size::OUTPUTS_PAD_X),
        "the word starts at {} and the row's padding leaves {}",
        sink.label.min.x,
        row.x + size::OUTPUTS_PAD_X
    );
    assert!(near(sink.label.center().y, row.y + row.h * 0.5));

    // **The class pill, between the word and the chip** — one `.outputs` gap
    // after the word, which is where `docs/manual/console.html` draws it and
    // why: this row has no bay head to put an indicator in, so it sits beside
    // the word that stands in for one. Its own geometry and everything it does
    // are `tests/mcp_pill.rs`; what is asserted here is that it is in this row
    // and that the chip is measured from it. See `view::Outputs::mcp`.
    assert!(
        near(sink.mcp.min.x, sink.label.max.x + size::OUTPUTS_GAP),
        "the class pill starts {} after the word and the gap is {}",
        sink.mcp.min.x - sink.label.max.x,
        size::OUTPUTS_GAP
    );
    // The chip: one `.outputs` gap after the pill, 18.5 tall, centred.
    assert!(
        near(sink.sink.min.x, sink.mcp.max.x + size::OUTPUTS_GAP),
        "the chip starts {} after the class pill and the gap is {}",
        sink.sink.min.x - sink.mcp.max.x,
        size::OUTPUTS_GAP
    );
    assert!(
        near(sink.sink.height(), 18.5),
        "the chip is {} tall and a sink is 11 at line-height 1.5 inside 1px of padding",
        sink.sink.height()
    );
    assert!(near(sink.sink.center().y, row.y + row.h * 0.5));

    // The dot: `.sink`'s left padding in, 7px square, centred.
    assert!(near(sink.dot.width(), size::SINK_DOT));
    assert!(near(sink.dot.height(), size::SINK_DOT));
    assert!(near(sink.dot.min.x, sink.sink.min.x + size::SINK_PAD_X));
    assert!(near(sink.dot.center().y, row.y + row.h * 0.5));

    // And the name has the rest of the chip: the dot, `.sink`'s own gap, and
    // its right padding. The width is `egui`'s to decide — the mock's face is
    // not shipped (`room`) — so this is the chip's box around it rather than a
    // number of pixels.
    let name = sink.sink.max.x - sink.dot.max.x - size::SINK_GAP - size::SINK_PAD_X;
    assert!(
        name > 0.0,
        "the chip has {name} left for the name after its dot, gap and padding"
    );

    // The whole of it is inside the row it is drawn in.
    assert!(
        egui::Rect::from_min_size(egui::pos2(row.x, row.y), egui::vec2(row.w, row.h))
            .contains_rect(sink.sink),
        "the chip {:?} is not inside the row {row:?}",
        sink.sink
    );
}

/// **The row is 34 because a sink is 18.5**, so the two are one derivation.
/// A row that stopped being the height the arrangement says would leave the
/// chip somewhere else, and the clearance below is what that would cost.
#[test]
fn the_row_is_the_height_a_sink_needs() {
    let (panel, _) = console(PLAUSIBLE);
    let row = rect_of(panel.layout(), "outputs");
    assert!(near(row.h, 34.0), "the outputs row is {} tall", row.h);
    assert!(
        near(size::SINK_H, 18.5),
        "a sink is {} tall and the arrangement's 34 is 8 + 18.5 + 8",
        size::SINK_H
    );
}

// ---------------------------------------------------------------------------
// The claim rule
// ---------------------------------------------------------------------------

/// **The control clears every boundary's grab, and this is the guard
/// `karakuri_console::input` has been asking for since it was written.**
///
/// Its documentation states the hazard exactly: `GRAB` widens every boundary
/// by six pixels either side, *"and those twelve pixels are inside the bays,
/// over whatever the bay draws at its edge. The first control placed near a
/// bay's edge is under a boundary's grab, and then both do think they are
/// dragging."*
///
/// The numbers say it clears: the row is 34, the chip is 18.5 and centred, so
/// there is (34 - 18.5) / 2 = **7.75** above the chip and 7.75 below, against a
/// grab of **6**. The boundary above the row gives up 1.75 pixels short of the
/// control.
///
/// **So this fails if the control moves, if the row gets shorter, or if `GRAB`
/// widens** — and the last one is the point. Widening the grab to 8 makes the
/// top of this chip undraggable *and* unclickable, and the fix then is not to
/// nudge the control: it is to change the rule in `input`, deliberately.
#[test]
fn the_control_clears_every_boundarys_grab() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (mut panel, ctx) = console(viewport);
        let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("the row draws its sink");
        let row = rect_of(panel.layout(), "outputs");

        // The clearance, stated against the constant it has to beat.
        let clearance = (row.h - size::SINK_H) * 0.5;
        assert!(
            clearance > GRAB,
            "the chip has {clearance} of row above it and a boundary grabs {GRAB} — the \
             control is inside a boundary's grab, and `input`'s rule is what has to change"
        );
        assert!(near(sink.sink.min.y - row.y, clearance));
        assert!(near(row.y + row.h - sink.sink.max.y, clearance));

        // And the hit test agrees, which is the statement the numbers above
        // are only evidence for: `Layout::hit` widens a boundary by `GRAB`
        // along its split's axis and descends by rectangle, so this asks it at
        // every corner and edge of the chip rather than trusting the sum.
        let probes = [
            sink.sink.left_top(),
            sink.sink.right_top(),
            sink.sink.left_bottom(),
            sink.sink.right_bottom(),
            sink.sink.center(),
            sink.sink.center_top(),
            sink.sink.center_bottom(),
        ];
        for probe in probes {
            assert!(
                !matches!(
                    panel.layout().hit(at(probe), GRAB),
                    karakuri_layout::Hit::Divider { .. }
                ),
                "a boundary grabs {probe:?}, which is on the control"
            );
            assert_eq!(
                claim(&mut panel, &ctx, &showing(&[]), at(probe)),
                Claim::Panel,
                "the panel does not get a press at {probe:?}, which is on its own control"
            );
        }

        // The band above it is still the boundary's, which is what says the
        // clearance is a clearance and not the grab having gone missing: the
        // row's own top edge is inside the boundary above it.
        assert!(
            matches!(
                panel
                    .layout()
                    .hit(Point::new(sink.sink.center().x, row.y), GRAB),
                karakuri_layout::Hit::Divider { .. }
            ),
            "the top edge of the outputs row is not in the grab of the boundary above it, \
             so this test is no longer measuring the clearance it was written for"
        );
    }
}

/// **A press beside the control is `egui`'s, and one on it is the panel's.**
///
/// The rule is *the boundary gets first refusal*, then the panel's own
/// controls, then `egui`; so the interesting assertion is the one at the chip's
/// edge — a pixel outside it is nothing the console draws, and it goes to
/// `egui` exactly as the solo pill does.
#[test]
fn only_the_control_is_claimed_out_of_the_outputs_row() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("the row draws its sink");
    let row = rect_of(panel.layout(), "outputs");

    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), at(sink.sink.center())),
        Claim::Panel
    );
    // The word is not a control: the mock's `.sink` is what carries the click.
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), at(sink.label.center())),
        Claim::Egui,
        "the word OUTPUTS is a heading and is being treated as a control"
    );
    // **And the class pill beside it is one**, which is this row's second
    // control and the only one on the console that is not an operation at
    // either end — `tests/mcp_pill.rs` is where it is all asserted, and this
    // is the row saying it has two things in it a press can land on.
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), at(sink.mcp.center())),
        Claim::Panel,
        "the class pill in the Outputs row is not being claimed"
    );
    // Past the right of the chip, where the mock draws three more sinks and an
    // add-a-region pill and this console draws none of them.
    assert_eq!(
        claim(
            &mut panel,
            &ctx,
            &showing(&[]),
            Point::new(sink.sink.max.x + 20.0, sink.sink.center().y)
        ),
        Claim::Egui,
        "the empty half of the outputs row is being claimed"
    );
    // And the row's own bottom edge, which is the window's.
    assert_eq!(
        claim(
            &mut panel,
            &ctx,
            &showing(&[]),
            Point::new(row.x + row.w * 0.5, row.y + row.h - 1.0)
        ),
        Claim::Egui
    );
}

/// **Before anything has been drawn there is no control**, which is not a
/// special case to be worked around: a chip is as wide as the name in it, the
/// name has not been laid out, and a press cannot be on something that has
/// never been on screen. A window loop has drawn long before a hand arrives.
#[test]
fn a_control_that_has_not_been_drawn_is_not_there() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let fresh = egui::Context::default();
    assert_eq!(fresh.cumulative_pass_nr(), 0);
    assert_eq!(outputs(&fresh, panel.layout(), Open::CLOSED), None);

    // And the claim rule falls back to what it was: the pointer is over the
    // outputs row, which without a control on it is `egui`'s.
    let row = rect_of(panel.layout(), "outputs");
    let middle = Point::new(row.x + row.w * 0.5, row.y + row.h * 0.5);
    assert_eq!(
        claim(&mut panel, &fresh, &showing(&[]), middle),
        Claim::Egui
    );
}

// ---------------------------------------------------------------------------
// The state behind the dot
// ---------------------------------------------------------------------------

/// **The dot is `layout.visible(program-view)` in both directions, and there is
/// no second copy of it.**
///
/// The manual: *"The picture is a sink, listed in Outputs as program view, and
/// it is on screen exactly when that sink is on."* So this drives the picture
/// off and on by every route that reaches it — the node itself, the bay around
/// it, a solo elsewhere — and asks the dot each time. A stored state would keep
/// the last thing the *control* did and be wrong for all three.
#[test]
fn the_dot_follows_the_picture_and_stores_nothing() {
    let ctx = drawn_once();
    let mut layout = solved(PLAUSIBLE);
    let picture = id_of(&layout, "program-view");

    let on = |layout: &Layout| {
        outputs(&ctx, layout, Open::CLOSED)
            .expect("the row draws its sink")
            .on
    };
    assert!(on(&layout), "the picture is on screen and the sink is dark");

    // The node itself, which is what the control asks for.
    layout.collapse(picture);
    layout.solve();
    assert!(
        !on(&layout),
        "the picture is folded and the sink is still lit"
    );
    assert_eq!(on(&layout), layout.visible(picture));
    layout.expand(picture);
    layout.solve();
    assert!(on(&layout));

    // The bay around it — `g` over the picture — which the control never asks
    // for and which the manual's sentence covers all the same: the picture is
    // not on screen, so the sink is not on.
    layout.collapse(id_of(&layout, "program"));
    layout.solve();
    assert!(
        !on(&layout),
        "the bay is folded and the picture's sink is lit"
    );
    assert_eq!(on(&layout), layout.visible(picture));
    layout.expand(id_of(&layout, "program"));
    layout.solve();
    assert!(on(&layout));

    // A solo on something else, which folds nothing and hides everything: the
    // outputs row goes with it, so there is no control at all to be wrong.
    layout.solo(id_of(&layout, "library"));
    layout.solve();
    assert_eq!(
        outputs(&ctx, &layout, Open::CLOSED),
        None,
        "a solo left the outputs row invisible and its control still drawn"
    );
    assert!(!layout.visible(picture));
    layout.unsolo();
    layout.solve();
    assert!(on(&layout), "the solo was undone and the sink stayed dark");
}

/// **A fold from the keyboard shows on the dot**, which is the same claim from
/// the other end: two surfaces, one state, and no message between them.
#[test]
fn a_fold_from_anywhere_else_shows_on_the_dot() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let picture = panel.layout().find("program-view").expect("program-view");

    assert!(
        outputs(&ctx, panel.layout(), Open::CLOSED)
            .expect("a sink")
            .on
    );
    panel.op(Op::Fold(picture));
    assert!(
        !outputs(&ctx, panel.layout(), Open::CLOSED)
            .expect("a sink")
            .on,
        "`f` over the picture folded it and the sink beside it says it is on"
    );
    panel.op(Op::UnfoldAll);
    assert!(
        outputs(&ctx, panel.layout(), Open::CLOSED)
            .expect("a sink")
            .on
    );
}

// ---------------------------------------------------------------------------
// What a press on it asks for
// ---------------------------------------------------------------------------

/// **A press on the control asks for `Fold(program-view)`, and a press on it
/// again asks for `Unfold(program-view)`.**
///
/// Two operations and no third one: the toggle is the control choosing between
/// them from the state it can see, and what it hands the model is a named
/// operation either surface could have asked for.
#[test]
fn the_dot_asks_for_a_fold_and_then_for_an_unfold() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let picture = panel.layout().find("program-view").expect("program-view");

    let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("a sink");
    assert!(sink.on);
    assert_eq!(sink.op(), Op::Fold(picture));
    assert_eq!(
        panel.op(sink.op()),
        Outcome::Folded {
            id: picture,
            folded: true,
            root: false
        }
    );

    let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("a sink");
    assert!(!sink.on);
    assert_eq!(sink.op(), Op::Unfold(picture));
    assert_eq!(
        panel.op(sink.op()),
        Outcome::Folded {
            id: picture,
            folded: false,
            root: false
        }
    );
    assert!(
        outputs(&ctx, panel.layout(), Open::CLOSED)
            .expect("a sink")
            .on
    );
}

/// **Off and on again through the control restores the arrangement exactly** —
/// every rectangle of it, not the picture's alone.
///
/// This is
/// [ADR-0174](../../../docs/adr/0174-a-node-claims-only-what-its-visible-content-can-use.md)'s
/// own round trip, reached from the control instead of from the layout: the
/// manual promises that turning the sink off *"gives its height to the
/// inspector"*, and that it comes back because nothing was written down while
/// it was away. A control that folded through some path of its own — a stored
/// state, a second toggle — would be the way that stops being true.
#[test]
fn folding_through_the_dot_and_back_restores_the_arrangement() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    panel.solve();
    let before = rects(panel.layout());
    let inspector = rect_of(panel.layout(), "inspector").h;
    let previews = rect_of(panel.layout(), "deck-previews").h;

    // Off: the picture goes, and the inspector takes its height.
    let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("a sink");
    panel.op(sink.op());
    panel.solve();
    assert!(
        near(rect_of(panel.layout(), "program-view").h, 0.0),
        "the picture is still holding height with its sink off"
    );
    assert!(
        rect_of(panel.layout(), "inspector").h > inspector + 300.0,
        "the inspector did not take the picture's height: {} against {inspector}",
        rect_of(panel.layout(), "inspector").h
    );
    assert!(
        near(rect_of(panel.layout(), "deck-previews").h, previews),
        "the deck previews are auditions of their own and swelled when the picture went"
    );

    // On: exactly what it was, rectangle for rectangle.
    let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("a sink");
    panel.op(sink.op());
    panel.solve();
    assert_eq!(
        before,
        rects(panel.layout()),
        "the sink went off and on and the arrangement did not come back"
    );
}

/// **The control is drawn where the row can hold it, and nowhere else.**
///
/// `picture_rect`'s rule, stated on a control: a folded row has a rectangle
/// with no extent in it, so there is nothing to paint and nothing to press.
#[test]
fn a_folded_row_draws_no_control() {
    let ctx = drawn_once();
    let mut layout = solved(PLAUSIBLE);
    assert!(outputs(&ctx, &layout, Open::CLOSED).is_some());

    layout.collapse(id_of(&layout, "outputs"));
    layout.solve();
    assert_eq!(
        outputs(&ctx, &layout, Open::CLOSED),
        None,
        "the outputs row is folded away and its control is still being drawn"
    );

    layout.expand(id_of(&layout, "outputs"));
    layout.solve();
    assert!(outputs(&ctx, &layout, Open::CLOSED).is_some());

    // And a window too narrow for the chip, which the mock answers by wrapping
    // the row and this console does not: there is one sink, and where it does
    // not fit there is no control rather than half of one.
    let narrow = solved(Rect {
        w: 40.0,
        ..PLAUSIBLE
    });
    assert_eq!(outputs(&ctx, &narrow, Open::CLOSED), None);
}

/// The row's own boundary is still draggable where the control is not: the
/// clearance is a clearance for both, and a panel that gave the whole row to
/// the control would have taken the boundary away.
#[test]
fn the_boundary_above_the_row_is_still_the_panels() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let row = rect_of(panel.layout(), "outputs");
    let above = Point::new(row.x + row.w * 0.5, row.y - 2.0);
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), above), Claim::Panel);

    let root = panel.layout().root();
    let last = panel
        .layout()
        .boundaries()
        .filter(|(split, _)| *split == root)
        .last()
        .expect("the root has boundaries");
    let gap = panel.layout().boundary(last.0, last.1).expect("a pair");
    assert_eq!(
        Axis::Column.far(gap),
        row.y,
        "that is not the row's own gap"
    );
}

// ---------------------------------------------------------------------------
// A press is never a press that does nothing
// ---------------------------------------------------------------------------

/// **A press on a lit dot darkens it and a press on a dark dot lights it,
/// always.** The property, over every way the picture can be off screen.
///
/// This is the assertion that makes *the control never appears not to respond*
/// checkable rather than argued. `docs/manual/console.html`'s note on this row
/// is the standing position it comes from — *"Nothing is refused here, so
/// nothing has to be explained: a control that quietly declines the last of
/// something is a rule an operator can only find by experiment"* — and a
/// control that lights nothing when pressed is worse than one that declines,
/// because it does not even decline out loud.
///
/// The three folds are three different distances from the picture: itself, the
/// bay around it, and the whole centre column two levels up. An `Unfold` that
/// expanded its node alone passes none of them but the first.
#[test]
fn a_press_darkens_a_lit_dot_and_lights_a_dark_one_always() {
    for hidden_by in ["program-view", "program", "centre"] {
        let (mut panel, ctx) = console(PLAUSIBLE);
        let picture = panel.layout().find("program-view").expect("program-view");

        // Lit, and one press darkens it.
        let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("a sink");
        assert!(sink.on);
        panel.op(sink.op());
        assert!(
            !outputs(&ctx, panel.layout(), Open::CLOSED)
                .expect("a sink")
                .on,
            "a press on a lit dot left it lit"
        );
        panel.op(Op::UnfoldAll);

        // Now folded from somewhere else, and one press is all an operator
        // gets: the dot is what they pressed, and it has to answer.
        let by = panel.layout().find(hidden_by).expect("a region");
        panel.op(Op::Fold(by));
        let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("a sink");
        assert!(
            !sink.on,
            "the picture is off screen behind {hidden_by} and the dot is lit"
        );
        panel.op(sink.op());
        assert!(
            outputs(&ctx, panel.layout(), Open::CLOSED)
                .expect("a sink")
                .on,
            "one press on the dark dot did not light it: the picture is folded behind \
             {hidden_by} and the operator has pressed the only control there is"
        );
        assert!(
            panel.layout().visible(picture),
            "the dot reads on and the picture is not on screen behind {hidden_by}"
        );
    }
}

/// **The bay folded around the picture comes back with one press**, stated on
/// its own because it is the case that prompted the rule: `g` over the picture
/// folds the Program bay, which is the fold an operator makes without thinking
/// about which node it landed on.
#[test]
fn the_bay_folded_around_the_picture_comes_back_with_one_press() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let picture = panel.layout().find("program-view").expect("program-view");
    let bay = panel.layout().find("program").expect("program");

    panel.op(Op::FoldEnclosing(picture));
    assert!(!panel.layout().visible(bay), "the bay did not fold");
    let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("a sink");
    assert!(!sink.on);
    assert_eq!(sink.op(), Op::Unfold(picture));

    panel.op(sink.op());
    panel.solve();
    assert!(
        panel.layout().visible(picture),
        "the picture is still off screen"
    );
    assert!(
        panel.layout().visible(bay),
        "the picture was expanded inside a bay that is still folded, so the press \
         lit nothing"
    );
    assert!(
        outputs(&ctx, panel.layout(), Open::CLOSED)
            .expect("a sink")
            .on
    );
    // And the picture has its height back, which is what *on screen* means.
    assert!(near(rect_of(panel.layout(), "program").h, 378.0));
}

/// **A solo that is hiding the picture is dropped by the dot, and one that is
/// not is left alone.**
///
/// Reachable from the control, and only one way: a solo on the outputs row
/// itself leaves this row holding the window with its sink drawn and dark,
/// because `Layout::solo` collapses everything off the solo's path. Every
/// other solo either leaves the picture on screen or takes the outputs row off
/// it, and a row that is not drawn has no control to press.
///
/// **Dropping it is the honest operation and not a workaround.** `Layout` has
/// no invariant tying `soloed` to the collapsed flags — `check_structure` asks
/// only that a recorded solo addresses a node — so expanding a path under a
/// live solo is a state the layout accepts, solves and reloads. What it is not
/// is a state anyone can reason about: `soloed` would name a region that is no
/// longer the only one on screen, and `unsolo` restores the flags the solo
/// replaced, which would silently throw away the expand the operator just
/// asked for.
#[test]
fn a_solo_hiding_the_picture_is_dropped_and_one_that_is_not_is_kept() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let picture = panel.layout().find("program-view").expect("program-view");
    let row = panel.layout().find("outputs").expect("outputs");

    // The one solo that leaves this control on screen with the picture off it.
    panel.op(Op::Solo(row));
    assert!(panel.layout().is_soloed());
    let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("the soloed row draws its sink");
    assert!(
        !sink.on,
        "everything but this row is folded and the dot is lit"
    );

    panel.op(sink.op());
    panel.solve();
    assert!(
        !panel.layout().is_soloed(),
        "the solo was what was hiding the picture and it is still in force"
    );
    assert!(
        panel.layout().visible(picture),
        "the picture is still off screen"
    );
    assert!(
        outputs(&ctx, panel.layout(), Open::CLOSED)
            .expect("a sink")
            .on
    );

    // **And the ordering, which is the case that decides it.** Fold the
    // picture, *then* solo this row: `unsolo` restores the flags the solo
    // replaced, and one of them is the fold the operator is now asking to
    // undo. Undo the solo first and expand after, and the dot lights; expand
    // first and undo the solo after, and the restore puts the fold straight
    // back and the press did nothing at all.
    let (mut panel, ctx) = console(PLAUSIBLE);
    let picture = panel.layout().find("program-view").expect("program-view");
    let row = panel.layout().find("outputs").expect("outputs");
    panel.op(Op::Fold(picture));
    panel.op(Op::Solo(row));
    let sink = outputs(&ctx, panel.layout(), Open::CLOSED).expect("the soloed row draws its sink");
    assert!(!sink.on);
    panel.op(sink.op());
    assert!(
        outputs(&ctx, panel.layout(), Open::CLOSED)
            .expect("a sink")
            .on,
        "the picture was folded before the solo, and undoing the solo put that fold \
         back over the press that was undoing it"
    );
    assert!(!panel.layout().is_soloed());

    // And the other direction: a solo that already has the picture on screen
    // is not this operation's business. It is not reachable from the control —
    // the outputs row is folded away under it, so there is no dot — so it is
    // asked of the operation by name, which is what a named operation is for.
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let picture = panel.layout().find("program-view").expect("program-view");
    panel.op(Op::Solo(panel.layout().find("program").expect("program")));
    assert!(panel.layout().visible(picture));
    assert_eq!(
        outputs(&ctx, panel.layout(), Open::CLOSED),
        None,
        "the outputs row is soloed away"
    );
    panel.op(Op::Unfold(picture));
    assert!(
        panel.layout().is_soloed(),
        "an unfold of something already on screen dropped a solo it had no quarrel with"
    );
}
