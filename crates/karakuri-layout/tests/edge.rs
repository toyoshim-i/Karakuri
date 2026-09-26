//! Tests for nodes configured with `keeps_its_edge`.
//!
//! Verifies that closed nodes retain their divider gaps and hit targets
//! without occupying extent, and that solos suppress preserved edges during isolation.

mod common;

use common::{assert_invariants, near, EPS};
use karakuri_layout::{Hit, Layout, NodeId, Point, Rect, Sizing, Spec};

/// Returns a 3-pane layout (`left`, `centre`, `right`) where side panes declare `keeps_its_edge`.
fn panes() -> Layout {
    let mut layout = Layout::new(Spec::row(
        10.0,
        vec![
            Spec::view("left").fixed(100.0).min(40.0).keeps_its_edge(),
            Spec::view("centre").flex(1.0).min(200.0),
            Spec::view("right").fixed(100.0).min(40.0).keeps_its_edge(),
        ],
    ));
    layout.set_viewport(Rect::new(0.0, 0.0, 1000.0, 400.0));
    layout.solve();
    layout
}

/// The same row with nothing declaring an edge — the control for the
/// measurement below, and the arrangement every fold in this crate had before.
fn plain() -> Layout {
    let mut layout = Layout::new(Spec::row(
        10.0,
        vec![
            Spec::view("left").fixed(100.0).min(40.0),
            Spec::view("centre").flex(1.0).min(200.0),
            Spec::view("right").fixed(100.0).min(40.0),
        ],
    ));
    layout.set_viewport(Rect::new(0.0, 0.0, 1000.0, 400.0));
    layout.solve();
    layout
}

fn id(l: &Layout, name: &str) -> NodeId {
    l.find(name)
        .unwrap_or_else(|| panic!("a node named {name}"))
}

/// A closed node has zero extent while retaining its divider.
#[test]
fn a_closed_node_keeps_its_divider_and_an_ordinary_fold_does_not() {
    for (name, keeps) in [("left", true), ("centre", false), ("right", true)] {
        let mut l = panes();
        let root = l.root();
        let node = id(&l, name);
        assert_eq!(
            l.keeps_its_edge(node),
            keeps,
            "`{name}` declares the wrong thing about its edge"
        );

        l.collapse(node);
        l.solve();
        assert_invariants(&l);

        assert!(
            near(l.rect(node).w, 0.0),
            "a folded `{name}` is {} wide",
            l.rect(node).w
        );
        assert!(!l.visible(node), "a folded `{name}` is still visible");
        assert_eq!(l.is_closed(node), keeps);
        assert_eq!(l.is_placed(node), keeps);
        assert_eq!(
            l.placed_children(root).count(),
            match keeps {
                true => 3,
                false => 2,
            },
            "`{name}` folded and the row tiles the wrong number of children"
        );
        assert_eq!(
            l.boundaries().filter(|(s, _)| *s == root).count(),
            match keeps {
                true => 2,
                false => 1,
            },
            "`{name}` folded and the row has the wrong number of boundaries"
        );
    }
}

/// The neighbour takes the pane extent while preserving the divider width.
#[test]
fn a_closed_pane_gives_its_extent_to_the_centre_and_keeps_the_divider() {
    let mut l = panes();
    let was = l.rect(id(&l, "centre")).w;
    let pane = l.rect(id(&l, "left")).w;
    let divider = l.divider(l.root()).expect("the row is a split");

    l.collapse(id(&l, "left"));
    l.solve();
    let now = l.rect(id(&l, "centre")).w;
    assert!(
        near(now, was + pane),
        "the centre went from {was} to {now}: it takes the pane's {pane} and leaves the \
         {divider} the pane keeps"
    );

    // The same fold on the same row with nothing declaring an edge, which is
    // the control: an ordinary fold hands over the divider as well, and the
    // difference between the two is exactly one.
    let mut l = plain();
    let was = l.rect(id(&l, "centre")).w;
    let pane = l.rect(id(&l, "left")).w;
    l.collapse(id(&l, "left"));
    l.solve();
    let ordinary = l.rect(id(&l, "centre")).w;
    assert!(
        near(ordinary, was + pane + divider),
        "with no edge declared the centre went from {was} to {ordinary} — an ordinary fold takes \
         its divider with it"
    );
    assert!(
        near(ordinary - now, divider),
        "the two folds differ by {} and the divider is {divider} — the edge costs exactly the \
         gap it keeps, and nothing else",
        ordinary - now
    );
}

