//! **The Inspector's parameter fader: the first control this console has
//! inside a pane's body, and the first whose target is one of however many
//! rows a Set happened to publish.**
//!
//! Nine things, and the first three are why this is its own file rather than a
//! few more assertions in `deck_head.rs`:
//!
//! 1. Where a row is — `.node-group`'s head, the renderer row where there is
//!    one, and `.param`'s own height from there — measured off the group
//!    rectangle `InspectorPane::group` answers rather than off the derivation
//!    that draws it.
//! 2. Where the fader is across that row: `.param`'s `grid-template-columns:
//!    15px 88px 1fr 58px`, and the fader is the `1fr`.
//! 3. **That the knob a hand takes hold of is the knob a frame painted**,
//!    which is the whole reason the row's arithmetic was lifted out of
//!    `node_into`: two copies of where a knob is is a knob drawn where nothing
//!    can grab it.
//! 4. **That the knob is the target and the track is not** — `Mixer::grab`'s
//!    rule, met one bay along on a row the mock gives no tooltip to.
//! 5. That every row is its own control, and that a press names the row it
//!    landed on rather than the first one drawn.
//! 6. **That a wildcard row writes the wildcard**, and not the node whose
//!    group it was drawn in — [ADR-0286](../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md).
//! 7. That the position and the value are one map read both ways, over a
//!    published range that is not `[0, 1]`.
//! 8. **That a range of no width is drawn and not taken hold of**, which is
//!    `Grab::new`'s refusal read on the value axis.
//! 9. That a group the pane had no room for is not reachable by a press
//!    either, and that each pane names its own deck.
//!
//! None of it needs a window, a device or a disk. It needs `egui`'s fonts only
//! where a whole frame is painted — `common::drawn_once` — because the pane
//! asks the shaper for nothing: every box in it is the width of the pane or a
//! track of the mock's own grid.
//!
//! # Where this stops
//!
//! Everything here ends at the **grip**: which deck, which control, and the
//! track it took hold of. Turning that into a `Grab` is
//! `crate::panel::Knob`'s, and turning the `Grab` into
//! `Operation::WriteParam` and applying it to a deck is
//! `crates/karakuri/src/main.rs`'s — see `ParamGrip`, which carries the one
//! line that closes it. What is asserted here is the half that has to be right
//! before any of that means anything.

mod common;

use common::{drawn_once, near, PLAUSIBLE};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    inspector, InspectorPane, Node, Pane, Param, Renderer, View, PANES, SYNCS,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::{Authority, Layer, NodeAt, ParamAt, Sync};

/// A row addressed at one node, over a published range, at a value.
fn row(ord: usize, name: &str, at: Option<(Layer, u32)>, range: [f32; 2], value: f32) -> Param {
    Param {
        ord,
        name: name.to_owned(),
        value,
        range,
        param: ParamAt {
            node: at.map(|(layer, index)| NodeAt { layer, index }),
            key: name.to_owned(),
        },
    }
}

