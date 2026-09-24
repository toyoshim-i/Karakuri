//! Tests for ephemeral `set_aside` region omission.
//!
//! Verifies that `set_aside` operates independently of user `collapse` state,
//! solves identically to zero-extent nodes, and is excluded from serialization.

mod common;

use common::{assert_invariants, near, rects, simple};
use karakuri_layout::{Hit, Layout, NodeId, Point, Rect, Spec};

/// The row of three, solved somewhere roomy: `left` 100, `centre` 836,
/// `right` 60, with 2-thick dividers.
fn row() -> (Layout, NodeId) {
    let mut l = simple();
    l.set_viewport(Rect::new(0.0, 0.0, 1000.0, 400.0));
    l.solve();
    let root = l.root();
    (l, root)
}

fn id(l: &Layout, name: &str) -> NodeId {
    l.find(name)
        .unwrap_or_else(|| panic!("no region named {name}"))
}

/// Returns a layout consisting of a fixed column split (`stack`) over a flexible view (`tail`).
fn stacked() -> Layout {
    let mut l = Layout::new(Spec::column(
        10.0,
        vec![
            Spec::column(
                8.0,
                vec![
                    Spec::view("head").flex(1.0).min(120.0),
                    Spec::view("foot").fixed(72.0).min(72.0),
                ],
            )
            .named("stack")
            .fixed(378.0)
            .min(200.0),
            Spec::view("tail").flex(1.0).min(126.0),
        ],
    ));
    l.set_viewport(Rect::new(0.0, 0.0, 600.0, 530.0));
    l.solve();
    l
}

/// Verifies that `is_collapsed` and `is_set_aside` maintain independent state flags.
#[test]
fn the_operators_fold_and_a_node_set_aside_are_two_bits_and_not_one() {
    let (mut l, _) = row();
    let centre = id(&l, "centre");

    assert!(!l.is_collapsed(centre) && !l.is_set_aside(centre));

    // Set aside: that bit and no other. Nobody folded it, and nothing that
    // reports a fold may say otherwise — an operation that unfolds everything
    // works off exactly this answer.
    l.set_aside(centre, true);
    assert!(l.is_set_aside(centre), "the bit was not written");
    assert!(
        !l.is_collapsed(centre),
        "setting a node aside reported it as folded by the operator"
    );

    // Folded: the other bit, and the first one is untouched.
    l.set_aside(centre, false);
    l.collapse(centre);
    assert!(l.is_collapsed(centre));
    assert!(
        !l.is_set_aside(centre),
        "a fold reported the node as set aside"
    );

    // Both at once, and each cleared by whoever set it. Neither clearing is
    // enough on its own: the node is out of the layout until both are.
    l.set_aside(centre, true);
    assert!(l.is_collapsed(centre) && l.is_set_aside(centre));
    l.solve();
    assert!(!l.visible(centre));

    l.expand(centre);
    l.solve();
    assert!(l.is_set_aside(centre), "an expand cleared the other bit");
    assert!(
        !l.visible(centre),
        "the node is still set aside, so it is still not laid out"
    );

    l.collapse(centre);
    l.set_aside(centre, false);
    l.solve();
    assert!(l.is_collapsed(centre), "clearing the other bit unfolded it");
    assert!(
        !l.visible(centre),
        "the node is still folded, so it is still not laid out"
    );

    l.expand(centre);
    l.solve();
    assert!(
        l.visible(centre),
        "both are clear and it is still not there"
    );
}

