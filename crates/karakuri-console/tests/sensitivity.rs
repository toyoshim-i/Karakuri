//! Layout and hit-testing for sensitivity rows and authority chips in Inspector node groups.
//!
//! Validates row expansions, chip positioning, curve cycling, take-back addressing,
//! readout pass-through, knob disabling on bound rows, and authority chip levels (P-0090).

mod common;

use common::{at, near, PLAUSIBLE};
use karakuri_console::room::size;
use karakuri_console::view::{
    inspector, sens_chips, InspectorPane, Node, NodeAuthority, Pane, Param, Renderer, SensChip,
    Source, AUTHORITIES, SYNCS,
};
use karakuri_layout::Rect;
use karakuri_operation::{Authority, BindAt, Curve, Layer, NodeAddress, Operation, ParamAt, Sync};

/// A row nothing is holding.
fn row(ord: usize, name: &str, index: u32, range: [f32; 2], value: f32) -> Param {
    Param {
        ord: Some(ord),
        name: name.to_owned(),
        value,
        range,
        param: ParamAt {
            node: Some(NodeAddress {
                layer: Layer::L1,
                index,
            }),
            key: name.to_owned(),
        },
        bound: None,
    }
}

/// Configures a mock attachment (`energy` via `pow2`) with a range independent of fader range.
fn attachment() -> Source {
    Source {
        signal: "energy".to_owned(),
        curve: Curve::Pow2,
        range: [0.1, 2.4],
        at: BindAt {
            layer: Layer::L1,
            index: Some(0),
            key: "turbulence".to_owned(),
        },
    }
}

/// Second mock attachment on an adjacent row to verify layout walking across multiple sensitivity rows.
fn second() -> Source {
    Source {
        signal: "beat".to_owned(),
        curve: Curve::Lin,
        range: [0.0, 1.0],
        at: BindAt {
            layer: Layer::L1,
            index: None,
            key: "spin".to_owned(),
        },
    }
}

/// The mock's `L1:0 drift_shell`: three rows, the second and third bound, so
/// there is a row above the first sensitivity row and a second sensitivity row
/// below a row that already grew one.
fn shell() -> Node {
    Node {
        keep: None,
        addr: "L1:0".to_owned(),
        name: "drift_shell".to_owned(),
        authority: Some(NodeAuthority {
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            level: Authority::Manual,
        }),
        uses: Vec::new(),
        renderers: Vec::new(),
        params: vec![
            row(1, "radius", 0, [0.0, 4.0], 2.4),
            Param {
                bound: Some(attachment()),
                ..row(2, "turbulence", 0, [0.0, 3.0], 1.4)
            },
            Param {
                bound: Some(second()),
                ..row(3, "spin", 0, [0.0, 1.0], 0.7)
            },
        ],
    }
}

/// The folded renderers head, which stands over two nodes and so has neither an
/// authority nor an address to press.
fn renderers() -> Node {
    Node {
        uses: Vec::new(),
        keep: None,
        addr: "L4".to_owned(),
        name: "renderers".to_owned(),
        authority: None,
        renderers: ["soft_points", "strand_strokes"]
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

fn mock() -> Pane {
    Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: Sync::Beat,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: true,
        // Not this test's row: the deck head's two build chips are
        // drawn from this and nothing here is about them.
        aimed: None,
        nodes: vec![shell(), renderers()],
    }
}

use common::console;

fn pane_at(panel: &karakuri_console::panel::Panel, pane: &Pane) -> InspectorPane {
    inspector(panel.layout(), 0, pane, 0.0).expect("a pane with room in it")
}

/// Calculates expected group row position using a layout walk rather than a fixed stride.
fn row_rect(at_pane: &InspectorPane, pane: &Pane, index: usize) -> egui::Rect {
    let group = at_pane.group(&pane.nodes, 0);
    let mut top = group.min.y + size::NODE_HEAD_H;
    for param in pane.nodes[0].params.iter().take(index) {
        top += size::PARAM_H;
        if param.bound.is_some() {
            top += size::SENS_H;
        }
    }
    egui::Rect::from_min_max(
        egui::pos2(group.min.x, top),
        egui::pos2(group.max.x, top + size::PARAM_H),
    )
}

/// The sensitivity row under the `index`th row, on the same terms.
fn sens_of(at_pane: &InspectorPane, pane: &Pane, index: usize) -> egui::Rect {
    let row = row_rect(at_pane, pane, index);
    egui::Rect::from_min_max(
        egui::pos2(row.min.x, row.max.y),
        egui::pos2(row.max.x, row.max.y + size::SENS_H),
    )
}

/// The whole of a pane, tall enough that nothing is cut off.
const TALL: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: PLAUSIBLE.w,
    h: PLAUSIBLE.h,
};

