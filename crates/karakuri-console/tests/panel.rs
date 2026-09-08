//! The panel under a pointer: a drag, a fold, a solo, and every hostile thing
//! a window can do to them.
//!
//! These drove `examples/layout.rs` when the model lived inside it. The model
//! is [`karakuri_console::panel`] now, so they are ordinary tests of the crate
//! and `cargo test -p karakuri-console` runs them — no event loop, no device,
//! and nothing built that needs either.
//!
//! **They assert on what an operation returned**, which is the point of an
//! operation returning a value rather than a line: a stop holding a drag is
//! `Dragged::held`, and a boundary that had nothing to say is a `None` rather
//! than a string that was not printed.

mod common;

use common::EPS;
use karakuri_console::panel::{Dragged, Op, Outcome, Panel, Pressed};
use karakuri_layout::{Axis, Hit, NodeId, Point, Rect};

/// Every rectangle in the arrangement, in tree order — what "the same
/// arrangement" is compared as.
fn rects(p: &mut Panel) -> Vec<Rect> {
    p.solve();
    common::rects(p.layout())
}

/// Every boundary in the arrangement, as `Layout` enumerates them.
fn dividers(p: &Panel) -> Vec<(NodeId, usize)> {
    p.layout().boundaries().collect()
}

/// Where a boundary is, along its split's axis: the near edge of the gap
/// `Layout::boundary` hands back.
fn boundary(p: &Panel, split: NodeId, index: usize) -> Option<f32> {
    let axis = p.layout().axis(split)?;
    Some(axis.origin(p.layout().boundary(split, index)?))
}

/// A point in the middle of a boundary's gap — what a hand aims at, and what
/// a test with no hand presses instead. **A test affordance**, which is why it
/// is here: the panel does not need it and neither does a view, since a view
/// has a pointer and gets the gap to draw from `Layout::boundary`.
fn grab_point(p: &Panel, split: NodeId, index: usize) -> Option<Point> {
    let gap = p.layout().boundary(split, index)?;
    Some(Point::new(gap.x + gap.w / 2.0, gap.y + gap.h / 2.0))
}

fn offset(axis: Axis, p: Point, by: f32) -> Point {
    match axis {
        Axis::Row => Point::new(p.x + by, p.y),
        Axis::Column => Point::new(p.x, p.y + by),
    }
}

fn same(a: &[Rect], b: &[Rect]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            (x.x - y.x).abs() <= EPS
                && (x.y - y.y).abs() <= EPS
                && (x.w - y.w).abs() <= EPS
                && (x.h - y.h).abs() <= EPS
        })
}

/// A name for a failure message. A split is often unnamed, and a failure that
/// says which one is worth more than one that does not.
fn label(p: &Panel, id: NodeId) -> String {
    match p.layout().name(id) {
        Some(name) => name.to_owned(),
        None => "(unnamed split)".to_owned(),
    }
}

/// **Whether a boundary is one a drag can fold a pane at** — a region beside
/// it whose fold leaves its edge behind (ADR-0300).
///
/// The two tests below drag every boundary far past every stop, which is the
/// gesture that closes such a pane; what they are about is a boundary that
/// *moves*, so those two are left to `tests/fold_grip.rs`, where the fold is
/// the subject rather than the accident. Read off the arrangement rather than
/// listed, so a third pane declaring it is skipped here without anybody
/// editing this file.
fn folds_a_pane(p: &Panel, split: NodeId, index: usize) -> bool {
    match p.pair(split, index) {
        Some((a, b)) => p.layout().keeps_its_edge(a) || p.layout().keeps_its_edge(b),
        None => false,
    }
}

