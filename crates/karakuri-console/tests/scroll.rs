//! Tests for Inspector pane scrolling, layout clipping, and visible range readouts.

mod common;

use common::{at, drawn_once, near, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    count_text, inspector, pane_count, InspectorPane, Node, NodeAuthority, Pane, Param, View, SYNCS,
};
use karakuri_operation::{Authority, Layer, NodeAddress, ParamAt, Sync};

/// A row over `[0, 1]` at the middle of it, so every knob in these panes is at
/// the same place across its row: a fixed point is on whichever row is under
/// it, which is what test 3 needs to be about the scroll rather than about two
/// values.
fn row(ord: usize, name: &str) -> Param {
    Param {
        ord: Some(ord),
        name: name.to_owned(),
        value: 0.5,
        range: [0.0, 1.0],
        param: ParamAt {
            node: Some(NodeAddress {
                layer: Layer::L1,
                index: 0,
            }),
            key: name.to_owned(),
        },
        bound: None,
    }
}

/// A node of `params` rows and no renderers, named so a failure says which.
fn node(index: usize, params: usize) -> Node {
    Node {
        keep: None,
        addr: format!("L1:{index}"),
        name: format!("node_{index}"),
        authority: Some(NodeAuthority {
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            level: Authority::Manual,
        }),
        uses: Vec::new(),
        renderers: Vec::new(),
        params: (0..params)
            .map(|at| row(at + 1, &format!("n{index}p{at}")))
            .collect(),
    }
}

/// A pane of `groups` node groups, each `params` rows deep.
fn pane_of(groups: usize, params: usize) -> Pane {
    Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: Sync::Beat,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: false,
        // Not this test's row: the deck head's two build chips are
        // drawn from this and nothing here is about them.
        aimed: None,
        nodes: (0..groups).map(|index| node(index, params)).collect(),
    }
}

/// A view with one deck's pane behind it, at a scroll position.
fn view_at(pane: &Pane, scroll: f32) -> View {
    let mut view = View::new(Room::Day);
    view.inspector = vec![pane.clone()];
    view.scroll_by(0, scroll);
    view
}

use common::console_panel as console;

/// The laid-out pane, at the position the view is holding.
fn laid(panel: &karakuri_console::panel::Panel, view: &View, pane: &Pane) -> InspectorPane {
    inspector(panel.layout(), 0, pane, view.scroll_in(0)).expect("a pane with room in it")
}

/// Where the `index`th row of the `group`th node goes, worked out from the
/// stylesheet's own numbers rather than asked of the crate — `param_fader.rs`'
/// helper, and here for its reason: a row drawn in the wrong place should be
/// two derivations disagreeing rather than one agreeing with itself.
fn row_rect(at: &InspectorPane, pane: &Pane, group: usize, index: usize) -> egui::Rect {
    let rect = at.group(&pane.nodes, group);
    let top = rect.min.y + size::NODE_HEAD_H + size::PARAM_H * index as f32;
    egui::Rect::from_min_max(
        egui::pos2(rect.min.x, top),
        egui::pos2(rect.max.x, top + size::PARAM_H),
    )
}

/// The centre of that row's knob, which is the middle of its track because
/// every row here is at the middle of its range.
fn knob(at: &InspectorPane, pane: &Pane, group: usize, index: usize) -> egui::Pos2 {
    let rect = row_rect(at, pane, group, index);
    let left = rect.min.x + size::PARAM_PAD_L;
    let right = rect.max.x - size::PARAM_PAD_R;
    let track_min =
        left + size::PARAM_ORD_W + size::PARAM_GAP + size::PARAM_NAME_W + size::PARAM_GAP;
    let track_max = right - size::PARAM_VAL_W - size::PARAM_GAP;
    egui::pos2((track_min + track_max) * 0.5, rect.center().y)
}

// ---------------------------------------------------------------------------
// 1. Nothing to scroll through
// ---------------------------------------------------------------------------

