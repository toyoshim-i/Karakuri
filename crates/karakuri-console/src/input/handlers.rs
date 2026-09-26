use karakuri_layout::Point;
use karakuri_operation::gate::Class;

use crate::panel::Panel;
use crate::view::{
    arrangement, audio_in, bay_grip, deck_head, deck_name, inspector, keep_pill, learn_pill,
    library, look, map_pill, master, mcp_pill, mixer, outputs_with_plugin_name, program_bay,
    program_head, sequencer, slot_mcp_pill, staging, tracker_group, transition, transport, View,
    REGIONS,
};

/// The Sequencer bay's controls, derived once for all of them, which is a mixer
/// strip's arrangement one bay down: a cell, a label, a minus and the mode pill
/// are four questions about one laid-out bay, and a second walk would put the
/// cell a press lands on somewhere the cell that was painted is not.
///
/// A console with no pattern behind it pays one branch — [`sequencer`]'s first
/// line is the reading, and `None` is a bay that draws nothing.
pub(super) fn on_step(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    sequencer(
        ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .is_some_and(|bay| bay.owns(p))
}

/// The `back` capsule at the end of a candidate row, asked before the row it
/// sits in: the capsule is *inside* the row, so the order here is what makes a
/// press on the capsule reach the capsule — rule 4's *a control claims what it
/// acts on and no more*, which is the Library bay's star and its row one bay
/// up.
///
/// The lane is derived twice rather than once for both, which is the Library
/// bay's arrangement rather than a mixer strip's: a value held across both
/// questions would outlive the question it answers, and [`staging`] is a
/// rectangle and a count rather than a walk of anything.
///
/// The candidates go in with the point, exactly as the Library's listing does:
/// what a row *is* — which node, which deck, which verdict — is a value the
/// host derived off a deck and handed over, and this crate holds none of it. A
/// lane with nothing outstanding is `None` and pays one branch.
pub(super) fn on_back(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    staging(panel.layout(), &view.staging)
        .is_some_and(|bay| bay.back(ctx, &view.staging, p).is_some())
}

/// Hit-tests a press against rows in the Staging lane.
pub(super) fn on_candidate(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    staging(panel.layout(), &view.staging)
        .is_some_and(|bay| bay.keep(ctx, &view.staging, p).is_some())
}

/// The Outputs row's sinks, and the first control this console drew — the rule
/// above is measured against it and `tests/outputs.rs` is where the arithmetic
/// is. Before the first frame there are no fonts and no drawn control at all,
/// which [`outputs`] answers `None` to.
///
/// It was the one sink until 2026-09-09. The row draws the list it has — the
/// program view, a projector window and the two plugin chips — and
/// [`crate::view::Outputs::chip_at`] is the whole row asked at once, off the
/// same derivation that paints them. The two plugin chips answer `false`: a
/// control that switches nothing does not take a press away from the row it
/// sits in, so a press there falls through to whatever is under it exactly as a
/// press on the row's ground does.
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

/// The two pills that are not in a bay, and the only two asked with their cards
/// already known to be shut: rule 2 has answered for the open case above, so
/// these are the capsules alone.
///
/// The audio-in pill is asked first because it is drawn first — the arrangement
/// pill is laid out from where it ends, so asking in the other order would
/// derive the second from the first anyway.
pub(super) fn on_audio(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    audio_in(ctx, panel.layout(), view.transport, view.audio.as_ref())
        .is_some_and(|pill| pill.hit(p))
}

/// The arrangement pill, laid out from where the audio-in pill ends — which is
/// why it is asked second: asking in the other order would derive this one from
/// that one anyway.
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

/// The tracker group's other three, derived once for all of them, which is a
/// strip's arrangement in a row rather than in a column: the offset's figure is
/// as wide as the number in it and the octave is laid out from where the tap
/// ends, so a second walk would put the chip a press lands on somewhere the
/// mark is not.
///
/// Asked after the two pills and before the arrangement's, which is the order
/// the row is laid out in and is what makes this the cheap answer it is: it
/// derives the row and the audio-in pill, where [`on_pill`] derives this group
/// as well. A console told nothing about the tracker pays one branch — the
/// `tracker?` is [`tracker_group`]'s first line.
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

/// The bay is derived once for all of its controls, since a knob, a blend chip,
/// a tally chip, a mask mini and the strip they sit in are five questions about
/// one laid-out strip.
///
/// The strip is asked last, and it is the only one of the five that could
/// answer for the other four. `Mixer::select` is the column's whole rectangle,
/// so this chain would come out the same with it first — and the order is the
/// caller's rather than an optimisation: `karakuri/src/main.rs` tries the four
/// that name something inside the column and takes the strip as what is left
/// over, so a press on a knob is that knob's and a press on the name, the
/// number, the meter or a fader's track is the deck's. Written in one order in
/// both places, the two cannot come apart.
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

/// The transition row's four capsules, derived once for all of them, which is a
/// strip's arrangement one row down: the pills are laid end to end from the
/// block's left padding, so where the third is depends on how wide the first
/// two words are, and a second walk would put the capsule a press lands on
/// somewhere the word is not.
///
/// It is asked whatever the deck is doing. `on_strip` above pays nothing on a
/// console with no deck because there are no strips to lay out; this row is the
/// console's own setting and is drawn either way, so it is a laid-out row and
/// four word widths on every press that gets this far.
///
/// The `go` capsule is claimed with no deck behind it too, and that is
/// `TransitionRow::owns`' own sentence rather than a decision here: a press on
/// it is refused and the refusal is the act, so the panel is what has to answer
/// for the press.
pub(super) fn on_transition(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.owns(p))
}