/// Every divider in the arrangement, dragged and dragged back — including far
/// past whatever stops it — leaves the arrangement exactly as it was, and at
/// least one of them moves on the way.
///
/// The second half is the point: `set_divider` takes an absolute coordinate,
/// so the frames spent past a stop contribute nothing to accumulate. A caller
/// that fed it deltas would come back short.
///
/// **A boundary beside a pane that keeps its edge is not one of these, and
/// that is a change rather than an exemption.** A drag far past that pane's
/// own minimum closes it, and bringing the pointer back does not open it: one
/// gesture asks for one fold, and the way back is another gesture
/// (ADR-0300). `tests/fold_grip.rs` is where both halves are demonstrated.
#[test]
fn a_drag_out_and_back_leaves_the_arrangement_where_it_was() {
    let mut probe = Panel::new(1600.0, 1000.0);
    probe.solve();
    let dividers = dividers(&probe);
    assert!(!dividers.is_empty(), "the arrangement has no dividers");
    let total = dividers.len();

    let mut moved = 0;
    let mut grabbed = 0;
    for (split, index) in dividers {
        let mut p = Panel::new(1600.0, 1000.0);
        p.solve();
        let before = rects(&mut p);
        let axis = p.layout().axis(split).expect("a divider is on a split");
        let start = boundary(&p, split, index).expect("a boundary");
        let point = grab_point(&p, split, index).expect("a gap to aim at");

        match p.press(point) {
            // Reported rather than asserted: a divider a pointer cannot reach
            // is a finding about `hit`, not about the drag.
            Pressed::Grabbed {
                split: s,
                index: i,
                axis: a,
                at,
                offset: o,
            } => {
                assert_eq!((s, i), (split, index));
                assert_eq!(a, axis);
                assert!(
                    (at - start).abs() <= EPS,
                    "the press reported {at}, not {start}"
                );
                // The grab point is the middle of the gap, so the pointer took
                // hold half a divider past the boundary.
                let divider = p.layout().divider(split).expect("a split has a divider");
                assert!(
                    (o - divider / 2.0).abs() <= EPS,
                    "the press reported an offset of {o} into a {divider} gap"
                );
                grabbed += 1;
            }
            _ => continue,
        }
        if folds_a_pane(&p, split, index) {
            continue;
        }

        let _ = p.moved(offset(axis, point, 40.0));
        if (boundary(&p, split, index).expect("a boundary") - start).abs() > EPS {
            moved += 1;
            assert!(
                !same(&before, &rects(&mut p)),
                "the boundary moved and no rectangle changed"
            );
        }

        // Past every stop there is, in both directions, and back to the exact
        // pointer position the drag began at.
        let _ = p.moved(offset(axis, point, 9000.0));
        let _ = p.moved(offset(axis, point, -9000.0));
        let _ = p.moved(point);
        p.released(None);

        let after = rects(&mut p);
        assert!(
            same(&before, &after),
            "divider {}#{index} did not come back: {:?} against {:?}",
            label(&p, split),
            before,
            after
        );
    }
    assert_eq!(
        grabbed, total,
        "a boundary the arrangement has that a pointer cannot grab"
    );
    // Two of the console's boundaries cannot move at all — a region whose
    // minimum meets its maximum is pinned — so this is "at least one", not
    // "all of them", and the restore above is what holds for every one.
    assert!(moved > 0, "no divider moved under a 40px drag");
}

/// A drag that runs off the left edge of the window. winit reports negative
/// pointer coordinates there, and the operator dragged into them.
#[test]
fn a_drag_past_the_left_edge_of_the_window() {
    let mut probe = Panel::new(1920.0, 1080.0);
    probe.solve();
    for (split, index) in dividers(&probe) {
        let mut p = Panel::new(1920.0, 1080.0);
        p.solve();
        let axis = p.layout().axis(split).expect("a divider is on a split");
        let point = grab_point(&p, split, index).expect("a gap to aim at");
        p.press(point);
        for to in [-40.0, -200.0, -1000.0, -5000.0] {
            let at = match axis {
                Axis::Row => Point::new(to, point.y),
                Axis::Column => Point::new(point.x, to),
            };
            let _ = p.moved(at);
        }
        p.released(None);
    }
}

