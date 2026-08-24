//! The questions that go **upward and across**: the parent of a node, the
//! visible children of a split, where one boundary is, where all of them are,
//! how a region claims its extent, what a solo is holding, and whether a node
//! is a view.
//!
//! Every one of these was a caller's problem until it was this crate's, and
//! the first caller to need them wrote a model of its own to answer them —
//! which is what these are here to stop the second one doing.
//!
//! **Most of them are about `visible` rather than `children`.** A divider
//! index counts the children a split is *showing*, so a test run with nothing
//! folded cannot tell a visible-child index from a child index, and that is
//! the exact confusion these methods exist to end. So the arrangement is
//! folded in the middle of nearly every test below, and the assertion is that
//! the answer moved with the fold.

mod common;

use common::{console, console_ids, named_console, near, EPS};
use karakuri_layout::{Axis, Layout, NodeId, Point, Rect, Sizing, Spec};

/// The console's arrangement, solved somewhere roomy. 1000x800 leaves every
/// region well inside its bounds, so a fold below is what moves things rather
/// than a clamp.
fn solved() -> Layout {
    let mut l = named_console();
    l.set_viewport(Rect::new(0.0, 0.0, 1000.0, 800.0));
    l.solve();
    l
}

fn id(l: &Layout, name: &str) -> NodeId {
    l.find(name)
        .unwrap_or_else(|| panic!("no region named {name}"))
}

/// Every node in arena order.
fn every(l: &Layout) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut pending = vec![l.root()];
    while let Some(id) = pending.pop() {
        out.push(id);
        pending.extend(l.children(id).iter().copied());
    }
    out
}

/// `parent` is the split whose children list holds the node, for every node,
/// and `None` for the root alone.
///
/// The route it exists for is the one a pointer cannot take: a hit resolves to
/// a leaf, and *fold the split enclosing it* starts here.
#[test]
fn parent_is_the_split_whose_children_list_holds_the_node() {
    let l = solved();

    assert_eq!(l.parent(l.root()), None, "the root hangs from something");

    let mut orphans = 0;
    for node in every(&l) {
        match l.parent(node) {
            None => {
                assert_eq!(node, l.root(), "a node other than the root has no parent");
                orphans += 1;
            }
            Some(parent) => assert!(
                l.children(parent).contains(&node),
                "{:?} says its parent is {:?}, whose children do not include it",
                l.name(node),
                l.name(parent)
            ),
        }
    }
    assert_eq!(orphans, 1, "an arrangement is one tree with one root");

    // The one a fold of the enclosing split actually asks for: a leaf, and the
    // pane it is stacked in.
    assert_eq!(l.parent(id(&l, "library")), Some(id(&l, "left-pane")));
    assert_eq!(l.parent(id(&l, "left-pane")), Some(id(&l, "panes")));

    // A fold does not move anything: `parent` is the arrangement's, and the
    // arrangement is what a fold leaves alone.
    let mut folded = solved();
    folded.collapse(id(&folded, "left-pane"));
    folded.solve();
    assert_eq!(
        folded.parent(id(&folded, "library")),
        Some(id(&folded, "left-pane"))
    );
}

/// `visible_children` is what a divider index counts, and folding the middle
/// of three is what tells it apart from `children`.
///
/// Both halves matter. The list skips what is folded, **and** the index that
/// list is read at addresses a different pair than the same index into
/// `children` would — which is the mistake every caller that reconstructed
/// this filter was one line away from.
#[test]
fn visible_children_skips_what_is_folded_and_that_is_what_an_index_counts() {
    let mut l = solved();
    let ids = console_ids(&l);

    let all: Vec<NodeId> = l.children(ids.panes).to_vec();
    assert_eq!(all.len(), 3);
    assert_eq!(
        l.visible_children(ids.panes).collect::<Vec<_>>(),
        all,
        "nothing is folded, so every child is visible"
    );

    l.collapse(ids.centre);
    l.solve();

    let visible: Vec<NodeId> = l.visible_children(ids.panes).collect();
    assert_eq!(
        visible,
        vec![ids.left, ids.right],
        "the folded centre is still a visible child"
    );

    // Visible child 1 is the *right* pane, where child 1 is the centre — so a
    // caller counting `children` would move the wrong pair. Boundary 0 is
    // between the two panes that are showing, and there is no boundary 1.
    assert_eq!(visible[1], all[2]);
    assert_ne!(visible[1], all[1]);
    let gap = l
        .boundary(ids.panes, 0)
        .expect("a boundary between the two");
    assert!(near(gap.x, l.rect(ids.left).x + l.rect(ids.left).w));
    assert!(near(gap.x + gap.w, l.rect(ids.right).x));
    assert_eq!(l.boundary(ids.panes, 1), None);

    // A visible child of a folded split is still listed: it is folded by its
    // ancestor rather than by itself, and everything under a fold solves to
    // zero extent rather than to nothing.
    assert_eq!(l.visible_children(ids.centre).count(), 2);
    assert!(!l.visible(l.children(ids.centre)[0]));

    // A view has none, and asking is not an error.
    assert_eq!(l.visible_children(id(&l, "library")).count(), 0);
}