/// A node that is set aside is not laid out: zero extent, no divider beside
/// it, nothing under it drawn, and not a hit target. The space it leaves goes
/// back to the split, exactly as a fold's does.
#[test]
fn a_node_set_aside_takes_no_extent_and_no_divider() {
    let (mut l, root) = row();
    assert_eq!(l.boundaries().count(), 2);

    l.set_aside(id(&l, "centre"), true);
    l.solve();
    assert_invariants(&l);

    let (left, centre, right) = (id(&l, "left"), id(&l, "centre"), id(&l, "right"));
    assert!(!l.visible(centre));
    assert_eq!(l.rect(centre).w, 0.0, "a node that is out took extent");

    // The two that are left tile the row with one divider between them: the
    // fixed pair take the whole 998 in proportion, `left` stopping at its
    // maximum of 150 and `right` taking the rest.
    assert!(near(l.rect(left).w, 150.0), "left is {}", l.rect(left).w);
    assert!(near(l.rect(right).w, 848.0), "right is {}", l.rect(right).w);
    assert_eq!(
        l.placed_children(root).collect::<Vec<_>>(),
        vec![left, right],
        "a node that is out is still a child a divider index counts"
    );

    // One boundary, between the pair that is showing, and none where the
    // node that is out used to be.
    assert_eq!(l.boundaries().count(), 1);
    let gap = l.boundary(root, 0).expect("a boundary between the two");
    assert!(near(gap.x, 150.0) && near(gap.w, 2.0), "the gap is {gap:?}");
    assert_eq!(l.boundary(root, 1), None);

    // And it is not a target: the space it left belongs to whoever took it.
    assert_eq!(l.hit(Point::new(400.0, 200.0), 3.0), Hit::View(right));

    // What a split can use follows it too, which is the rule a fold already
    // obeyed: the stack's visible content is one fixed row of 72, so 72 is
    // what it claims of the 378 it stores, and the flexible sibling gets the
    // difference. Nothing was written back — the 378 is still there.
    let mut s = stacked();
    let stack = id(&s, "stack");
    s.set_aside(id(&s, "head"), true);
    s.solve();
    assert_invariants(&s);
    assert!(near(s.rect(stack).h, 72.0), "stack is {}", s.rect(stack).h);
    assert!(
        near(s.rect(id(&s, "tail")).h, 448.0),
        "tail is {}",
        s.rect(id(&s, "tail")).h
    );
    s.set_aside(id(&s, "head"), false);
    s.solve();
    assert!(near(s.rect(stack).h, 378.0), "stack is {}", s.rect(stack).h);
}

/// Verifies that `set_aside` produces solved rectangles and boundaries identical to `collapse`.
#[test]
fn setting_a_node_aside_lays_out_exactly_as_folding_it_does() {
    for name in ["left", "centre", "right"] {
        let (mut folded, _) = row();
        folded.collapse(id(&folded, name));
        folded.solve();

        let (mut aside, _) = row();
        aside.set_aside(id(&aside, name), true);
        aside.solve();

        assert_eq!(
            rects(&aside),
            rects(&folded),
            "{name} set aside did not solve as {name} folded"
        );
        assert_eq!(
            aside.boundaries().collect::<Vec<_>>(),
            folded.boundaries().collect::<Vec<_>>(),
            "{name}: the boundaries disagree"
        );
        for node in [
            id(&aside, "left"),
            id(&aside, "centre"),
            id(&aside, "right"),
        ] {
            assert_eq!(
                aside.visible(node),
                folded.visible(node),
                "{name}: {node:?} is drawn in one and not the other"
            );
        }

        // And the arrangements do not agree about *why*, which is the whole of
        // what is stored differently.
        assert!(aside.is_set_aside(id(&aside, name)) && !aside.is_collapsed(id(&aside, name)));
        assert!(folded.is_collapsed(id(&folded, name)) && !folded.is_set_aside(id(&folded, name)));
    }
}

/// Verifies that the set-aside bit is ephemeral and excluded from serialized
/// arrangements without introducing wire format fields (ADR-0157).
#[test]
fn a_saved_arrangement_neither_carries_the_bit_nor_gained_a_field_for_it() {
    let (plain, _) = row();
    let untouched = serde_json::to_value(&plain).expect("the arrangement serialises");

    let (mut l, _) = row();
    let centre = id(&l, "centre");
    l.set_aside(centre, true);
    l.collapse(id(&l, "left"));
    l.solve();
    let file = serde_json::to_value(&l).expect("the arrangement serialises");

    // The fold is in the file and the other bit is nowhere in it — which is
    // also the field count, since one file is the other with a fold flipped.
    assert_ne!(file, untouched, "the fold was not written either");
    assert_eq!(
        file["nodes"][centre_index(&file)],
        untouched["nodes"][centre_index(&untouched)],
        "the node that was set aside was saved differently from one that was not"
    );
    assert!(
        !file.to_string().contains("aside"),
        "the wire format gained a field: {file}"
    );

    // A load lays it out again, and keeps the operator's fold.
    let back: Layout = serde_json::from_value(file).expect("and reads back");
    let centre = id(&back, "centre");
    assert!(
        !back.is_set_aside(centre),
        "a load brought the bit back from somewhere"
    );
    assert!(back.visible(centre), "the loaded node is not laid out");
    assert!(
        back.is_collapsed(id(&back, "left")),
        "the fold did not load"
    );

    // Nor from the arena the load assembled into: `Layout::new` starts every
    // node laid out, whatever a `Spec` says about folds, so there is nowhere
    // for a stale bit to come from on either path.
    let fresh = simple();
    for node in [
        id(&fresh, "left"),
        id(&fresh, "centre"),
        id(&fresh, "right"),
        fresh.root(),
    ] {
        assert!(!fresh.is_set_aside(node), "{node:?} was built set aside");
    }
}

