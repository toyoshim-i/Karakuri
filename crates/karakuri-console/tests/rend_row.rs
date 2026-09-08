//! **The renderer chips under a node group's head: the control that turns a
//! deck's fold into a choice.**
//!
//! Seven things:
//!
//! 1. Where the row is inside its group — under `.node-head`, and exactly as
//!    tall as `group_h` counted it, so the parameter rows under it start where
//!    the row ends.
//! 2. Where the chips are in it, as `.rend-row`'s own padding and gap lay them
//!    out, each as wide as the name in it.
//! 3. **That the chips clear every boundary's grab**, which is the deck head's
//!    arithmetic two rows up: `.rend-row`'s left padding is 12 against a
//!    `GRAB` of 6.
//! 4. That a press on a chip asks for **that** renderer, by its index in draw
//!    order — the numbering a `select` record uses.
//! 5. **That an overdrawn deck's chips are drawn and claimed by nothing**,
//!    which is the manual's *"Only where the deck composites"* answered by a
//!    state rather than by a missing row.
//! 6. **That a lone renderer is drawn and not claimed** — *"and holds two or
//!    more"*, the other half of the same sentence.
//! 7. That a group the pane had no room to draw is not pressable, which is
//!    `InspectorPane::shown` reaching a control.
//!
//! None of it needs a window, a device or a disk. It does need `egui`'s fonts,
//! because a chip is as wide as the name in it — see `common::drawn_once`.
//!
//! **What is not here and cannot be**: that `input::claim` gives the panel a
//! press on a chip. That is a row in `input::PROBES` and it is the
//! registration half of this control, which lives in files this test's author
//! does not own; until it lands a press here reaches `egui`.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{
    inspector, rend_chips, InspectorPane, Node, Pane, Renderer, PANE_NAMES, SYNCS,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::{Authority, Operation, Sync};

/// **The mock's own `L4 renderers` group**: three renderers folded under one
/// head, the first of them live, and no authority chip — a head standing over
/// three nodes is not one of them.
fn renderers() -> Node {
    Node {
        addr: "L4".to_owned(),
        name: "renderers".to_owned(),
        authority: None,
        renderers: ["soft_points", "strand_strokes", "spark_fountain"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| Renderer {
                name: name.to_owned(),
                live: index == 0,
            })
            .collect(),
        params: Vec::new(),
    }
}

/// **An ordinary group with no renderers**, which is what says the chips are
/// the renderers group's and not every group's. It carries no parameter rows:
/// what a `.param` is made of is another control's, and a test that named its
/// fields would fail for a reason that is not this row's.
fn shell() -> Node {
    Node {
        addr: "L1:0".to_owned(),
        name: "drift_shell".to_owned(),
        authority: Some(Authority::Manual),
        renderers: Vec::new(),
        params: Vec::new(),
    }
}

/// **The mock's own deck A**, compositing, with the renderers group first so
/// that a short pane still draws it.
fn mock() -> Pane {
    Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: Sync::Tempo,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: true,
        nodes: vec![renderers(), shell()],
    }
}

/// A panel at a viewport, solved, with a context that has drawn once.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// The laid-out pane, and the boxes of the chips in its first group.
fn chips(
    panel: &Panel,
    ctx: &egui::Context,
    index: usize,
    pane: &Pane,
) -> (InspectorPane, Vec<egui::Rect>) {
    let at_pane = inspector(panel.layout(), index, pane, 0.0).expect("a pane with room in it");
    assert!(
        at_pane.shown > 0,
        "the pane drew no group at all, so there is no renderer row in it to measure"
    );
    let group = at_pane.group(&pane.nodes, 0);
    let row = row_of(group);
    let boxes = rend_chips(ctx, row, &pane.nodes[0].renderers)
        .map(|(_, chip)| chip)
        .collect();
    (at_pane, boxes)
}

/// **Where the renderer row is inside a group**, written here as the mock's
/// own arithmetic rather than imported: `.rend-row` sits under `.node-head`
/// and is `REND_ROW_H` tall, and a test that asked `view.rs` for the answer
/// could not tell a right one from a moved one.
fn row_of(group: egui::Rect) -> egui::Rect {
    let top = group.min.y + size::NODE_HEAD_H;
    egui::Rect::from_min_max(
        egui::Pos2::new(group.min.x, top),
        egui::Pos2::new(group.max.x, top + size::REND_ROW_H),
    )
}