/// A pane taller than its content draws without scrolling and clamps scroll position to zero.
#[test]
fn a_pane_taller_than_its_content_draws_it_unscrolled() {
    let pane = pane_of(3, 3);
    let panel = console(PLAUSIBLE);

    let at_top = laid(&panel, &view_at(&pane, 0.0), &pane);
    assert!(
        at_top.content <= at_top.body.height(),
        "this test needs a pane with room to spare: {} of content in {} of body",
        at_top.content,
        at_top.body.height()
    );

    // And the same pane with the wheel spun as far as it goes.
    let spun = view_at(&pane, 10_000.0);
    assert!(
        spun.scroll_in(0) > 0.0,
        "the store refused a position, so this test cannot tell a clamp from a refusal"
    );
    let at_spun = laid(&panel, &spun, &pane);

    for (which, at) in [("at rest", at_top), ("spun", at_spun)] {
        assert!(
            near(at.scroll, 0.0),
            "{which}: the pane is scrolled to {} and there is nothing under it to reach",
            at.scroll
        );
        assert_eq!(at.shown, pane.nodes.len(), "{which}: every group is whole");
        assert_eq!(
            at.drawn(&pane.nodes),
            0..pane.nodes.len(),
            "{which}: every group is drawn"
        );
        assert!(
            near(at.group(&pane.nodes, 0).min.y, at.body.min.y),
            "{which}: the first group does not start at the top of the body"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. The end of the list
// ---------------------------------------------------------------------------

/// Scrolling to the end displays the final group and clamps against `content - body`.
#[test]
fn a_short_pane_scrolled_to_the_end_draws_the_last_group_and_not_the_first() {
    let pane = pane_of(3, 3);
    let panel = console(SMALLEST);
    let last = pane.nodes.len() - 1;

    let at_top = laid(&panel, &view_at(&pane, 0.0), &pane);
    assert!(
        at_top.content > at_top.body.height(),
        "this test needs a pane that overflows: {} of content in {} of body",
        at_top.content,
        at_top.body.height()
    );
    assert!(
        at_top.drawn(&pane.nodes).contains(&0),
        "the first group is drawn before anything is scrolled"
    );
    assert!(
        !at_top.drawn(&pane.nodes).contains(&last),
        "the last group is already on screen, so scrolling to it proves nothing"
    );

    let at_end = laid(&panel, &view_at(&pane, 10_000.0), &pane);
    assert!(
        near(at_end.scroll, at_end.content - at_end.body.height()),
        "the end is {} and the pane stopped at {}",
        at_end.content - at_end.body.height(),
        at_end.scroll
    );
    assert!(
        near(at_end.group(&pane.nodes, last).max.y, at_end.body.max.y),
        "the last group ends at {} and the body ends at {}",
        at_end.group(&pane.nodes, last).max.y,
        at_end.body.max.y
    );
    assert!(
        at_end.drawn(&pane.nodes).contains(&last),
        "the last group is not drawn at the end of the list"
    );
    assert!(
        !at_end.drawn(&pane.nodes).contains(&0),
        "the first group is still drawn with the pane scrolled to the end"
    );
}

// ---------------------------------------------------------------------------
// 3. What a press lands on
// ---------------------------------------------------------------------------

/// Presses land on currently visible rows; scrolled-off rows under the head are non-interactive.
#[test]
fn a_press_after_scrolling_lands_on_the_row_now_under_it() {
    let pane = pane_of(3, 3);
    let panel = console(SMALLEST);

    // The knob of the first row of the first group, before anything moves.
    let at_top = laid(&panel, &view_at(&pane, 0.0), &pane);
    let p = knob(&at_top, &pane, 0, 0);
    let first = at_top
        .grip(&pane, at(p))
        .expect("a press on the first row's knob");
    assert_eq!(first.param.name, "n0p0");

    // One parameter row down the list, and the point has not moved.
    let stepped = laid(&panel, &view_at(&pane, size::PARAM_H), &pane);
    assert!(
        near(stepped.scroll, size::PARAM_H),
        "the pane did not scroll one row: {}",
        stepped.scroll
    );
    let now = stepped
        .grip(&pane, at(p))
        .expect("a press on whatever is under that point now");
    assert_eq!(
        now.param.name, "n0p1",
        "the press landed on `{}`, which is the row that was there before it moved",
        now.param.name
    );

    // And far enough that the first row is under the deck head.
    let past = laid(
        &panel,
        &view_at(&pane, size::NODE_HEAD_H + size::PARAM_H * 1.5),
        &pane,
    );
    let hidden = knob(&past, &pane, 0, 0);
    assert!(
        hidden.y < past.body.min.y,
        "the guard on the assertion under it: the knob is at {} and the body starts at {}",
        hidden.y,
        past.body.min.y
    );
    assert!(
        past.grip(&pane, at(hidden)).is_none(),
        "a row scrolled up under the heads took a press where it is not drawn"
    );

    // Input claim routing delegates to grip ownership, rejecting scrolled-out knobs.
    let mut panel = console(SMALLEST);
    let view = view_at(&pane, size::NODE_HEAD_H + size::PARAM_H * 1.5);
    let ctx = drawn_once();
    assert_eq!(
        claim(&mut panel, &ctx, &view, at(knob(&past, &pane, 0, 0))),
        Claim::Egui,
        "the panel claimed a press on a knob that is scrolled under its heads"
    );
    assert_eq!(
        claim(&mut panel, &ctx, &view, at(knob(&past, &pane, 0, 2))),
        Claim::Panel,
        "the guard on the assertion above: a knob this pane *is* drawing is the panel's"
    );
}

// ---------------------------------------------------------------------------
// 4. What a resize does to it, which is nothing
// ---------------------------------------------------------------------------

/// Scroll position persists through pane resize without being rewritten or clamped into storage (P-0082, ADR-0250).
#[test]
fn a_position_survives_the_pane_growing_and_shrinking() {
    let pane = pane_of(3, 3);
    let short = console(SMALLEST);
    let tall = console(PLAUSIBLE);
    let view = view_at(&pane, 10_000.0);
    let stored = view.scroll_in(0);

    let before = laid(&short, &view, &pane);
    assert!(
        before.scroll > 0.0,
        "the short pane is not scrolled, so this test has nothing to lose"
    );
    assert!(
        stored > before.scroll,
        "the stored position {stored} is not past what the short pane can use, so a clamp \
         written back would not show"
    );

    let grown = laid(&tall, &view, &pane);
    assert!(
        near(grown.scroll, 0.0),
        "the tall pane holds everything and is scrolled to {}",
        grown.scroll
    );
    assert_eq!(
        view.scroll_in(0),
        stored,
        "growing the pane rewrote the stored position"
    );

    let after = laid(&short, &view, &pane);
    assert_eq!(
        view.scroll_in(0),
        stored,
        "shrinking the pane rewrote the stored position"
    );
    assert_eq!(
        after.scroll, before.scroll,
        "the pane came back to a different place than it left"
    );
    assert_eq!(
        after.drawn(&pane.nodes),
        before.drawn(&pane.nodes),
        "the same pane at the same size is drawing different groups"
    );
}

// ---------------------------------------------------------------------------
// 5. Saying how much
// ---------------------------------------------------------------------------

/// Head counts indicate partially shown content by tracking whole visible groups (rule 04).
#[test]
fn the_head_says_how_many_groups_are_not_shown() {
    let ctx = drawn_once();
    // One row per group, so a short pane holds one of them whole and the
    // count has somewhere to sit between 0 and 3.
    let pane = pane_of(3, 1);

    let tall = console(PLAUSIBLE);
    let all = laid(&tall, &view_at(&pane, 0.0), &pane);
    assert_eq!(count_text(&all, &pane), "3 of 3");
    assert!(
        pane_count(&ctx, &all, &pane, None, None).is_some(),
        "a pane with room for its words drew no count"
    );

    let short = console(SMALLEST);
    let some = laid(&short, &view_at(&pane, 0.0), &pane);
    assert!(
        some.drawn(&pane.nodes).len() > some.shown,
        "this pane draws {} groups and calls {} of them whole, so the count says nothing the \
         walk does not",
        some.drawn(&pane.nodes).len(),
        some.shown
    );
    assert_eq!(
        count_text(&some, &pane),
        format!("{} of 3", some.shown),
        "the count is not what the pane is showing whole"
    );
    assert!(
        some.shown < pane.nodes.len(),
        "the short pane is showing everything, so there is nothing for it to say"
    );
    let rect = pane_count(&ctx, &some, &pane, None, None).expect("a count in the head");
    assert!(
        some.head.contains_rect(rect),
        "the count is drawn outside the head it is in"
    );
}