/// The boundary gap kept by a closed pane remains hit-testable at the split outer edge.
#[test]
fn a_pointer_finds_the_boundary_a_closed_pane_keeps() {
    const GRAB: f32 = 6.0;
    for (name, outer) in [("left", 0.0f32), ("right", 999.0)] {
        let mut l = panes();
        let mid = Point::new(outer, 200.0);
        assert!(
            !matches!(l.hit(mid, GRAB), Hit::Divider { .. }),
            "a boundary already grabs {mid:?} with `{name}` open, and the split's outer edge is \
             supposed to be nobody's"
        );

        l.collapse(id(&l, name));
        l.solve();
        let hit = l.hit(mid, GRAB);
        assert!(
            matches!(hit, Hit::Divider { .. }),
            "with `{name}` closed, {mid:?} answered {hit:?} — the divider the pane kept is what a \
             hand takes hold of to bring it back, and there is nothing else there"
        );
    }
}

/// A closed fold reports `is_collapsed`, preserving its stored size for subsequent expansion.
#[test]
fn a_closed_fold_is_a_fold_and_expand_restores_the_size() {
    let mut l = panes();
    let node = id(&l, "left");
    let was = l.rect(node).w;

    l.collapse(node);
    l.solve();
    assert!(l.is_collapsed(node), "a closed node is not collapsed");
    assert!(!l.visible(node), "a closed node is visible");

    l.expand(node);
    l.solve();
    assert!(
        near(l.rect(node).w, was),
        "`expand` brought the pane back at {} rather than the {was} it was storing",
        l.rect(node).w
    );
    assert!(!l.is_closed(node));
    assert_eq!(l.sizing(node), Sizing::Fixed(100.0));
}

/// Verifies that a solo collapses all sibling edges to give the soloed node the entire viewport.
#[test]
fn a_solo_leaves_no_edges_behind_and_the_unsolo_puts_them_back() {
    let mut l = panes();
    let centre = id(&l, "centre");
    let left = id(&l, "left");

    // A pane the operator had already closed, so the unsolo has an edge to
    // restore rather than only edges to have suppressed.
    l.collapse(left);
    l.solve();
    assert!(l.is_closed(left));

    l.solo(centre);
    l.solve();
    assert_invariants(&l);
    assert_eq!(
        l.rect(centre),
        l.viewport(),
        "a solo left the centre short of the viewport, which is what an edge left behind costs"
    );
    for name in ["left", "right"] {
        let node = id(&l, name);
        assert!(
            !l.is_closed(node),
            "`{name}` kept its edge under a solo, so the soloed region does not hold the window"
        );
        assert!(!l.is_placed(node));
    }
    assert_eq!(l.placed_children(l.root()).count(), 1);

    l.unsolo();
    l.solve();
    assert!(
        l.is_closed(left),
        "the unsolo did not put back the edge the operator's own fold had left"
    );
    assert!(!l.is_collapsed(id(&l, "right")));
}

/// Verifies that dragging a boundary adjacent to a closed node is refused, preserving stored sizes.
#[test]
fn a_drag_against_a_closed_node_moves_nothing_and_forgets_nothing() {
    let mut l = panes();
    let root = l.root();
    let left = id(&l, "left");
    l.collapse(left);
    l.solve();

    let before: Vec<Rect> = (0..3).map(|k| l.rect(l.children(root)[k])).collect();
    let at = l
        .boundary(root, 0)
        .expect("a closed pane keeps its boundary");
    for position in [-500.0f32, 0.0, 5.0, 300.0, 5000.0] {
        let landed = l.set_divider(root, 0, position);
        l.solve();
        assert!(
            (landed - at.x).abs() <= EPS,
            "a drag to {position} moved the boundary beside a closed pane from {} to {landed}",
            at.x
        );
    }
    let after: Vec<Rect> = (0..3).map(|k| l.rect(l.children(root)[k])).collect();
    assert_eq!(
        before, after,
        "a drag against a closed pane moved something"
    );
    assert_eq!(
        l.sizing(left),
        Sizing::Fixed(100.0),
        "a drag against a closed pane overwrote the width `expand` exists to restore"
    );

    l.expand(left);
    l.solve();
    assert!(near(l.rect(left).w, 100.0));
}
