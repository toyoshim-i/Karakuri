//! **The `keep` capsule on a node group's head**, which writes one node's
//! source into the operator's own library —
//! [ADR-0338](../../../docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md),
//! decision 4.
//!
//! Eight things:
//!
//! 1. Where the capsule goes: hard against the head's right-hand padding, at
//!    `.mini`'s own box, which is the mock's
//!    `<span class="mini">keep</span>` after the `.auth` chips.
//! 2. **That the three authority chips are laid out inside what it leaves**,
//!    so neither control is drawn where the other is pressed. That is the one
//!    thing about this row that could go wrong silently: the chips were
//!    right-aligned on the head before the capsule existed.
//! 3. That a press on it emits `KeepProcedure` naming **that node**, with
//!    `id: None` — the press that types nothing takes a stamp (ADR-0128).
//! 4. **That the two heads which carry no capsule are not targets**: a head
//!    standing over several nodes, and the built-in camera. Both are
//!    `Node::keep` being `None`, and the mock draws both absences.
//! 5. That a press on the chips is still the chips' and a press on the
//!    capsule is not a chip's.
//! 6. That a console which has not drawn has no capsule — every measured
//!    control's guard.
//!
//! **What is not here and cannot be**: that `input::claim` gives the panel a
//! press on the capsule, and that the window writes the file. Those are
//! `input::PROBES`' row and `crates/karakuri/src/main.rs`'s press arm.

mod common;

use common::{drawn_once, near, PLAUSIBLE};
use karakuri_console::panel::Panel;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    auth_chips, inspector, node_keep, Node, NodeAuthority, Pane, View, AUTHORITIES, PANES, SYNCS,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::{Authority, Layer, NodeAt, Operation, Sync};

/// A node group with a source behind it — every node but the built-in camera.
fn kept(addr: &str, name: &str, at: NodeAt) -> Node {
    Node {
        addr: addr.to_owned(),
        name: name.to_owned(),
        authority: Some(NodeAuthority {
            at,
            level: Authority::Manual,
        }),
        keep: Some(at),
        renderers: Vec::new(),
        uses: Vec::new(),
        params: Vec::new(),
    }
}

/// **The built-in camera**: a node with an authority and no procedure behind
/// it, so there is nothing to write and the head carries no capsule.
fn built_in_camera() -> Node {
    Node {
        keep: None,
        ..kept(
            "L3:0",
            "orbit",
            NodeAt {
                layer: Layer::L3,
                index: 0,
            },
        )
    }
}

/// **The mock's folded `L4 renderers` head**, standing over three nodes: no
/// one authority and no one source, which are the same absence twice.
fn folded_renderers() -> Node {
    Node {
        addr: "L4".to_owned(),
        name: "renderers".to_owned(),
        authority: None,
        keep: None,
        renderers: Vec::new(),
        uses: Vec::new(),
        params: Vec::new(),
    }
}

/// The mock's deck A, with whatever groups the test is about.
fn pane_of(nodes: Vec<Node>) -> Pane {
    Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: Sync::Tempo,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: true,
        aimed: None,
        nodes,
    }
}

fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// A view with one pane over the groups given, and nothing else.
fn view_of(nodes: Vec<Node>) -> View {
    let mut view = View::new(Room::Day);
    view.inspector = vec![pane_of(nodes)];
    view
}

/// The first group's head, in the pane's own coordinates.
fn head(panel: &Panel, pane: &Pane, index: usize) -> egui::Rect {
    let laid = inspector(panel.layout(), 0, pane, 0.0).expect("a pane with room in it");
    let group = laid.group(&pane.nodes, index);
    egui::Rect::from_min_max(
        group.min,
        egui::Pos2::new(group.max.x, group.min.y + size::NODE_HEAD_H),
    )
}

// ---------------------------------------------------------------------------
// Where the control is
// ---------------------------------------------------------------------------

