//! Dragging, folding, soloing, and what a point is touching.

mod common;

use common::{assert_invariants, console, console_ids, near, rects, simple, stack};
use karakuri_layout::{Hit, Point, Rect};

#[test]
fn a_drag_stops_at_the_first_constraint_and_stays_there() {
    let mut l = simple();
    l.set_viewport(Rect::new(0.0, 0.0, 1000.0, 400.0));
    l.solve();
    let root = l.root();

    // `left` is capped at 150, so a drag to 400 lands at 150 and says so.
    let landed = l.set_divider(root, 0, 400.0);
    assert!(near(landed, 150.0), "landed at {landed}");
    assert!(near(l.rect(l.find("left").unwrap()).w, 150.0));
    let stopped = rects(&l);

    // Dragging further past changes nothing at all — which is the property an
    // absolute position buys over a delta, where each frame past the stop would
    // contribute drift the clamp threw away.
    for further in [500.0, 900.0, 4000.0] {
        let again = l.set_divider(root, 0, further);
        assert!(near(again, 150.0), "second drag landed at {again}");
        assert_eq!(rects(&l), stopped);
    }

    // And the same at the other end, where `left`'s minimum is the stop.
    let landed = l.set_divider(root, 0, -300.0);
    assert!(near(landed, 50.0), "landed at {landed}");
    assert_eq!(l.set_divider(root, 0, -900.0), landed);
    assert_invariants(&l);
}

#[test]
fn a_drag_out_and_back_reproduces_the_arrangement() {
    // A fixed neighbour, and a flexible one.
    for (mut l, size, out, home) in [
        (simple(), (1000.0, 400.0), 130.0, 100.0),
        (stack(), (300.0, 306.0), 250.0, 200.0),
    ] {
        l.set_viewport(Rect::new(0.0, 0.0, size.0, size.1));
        l.solve();
        let root = l.root();
        let before = rects(&l);

        assert!(near(l.set_divider(root, 0, out), out));
        assert_ne!(rects(&l), before);
        assert!(near(l.set_divider(root, 0, home), home));
        assert_eq!(rects(&l), before);
        assert_invariants(&l);
    }
}

#[test]
fn a_drag_never_pushes_through_to_a_further_neighbour() {
    let mut l = simple();
    l.set_viewport(Rect::new(0.0, 0.0, 1000.0, 400.0));
    l.solve();
    let root = l.root();
    let left = l.find("left").unwrap();
    let centre = l.find("centre").unwrap();
    let right = l.find("right").unwrap();

    // Boundary 1 is centre|right. Dragging it far left runs into `centre`'s
    // minimum of 80 — and stops there, rather than carrying on into `left`.
    let landed = l.set_divider(root, 1, 150.0);
    assert!(near(landed, 182.0), "landed at {landed}");
    assert!(near(l.rect(centre).w, 80.0));
    assert!(near(l.rect(left).w, 100.0), "the far neighbour moved");
    // The pair's combined extent is what it was: nothing else gave or took.
    assert!(near(l.rect(centre).w + l.rect(right).w, 836.0 + 60.0));
    assert_invariants(&l);
}

#[test]
fn collapse_then_expand_restores_the_exact_previous_size() {
    let mut l = console();
    l.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
    l.solve();
    let ids = console_ids(&l);
    // Drag first, so what has to come back is an operator's size rather than
    // the one the arrangement was built with.
    l.set_divider(ids.panes, 0, 288.0);
    let before = rects(&l);

    l.collapse(ids.left);
    l.solve();
    assert_invariants(&l);
    assert!(!l.visible(ids.left));

    l.expand(ids.left);
    l.solve();
    assert_eq!(rects(&l), before);

    // And `toggle` is the same operation, reporting where it left things.
    assert!(l.toggle(ids.left));
    assert!(!l.toggle(ids.left));
    l.solve();
    assert_eq!(rects(&l), before);
}