/// A boundary is the **gap between a pair**: the rectangle from the far edge
/// of one visible child to the near edge of the next, spanning the split
/// across its axis and as thick as the split's divider.
#[test]
fn a_boundary_is_the_gap_between_the_pair_it_is_between() {
    let l = solved();
    let ids = console_ids(&l);

    // A row: the gap between the left pane and the centre.
    let gap = l.boundary(ids.panes, 0).expect("the first boundary");
    let (left, centre) = (l.rect(ids.left), l.rect(ids.centre));
    let divider = l.divider(ids.panes).expect("a split has a divider");
    assert!(
        near(gap.x, left.x + left.w),
        "the gap starts where left ends"
    );
    assert!(
        near(gap.w, divider),
        "the gap is {} and not {divider}",
        gap.w
    );
    assert!(near(gap.x + gap.w, centre.x));
    let panes = l.rect(ids.panes);
    assert!(
        near(gap.y, panes.y) && near(gap.h, panes.h),
        "the gap does not span the split across its axis"
    );

    // The coordinate `set_divider` speaks in is the near edge of that gap,
    // which is what a press needs to know where along the boundary the pointer
    // took hold.
    let axis = l.axis(ids.panes).expect("a split has an axis");
    assert_eq!(axis, Axis::Row);
    assert!(near(axis.origin(gap), left.x + left.w));

    // And it is where `hit` finds the divider: aim at the middle of the gap.
    let middle = Point::new(gap.x + gap.w / 2.0, gap.y + gap.h / 2.0);
    assert_eq!(
        l.hit(middle, 0.0),
        karakuri_layout::Hit::Divider {
            split: ids.panes,
            index: 0
        }
    );

    // A column, so the gap is horizontal and the arithmetic is the other way.
    let root = l.root();
    let gap = l.boundary(root, 0).expect("the root's first boundary");
    let transport = l.rect(id(&l, "transport"));
    assert!(near(gap.y, transport.y + transport.h));
    assert!(near(gap.h, l.divider(root).unwrap()));
    assert!(near(gap.x, 0.0) && near(gap.w, 1000.0));

    // A view has no boundaries, and an index past the last is `None` rather
    // than the split's own far edge.
    assert_eq!(l.boundary(id(&l, "library"), 0), None);
    assert_eq!(l.boundary(root, 2), None);
}

/// **A boundary needs both sides.** Folding the far side of one leaves no
/// boundary at that index, and the answer is `None` rather than a position for
/// something else.
///
/// This is the defect the console carried: it checked that visible child
/// `index` existed and returned that child's far edge, so folding the far side
/// mid-drag handed back the *split's* own far edge — a plausible number for a
/// boundary that is not there, which is why `Released::Gone` was unreachable
/// through it.
#[test]
fn folding_the_far_side_of_a_boundary_leaves_no_boundary_rather_than_a_position() {
    let root = solved().root();
    // The last boundary of the root, so the child after it is the last one and
    // the split's own far edge is what the defect reached for.
    let last = solved().boundaries().filter(|(s, _)| *s == root).count() - 1;

    let mut l = solved();
    let far_side = l.visible_children(root).nth(last + 1).expect("a far side");
    let before = l.boundary(root, last).expect("a boundary before the fold");
    assert!(
        near(before.y + before.h, l.rect(far_side).y),
        "the boundary is not against the child after it"
    );

    l.collapse(far_side);
    l.solve();
    assert_eq!(
        l.boundary(root, last),
        None,
        "the far side is folded, so there is no boundary at that index"
    );

    // What the defect returned instead, named so that this cannot be read as
    // an assertion about an index that was simply out of range: the child
    // before the boundary now runs to the split's own far edge, and that edge
    // is a plausible number for a boundary that is not there.
    let near_side = l.visible_children(root).nth(last).expect("a near side");
    let (child, split) = (l.rect(near_side), l.rect(root));
    assert!(
        near(child.y + child.h, split.y + split.h),
        "the child before the boundary does not reach the split's far edge, so \
         this test is no longer aimed at the defect it was written for"
    );

    // The near side folded instead: the same index, the other way round, and
    // the same answer.
    let mut l = solved();
    let near_side = l.visible_children(root).nth(last).expect("a near side");
    l.collapse(near_side);
    l.solve();
    assert_eq!(l.boundary(root, last), None);
}

