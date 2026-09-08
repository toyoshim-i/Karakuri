//! What a node claims of its parent, and the ceiling its own content puts on
//! it.
//!
//! **No node claims more than its visible content can use.** A view says what
//! it can use directly — a flexible one any amount, a fixed one the size it
//! stores — and a split's is the sum of its visible children's plus the
//! dividers between them, so folding what is inside a split lowers what the
//! split asks for. Two things are capped by it and they are the same sentence
//! twice: a fixed node's claim, and its declared minimum.
//!
//! Stated in this crate's own words throughout — a leaf is a view and an
//! interior node is a split — because not knowing what any of these regions
//! *is* is why the crate exists.

mod common;

use common::{assert_invariants, near};
use karakuri_layout::{Layout, Rect, Spec};

/// A column of two: a **fixed split** over a flexible view.
///
/// The split holds a flexible view over a fixed one, which is the shape of any
/// bay whose content is one thing that stretches and one row that does not.
/// It stores 378 and declares a minimum of 200, and the row under it is 72.
///
/// At 530 high with a 10-thick divider: `stack` 378, `tail` 142; and inside
/// the stack, `head` 298 over an 8-thick divider over `foot` 72.
fn stacked() -> Layout {
    Layout::new(Spec::column(
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
    ))
}

fn at(l: &mut Layout, h: f32) {
    l.set_viewport(Rect::new(0.0, 0.0, 600.0, h));
    l.solve();
}

/// The extent of a named region along the column these arrangements are.
fn h(l: &Layout, name: &str) -> f32 {
    l.rect(l.find(name).unwrap()).h
}

#[test]
fn a_fixed_split_claims_what_its_visible_content_can_use() {
    let mut l = stacked();
    at(&mut l, 530.0);
    assert_invariants(&l);
    assert!(near(h(&l, "stack"), 378.0));
    assert!(near(h(&l, "tail"), 142.0));

    // Fold the flexible half away and the split's visible content is one
    // fixed view of 72. So 72 is what it claims, rather than the 378 it
    // stores, and the flexible sibling absorbs the whole difference.
    let head = l.find("head").unwrap();
    l.collapse(head);
    l.solve();
    assert_invariants(&l);
    assert!(near(h(&l, "stack"), 72.0), "stack is {}", h(&l, "stack"));
    assert!(near(h(&l, "foot"), 72.0), "foot is {}", h(&l, "foot"));
    assert!(near(h(&l, "tail"), 448.0), "tail is {}", h(&l, "tail"));

    // And nothing was written back on the way: the 378 is still there to come
    // back to, which is the whole of P-0082 applied to a measuring pass that
    // runs bottom-up.
    l.expand(head);
    l.solve();
    assert_invariants(&l);
    assert!(near(h(&l, "stack"), 378.0));
    assert!(near(h(&l, "tail"), 142.0));
}

#[test]
fn the_dividers_between_placed_children_count_toward_what_a_split_can_use() {
    // Two fixed views in a split whose divider is 8: what the split can use is
    // 40 + 8 + 60, not 100. A cap that summed the children alone would put
    // eight pixels of the split's content outside the split.
    let mut l = Layout::new(Spec::column(
        10.0,
        vec![
            Spec::column(
                8.0,
                vec![
                    Spec::view("upper").fixed(40.0),
                    Spec::view("lower").fixed(60.0),
                ],
            )
            .named("pair")
            .fixed(300.0),
            Spec::view("tail").flex(1.0),
        ],
    ));
    at(&mut l, 500.0);
    assert_invariants(&l);

    assert!(near(h(&l, "pair"), 108.0), "pair is {}", h(&l, "pair"));
    assert!(near(h(&l, "tail"), 382.0), "tail is {}", h(&l, "tail"));
    assert!(near(h(&l, "upper"), 40.0));
    assert!(near(h(&l, "lower"), 60.0));

    // Fold one of them and the divider goes with it: one visible child is 60
    // and no gap at all.
    l.collapse(l.find("upper").unwrap());
    l.solve();
    assert_invariants(&l);
    assert!(near(h(&l, "pair"), 60.0), "pair is {}", h(&l, "pair"));
}

