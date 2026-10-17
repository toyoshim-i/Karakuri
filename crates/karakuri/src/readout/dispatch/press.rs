use karakuri_console::egui;
use karakuri_layout::Point;

use super::*;
use crate::refusal;

impl Readout {
    /// Dispatches pointer presses to open modal cards and pulldown overlay menus.
    ///
    /// Per Rule 2 (ADR-0311), while a modal overlay is open, every press on the console
    /// belongs to it, and a press outside its bounds dismisses the overlay.
    pub(crate) fn dispatch_modal_press(&mut self, ctx: &egui::Context, at: Point) -> Option<Acted> {
        // Library bay context menu
        if self.view.menu_open() {
            let picked = library_bay(
                self.panel.layout(),
                &self.view.scopes,
                &self.view.library,
                self.view.opened(),
                self.view.pointed(),
                self.view.library_scroll(),
            )
            .and_then(|bay| {
                bay.menu_ask(
                    ctx,
                    view::to_egui(self.panel.layout().viewport()),
                    self.view.menued(),
                    self.view.rows(),
                    at,
                )
            });
            return Some(self.menued(picked.unwrap_or(Picked::Shut)));
        }

        let tl = self.view.transport_layout(ctx, self.panel.layout());

        // Audio-in device selection pill and card
        let heard = tl
            .as_ref()
            .and_then(|tl| tl.audio.as_ref())
            .zip(self.view.audio.as_ref())
            .and_then(|(pill, audio)| pill.ask(audio, at));
        if self.view.audio.as_ref().is_some_and(AudioIn::open) {
            return Some(self.listened(heard.unwrap_or(AudioAsk::Shut)));
        }
        if let Some(ask) = heard {
            return Some(self.listened(ask));
        }

        // Arrangement preset pill and card
        let asked = tl
            .as_ref()
            .and_then(|tl| tl.arrangement.as_ref())
            .and_then(|pill| pill.ask(&self.view.arrangement, at));
        if self.view.arrangement.open() {
            return Some(self.arranged(asked.unwrap_or(Ask::Shut)));
        }
        if let Some(ask) = asked {
            return Some(self.arranged(ask));
        }

        // Theme selector pill and card
        let asked = tl
            .as_ref()
            .and_then(|tl| tl.theme.as_ref())
            .and_then(|pill| pill.ask(self.view.theme_menu_open, at));
        if self.view.theme_menu_open {
            return Some(self.themed(ctx, asked.unwrap_or(view::ThemeAsk::Shut)));
        }
        if let Some(ask) = asked {
            return Some(self.themed(ctx, ask));
        }

        // MCP server configuration pill and card
        let asked = tl
            .as_ref()
            .and_then(|tl| tl.mcp.as_ref())
            .and_then(|pill| pill.ask(&self.view.mcp_server, at));
        if self.view.mcp_server.open() {
            return Some(self.mcp_configured(asked.unwrap_or(view::McpAsk::Shut)));
        }
        if let Some(ask) = asked {
            return Some(self.mcp_configured(ask));
        }

        // Inspector wiring / uses card
        let room = view::to_egui(self.panel.layout().viewport());
        let picked = self.view.wiring_open().and_then(|(pane_at, node, input)| {
            let pane = self.view.inspector.get(pane_at)?;
            let laid = inspector_pane(
                self.panel.layout(),
                pane_at,
                pane,
                self.view.scroll_in(pane_at),
            )?;
            laid.wired(ctx, pane, room, (node, input), at)
                .map(Wiring::Pick)
        });
        let wiring = picked.or_else(|| {
            self.view
                .inspector
                .iter()
                .enumerate()
                .find_map(|(index, pane)| {
                    let laid = inspector_pane(
                        self.panel.layout(),
                        index,
                        pane,
                        self.view.scroll_in(index),
                    )?;
                    let (node, input) = laid.uses_chip(ctx, pane, at)?;
                    Some(Wiring::Chip {
                        pane: index,
                        node,
                        input,
                    })
                })
        });
        if self.view.wiring_open().is_some() {
            return Some(self.wired(wiring.unwrap_or(Wiring::Shut)));
        }
        if let Some(ask) = wiring {
            return Some(self.wired(ask));
        }

        // Pane head deck selection pulldown
        let picked = self.view.pane_target_open().and_then(|pane_at| {
            let pane = self.view.inspector.get(pane_at)?;
            let laid = inspector_pane(
                self.panel.layout(),
                pane_at,
                pane,
                self.view.scroll_in(pane_at),
            )?;
            self.view
                .pane_pulldown(ctx, &laid, pane, pane_at)?
                .picked(room, at)
                .map(view::Pointing::Pick)
        });
        let pointing = picked.or_else(|| {
            self.view
                .inspector
                .iter()
                .enumerate()
                .find_map(|(index, pane)| {
                    let laid = inspector_pane(
                        self.panel.layout(),
                        index,
                        pane,
                        self.view.scroll_in(index),
                    )?;
                    let target = self.view.pane_pulldown(ctx, &laid, pane, index)?;
                    target.hit(at).then_some(view::Pointing::Mark(index))
                })
        });
        if self.view.pane_target_open().is_some() {
            return Some(self.pointing(pointing.unwrap_or(view::Pointing::Shut)));
        }
        if let Some(ask) = pointing {
            return Some(self.pointing(ask));
        }

        // Library bay target deck pulldown / load control
        let aimed = library_bay(
            self.panel.layout(),
            &self.view.scopes,
            &self.view.library,
            self.view.opened(),
            self.view.pointed(),
            self.view.library_scroll(),
        )
        .and_then(|bay| {
            bay.aim(
                ctx,
                view::to_egui(self.panel.layout().viewport()),
                self.view.target(),
                self.view.rows(),
                self.view.cursor_row(),
                at,
            )
        });
        if self.view.target_open() {
            return Some(self.aimed(aimed.unwrap_or(Aim::Shut)));
        }
        if let Some(ask) = aimed {
            return Some(self.aimed(ask));
        }

        // Prompt bay CLI selection dropdown menu and header pill
        if let Some(id) = self.panel.layout().find("prompt") {
            let bay_rect = view::to_egui(self.panel.layout().rect(id));
            let viewport = view::to_egui(self.panel.layout().viewport());
            let asked = view::prompt_ask(ctx, bay_rect, viewport, &self.view.prompt, at);
            if self.view.prompt.menu_open {
                return Some(self.prompted(asked.unwrap_or(view::PromptAsk::Shut)));
            }
            if let Some(ask) = asked {
                return Some(self.prompted(ask));
            }
        }

        // Program bay resolution dropdown menu and header pill
        if let Some(id) = self.panel.layout().find("program") {
            let bay_rect = view::to_egui(self.panel.layout().rect(id));
            let viewport = view::to_egui(self.panel.layout().viewport());
            if let Some(pill) = view::resolution_pill(
                ctx,
                bay_rect,
                viewport,
                &self.view.output_resolutions,
                self.view.output_resolution_selected,
                self.view.resolution_menu_open,
            ) {
                let asked = pill.ask(
                    self.view.resolution_menu_open,
                    self.view.output_resolutions.len(),
                    at,
                );
                if self.view.resolution_menu_open {
                    return Some(self.resolution_asked(asked.unwrap_or(view::ResolutionAsk::Shut)));
                }
                if let Some(ask) = asked {
                    return Some(self.resolution_asked(ask));
                }
            }
        }

        // Outputs WebXR mode dropdown menu and pill
        if let Some(row) = self.view.outputs_layout(ctx, self.panel.layout()) {
            if let Some(ref pill) = row.xr_pill {
                let asked = pill.ask(self.view.xr_menu_open, at);
                if self.view.xr_menu_open {
                    return Some(self.xr_mode_asked(asked.unwrap_or(view::XrAsk::Shut)));
                }
                if let Some(ask) = asked {
                    return Some(self.xr_mode_asked(ask));
                }
            }
        }

        None
    }