/// The two at the end of the transport row, derived once for both: the exposure
/// track's place is measured from the tone map's capsule, so they are two
/// questions about one laid-out group. The `learn` pill, derived the one way
/// [`crate::view::learn_pill`] derives it — armed or not, since a control's
/// rectangle does not move with its lamp.
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

/// The `map` pill, which is a readout: the pointer reaches it and a press on it
/// asks for nothing.
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

/// The `rec` pill at the very end of the transport row, and it is the row
/// itself that answers: the pill takes the row's right padding and the health
/// capsule and the frame readout are laid out backwards from it, so where it is
/// *is* [`transport`]'s answer and a second derivation beside the row would be
/// a second one.
///
/// The cheapest probe here after the sink: one laid-out row, which is what
/// [`on_audio`] and [`on_tracker`] each derive before they derive anything
/// else. A console nobody has told about recording draws no pill and this
/// answers `false` without a rectangle.
pub(super) fn on_rec(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transport(ctx, panel.layout(), view.transport).is_some_and(|row| row.on_rec(p))
}

/// The tempo figure at the head of the transport row, and the row answers for
/// it exactly as it answers for the `rec` pill one item along: the figure is
/// the row's first item and the control *is* the reading, so where it is and
/// how wide it is are [`transport`]'s answer.
///
/// The band is inside `on_tempo` and not here, which is [`on_tracker`]'s
/// arrangement for the octave's inert half: a press on the guard either side of
/// the number asks for a tempo a hand is not trusted to have meant, so it is
/// not this control's and falls through to `egui`
/// ([ADR-0291](../../../docs/adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)).
pub(super) fn on_tempo(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transport(ctx, panel.layout(), view.transport).is_some_and(|row| row.on_tempo(p))
}

/// The Master bay's five, and the bay is derived for them exactly as the
/// mixer's is for its five: one derivation, asked once, answering for the out
/// knob, the three effect rows' knobs and the feedback row's cut chip. A
/// console with no level behind it has no row at all and pays nothing, and one
/// with a level and no chain has the out knob alone.
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

/// The name in each Inspector pane's head, and it is [`on_keep`]'s arrangement
/// at the other end of the same row: the pane is derived per index and the run
/// from the pane. What it takes besides the pane is what that head is *asking*
/// for, because a field being typed into is a different run of text and a
/// different width — [`crate::view::View::naming_set_in`] is that answer, and
/// it is the same one [`crate::view::View::draw`] paints from.
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