/// The index of the node named `centre` in a saved arrangement.
fn centre_index(file: &serde_json::Value) -> usize {
    file["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["kind"]["View"]["name"] == "centre")
        .expect("no node named centre in the saved arrangement")
}

/// Verifies that writing an identical `set_aside` value does not set the dirty flag.
#[test]
fn writing_the_bit_a_node_already_carries_marks_nothing_dirty() {
    let (mut l, _) = row();
    let centre = id(&l, "centre");
    let before = rects(&l);

    l.set_aside(centre, true);
    l.solve();
    let after = rects(&l);
    assert_ne!(
        after, before,
        "the write did nothing, so this proves nothing"
    );

    // The same value again: no solve follows, and the rectangles still answer.
    l.set_aside(centre, true);
    assert_eq!(rects(&l), after, "a redundant write moved something");

    // And the other way round, so this is about writing the same value rather
    // than about writing `true`.
    l.set_aside(centre, false);
    l.solve();
    assert_eq!(rects(&l), before);
    l.set_aside(centre, false);
    assert_eq!(rects(&l), before);

    // A hundred frames of a caller re-stating what it stated last frame. The
    // read at the end is the whole assertion; the loop is what a frame does.
    for _ in 0..100 {
        for node in [id(&l, "left"), centre, id(&l, "right")] {
            l.set_aside(node, false);
        }
    }
    assert_eq!(rects(&l), before, "a still arrangement moved");
}

/// Verifies that `solo` and `unsolo` only manipulate `collapse` state without altering `set_aside`.
#[test]
fn a_solo_neither_saves_nor_restores_what_it_does_not_own() {
    let (mut l, _) = row();
    let (left, centre, right) = (id(&l, "left"), id(&l, "centre"), id(&l, "right"));

    l.collapse(left);
    l.set_aside(right, true);
    l.solve();
    let before = rects(&l);

    l.solo(centre);
    l.solve();
    assert_invariants(&l);
    assert_eq!(l.soloed(), Some(centre));
    assert!(
        l.is_set_aside(right),
        "the solo cleared a bit that is not its to write"
    );
    assert!(!l.is_set_aside(centre) && !l.is_set_aside(left));

    // The unsolo puts the folds back and leaves the other bit exactly where it
    // found it — which is where it is, since nothing touched it in between.
    l.unsolo();
    l.solve();
    assert_eq!(rects(&l), before, "the unsolo did not restore");
    assert!(l.is_collapsed(left) && !l.is_collapsed(centre));
    assert!(l.is_set_aside(right), "the unsolo wrote the other bit back");

    // Soloing the node that is set aside: it holds the whole viewport and is
    // still not laid out, because the solo said which region is alone on
    // screen and not whether the caller is drawing it here.
    l.solo(right);
    l.solve();
    assert_invariants(&l);
    assert!(l.is_set_aside(right), "the solo laid out what it was given");
    assert!(!l.visible(right), "a node that is set aside was drawn");
    assert_eq!(l.rect(right).w, 0.0);

    // Clearing it is the caller's, and it is one call: the solo left nothing
    // else in the way.
    l.set_aside(right, false);
    l.solve();
    assert!(l.visible(right));
    let (r, v) = (l.rect(right), l.viewport());
    assert!(
        near(r.x, v.x) && near(r.y, v.y) && near(r.w, v.w) && near(r.h, v.h),
        "the soloed region is {r:?} rather than the viewport {v:?}"
    );

    // And the bit the caller cleared is not resurrected by the unsolo either:
    // there was nothing saved to resurrect it from.
    l.unsolo();
    l.solve();
    assert!(!l.is_set_aside(right), "the unsolo brought the bit back");
    assert!(l.is_collapsed(left), "the unsolo lost a fold");
}