/// **Hard against the head's right-hand padding**, at `.mini`'s own box — the
/// mock's `.node-head` is a flex row ending `…, .sep, .auth, .mini`.
#[test]
fn the_capsule_sits_inside_the_heads_right_hand_padding() {
    let (panel, ctx) = console(PLAUSIBLE);
    let pane = pane_of(vec![kept(
        "L1:0",
        "drift_shell",
        NodeAt {
            layer: Layer::L1,
            index: 0,
        },
    )]);
    let head = head(&panel, &pane, 0);
    let pill = node_keep(&ctx, head, &pane.nodes[0]).expect("a head with room for it");
    assert!(
        near(pill.max.x, head.max.x - size::NODE_HEAD_PAD_X),
        "the capsule ends at {} and the head's padding ends at {}",
        pill.max.x,
        head.max.x - size::NODE_HEAD_PAD_X
    );
    assert!(
        near(pill.height(), size::MINI_H),
        "the capsule is {} tall and a `.mini` is {}",
        pill.height(),
        size::MINI_H
    );
    assert!(
        near(pill.center().y, head.center().y),
        "the capsule is not centred in the row it is in"
    );
    assert!(
        head.contains_rect(pill),
        "the capsule is not inside the head it is drawn in"
    );
}

/// **The three authority chips are laid out inside what the capsule leaves**,
/// one `.node-head` gap clear of it.
///
/// This is the assertion the pass exists to make: the chips were right-aligned
/// on the head's own padding before the capsule was drawn, so a capsule added
/// without moving them would sit on top of the `auto` chip — a control painted
/// where a hand presses another one.
#[test]
fn the_authority_chips_clear_the_capsule() {
    let (panel, ctx) = console(PLAUSIBLE);
    let node = kept(
        "L1:0",
        "drift_shell",
        NodeAt {
            layer: Layer::L1,
            index: 0,
        },
    );
    let pane = pane_of(vec![node.clone()]);
    let head = head(&panel, &pane, 0);
    let pill = node_keep(&ctx, head, &node).expect("a head with room for it");
    let laid = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
    let chips: Vec<egui::Rect> = AUTHORITIES
        .iter()
        .map(|level| {
            // The chip's rectangle as the hit-test finds it: a press on the
            // middle of each has to answer that level. `auth_chips` applies
            // the trim itself, which is what keeps this one derivation.
            auth_chips(&ctx, head, &node)
                .find(|(l, _)| l == level)
                .expect("a chip per level")
                .1
        })
        .collect();
    let last = chips.last().expect("three chips");
    // Asked through the pane rather than through `auth_chips` alone, because
    // the trim is what `set_authority` applies and a chip that cleared the
    // capsule in one derivation and not in the other is exactly the defect.
    let pressed = laid.set_authority(&ctx, &pane, at(last.center()));
    assert!(
        pressed.is_some(),
        "the `auto` chip is drawn and a press on its middle reaches nothing"
    );
    for chip in &chips {
        assert!(
            chip.max.x <= pill.min.x - size::NODE_HEAD_GAP + 0.001,
            "an authority chip ends at {} and the capsule starts at {}, one `.node-head` gap of \
             {} after it",
            chip.max.x,
            pill.min.x,
            size::NODE_HEAD_GAP
        );
    }
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// **A press on the capsule emits `KeepProcedure` naming that node**, with
/// `id: None` — the capsule is the press that types nothing.
#[test]
fn a_press_on_the_capsule_keeps_that_node() {
    let (panel, ctx) = console(PLAUSIBLE);
    let second = NodeAt {
        layer: Layer::L2,
        index: 0,
    };
    let pane = pane_of(vec![
        kept(
            "L1:0",
            "drift_shell",
            NodeAt {
                layer: Layer::L1,
                index: 0,
            },
        ),
        kept("L2:0", "swirl_warp", second),
    ]);
    let laid = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
    let head = head(&panel, &pane, 1);
    let pill = node_keep(&ctx, head, &pane.nodes[1]).expect("a head with room for it");
    assert_eq!(
        laid.keep_procedure(&ctx, &pane, at(pill.center())),
        Some(Operation::KeepProcedure {
            deck: 0,
            node: second,
            id: None,
        }),
        "the second group's capsule named a different node than the group it is drawn on"
    );
}

/// **A head standing over several nodes carries no capsule**, which is
/// `SetAuthority`'s own rule on the same head: one control there would be one
/// of several answers drawn as the answer.
#[test]
fn a_head_over_several_nodes_has_no_capsule() {
    let (panel, ctx) = console(PLAUSIBLE);
    let pane = pane_of(vec![folded_renderers()]);
    let laid = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
    let head = head(&panel, &pane, 0);
    assert_eq!(
        node_keep(&ctx, head, &pane.nodes[0]),
        None,
        "the folded renderers head drew a capsule"
    );
    // The place a capsule *would* be, pressed: nothing is there.
    let would_be = egui::Pos2::new(
        head.max.x - size::NODE_HEAD_PAD_X - size::MINI_PAD_X,
        head.center().y,
    );
    assert_eq!(
        laid.keep_procedure(&ctx, &pane, at(would_be)),
        None,
        "a press where the capsule would be asked to keep a head that is not a node"
    );
}

/// **The built-in camera carries none either**, and it is the other absence:
/// a node with an authority and no procedure behind it, so there is no source
/// to write (`docs/ir-spec.md`, *Several cameras*).
#[test]
fn the_built_in_camera_has_no_capsule() {
    let (panel, ctx) = console(PLAUSIBLE);
    let pane = pane_of(vec![built_in_camera()]);
    let laid = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
    let head = head(&panel, &pane, 0);
    assert!(
        pane.nodes[0].authority.is_some(),
        "the camera is a node and has an authority — this test is about the other absence"
    );
    assert_eq!(
        node_keep(&ctx, head, &pane.nodes[0]),
        None,
        "the built-in camera's head drew a capsule over a node with no source"
    );
    let would_be = egui::Pos2::new(
        head.max.x - size::NODE_HEAD_PAD_X - size::MINI_PAD_X,
        head.center().y,
    );
    assert_eq!(
        laid.keep_procedure(&ctx, &pane, at(would_be)),
        None,
        "a press where the capsule would be asked to keep the built-in camera"
    );
}

/// **A press on the capsule is not a press on a chip**, and the other way
/// round: the two controls at the right of this head own disjoint rectangles.
#[test]
fn the_capsule_and_the_chips_do_not_overlap() {
    let (panel, ctx) = console(PLAUSIBLE);
    let node = kept(
        "L1:0",
        "drift_shell",
        NodeAt {
            layer: Layer::L1,
            index: 0,
        },
    );
    let pane = pane_of(vec![node.clone()]);
    let laid = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
    let head = head(&panel, &pane, 0);
    let pill = node_keep(&ctx, head, &node).expect("a head with room for it");
    assert_eq!(
        laid.set_authority(&ctx, &pane, at(pill.center())),
        None,
        "a press in the middle of the capsule set an authority"
    );
    for (_, chip) in auth_chips(&ctx, head, &node) {
        assert_eq!(
            laid.keep_procedure(&ctx, &pane, at(chip.center())),
            None,
            "a press on an authority chip kept the node's procedure"
        );
    }
}

/// **Before the first frame there is nothing here to press.**
/// `Context::fonts` is not valid until a pass has run, and this capsule is as
/// wide as the word in it — the same guard every measured control carries.
#[test]
fn a_console_that_has_not_drawn_has_no_capsule() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let ctx = egui::Context::default();
    let pane = pane_of(vec![kept(
        "L1:0",
        "drift_shell",
        NodeAt {
            layer: Layer::L1,
            index: 0,
        },
    )]);
    let head = head(&panel, &pane, 0);
    assert_eq!(node_keep(&ctx, head, &pane.nodes[0]), None);
}