    /// Dispatches pointer presses in the Transport bay.
    pub(crate) fn dispatch_transport_press(
        &mut self,
        ctx: &egui::Context,
        at: Point,
    ) -> Option<Acted> {
        let tl = self.view.transport_layout(ctx, self.panel.layout());
        let tracked = tl
            .as_ref()
            .and_then(|tl| tl.tracker.as_ref())
            .and_then(|group| {
                group
                    .tapped(at)
                    .or_else(|| group.octave(at))
                    .or_else(|| group.nudge(at))
            });
        if let Some(operation) = tracked {
            return Some(Acted::Emitted(Some(operation)));
        }

        let look = tl.as_ref().and_then(|tl| tl.look.as_ref());
        let tone = look.and_then(|row| row.tonemap(at));
        let exposure = look.and_then(|row| row.exposure(at));
        if let Some(operation) = tone.or(exposure) {
            return Some(Acted::Emitted(Some(operation)));
        }

        if let Some(pill) = tl.as_ref().and_then(|tl| tl.learn.as_ref()) {
            if pill.hit(at) {
                self.view.learn = pill.next();
                println!(
                    "{}",
                    match self.view.learn {
                        true =>
                            "learn: armed. point at a control and move a knob or hit a pad, and the two are bound — the line goes in your own map file. nothing is played from the surface while this is lit, and it stays lit until you press it again.",
                        false => "learn: off. the surface plays again.",
                    }
                );
                return Some(Acted::Opened);
            }
        }

        if tl
            .as_ref()
            .and_then(|tl| tl.map.as_ref())
            .is_some_and(|row| row.pill.contains(egui::Pos2::new(at.x, at.y)))
        {
            return Some(Acted::MidiRequest);
        }

        let recording = tl.as_ref().and_then(|tl| tl.row.record(at));
        if let Some(operation) = recording {
            return Some(Acted::Emitted(Some(operation)));
        }

        let tempo = tl.as_ref().and_then(|tl| tl.row.tempo(at));
        if let Some(operation) = tempo {
            return Some(Acted::Emitted(Some(operation)));
        }

        None
    }

