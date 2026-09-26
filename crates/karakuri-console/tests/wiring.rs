//! Node input wiring (`uses` line) and parameter publish mark controls (ADR-0329).
//!
//! Validates `uses` line positioning, dropdown card interactions, candidate filtering,
//! publish toggle states, and preservation of interface order across operations.

mod common;

use common::{at, console, drawn_once, near, PLAUSIBLE};
use karakuri_console::panel::Panel;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    inspector, InspectorPane, Node, Pane, Param, Uses, View, PANES, SYNCS,
};
use karakuri_operation::{Control, Layer, NodeAddress, Operation, ParamAt, Sync};

/// A published row: a position in the interface, a name, an address and a
/// range.
fn row(ord: usize, name: &str, at: Option<(Layer, u32)>) -> Param {
    Param {
        ord: Some(ord),
        name: name.to_owned(),
        value: 0.5,
        range: [0.0, 1.0],
        param: ParamAt {
            node: at.map(|(layer, index)| NodeAddress { layer, index }),
            key: name.to_owned(),
        },
        bound: None,
    }
}

/// The same control with the interface not carrying it — no position, and
/// therefore no fader and no figure.
fn off(name: &str, at: Option<(Layer, u32)>) -> Param {
    Param {
        ord: None,
        ..row(1, name, at)
    }
}

/// What a row of the pane below becomes when it is on the interface.
fn control(name: &str, at: Option<(Layer, u32)>) -> Control {
    Control {
        name: name.to_owned(),
        node: at.map(|(layer, index)| NodeAddress { layer, index }),
        key: name.to_owned(),
        range: [0.0, 1.0],
    }
}

/// Configures a mock deck with declared input wiring and non-sequential interface parameter ordering (ADR-0152).
fn mock() -> Pane {
    Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: Sync::Beat,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.25,
        composite: true,
        // Not this file's row.
        aimed: None,
        nodes: vec![
            Node {
                keep: None,
                addr: "L2:0".to_owned(),
                name: "swirl_warp".to_owned(),
                authority: None,
                uses: vec![Uses {
                    slot: "far".into(),
                    to: "sphere_shell".to_owned(),
                    candidates: vec!["drift_shell".to_owned()],
                }],
                renderers: Vec::new(),
                params: vec![
                    row(3, "amount", Some((Layer::L2, 0))),
                    // Declared and off the interface, so it keeps its row.
                    off("twist", Some((Layer::L2, 0))),
                ],
            },
            Node {
                keep: None,
                addr: "L1:1".to_owned(),
                name: "sphere_shell".to_owned(),
                authority: None,
                uses: Vec::new(),
                renderers: Vec::new(),
                params: vec![row(2, "detail", Some((Layer::L1, 1)))],
            },
            Node {
                keep: None,
                addr: "L4".to_owned(),
                name: "renderers".to_owned(),
                authority: None,
                uses: Vec::new(),
                renderers: Vec::new(),
                params: vec![row(1, "exposure", None)],
            },
        ],
    }
}

/// A view with that pane in both slots, which is what `claim` hit-tests.
fn view(pane: &Pane) -> View {
    let mut view = View::new(Room::Day);
    view.inspector = vec![pane.clone(); PANES];
    view
}

/// The laid-out pane.
fn pane_at(panel: &Panel, index: usize, pane: &Pane) -> InspectorPane {
    inspector(panel.layout(), index, pane, 0.0).expect("a pane with room in it")
}

// ---------------------------------------------------------------------------
// Where the line is
// ---------------------------------------------------------------------------