/// Binding a row adds a sensitivity row underneath, shifting subsequent rows downward by its height.
#[test]
fn a_bound_row_grows_a_sensitivity_row_and_moves_the_rows_under_it() {
    let (panel, _ctx) = console(TALL);
    let pane = mock();
    let at_pane = pane_at(&panel, &pane);
    let group = at_pane.group(&pane.nodes, 0);

    let wanted = size::NODE_HEAD_H + size::PARAM_H * 3.0 + size::SENS_H * 2.0;
    assert!(
        near(group.height(), wanted),
        "a group holding two bound rows is a head, three rows and two sensitivity rows: {} \
         against {wanted}",
        group.height()
    );

    let second = row_rect(&at_pane, &pane, 1);
    let third = row_rect(&at_pane, &pane, 2);
    assert!(
        near(third.min.y - second.max.y, size::SENS_H),
        "the row under a bound one is not a sensitivity row's height below it: {}",
        third.min.y - second.max.y
    );
    // And the group ends where the *last* sensitivity row does, so nothing
    // hangs over the group below it.
    assert!(
        near(sens_of(&at_pane, &pane, 2).max.y, group.max.y),
        "the last sensitivity row is not where the group ends"
    );
    // The first row is not moved by anything: a sensitivity row grows
    // downwards from the row it belongs to and never upwards.
    assert!(
        near(
            row_rect(&at_pane, &pane, 0).min.y,
            group.min.y + size::NODE_HEAD_H
        ),
        "the first row is not directly under the node head"
    );
}

/// Chips are laid out inside the sensitivity row with expected order and pill padding.
#[test]
fn the_chips_sit_in_the_sensitivity_rows_own_tracks() {
    let (panel, ctx) = console(TALL);
    let pane = mock();
    let at_pane = pane_at(&panel, &pane);
    let source = attachment();
    let row = sens_of(&at_pane, &pane, 1);

    let chips: Vec<_> = sens_chips(&ctx, row, &source).collect();
    assert_eq!(
        chips.iter().map(|(chip, _)| *chip).collect::<Vec<_>>(),
        SensChip::ALL.to_vec(),
        "the chips are not in the order the mock draws them"
    );

    let first = chips[0].1;
    assert!(
        near(
            first.min.x,
            row.min.x + size::SENS_PAD_L + size::SENS_LABEL_W + size::SENS_GAP
        ),
        "the first chip does not start at the second of `.sens`'s two tracks: {}",
        first.min.x
    );
    assert!(
        near(first.min.y, row.min.y + size::SENS_PAD_T),
        "a chip is not one top padding down from the row"
    );
    assert!(
        near(first.height(), size::SENS_CHIP_H),
        "a chip is the wrong height"
    );

    for pair in chips.windows(2) {
        assert!(
            near(pair[1].1.min.x - pair[0].1.max.x, size::SENS_CHIP_GAP),
            "two chips are not `.sens .chips`'s gap apart"
        );
    }
    // The last chip has to be inside the pane, or the control an operator
    // needs most is the one clipped away.
    assert!(
        chips[3].1.max.x <= row.max.x,
        "`take back` is drawn past the right-hand edge of the pane"
    );
}

/// The curve chip restates the attachment and changes only the shape.
#[test]
fn the_curve_chip_attaches_the_same_signal_through_the_next_shape() {
    let (panel, ctx) = console(TALL);
    let pane = mock();
    let at_pane = pane_at(&panel, &pane);
    let source = attachment();
    let row = sens_of(&at_pane, &pane, 1);
    let (_, chip) = sens_chips(&ctx, row, &source)
        .find(|(chip, _)| *chip == SensChip::Curve)
        .expect("the curve chip is one of the four");

    let asked = at_pane
        .sensitivity(&ctx, &pane, at(chip.center()))
        .expect("a press on the curve chip asks for something");
    assert_eq!(
        asked,
        Operation::AttachSignal {
            deck: 0,
            param: source.at.clone(),
            signal: "energy".to_owned(),
            // `pow2` is second of the four, so the next is `sqrt`.
            curve: Curve::Sqrt,
            // Preserves existing attachment range rather than default row range.
            range: [0.1, 2.4],
        },
        "the curve chip did not restate the attachment it was drawn from"
    );
}

/// Take-back operations target the exact attachment address and binding key.
#[test]
fn take_back_removes_the_attachment_the_row_was_drawn_from() {
    let (panel, ctx) = console(TALL);
    let pane = mock();
    let at_pane = pane_at(&panel, &pane);
    let source = attachment();
    let row = sens_of(&at_pane, &pane, 1);
    let (_, chip) = sens_chips(&ctx, row, &source)
        .find(|(chip, _)| *chip == SensChip::TakeBack)
        .expect("`take back` is the last of the four");

    assert_eq!(
        at_pane
            .sensitivity(&ctx, &pane, at(chip.center()))
            .expect("a press on `take back` asks for something"),
        Operation::TakeParamBack {
            deck: 0,
            param: source.at,
        }
    );

    // Verifies wildcard attachment take-back operations preserve wildcard scope over multiple rows.
    let below = second();
    let row = sens_of(&at_pane, &pane, 2);
    let (_, chip) = sens_chips(&ctx, row, &below)
        .find(|(chip, _)| *chip == SensChip::TakeBack)
        .expect("the second bound row has a `take back` too");
    assert_eq!(
        at_pane
            .sensitivity(&ctx, &pane, at(chip.center()))
            .expect("a press on the second row's `take back` asks for something"),
        Operation::TakeParamBack {
            deck: 0,
            param: below.at,
        },
        "the second sensitivity row is not where the rows above it put it"
    );
}