/// **The mock's deck A, with the two shapes of group on it.**
///
/// The first group is a plain node with three rows, one of which has a
/// published range of no width. The second is the renderers' — a `.rend-row`
/// between the head and the rows, which is the offset a row's place has to
/// carry — and one of its two rows is a **wildcard**, which is what the whole
/// of a default interface is made of and is the case ADR-0286 is about.
///
/// The values are apart from each other, none of them is at an end of its own
/// range, and no two rows share a range — so a press answered off the wrong
/// row, or a value read against the wrong range, says so.
fn mock() -> Pane {
    Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: Sync::Beat,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.25,
        composite: true,
        nodes: vec![
            Node {
                addr: "L1:0".to_owned(),
                name: "drift_shell".to_owned(),
                authority: Some(Authority::Manual),
                renderers: Vec::new(),
                params: vec![
                    row(1, "radius", Some((Layer::L1, 0)), [0.0, 4.0], 2.4),
                    row(2, "turbulence", Some((Layer::L1, 0)), [0.1, 2.4], 1.4),
                    // A published range that narrowed to nothing: one
                    // position, so the row is drawn and is not a handle.
                    row(3, "pinned", Some((Layer::L1, 0)), [1.0, 1.0], 1.0),
                ],
            },
            Node {
                addr: "L4".to_owned(),
                name: "renderers".to_owned(),
                authority: None,
                renderers: vec![
                    Renderer {
                        name: "soft_points".to_owned(),
                        live: true,
                    },
                    Renderer {
                        name: "strand_strokes".to_owned(),
                        live: false,
                    },
                ],
                params: vec![
                    // Published bare: one control over every node that
                    // declares `exposure`, drawn in this group because it
                    // covers exactly one of them today.
                    row(4, "exposure", None, [0.5, 2.0], 1.1),
                    // A component of a `vec3`, which is a key like any other
                    // (ADR-0268).
                    row(5, "glow.x", Some((Layer::L4, 0)), [0.0, 4.0], 0.4),
                ],
            },
        ],
    }
}

/// A view with a deck behind it — two panes — which is what `View::draw`
/// paints from and what a press is resolved against, one value.
fn view(panes: &[Pane]) -> View {
    let mut view = View::new(Room::Day);
    view.inspector = panes.to_vec();
    view
}

/// A panel at a viewport, solved.
fn console(viewport: Rect) -> karakuri_console::panel::Panel {
    let mut panel = karakuri_console::panel::Panel::new(viewport.w, viewport.h);
    panel.solve();
    panel
}

/// The laid-out pane, which is what every test here starts from.
fn pane_at(panel: &karakuri_console::panel::Panel, index: usize, pane: &Pane) -> InspectorPane {
    inspector(panel.layout(), index, pane).expect("a pane with room in it")
}