/// `boundaries` enumerates exactly the boundaries `boundary` answers for, and
/// the count follows the folds.
#[test]
fn boundaries_are_exactly_the_ones_that_have_a_pair() {
    let mut l = solved();

    let agrees = |l: &Layout| {
        for (split, index) in l.boundaries() {
            assert!(
                l.boundary(split, index).is_some(),
                "{:?}#{index} is enumerated and has no gap",
                l.name(split)
            );
        }
        // One past the last of each split is not one.
        for split in every(l) {
            let n = l.boundaries().filter(|(s, _)| *s == split).count();
            assert_eq!(
                n,
                l.visible_children(split).count().saturating_sub(1),
                "{:?} has the wrong number of boundaries",
                l.name(split)
            );
            assert_eq!(
                l.boundary(split, n),
                None,
                "{:?} answered for one boundary past its last",
                l.name(split)
            );
        }
    };

    // The console: 2 in the root, 2 in the row of panes, and 1 in each pane.
    assert_eq!(l.boundaries().count(), 7);
    agrees(&l);

    // Folding a pane takes the boundary beside it away — the row of panes has
    // one gap left — and leaves the pane's own inside alone, because what is
    // inside a fold is folded by its ancestor rather than by itself.
    let ids = console_ids(&l);
    l.collapse(ids.centre);
    l.solve();
    assert_eq!(l.boundaries().count(), 6);
    assert_eq!(
        l.boundaries().filter(|(s, _)| *s == ids.panes).count(),
        1,
        "the row of panes is showing two children, so it has one gap"
    );
    assert_eq!(
        l.boundaries().filter(|(s, _)| *s == ids.centre).count(),
        1,
        "what is inside the fold is not itself folded"
    );
    agrees(&l);

    // And that inner one is empty, because everything under a fold takes zero
    // extent along the axis the fold is on: a view iterating these to draw
    // draws nothing for it rather than something in the wrong place.
    let inner = l.boundary(ids.centre, 0).expect("still a boundary");
    assert!(
        near(inner.w, 0.0),
        "a boundary inside a folded pane has area: {inner:?}"
    );

    // A split showing one child has no boundary at all.
    l.collapse(ids.left);
    l.collapse(ids.right);
    l.solve();
    assert_eq!(l.boundaries().filter(|(s, _)| *s == ids.panes).count(), 0);
    agrees(&l);
}

/// `sizing` is why a region keeps its size when the window widens, which
/// `bounds` cannot say: a fixed region with no maximum still does not grow.
#[test]
fn sizing_says_which_regions_keep_their_size() {
    let mut l = solved();
    let ids = console_ids(&l);

    assert_eq!(l.sizing(ids.left), Sizing::Fixed(240.0));
    assert_eq!(l.sizing(ids.centre), Sizing::Flex(1.0));

    // The pair of claims a legend is made of: the fixed pane keeps its width
    // across a resize and the flexible centre absorbs the difference, and
    // `sizing` is what says which is which *before* a resize proves it.
    let (left, centre) = (l.rect(ids.left).w, l.rect(ids.centre).w);
    l.set_viewport(Rect::new(0.0, 0.0, 1400.0, 800.0));
    l.solve();
    assert!(near(l.rect(ids.left).w, left), "the fixed pane moved");
    assert!(near(l.rect(ids.centre).w, centre + 400.0));
    assert_eq!(
        l.sizing(ids.left),
        Sizing::Fixed(240.0),
        "a resize is not an edit, so it did not rewrite a sizing either"
    );

    // A drag is an edit, and it is the one thing that does rewrite it.
    l.set_divider(ids.panes, 0, 300.0);
    l.solve();
    assert_eq!(l.sizing(ids.left), Sizing::Fixed(300.0));
}