    /// Dispatches pointer presses in Inspector bay panes.
    pub(crate) fn dispatch_inspector_press(
        &mut self,
        ctx: &egui::Context,
        at: Point,
    ) -> Option<Acted> {
        let deck_head = self
            .view
            .inspector
            .iter()
            .enumerate()
            .find_map(|(index, pane)| {
                let at_pane =
                    inspector_pane(self.panel.layout(), index, pane, self.view.scroll_in(index))?;
                let head = deck_head_row(ctx, &at_pane, pane)?;
                head.sync(at)
                    .or_else(|| head.reanchor(at))
                    .or_else(|| head.scrub(at))
                    .or_else(|| head.resized(at))
                    .or_else(|| head.re_salted(at))
                    .or_else(|| head.compositing(at))
            });
        if let Some(operation) = deck_head {
            return Some(Acted::Emitted(Some(operation)));
        }

        let naming = self
            .view
            .inspector
            .iter()
            .enumerate()
            .find_map(|(index, pane)| {
                let at_pane =
                    inspector_pane(self.panel.layout(), index, pane, self.view.scroll_in(index))?;
                let policy = self
                    .view
                    .slot_policies
                    .get(pane.deck)
                    .copied()
                    .unwrap_or_default();
                let mcp = slot_mcp_pill(ctx, &at_pane, pane, policy).map(|p| p.pill);
                let named = deck_name(ctx, &at_pane, pane, self.view.naming_set_in(index), mcp)?;
                named.hit(at).then_some(index)
            });
        if let Some(index) = naming {
            println!(
                "inspector: type a name and press return — letters, digits, `-` and `_`, and escape keeps nothing"
            );
            self.view.name_set(index);
            return Some(Acted::Nothing);
        }

        let keep = self
            .view
            .inspector
            .iter()
            .enumerate()
            .find_map(|(index, pane)| {
                let at_pane =
                    inspector_pane(self.panel.layout(), index, pane, self.view.scroll_in(index))?;
                let pill = keep_pill(ctx, &at_pane, pane)?;
                pill.keep(at)
            });
        if let Some(operation) = keep {
            return Some(Acted::Emitted(Some(operation)));
        }

        let slot_mcp = self
            .view
            .inspector
            .iter()
            .enumerate()
            .find_map(|(index, pane)| {
                let at_pane =
                    inspector_pane(self.panel.layout(), index, pane, self.view.scroll_in(index))?;
                let policy = self
                    .view
                    .slot_policies
                    .get(pane.deck)
                    .copied()
                    .unwrap_or_default();
                let pill = slot_mcp_pill(ctx, &at_pane, pane, policy)?;
                pill.hit(at).then_some(pane.deck)
            });
        if let Some(deck) = slot_mcp {
            return Some(self.cycle_slot_policy(deck));
        }

        let chosen = self
            .view
            .inspector
            .iter()
            .enumerate()
            .find_map(|(index, pane)| {
                let at_pane =
                    inspector_pane(self.panel.layout(), index, pane, self.view.scroll_in(index))?;
                at_pane.select_renderer(ctx, pane, at)
            });
        if let Some(operation) = chosen {
            return Some(Acted::Emitted(Some(operation)));
        }

        let spoken = self
            .view
            .inspector
            .iter()
            .enumerate()
            .find_map(|(index, pane)| {
                let at_pane =
                    inspector_pane(self.panel.layout(), index, pane, self.view.scroll_in(index))?;
                at_pane.set_authority(ctx, pane, at)
            });
        if let Some(operation) = spoken {
            return Some(Acted::Emitted(Some(operation)));
        }

        let kept = self
            .view
            .inspector
            .iter()
            .enumerate()
            .find_map(|(index, pane)| {
                let at_pane =
                    inspector_pane(self.panel.layout(), index, pane, self.view.scroll_in(index))?;
                at_pane.keep_procedure(ctx, pane, at)
            });
        if let Some(operation) = kept {
            return Some(Acted::Emitted(Some(operation)));
        }

        let sensed = self
            .view
            .inspector
            .iter()
            .enumerate()
            .find_map(|(index, pane)| {
                let at_pane =
                    inspector_pane(self.panel.layout(), index, pane, self.view.scroll_in(index))?;
                at_pane.sensitivity(ctx, pane, at)
            });
        if let Some(operation) = sensed {
            return Some(Acted::Emitted(Some(operation)));
        }

        let published = self
            .view
            .inspector
            .iter()
            .enumerate()
            .find_map(|(index, pane)| {
                let at_pane =
                    inspector_pane(self.panel.layout(), index, pane, self.view.scroll_in(index))?;
                at_pane.publishing(pane, at)
            });
        if let Some(operation) = published {
            return Some(Acted::Emitted(Some(operation)));
        }

        let folded = self
            .view
            .inspector
            .iter()
            .enumerate()
            .find_map(|(pane_idx, pane)| {
                let at_pane = inspector_pane(
                    self.panel.layout(),
                    pane_idx,
                    pane,
                    self.view.scroll_in(pane_idx),
                )?;
                let node_idx = at_pane.fold_node(ctx, pane, at)?;
                Some((pane_idx, node_idx))
            });
        if let Some((pane_idx, node_idx)) = folded {
            self.view.toggle_node_fold(pane_idx, node_idx);
            return Some(Acted::Pointed);
        }

        None
    }