/// A `karakuri_layout` point, from `egui`'s. Named for the press rather than
/// for the point, because `at` is the laid-out pane in most of these.
fn at_point(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// **Where the `index`th row of the `group`th node goes, worked out here from
/// the stylesheet's own numbers** rather than asked of the crate — so that a
/// row drawn in the wrong place is two derivations disagreeing rather than one
/// agreeing with itself.
fn row_rect(at: &InspectorPane, pane: &Pane, group: usize, index: usize) -> egui::Rect {
    let node = &pane.nodes[group];
    let rect = at.group(&pane.nodes, group);
    let top = rect.min.y
        + size::NODE_HEAD_H
        + match node.renderers.is_empty() {
            true => 0.0,
            false => size::REND_ROW_H,
        }
        + size::PARAM_H * index as f32;
    egui::Rect::from_min_max(
        egui::pos2(rect.min.x, top),
        egui::pos2(rect.max.x, top + size::PARAM_H),
    )
}

/// **Where the knob of that row sits**, from the row's own rectangle and the
/// value the row is carrying — `.param`'s four tracks, with the fader taking
/// what is left between the name and the figure.
fn knob(at: &InspectorPane, pane: &Pane, group: usize, index: usize) -> egui::Rect {
    let param = &pane.nodes[group].params[index];
    let rect = row_rect(at, pane, group, index);
    let left = rect.min.x + size::PARAM_PAD_L;
    let right = rect.max.x - size::PARAM_PAD_R;
    let track_min =
        left + size::PARAM_ORD_W + size::PARAM_GAP + size::PARAM_NAME_W + size::PARAM_GAP;
    let track_max = right - size::PARAM_VAL_W - size::PARAM_GAP;
    let [low, high] = param.range;
    let along = match high > low {
        false => 0.0,
        true => (param.value - low) / (high - low),
    };
    egui::Rect::from_center_size(
        egui::pos2(track_min + (track_max - track_min) * along, rect.center().y),
        egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
    )
}

/// **Every shape a whole frame put wholly inside `rect`**, as bounds —
/// `fader.rs`'s helper, and here for its reason: what a value moves is where a
/// mark is, and a failure that prints rectangles is one a reader can act on.
fn drawn(
    panel: &mut karakuri_console::panel::Panel,
    panes: &[Pane],
    rect: egui::Rect,
) -> Vec<egui::Rect> {
    let ctx = drawn_once();
    let mut view = view(panes);
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .map(|clipped| clipped.shape.visual_bounding_rect())
        .filter(|bounds| bounds.is_finite() && rect.contains_rect(*bounds))
        .collect()
}

// ---------------------------------------------------------------------------
// Where a row is, and where its fader is across it
// ---------------------------------------------------------------------------

/// **A row is its group's head, the renderer row where there is one, and
/// `.param`'s own height from there** — and the last row of a group ends
/// exactly where the group does, which is `group_h` and the row's offset being
/// one statement.
#[test]
fn a_row_is_where_the_group_and_the_stylesheet_put_it() {
    let pane = mock();
    let panel = console(PLAUSIBLE);
    let at = pane_at(&panel, 0, &pane);
    assert_eq!(at.shown, pane.nodes.len(), "both groups fit at this size");

    for (group, node) in pane.nodes.iter().enumerate() {
        let rect = at.group(&pane.nodes, group);
        let first = row_rect(&at, &pane, group, 0);
        let head = match node.renderers.is_empty() {
            true => size::NODE_HEAD_H,
            false => size::NODE_HEAD_H + size::REND_ROW_H,
        };
        assert!(
            near(first.min.y, rect.min.y + head),
            "group {group}'s first row starts under its head and its renderer row"
        );
        let last = row_rect(&at, &pane, group, node.params.len() - 1);
        assert!(
            near(last.max.y, rect.max.y),
            "group {group}'s last row ends where the group does"
        );
    }
}

/// **The fader is `.param`'s `1fr`**: the ordinal, the name and the figure are
/// stated widths and the track is what is left between them, with the knob on
/// the fill's moving edge.
#[test]
fn the_fader_is_the_track_left_between_the_name_and_the_figure() {
    let pane = mock();
    let panel = console(PLAUSIBLE);
    let at = pane_at(&panel, 0, &pane);
    let rect = row_rect(&at, &pane, 0, 0);

    let grip = at
        .grip(&pane, at_point(knob(&at, &pane, 0, 0).center()))
        .expect("the first row's knob");
    let track = grip.fader.track;

    assert!(near(track.height(), size::FADER_H));
    assert!(near(track.center().y, rect.center().y));
    assert!(near(
        track.min.x,
        rect.min.x
            + size::PARAM_PAD_L
            + size::PARAM_ORD_W
            + size::PARAM_GAP
            + size::PARAM_NAME_W
            + size::PARAM_GAP
    ));
    assert!(near(
        track.max.x,
        rect.max.x - size::PARAM_PAD_R - size::PARAM_VAL_W - size::PARAM_GAP
    ));
    // The knob is centred on the fill's moving edge, which is what stops the
    // fill and the handle disagreeing about one number.
    assert!(near(grip.fader.knob.center().x, grip.fader.fill.max.x));
    assert!(near(grip.fader.travel, track.width()));
}

/// **The knob a hand takes hold of is the knob a frame painted.**
///
/// The row's arithmetic used to be a running sum inside the derivation that
/// paints it, so a press had nothing to ask. This paints a whole frame and
/// looks for the grabbed rectangle among what landed in the row.
#[test]
fn the_knob_that_is_grabbed_is_the_knob_that_is_painted() {
    let pane = mock();
    let mut panel = console(PLAUSIBLE);
    let at = pane_at(&panel, 0, &pane);
    let rect = row_rect(&at, &pane, 0, 0);

    let grip = at
        .grip(&pane, at_point(knob(&at, &pane, 0, 0).center()))
        .expect("the first row's knob");
    let taken = grip.fader.knob;

    let painted = drawn(&mut panel, &[pane.clone(), pane.clone()], rect);
    assert!(
        painted
            .iter()
            .any(|bounds| near(bounds.center().x, taken.center().x)
                && near(bounds.center().y, taken.center().y)
                && near(bounds.width(), taken.width())),
        "nothing the size of the grabbed knob was painted in that row: {painted:?}"
    );
}

// ---------------------------------------------------------------------------
// The knob, and not the track
// ---------------------------------------------------------------------------

/// **A press on the knob takes it; a press on the track does nothing.**
///
/// `Mixer::grab`'s rule, met one bay along: a parameter at 0.6 whose track was
/// clicked would jump to the far end of its published range, on stage, because
/// a hand landed three pixels off a handle.
#[test]
fn the_knob_is_grabbed_and_the_track_is_not() {
    let pane = mock();
    let panel = console(PLAUSIBLE);
    let at = pane_at(&panel, 0, &pane);

    let handle = knob(&at, &pane, 0, 0);
    assert!(at.owns(&pane, at_point(handle.center())));

    let grip = at.grip(&pane, at_point(handle.center())).expect("the knob");
    let track = grip.fader.track;
    let off = [
        egui::pos2(track.min.x + 1.0, track.center().y),
        egui::pos2(track.max.x - 1.0, track.center().y),
    ];
    for p in off {
        // The guard on the assertion: a point that turned out to be *on* the
        // knob would make it pass for the wrong reason.
        assert!(!handle.contains(p), "{p:?} is on the knob");
        assert!(
            at.grip(&pane, at_point(p)).is_none(),
            "a press at {p:?} took hold of something, and it is not a knob"
        );
        assert!(!at.owns(&pane, at_point(p)));
    }
}

/// **Nothing else in the pane is a control of this derivation's** — the two
/// heads above the body, a node head, the renderer row, and the ordinal, the
/// name and the figure of a row.
#[test]
fn a_press_anywhere_else_in_the_pane_takes_hold_of_nothing() {
    let pane = mock();
    let panel = console(PLAUSIBLE);
    let at = pane_at(&panel, 0, &pane);
    let rect = row_rect(&at, &pane, 0, 0);
    let group = at.group(&pane.nodes, 1);

    let elsewhere = [
        at.head.center(),
        at.deck_head.center(),
        // The node head and the renderer row of the second group.
        egui::pos2(group.center().x, group.min.y + size::NODE_HEAD_H * 0.5),
        egui::pos2(
            group.center().x,
            group.min.y + size::NODE_HEAD_H + size::REND_ROW_H * 0.5,
        ),
        // The ordinal and the figure of the first row.
        egui::pos2(rect.min.x + 1.0, rect.center().y),
        egui::pos2(rect.max.x - 1.0, rect.center().y),
    ];
    for p in elsewhere {
        assert!(
            at.grip(&pane, at_point(p)).is_none(),
            "a press at {p:?} took hold of a parameter fader"
        );
    }
}

// ---------------------------------------------------------------------------
// Which control a press named
// ---------------------------------------------------------------------------

/// **Every row is its own control**, across both groups and across the
/// renderer row's offset — five rows, one of which is not a handle.
#[test]
fn each_row_is_its_own_control() {
    let pane = mock();
    let panel = console(PLAUSIBLE);
    let at = pane_at(&panel, 0, &pane);

    for (group, node) in pane.nodes.iter().enumerate() {
        for (index, param) in node.params.iter().enumerate() {
            let p = at_point(knob(&at, &pane, group, index).center());
            match param.range[1] > param.range[0] {
                false => assert!(
                    at.grip(&pane, p).is_none(),
                    "`{}` has a range of no width and was taken hold of",
                    param.name
                ),
                true => {
                    let grip = at
                        .grip(&pane, p)
                        .unwrap_or_else(|| panic!("`{}`'s knob", param.name));
                    assert_eq!(grip.param.param, param.param, "`{}`", param.name);
                    assert_eq!(grip.param.ord, param.ord);
                    assert_eq!(grip.deck, 0);
                }
            }
        }
    }
}

/// **A wildcard row writes the wildcard, and not the group it was drawn in.**
///
/// `exposure` is published bare — one control over every node that declares
/// the key — and is drawn under the renderers' head because it covers exactly
/// one of them today. ADR-0286: the placement is where the row goes and the
/// control is what the write names, and reading the placement back as the
/// address would narrow the control to the node it happens to reach.
#[test]
fn a_wildcard_row_writes_the_wildcard_and_not_the_group_it_was_drawn_in() {
    let pane = mock();
    let panel = console(PLAUSIBLE);
    let at = pane_at(&panel, 0, &pane);

    let grip = at
        .grip(&pane, at_point(knob(&at, &pane, 1, 0).center()))
        .expect("the wildcard row's knob");
    assert_eq!(grip.param.name, "exposure");
    assert_eq!(
        grip.param.param,
        ParamAt {
            node: None,
            key: "exposure".to_owned(),
        },
        "the row is drawn under L4 and the control still names no node"
    );

    // Its neighbour in the same group is addressed, so the answer above is the
    // control's own and not this group's.
    let grip = at
        .grip(&pane, at_point(knob(&at, &pane, 1, 1).center()))
        .expect("the component row's knob");
    assert_eq!(
        grip.param.param,
        ParamAt {
            node: Some(NodeAt {
                layer: Layer::L4,
                index: 0,
            }),
            key: "glow.x".to_owned(),
        }
    );
}

/// **Each pane names the deck it is pointed at**, which is `Pane::deck` and
/// not the deck selection.
#[test]
fn each_pane_names_its_own_deck() {
    let first = mock();
    let second = Pane { deck: 2, ..mock() };
    let panel = console(PLAUSIBLE);
    assert_eq!(PANES, 2, "the arrangement's two panes");

    for (index, pane) in [&first, &second].into_iter().enumerate() {
        let at = pane_at(&panel, index, pane);
        let grip = at
            .grip(pane, at_point(knob(&at, pane, 0, 0).center()))
            .expect("the first row's knob");
        assert_eq!(grip.deck, pane.deck as u8);
    }
}

// ---------------------------------------------------------------------------
// The value
// ---------------------------------------------------------------------------

/// **The position and the value are one map read both ways**, over a published
/// range that is not `[0, 1]` — and both ends are exactly reachable, which is
/// `Grab::value`'s own rule met on the value axis.
#[test]
fn the_position_and_the_value_are_one_map_read_both_ways() {
    let param = row(1, "radius", Some((Layer::L1, 0)), [0.5, 2.5], 1.0);

    assert!(near(param.at(), 0.25));
    assert!(near(param.valued(0.25), 1.0));
    assert!(near(param.valued(0.0), 0.5), "the bottom is exact");
    assert!(near(param.valued(1.0), 2.5), "the top is exact");
    // Past either end is clamped to it: a published range narrows and never
    // redefines, so there is nothing outside it to ask for.
    assert!(near(param.valued(-1.0), 0.5));
    assert!(near(param.valued(2.0), 2.5));

    // A value outside the range, which nothing in this workspace writes and a
    // harness could: the fill is clamped rather than drawn off the end.
    let over = Param {
        value: 9.0,
        ..param.clone()
    };
    assert!(near(over.at(), 1.0));

    // A range of no width is one position, and the fader sits at its start
    // rather than at a division by zero.
    let flat = row(1, "pinned", Some((Layer::L1, 0)), [1.0, 1.0], 1.0);
    assert!(near(flat.at(), 0.0));
    assert!(near(flat.valued(0.7), 1.0));
}

/// **The knob is where the value put it**, so the same row at two values is
/// two knobs — and a press at one of them does not find the other.
#[test]
fn the_knob_moves_with_the_value() {
    let panel = console(PLAUSIBLE);
    let low = mock();
    let mut high = mock();
    high.nodes[0].params[0].value = 3.6;

    let at = pane_at(&panel, 0, &low);
    let a = knob(&at, &low, 0, 0).center();
    let b = knob(&at, &high, 0, 0).center();
    assert!(b.x > a.x, "3.6 of [0, 4] is further along than 2.4");

    assert!(at.grip(&low, at_point(a)).is_some());
    assert!(at.grip(&high, at_point(b)).is_some());
    assert!(
        at.grip(&high, at_point(a)).is_none(),
        "the knob is no longer where the lower value left it"
    );
}

// ---------------------------------------------------------------------------
// What is not drawn is not reachable
// ---------------------------------------------------------------------------

/// **A group the pane had no room for is not reachable by a press either.**
///
/// `InspectorPane::shown` is how many groups fit **whole**, and a press
/// resolved past it would be a hand reaching a control that is not on screen.
/// The pane is narrowed down the column until only the first group is drawn,
/// and the second group's rows are asked for where they would have been.
#[test]
fn a_group_the_pane_had_no_room_for_is_not_reachable() {
    let pane = mock();
    let panel = console(PLAUSIBLE);
    let whole = pane_at(&panel, 0, &pane);
    assert_eq!(whole.shown, 2);
    let second = whole.group(&pane.nodes, 1);
    let clipped = InspectorPane { shown: 1, ..whole };

    for index in 0..pane.nodes[1].params.len() {
        let p = at_point(knob(&whole, &pane, 1, index).center());
        // The guard on the assertion under it, so a `None` there is about
        // `shown` rather than about the arithmetic that found the knob.
        assert!(
            second.contains(egui::pos2(p.x, p.y)),
            "row {index} of the second group is inside it"
        );
        assert!(
            whole.grip(&pane, p).is_some(),
            "row {index} is reachable on the pane that drew it"
        );
        assert!(
            clipped.grip(&pane, p).is_none(),
            "row {index} of a group the pane did not draw was reached by a press"
        );
    }
}

/// **A pane too narrow for a fader claims no press at all.**
///
/// `.param`'s four tracks are three stated widths and a `1fr`, and a pane
/// narrow enough that the three meet has no track left — which is ADR-0279's
/// own measurement (*"the fader is the `1fr` track and is drawn only where
/// what is left over is positive"*) asked as a control.
#[test]
fn a_pane_with_no_room_for_a_track_claims_nothing() {
    let pane = mock();
    let panel = console(PLAUSIBLE);
    let at = pane_at(&panel, 0, &pane);

    let narrow = InspectorPane {
        body: egui::Rect::from_min_max(
            at.body.min,
            egui::pos2(
                at.body.min.x
                    + size::PARAM_PAD_L
                    + size::PARAM_ORD_W
                    + size::PARAM_GAP
                    + size::PARAM_NAME_W
                    + size::PARAM_GAP
                    + size::PARAM_VAL_W
                    + size::PARAM_GAP
                    + size::PARAM_PAD_R,
                at.body.max.y,
            ),
        ),
        ..at
    };
    for (group, node) in pane.nodes.iter().enumerate() {
        for index in 0..node.params.len() {
            let rect = row_rect(&narrow, &pane, group, index);
            // Across the whole row rather than at one point: with no track
            // there is no knob to aim at, so what is asserted is that nothing
            // anywhere along it answers.
            for step in 0..=20 {
                let x = rect.min.x + rect.width() * step as f32 / 20.0;
                let p = at_point(egui::pos2(x, rect.center().y));
                assert!(
                    narrow.grip(&pane, p).is_none(),
                    "a press at {p:?} on a pane with no room for a track took hold of something"
                );
            }
        }
    }
}
