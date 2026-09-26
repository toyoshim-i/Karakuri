//! Test fixtures and layout invariant verification helpers.

#![allow(dead_code)] // Each test file uses a subset.

use karakuri_layout::{Axis, Layout, NodeId, Rect, Spec};

/// Float comparison epsilon for pixel coordinates.
pub const EPS: f32 = 1e-3;

pub fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= EPS
}

/// A row of three: a fixed side pane with both bounds, a flexible centre, and a
/// fixed side pane with only a minimum.
///
/// At 1000 wide with 2-thick dividers: `left` 100, `centre` 836, `right` 60.
pub fn simple() -> Layout {
    Layout::new(Spec::row(
        2.0,
        vec![
            Spec::view("left").fixed(100.0).min(50.0).max(150.0),
            Spec::view("centre").flex(1.0).min(80.0),
            Spec::view("right").fixed(60.0).min(30.0),
        ],
    ))
}

/// A column of two flexible views — the shape a pane's stack of views has, and
/// the one where a drag rewrites weights rather than sizes.
pub fn stack() -> Layout {
    Layout::new(Spec::column(
        6.0,
        vec![
            Spec::view("upper").flex(2.0).min(40.0),
            Spec::view("lower").flex(1.0).min(30.0),
        ],
    ))
}

/// Returns the standard console layout tree (unnamed splits).
pub fn console() -> Layout {
    Layout::new(Spec::column(
        4.0,
        vec![
            Spec::view("transport").fixed(48.0).min(48.0).max(48.0),
            Spec::row(
                4.0,
                vec![
                    Spec::column(
                        4.0,
                        vec![
                            Spec::view("library").flex(2.0).min(80.0),
                            Spec::view("staging").flex(1.0).min(60.0),
                        ],
                    )
                    .fixed(240.0)
                    .min(160.0)
                    .max(400.0),
                    Spec::column(
                        4.0,
                        vec![
                            Spec::view("program").flex(1.0).min(120.0),
                            Spec::view("inspector").flex(1.0).min(100.0),
                        ],
                    )
                    .flex(1.0)
                    .min(320.0),
                    Spec::column(
                        4.0,
                        vec![
                            Spec::view("mixer").flex(1.0).min(80.0),
                            Spec::view("master").flex(1.0).min(60.0),
                        ],
                    )
                    .fixed(320.0)
                    .min(220.0)
                    .max(480.0),
                ],
            )
            .flex(1.0)
            .min(200.0),
            Spec::view("status").fixed(24.0).min(24.0).max(24.0),
        ],
    ))
}

/// Returns the standard console layout tree with named split containers.
pub fn named_console() -> Layout {
    Layout::new(Spec::column(
        4.0,
        vec![
            Spec::view("transport").fixed(48.0).min(48.0).max(48.0),
            Spec::row(
                4.0,
                vec![
                    Spec::column(
                        4.0,
                        vec![
                            Spec::view("library").flex(2.0).min(80.0),
                            Spec::view("staging").flex(1.0).min(60.0),
                        ],
                    )
                    .named("left-pane")
                    .fixed(240.0)
                    .min(160.0)
                    .max(400.0),
                    Spec::column(
                        4.0,
                        vec![
                            Spec::view("program").flex(1.0).min(120.0),
                            Spec::view("inspector").flex(1.0).min(100.0),
                        ],
                    )
                    .named("centre")
                    .flex(1.0)
                    .min(320.0),
                    Spec::column(
                        4.0,
                        vec![
                            Spec::view("mixer").flex(1.0).min(80.0),
                            Spec::view("master").flex(1.0).min(60.0),
                        ],
                    )
                    .named("right-pane")
                    .fixed(320.0)
                    .min(220.0)
                    .max(480.0),
                ],
            )
            .named("panes")
            .flex(1.0)
            .min(200.0),
            Spec::view("status").fixed(24.0).min(24.0).max(24.0),
        ],
    ))
}

/// Node handles for primary pane splits within the console layout.
pub struct Console {
    pub panes: NodeId,
    pub left: NodeId,
    pub centre: NodeId,
    pub right: NodeId,
}

pub fn console_ids(l: &Layout) -> Console {
    let panes = l.children(l.root())[1];
    let c = l.children(panes);
    Console {
        panes,
        left: c[0],
        centre: c[1],
        right: c[2],
    }
}

/// Every rectangle in the layout, in arena order — what "the same arrangement"
/// is compared as.
pub fn rects(l: &Layout) -> Vec<Rect> {
    let mut out = Vec::new();
    collect(l, l.root(), &mut out);
    out
}