/// A `uses` line sits under the node head and above that node's rows, and the
/// rows below it carry the offset — which is the whole of what a line costs the
/// group's arithmetic. A row resolved against a group that had not counted it
/// is a fader drawn where a hand cannot reach it.
#[test]
fn a_uses_line_sits_between_the_head_and_the_rows() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let laid = pane_at(&panel, 0, &pane);
    let line = laid
        .uses_line(&ctx, &pane, 0, 0, false)
        .expect("the first group declares an input");

    assert!(
        near(line.row.height(), size::USES_H),
        "a `uses` line is {} tall and `.uses` is {}",
        line.row.height(),
        size::USES_H
    );
    assert!(
        near(line.chip.height(), size::USES_CHIP_H),
        "the capsule is not the height `.uses`'s pill is"
    );
    assert!(
        line.row.contains_rect(line.chip),
        "the capsule is not inside the line it is drawn on"
    );
    // The capsule aligns right against the row boundary via `.sep { flex: 1 }`.
    assert!(near(line.row.max.x - line.chip.max.x, size::PARAM_PAD_R));
    // Groups without inputs produce no `uses` line.
    assert_eq!(laid.uses_line(&ctx, &pane, 1, 0, false), None);
    assert_eq!(laid.uses_line(&ctx, &pane, 0, 1, false), None);

    // Subsequent rows are offset below the `uses` line.
    let first = laid.publish_mark(&pane, 0, 0).expect("the first row");
    assert!(
        first.min.y >= line.row.max.y,
        "the first row of the group begins at {} and the `uses` line ends at {} — the row is \
         drawn over the line",
        first.min.y,
        line.row.max.y
    );

    // Verifies the node group height precisely encloses its head, uses line, and rows.
    let group = laid.group(&pane.nodes, 0);
    let last = laid
        .publish_mark(&pane, 0, 1)
        .expect("the group's second row");
    assert!(
        group.contains_rect(line.row) && group.contains_rect(last),
        "the group is {} tall and holds a {} line and rows ending at {} — a group whose height \
         did not count its `uses` lines draws the row after it over the group below",
        group.height(),
        size::USES_H,
        last.max.y - group.min.y
    );
    assert!(
        near(
            group.height(),
            size::NODE_HEAD_H + size::USES_H + size::PARAM_H * 2.0
        ),
        "a head, one `uses` line and two rows come to {} and the group is {}",
        size::NODE_HEAD_H + size::USES_H + size::PARAM_H * 2.0,
        group.height()
    );
}

/// The card is not down until the capsule is pressed, and while it is down it
/// offers exactly the candidates the line was handed — one here, because the
/// node already wired is not in its own list.
#[test]
fn the_card_is_shut_until_it_is_opened() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let laid = pane_at(&panel, 0, &pane);
    let room = karakuri_console::view::to_egui(PLAUSIBLE);

    let shut = laid.uses_line(&ctx, &pane, 0, 0, false).expect("a line");
    assert_eq!(shut.rows, 0);
    assert_eq!(
        shut.list(room),
        None,
        "a card nobody opened handed out a rectangle, which `Load::rows` exists to stop"
    );
    assert_eq!(shut.row_at(room, 0), None);

    let open = laid.uses_line(&ctx, &pane, 0, 0, true).expect("a line");
    assert_eq!(open.rows, 1);
    let card = open.list(room).expect("an open card");
    let first = open.row_at(room, 0).expect("its one row");
    assert!(card.contains_rect(first));
    assert_eq!(open.row_at(room, 1), None, "a row past the end of the card");
    // The open card extends downward below the capsule inside the pane body.
    assert!(
        card.min.y >= open.chip.max.y,
        "the card is over the capsule rather than under it"
    );
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// A pick names the deck, the node, the input's own word and the node picked —
/// `Record::Edge`'s two halves and the deck the operation is addressed to,
/// which is the one operation in this vocabulary addressed by name at both
/// ends.
#[test]
fn a_pick_names_the_node_the_input_and_the_deck() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let laid = pane_at(&panel, 0, &pane);
    let card_room = karakuri_console::view::to_egui(PLAUSIBLE);
    let open = laid.uses_line(&ctx, &pane, 0, 0, true).expect("a line");
    let row = open.row_at(card_room, 0).expect("its one row");

    assert_eq!(
        laid.wired(&ctx, &pane, card_room, (0, 0), at(row.center())),
        Some(Operation::WireInput {
            deck: 0,
            node: "swirl_warp".to_owned(),
            slot: "far".into(),
            to: "drift_shell".to_owned(),
        })
    );
    // Off every row of the card, nothing is asked for.
    assert_eq!(
        laid.wired(&ctx, &pane, card_room, (0, 0), at(open.chip.center())),
        None
    );
}

/// A line with nothing to offer opens no card, which is a deck holding one node
/// of the kind the input takes: the node already wired is left out of its own
/// list, so there is nothing to pick.
#[test]
fn a_line_with_no_candidate_offers_no_card() {
    let mut pane = mock();
    pane.nodes[0].uses[0].candidates.clear();
    let (panel, ctx) = console(PLAUSIBLE);
    let laid = pane_at(&panel, 0, &pane);
    let room = karakuri_console::view::to_egui(PLAUSIBLE);
    let open = laid.uses_line(&ctx, &pane, 0, 0, true).expect("a line");

    assert_eq!(open.rows, 0);
    assert_eq!(open.list(room), None);
    assert!(
        open.chip.width() > 0.0,
        "a line with nothing to offer lost its shape as well as its card"
    );
    assert_eq!(
        laid.wired(&ctx, &pane, room, (0, 0), at(open.chip.center())),
        None
    );
}