/// **How wide a chip is**, asked of the same fonts the paint asks: `.rend`'s
/// `padding: 0 7px` around a name at `BASE`.
fn chip_w(ctx: &egui::Context, name: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            name.to_owned(),
            egui::FontId::new(size::BASE, egui::FontFamily::Proportional),
            egui::Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::REND_PAD_X * 2.0
}

// ---------------------------------------------------------------------------
// Where the controls are
// ---------------------------------------------------------------------------

/// **The row is under the head and it is the rest of the group**, which is
/// `group_h`'s own sum read back off a laid-out group: a group with a renderer
/// row and no parameters is a head and that row, and nothing is left over.
///
/// **And a group with no renderers has no row at all** — the chips belong to
/// the renderers group and to no other, which is what the second group here is
/// for.
#[test]
fn the_row_sits_under_the_node_head_and_is_the_rest_of_the_group() {
    let pane = mock();
    let (panel, _) = console(PLAUSIBLE);
    let at_pane = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
    let group = at_pane.group(&pane.nodes, 0);
    let row = row_of(group);

    assert!(near(row.min.y, group.min.y + size::NODE_HEAD_H));
    assert!(near(row.height(), size::REND_ROW_H));
    assert!(
        near(row.max.y, group.max.y),
        "the group is {} tall and its head plus its renderer row come to {} — a group with no \
         parameters has nothing under the chips",
        group.height(),
        size::NODE_HEAD_H + size::REND_ROW_H
    );
    assert!(near(row.width(), group.width()));

    let plain = at_pane.group(&pane.nodes, 1);
    assert!(
        pane.nodes[1].renderers.is_empty() && near(plain.height(), size::NODE_HEAD_H),
        "a group with no renderers is {} tall and a bare node head is {}",
        plain.height(),
        size::NODE_HEAD_H
    );
}

/// **The chips are `.rend-row`'s own flex row**: the first against the row's
/// left padding, one padding down from its top rather than centred in it, each
/// as wide as the name in it, and one `REND_GAP` apart.
#[test]
fn the_chips_are_the_rows_own_geometry() {
    let pane = mock();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        let (at_pane, boxes) = chips(&panel, &ctx, 0, &pane);
        let row = row_of(at_pane.group(&pane.nodes, 0));
        assert_eq!(boxes.len(), pane.nodes[0].renderers.len());

        assert!(
            near(boxes[0].min.x, row.min.x + size::REND_ROW_PAD_L),
            "the first chip is {} in from the row's left edge and `.rend-row`'s padding is {}",
            boxes[0].min.x - row.min.x,
            size::REND_ROW_PAD_L
        );
        for (index, chip) in boxes.iter().enumerate() {
            assert!(
                near(chip.min.y, row.min.y + size::REND_ROW_PAD_T),
                "chip {index} is {} down from the top of the row and `.rend-row`'s top padding \
                 is {} — the space under a chip is twice the space over it",
                chip.min.y - row.min.y,
                size::REND_ROW_PAD_T
            );
            assert!(near(chip.height(), size::REND_H));
            assert!(near(
                chip.width(),
                chip_w(&ctx, &pane.nodes[0].renderers[index].name)
            ));
            if index > 0 {
                assert!(
                    near(chip.min.x, boxes[index - 1].max.x + size::REND_GAP),
                    "chip {index} starts {} after the one before it and `.rend-row`'s gap is {}",
                    chip.min.x - boxes[index - 1].max.x,
                    size::REND_GAP
                );
            }
        }
        assert!(
            near(row.max.y - boxes[0].max.y, size::REND_ROW_PAD_B),
            "the chips leave {} under them and `.rend-row`'s bottom padding is {}",
            row.max.y - boxes[0].max.y,
            size::REND_ROW_PAD_B
        );
    }
}

// ---------------------------------------------------------------------------
// The claim rule
// ---------------------------------------------------------------------------

