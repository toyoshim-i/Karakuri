//! Naming a region, and what a name is worth: it addresses **one** node, of
//! either kind.

mod common;

use common::{assert_invariants, console_ids, named_console, rects};
use karakuri_layout::{Axis, Hit, Layout, Point, Rect, Spec};

fn at(l: &mut Layout, w: f32, h: f32) {
    l.set_viewport(Rect::new(0.0, 0.0, w, h));
    l.solve();
}

#[test]
fn find_resolves_a_named_split_a_named_view_and_nothing_else() {
    let mut l = named_console();
    at(&mut l, 1280.0, 720.0);
    let ids = console_ids(&l);

    // A split answers to its name exactly as a view does, and the id is the
    // same one the arrangement's structure gives.
    assert_eq!(l.find("panes"), Some(ids.panes));
    assert_eq!(l.find("left-pane"), Some(ids.left));
    assert_eq!(l.find("centre-pane"), Some(ids.centre));
    assert_eq!(l.find("right-pane"), Some(ids.right));
    assert_eq!(l.name(ids.left), Some("left-pane"));
    assert_eq!(l.axis(ids.left), Some(Axis::Column));

    // Views are unaffected, and a name that is nobody's resolves to nothing.
    assert_eq!(l.name(l.find("library").unwrap()), Some("library"));
    assert_eq!(l.find("left"), None);
    assert_eq!(l.find(""), None);

    // A split the arrangement did not name carries no name and answers to
    // none, which is the default and most of them.
    let stack = Layout::new(Spec::row(
        2.0,
        vec![Spec::view("a"), Spec::view("b").named("c")],
    ));
    assert_eq!(stack.name(stack.root()), None);
    // `named` on a view replaces the name it was built with rather than
    // adding a second one to answer to.
    assert_eq!(stack.find("c"), Some(stack.children(stack.root())[1]));
    assert_eq!(stack.find("b"), None);
}

#[test]
fn folding_a_named_split_takes_its_whole_subtree_off_screen() {
    // The operation ADR-0156 is written around: the console's left pane is a
    // split holding the library and the staging lane, and "fold the left pane
    // away" reaches it by name.
    let mut l = named_console();
    at(&mut l, 1280.0, 720.0);
    let ids = console_ids(&l);
    let library = l.find("library").unwrap();
    let staging = l.find("staging").unwrap();
    let before = rects(&l);

    let pane = l.find("left-pane").unwrap();
    l.collapse(pane);
    l.solve();
    assert_invariants(&l);

    // The pane itself, and everything under it: not visible, and taking no
    // space at all.
    assert!(!l.visible(pane) && !l.visible(library) && !l.visible(staging));
    assert_eq!(l.rect(pane).w, 0.0);
    assert_eq!(l.rect(library).w, 0.0);
    assert_eq!(l.rect(staging).w, 0.0);

    // Its 240 and the divider beside it went to the centre, and where the
    // library used to be is now the program.
    assert_eq!(l.rect(ids.centre).w, 712.0 + 240.0 + 4.0);
    assert_eq!(
        l.hit(Point::new(100.0, 300.0), 3.0),
        Hit::View(l.find("program").unwrap())
    );

    // The subtree is gone from the arrangement on screen, not from the
    // arrangement: it is still addressable, and unfolding restores it exactly.
    assert_eq!(l.find("library"), Some(library));
    l.expand(pane);
    l.solve();
    assert_eq!(rects(&l), before);
}

#[test]
#[should_panic(expected = "two nodes are named \"library\"")]
fn two_views_with_one_name_are_refused() {
    Layout::new(Spec::row(
        2.0,
        vec![
            Spec::view("library"),
            Spec::view("staging"),
            Spec::view("library"),
        ],
    ));
}

#[test]
#[should_panic(expected = "two nodes are named \"left-pane\"")]
fn a_split_and_a_view_with_one_name_are_refused() {
    // The collision the optional name makes possible, and the one a caller is
    // most likely to write: the pane and the view it holds, called the same
    // thing.
    Layout::new(
        Spec::column(4.0, vec![Spec::view("left-pane"), Spec::view("staging")]).named("left-pane"),
    );
}

#[test]
fn a_saved_arrangement_that_repeats_a_name_fails_to_load_rather_than_panicking() {
    let mut l = named_console();
    at(&mut l, 1280.0, 720.0);
    let json = serde_json::to_string(&l).unwrap();

    // The negative control: this arrangement loads, so what the assertion
    // below catches is the repeated name and not a format that rejects
    // everything.
    let back: Layout = serde_json::from_str(&json).unwrap();
    assert_eq!(rects(&back), rects(&l));

    // A file is data rather than code, so the same rule that panics in
    // `Layout::new` is an error here.
    let clash = json.replace("staging", "library");
    let err = serde_json::from_str::<Layout>(&clash).unwrap_err();
    assert!(
        err.to_string().contains("two nodes are named \"library\""),
        "loaded, or failed for the wrong reason: {err}"
    );
}