#[test]
fn a_visible_flexible_child_leaves_a_split_able_to_use_any_amount() {
    // The other half of the rule, and the half that has to change nothing: a
    // flexible view can use whatever it is given, so the split holding one can
    // too, so its stored size and its declared minimum are what they always
    // were.
    let mut l = stacked();
    // 514 is where the arrangement stops fitting — 378, the divider, and the
    // flexible view's own minimum — and below that everything scales together.
    for height in [530.0, 900.0, 2400.0] {
        at(&mut l, height);
        assert_invariants(&l);
        assert!(
            near(h(&l, "stack"), 378.0),
            "stack is {} at {height}",
            h(&l, "stack")
        );
        assert!(
            near(h(&l, "head"), 298.0),
            "head is {} at {height}",
            h(&l, "head")
        );
        assert!(
            near(h(&l, "foot"), 72.0),
            "foot is {} at {height}",
            h(&l, "foot")
        );
    }

    // And the declared minimum is still a floor a drag stops at, which is what
    // says the 200 is untouched rather than merely unreached.
    at(&mut l, 900.0);
    let root = l.root();
    l.set_divider(root, 0, 10.0);
    l.solve();
    assert_invariants(&l);
    assert!(near(h(&l, "stack"), 200.0), "stack is {}", h(&l, "stack"));
}

#[test]
fn a_minimum_larger_than_what_the_content_can_use_does_not_hold_space() {
    let mut l = stacked();
    at(&mut l, 530.0);
    let stack = l.find("stack").unwrap();
    assert_eq!(l.bounds(stack).0, 200.0, "the minimum is still declared");

    // The declared 200 is what the split needs *while it has 200 worth of
    // content*. With only the 72-tall row left visible, holding 200 is holding
    // 128 the split will leave empty — and there is room here for it to have
    // held it, since the flexible sibling is 306 above its own minimum.
    l.collapse(l.find("head").unwrap());
    l.solve();
    assert_invariants(&l);
    assert!(near(h(&l, "stack"), 72.0), "stack is {}", h(&l, "stack"));
    assert!(
        h(&l, "stack") < l.bounds(stack).0,
        "the effective minimum is the content's, not the declared one"
    );
    assert_eq!(l.bounds(stack).0, 200.0, "and nothing rewrote the minimum");
}

#[test]
fn a_fixed_view_claims_its_stored_size_whatever_is_folded_around_it() {
    // The rule must be a no-op except where it bites, and a fixed *view* is
    // where it never bites: what a leaf can use is exactly what it stores, so
    // the cap is the size it was already claiming.
    let mut l = stacked();
    for height in [530.0, 900.0] {
        at(&mut l, height);
        assert!(
            near(h(&l, "foot"), 72.0),
            "foot is {} at {height}",
            h(&l, "foot")
        );
    }

    // Beside a folded sibling, inside a split that is itself capped, and in a
    // viewport too small for the arrangement — 72 in each of them.
    at(&mut l, 530.0);
    l.collapse(l.find("head").unwrap());
    l.solve();
    assert_invariants(&l);
    assert!(near(h(&l, "foot"), 72.0), "foot is {}", h(&l, "foot"));

    l.expand(l.find("head").unwrap());
    l.collapse(l.find("tail").unwrap());
    l.solve();
    assert_invariants(&l);
    assert!(near(h(&l, "foot"), 72.0), "foot is {}", h(&l, "foot"));
}

#[test]
fn a_split_with_everything_folded_claims_nothing() {
    let mut l = stacked();
    at(&mut l, 530.0);
    l.collapse(l.find("head").unwrap());
    l.collapse(l.find("foot").unwrap());
    l.solve();
    assert_invariants(&l);

    // Nothing inside it is drawn, so it needs no room to draw it in. The
    // split is still visible — it is its children that are folded — so the
    // divider beside it is still there, and the flexible sibling has the rest.
    let stack = l.find("stack").unwrap();
    assert!(near(h(&l, "stack"), 0.0), "stack is {}", h(&l, "stack"));
    assert!(!l.is_collapsed(stack));
    assert!(near(h(&l, "tail"), 520.0), "tail is {}", h(&l, "tail"));

    // Nothing downstream of a zero is negative, infinite or NaN — a sum that
    // reached a division would show here and nowhere else.
    for height in [0.0, 1.0, 530.0, 2400.0] {
        at(&mut l, height);
        assert_invariants(&l);
        for id in [
            l.root(),
            stack,
            l.find("head").unwrap(),
            l.find("foot").unwrap(),
        ] {
            let r = l.rect(id);
            assert!(
                r.x.is_finite() && r.y.is_finite() && r.w.is_finite() && r.h.is_finite(),
                "{r:?} at {id:?}, viewport {height} high"
            );
            assert!(r.w >= 0.0 && r.h >= 0.0, "{r:?} at {id:?}");
        }
    }
}