/// **Every chip clears every boundary's grab.** The nearest boundary is the
/// pane divider and what a chip has to clear sideways is `.rend-row`'s left
/// padding — **12**, against a `GRAB` of 6, which is two more than the deck
/// head's 10 and so is not the number that goes first.
#[test]
fn every_chip_clears_every_boundarys_grab() {
    let pane = mock();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        for (index, name) in PANE_NAMES.iter().enumerate() {
            let (at_pane, boxes) = chips(&panel, &ctx, index, &pane);
            let row = row_of(at_pane.group(&pane.nodes, 0));

            let clearance = boxes[0].min.x - row.min.x;
            assert!(
                near(clearance, size::REND_ROW_PAD_L),
                "the first chip is {clearance} in from the pane's edge and `.rend-row`'s padding \
                 is {}",
                size::REND_ROW_PAD_L
            );
            assert!(
                clearance > GRAB,
                "the leftmost chip is {clearance} in from the pane's edge and a boundary grabs \
                 {GRAB} — the control is inside a boundary's grab, and `input`'s rule is what \
                 has to change"
            );

            let bay = rect_of(panel.layout(), name);
            let above = boxes[0].min.y - bay.y;
            assert!(
                above > GRAB,
                "the chips have {above} of pane above them, against a grab of {GRAB}"
            );

            for (which, chip) in boxes.iter().enumerate() {
                // The last chip of a wide row can finish outside the pane, and
                // the part of it that is not drawn is not a target — so only
                // the corners inside the row are asked.
                if !row.contains_rect(*chip) {
                    continue;
                }
                for probe in [
                    chip.left_top(),
                    chip.right_top(),
                    chip.left_bottom(),
                    chip.right_bottom(),
                    chip.center(),
                ] {
                    assert!(
                        !matches!(
                            panel.layout().hit(at(probe), GRAB),
                            karakuri_layout::Hit::Divider { .. }
                        ),
                        "a boundary grabs {probe:?}, which is on chip {which} of pane {index}"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// **A press on a chip asks for that renderer, by its position in draw
/// order** — the numbering `--param L4:1:…` and a `select` record use, and not
/// a position in whatever this row managed to draw.
///
/// **The lit chip is claimed with the rest**: it names a destination the deck
/// is already at, which is the anchor's shape two rows up, and a chip that
/// stopped being pressable the moment it lit would take the claim out from
/// under a hand on the beat the swap landed.
#[test]
fn a_press_on_a_chip_asks_for_that_renderer() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (at_pane, boxes) = chips(&panel, &ctx, 0, &pane);
    for (index, chip) in boxes.iter().enumerate() {
        assert_eq!(
            at_pane.select_renderer(&ctx, &pane, at(chip.center())),
            Some(Operation::SelectRenderer {
                deck: 0,
                renderer: index as u32
            }),
            "a press on chip {index} asked for another renderer, or for nothing"
        );
    }
}

/// **A press names the pane's own deck**, which is [`Pane::deck`] carried
/// through the answer the way the deck head carries it.
#[test]
fn a_press_names_the_panes_own_deck() {
    let (panel, ctx) = console(PLAUSIBLE);
    for deck in 0..4 {
        let pane = Pane { deck, ..mock() };
        let (at_pane, boxes) = chips(&panel, &ctx, 0, &pane);
        assert_eq!(
            at_pane.select_renderer(&ctx, &pane, at(boxes[1].center())),
            Some(Operation::SelectRenderer {
                deck: deck as u8,
                renderer: 1
            })
        );
    }
}

/// **An overdrawn deck draws its chips and claims none of them.** *"Only where
/// the deck composites"* — under overdraw every renderer draws, so a selection
/// would name a state the picture is not in. The chips keep their shape,
/// because a row that vanished would move every parameter row under it.
#[test]
fn an_overdrawn_deck_draws_its_chips_and_claims_none() {
    let pane = Pane {
        composite: false,
        ..mock()
    };
    let (panel, ctx) = console(PLAUSIBLE);
    let (at_pane, boxes) = chips(&panel, &ctx, 0, &pane);
    assert_eq!(
        boxes.len(),
        3,
        "an overdrawn deck stopped drawing its chips, and the row is drawn either way"
    );
    for (index, chip) in boxes.iter().enumerate() {
        assert_eq!(
            at_pane.select_renderer(&ctx, &pane, at(chip.center())),
            None,
            "chip {index} of an overdrawn deck was claimed"
        );
    }
}

/// **A Set with one renderer draws its chip and claims it for nothing** —
/// *"and holds two or more"*, which is the other half of the row's own
/// sentence: there is nothing to choose between.
#[test]
fn a_lone_renderer_is_drawn_and_not_claimed() {
    let mut pane = mock();
    pane.nodes[0].renderers = vec![Renderer {
        name: "dots".to_owned(),
        live: true,
    }];
    let (panel, ctx) = console(PLAUSIBLE);
    let (at_pane, boxes) = chips(&panel, &ctx, 0, &pane);
    assert_eq!(boxes.len(), 1);
    assert_eq!(
        at_pane.select_renderer(&ctx, &pane, at(boxes[0].center())),
        None,
        "the one chip of a Set with one renderer was claimed, and there is nothing to choose"
    );
}

/// **A press between two chips, or past the last of them, is on nothing** —
/// *a control claims what it acts on and no more*, and the row around a chip
/// is not a target.
#[test]
fn a_press_off_every_chip_asks_for_nothing() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (at_pane, boxes) = chips(&panel, &ctx, 0, &pane);
    let row = row_of(at_pane.group(&pane.nodes, 0));
    for probe in [
        // The row's left padding, before the first chip.
        egui::Pos2::new(row.min.x + size::REND_ROW_PAD_L * 0.5, boxes[0].center().y),
        // The gap between the first two.
        egui::Pos2::new(boxes[0].max.x + size::REND_GAP * 0.5, boxes[0].center().y),
        // Under the chips, in `.rend-row`'s deeper bottom padding.
        egui::Pos2::new(boxes[0].center().x, row.max.y - size::REND_ROW_PAD_B * 0.5),
        // Over them, in the node head.
        egui::Pos2::new(boxes[0].center().x, row.min.y - size::REND_ROW_PAD_T),
    ] {
        assert_eq!(
            at_pane.select_renderer(&ctx, &pane, at(probe)),
            None,
            "a press at {probe:?} was claimed by a chip, and it is on none of them"
        );
    }
}

/// **A row the pane's body does not reach is not pressable**, which is the
/// clip reaching a control: what is not drawn is not a target.
///
/// **The body says so and no longer `shown`.** A pane scrolls now
/// (ADR-0307), so *what is on screen* is the body rectangle rather than a
/// count of whole groups — `InspectorPane::drawn` walks it, `grip` and this
/// refuse a press outside it, and `inspector_into` clips the paint to it. The
/// body is emptied down to nothing here, which is a pane folded to its two
/// heads.
#[test]
fn a_row_the_body_does_not_reach_is_not_pressable() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (at_pane, boxes) = chips(&panel, &ctx, 0, &pane);
    // **Cut between the group's head and its renderer row.** The group is
    // still drawn — its head is inside the body — so `InspectorPane::drawn`
    // has nothing to say about these points and what refuses them is the body
    // rectangle itself.
    let row = row_of(at_pane.group(&pane.nodes, 0));
    let clipped = InspectorPane {
        body: egui::Rect::from_min_max(at_pane.body.min, egui::pos2(at_pane.body.max.x, row.min.y)),
        ..at_pane
    };
    assert!(
        clipped.drawn(&pane.nodes).contains(&0),
        "the guard on the assertions below: the group is still drawn"
    );
    for (index, chip) in boxes.iter().enumerate() {
        assert!(
            at_pane
                .select_renderer(&ctx, &pane, at(chip.center()))
                .is_some(),
            "chip {index} is not pressable on the pane that drew the whole row, so this test \
             cannot tell a clip from a miss"
        );
        assert_eq!(
            clipped.select_renderer(&ctx, &pane, at(chip.center())),
            None,
            "chip {index} below the pane's body was claimed, where the paint is clipped away"
        );
    }
}

/// **Before the first frame there is nothing here to press**, which is
/// `deck_head`'s guard: a chip is as wide as the name in it and
/// `Context::fonts` is not valid until a pass has run.
#[test]
fn a_console_that_has_not_drawn_claims_no_chip() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (at_pane, boxes) = chips(&panel, &ctx, 0, &pane);
    assert_eq!(
        at_pane.select_renderer(&egui::Context::default(), &pane, at(boxes[0].center())),
        None
    );
}