/// The source and the range are drawn and claimed by nothing.
///
/// A press on either lands inside the row and asks for nothing — it does not
/// fall through to the chip beside it, and it does not reach the row above.
#[test]
fn the_source_and_the_range_are_readouts() {
    let (panel, ctx) = console(TALL);
    let pane = mock();
    let at_pane = pane_at(&panel, &pane);
    let source = attachment();
    let row = sens_of(&at_pane, &pane, 1);

    for want in [SensChip::Signal, SensChip::Range] {
        let (_, chip) = sens_chips(&ctx, row, &source)
            .find(|(chip, _)| *chip == want)
            .expect("every chip is drawn");
        assert_eq!(
            at_pane.sensitivity(&ctx, &pane, at(chip.center())),
            None,
            "{want:?} is a readout and claimed a press"
        );
    }
}

/// A bound row's knob is drawn and is not taken hold of, and the rows either
/// side of it still are — so this is the row's state and not the pane going
/// inert.
#[test]
fn a_bound_rows_knob_is_drawn_and_is_not_a_handle() {
    let (panel, ctx) = console(TALL);
    let pane = mock();
    let at_pane = pane_at(&panel, &pane);

    for (index, movable) in [(0usize, true), (1, false), (2, false)] {
        let row = row_rect(&at_pane, &pane, index);
        // The knob of a row is on the fill's moving edge; the whole row is
        // walked so this does not have to restate where that is.
        let mut took = false;
        let mut x = row.min.x;
        while x < row.max.x {
            if at_pane
                .grip(&pane, at(egui::pos2(x, row.center().y)))
                .is_some()
            {
                took = true;
                break;
            }
            x += 1.0;
        }
        assert_eq!(
            took,
            movable,
            "row {index} is {}takeable and should not be",
            match took {
                true => "",
                false => "not ",
            }
        );
    }
    // And the row is still painted: a group tall enough for it exists, which
    // the height assertion above already made, and the sensitivity row under
    // it is inside the group.
    let sens = sens_of(&at_pane, &pane, 1);
    assert!(
        sens.max.y <= at_pane.group(&pane.nodes, 0).max.y,
        "the sensitivity row hangs out of the group it belongs to"
    );
    let _ = ctx;
}

/// Every one of the three authority chips names the level it lands on, the lit
/// one included — a destination and never a step.
#[test]
fn every_authority_chip_names_the_level_it_lands_on() {
    let (panel, ctx) = console(TALL);
    let pane = mock();
    let at_pane = pane_at(&panel, &pane);
    let group = at_pane.group(&pane.nodes, 0);
    let head = egui::Rect::from_min_max(
        group.min,
        egui::pos2(group.max.x, group.min.y + size::NODE_HEAD_H),
    );

    let chips: Vec<_> = karakuri_console::view::auth_chips(&ctx, head, &pane.nodes[0]).collect();
    assert_eq!(
        chips.iter().map(|(level, _)| *level).collect::<Vec<_>>(),
        AUTHORITIES.to_vec(),
        "the chips are not in the order a node head shows them"
    );
    for (level, chip) in chips {
        assert_eq!(
            at_pane
                .set_authority(&ctx, &pane, at(chip.center()))
                .unwrap_or_else(|| panic!("the {level:?} chip claimed nothing")),
            Operation::SetAuthority {
                deck: 0,
                node: NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                },
                authority: level,
            },
            "a chip asked for a level that is not the one it draws"
        );
    }
}

/// A head standing over more than one node claims nothing, because authority is
/// per node and one chip on it would be one of two answers.
#[test]
fn a_head_over_more_than_one_node_claims_no_press() {
    let (panel, ctx) = console(TALL);
    let pane = mock();
    let at_pane = pane_at(&panel, &pane);
    let group = at_pane.group(&pane.nodes, 1);
    let head = egui::Rect::from_min_max(
        group.min,
        egui::pos2(group.max.x, group.min.y + size::NODE_HEAD_H),
    );
    assert!(
        pane.nodes[1].authority.is_none(),
        "the folded renderers head is the one with no chip, and this fixture gave it one"
    );
    // Every chip position the head *would* have had, which is where a press
    // would land if the claim were the head's rather than the node's.
    let mut x = head.min.x;
    while x < head.max.x {
        assert_eq!(
            at_pane.set_authority(&ctx, &pane, at(egui::pos2(x, head.center().y))),
            None,
            "a press on a head standing over two nodes asked for an authority"
        );
        x += 4.0;
    }
}
