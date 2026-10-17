use super::*;
use egui::Ui;

impl View {
    /// Draw the whole console. The `ui` is the root one [`egui::Context::run_ui`]
    /// hands the frame's closure.
    pub fn draw(&mut self, ui: &mut Ui, panel: &mut Panel) {
        self.mixer_dirty = false;
        self.prompt.cleanup_if_exited();
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
        let arr = &self.arrangement;
        // Audio input settings for pill layout.
        let audio = self.audio.as_ref();
        // MIDI mapping state.
        let map = self.map.as_ref();
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

        // Resolution tracking across closure boundary.
        let output_resolutions = self.output_resolutions.as_slice();
        let output_resolution_selected = self.output_resolution_selected;
        let resolution_menu_open = self.resolution_menu_open;
        let xr_mode = self.xr_mode;
        let xr_menu_open = self.xr_menu_open;

        let is_prompt_focused = self.focused(panel).map(|r| r.name) == Some("prompt");
        if (!is_prompt_focused || self.prompt.menu_open) && self.prompt.is_captured() {
            self.prompt.set_captured(false);
        }
        let master_chain_building = self.master_chain_building;
        let master_row = self.master_row_layout(ui.ctx(), panel.layout());
        let transport_layout = self.transport_layout(ui.ctx(), panel.layout());
        let outputs_layout = self.outputs_layout(ui.ctx(), panel.layout());

        let frame = egui::Frame::NONE.fill(pal.ground);
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            let draw_placed = |ui: &mut Ui, placed: &Placed| {
                let rect = to_egui(placed.rect);
                if panel.layout().is_collapsed(placed.id) {
                    let title = head_of(placed.region).map_or("", |head| head.title);
                    folded_head_into(ui, &pal, rect, title);
                    return;
                }
                match placed.region.kind {
                    Kind::Bay { .. } => {
                        bay_card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        if placed.region.name == "program" {
                            if let Some(pill) = resolution_pill(
                                ui.ctx(),
                                rect,
                                to_egui(panel.layout().viewport()),
                                output_resolutions,
                                output_resolution_selected,
                                resolution_menu_open,
                            ) {
                                resolution_pill_into(
                                    ui,
                                    &pal,
                                    &pill,
                                    output_resolutions,
                                    output_resolution_selected,
                                    resolution_menu_open,
                                );
                            }
                        }
                        // The inspector is the one bay that is a split, and
                        // its panes' boundary is drawn as the mock's
                        // `.divider-v` rather than left as bare ground: it is
                        // inside a card, where the ground does not reach.
                        pane_dividers(ui, &pal, panel, placed.id, rect);
                    }
                    // Transport row readouts and controls.
                    Kind::Transport => {
                        bay_card(ui, &pal, rect);
                        if let Some(ref tl) = transport_layout {
                            transport::transport_into(ui, &pal, &tl.row);
                            if let Some(ref group) = tl.tracker {
                                transport::tracker_into(ui, &pal, group);
                            }
                            if let Some(ref row) = tl.look {
                                transport::look_into(ui, &pal, row);
                            }
                        }
                    }
                    // Outputs control bay.
                    Kind::Outputs => {
                        bay_card(ui, &pal, rect);
                        if let Some(ref row) = outputs_layout {
                            outputs::outputs_into(ui, &pal, row);
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
                                let head = head.with_building(master_chain_building);
                                bay_head(ui, &pal, rect, &head, opening);
                            }
                            None => head_into(ui, &pal, rect, placed.region, opening),
                        }
                        if let Some(ref row) = master_row {
                            master::master_into(ui, &pal, row);
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
                        prompt::prompt_into(ui, &pal, rect, &self.prompt, is_prompt_focused);
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
            };

            for placed in &self.placed {
                if panel.layout().is_in_overlay(placed.id) {
                    continue;
                }
                draw_placed(ui, placed);
            }

            // Deck preview cells rendered over the Program bay.
            if let Some(cells) = cells {
                for (deck, cell) in cells.into_iter().enumerate() {
                    // Carry drop target ring and preview content.
                    let marked = marked_cell == Some(deck as u8);
                    if marked {
                        drop_ring(ui, &pal, cell, size::PREVIEW_RADIUS);
                    }
                    program::preview(ui, &pal, cell, previews[deck], overloaded[deck]);
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
                    if overloaded[deck] {
                        let slot_rect = Rect::from_min_max(
                            cell.min,
                            Pos2::new(cell.max.x, cell.max.y + program::caption_band()),
                        );
                        if ui
                            .input(|i| i.pointer.hover_pos())
                            .is_some_and(|pos| slot_rect.contains(pos))
                        {
                            let painter = ui.painter();
                            let tip =
                                "Overloaded: dropped below 10 FPS. Click to restore / ungate.";
                            let font = FontId::new(size::BASE, FontFamily::Proportional);
                            let galley = painter.layout_no_wrap(
                                tip.into(),
                                font,
                                Color32::from_rgb(255, 230, 230),
                            );
                            let pad_x = 8.0;
                            let pad_y = 4.0;
                            let tip_w = galley.size().x + pad_x * 2.0;
                            let tip_h = galley.size().y + pad_y * 2.0;
                            let tip_pos = Pos2::new(
                                (cell.center().x - tip_w * 0.5).max(cell.min.x),
                                (cell.max.y - tip_h - 4.0).max(cell.min.y),
                            );
                            let tip_rect = Rect::from_min_size(tip_pos, egui::vec2(tip_w, tip_h));
                            painter.rect_filled(
                                tip_rect,
                                CornerRadius::same(4),
                                Color32::from_rgba_unmultiplied(15, 15, 20, 230),
                            );
                            painter.rect_stroke(
                                tip_rect,
                                CornerRadius::same(4),
                                Stroke::new(1.0, Color32::from_rgb(255, 80, 80)),
                                StrokeKind::Inside,
                            );
                            painter.galley(
                                Pos2::new(tip_rect.min.x + pad_x, tip_rect.min.y + pad_y),
                                galley,
                                Color32::from_rgb(255, 230, 230),
                            );
                        }
                    }
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
                    if let Some(drop_box) = carried.and_then(|p| at.dropped(p)) {
                        drop_ring(ui, &pal, drop_box, 0.0);
                    }
                }
            }

            // Keyboard focus indicator ring, rendered proud of bay heads but below floating cards/menus (Rule 2).
            if let Some((mark, _)) = folded {
                folded_wfocus_into(ui, &pal, mark);
            }
            if let Some(mark) = focused {
                wfocus_into(ui, &pal, mark);
            }

            // Semi-transparent backdrop scrim and overlay drawer pass (ADR-0338 / responsive overlay).
            if let Some((_ov_id, ov_rect)) = panel.layout().overlay() {
                let egui_ov = to_egui(ov_rect);
                if let Some(body_rect) = panel.body_rect() {
                    let egui_body = to_egui(body_rect);
                    ui.painter().rect_filled(
                        egui_body,
                        CornerRadius::ZERO,
                        Color32::from_rgba_unmultiplied(0, 0, 0, 140),
                    );
                }
                let radius = CornerRadius::same(size::BAY_RADIUS as u8);
                ui.painter().add(pal.shadow.as_shape(egui_ov, radius));
                ui.painter().rect_filled(egui_ov, radius, pal.panel);
                ui.painter().rect_stroke(
                    egui_ov,
                    radius,
                    Stroke::new(size::HAIRLINE, pal.line),
                    StrokeKind::Inside,
                );

                for placed in &self.placed {
                    if !panel.layout().is_in_overlay(placed.id) {
                        continue;
                    }
                    draw_placed(ui, placed);
                }
            }

            // Protruding drawer tabs in narrow viewports (Option 1-B).
            if panel.is_narrow_body() {
                for side in [DrawerSide::Left, DrawerSide::Right] {
                    if let Some(tab_rect) = panel.drawer_tab_rect(side).map(to_egui) {
                        let is_active = panel.active_drawer() == Some(side);
                        let radius = match side {
                            DrawerSide::Left => CornerRadius {
                                nw: 0,
                                sw: 0,
                                ne: 6,
                                se: 6,
                            },
                            DrawerSide::Right => CornerRadius {
                                nw: 6,
                                sw: 6,
                                ne: 0,
                                se: 0,
                            },
                        };
                        ui.painter().add(pal.shadow.as_shape(tab_rect, radius));
                        ui.painter().rect_filled(
                            tab_rect,
                            radius,
                            if is_active { pal.well } else { pal.panel },
                        );
                        ui.painter().rect_stroke(
                            tab_rect,
                            radius,
                            Stroke::new(
                                size::HAIRLINE,
                                if is_active { pal.mint } else { pal.line },
                            ),
                            StrokeKind::Inside,
                        );

                        let ink = if is_active { pal.mint } else { pal.dim };
                        let label = match side {
                            DrawerSide::Left => "LIB",
                            DrawerSide::Right => "MIX",
                        };
                        let chars: Vec<char> = label.chars().collect();
                        let line_h = 14.0;
                        let total_h = line_h * chars.len() as f32;
                        let start_y = tab_rect.center().y - total_h * 0.5;
                        for (i, ch) in chars.iter().enumerate() {
                            let galley = ui.painter().layout_no_wrap(
                                ch.to_string(),
                                FontId::new(size::HEAD_SIZE, FontFamily::Proportional),
                                ink,
                            );
                            let x = tab_rect.center().x - galley.size().x * 0.5;
                            let y = start_y + i as f32 * line_h + (line_h - galley.size().y) * 0.5;
                            ui.painter().galley(Pos2::new(x, y), galley, ink);
                        }
                    }
                }
            }

            // Audio-in and arrangement cards/menus rendered above the bays (Rule 2).
            if let Some(ref tl) = transport_layout {
                if let (Some(ref pill), Some(audio)) = (&tl.audio, audio) {
                    transport::audio_in_into(ui, &pal, pill, audio);
                }
                if let Some(ref pill) = tl.learn {
                    transport::learn_into(ui, &pal, pill);
                }
                if let (Some(ref pill), Some(map)) = (&tl.map, map) {
                    transport::map_into(ui, &pal, pill, map);
                }
                if let Some(ref pill) = tl.arrangement {
                    transport::arrangement_into(ui, &pal, pill, arr);
                }
                if let Some(ref pill) = tl.mcp {
                    transport::mcp_server_into(ui, &pal, pill, &self.mcp_server);
                }
                if let Some(ref pill) = tl.theme {
                    transport::theme_into(ui, &pal, pill, self.theme_mode, self.theme_menu_open);
                }
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
            // Program bay resolution dropdown menu rendered above bays (Rule 2).
            if resolution_menu_open {
                if let Some(id) = panel.layout().find("program") {
                    let bay_rect = to_egui(panel.layout().rect(id));
                    if let Some(pill) = resolution_pill(
                        ui.ctx(),
                        bay_rect,
                        to_egui(panel.layout().viewport()),
                        output_resolutions,
                        output_resolution_selected,
                        true,
                    ) {
                        resolution_menu_into(
                            ui,
                            &pal,
                            &pill,
                            output_resolutions,
                            output_resolution_selected,
                        );
                    }
                }
            }
            // Outputs WebXR mode dropdown menu rendered above bays (Rule 2).
            if xr_menu_open {
                if let Some(ref row) = outputs_layout {
                    if let Some(ref xr_pill) = row.xr_pill {
                        outputs::xr_menu_into(ui, &pal, xr_pill, xr_mode);
                    }
                }
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
