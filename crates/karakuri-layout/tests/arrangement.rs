//! What a solve produces, at every viewport it can be given.

mod common;

use common::{assert_invariants, console, console_ids, near, rects, simple, EPS};
use karakuri_layout::{Layout, Rect};

/// Viewports worth solving at: the ordinary one, degenerate ones, ones far
/// below the sum of the minima, and one that is not at the origin.
const VIEWPORTS: &[(f32, f32)] = &[
    (1280.0, 720.0),
    (2560.0, 1440.0),
    (800.0, 600.0),
    (320.0, 240.0),
    (40.0, 900.0),
    (900.0, 40.0),
    (1.0, 1.0),
    (0.0, 0.0),
];

fn at(l: &mut Layout, w: f32, h: f32) {
    l.set_viewport(Rect::new(0.0, 0.0, w, h));
    l.solve();
}

#[test]
fn children_tile_their_parent() {
    let mut l = console();
    at(&mut l, 1280.0, 720.0);
    assert_invariants(&l);

    // And the arrangement is the one the numbers say it is: a 48 transport, a
    // 24 status and 4-thick dividers leave 640 for the row of panes.
    let ids = console_ids(&l);
    assert_eq!(
        l.rect(l.find("transport").unwrap()),
        Rect::new(0.0, 0.0, 1280.0, 48.0)
    );
    assert_eq!(l.rect(ids.panes), Rect::new(0.0, 52.0, 1280.0, 640.0));
    assert_eq!(
        l.rect(l.find("status").unwrap()),
        Rect::new(0.0, 696.0, 1280.0, 24.0)
    );
    assert_eq!(l.rect(ids.left), Rect::new(0.0, 52.0, 240.0, 640.0));
    assert_eq!(l.rect(ids.centre), Rect::new(244.0, 52.0, 712.0, 640.0));
    assert_eq!(l.rect(ids.right), Rect::new(960.0, 52.0, 320.0, 640.0));

    let mut s = simple();
    at(&mut s, 1000.0, 400.0);
    assert_invariants(&s);
    assert_eq!(
        s.rect(s.find("left").unwrap()),
        Rect::new(0.0, 0.0, 100.0, 400.0)
    );
    assert_eq!(
        s.rect(s.find("centre").unwrap()),
        Rect::new(102.0, 0.0, 836.0, 400.0)
    );
    assert_eq!(
        s.rect(s.find("right").unwrap()),
        Rect::new(940.0, 0.0, 60.0, 400.0)
    );
}

#[test]
fn no_rectangle_is_ever_negative() {
    let mut l = console();
    for (w, h) in VIEWPORTS {
        at(&mut l, *w, *h);
        assert_invariants(&l);
    }

    // A viewport given as a negative size is a caller's arithmetic, and it
    // still may not produce a negative rectangle.
    l.set_viewport(Rect::new(0.0, 0.0, -100.0, -50.0));
    l.solve();
    assert_invariants(&l);
    assert_eq!(l.viewport(), Rect::new(0.0, 0.0, 0.0, 0.0));
}

#[test]
fn a_visible_view_lies_inside_the_viewport() {
    let mut l = console();
    // Not at the origin: a solve that assumed one would pass every other test
    // here and fail this.
    l.set_viewport(Rect::new(37.0, 11.0, 900.0, 500.0));
    l.solve();
    assert_invariants(&l);

    let program = l.rect(l.find("program").unwrap());
    assert!(program.x >= 37.0 && program.y >= 11.0);
    assert!(program.x + program.w <= 937.0 + EPS && program.y + program.h <= 511.0 + EPS);
}