/// A pointer dragged on past a stop says so once.
///
/// This is the readout's own bug, and it is worst exactly where a person is
/// looking hardest: at a stop the boundary does not move, and a report per
/// pointer event is hundreds of identical lines a second into a terminal that
/// has to keep up with them. So the first move past the stop comes back
/// `held`, and the two hundred after it come back with nothing to say at all.
///
/// **A boundary beside a pane that keeps its edge answers the first move with
/// a fold instead** (ADR-0300): pulling on past the stop is what closes the
/// pane, so what the drag has to say is *that* rather than *held*. The
/// property under test is the same one either way and the two hundred moves
/// after it are what it is about — the pane is closed, its boundary does not
/// move again, and one gesture asks for one fold.
#[test]
fn a_drag_held_at_a_stop_says_so_once() {
    let mut probe = Panel::new(1600.0, 1000.0);
    probe.solve();
    let mut stops = 0;
    for (split, index) in dividers(&probe) {
        let mut p = Panel::new(1600.0, 1000.0);
        p.solve();
        let axis = p.layout().axis(split).expect("a divider is on a split");
        let point = grab_point(&p, split, index).expect("a gap to aim at");
        p.press(point);

        // Well past whatever stops it, and then on, a pixel at a time, the way
        // a hand held against the edge of the window does.
        let held = p.moved(offset(axis, point, -9000.0));
        assert!(
            matches!(&held, Some(Dragged::Boundary { held, .. }) if held.is_some())
                || matches!(&held, Some(Dragged::Pane(Op::Fold(_)))),
            "a drag 9000px past every stop did not report one holding it, or the fold it \
             performed instead: {held:?}"
        );
        let at = boundary(&p, split, index).expect("a boundary");
        let mut said = 0;
        for step in 1..=200 {
            if p.moved(offset(axis, point, -9000.0 - step as f32))
                .is_some()
            {
                said += 1;
            }
        }
        let still = boundary(&p, split, index).expect("a boundary");
        assert!(
            (still - at).abs() <= EPS,
            "the boundary was supposed to be against a stop and moved"
        );
        assert_eq!(
            said, 0,
            "a boundary that did not move said something 200 times over"
        );
        stops += 1;
    }
    assert!(stops > 0, "no divider was driven against a stop");
}

/// A sweep of everything a window can do to the panel, at viewports from
/// comfortable down to below the arrangement's own minima, with the pointer
/// walked well outside each of them.
#[test]
fn a_sweep_of_hostile_input() {
    for (vw, vh) in [
        (1920.0_f32, 1080.0_f32),
        (1440.0, 900.0),
        (990.0, 632.0),
        (400.0, 300.0),
        (1.0, 1.0),
    ] {
        let mut probe = Panel::new(vw, vh);
        probe.solve();
        for (split, index) in dividers(&probe) {
            let mut p = Panel::new(vw, vh);
            p.solve();
            let Some(point) = grab_point(&p, split, index) else {
                continue;
            };
            p.press(point);
            for to in [
                Point::new(-1.0, -1.0),
                Point::new(-4000.0, point.y),
                Point::new(point.x, -4000.0),
                Point::new(vw + 4000.0, vh + 4000.0),
                Point::new(0.0, 0.0),
                Point::new(f32::MAX, f32::MAX),
                point,
            ] {
                let _ = p.moved(to);
                // **This loop used to be a list of seven `Op`s** and it
                // compiled only because an operation meant *whatever is under
                // the pointer*: the hostile part of it was that the pointer
                // was at `f32::MAX`, and the model did the resolving. The
                // resolution is the caller's now, so the sweep does it — which
                // is the same hostility, with the failing resolution visible
                // rather than swallowed into an `Outcome::Nothing`.
                let target = match p.under() {
                    Hit::View(id) => Some(id),
                    Hit::Divider { split, .. } => Some(split),
                    Hit::Nothing => None,
                };
                for op in target
                    .into_iter()
                    .flat_map(|id| {
                        [
                            Op::Fold(id),
                            Op::Unfold(id),
                            Op::FoldEnclosing(id),
                            Op::Solo(id),
                        ]
                    })
                    .chain([Op::Report, Op::Unsolo, Op::UnfoldAll])
                {
                    p.op(op);
                }
                // A resize in the middle of a drag, which a window can do.
                p.set_viewport(vw / 2.0, vh / 2.0);
                let _ = p.moved(to);
                p.set_viewport(0.0, 0.0);
                let _ = p.moved(to);
                p.set_viewport(vw, vh);
            }
            p.released(None);
            // A release with nothing in hand, and a press outside.
            assert_eq!(
                p.released(None),
                None,
                "a release let go of something twice"
            );
            p.press(Point::new(-10.0, -10.0));
            let _ = p.moved(Point::new(-10.0, -10.0));
            p.op(Op::Reset);
        }
    }
}

