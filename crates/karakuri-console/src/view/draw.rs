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
        // **Beside the pictures, and read once for the frame with them**: the
        // word a caption draws is a function of the pair, so a frame that read
        // one of them twice could draw a mark against the other's answer.
        let overloaded = self.overloaded;
        // **Beside the pictures, and read once for the frame for their
        // reason.** What each cell costs and what each cell is showing are two
        // fields because they arrive from two places — see [`View::costs`].
        let costs = self.costs;
        let values = self.transport;
        let arr = &self.arrangement;
        // **Read once for the frame beside the arrangement**, and for the same
        // reason: the arrangement pill is laid out from where this one ends,
        // so a frame that asked twice could lay the two out from two answers.
        let audio = self.audio.as_ref();
        // **And the tracker's own three, read once beside it.** The arrangement
        // pill is laid out from where the octave's second half ends, so a frame
        // that asked twice could lay the row out from two answers.
        let tracking = self.tracker;
        // **And the map, read once beside the two above and for their reason**:
        // `learn`, `map` and the arrangement pill are laid out one from the
        // next, so a frame that asked twice could lay three controls out from
        // three answers.
        let map = self.map.as_ref();
        let armed = self.learn;
        let look_at = self.look;
        let out = self.master_out;
        let chain = self.master_chain.as_ref();
        // What `+ add` offers, read once for the frame: the card that is
        // painted and the card a press lands on are one derivation.
        let adding = self.chain_choices();
        let strips = self.mixer.as_slice();
        let sets = self.library.as_slice();
        // **And what each of those rows is**, read beside the names for their
        // reason: the two halves are one listing (`Rows`), and a badge drawn
        // from a second read could describe a row that had been rewritten
        // under it (ADR-0338).
        let kinds = self.kinds.as_slice();
        // **Which of them are starred, read once for the frame beside the
        // listing it points into** — `draw` takes `&mut self`, and the arm
        // below borrows both.
        let starred = &self.starred;
        let scopes = self.scopes.as_slice();
        // **The fifth pointer, read once for the frame** beside the two slices
        // it borrows from — `draw` takes `&mut self`, and a filter read inside
        // the arm below would be a second borrow of `holds`.
        let narrowed = self.filters();
        // **And where this library is pointed, read once beside it** — the
        // `.path` row is a rectangle in the bay as well as a line of type, so
        // the derivation and the paint are asked one value, and `draw` takes
        // `&mut self` where this borrows two fields.
        let pointed = self.pointed();
        // **The sixth, read here for the two above's reason**: it borrows the
        // listing this frame is drawing, and `draw` takes `&mut self`. It is
        // the cursor and the reading put together — see [`View::opened`].
        let opened = self.opened();
        // **The two pointers, read once for the frame** beside the readings
        // they are drawn against — `draw` takes `&mut self` and the arms below
        // borrow these slices, so a pointer read inside an arm would be a
        // second borrow of the thing it points into.
        let selection = self.selection();
        let cursor_row = self.cursor_row();
        // **Where the dashed ring goes, asked once for the frame** beside the
        // three pointers it is now the same field as — [`View::focus_mark`],
        // which is the derivation this paints from rather than a second
        // reading of where focus is.
        let focused = self.focus_mark(panel);
        // **And the mark a folded bay wears**, read here for the reason above
        // it: it is the same pointer asked a second question, and a folded bay
        // has no rectangle for the ring alone to sit on.
        let folded = self.folded_mark(panel);
        // **The third of them**, and it is read the same way and for the same
        // reason: which chip is marked is a position in the row this frame is
        // drawing, and a scope past its end is the last chip there is.
        let scope = self.marked();
        // **The fourth, and it is read here for the same reason** — the
        // transition row is laid out from it and painted from it, and `draw`
        // takes `&mut self` while the arms below borrow the slices beside it.
        let transition_at = self.transition;
        // Target deck for load controls; distinct from active key selection (ADR-0305).
        let load = self.target();
        // Active popup card coordinates and states, read once per frame.
        let wiring = self.wiring_open;
        let pane_open = self.pane_open;
        let menued = self.menued();
        // **And how far the Library bay is scrolled**, read once for the frame
        // beside the two pointers above it: `library` clamps it and hands the
        // clamped value back, and this is the stored one going in
        // (`LibraryBay::scroll`, P-0082).
        let scrolled_to = self.library_scroll();
        let waiting = self.staging.as_slice();
        // **What the Sequencer bay reads, taken once for the pass** beside the
        // strips it sits under: `draw` takes `&mut self` and the loop below
        // borrows the fields a reading would be read off.
        let sequenced = self.sequencer.as_ref();
        // **And what its `+ lane` chooser offers**, taken here for the same
        // reason and read off three of this console's own values — the strips,
        // the load pulldown's deck and that deck's published rows.
        let choices = self.lane_choices();
        let panes = self.inspector.as_slice();
        // **The eighth pointer, read once for the frame beside the panes it is
        // about** — how far each of them is scrolled. It is `Copy` and two
        // `f32`s wide, and it is read here for the reason the seventh below is:
        // `draw` takes `&mut self` and the loop already borrows `inspector`.
        let scrolled = self.scroll;
        // **The seventh pointer, read once for the frame** beside the panes it
        // points into — `draw` takes `&mut self`, and a head asked inside the
        // loop below would be a second borrow of the same struct.
        let naming = self.naming.as_ref();
        let phase = self.phase;
        // **The opening, read once for the pass.** Every head that opens a
        // class lays its pills out against this one value, so no two capsules
        // on a frame can be placed against two different states.
        let opening = self.opening;
        // **What this crate cannot see** — see `Outputs::told`. Copied out
        // beside `opening` for the same reason: the loop below borrows the
        // arrangement and the palette, and one `bool` read here is one place
        // the answer comes from.
        let projector = self.projector;
        let plugin = self.plugin;
        let plugin_available = self.plugin_available;
        let plugin_name = self.plugin_name;
        let frame = egui::Frame::NONE.fill(pal.ground);
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            for placed in &self.placed {
                let rect = to_egui(placed.rect);
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
                            // **The same derivation `claim` hit-tests**, asked
                            // here rather than inside the paint because the rows
                            // it offers are the mixer's, and this loop already
                            // borrows what a reading of them would come off.
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
            // Dashed keyboard focus indicator ring, rendered proud of bay heads.
            if let Some((mark, title)) = folded {
                folded_head_into(ui, &pal, mark, title);
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