#[test]
fn solo_then_unsolo_restores_the_arrangement_including_what_was_already_collapsed() {
    let mut l = console();
    l.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
    l.solve();
    let ids = console_ids(&l);
    let program = l.find("program").unwrap();
    let staging = l.find("staging").unwrap();
    let master = l.find("master").unwrap();

    // An arrangement with folds of its own already in it. Restoring these is
    // the part a "collapse everything, then expand everything" solo gets wrong.
    l.collapse(staging);
    l.collapse(master);
    l.set_divider(ids.panes, 0, 300.0);
    l.solve();
    let before = rects(&l);

    l.solo(program);
    l.solve();
    assert!(l.is_soloed());
    assert_invariants(&l);
    assert!(!l.visible(ids.left) && !l.visible(ids.right));
    assert!(l.visible(program));

    // Soloing again aims elsewhere without saving again, so one unsolo is
    // still the way back. The right pane is capped at 480, and soloing
    // something inside it leaves it at 480 with the rest of the window empty:
    // a maximum is honoured rather than overridden by there being nobody else
    // on screen.
    l.solo(l.find("mixer").unwrap());
    l.solve();
    assert_invariants(&l);
    assert!(near(l.rect(ids.right).w, 480.0));

    l.unsolo();
    l.solve();
    assert!(!l.is_soloed());
    assert_eq!(rects(&l), before);
    assert!(l.is_collapsed(staging));
    assert!(l.is_collapsed(master));
    assert!(!l.is_collapsed(program));
}

#[test]
fn solo_leaves_the_soloed_view_holding_the_whole_viewport() {
    // "the panel folds away and only the picture is left, which is also how you
    // capture this window" — so the picture is the window, exactly, with no
    // divider and no residue of the rows it was nested in.
    let mut l = console();
    let viewport = Rect::new(0.0, 0.0, 1280.0, 720.0);
    l.set_viewport(viewport);
    l.solve();
    let program = l.find("program").unwrap();

    l.solo(program);
    l.solve();
    assert_invariants(&l);
    assert_eq!(l.rect(program), viewport);

    // Soloing the root is soloing nothing, since everything is on the path.
    let before = rects(&l);
    l.unsolo();
    l.solve();
    let open = rects(&l);
    l.solo(l.root());
    l.solve();
    assert_eq!(rects(&l), open);
    assert_ne!(open, before);
}

#[test]
fn a_divider_is_grabbed_from_wider_than_it_is_drawn() {
    let mut l = simple();
    l.set_viewport(Rect::new(0.0, 0.0, 1000.0, 400.0));
    l.solve();
    let root = l.root();
    let left = l.find("left").unwrap();
    let centre = l.find("centre").unwrap();
    let divider = Hit::Divider {
        split: root,
        index: 0,
    };

    // The divider is drawn across [100, 102]; with a grab of 4 it is caught
    // from [96, 106], and a point in there belongs to it rather than to the
    // view under it.
    assert_eq!(l.hit(Point::new(101.0, 200.0), 4.0), divider);
    assert_eq!(l.hit(Point::new(97.0, 200.0), 4.0), divider);
    assert_eq!(l.hit(Point::new(105.0, 200.0), 4.0), divider);
    // Beyond it, the views.
    assert_eq!(l.hit(Point::new(90.0, 200.0), 4.0), Hit::View(left));
    assert_eq!(l.hit(Point::new(110.0, 200.0), 4.0), Hit::View(centre));
    // With no grab at all it is only the two pixels it draws.
    assert_eq!(l.hit(Point::new(97.0, 200.0), 0.0), Hit::View(left));
    assert_eq!(l.hit(Point::new(101.0, 200.0), 0.0), divider);
}

#[test]
fn hit_finds_a_nested_divider_and_nothing_outside_the_viewport() {
    let mut l = console();
    l.set_viewport(Rect::new(10.0, 20.0, 1280.0, 720.0));
    l.solve();
    let ids = console_ids(&l);
    let program = l.find("program").unwrap();
    let library = l.find("library").unwrap();

    // Inside the centre column, between program and inspector.
    let seam = l.rect(program).y + l.rect(program).h;
    assert_eq!(
        l.hit(Point::new(600.0, seam + 1.0), 3.0),
        Hit::Divider {
            split: ids.centre,
            index: 0
        }
    );
    assert_eq!(
        l.hit(Point::new(600.0, seam - 40.0), 3.0),
        Hit::View(program)
    );
    assert_eq!(l.hit(Point::new(100.0, 100.0), 3.0), Hit::View(library));

    // Outside, on every side.
    for p in [
        Point::new(5.0, 100.0),
        Point::new(100.0, 5.0),
        Point::new(2000.0, 100.0),
        Point::new(100.0, 2000.0),
        Point::new(-40.0, -40.0),
    ] {
        assert_eq!(l.hit(p, 8.0), Hit::Nothing, "at {p:?}");
    }

    // A collapsed pane is not a target, and the space it left belongs to
    // whoever took it.
    l.collapse(ids.left);
    l.solve();
    assert_eq!(l.hit(Point::new(12.0, 100.0), 3.0), Hit::View(program));
}