/// Every operation, in a debug build, with a read after each one.
///
/// `rect()` and `hit()` both `debug_assert!` that the layout is not dirty, so
/// this fails if any operation here leaves a solve owed — which is the mistake
/// a real view will make first, and the reason the panel solves at the top of
/// everything that reads.
///
/// It also asserts what each operation is *for*, and asserts it twice over:
/// once on the rectangles, and once on what the operation said it did. A fold
/// folds, an unfold puts the arrangement back exactly and names what it
/// unfolded, a solo leaves one region visible, and an unsolo restores what it
/// replaced.
#[test]
fn every_operation_leaves_the_layout_readable_and_undoes_exactly() {
    let mut p = Panel::new(1600.0, 1000.0);
    p.solve();
    let before = rects(&mut p);

    // Name-free: the pointer goes to the middle of the first leaf big enough
    // to aim at.
    let leaf = p
        .nodes()
        .iter()
        .filter(|n| p.layout().is_view(n.id))
        .map(|n| (n.id, p.layout().rect(n.id)))
        .find(|(_, r)| r.w > 20.0 && r.h > 20.0)
        .expect("a leaf to point at");
    p.set_cursor(Point::new(
        leaf.1.x + leaf.1.w / 2.0,
        leaf.1.y + leaf.1.h / 2.0,
    ));

    let folded = |p: &Panel| {
        p.nodes()
            .iter()
            .filter(|n| p.layout().is_collapsed(n.id))
            .count()
    };

    // **The pointer resolves to a target and the operation names it.** The
    // cursor is set above and `under` is what a surface asks; the assertion is
    // the same one it always was — `f` folds the region under the pointer —
    // with the two halves of that sentence now in the two places they belong.
    assert_eq!(p.under(), Hit::View(leaf.0), "the pointer lost its region");
    assert_eq!(
        p.op(Op::Fold(leaf.0)),
        Outcome::Folded {
            id: leaf.0,
            folded: true,
            root: false
        },
        "f folded something other than the region under the pointer"
    );
    assert_eq!(folded(&p), 1, "f folded something other than one region");
    assert_eq!(
        p.op(Op::UnfoldAll),
        Outcome::Unfolded(vec![leaf.0]),
        "z unfolded something other than what f folded"
    );
    assert_eq!(folded(&p), 0);
    assert!(same(&before, &rects(&mut p)), "an unfold did not restore");

    let enclosing = p.layout().parent(leaf.0).expect("a leaf is inside a split");
    // Whether that split is the root is the arrangement's business, not this
    // test's — the first leaf big enough to aim at is the transport, and the
    // transport's parent happens to be the root, which is exactly the case
    // the outcome carries a `root` flag for.
    let is_root = enclosing == p.layout().root();
    assert_eq!(
        p.op(Op::FoldEnclosing(leaf.0)),
        Outcome::Folded {
            id: enclosing,
            folded: true,
            root: is_root
        },
        "g folded something other than the split enclosing the pointer"
    );
    assert!(folded(&p) > 0, "g folded nothing");
    p.op(Op::UnfoldAll);
    assert!(same(&before, &rects(&mut p)), "an unfold did not restore");

    assert_eq!(p.op(Op::Solo(leaf.0)), Outcome::Soloed(leaf.0));
    assert!(p.layout().is_soloed());
    assert!(
        p.layout().visible(leaf.0),
        "a solo folded the region it was aimed at"
    );
    // Straight into an operation that reads every rectangle, with nothing
    // solving in between: this is the pair that catches a stale solve, and it
    // fails on `rect() read a stale solve` if `op` stops solving.
    let Outcome::Report(rows) = p.op(Op::Report) else {
        panic!("p did not report");
    };
    assert_eq!(rows.len(), p.nodes().len(), "the report skipped a node");
    assert_eq!(
        p.op(Op::Unsolo),
        Outcome::Unsoloed { was: true },
        "an unsolo after a solo did not know there had been one"
    );
    assert!(!p.layout().is_soloed());
    assert!(same(&before, &rects(&mut p)), "an unsolo did not restore");
    assert_eq!(
        p.op(Op::Unsolo),
        Outcome::Unsoloed { was: false },
        "a second unsolo claimed there was still a solo to undo"
    );

    // A drag, then a reset: the arrangement is fresh, at the same viewport.
    let (split, index) = dividers(&p)[0];
    let point = grab_point(&p, split, index).expect("a gap to aim at");
    let axis = p.layout().axis(split).expect("a divider is on a split");
    p.press(point);
    let _ = p.moved(offset(axis, point, 60.0));
    p.released(None);
    assert_eq!(p.op(Op::Reset), Outcome::Reset);
    assert!(same(&before, &rects(&mut p)), "a reset did not restore");
}