/// The `keep` capsule in each Inspector pane's head, one derivation per pane:
/// the pane is [`inspector`]'s answer and the capsule is [`keep_pill`]'s, which
/// is [`on_deck_head`]'s own arrangement one row up. One galley lookup per
/// pane, for the word in the capsule.
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

/// The `▾` in each Inspector pane's head, and the card a press on it brings
/// down — two controls and one row, which is the Library bay's `load` button
/// and deck pulldown's arrangement and its reason: they are one derivation and
/// one ask.
///
/// The mark emits nothing, which is what makes it a control here and no row of
/// its own on the operations page: it opens a list, and *open the list* is not
/// something a map or a model could want to say (ADR-0305). The card's rows
/// emit `PointPane`.
///
/// The card is asked first, because it is drawn over the pane the mark hangs
/// off — a press inside it belongs to the card and not to the groups under it.
/// That is [`on_uses`]'s own order one row down, and while the card is down
/// [`claim`]'s rule 2 has already answered anyway; this row is what makes the
/// *mark* reachable when it is not.
pub(super) fn on_pane_target(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    let room = crate::view::to_egui(panel.layout().viewport());
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| view.pane_pulldown(ctx, &at, pane, index))
            .is_some_and(|target| target.hit(p) || target.picked(room, p).is_some())
    })
}

/// The `keep` capsule on a node group's head, and it is [`on_auth`]'s
/// arrangement at the other end of the same row: the pane is derived per index
/// and the capsule from the group's head.
///
/// A head with nothing to keep is not a target — `Node::keep` is `None` on a
/// head standing over several nodes and on the built-in camera — so the two
/// absences the mock draws fall out of the derivation rather than out of a
/// check here.
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

/// The renderer chips in the Inspector's panes, and the count is one for the
/// Library list's reason: how many chips are drawn is a property of the Set in
/// the slot, which changes while nobody presses anything, where `Scope::ALL`'s
/// length is a fact about this console. The row is asked as a whole —
/// [`crate::view::InspectorPane::select_renderer`] walks only the groups the
/// pane drew, only the rows a press is a choice on, and only until the chip
/// under the pointer.
pub(super) fn on_rend(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.select_renderer(ctx, pane, p).is_some())
    })
}

/// A parameter row's publish mark, which is the row's leftmost cell — the
/// number where the control is on the interface and a dot where it is not.
///
/// The count is one for the fader's reason and not the authority chips': how
/// many rows a pane draws is a property of the Set in the slot, where the three
/// authority levels are a closed list this console owns.
///
/// It is the same cell in both states, so this is one control and not two: what
/// the press asks for differs — off the list or back onto the end of it — and
/// the mark a hand lands on does not (`docs/adr/0329-…`).
pub(super) fn on_publish(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    let _ = ctx;
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.publishing(pane, p).is_some())
    })
}

/// A node group's `uses` capsule, and the card a press on it brings down — two
/// controls and one row, which is the Library bay's `load` button and deck
/// pulldown's arrangement and its reason: they are one derivation and one ask.
///
/// The capsule emits nothing, which is what makes it a control here and no row
/// on the operations page: it opens a list, and *open the list* is not
/// something a map or a model could want to say (ADR-0305). The card's rows
/// emit `WireInput`.
///
/// The card is asked first, because it is drawn over the pane the capsule sits
/// in: a press inside it belongs to the card. That ordering is rule 2's job at
/// the top of [`claim`] and this is the same fact inside one bay.
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

/// The `man / sug / auto` chips on a node head, and the count is three for the
/// tracker group's reason and not the renderer row's: the three levels are a
/// closed list this console owns ([`crate::view::AUTHORITIES`]), where how many
/// renderer chips are drawn is a property of the Set in the slot. How many
/// *heads* draw them is data and is not counted, exactly as how many renderer
/// rows there are is not.
///
/// They were drawn and claimed by nothing from 2026-08-29 until the engine had
/// a writer for them (ADR-0319).
pub(super) fn on_auth(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.set_authority(ctx, pane, p).is_some())
    })
}