// ---------------------------------------------------------------------------
// The publish mark
// ---------------------------------------------------------------------------

/// Unpublishing a parameter removes it while strictly preserving interface numbering order.
#[test]
fn taking_a_control_off_names_the_rest_in_interface_order() {
    let pane = mock();
    let (panel, _) = console(PLAUSIBLE);
    let laid = pane_at(&panel, 0, &pane);
    // `amount` is the first row of the first group, under one `uses` line.
    let mark = laid
        .publish_mark(&pane, 0, 0)
        .expect("the first row's mark");

    assert_eq!(
        laid.publishing(&pane, at(mark.center())),
        Some(Operation::Publish {
            deck: 0,
            controls: vec![
                control("exposure", None),
                control("detail", Some((Layer::L1, 1))),
            ],
        }),
        "the list is not the interface less `amount`, in interface order"
    );
}

/// Verifies that republishing a control appends it to the end of the controls list.
#[test]
fn putting_a_control_back_lands_it_at_the_end() {
    let pane = mock();
    let (panel, _) = console(PLAUSIBLE);
    let laid = pane_at(&panel, 0, &pane);
    // `twist` is the second row of the first group and is off the interface.
    let mark = laid
        .publish_mark(&pane, 0, 1)
        .expect("the second row's mark");

    assert_eq!(
        laid.publishing(&pane, at(mark.center())),
        Some(Operation::Publish {
            deck: 0,
            controls: vec![
                control("exposure", None),
                control("detail", Some((Layer::L1, 1))),
                control("amount", Some((Layer::L2, 0))),
                control("twist", Some((Layer::L2, 0))),
            ],
        })
    );
}

/// The mark is claimed and the rest of the row is not, which is *a control
/// claims what it acts on and no more*: the cell is a fixed track of the mock's
/// grid, and the name beside it is a readout.
#[test]
fn the_mark_is_the_cell_and_nothing_beside_it() {
    let pane = mock();
    let (mut panel, ctx) = console(PLAUSIBLE);
    let view = view(&pane);
    let laid = pane_at(&panel, 0, &pane);
    let mark = laid.publish_mark(&pane, 0, 0).expect("a mark");
    let _ = ctx;

    assert_eq!(
        karakuri_console::input::claim(&mut panel, &drawn_once(), &view, at(mark.center())),
        karakuri_console::input::Claim::Panel,
        "the publish mark is not claimed by the panel"
    );
    // One track along is the name, which asks for nothing.
    let beside = egui::pos2(mark.max.x + size::PARAM_GAP + 1.0, mark.center().y);
    assert_eq!(laid.publishing(&pane, at(beside)), None);
}

/// Verifies that an unpublished row renders no fader handle, while a published row does.
#[test]
fn an_unpublished_row_has_no_fader() {
    let pane = mock();
    let (panel, _) = console(PLAUSIBLE);
    let laid = pane_at(&panel, 0, &pane);
    let on = laid.publish_mark(&pane, 0, 0).expect("the published row");
    let off = laid.publish_mark(&pane, 0, 1).expect("the row is drawn");

    // The row is there — this is the whole reason it stays.
    assert!(off.width() > 0.0);
    // 1px horizontal sweep ensures narrow fader handles (~10px) are detected.
    let sweep = |y: f32| -> Vec<f32> {
        let mut found = Vec::new();
        let mut x = laid.body.min.x;
        while x <= laid.body.max.x {
            if laid.owns(&pane, at(egui::pos2(x, y))) {
                found.push(x);
            }
            x += 1.0;
        }
        found
    };
    let published = sweep(on.center().y);
    assert!(
        !published.is_empty(),
        "the published row beside it offers no handle either, so this sweep is finding nothing \
         rather than finding that a row off the interface has none"
    );
    let unpublished = sweep(off.center().y);
    assert!(
        unpublished.is_empty(),
        "a row off the interface offered a handle at {unpublished:?}, where the published row \
         above it offers one at {published:?}"
    );
}