/// A drag lands where it is asked unless something stops it, and what it
/// reports is what actually happened — the pair either side keeps its combined
/// extent whatever was asked for, and a stop that held it is `held` rather
/// than a difference the caller has to work out.
#[test]
fn a_drag_reports_where_it_landed_and_moves_only_the_pair() {
    let mut probe = Panel::new(1600.0, 1000.0);
    probe.solve();
    let mut checked = 0;
    for (split, index) in dividers(&probe) {
        let mut p = Panel::new(1600.0, 1000.0);
        p.solve();
        let axis = p.layout().axis(split).expect("a divider is on a split");
        let (a, b) = p.pair(split, index).expect("a boundary has a pair");
        let span = axis.extent(p.layout().rect(a)) + axis.extent(p.layout().rect(b));
        let siblings: Vec<NodeId> = p
            .layout()
            .placed_children(split)
            .filter(|c| *c != a && *c != b)
            .collect();
        let others: Vec<Rect> = siblings.iter().map(|c| p.layout().rect(*c)).collect();

        let point = grab_point(&p, split, index).expect("a gap to aim at");
        if !matches!(p.press(point), Pressed::Grabbed { .. }) {
            continue;
        }
        // A drag 9000px out through a pane's own edge closes the pane rather
        // than moving the boundary, which is `tests/fold_grip.rs`'s subject
        // and not this one's — see `folds_a_pane`.
        if folds_a_pane(&p, split, index) {
            continue;
        }
        checked += 1;
        let dragged = p
            .moved(offset(axis, point, 9000.0))
            .expect("a drag 9000px out has something to report");
        let Dragged::Boundary {
            asked,
            landed: said,
            held,
            ..
        } = dragged
        else {
            panic!("a drag on a boundary reported something else: {dragged:?}")
        };

        // What the drag said, against what the layout did.
        let landed = boundary(&p, split, index).expect("a boundary");
        assert!(
            (said - landed).abs() <= EPS,
            "the drag reported landing at {said} and the boundary is at {landed}"
        );
        assert_eq!(
            held,
            Some(said - asked),
            "a drag 9000px out was not reported as held by a stop"
        );
        assert!(said < asked, "a drag out landed past what it asked for");

        let after: Vec<Rect> = siblings.iter().map(|c| p.layout().rect(*c)).collect();
        assert!(
            same(&others, &after),
            "a drag moved a sibling that is not either side of it"
        );
        let now = axis.extent(p.layout().rect(a)) + axis.extent(p.layout().rect(b));
        assert!(
            (now - span).abs() <= EPS,
            "the pair's combined extent changed: {span} to {now}"
        );
        let (min, max) = p.layout().bounds(a);
        let took = landed - axis.origin(p.layout().rect(a));
        assert!(
            took <= max + EPS && took >= min - EPS,
            "a drag landed outside the bounds it was stopped by"
        );
        // The boundary is the far edge of the child before it, which is what
        // every index in this file is counting on.
        assert!(
            (axis.far(p.layout().rect(a)) - landed).abs() <= EPS,
            "the boundary is not the far edge of the child before it"
        );
    }
    assert!(
        checked >= 4,
        "only {checked} dividers could be grabbed and dragged without folding a pane"
    );
}

