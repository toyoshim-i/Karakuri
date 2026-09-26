use karakuri_layout::Point;
use karakuri_operation::gate::Class;

use crate::panel::Panel;
use crate::view::{
    arrangement, audio_in, bay_grip, deck_head, deck_name, inspector, keep_pill, learn_pill,
    library, look, map_pill, master, mcp_pill, mixer, outputs_with_plugin_name, program_bay,
    program_head, sequencer, slot_mcp_pill, staging, tracker_group, transition, transport, View,
    REGIONS,
};

/// Hit-tests Sequencer bay controls (cells, labels, minus button, mode pill)
/// from a single layout derivation.
pub(super) fn on_step(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    sequencer(
        ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .is_some_and(|bay| bay.owns(p))
}

/// Hit-tests the `back` capsule inside a candidate row before testing the row itself.
pub(super) fn on_back(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    staging(panel.layout(), &view.staging)
        .is_some_and(|bay| bay.back(ctx, &view.staging, p).is_some())
}

/// Hit-tests a press against rows in the Staging lane.
pub(super) fn on_candidate(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    staging(panel.layout(), &view.staging)
        .is_some_and(|bay| bay.keep(ctx, &view.staging, p).is_some())
}

/// Hit-tests active sinks in the Outputs row. Plugin chips return `false`
/// so pointer events fall through to underlying layers.
pub(super) fn on_sink(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    outputs_with_plugin_name(
        ctx,
        panel.layout(),
        view.opening,
        view.plugin_available,
        view.plugin_name,
    )
    .map(|row| row.told_plugin_name(0, view.plugin, view.plugin_available, view.plugin_name))
    .is_some_and(|row| row.chip_at(p).is_some())
}

/// Hit-tests the audio-in pill in the top row.
pub(super) fn on_audio(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    audio_in(ctx, panel.layout(), view.transport, view.audio.as_ref())
        .is_some_and(|pill| pill.hit(p))
}

/// Hit-tests the arrangement pill in the transport bar.
pub(super) fn on_pill(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    arrangement(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        None,
        &view.arrangement,
    )
    .is_some_and(|pill| pill.hit(p))
}

/// Hit-tests controls in the tracker group (offset figure, octave, tap) derived once.
pub(super) fn on_tracker(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    tracker_group(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
    )
    .is_some_and(|group| group.owns(p))
}

/// Hit-tests Mixer bay controls across all strips (knob, blend, tally, mask mini, and strip selection).
pub(super) fn on_strip(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| {
        bay.grab(p).is_some()
            || bay.solo(p).is_some()
            || bay.mute(p).is_some()
            || bay.blend(p).is_some()
            || bay.tally(p).is_some()
            || bay.mask(p).is_some()
            || bay.select(p).is_some()
    })
}

/// Hit-tests the four capsules in the transition row (including the `go` capsule).
pub(super) fn on_transition(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.owns(p))
}

/// Hit-tests the `learn` pill at the end of the transport row.
pub(super) fn on_learn(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    learn_pill(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        view.learn,
    )
    .is_some_and(|pill| pill.hit(p))
}

/// Hit-tests the map pill in the transport bar.
pub(super) fn on_map(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    map_pill(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
    )
    .is_some_and(|row| row.pill.contains(egui::Pos2::new(p.x, p.y)))
}

pub(super) fn on_look(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    look(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        None,
        &view.arrangement,
        view.look,
    )
    .is_some_and(|row| row.owns(p))
}

/// Hit-tests the `rec` pill at the end of the transport row.
pub(super) fn on_rec(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transport(ctx, panel.layout(), view.transport).is_some_and(|row| row.on_rec(p))
}

/// Hit-tests the tempo figure at the head of the transport row.
/// Hits on the surrounding guard band fall through to egui (ADR-0291).
pub(super) fn on_tempo(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transport(ctx, panel.layout(), view.transport).is_some_and(|row| row.on_tempo(p))
}

/// Hit-tests Master bay controls (out knob, effect knobs, feedback cut chip) in one derivation.
pub(super) fn on_master(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    master(
        ctx,
        panel.layout(),
        view.master_out,
        view.master_chain.as_ref(),
        &view.chain_choices(),
    )
    .is_some_and(|row| row.owns(p))
}

/// Hit-tests the editable name in each Inspector pane's head.
pub(super) fn on_deck_name(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        let policy = view
            .slot_policies
            .get(pane.deck)
            .copied()
            .unwrap_or_default();
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| {
                let mcp = slot_mcp_pill(ctx, &at, pane, policy).map(|p| p.pill);
                deck_name(ctx, &at, pane, view.naming_set_in(index), mcp)
            })
            .is_some_and(|named| named.hit(p))
    })
}

/// Hit-tests the `keep` capsule in each Inspector pane's head.
pub(super) fn on_keep(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| keep_pill(ctx, &at, pane))
            .is_some_and(|pill| pill.hit(p))
    })
}

/// The slot MCP policy capsule in each Inspector pane's head.
pub(super) fn on_slot_mcp(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        let policy = view
            .slot_policies
            .get(pane.deck)
            .copied()
            .unwrap_or_default();
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| slot_mcp_pill(ctx, &at, pane, policy))
            .is_some_and(|pill| pill.hit(p))
    })
}