#[test]
fn a_maximum_and_the_cap_are_one_ceiling() {
    // A maximum is the other thing that says a node cannot use more, so it
    // caps what a node can use like any other — and the smaller of the two
    // wins, whichever it is.
    let capped = |max: f32| {
        Layout::new(Spec::column(
            10.0,
            vec![
                Spec::column(
                    8.0,
                    vec![Spec::view("head").flex(1.0), Spec::view("foot").fixed(72.0)],
                )
                .named("stack")
                .fixed(378.0)
                .max(max),
                Spec::view("tail").flex(1.0),
            ],
        ))
    };

    // The maximum is the smaller: the split claims 150 of its stored 378,
    // which is what it would have been clamped to anyway. ADR-0157 is
    // untouched — a maximum is still honoured, and it is honoured earlier.
    let mut l = capped(150.0);
    at(&mut l, 530.0);
    assert_invariants(&l);
    assert!(near(h(&l, "stack"), 150.0), "stack is {}", h(&l, "stack"));
    assert!(near(h(&l, "tail"), 370.0), "tail is {}", h(&l, "tail"));

    // The content is the smaller: fold the flexible half and the 72 wins over
    // the 150.
    l.collapse(l.find("head").unwrap());
    l.solve();
    assert_invariants(&l);
    assert!(near(h(&l, "stack"), 72.0), "stack is {}", h(&l, "stack"));

    // And a maximum stated *inside* the split is a cap on what the split can
    // use, because it is a cap on what the child can. A picture that will
    // never be taller than 100 over a 72 row is a 180-tall split, and the 198
    // that used to sit inside it as trailing space is now space the split
    // never asked for — so a flexible sibling has it instead.
    let mut inner = Layout::new(Spec::column(
        10.0,
        vec![
            Spec::column(
                8.0,
                vec![
                    Spec::view("head").flex(1.0).max(100.0),
                    Spec::view("foot").fixed(72.0),
                ],
            )
            .named("stack")
            .fixed(378.0),
            Spec::view("tail").flex(1.0),
        ],
    ));
    at(&mut inner, 530.0);
    assert_invariants(&inner);
    assert!(
        near(h(&inner, "stack"), 180.0),
        "stack is {}",
        h(&inner, "stack")
    );
    assert!(near(h(&inner, "head"), 100.0));
    assert!(near(h(&inner, "foot"), 72.0));
    assert!(
        near(h(&inner, "tail"), 340.0),
        "tail is {}",
        h(&inner, "tail")
    );
}

#[test]
fn a_split_laid_out_across_its_parents_axis_is_not_a_sum() {
    // The cap is one number per node, stated along its *parent's* axis, and a
    // split laid out the other way has nothing to say in it: its children's
    // sizes are heights where its parent is handing out widths. So it claims
    // its stored size like any other node, and the sum of what is inside it is
    // arithmetic on the wrong question.
    let mut l = Layout::new(Spec::row(
        10.0,
        vec![
            Spec::column(
                8.0,
                vec![
                    Spec::view("upper").fixed(30.0),
                    Spec::view("lower").fixed(40.0),
                ],
            )
            .named("pane")
            .fixed(200.0),
            Spec::view("rest").flex(1.0),
        ],
    ));
    l.set_viewport(Rect::new(0.0, 0.0, 800.0, 400.0));
    l.solve();
    assert_invariants(&l);

    let pane = l.rect(l.find("pane").unwrap());
    assert!(near(pane.w, 200.0), "the pane is {} wide", pane.w);
    assert!(near(l.rect(l.find("rest").unwrap()).w, 590.0));

    // Folding everything inside it is still nothing to show, whichever way it
    // stacks what it is showing.
    l.collapse(l.find("upper").unwrap());
    l.collapse(l.find("lower").unwrap());
    l.solve();
    assert_invariants(&l);
    assert!(
        near(l.rect(l.find("pane").unwrap()).w, 0.0),
        "the pane is {} wide",
        l.rect(l.find("pane").unwrap()).w
    );
}
