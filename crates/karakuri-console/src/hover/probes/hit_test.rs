use egui::Pos2;
use karakuri_layout::Point;
use karakuri_operation::gate::Class;
use karakuri_operation::{Layer, Operation, Output};

use crate::panel::Panel;
use crate::view::{
    arrangement, audio_in, deck_head, inspector, keep_pill, library, look, master, mcp_pill, mixer,
    outputs, program_bay, program_head, sequencer, staging, tracker_group, transition, transport,
    Field, KindChip, Scope, View,
};

// Hit-test derivations per tipped control, refining `crate::input` probes to identify
// specific sub-controls without diverging from frame layout.

pub(crate) fn on_program_sink(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    outputs(ctx, panel.layout(), view.opening)
        .and_then(|row| row.chip_at(p))
        .is_some_and(|out| out == Output::Program)
}

pub(crate) fn on_projector_sink(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    outputs(ctx, panel.layout(), view.opening)
        .and_then(|row| row.chip_at(p))
        .is_some_and(|out| matches!(out, Output::Projector(_)))
}

pub(crate) fn on_audio(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    audio_in(ctx, panel.layout(), view.transport, view.audio.as_ref())
        .is_some_and(|pill| pill.hit(p))
}

pub(crate) fn on_tap(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    tracker_group(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
    )
    .is_some_and(|group| group.tapped(p).is_some())
}

pub(crate) fn on_octave(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    tracker_group(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
    )
    .is_some_and(|group| group.octave(p).is_some())
}

pub(crate) fn on_offset(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    tracker_group(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
    )
    .is_some_and(|group| group.nudge(p).is_some())
}

pub(crate) fn on_learn_pill(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    crate::view::learn_pill(
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

pub(crate) fn on_map_pill(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    crate::view::map_pill(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
    )
    .is_some_and(|row| row.pill.contains(Pos2::new(p.x, p.y)))
}

pub(crate) fn on_pill(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
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

pub(crate) fn on_tonemap(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    look_row(panel, ctx, view).is_some_and(|row| row.tonemap(p).is_some())
}

pub(crate) fn on_exposure(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    look_row(panel, ctx, view).is_some_and(|row| row.exposure(p).is_some())
}

/// The look group, derived the one way [`crate::input::claim`] derives it.
fn look_row(panel: &Panel, ctx: &egui::Context, view: &View) -> Option<crate::view::LookRow> {
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
}

pub(crate) fn on_rec(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transport(ctx, panel.layout(), view.transport).is_some_and(|row| row.on_rec(p))
}

pub(crate) fn on_tempo(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transport(ctx, panel.layout(), view.transport).is_some_and(|row| row.on_tempo(p))
}

pub(crate) fn on_fader(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| bay.grab(p).is_some())
}

pub(crate) fn on_blend(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| bay.blend(p).is_some())
}

pub(crate) fn on_tally(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| bay.tally(p).is_some())
}

pub(crate) fn on_mask(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| bay.mask(p).is_some())
}

pub(crate) fn on_select(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| bay.select(p).is_some())
}

pub(crate) fn on_shape(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.shape(p).is_some())
}

pub(crate) fn on_quantum(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.quantum(p).is_some())
}

pub(crate) fn on_length(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.length(p).is_some())
}

/// Hit-tests the transition `go` capsule over remaining row bounds.
pub(crate) fn on_go(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.owns(p))
}

pub(crate) fn on_master_out(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    master(
        ctx,
        panel.layout(),
        view.master_out,
        view.master_chain.as_ref(),
        &view.chain_choices(),
    )
    .is_some_and(|row| row.grab(p).is_some())
}

pub(crate) fn on_master_chip(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    master(
        ctx,
        panel.layout(),
        view.master_out,
        view.master_chain.as_ref(),
        &view.chain_choices(),
    )
    .is_some_and(|row| row.chip(p).is_some() || row.chose(p, &view.chain_choices()).is_some())
}

pub(crate) fn on_keep(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| keep_pill(ctx, &at, pane))
            .is_some_and(|pill| pill.hit(p))
    })
}

/// The mark and not the card, where `input`'s row answers for both: a tip
/// explains a control an operator is pointing at, and while the card is down
/// the hover layer is not what the next press is about — `claim`'s rule 2 has
/// already taken it.
pub(crate) fn on_pane_target(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| view.pane_pulldown(ctx, &at, pane, index))
            .is_some_and(|target| target.hit(p))
    })
}

pub(crate) fn on_node_keep(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.keep_procedure(ctx, pane, p).is_some())
    })
}

pub(crate) fn on_sync(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.sync(p).is_some())
}

pub(crate) fn on_anchor(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.reanchor(p).is_some())
}

pub(crate) fn on_scrub(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.scrub(p).is_some())
}

/// Hit-tests the deck head capacity chip via `DeckHead::resized`.
pub(crate) fn on_size(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.resized(p).is_some())
}

/// The `re-salt` capsule. There is no state in which it is drawn and inert, so
/// this is `hit_salt` and `re_salted` at once; it is the latter for the chip
/// above's reason and for the press handler's.
pub(crate) fn on_salt(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.re_salted(p).is_some())
}