/// The sensitivity row's chips under a bound parameter, and the count is two
/// because two of the four are controls: the curve chip and `take back`. The
/// source and the range are drawn and claimed by nothing —
/// [`crate::view::SensChip`] carries why — which is this file's own rule that a
/// control claims what it acts on and no more, and it is why this number is not
/// four.
pub(super) fn on_sens(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.sensitivity(ctx, pane, p).is_some())
    })
}

/// A parameter row's fader, one pane at a time, and the last control in an
/// Inspector pane. It asks `egui` for nothing: every box in a pane is the full
/// width of the pane or a track of the mock's own grid, so this is the cheapest
/// probe in a pane. A console with no deck behind it has no panes and pays
/// nothing.
pub(super) fn on_param(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.owns(pane, p))
    })
}

/// The grip in a bay head, one derivation asked once per bay — the class pills'
/// arrangement rather than a strip's: they are in four different heads and
/// cannot be one laid-out box, but they are one type and one question. It asks
/// `egui` for nothing — a grip is six dots at a fixed width — so this is the
/// cheapest probe here beside the preview cells, and the only control in a bay
/// head that exists before the first frame.
///
/// A pane needs no row here. A pane folds by its own boundary being pulled past
/// the narrowest it goes, which rule 3 above claims before rule 4 is reached at
/// all
/// ([ADR-0300](../../../docs/adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md)).
pub(super) fn on_grip(panel: &Panel, _ctx: &egui::Context, _view: &View, p: Point) -> bool {
    REGIONS
        .iter()
        .any(|region| bay_grip(panel.layout(), region.name).is_some_and(|grip| grip.hit(p)))
}

/// The one control this console has in a bay head, and the only one of the
/// twenty-three whose capsule a boundary's grab reaches — `view::program_head`
/// is where that 0.75 of a pixel is measured and argued, and rule 3 above is
/// what decides it.
pub(super) fn on_solo(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    program_head(ctx, panel.layout(), view.opening).is_some_and(|head| head.hit(p))
}

/// The four deck preview cells, derived once for all of them, which is a
/// strip's arrangement one bay over: the cells are four questions about one
/// arranged Program bay, and a second derivation would put the cells a press
/// lands on in the other of the two arrangements. It asks for no type at all —
/// a cell is a rectangle and a letter — so this is the cheapest probe here.
pub(super) fn on_cells(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    program_bay(panel.layout(), view.canvas).is_some_and(|bay| bay.owns(p))
}

/// The Library bay's scope chips, and it is the first control this console has
/// whose *count* is a value rather than a constant: the row is as long as the
/// slice the host handed the bay. One derivation for all of them, which is a
/// strip's arrangement — the chips are laid end to end from the row's left
/// padding, so where the fourth is depends on how wide the first three words
/// are, and a second walk would put the capsule a press lands on somewhere the
/// wash is not.
///
/// The row is asked before any chip is, inside `LibraryBay::chip`: `.scopes`
/// clips, so at the mock's own width the fourth chip finishes outside the bay,
/// and the part of it that is not drawn is not a target. The part that is
/// inside the pane divider's grab is the boundary's under rule 3 above, which
/// is this rule's ordinary price and is measured in `tests/library.rs`.
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

/// The Library bay's two filter fields, one row under the chips, and the bay is
/// derived again rather than shared with the probe above it — `any`
/// short-circuits, so the second derivation is only ever paid by a press that
/// got past the chips, and a bay held across two probes would be a value living
/// longer than the question it answers.
///
/// It asks `egui` for nothing, where the chips ask it for a word width apiece:
/// `.field` is `flex: 1`, so where the two fields are is a division of the row
/// rather than a measurement of what is in them — `LibraryBay::field`. So this
/// is the cheapest probe in rule 4 after the preview cells, and it is asked
/// here rather than earlier because the row it is in is drawn only where the
/// chips above it are.
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