/// A fold during a drag takes the boundary away, and the release says so.
///
/// [`Released::Gone`] has existed for exactly this since the model was
/// written, and it was **unreachable**: where a boundary is was answered by
/// checking that the visible child before it was there and returning that
/// child's far edge, so folding the far side handed back a position — the
/// split's own far edge — for a boundary that is not there. A release then
/// rested on a coordinate that is not a boundary and the readout said so.
///
/// The route is the one an operator takes: press in a gap, move the pointer
/// into the region on the far side of it, and fold that region from the
/// keyboard while the button is still down.
#[test]
fn a_fold_during_a_drag_leaves_the_release_with_no_boundary() {
    let mut p = Panel::new(1600.0, 1000.0);
    p.solve();

    // The root's last boundary, so the region on the far side of it is the
    // last one and the split's own far edge is what the defect reached for.
    let root = p.layout().root();
    let (split, index) = *dividers(&p)
        .iter()
        .rfind(|(s, _)| *s == root)
        .expect("the root has boundaries");
    let (_, far_side) = p.pair(split, index).expect("a boundary has a pair");
    let point = grab_point(&p, split, index).expect("a gap to aim at");

    assert!(
        matches!(p.press(point), Pressed::Grabbed { .. }),
        "the boundary was not taken in hand"
    );

    // Into the region beyond the boundary. The drag follows the pointer and
    // the boundary does not move — that region's minimum meets its maximum —
    // which is what leaves the pointer inside it rather than ahead of it.
    let inside = p.layout().rect(far_side);
    let cursor = Point::new(inside.x + inside.w / 2.0, inside.y + inside.h / 2.0);
    let _ = p.moved(cursor);
    assert_eq!(
        p.under(),
        Hit::View(far_side),
        "the pointer is not in the region beyond the boundary"
    );
    assert_eq!(
        p.op(Op::Fold(far_side)),
        Outcome::Folded {
            id: far_side,
            folded: true,
            root: false
        },
        "the fold did not take the far side of the boundary"
    );

    assert_eq!(
        p.released(None),
        Some(karakuri_console::panel::Released::Gone { split, index }),
        "the boundary is gone and the release rested on something"
    );
}

/// **With the root folded, the panel is blank and answers no pointer — and
/// the way back was never the pointer's.**
///
/// `g` over the transport folds the split enclosing it, which is the root, and
/// what that leaves is a window with nothing drawn in it: a view's plan skips
/// every node `Layout::visible` says no to, and `visible` walks up to the
/// root. The hit test used to disagree with the plan — it descends from the
/// root testing *children* — so `f`, `g` and `s` went on folding and soloing
/// regions nobody could see.
///
/// What the change leaves working is the whole of what an operator needs, and
/// it is what it always was: **`z` and `r` take no target.** `UnfoldAll` reads
/// the arrangement and `Reset` builds a fresh one, so neither asks where the
/// pointer is — which is the manual's own sentence, *"a folded region has no
/// rectangle, so a pointer cannot reach it to undo itself"*, one node further
/// up than it was written for.
#[test]
fn a_folded_root_answers_no_pointer_and_z_and_r_are_still_the_way_back() {
    let mut p = Panel::new(1600.0, 1000.0);
    p.solve();
    let before = rects(&mut p);
    let root = p.layout().root();
    let middle = Point::new(800.0, 500.0);

    p.set_cursor(middle);
    let under = p.under();
    assert!(
        matches!(under, Hit::View(_)),
        "the middle of the window is not a region: {under:?}"
    );

    assert_eq!(
        p.op(Op::Fold(root)),
        Outcome::Folded {
            id: root,
            folded: true,
            root: true
        }
    );
    p.set_cursor(middle);
    assert_eq!(
        p.under(),
        Hit::Nothing,
        "the panel is empty and the pointer still named a region to fold"
    );
    assert_eq!(
        p.press(middle),
        Pressed::Nothing,
        "a press on an empty panel found something to take hold of"
    );

    // `z`: no target, so the fold that hid everything is undone by the one
    // operation a hand can still reach.
    assert_eq!(
        p.op(Op::UnfoldAll),
        Outcome::Unfolded(vec![root]),
        "z did not unfold the root, which is the fold nothing else can reach"
    );
    assert!(
        same(&before, &rects(&mut p)),
        "z did not bring the panel back"
    );
    p.set_cursor(middle);
    assert_eq!(p.under(), under, "z brought back a different arrangement");

    // `r`: the other one, and it does not read the arrangement at all.
    p.op(Op::Fold(root));
    p.set_cursor(middle);
    assert_eq!(p.under(), Hit::Nothing);
    assert_eq!(p.op(Op::Reset), Outcome::Reset);
    assert!(
        same(&before, &rects(&mut p)),
        "r did not bring the panel back"
    );
    p.set_cursor(middle);
    assert_eq!(p.under(), under, "r built a different arrangement");
}