#[test]
fn a_child_stays_within_its_bounds_when_there_is_room() {
    let mut l = console();
    let ids = console_ids(&l);
    let transport = l.find("transport").unwrap();
    let status = l.find("status").unwrap();
    let program = l.find("program").unwrap();
    let inspector = l.find("inspector").unwrap();

    // The sum of the row's minima is 700 wide plus 8 of divider, and of the
    // column's 272 plus 8; everything below sweeps clear of both.
    let mut w = 800.0;
    while w <= 2600.0 {
        let mut h = 600.0;
        while h <= 1500.0 {
            at(&mut l, w, h);
            assert_invariants(&l);

            assert!(near(l.rect(transport).h, 48.0), "transport at {w}x{h}");
            assert!(near(l.rect(status).h, 24.0), "status at {w}x{h}");

            let left = l.rect(ids.left).w;
            assert!((160.0..=400.0).contains(&left), "left {left} at {w}x{h}");
            let right = l.rect(ids.right).w;
            assert!((220.0..=480.0).contains(&right), "right {right} at {w}x{h}");
            assert!(l.rect(ids.centre).w >= 320.0 - EPS, "centre at {w}x{h}");
            assert!(l.rect(program).h >= 120.0 - EPS, "program at {w}x{h}");
            assert!(l.rect(inspector).h >= 100.0 - EPS, "inspector at {w}x{h}");

            h += 137.0;
        }
        w += 173.0;
    }
}

#[test]
fn shrinking_below_the_minima_and_growing_back_reproduces_the_arrangement() {
    // The rule the crate is built around: a solve never writes back into the
    // model, so a viewport small enough to violate every minimum destroys
    // nothing. Exact equality, not near — nothing was recomputed from a
    // rounded intermediate, because nothing was recomputed at all.
    let mut l = console();
    let ids = console_ids(&l);
    at(&mut l, 1280.0, 720.0);
    let before = rects(&l);

    // Anchored to what the arrangement *declares*, not only to what it
    // happened to solve to. A layout that clamped its model to the space it
    // had would have done so during its very first solve, at the empty
    // viewport it is built with — before any baseline taken here — and a
    // round trip against that baseline would then agree with itself all the
    // way down.
    assert_eq!(l.rect(ids.left).w, 240.0);
    assert_eq!(l.rect(ids.right).w, 320.0);

    for (w, h) in VIEWPORTS {
        at(&mut l, *w, *h);
        assert_invariants(&l);
    }

    at(&mut l, 1280.0, 720.0);
    assert_eq!(l.rect(ids.left).w, 240.0);
    assert_eq!(l.rect(ids.right).w, 320.0);
    assert_eq!(rects(&l), before);
}

#[test]
fn a_drag_survives_the_viewport_collapsing_and_coming_back() {
    // The same rule, with an operator's edit in it: what has to come back is
    // where they dragged the divider, not where it started.
    let mut l = console();
    at(&mut l, 1280.0, 720.0);
    let ids = console_ids(&l);
    l.set_divider(ids.panes, 0, 310.0);
    let dragged = rects(&l);

    at(&mut l, 0.0, 0.0);
    at(&mut l, 12.0, 8.0);
    at(&mut l, 1280.0, 720.0);

    assert_eq!(rects(&l), dragged);
}

#[test]
fn a_collapsed_child_takes_no_space_and_its_siblings_take_all_of_it() {
    let mut l = console();
    at(&mut l, 1280.0, 720.0);
    let ids = console_ids(&l);
    let staging = l.find("staging").unwrap();
    let library = l.find("library").unwrap();

    l.collapse(ids.right);
    l.collapse(staging);
    l.solve();
    assert_invariants(&l);

    // No extent, and — since the assertion above counts them — no divider
    // beside it either. The right pane's 320 and the divider that separated it
    // both go to the centre.
    assert_eq!(l.rect(ids.right).w, 0.0);
    assert_eq!(l.rect(staging).h, 0.0);
    assert_eq!(l.rect(ids.centre).w, 712.0 + 320.0 + 4.0);
    assert_eq!(l.rect(library).h, l.rect(ids.left).h);
    assert!(!l.visible(staging));
    assert!(l.visible(library));
}