/// `soloed` says **which** region a solo is holding, not merely that one is.
#[test]
fn soloed_names_the_region_it_is_holding() {
    let mut l = solved();
    let program = id(&l, "program");
    let mixer = id(&l, "mixer");

    assert_eq!(l.soloed(), None);
    assert!(!l.is_soloed());

    l.solo(program);
    assert_eq!(l.soloed(), Some(program));
    assert!(l.is_soloed());

    // A solo while already soloed re-aims, and the name follows.
    l.solo(mixer);
    assert_eq!(l.soloed(), Some(mixer));

    l.unsolo();
    assert_eq!(l.soloed(), None);
    assert!(!l.is_soloed());

    // It survives being written down, which is the path a session takes.
    l.solo(program);
    l.solve();
    let text = serde_json::to_string(&l).expect("the arrangement serialises");
    let back: Layout = serde_json::from_str(&text).expect("and reads back");
    assert_eq!(back.soloed(), Some(program));
}

/// Why the soloed node is **stored** rather than worked out from the collapsed
/// flags: a split with a single child leaves exactly the flags its child does,
/// so nothing in the arrangement tells the two apart afterwards.
#[test]
fn a_solo_on_a_single_child_split_cannot_be_told_from_one_on_its_child() {
    let arrangement = || {
        Spec::row(
            4.0,
            vec![
                Spec::column(4.0, vec![Spec::view("only")]).named("wrapper"),
                Spec::view("other"),
            ],
        )
    };
    let flags = |l: &Layout| {
        let mut out = Vec::new();
        let mut pending = vec![l.root()];
        while let Some(id) = pending.pop() {
            out.push(l.is_collapsed(id));
            pending.extend(l.children(id).iter().copied());
        }
        out
    };

    let mut wrapper = Layout::new(arrangement());
    let w = wrapper.find("wrapper").unwrap();
    wrapper.solo(w);

    let mut only = Layout::new(arrangement());
    let o = only.find("only").unwrap();
    only.solo(o);

    assert_eq!(
        flags(&wrapper),
        flags(&only),
        "the two solos are supposed to be indistinguishable by their flags"
    );
    assert_ne!(
        wrapper.soloed(),
        only.soloed(),
        "and the answer has to come from somewhere other than the flags"
    );
    assert_eq!(wrapper.soloed(), Some(w));
    assert_eq!(only.soloed(), Some(o));
}

/// `is_view` tells the thing something paints from the thing whose gaps are
/// drawn between its children.
#[test]
fn is_view_tells_a_leaf_from_a_split() {
    let l = solved();

    assert!(
        !l.is_view(l.root()),
        "the root of this arrangement is a split"
    );
    assert!(l.is_view(id(&l, "library")));
    assert!(!l.is_view(id(&l, "left-pane")));

    let mut views = 0;
    for node in every(&l) {
        assert_eq!(
            l.is_view(node),
            l.children(node).is_empty(),
            "{:?} is a view and has children, or is a split and has none",
            l.name(node)
        );
        // A view has no axis and no divider; a split has both.
        assert_eq!(l.is_view(node), l.axis(node).is_none());
        assert_eq!(l.is_view(node), l.divider(node).is_none());
        views += usize::from(l.is_view(node));
    }
    assert_eq!(views, 8, "the test arrangement has eight regions");
}

/// The unnamed splits answer too — `console` leaves every split unnamed, and
/// none of these methods goes through a name.
#[test]
fn every_answer_holds_of_an_arrangement_whose_splits_have_no_names() {
    let mut l = console();
    l.set_viewport(Rect::new(0.0, 0.0, 1000.0, 800.0));
    l.solve();
    let ids = console_ids(&l);

    assert_eq!(l.name(ids.panes), None);
    assert_eq!(l.parent(ids.left), Some(ids.panes));
    assert!(!l.is_view(ids.panes));
    assert_eq!(l.visible_children(ids.panes).count(), 3);
    assert!(l.boundary(ids.panes, 1).is_some());
    assert!(l.boundaries().any(|(s, i)| s == ids.panes && i == 1));

    // The gap is where the two rectangles are not.
    let gap = l.boundary(ids.panes, 1).unwrap();
    assert!(gap.w > EPS, "a divider of zero width would prove nothing");
    assert!(near(gap.x, l.rect(ids.centre).x + l.rect(ids.centre).w));
}