    /// Dispatches pointer presses to bay grips, Program head solo, and MCP class pills.
    pub(crate) fn dispatch_head_press(&mut self, ctx: &egui::Context, at: Point) -> Option<Acted> {
        if let Some(head) =
            program_head(ctx, self.panel.layout(), self.view.opening).filter(|head| head.hit(at))
        {
            return Some(Acted::Operated(self.soloed(head.op())));
        }

        // Bay grip clicks are reserved for the upcoming Bay Context Menu (ADR-0364).
        if REGIONS.iter().any(|region| {
            bay_grip(self.panel.layout(), region.name).is_some_and(|grip| grip.hit(at))
        }) {
            return Some(Acted::Nothing);
        }

        if let Some(pill) = Class::ALL.iter().find_map(|class| {
            let mut pill = mcp_pill(ctx, self.panel.layout(), *class, self.view.opening)?;
            if *class == Class::InputsAndOutputs {
                if let Some(out) = self.view.outputs_layout(ctx, self.panel.layout()) {
                    pill.pill = out.mcp;
                }
            }
            pill.hit(at).then_some(pill)
        }) {
            return Some(self.opened(&pill));
        }

        if let Some(pill) = library_mcp_pill(ctx, self.panel.layout(), self.view.opening) {
            if pill.hit(at) {
                return Some(self.cycle_library_gate());
            }
        }

        None
    }