fn collect(l: &Layout, id: NodeId, out: &mut Vec<Rect>) {
    out.push(l.rect(id));
    for c in l.children(id) {
        collect(l, *c, out);
    }
}

/// Verifies core geometric invariants (non-negative bounds, viewport containment, tiling consistency).
pub fn assert_invariants(l: &Layout) {
    check(l, l.root());
}

fn check(l: &Layout, id: NodeId) {
    let r = l.rect(id);

    // A rectangle is never negative.
    assert!(
        r.w >= 0.0 && r.h >= 0.0,
        "negative rectangle {r:?} at {id:?}"
    );

    // A visible node lies inside the viewport.
    if l.visible(id) {
        let v = l.viewport();
        assert!(
            r.x >= v.x - EPS
                && r.y >= v.y - EPS
                && r.x + r.w <= v.x + v.w + EPS
                && r.y + r.h <= v.y + v.h + EPS,
            "{r:?} at {id:?} escapes viewport {v:?}"
        );
    }

    if let Some(axis) = l.axis(id) {
        assert_tiles(l, id, axis);
        for c in l.children(id) {
            check(l, *c);
        }
    }
}

/// Verifies that placed children tile their parent without overlap, accounting for divider gaps.
fn assert_tiles(l: &Layout, id: NodeId, axis: Axis) {
    let parent = l.rect(id);
    let declared = l.divider(id).unwrap();
    let (origin, extent) = along(axis, parent);

    let mut gaps: Vec<f32> = Vec::new();
    let mut cursor = origin;
    let mut first = true;
    let mut visible = 0;

    for c in l.children(id) {
        let r = l.rect(*c);
        let (o, e) = along(axis, r);

        // Across the axis, a child is exactly its parent.
        let (po, pe) = across(axis, parent);
        let (co, ce) = across(axis, r);
        assert!(
            near(po, co) && near(pe, ce),
            "child {c:?} does not span its parent across the axis: {r:?} in {parent:?}"
        );

        if !laid_out(l, *c) && !l.is_retained(*c) {
            // A child that is out of the layout takes no space at all.
            assert!(near(e, 0.0), "child {c:?} is out and took {e} of extent");
        }
        if !l.is_placed(*c) {
            continue;
        }
        visible += 1;

        if first {
            assert!(
                near(o, origin),
                "first visible child starts at {o}, not at the parent's {origin}"
            );
            first = false;
        } else {
            // No overlap, and whatever separates them is the divider.
            assert!(o >= cursor - EPS, "child {c:?} overlaps its predecessor");
            gaps.push(o - cursor);
        }
        cursor = o + e;
    }

    if visible == 0 {
        return;
    }

    // Every gap is the same, none is wider than the declared divider, and where
    // the split can afford them they are exactly it. No divider is drawn beside
    // a child that is out of the layout, which is what makes this count
    // `visible - 1`.
    assert_eq!(
        gaps.len(),
        visible - 1,
        "wrong number of dividers in {id:?}"
    );
    let affordable = declared * gaps.len() as f32 <= extent;
    for g in &gaps {
        assert!(near(*g, gaps[0]), "dividers in {id:?} disagree: {gaps:?}");
        assert!(
            *g <= declared + EPS,
            "divider {g} exceeds the declared {declared}"
        );
        if affordable {
            assert!(
                near(*g, declared),
                "divider {g} is not the declared {declared}"
            );
        }
    }

    // And the whole of the parent is accounted for — with the one exception
    // the solve states: where every visible child is already at its maximum,
    // what is left over is trailing space rather than an overflow onto someone
    // who said they did not want it.
    if !near(cursor, origin + extent) {
        assert!(
            cursor < origin + extent,
            "children of {id:?} overflow their parent, ending at {cursor} past {}",
            origin + extent
        );
        for c in l.children(id) {
            if !laid_out(l, *c) {
                continue;
            }
            let (_, max) = l.bounds(*c);
            let (_, e) = along(axis, l.rect(*c));
            assert!(
                near(e, max),
                "children of {id:?} end at {cursor} rather than {}, and {c:?} is at {e} \
                 with room to its maximum of {max}",
                origin + extent
            );
        }
    }
}

/// Returns true if `id` is active in layout (neither collapsed nor set aside).
pub fn laid_out(l: &Layout, id: NodeId) -> bool {
    !l.is_collapsed(id) && !l.is_set_aside(id)
}

fn along(axis: Axis, r: Rect) -> (f32, f32) {
    match axis {
        Axis::Row => (r.x, r.w),
        Axis::Column => (r.y, r.h),
    }
}

fn across(axis: Axis, r: Rect) -> (f32, f32) {
    match axis {
        Axis::Row => (r.y, r.h),
        Axis::Column => (r.x, r.w),
    }
}