pub(crate) fn on_compositing(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.compositing(p).is_some())
}

/// Every pane's deck head, derived per pane the way `crate::input` derives it,
/// asked one of its own questions.
pub(crate) fn on_head(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    ask: impl Fn(&crate::view::DeckHead) -> bool,
) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| deck_head(ctx, &at, pane))
            .is_some_and(|head| ask(&head))
    })
}

pub(crate) fn on_rend(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.select_renderer(ctx, pane, p).is_some())
    })
}

pub(crate) fn on_param(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.owns(pane, p))
    })
}

pub(crate) fn on_publish(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.publishing(pane, p).is_some())
    })
}

pub(crate) fn on_uses(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
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

pub(crate) fn on_auth(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.set_authority(ctx, pane, p).is_some())
    })
}

pub(crate) fn on_curve(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_sens(panel, ctx, view, p, |op| {
        matches!(op, Operation::AttachSignal { .. })
    })
}

pub(crate) fn on_take_back(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_sens(panel, ctx, view, p, |op| {
        matches!(op, Operation::TakeParamBack { .. })
    })
}

/// Hit-tests sensitivity row controls using `InspectorPane::sensitivity` operations.
pub(crate) fn on_sens(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    p: Point,
    which: impl Fn(&Operation) -> bool,
) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| at.sensitivity(ctx, pane, p))
            .is_some_and(|op| which(&op))
    })
}

pub(crate) fn on_solo(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    program_head(ctx, panel.layout(), view.opening).is_some_and(|head| head.hit(p))
}

pub(crate) fn on_cell_a(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_cell(panel, view, p, 0)
}

pub(crate) fn on_cell_b(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_cell(panel, view, p, 1)
}

pub(crate) fn on_cell_c(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_cell(panel, view, p, 2)
}

pub(crate) fn on_cell_d(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_cell(panel, view, p, 3)
}

/// Hit-tests a preview cell against the specified deck index in `ProgramBay`.
pub(crate) fn on_cell(panel: &Panel, view: &View, p: Point, deck: u8) -> bool {
    program_bay(panel.layout(), view.canvas).is_some_and(|bay| bay.cell(p) == Some(deck))
}

pub(crate) fn on_scope_all(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_scope(panel, ctx, view, p, Scope::AllSets)
}

pub(crate) fn on_scope_mine(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_scope(panel, ctx, view, p, Scope::MySets)
}

pub(crate) fn on_scope_presets(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_scope(panel, ctx, view, p, Scope::Presets)
}

pub(crate) fn on_scope_folder(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_scope(panel, ctx, view, p, Scope::Folder)
}

pub(crate) fn on_scope_history(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_scope(panel, ctx, view, p, Scope::History)
}

/// One scope chip, told from the four beside it by the chip the bay says the
/// pointer is on — `LibraryBay::chip`'s own answer, which carries the scope
/// beside the operation, rather than a second walk of the row.
pub(crate) fn on_scope(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    p: Point,
    which: Scope,
) -> bool {
    library_bay(panel, view)
        .and_then(|bay| bay.chip(ctx, &view.scopes, p))
        .is_some_and(|chosen| chosen.scope == which)
}

pub(crate) fn on_holds(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_field(panel, view, p, Field::Holds)
}

pub(crate) fn on_kind_l1(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Layer(Layer::L1))
}

pub(crate) fn on_kind_l2(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Layer(Layer::L2))
}

pub(crate) fn on_kind_l3(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Layer(Layer::L3))
}

pub(crate) fn on_kind_l4(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Layer(Layer::L4))
}

pub(crate) fn on_kind_field(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Layer(Layer::Field))
}

pub(crate) fn on_kind_sets(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Sets)
}

/// One kind chip, told from the five beside it by the chip the bay says is at
/// that point — `LibraryBay::kind_chips` is the same walk the paint makes and
/// the same one a press is resolved against, so a tip and a press cannot land
/// on two different chips.
pub(crate) fn on_kind(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    p: Point,
    want: KindChip,
) -> bool {
    let at = Pos2::new(p.x, p.y);
    library_bay(panel, view).is_some_and(|bay| {
        bay.kinds.is_some_and(|row| row.contains(at))
            && bay
                .kind_chips(ctx)
                .any(|(chip, box_)| chip == want && box_.contains(at))
    })
}

/// A row's badges, which is the one readout in this bay's list: nothing is
/// pressed there, and the tip is what says so and what the words mean. Asked of
/// the rows the bay is drawing, so a badge under the pointer is a badge on
/// screen.
pub(crate) fn on_badges(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    let at = Pos2::new(p.x, p.y);
    library_bay(panel, view).is_some_and(|bay| {
        let rows = view.rows();
        bay.list.contains(at)
            && bay.drawn().any(|index| {
                bay.badges(ctx, index, &rows.badges(index))
                    .any(|(_, box_)| box_.contains(at))
            })
    })
}