    /// Dispatches pointer presses in the Library bay.
    pub(crate) fn dispatch_library_press(
        &mut self,
        ctx: &egui::Context,
        at: Point,
    ) -> Option<Acted> {
        if let Some(chosen) = library_bay(
            self.panel.layout(),
            &self.view.scopes,
            &self.view.library,
            self.view.opened(),
            self.view.pointed(),
            self.view.library_scroll(),
        )
        .and_then(|bay| bay.chip(ctx, &self.view.scopes, at))
        {
            return Some(self.chose(chosen));
        }

        if let Some(operation) = library_bay(
            self.panel.layout(),
            &self.view.scopes,
            &self.view.library,
            self.view.opened(),
            self.view.pointed(),
            self.view.library_scroll(),
        )
        .and_then(|bay| bay.filter(&self.view.holds, self.view.filters(), at))
        {
            return Some(self.narrowed(operation));
        }

        if let Some(operation) = library_bay(
            self.panel.layout(),
            &self.view.scopes,
            &self.view.library,
            self.view.opened(),
            self.view.pointed(),
            self.view.library_scroll(),
        )
        .and_then(|bay| bay.kind(ctx, self.view.filters(), at))
        {
            return Some(self.narrowed(operation));
        }

        if let Some(ask) = library_bay(
            self.panel.layout(),
            &self.view.scopes,
            &self.view.library,
            self.view.opened(),
            self.view.pointed(),
            self.view.library_scroll(),
        )
        .and_then(|bay| {
            bay.read(
                ctx,
                self.view.target(),
                self.view
                    .sets()
                    .get(self.view.cursor_row())
                    .map(String::as_str),
                at,
            )
        }) {
            return Some(self.asked_to_read(ask));
        }

        if let Some(operation) = library_bay(
            self.panel.layout(),
            &self.view.scopes,
            &self.view.library,
            self.view.opened(),
            self.view.pointed(),
            self.view.library_scroll(),
        )
        .and_then(|bay| bay.starred(self.view.rows(), &self.view.starred, at))
        {
            return Some(Acted::Emitted(Some(operation)));
        }

        if let Some(taken) = library_bay(
            self.panel.layout(),
            &self.view.scopes,
            &self.view.library,
            self.view.opened(),
            self.view.pointed(),
            self.view.library_scroll(),
        )
        .and_then(|bay| bay.take(self.view.rows(), at))
        {
            return Some(self.took(at, taken));
        }

        if let Some(operation) = library_bay(
            self.panel.layout(),
            &self.view.scopes,
            &self.view.library,
            self.view.opened(),
            self.view.pointed(),
            self.view.library_scroll(),
        )
        .and_then(|bay| bay.land(self.view.versions(), self.view.target(), at))
        {
            return Some(self.landed(operation));
        }

        None
    }

