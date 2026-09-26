use super::*;
use egui::Ui;

impl View {
    /// Draw the whole console. The `ui` is the root one [`egui::Context::run_ui`]
    /// hands the frame's closure.
    pub fn draw(&mut self, ui: &mut Ui, panel: &mut Panel) {
        self.mixer_dirty = false;
        let pal = self.room.palette();
        // The bay arranges itself before any other layout reads this frame.
        plan_into(panel, self.canvas, &mut self.placed);
        self.cursor(ui.ctx(), panel);

        // Resolve Program bay and preview cell positions once from the layout.
        let program = program_bay(panel.layout(), self.canvas);
        let cells = program.and_then(|bay| bay.cells);
        // Resolve carry drop landing cell, if a drag is in progress (ADR-0273).
        let carried = matches!(panel.in_hand(), Some(InHand::Carrying)).then(|| panel.cursor());
        let marked_cell =
            carried.and_then(|at| program.and_then(|bay| bay.dropped(at, self.mixer.len())));
        let picture = self.picture;
        let previews = self.previews;
        // Deck overload state, read once for consistent caption rendering.
        let overloaded = self.overloaded;
        // Frame computation costs per deck cell.
        let costs = self.costs;
        let values = self.transport;
        let arr = &self.arrangement;
        // Audio input settings for pill layout.
        let audio = self.audio.as_ref();
        // Beat tracking state for tracker pill layout.
        let tracking = self.tracker;
        // MIDI mapping and learn mode states.
        let map = self.map.as_ref();
        let armed = self.learn;
        let look_at = self.look;
        let out = self.master_out;
        let chain = self.master_chain.as_ref();
        // Available chain additions for `+ add` card.
        let adding = self.chain_choices();
        let strips = self.mixer.as_slice();
        let sets = self.library.as_slice();
        // Item kind tags aligned with library rows (ADR-0338).
        let kinds = self.kinds.as_slice();
        // Starred items set snapshot.
        let starred = &self.starred;
        let scopes = self.scopes.as_slice();
        // Active filter criteria.
        let narrowed = self.filters();
        // Active library directory path.
        let pointed = self.pointed();
        // Active open library reading.
        let opened = self.opened();
        // Mixer selection and library cursor row snapshots.
        let selection = self.selection();
        let cursor_row = self.cursor_row();
        // Active focus indicator position.
        let focused = self.focus_mark(panel);
        // Folded bay focus mark indicator.
        let folded = self.folded_mark(panel);
        // Marked scope chip index.
        let scope = self.marked();
        // Active transition configuration snapshot.
        let transition_at = self.transition;
        // Target deck for load controls (ADR-0305).
        let load = self.target();
        // Active popup card coordinates and states, read once per frame.
        let wiring = self.wiring_open;
        let pane_open = self.pane_open;
        let menued = self.menued();
        // Library vertical scroll offset (P-0082).
        let scrolled_to = self.library_scroll();
        let waiting = self.staging.as_slice();
        // Sequencer state snapshot.
        let sequenced = self.sequencer.as_ref();
        // Sequencer lane addition choices snapshot.
        let choices = self.lane_choices();
        let panes = self.inspector.as_slice();
        // Inspector pane scroll offsets snapshot.
        let scrolled = self.scroll;
        // Deck name editing prompt state.
        let naming = self.naming.as_ref();
        let phase = self.phase;
        // Bay drawer opening state snapshot.
        let opening = self.opening;
        // Projector and plugin sink states.
        let projector = self.projector;
        let plugin = self.plugin;
        let plugin_available = self.plugin_available;
        let plugin_name = self.plugin_name;
        let frame = egui::Frame::NONE.fill(pal.ground);
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            for placed in &self.placed {
                let rect = to_egui(placed.rect);
                if panel.layout().is_collapsed(placed.id) {
                    let title = head_of(placed.region).map_or("", |head| head.title);
                    folded_head_into(ui, &pal, rect, title);
                    continue;
                }
                match placed.region.kind {
                    Kind::Bay { .. } => {
                        bay_card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        // The inspector is the one bay that is a split, and
                        // its panes' boundary is drawn as the mock's
                        // `.divider-v` rather than left as bare ground: it is
                        // inside a card, where the ground does not reach.
                        pane_dividers(ui, &pal, panel, placed.id, rect);
                    }
                    // Transport row readouts and controls.
                    Kind::Transport => {
                        bay_card(ui, &pal, rect);
                        if let Some(row) = transport(ui.ctx(), panel.layout(), values) {
                            transport::transport_into(ui, &pal, &row);
                        }
                        // Tracker controls and look parameters within the Transport bay.
                        if let Some(group) =
                            tracker_group(ui.ctx(), panel.layout(), values, audio, tracking)
                        {
                            transport::tracker_into(ui, &pal, &group);
                        }
                        if let Some(row) = look(
                            ui.ctx(),
                            panel.layout(),
                            values,
                            audio,
                            tracking,
                            None,
                            arr,
                            look_at,
                        ) {
                            transport::look_into(ui, &pal, &row);
                        }
                    }
                    // Outputs control bay.
                    Kind::Outputs => {
                        bay_card(ui, &pal, rect);
                        if let Some(row) = outputs_with_plugin_name(
                            ui.ctx(),
                            panel.layout(),
                            opening,
                            plugin_available,
                            plugin_name,
                        )
                        .map(|r| {
                            r.told(projector).told_plugin_name(
                                0,
                                plugin,
                                plugin_available,
                                plugin_name,
                            )
                        }) {
                            outputs::outputs_into(ui, &pal, &row);
                        }
                    }
                    // Mixer bay strips and volume controls.
                    Kind::Mixer => {
                        bay_card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(bay) = mixer(ui.ctx(), panel.layout(), strips) {
                            // Strip highlight for carry drop landing.
                            let marked = carried.and_then(|at| bay.dropped(at));
                            mixer::mixer_into(ui, &pal, &bay, phase, selection, marked);
                        }
                        // Transition row controls (cut/wipe/fade settings).
                        if let Some(row) = transition(ui.ctx(), panel.layout(), transition_at) {
                            mixer::transition_into(ui, &pal, &row, strips.len());
                        }
                    }
                    // Sequencer bay grid and patterns.
                    Kind::Sequencer => {
                        bay_card(ui, &pal, rect);
                        // Sequencer bank indicators.
                        match sequenced.map(|reading| reading.bank) {
                            Some(armed) => {
                                if let Some(head) = head_of(placed.region) {
                                    bay_head(ui, &pal, rect, &head.with_banks(armed), opening);
                                }
                            }
                            None => head_into(ui, &pal, rect, placed.region, opening),
                        }
                        if let Some(bay) = sequencer(ui.ctx(), panel.layout(), sequenced, &choices)
                        {
                            sequencer::sequencer_into(ui, &pal, &bay);
                        }
                    }
                    Kind::Master => {
                        bay_card(ui, &pal, rect);
                        match head_of(placed.region) {
                            Some(head) => {
                                let head = head.with_building(self.master_chain_building);
                                bay_head(ui, &pal, rect, &head, opening);
                            }
                            None => head_into(ui, &pal, rect, placed.region, opening),
                        }
                        if let Some(row) = master(ui.ctx(), panel.layout(), out, chain, &adding) {
                            master::master_into(ui, &pal, &row);
                            // Chain list drop landing indicator (ADR-0273).
                            if let Some(list) = carried.and_then(|at| row.dropped(at)) {
                                drop_ring(ui, &pal, list, f32::from(size::FX_RADIUS));
                            }
                        }
                    }
                    // Library bay listing, scopes, and search controls.
                    Kind::Library => {
                        bay_card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(bay) =
                            library(panel.layout(), scopes, sets, opened, pointed, scrolled_to)
                        {
                            // Scope selection chips and directory path row (ADR-0311).
                            library::scopes_into(ui, &pal, &bay, scopes, scope);
                            if let Some(at) = pointed {
                                library::path_into(ui, &pal, &bay, at);
                            }
                            // Filter and kind filter chips (ADR-0338).
                            library::filters_into(ui, &pal, &bay, narrowed);
                            library::kinds_into(ui, &pal, &bay, narrowed);
                            library::library_into(
                                ui,
                                &pal,
                                &bay,
                                library::Listed {
                                    rows: Rows { names: sets, kinds },
                                    starred,
                                },
                                cursor_row,
                                load,
                                opened,
                            );
                        }
                    }
                    // Staging lane candidates awaiting swap or judgment.
                    Kind::Staging => {
                        bay_card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(bay) = staging(panel.layout(), waiting) {
                            staging::staging_into(ui, &pal, &bay, waiting);
                        }
                    }
                    // Prompt bay terminal and agent CLI selection.
                    Kind::Prompt => {
                        bay_card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        prompt::prompt_head_into(ui, &pal, rect, &self.prompt);
                        prompt::prompt_into(ui, &pal, rect, &self.prompt);
                    }
                    // A pane draws nothing of its own. It has no card — it is
                    // inside the bay's — and no head, and its body is as empty
                    // as every other body in this pass.
                    Kind::Pane => {}
                    // Picture output texture from engine.
                    Kind::Picture => {
                        if let Some(picture) = picture {
                            ui.painter().with_clip_rect(rect).image(
                                picture.id,
                                picture.rect,
                                WHOLE_TEXTURE,
                                Color32::WHITE,
                            );
                        }
                    }
                    // Previews region placeholder; drawn post-pass to avoid clipping.
                    Kind::Previews => {}
                }
            }

            // Deck preview cells rendered over the Program bay.
            if let Some(cells) = cells {
                for (deck, cell) in cells.into_iter().enumerate() {
                    // Carry drop target ring and preview content.
                    let marked = marked_cell == Some(deck as u8);
                    if marked {
                        drop_ring(ui, &pal, cell, size::PREVIEW_RADIUS);
                    }
                    program::preview(ui, &pal, cell, previews[deck]);
                    program::caption_into(
                        ui,
                        &pal,
                        cell,
                        deck,
                        previews[deck],
                        overloaded[deck],
                        costs[deck],
                        marked,
                    );
                }
            }

            // Inspector panes rendered over the Inspector bay.
            for (index, pane) in panes.iter().enumerate().take(PANES) {
                if let Some(at) = inspector(panel.layout(), index, pane, scrolled[index]) {
                    let typed = naming
                        .filter(|naming| naming.pane == index)
                        .map(Naming::typed);
                    let policy = self
                        .slot_policies
                        .get(pane.deck)
                        .copied()
                        .unwrap_or_default();
                    let mcp = inspector::slot_mcp_pill(ui.ctx(), &at, pane, policy).map(|p| p.pill);
                    inspector::inspector_into(
                        ui,
                        &pal,
                        inspector::InspectorIntoCtx {
                            at: &at,
                            pane,
                            on_air: inspector::on_air(strips, pane.deck),
                            policy,
                            naming: typed,
                            // Pane target derivation matching input hit-test.
                            target: pane_target(
                                ui.ctx(),
                                inspector::PaneTargetCtx {
                                    at: &at,
                                    pane,
                                    index,
                                    naming: typed,
                                    decks: load.decks,
                                    open: pane_open == Some(index),
                                    mcp,
                                },
                            ),
                        },
                    );
                }
            }

            // Audio-in and arrangement cards/menus rendered above the bays (Rule 2).
            if let Some(audio) = audio {
                if let Some(pill) = audio_in(ui.ctx(), panel.layout(), values, Some(audio)) {
                    transport::audio_in_into(ui, &pal, &pill, audio);
                }
            }
            // Learn, map, and arrangement controls laid out in order.
            if let Some(pill) = learn_pill(
                ui.ctx(),
                panel.layout(),
                values,
                audio,
                tracking,
                map,
                armed,
            ) {
                transport::learn_into(ui, &pal, &pill);
            }
            if let Some((pill, map)) =
                map_pill(ui.ctx(), panel.layout(), values, audio, tracking, map).zip(map)
            {
                transport::map_into(ui, &pal, &pill, map);
            }
            if let Some(pill) =
                arrangement(ui.ctx(), panel.layout(), values, audio, tracking, map, arr)
            {
                transport::arrangement_into(ui, &pal, &pill, arr);
            }
            // Inspector wiring card rendered above bays (Rule 2).
            if let Some((pane_at, node, input)) = wiring {
                if let Some(pane) = panes.get(pane_at) {
                    if let Some(at) = inspector(panel.layout(), pane_at, pane, scrolled[pane_at]) {
                        if let Some(line) = at.uses_line(ui.ctx(), pane, node, input, true) {
                            let room = to_egui(panel.layout().viewport());
                            if let (Some(card), Some(uses)) = (
                                line.list(room),
                                pane.nodes.get(node).and_then(|at| at.uses.get(input)),
                            ) {
                                inspector::uses_card_into(ui, &pal, &line, uses, card, room);
                            }
                        }
                    }
                }
            }
            // Pane deck selection dropdown rendered above bays (Rule 2).
            if let Some(pane_at) = pane_open {
                if let Some(pane) = panes.get(pane_at) {
                    if let Some(at) = inspector(panel.layout(), pane_at, pane, scrolled[pane_at]) {
                        let typed = naming
                            .filter(|naming| naming.pane == pane_at)
                            .map(Naming::typed);
                        let room = to_egui(panel.layout().viewport());
                        let policy = self
                            .slot_policies
                            .get(pane.deck)
                            .copied()
                            .unwrap_or_default();
                        let mcp =
                            inspector::slot_mcp_pill(ui.ctx(), &at, pane, policy).map(|p| p.pill);
                        if let Some(target) = pane_target(
                            ui.ctx(),
                            inspector::PaneTargetCtx {
                                at: &at,
                                pane,
                                index: pane_at,
                                naming: typed,
                                decks: load.decks,
                                open: true,
                                mcp,
                            },
                        ) {
                            if let Some(card) = target.list(room) {
                                inspector::pane_list_into(ui, &pal, &target, pane.deck, card);
                            }
                        }
                    }
                }
            }
            // Library target deck list rendered above bays (Rule 2).
            if load.open {
                if let Some(bay) =
                    library(panel.layout(), scopes, sets, opened, pointed, scrolled_to)
                {
                    let at = bay.load(ui.ctx(), load);
                    if let Some(card) = at.list(to_egui(panel.layout().viewport())) {
                        library::deck_list_into(ui, &pal, &at, load, card);
                    }
                }
            }
            // Library row contextual menu rendered above bays.
            if menued.row.is_some() {
                if let Some(bay) =
                    library(panel.layout(), scopes, sets, opened, pointed, scrolled_to)
                {
                    if let Some(menu) =
                        bay.menu(ui.ctx(), to_egui(panel.layout().viewport()), menued)
                    {
                        library::row_menu_into(ui, &pal, &menu);
                    }
                }
            }
            // Sequencer lane card rendered above bays (Rule 2).
            if choices.open {
                if let Some(bay) = sequencer(ui.ctx(), panel.layout(), sequenced, &choices) {
                    if let Some(card) = bay.card {
                        sequencer::lane_card_into(ui, &pal, &card, &choices);
                    }
                }
            }
            // Prompt bay CLI selection dropdown menu rendered above bays (Rule 2).
            if self.prompt.menu_open {
                prompt::prompt_menu_into(ui, &pal, panel.layout(), &self.prompt);
            }
            // Dashed keyboard focus indicator ring, rendered proud of bay heads.
            if let Some((mark, _)) = folded {
                folded_wfocus_into(ui, &pal, mark);
            }
            if let Some(mark) = focused {
                wfocus_into(ui, &pal, mark);
            }
        });
    }

    /// Updates platform cursor icon based on hovered controls or active drag (ADR-0273).
    fn cursor(&self, ctx: &egui::Context, panel: &mut Panel) {
        panel.solve();
        // Active gestures (boundary resizing, carrying, fader drag) determine cursor shape (ADR-0273).
        let axis = match panel.in_hand() {
            Some(InHand::Boundary(axis)) => Some(axis),
            Some(InHand::Carrying) => {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                return;
            }
            Some(InHand::Fader) => None,
            None => match panel.layout().hit(panel.cursor(), GRAB) {
                Hit::Divider { split, .. } => panel.layout().axis(split),
                _ => None,
            },
        };
        match axis {
            Some(Axis::Row) => ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal),
            Some(Axis::Column) => ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical),
            None => {}
        }
    }
}