/// Hit-tests the pulldown chevron (`▾`) and open card in each Inspector pane's head (ADR-0305).
pub(super) fn on_pane_target(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    let room = crate::view::to_egui(panel.layout().viewport());
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| view.pane_pulldown(ctx, &at, pane, index))
            .is_some_and(|target| target.hit(p) || target.picked(room, p).is_some())
    })
}

/// Hit-tests the `keep` capsule on each node group head across Inspector panes.
pub(super) fn on_node_keep(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.keep_procedure(ctx, pane, p).is_some())
    })
}

/// Hit-tests a press against Inspector pane deck head controls across all panes.
pub(super) fn on_deck_head(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| deck_head(ctx, &at, pane))
            .is_some_and(|head| head.owns(p))
    })
}

/// Hit-tests the renderer selection chips across all Inspector panes.
pub(super) fn on_rend(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.select_renderer(ctx, pane, p).is_some())
    })
}

/// Hit-tests a parameter row's publish mark (ADR-0329) across all Inspector panes.
pub(super) fn on_publish(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    let _ = ctx;
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.publishing(pane, p).is_some())
    })
}

/// Hit-tests a node group's `uses` capsule and its dropdown card (ADR-0305).
pub(super) fn on_uses(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    let room = crate::view::to_egui(panel.layout().viewport());
    if let Some((pane_at, node, input)) = view.wiring_open() {
        let picked = view.inspector.get(pane_at).is_some_and(|pane| {
            inspector(panel.layout(), pane_at, pane, view.scroll_in(pane_at))
                .is_some_and(|at| at.wired(ctx, pane, room, (node, input), p).is_some())
        });
        if picked {
            return true;
        }
    }
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.uses_chip(ctx, pane, p).is_some())
    })
}

/// Hit-tests the authority level chips (`man / sug / auto`, ADR-0319) on node group heads.
pub(super) fn on_auth(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.set_authority(ctx, pane, p).is_some())
    })
}

/// Hit-tests interactive sensitivity chips (curve chip and `take back`) under bound parameters.
pub(super) fn on_sens(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.sensitivity(ctx, pane, p).is_some())
    })
}

/// Hit-tests parameter row faders across all Inspector panes.
pub(super) fn on_param(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.owns(pane, p))
    })
}

/// Hit-tests the fold/drag grip in each bay head. Panes fold via boundary dragging (ADR-0300).
pub(super) fn on_grip(panel: &Panel, _ctx: &egui::Context, _view: &View, p: Point) -> bool {
    REGIONS
        .iter()
        .any(|region| bay_grip(panel.layout(), region.name).is_some_and(|grip| grip.hit(p)))
}

/// Hit-tests the solo capsule in the Program bay head.
pub(super) fn on_solo(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    program_head(ctx, panel.layout(), view.opening).is_some_and(|head| head.hit(p))
}

/// Hit-tests the four deck preview cells in the Program bay.
pub(super) fn on_cells(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    program_bay(panel.layout(), view.canvas).is_some_and(|bay| bay.owns(p))
}

/// Hit-tests the dynamic scope chips in the Library bay head.
pub(super) fn on_scope(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
    .is_some_and(|bay| bay.chip(ctx, &view.scopes, p).is_some())
}

/// Hit-tests the two filter fields in the Library bay below the scope chips.
pub(super) fn on_filter(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
    .is_some_and(|bay| bay.filter(&view.holds, view.filters(), p).is_some())
}

/// Hit-tests the kind chips row in the Library bay.
pub(super) fn on_kinds(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
    .is_some_and(|bay| bay.kind(ctx, view.filters(), p).is_some())
}

/// Hit-tests row badges in the Library bay to host tooltips (ADR-0338).
pub(super) fn on_badges(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    let at = egui::Pos2::new(p.x, p.y);
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
    .is_some_and(|bay| {
        let rows = view.rows();
        bay.list.contains(at)
            && bay.drawn().any(|index| {
                bay.badges(ctx, index, &rows.badges(index))
                    .any(|(_, box_)| box_.contains(at))
            })
    })
}

/// Hit-tests the `params` chip in the Library bay foot for the selected Set (ADR-0156).
pub(super) fn on_read(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
    .is_some_and(|bay| {
        bay.read(
            ctx,
            view.target(),
            view.sets().get(view.cursor_row()).map(String::as_str),
            p,
        )
        .is_some()
    })
}

/// Hit-tests a press against the Library footer `load` button or deck pulldown pill.
pub(super) fn on_load(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
    .is_some_and(|bay| {
        let load = bay.load(ctx, view.target());
        load.hit_button(p) || load.hit_deck(p)
    })
}

/// Hit-tests the star icon on Library bay rows before testing the row itself (ADR-0156).
pub(super) fn on_star(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
    .is_some_and(|bay| bay.starred(view.rows(), &view.starred, p).is_some())
}

/// Hit-tests rows of the Library bay listing (taking a Set or landing a version, ADR-0156).
pub(super) fn on_row(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
    .is_some_and(|bay| {
        bay.take(view.rows(), p).is_some() || bay.land(view.versions(), view.target(), p).is_some()
    })
}

/// Hit-tests the four MCP class pills across bay headers.
pub(super) fn on_mcp(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    Class::ALL.iter().any(|class| {
        mcp_pill(ctx, panel.layout(), *class, view.opening).is_some_and(|pill| pill.hit(p))
    })
}
