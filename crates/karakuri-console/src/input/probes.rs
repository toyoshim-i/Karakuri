use karakuri_layout::Point;
use karakuri_operation::gate::Class;

use crate::panel::Panel;
use crate::view::{Field, KindChip, Scope, View, BAY_GRIPS, DECKS};

use super::handlers::*;

/// Table of input hit-test probes executed by [`claim`].
///
/// Ordered roughly from cheapest hit-tests to more expensive layout derivations.
/// See ADR-0274 for table-driven probe architecture.
pub const PROBES: [Probe; 39] = [
    Probe {
        name: "the Outputs row's sinks",
        claims: 1,
        ask: on_sink,
    },
    Probe {
        name: "the audio-in pill",
        claims: 1,
        ask: on_audio,
    },
    Probe {
        name: "the tracker group's three",
        claims: 3,
        ask: on_tracker,
    },
    Probe {
        name: "the transport row's learn pill",
        claims: 1,
        ask: on_learn,
    },
    // Readout registered to capture pointer clicks and prevent fall-through to egui,
    // also hosting the pill's tooltip.
    Probe {
        name: "the transport row's map pill",
        claims: 1,
        ask: on_map,
    },
    Probe {
        name: "the arrangement pill",
        claims: 1,
        ask: on_pill,
    },
    Probe {
        name: "the look group's two",
        claims: 2,
        ask: on_look,
    },
    Probe {
        name: "the transport row's rec pill",
        claims: 1,
        ask: on_rec,
    },
    Probe {
        name: "the transport row's tempo figure",
        claims: 1,
        ask: on_tempo,
    },
    Probe {
        name: "a mixer strip's five",
        claims: 5,
        ask: on_strip,
    },
    Probe {
        name: "the transition row's four",
        claims: 4,
        ask: on_transition,
    },
    Probe {
        name: "the Master bay's five",
        claims: 5,
        ask: on_master,
    },
    Probe {
        name: "the Inspector pane heads' name",
        claims: 1,
        ask: on_deck_name,
    },
    Probe {
        name: "the Inspector pane heads' keep",
        claims: 1,
        ask: on_keep,
    },
    Probe {
        name: "the Inspector pane heads' slot mcp policy",
        claims: 1,
        ask: on_slot_mcp,
    },
    // Inspector pane header deck pulldown pill and menu trigger.
    Probe {
        name: "the Inspector pane heads' deck pulldown",
        claims: 2,
        ask: on_pane_target,
    },
    Probe {
        name: "a deck head's seven",
        claims: 7,
        ask: on_deck_head,
    },
    Probe {
        name: "the renderer chips",
        claims: 1,
        ask: on_rend,
    },
    Probe {
        name: "a parameter row's fader",
        claims: 1,
        ask: on_param,
    },
    Probe {
        name: "a parameter row's publish mark",
        claims: 1,
        ask: on_publish,
    },
    Probe {
        name: "a node group's `uses` capsule and its card",
        claims: 2,
        ask: on_uses,
    },
    Probe {
        name: "a node head's three authority chips",
        claims: crate::view::AUTHORITIES.len(),
        ask: on_auth,
    },
    // Capsule at right of node group head.
    Probe {
        name: "a node head's keep capsule",
        claims: 1,
        ask: on_node_keep,
    },
    Probe {
        name: "a sensitivity row's curve and take back",
        claims: 2,
        ask: on_sens,
    },
    Probe {
        name: "the Program bay head's solo",
        claims: 1,
        ask: on_solo,
    },
    Probe {
        name: "the grip in a bay head",
        claims: BAY_GRIPS,
        ask: on_grip,
    },
    Probe {
        name: "the deck preview cells",
        claims: DECKS,
        ask: on_cells,
    },
    Probe {
        name: "the Library bay's scope chips",
        claims: Scope::ALL.len(),
        ask: on_scope,
    },
    Probe {
        name: "the Library bay's filter fields",
        claims: Field::ALL.len(),
        ask: on_filter,
    },
    Probe {
        name: "the Library bay's kind chips",
        claims: KindChip::ALL.len(),
        ask: on_kinds,
    },
    Probe {
        name: "the Library bay's row badges",
        claims: 1,
        ask: on_badges,
    },
    Probe {
        name: "the params chip in the Library bay's foot",
        claims: 1,
        ask: on_read,
    },
    Probe {
        name: "the Library bay's load button and deck pulldown",
        claims: 2,
        ask: on_load,
    },
    Probe {
        name: "the Library bay's stars",
        claims: 1,
        ask: on_star,
    },
    Probe {
        name: "the Library bay's list",
        claims: 2,
        ask: on_row,
    },
    Probe {
        name: "the class pills",
        claims: Class::ALL.len(),
        ask: on_mcp,
    },
    Probe {
        name: "the Sequencer bay's cells, labels, minus glyphs, mode pill, bank pills and + lane",
        claims: SEQ_CONTROLS,
        ask: on_step,
    },
    Probe {
        name: "the Staging lane's back capsules",
        claims: 1,
        ask: on_back,
    },
    Probe {
        name: "the Staging lane's rows",
        claims: 1,
        ask: on_candidate,
    },
];

/// Hit-test probe descriptor registering a control's name, claim count, and test fn.
pub struct Probe {
    /// Human-readable descriptor of what the derivation answers for, keyed by the host.
    pub name: &'static str,
    /// Number of distinct controls handled by this probe.
    pub claims: usize,
    /// Derivation testing whether the point hits this control given panel layout, egui ctx, and view.
    pub ask: fn(&Panel, &egui::Context, &View, Point) -> bool,
}

/// Upper bound on the number of controls the Sequencer bay can claim across all lanes.
const SEQ_CONTROLS: usize = DECKS * (karakuri_pattern::SLOTS + 2) + 1 + karakuri_pattern::BANKS + 1;

/// Total number of hit-testable controls across all [`PROBES`].
pub const CONTROLS: usize = summed(&PROBES);

/// Sums claims across all probes in a const context.
const fn summed(probes: &[Probe]) -> usize {
    let mut total = 0;
    let mut at = 0;
    while at < probes.len() {
        total += probes[at].claims;
        at += 1;
    }
    total
}