/// The Library bay's six kind chips, one row under the field, and the bay is
/// derived again for [`on_filter`]'s reason.
///
/// It asks `egui` for a word width apiece, which is the scope chips' cost two
/// rows up and the same argument: a chip is as wide as the word in it, and the
/// only thing that knows how wide a word is is the thing that will paint it.
/// `LibraryBay::kind` is the derivation, asked for the whole row rather than
/// chip by chip, so a press that lands between two of them is on the row's own
/// ground and belongs to nobody.
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

/// A row's badges, which say what that row implements.
///
/// A readout and still a row here, which is the `map` pill's own sentence in
/// this table: nothing is pressed, a press on one lands on the panel and takes
/// the row in hand exactly as a press on the name does, and what this row buys
/// is the tooltip — the words that say what a badge means and that the control
/// which narrows by kind is the chip row above (ADR-0338).
///
/// Its count is one for the stars' reason: a pointer is over the badges of one
/// row of however many the bay drew, and how many that is moves when a divider
/// moves.
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

/// The `params` chip in the Library bay's foot, and it is the one control in
/// this bay that is not in its head: the chips say which library and the fields
/// narrow it, where this reads the row the cursor is on. The bay is derived a
/// third time for the second probe's reason — `any` short-circuits, so a press
/// that got this far has already been turned down by the two rows above.
///
/// The Set under the cursor goes in with the point, because what a press asks
/// for names it: `Operation::ReadSet` carries an id, and this crate reads no
/// store (ADR-0156), so the operand is the row the host handed in. A listing
/// with nothing in it has no Set there, and the chip then claims nothing —
/// `LibraryBay::read`.
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

/// The star at the left of each row of the Library bay's list, and it is asked
/// before the row it is in: a star is inside a row, so a press on one would
/// otherwise be answered by the probe below and the mark would be dead. Rule
/// 4's *a control claims what it acts on and no more* is what puts the smaller
/// box first — the same order the deck head's chips are asked in before the
/// head's own ground.
///
/// The listing and the marks go in with the point, exactly as the listing does
/// for the row below: what a star means is a name this crate reads no store for
/// and a set of ids the host handed over (ADR-0156, `View::starred`), and a row
/// with no Set behind it is not a target — `LibraryBay::starred`.
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

/// The rows of the Library bay's list, and they are the first thing on this
/// console a press *takes hold of* rather than acts on: a press on a row picks
/// that Set up, and what it asks for is decided where it is let go —
/// `LibraryBay::take`, and `crate::panel::Panel::carry`. The bay is derived a
/// fourth time for the third probe's reason, and this one is asked after the
/// three above because the head and the foot are drawn over the ends of the
/// same bay and a row is what is between them.
///
/// The listing goes in with the point, exactly as it does for the `params` chip
/// above and for the same reason: what a row means is a name this crate reads
/// no store for (ADR-0156), and a row with no Set behind it is not a target. A
/// press on the list's own ground below the last row is nobody's, which is rule
/// 4's *a control claims what it acts on and no more*.
///
/// One rectangle and two things a press on it can mean, which is why two calls
/// are one row of [`PROBES`]: under [`crate::view::Scope::History`] a row is a
/// version rather than a Set, and a press on it lands that version on a node
/// (`LibraryBay::land`) instead of taking a Set in hand. The two are told apart
/// by which listing is handed in — `View::sets` is empty under that scope and
/// `View::versions` is empty under every other — so at most one of them can
/// answer and neither is told what the scope is.
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

/// The four class pills, one derivation asked four times, which is a deck
/// head's arrangement rather than a strip's: they are in four different regions
/// and cannot be one laid-out box, but they are one type and one question —
/// `view::mcp_pill`, asked whether the point is on the capsule that region
/// draws.
///
/// Asked last because it is the dearest probe here. Each class lays out its
/// bay's whole head to find one capsule in it, and the Outputs one lays out the
/// row's word and its sink's name as well; `any` short-circuits, so the common
/// press — which is on nothing — still pays it only after every cheaper answer
/// has said no.
pub(super) fn on_mcp(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    Class::ALL.iter().any(|class| {
        mcp_pill(ctx, panel.layout(), *class, view.opening).is_some_and(|pill| pill.hit(p))
    })
}