/// One filter field. `LibraryBay::filter` answers with the `ListSets` a press
/// asks for and not with which box it landed in, so the box is asked for —
/// `LibraryBay::field` is the same rectangle that method tests, asked by name.
pub(crate) fn on_field(panel: &Panel, view: &View, p: Point, which: Field) -> bool {
    library_bay(panel, view)
        .and_then(|bay| bay.field(which))
        .is_some_and(|box_| box_.contains(Pos2::new(p.x, p.y)))
}

pub(crate) fn on_read(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library_bay(panel, view).is_some_and(|bay| {
        bay.read(ctx, view.target(), view.rows().set(view.cursor_row()), p)
            .is_some()
    })
}

pub(crate) fn on_load_button(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library_bay(panel, view).is_some_and(|bay| bay.load(ctx, view.target()).hit_button(p))
}

pub(crate) fn on_load_deck(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library_bay(panel, view).is_some_and(|bay| bay.load(ctx, view.target()).hit_deck(p))
}

pub(crate) fn on_star(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    library_bay(panel, view).is_some_and(|bay| bay.starred(view.rows(), &view.starred, p).is_some())
}

/// The Library bay, derived the one way [`crate::input::claim`] derives it.
fn library_bay(panel: &Panel, view: &View) -> Option<crate::view::LibraryBay> {
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
}

pub(crate) fn on_mcp_live_deck(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_mcp(panel, ctx, view, p, Class::LiveDeck)
}

pub(crate) fn on_mcp_mix_faders(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_mcp(panel, ctx, view, p, Class::MixFaders)
}

pub(crate) fn on_mcp_master_effects(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    p: Point,
) -> bool {
    on_mcp(panel, ctx, view, p, Class::MasterEffects)
}

pub(crate) fn on_mcp_inputs_and_outputs(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    p: Point,
) -> bool {
    on_mcp(panel, ctx, view, p, Class::InputsAndOutputs)
}

/// One class pill. The four are in four different bay heads and cannot be one
/// laid-out box, so the class is the argument that lays one out — which is the
/// press handler's own arrangement, one derivation asked four times.
pub(crate) fn on_mcp(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    p: Point,
    class: Class,
) -> bool {
    mcp_pill(ctx, panel.layout(), class, view.opening).is_some_and(|pill| pill.hit(p))
}

pub(crate) fn on_bank_1(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_bank(panel, ctx, view, p, 0)
}

pub(crate) fn on_bank_2(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_bank(panel, ctx, view, p, 1)
}

pub(crate) fn on_bank_3(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_bank(panel, ctx, view, p, 2)
}

pub(crate) fn on_bank_4(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_bank(panel, ctx, view, p, 3)
}

/// One bank pill, told from the three beside it by the pattern the press would
/// name: `Sequencer::press` answers `SelectPattern` carrying the bank, which is
/// this bay's own rule that *every arm names the bank*.
pub(crate) fn on_bank(panel: &Panel, ctx: &egui::Context, view: &View, p: Point, bank: u8) -> bool {
    on_seq(
        panel,
        ctx,
        view,
        p,
        |op| matches!(op, Operation::SelectPattern { pattern } if *pattern == bank),
    )
}

pub(crate) fn on_step(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_seq(panel, ctx, view, p, |op| {
        matches!(op, Operation::SetStep { .. })
    })
}

pub(crate) fn on_lane_label(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_seq(panel, ctx, view, p, |op| {
        matches!(op, Operation::SetLaneMute { .. })
    })
}

pub(crate) fn on_lane_remove(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_seq(panel, ctx, view, p, |op| {
        matches!(op, Operation::RemoveLane { .. })
    })
}

pub(crate) fn on_step_mode(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_seq(panel, ctx, view, p, |op| {
        matches!(op, Operation::SetPatternGrid { .. })
    })
}

/// Hit-tests controls in the Sequencer bay by matching against `Sequencer::press` operations.
pub(crate) fn on_seq(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    p: Point,
    which: impl Fn(&Operation) -> bool,
) -> bool {
    sequencer_bay(panel, ctx, view)
        .and_then(|bay| bay.press(p))
        .is_some_and(|op| which(&op))
}

/// Hit-tests point `p` against the `+ lane` pill in the sequencer bay.
pub(crate) fn on_add_lane(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    // The one derivation, and the choices it was laid out from asked again:
    // the card's items are the chooser's own listing, so a bay drawn from one
    // reading and asked against another would answer for a card it did not
    // draw — `Sequencer::chose`'s own re-check, from this side.
    let choices = view.lane_choices();
    sequencer(ctx, panel.layout(), view.sequencer.as_ref(), &choices)
        .is_some_and(|bay| matches!(bay.chose(p, &choices), Some(crate::view::Chose::Open)))
}

/// The Sequencer bay, derived the one way [`crate::input::claim`] derives it.
fn sequencer_bay(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
) -> Option<crate::view::Sequencer> {
    sequencer(
        ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
}

pub(crate) fn on_candidate(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    staging(panel.layout(), &view.staging)
        .is_some_and(|bay| bay.keep(ctx, &view.staging, p).is_some())
}