    /// Dispatches pointer presses in the Staging bay.
    pub(crate) fn dispatch_staging_press(
        &mut self,
        ctx: &egui::Context,
        at: Point,
    ) -> Option<Acted> {
        if let Some(operation) = staging_bay(self.panel.layout(), &self.view.staging)
            .and_then(|bay| bay.back(ctx, &self.view.staging, at))
        {
            return Some(self.landed(operation));
        }

        if let Some(operation) = staging_bay(self.panel.layout(), &self.view.staging)
            .and_then(|bay| bay.keep(ctx, &self.view.staging, at))
        {
            return Some(self.kept(operation));
        }

        None
    }

    /// Dispatches pointer presses in the Transition controls row.
    pub(crate) fn dispatch_transition_press(
        &mut self,
        ctx: &egui::Context,
        at: Point,
    ) -> Option<Acted> {
        let row = transition_row(ctx, self.panel.layout(), self.view.transition());
        if let Some(row) = row.as_ref() {
            if let Some(operation) = row
                .shape(at)
                .or_else(|| row.quantum(at))
                .or_else(|| row.length(at))
            {
                return Some(Acted::Emitted(Some(operation)));
            }

            match row.go(at, self.view.selection(), self.view.mixer.len()) {
                Some(Go::Wipe(operation)) => return Some(Acted::Emitted(Some(operation))),
                Some(refused) => {
                    println!("{}", refusal(&refused, self.view.mixer.len()));
                    return Some(Acted::Nothing);
                }
                None => {}
            }
        }

        None
    }

    /// Dispatches pointer presses in the Sequencer bay.
    pub(crate) fn dispatch_sequencer_press(
        &mut self,
        ctx: &egui::Context,
        at: Point,
    ) -> Option<Acted> {
        let choices = self.view.lane_choices();
        let seq = sequencer_bay(
            ctx,
            self.panel.layout(),
            self.view.sequencer.as_ref(),
            &choices,
        );

        if self.view.lane_open() {
            let did = self.chosen(
                seq.as_ref()
                    .and_then(|bay| bay.chose(at, &choices))
                    .unwrap_or(Chose::Shut),
            );
            return Some(did);
        }
        if let Some(operation) = seq.as_ref().and_then(|bay| bay.press(at)) {
            return Some(Acted::Emitted(Some(operation)));
        }

        if let Some(chose) = seq.as_ref().and_then(|bay| bay.chose(at, &choices)) {
            let did = self.chosen(chose);
            return Some(did);
        }

        None
    }

