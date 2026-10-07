use super::*;
use egui::{vec2, Ui};

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

        // Resolution and overload interaction tracking across closure boundary.
        let output_resolutions = self.output_resolutions.as_slice();
        let output_resolution_selected = self.output_resolution_selected;
        let resolution_menu_open = self.resolution_menu_open;
        let mut clicked_recover_deck = None;
        let mut toggle_resolution_menu = false;
        let mut new_resolution_selected = None;
        let mut close_resolution_menu = false;

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
                        if placed.region.name == "program"
                            && draw_program_resolution_pill(
                                ui,
                                &pal,
                                rect,
                                output_resolutions,
                                output_resolution_selected,
                                resolution_menu_open,
                            )
                        {
                            toggle_resolution_menu = true;
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
                        if let Some(row) = self.master_row_layout(ui.ctx(), panel.layout()) {
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
                        let is_focused = self.focused(panel).map(|r| r.name) == Some("prompt");
                        if (!is_focused || self.prompt.menu_open) && self.prompt.is_captured() {
                            self.prompt.set_captured(false);
                        }
                        prompt::prompt_into(ui, &pal, rect, &self.prompt, is_focused);
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
                    if overloaded[deck] {
                        let resp = ui.interact(
                            cell,
                            ui.id().with(("cell_overload_recover", deck)),
                            egui::Sense::click(),
                        );
                        if resp.clicked() {
                            clicked_recover_deck = Some(deck);
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
            if let Some(pill) = mcp_server_pill(
                ui.ctx(),
                panel.layout(),
                values,
                audio,
                tracking,
                map,
                arr,
                &self.mcp_server,
                self.theme_mode,
                self.theme_menu_open,
            ) {
                transport::mcp_server_into(ui, &pal, &pill, &self.mcp_server);
            }
            if let Some(pill) = theme_pill(
                ui.ctx(),
                panel.layout(),
                values,
                audio,
                tracking,
                map,
                arr,
                self.theme_mode,
                self.theme_menu_open,
            ) {
                transport::theme_into(ui, &pal, &pill, self.theme_mode, self.theme_menu_open);
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
                    let (selected, close) = draw_resolution_menu(
                        ui,
                        &pal,
                        bay_rect,
                        to_egui(panel.layout().viewport()),
                        output_resolutions,
                        output_resolution_selected,
                    );
                    if let Some(idx) = selected {
                        new_resolution_selected = Some(idx);
                    }
                    if close {
                        close_resolution_menu = true;
                    }
                }
            }
        });

        // Apply deferred updates after the panel closure to satisfy borrow rules.
        if let Some(deck) = clicked_recover_deck {
            self.ungate_requested = Some(deck);
        }
        if toggle_resolution_menu {
            self.resolution_menu_open = !self.resolution_menu_open;
        }
        if close_resolution_menu {
            self.resolution_menu_open = false;
        }
        if let Some(new_idx) = new_resolution_selected {
            if self.output_resolution_selected != new_idx {
                self.output_resolution_selected = new_idx;
                self.output_resolution_changed = true;
            }
            self.resolution_menu_open = false;
        }
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

/// Measures the bounding rectangle of the resolution selector pill in the Program bay header.
fn resolution_pill_rect(
    ctx: &egui::Context,
    bay_rect: Rect,
    resolutions: &[((u32, u32), String)],
    selected: usize,
) -> Rect {
    let head = head_box(bay_rect);
    let mid = head.center().y;
    // Position next to title "PROGRAM".
    let title_w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            "PROGRAM".to_owned(),
            FontId::new(size::HEAD_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    let label = resolutions
        .get(selected)
        .map(|(_, l)| l.as_str())
        .unwrap_or("---");
    let text_w = pill_width(ctx, label);
    let pill_w = text_w + size::SINK_GAP + CHEVRON_W;
    let left = head.min.x + size::HEAD_PAD_X + title_w + 12.0;
    Rect::from_min_size(
        Pos2::new(left, mid - size::PILL_H * 0.5),
        vec2(pill_w, size::PILL_H),
    )
}

/// Draws the resolution selector pill next to the "PROGRAM" title in the bay header.
fn draw_program_resolution_pill(
    ui: &mut Ui,
    pal: &Palette,
    bay_rect: Rect,
    resolutions: &[((u32, u32), String)],
    selected: usize,
    armed: bool,
) -> bool {
    if resolutions.is_empty() {
        return false;
    }
    let pill = resolution_pill_rect(ui.ctx(), bay_rect, resolutions, selected);
    let label = resolutions
        .get(selected)
        .map(|(_, l)| l.clone())
        .unwrap_or_else(|| "---".into());

    pill_into(ui, pal, pill, &label, armed);

    let chevron_rect = Rect::from_center_size(
        Pos2::new(
            pill.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5,
            pill.center().y,
        ),
        vec2(CHEVRON_W, CHEVRON_H),
    );
    let chevron_color = if armed { pal.mint } else { pal.dim };
    chevron_down(ui.painter(), chevron_rect, chevron_color);

    let resp = ui.interact(
        pill,
        ui.id().with("program_resolution_pill"),
        egui::Sense::click(),
    );
    resp.clicked()
}

/// Draws the floating resolution selection dropdown menu above bays (Rule 2 modal).
fn draw_resolution_menu(
    ui: &mut Ui,
    pal: &Palette,
    bay_rect: Rect,
    viewport: Rect,
    resolutions: &[((u32, u32), String)],
    selected: usize,
) -> (Option<usize>, bool) {
    let pill = resolution_pill_rect(ui.ctx(), bay_rect, resolutions, selected);
    let count = resolutions.len();
    let menu_w = 160.0f32.max(pill.width());
    let row_h = size::LIB_ROW_H;
    let menu_h = size::LIB_LIST_PAD * 2.0 + row_h * count as f32;

    let min_x = pill
        .min
        .x
        .clamp(viewport.min.x + 8.0, viewport.max.x - 8.0 - menu_w);
    let min_y = if pill.max.y + 4.0 + menu_h <= viewport.max.y - 8.0 {
        pill.max.y + 4.0
    } else {
        (pill.min.y - 4.0 - menu_h).max(viewport.min.y + 8.0)
    };
    let menu_rect = Rect::from_min_size(Pos2::new(min_x, min_y), vec2(menu_w, menu_h));

    popup_card(ui.painter(), pal, menu_rect);

    let hover_pos = ui.input(|i| i.pointer.hover_pos());
    let clicked = ui.input(|i| i.pointer.primary_clicked());

    // Check click outside menu and pill to dismiss
    if clicked {
        if let Some(pos) = hover_pos {
            if !menu_rect.contains(pos) && !pill.contains(pos) {
                return (None, true);
            }
        }
    }

    let painter = ui.painter().with_clip_rect(menu_rect);
    let mut selected_choice = None;
    let mut close = false;

    for (index, (_res, label)) in resolutions.iter().enumerate() {
        let row_y = menu_rect.min.y + size::LIB_LIST_PAD + index as f32 * row_h;
        let row_rect = Rect::from_min_size(
            Pos2::new(menu_rect.min.x + size::LIB_LIST_PAD, row_y),
            vec2(menu_rect.width() - size::LIB_LIST_PAD * 2.0, row_h),
        );

        let is_selected = index == selected;
        let is_hovered = hover_pos.is_some_and(|pos| row_rect.contains(pos));

        if is_hovered {
            painter.rect_filled(row_rect, CornerRadius::same(3), tint(pal.mint, 22));
        }

        let text_color = if is_selected {
            pal.mint
        } else if is_hovered {
            pal.text
        } else {
            pal.dim
        };

        card_row_text(&painter, row_rect, label, text_color);

        if is_selected {
            let dot_x = row_rect.max.x - 8.0;
            let mid_y = row_rect.center().y;
            painter.circle_filled(Pos2::new(dot_x, mid_y), 2.5, pal.mint);
        }

        if clicked && is_hovered {
            selected_choice = Some(index);
            close = true;
        }
    }

    (selected_choice, close)
}