#[test]
fn the_solve_is_a_function_of_the_state_and_not_of_the_route_to_it() {
    // Determinism is worth little if the state itself remembers where it has
    // been. These two layouts are in the same state by different routes — one
    // of them through viewports nothing fits in and a drag that was undone —
    // and a solve is a function of the state alone, so they are the same
    // layout. The drag happens while there is something to drag: a drag is an
    // edit wherever it is made, and one made at a viewport showing nothing is
    // an edit to nothing, which is a different state and not a different
    // route.
    let mut long = console();
    at(&mut long, 1000.0, 700.0);
    let ids = console_ids(&long);
    long.collapse(long.find("staging").unwrap());
    long.set_divider(ids.panes, 0, 300.0);
    long.set_divider(ids.panes, 0, 240.0);
    at(&mut long, 40.0, 30.0);
    at(&mut long, 0.0, 0.0);
    long.solo(long.find("program").unwrap());
    long.unsolo();
    at(&mut long, 1920.0, 1080.0);

    let mut direct = console();
    direct.collapse(direct.find("staging").unwrap());
    at(&mut direct, 1920.0, 1080.0);

    assert_eq!(rects(&long), rects(&direct));

    // The same inputs twice over are the same rectangles, and solving again
    // changes nothing: the cache is a cache.
    let mut again = console();
    again.collapse(again.find("staging").unwrap());
    at(&mut again, 1920.0, 1080.0);
    assert_eq!(rects(&again), rects(&direct));
    let once = rects(&again);
    again.solve();
    assert_eq!(rects(&again), once);
}

#[test]
fn a_serde_round_trip_solves_identically() {
    let mut l = console();
    at(&mut l, 1280.0, 720.0);
    let ids = console_ids(&l);
    l.set_divider(ids.panes, 0, 330.0);
    l.collapse(l.find("inspector").unwrap());
    l.solve();
    let before = rects(&l);

    let json = serde_json::to_string(&l).unwrap();
    // An unbounded maximum has no spelling as a JSON number, so it is written
    // as an absence. Asserted here because a format that silently dropped it
    // would only be noticed by a layout that failed to load.
    assert!(
        json.contains("null"),
        "an unbounded max should round trip: {json}"
    );

    let mut back: Layout = serde_json::from_str(&json).unwrap();
    back.solve();
    assert_eq!(rects(&back), before);

    // The viewport and the collapsed state came with it, so it needs no
    // second setup call to be the same layout.
    assert_eq!(back.viewport(), l.viewport());
    assert!(back.is_collapsed(back.find("inspector").unwrap()));
    assert_invariants(&back);
}

#[test]
fn a_layout_of_one_view_is_the_viewport() {
    // The degenerate arrangement, which is also the shape a solo reduces to:
    // no split, no divider, nothing to distribute.
    let mut l = Layout::new(karakuri_layout::Spec::view("only"));
    at(&mut l, 640.0, 480.0);
    assert_invariants(&l);
    assert_eq!(l.rect(l.root()), Rect::new(0.0, 0.0, 640.0, 480.0));
    assert_eq!(l.find("only"), Some(l.root()));
    assert!(l.children(l.root()).is_empty());
    assert_eq!(l.axis(l.root()), None);
}

#[test]
fn a_split_with_no_children_is_an_empty_region_rather_than_a_special_case() {
    // `Spec::row(_, vec![])` is expressible, so the question is what it means
    // rather than whether to refuse it — and the solve already answers: a
    // childless split claims its extent like any other node, hands none of it
    // on, and hits as nothing. There is no divider to draw, which is why every
    // count of gaps here is `visible - 1` saturated at zero rather than a
    // subtraction that would go negative on exactly this arrangement.
    let mut l = Layout::new(karakuri_layout::Spec::row(
        4.0,
        vec![
            karakuri_layout::Spec::view("side").fixed(100.0),
            karakuri_layout::Spec::row(4.0, vec![]).flex(1.0),
        ],
    ));
    at(&mut l, 640.0, 480.0);
    assert_invariants(&l);

    let empty = l.children(l.root())[1];
    assert!(l.children(empty).is_empty());
    assert_eq!(l.rect(empty), Rect::new(104.0, 0.0, 536.0, 480.0));
    assert_eq!(
        l.hit(karakuri_layout::Point::new(300.0, 100.0), 3.0),
        karakuri_layout::Hit::Nothing
    );

    // And on its own it is the whole viewport, still with nothing in it.
    let mut lone = Layout::new(karakuri_layout::Spec::column(4.0, vec![]));
    at(&mut lone, 640.0, 480.0);
    assert_invariants(&lone);
    assert_eq!(lone.rect(lone.root()), Rect::new(0.0, 0.0, 640.0, 480.0));
}
