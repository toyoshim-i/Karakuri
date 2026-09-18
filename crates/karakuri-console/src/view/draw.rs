use super::*;
use egui::Ui;

impl View {
    /// Draw the whole console. The `ui` is the root one [`egui::Context::run_ui`]
    /// hands the frame's closure.
    pub fn draw(&mut self, ui: &mut Ui, panel: &mut Panel) {
        self.mixer_dirty = false;
        let pal = self.room.palette();
        // **The bay arranges itself before anything else this frame reads the
        // layout**, the cursor below included: the boundary between the
        // picture and the row is not there while the cells are beside the
        // picture, and a resize cursor drawn from the solve before the
        // rearrangement is a cursor for a divider that has gone.
        plan_into(panel, self.canvas, &mut self.placed);
        self.cursor(ui.ctx(), panel);

        // **Where the four cells go, asked once and off the same derivation
        // the picture came through.** Not from `placed`: beside the picture
        // the row is set aside, so `deck-previews` is not in the plan at all
        // and a cell drawn from its entry would be a cell drawn in one of the
        // two arrangements only.
        let program = program_bay(panel.layout(), self.canvas);
        let cells = program.and_then(|bay| bay.cells);
        // **Where a Set in hand would land, resolved once for the frame off
        // the derivations the release asks.** `None` for every other drag and
        // for no drag at all, which is what makes this the *carry's* mark
        // rather than a hover: `Panel::in_hand` is the question *is a gesture
        // in progress*, and `Panel::cursor` is where the pointer is whoever
        // claimed the event.
        //
        // **At most one of the two answers**, and that is where the pointer is
        // rather than a rule either bay keeps: the mixer's strips and the
        // Program bay's cells are rectangles in two regions of the panel, so a
        // point inside one set is outside the other. Over an alley between two
        // strips, over a bay head, over the transition row or over another bay
        // both answer `None` and nothing at all is marked — a mark that
        // snapped to the nearest strip would name a deck nobody pointed at,
        // and the release that followed it would load one.
        //
        // **The strip half is resolved inside the bay's own arm below**, off
        // the one `mixer` call this frame makes: a second derivation would be
        // a second answer, and the mark has to be round the strip the release
        // will name. Only the cell half is answered here, where the bay it
        // reads is already derived.
        //
        // **The slot count goes in with the point**, and it is the same
        // reading [`View::select`] refuses a key on: the row is [`DECKS`]
        // cells whatever the deck holds, so a cell whose letter names no slot
        // is a rectangle with nothing to load into
        // ([`ProgramBay::dropped`]).
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
        // **What the foot's load control is aimed at, read once for the
        // frame** beside the pointers above it and for their reason: `draw`
        // takes `&mut self` and the arm below borrows the slices this reads
        // its deck count off. **It is not `selection`** — the pulldown is this
        // bay's own mark and the two are free to name two different decks,
        // which is `View::target` the field.
        let load = self.target();
        // **And the row menu's reading, off the same borrow and for the same
        // reason** — it counts the strips too, and the arm below borrows the
        // slices it would be read off.
        // **Which `uses` line has its card down**, read once for the frame like
        // every other pointer this console keeps — see [`View::wiring_open`].
        let wiring = self.wiring_open;
        // **And which pane head's pulldown is down, read the same way** — see
        // [`View::pane_open`]. The count of decks it offers is the strips',
        // which `load` above has already read off the same slice.
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
                    // The other row with something in it, and everything in it
                    // is a readout: where each one goes is `transport`'s
                    // answer, and with no engine behind the console there is
                    // no answer and the card is as empty as every other body
                    // in this pass.
                    Kind::Transport => {
                        bay_card(ui, &pal, rect);
                        if let Some(row) = transport(ui.ctx(), panel.layout(), values) {
                            transport::transport_into(ui, &pal, &row);
                        }
                        // **The look's two controls, inside the row they
                        // belong to.** Unlike the arrangement pill this has
                        // nothing that hangs out of the row, so it is painted
                        // here rather than after the loop — and the pill's
                        // menu, which does hang out, is painted after and
                        // therefore over it, which is the order a card that is
                        // above the bays wants. Where it goes is `look`'s
                        // answer and not this pass's: the same call
                        // `input::claim` makes.
                        // **The tracker's other three, inside the row they
                        // belong to**, and painted before the look for the one
                        // reason that matters: the look is laid out from the
                        // arrangement pill, which is laid out from these, so
                        // this is the first of the three answers rather than a
                        // second one. Where it goes is `tracker_group`'s — the
                        // same call `input::claim` makes.
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
                    // The one row with something in it, and the something is
                    // one control. Where it goes is `outputs`'s answer and
                    // not this pass's: the same call `input::claim` makes, so
                    // the chip that is painted is the chip that is clicked.
                    Kind::Outputs => {
                        bay_card(ui, &pal, rect);
                        if let Some(row) =
                            outputs(ui.ctx(), panel.layout(), opening).map(|r| r.told(projector))
                        {
                            outputs::outputs_into(ui, &pal, &row);
                        }
                    }
                    // The one bay with something in its body, and it is a bay
                    // in every other respect: the same card and the same head
                    // the other six get, and then as many strips as the deck
                    // has. With no deck behind the console there are none and
                    // the body is as empty as every other one in this pass —
                    // `mixer`'s answer, not this pass's, so that *no deck
                    // means nothing at all* is decided in one place.
                    Kind::Mixer => {
                        bay_card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(bay) = mixer(ui.ctx(), panel.layout(), strips) {
                            // **Which strip a Set in hand would land on, off
                            // this bay and not a second one.** `Mixer::dropped`
                            // is the derivation the release asks, so the ring
                            // is painted round the strip that release names.
                            let marked = carried.and_then(|at| bay.dropped(at));
                            mixer::mixer_into(ui, &pal, &bay, phase, selection, marked);
                        }
                        // **The transition row, under the strips**, and it is
                        // painted from here rather than from inside
                        // `mixer_into` because it is not a strip's: the three
                        // settings are the console's own and are drawn with or
                        // without a deck, where `mixer` answers `None` for a
                        // console that has none. Where the row goes is
                        // `transition`'s answer and not this pass's: the same
                        // call `input::claim` makes, so the pill that is
                        // painted is the pill that is clicked. It is the
                        // arrangement `look_into` already has beside
                        // `transport_into`, one row down.
                        //
                        // **The strip count goes with it**, and it is the one
                        // thing on this row that is a reading: whether the
                        // `go` capsule is lit is *whether a wipe has anywhere
                        // to come from*, which is the mixer's own length and
                        // the same count `TransitionRow::go` is asked with.
                        if let Some(row) = transition(ui.ctx(), panel.layout(), transition_at) {
                            mixer::transition_into(ui, &pal, &row, strips.len());
                        }
                    }
                    // **The third bay with something in its body, and it is
                    // one row of the three the mock draws there.** The card
                    // and the head are every other bay's; under the row the
                    // card shows through, because the chain the mock draws is
                    // three effects that exist nowhere in this workspace and a
                    // control over machinery that is not there is the
                    // scaffolding this module refuses. Where the row goes is
                    // `master`'s answer and not this pass's: the same call
                    // `input::claim` makes, so the knob that is painted is the
                    // knob a hand takes hold of.
                    // **The fifth bay with something in its body**, and the
                    // first one whose body reads a value no engine holds: a
                    // pattern is authored state a session keeps, handed in
                    // per frame like every other reading here. The card and
                    // the head are every other bay's; with no pattern behind
                    // the console there are no rows and the body is as empty
                    // as every other one in this pass — `sequencer`'s answer,
                    // not this pass's. Where the cells go is that function's
                    // too: the same call `input::claim` makes, so the cell
                    // that is painted is the cell a press lands on.
                    Kind::Sequencer => {
                        bay_card(ui, &pal, rect);
                        // **The one head on this console that is handed a
                        // value**: the bank pills say which of the four the
                        // rows are reading, and that is the armed bank rather
                        // than anything the region table could carry
                        // ([`Head::banks`]). With no pattern behind the console
                        // there is no armed bank either, and the head is every
                        // other bay's.
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
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(row) = master(ui.ctx(), panel.layout(), out, chain, &adding) {
                            master::master_into(ui, &pal, &row);
                            // The third set of rectangles a release can land
                            // on (ADR-0273). One ring round the whole list: an
                            // add appends, so every point of it names the same
                            // landing.
                            if let Some(list) = carried.and_then(|at| row.dropped(at)) {
                                drop_ring(ui, &pal, list, f32::from(size::FX_RADIUS));
                            }
                        }
                    }
                    // The other bay with something in its body, and it is a
                    // bay in every other respect: the same card and the same
                    // head, then the row of chips saying which library is
                    // being read, and then as many rows of that library as the
                    // bay has room for. Told nothing about any library there
                    // is neither, and the body is as empty as every other one
                    // in this pass — `library`'s answer, not this pass's, so
                    // that *nothing said means nothing at all* is decided in
                    // one place.
                    Kind::Library => {
                        bay_card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(bay) =
                            library(panel.layout(), scopes, sets, opened, pointed, scrolled_to)
                        {
                            // **The chips before the rows**, and painted from
                            // the draw rather than from inside the listing's
                            // own paint: the scope row belongs to the bay's
                            // head — it says which library is being read,
                            // where everything under it is what that library
                            // holds.
                            library::scopes_into(ui, &pal, &bay, scopes, scope);
                            // **And the path row between them and the fields**,
                            // for the scope row's reason and one more of its
                            // own: it says which directory this library is
                            // pointed at, where the chips say which library —
                            // and it is drawn whichever chip is marked,
                            // because it is also where a send's save dialog
                            // opens (ADR-0311).
                            if let Some(at) = pointed {
                                library::path_into(ui, &pal, &bay, at);
                            }
                            // **And the filter row between them and the
                            // rows**, for the scope row's reason: it belongs
                            // to the bay's head — it narrows what the library
                            // being read answers, where everything under it is
                            // what came back.
                            library::filters_into(ui, &pal, &bay, narrowed);
                            // **And the kind row under the field**, which is
                            // the filter row's own sentence one band down: it
                            // narrows what the library answers by what a row
                            // *is*, where the field above narrows by what a
                            // Set is made of (ADR-0338).
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
                    // **The fourth bay with something in its body**, and it is
                    // a bay in every other respect: the same card and the same
                    // head, and then a row per node a build changed and whose verdict is
                    // outstanding. With no engine behind the console there are
                    // none and the body is as empty as every other one in this
                    // pass — `staging`'s answer, not this pass's, so that
                    // *nothing waiting means nothing at all* is decided in one
                    // place. The head takes no pill: the mock's `2 waiting` is
                    // a readout, and a bay head's pills are its controls.
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
                    // The one body that is not empty, because its texels are
                    // not this crate's to invent: a texture is there or it is
                    // not, and where it is not the bay's card shows through
                    // exactly as every other empty body does. No placeholder,
                    // for the reason this module's documentation gives.
                    Kind::Picture => {
                        if let Some(picture) = picture {
                            // Clipped to the region: the rectangle came from
                            // outside, and a stale one is a picture painted
                            // over the inspector rather than a wrong picture.
                            ui.painter().with_clip_rect(rect).image(
                                picture.id,
                                picture.rect,
                                WHOLE_TEXTURE,
                                Color32::WHITE,
                            );
                        }
                    }
                    // **The one arm that draws nothing, and it is not an
                    // empty body.** The four cells are the bay's and not this
                    // region's: they are drawn below, once, from wherever
                    // `program_bay` put them — which is under the picture on a
                    // narrow bay and down the sides of a wide one, where this
                    // region has no extent and no entry in the plan at all. A
                    // cell drawn from here would be a cell drawn in one of the
                    // two arrangements only.
                    Kind::Previews => {}
                }
            }

            // **The deck previews, wherever they went.** Painted after the
            // loop rather than in it for the reason the arm above gives, and
            // painting last costs nothing: they sit inside the Program bay's
            // card, which the loop has already painted, and nothing else on
            // the panel overlaps them. A cell is the region's own face rather
            // than a placeholder — the module documentation is where that
            // argument is.
            if let Some(cells) = cells {
                for (deck, cell) in cells.into_iter().enumerate() {
                    // **The drop mark, and it is not a reading of the cell.**
                    // What is behind the rectangle is not read — a cell
                    // drawing material, a still, or nothing at all is a target
                    // exactly the same — so this is `marked_cell`'s answer and
                    // not `previews[deck]`'s. Painted before the image so the
                    // hairline round the well goes on over it, exactly as the
                    // ring round a strip goes under the selection's.
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

            // **The Inspector's panes, after the loop for the same reason.**
            // `Kind::Pane` is a unit variant and stays one — `tests/view.rs`
            // asserts that both panes are one — so it cannot carry *which*
            // pane, and comparing `inspector-1` against a name on the frame
            // path is the string in the loop this table exists to avoid. They
            // sit inside the Inspector bay's card, which the loop has already
            // painted, and nothing else on the panel overlaps them.
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
                        &at,
                        pane,
                        inspector::on_air(strips, pane.deck),
                        policy,
                        typed,
                        // **The same derivation `claim` hit-tests**, asked
                        // here rather than inside the paint because the rows
                        // it offers are the mixer's, and this loop already
                        // borrows what a reading of them would come off.
                        pane_target(
                            ui.ctx(),
                            &at,
                            pane,
                            index,
                            typed,
                            load.decks,
                            pane_open == Some(index),
                            mcp,
                        ),
                    );
                }
            }

            // **The arrangement pill, and its menu over everything.** Painted
            // after the loop for a reason the two above do not have: the menu
            // hangs *out* of the row it belongs to and down over the bays, so
            // a card painted from the `Kind::Transport` arm would go on before
            // the Library bay and end up under it. The pill itself is inside
            // its row and would not care; the two halves of one control are
            // one call, and where it goes is `arrangement`'s answer — the same
            // call `input::claim` makes, so the pill that is painted is the
            // pill that is clicked.
            // **The audio-in pill, and its card over everything**, for the
            // arrangement pill's reason one item to the left: the card hangs
            // out of the row and down over the bays. It is painted before the
            // arrangement pill because it is drawn before it in the row; the
            // two cards can never both be down, since `input`'s rule 2 sends
            // the press that would open the second one to whichever is already
            // open, and that press shuts it.
            if let Some(audio) = audio {
                if let Some(pill) = audio_in(ui.ctx(), panel.layout(), values, Some(audio)) {
                    transport::audio_in_into(ui, &pal, &pill, audio);
                }
            }
            // **`learn` and `map`, in the mock's order and before the
            // arrangement pill**, which is where they sit in `.transport`:
            // `learn`, `map · nanoKONTROL2 ▾`, `arr · night ▾`. Neither has a
            // card to hang out of the row, so either could have been painted
            // from the `Kind::Transport` arm; they are here so that the three
            // controls that are laid out one from the next are painted in one
            // place, in the order they are laid out.
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
            // **A `uses` line's card, over everything for the same reason** —
            // it hangs out of a line inside a pane and down over the groups
            // under it, so a card painted from inside the pane loop would go on
            // before them. It can never be down while any of the others is,
            // because `input`'s rule 2 sends the press that would open a second
            // one to whichever is already open.
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
            // **A pane head's deck list, over everything for the same reason**
            // — it hangs out of a head at the top of a pane and down over that
            // pane's own groups, so a card painted from inside the pane loop
            // would go on before them. It can never be down while any of the
            // others is, because `input`'s rule 2 sends the press that would
            // open a second one to whichever is already open.
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
                        if let Some(target) =
                            pane_target(ui.ctx(), &at, pane, pane_at, typed, load.decks, true, mcp)
                        {
                            if let Some(card) = target.list(room) {
                                inspector::pane_list_into(ui, &pal, &target, pane.deck, card);
                            }
                        }
                    }
                }
            }
            // **The Library bay's deck list, over everything for the two
            // cards' reason** — it hangs out of that bay's foot and up over
            // its own rows, so a card painted from inside the arm would go on
            // before them. It can never be down while either of those is,
            // because `input`'s rule 2 sends the press that would open a
            // second one to whichever is already open.
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
            // **And a row's menu, last of the four cards** — it hangs out of a
            // row of that bay's list and down over the rows under it, so a
            // card painted from inside the arm would go on before them. It can
            // never be down while any of the other three is, for their reason.
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
            // **And the Sequencer's `+ lane` chooser, a fifth card here for
            // the four above it's reason** — it hangs out of that bay's foot
            // and up over its own rows, so a card painted from inside the arm
            // would go on before them and under whatever is drawn after that
            // bay. It can never be down while any of the other four is, for
            // their reason.
            if choices.open {
                if let Some(bay) = sequencer(ui.ctx(), panel.layout(), sequenced, &choices) {
                    if let Some(card) = bay.card {
                        sequencer::lane_card_into(ui, &pal, &card, &choices);
                    }
                }
            }
            // **The dashed focus ring, painted last** — after every card and
            // every menu, for the five cards above it's reason read the other
            // way: this mark is *proud* of the head it rings
            // ([`size::WFOCUS_OFFSET`]), so it lands on the ground between two
            // bays, and anything drawn after it would go over it. **One ring
            // on one bay**, because focus is in one place — see
            // [`View::focus_mark`], which is the derivation and where the
            // folded case is argued.
            if let Some((mark, title)) = folded {
                folded_head_into(ui, &pal, mark, title);
                folded_wfocus_into(ui, &pal, mark);
            }
            if let Some(mark) = focused {
                wfocus_into(ui, &pal, mark);
            }
        });
    }

    /// Say what is under the pointer.
    ///
    /// `egui` sets it, not `winit`. `egui_winit`'s `handle_platform_output` already
    /// writes the window's cursor from `PlatformOutput` every frame, so a
    /// `Window::set_cursor` call beside it is a second writer and the last one each
    /// frame wins — which is a flicker that depends on event order. One writer, and
    /// it is the one that is already there.
    fn cursor(&self, ctx: &egui::Context, panel: &mut Panel) {
        panel.solve();
        // A boundary in hand keeps the resize cursor even where the pointer
        // has run off it, for the reason `input`'s rule 1 keeps the events:
        // the gesture is what is happening, not the position.
        //
        // **And a gesture that is not a boundary drag suppresses it**, for the
        // same reason and in the other direction. Falling through to the hit
        // test with a fader in hand would flick a resize cursor on the moment
        // a fader held against its top let the pointer wander across a
        // boundary, which is a cursor for a gesture that is not happening.
        //
        // There is no cursor of its own for a fader: a knob under the hand is
        // already drawn where the hand is, and a value moving is the whole of
        // what a fader drag says. So it suppresses the resize and asks for
        // nothing in its place.
        //
        // **A carry suppresses it for the same reason and does not stop
        // there.** A Set on its way to a deck crosses every boundary between
        // the Library bay and the mixer, so falling through to the hit test
        // would flick a resize cursor on over each of them — a cursor for a
        // gesture that is not happening. What goes on instead is
        // `CursorIcon::Grabbing`, and `console.html` is what asks for it:
        // *"the pointer itself is a grab for as long as the Set is in hand …
        // it is the one thing that says a gesture is still running while the
        // hand is over nothing at all"*.
        //
        // **This comment argued against that shape and the argument was
        // wrong**, so it is rewritten rather than deleted: it read that a
        // grab *"would be a third mark in a vocabulary of two, said by one
        // control on the panel"*. Two things are wrong with it. A carry is
        // not said by a control — the Set leaves the Library bay's list and is
        // over no control at all for most of the gesture, which is the one
        // state on this panel that nothing drawn can report. And a carry is
        // the only drag here that can be **cancelled**: a boundary and a fader
        // come to rest wherever the pointer left them, so a cursor for either
        // would be decoration, where this one is the difference between a
        // gesture still running and a press that was missed. The page is
        // where that was settled
        // ([ADR-0273](../../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md));
        // ADR-0265's consequence naming the old shape is annotated there.
        //
        // **Still one writer and still one icon per frame.** The grab is set
        // here rather than beside the drop mark for the reason the whole
        // function exists: `egui_winit` writes the window's cursor out of
        // `PlatformOutput` once, and a second setter is a flicker that depends
        // on event order.
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