    /// Dispatches pointer presses in the Mixer and Master bays and parameter knob grabs.
    pub(crate) fn dispatch_mixer_and_master_press(
        &mut self,
        ctx: &egui::Context,
        at: Point,
    ) -> Acted {
        let outputs_row = self.view.outputs_layout(ctx, self.panel.layout());
        if let Some(ref row) = outputs_row {
            if let Some(action) = row.action_at(at) {
                return match action {
                    view::OutputAction::Share => Acted::ShareRequest,
                    view::OutputAction::Logs => Acted::LogToggleRequest,
                };
            }
        }
        let sink = outputs_row.and_then(|row| row.chip_at(at).map(|output| (row, output)));
        let bay = mixer_bay(ctx, self.panel.layout(), &self.view.mixer);
        let adding = self.view.chain_choices();
        let master = self.view.master_row_layout(ctx, self.panel.layout());
        let knob = bay
            .as_ref()
            .and_then(|bay| bay.grab(at))
            .or_else(|| master.as_ref().and_then(|row| row.grab(at)))
            .or_else(|| {
                self.view
                    .inspector
                    .iter()
                    .enumerate()
                    .find_map(|(index, pane)| {
                        let at_pane = inspector_pane(
                            self.panel.layout(),
                            index,
                            pane,
                            self.view.scroll_in(index),
                        )?;
                        at_pane.grab(pane, at)
                    })
            });
        let chip = bay
            .as_ref()
            .and_then(|bay| bay.blend(at))
            .or_else(|| master.as_ref().and_then(|row| row.chip(at)));
        let tally = bay.as_ref().and_then(|bay| {
            bay.solo(at)
                .or_else(|| bay.mute(at))
                .or_else(|| bay.tally(at))
        });
        let mask = bay.as_ref().and_then(|bay| bay.mask(at));
        let chose = master.as_ref().and_then(|row| row.chose(at, &adding));
        if let Some(chose) = chose {
            return self.chain_chose(chose);
        }
        if let Some(slot) = master.as_ref().and_then(|row| row.solo(at)) {
            self.view.toggle_master_solo(slot);
            return Acted::Pointed;
        }
        if let Some(slot) = master.as_ref().and_then(|row| row.mute(at)) {
            self.view.toggle_master_mute(slot);
            return Acted::Pointed;
        }
        if let Some(slot) = master.as_ref().and_then(|row| row.fold(at)) {
            self.view.toggle_master_fold(slot);
            return Acted::Pointed;
        }
        if let Some(ask) = master.as_ref().and_then(|row| row.vr_ask(at)) {
            match ask {
                view::VrAsk::CycleMode => {
                    self.view.vr_projection.mode = self.view.vr_projection.mode.next();
                    return Acted::Pointed;
                }
                view::VrAsk::ToggleFold => {
                    self.view.toggle_vr_projection_fold();
                    return Acted::Pointed;
                }
                view::VrAsk::SetParam { key, value } => {
                    self.view.set_vr_param(key.label(), value);
                    if let Some(vr) = master.as_ref().and_then(|r| r.vr_stage.as_ref()) {
                        if let Some(param) = vr.params.iter().find(|p| p.key == key) {
                            self.vr_drag = Some(crate::readout::VrDrag {
                                key,
                                track_min_x: param.fader.track.min.x,
                                track_width: param.fader.track.width(),
                                range: param.range,
                            });
                        }
                    }
                    return Acted::Pointed;
                }
            }
        }
        match (sink, knob, chip, tally, mask) {
            (Some((row, Output::Program)), ..) => Acted::Operated(self.sink(row.route(), row.op())),
            (Some((row, output)), ..) => Acted::Emitted(
                row.more
                    .iter()
                    .find(|chip| chip.output == output)
                    .and_then(view::SinkChip::route),
            ),
            (None, Some(grab), ..) => {
                println!(
                    "press ({:.0}, {:.0}): {} — the {} is in hand",
                    at.x,
                    at.y,
                    knob_where(&grab.knob()),
                    knob_word(&grab.knob())
                );
                self.panel.grab(at, grab);
                Acted::Nothing
            }
            (None, None, Some(operation), ..)
            | (None, None, None, Some(operation), _)
            | (None, None, None, None, Some(operation)) => Acted::Emitted(Some(operation)),
            (None, None, None, None, None) => match bay.as_ref().and_then(|bay| bay.select(at)) {
                Some(operation) => Acted::Emitted(Some(operation)),
                None => {
                    self.press(at);
                    Acted::Nothing
                }
            },
        }
    }

    /// Dispatches pointer presses in the Program bay (e.g. un-gating overloaded decks).
    pub(crate) fn dispatch_program_press(
        &mut self,
        _ctx: &egui::Context,
        at: Point,
    ) -> Option<Acted> {
        let bay = program_bay(self.panel.layout(), self.view.canvas)?;
        let deck = usize::from(bay.cell(at)?);
        if self.view.overloaded.get(deck).copied().unwrap_or(false) {
            self.view.ungate_requested = Some(deck);
            return Some(Acted::Pointed);
        }
        None
    }
}