/// **Every pane draws its own**, which is the pane loop rather than a rule:
/// a console with two panes has a capsule per group in each of them, and a
/// press in the second answers with the second's deck.
#[test]
fn the_second_pane_keeps_its_own_deck() {
    let (panel, ctx) = console(PLAUSIBLE);
    let at_node = NodeAt {
        layer: Layer::L1,
        index: 0,
    };
    let first = pane_of(vec![kept("L1:0", "drift_shell", at_node)]);
    let second = Pane {
        deck: 1,
        material: "lattice_veil".to_owned(),
        ..pane_of(vec![kept("L1:0", "lattice_shell", at_node)])
    };
    let mut view = view_of(Vec::new());
    view.inspector = vec![first, second.clone()];
    assert_eq!(view.inspector.len(), PANES);
    let laid = inspector(panel.layout(), 1, &second, 0.0).expect("a second pane with room in it");
    let group = laid.group(&second.nodes, 0);
    let head = egui::Rect::from_min_max(
        group.min,
        egui::Pos2::new(group.max.x, group.min.y + size::NODE_HEAD_H),
    );
    let pill = node_keep(&ctx, head, &second.nodes[0]).expect("a head with room for it");
    assert_eq!(
        laid.keep_procedure(&ctx, &second, at(pill.center())),
        Some(Operation::KeepProcedure {
            deck: 1,
            node: at_node,
            id: None,
        }),
        "the second pane's capsule kept the first pane's deck"
    );
}
